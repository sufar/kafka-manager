#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;
use std::sync::Arc;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use arc_swap::ArcSwap;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};

// 引用主项目的 kafka-manager-api crate
use kafka_manager_api::{
    Config, DbPool, KafkaClients, AuthMiddleware, ClusterPools,
    MetadataCache,
    AppState, create_router,
};
use tauri::{Manager, Emitter};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 简单的日志函数，确保在 Windows 上也能看到输出
fn log(msg: &str) {
    eprintln!("[KAFKA-MANAGER] {}", msg);
}

// 全局AppState存储
static GLOBAL_APP_STATE: once_cell::sync::OnceCell<Arc<AppState>> = once_cell::sync::OnceCell::new();

// 取消查询handles
static CANCEL_HANDLES: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<()>>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// 消息查询SSE相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageQueryRequest {
    cluster_id: String,
    topic: String,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: Option<usize>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<String>,
    sort: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MessageRecord {
    partition: i32,
    offset: i64,
    key: Option<String>,
    value: Option<String>,
    timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
struct MessageEvent {
    #[serde(rename = "type")]
    event_type: String,
    data: Option<MessageRecord>,
    current: Option<usize>,
    total: Option<usize>,
    count: Option<usize>,
    message: Option<String>,
}

/// 启动后端服务器
async fn start_backend(ready_tx: mpsc::Sender<bool>) {
    log("=========================================");
    log("Backend starting...");
    log("=========================================");

    // 获取可执行文件路径信息
    let exe_path = std::env::current_exe().unwrap_or_else(|e| {
        log(&format!("Failed to get exe path: {}", e));
        PathBuf::from(".")
    });
    let exe_dir = exe_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));

    log(&format!("EXE path: {:?}", exe_path));
    log(&format!("EXE dir: {:?}", exe_dir));
    log(&format!("Current dir: {:?}", std::env::current_dir()));

    // 列出 EXE 目录内容（用于诊断）
    log("EXE directory contents:");
    if let Ok(entries) = std::fs::read_dir(&exe_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let meta = entry.metadata();
            log(&format!("  - {:?} (exists: {})", name, meta.is_ok()));
        }
    }

    // 检查 _up_ 目录
    let up_dir = exe_dir.join("_up_");
    log(&format!("_up_ dir exists: {}", up_dir.exists()));
    if up_dir.exists() {
        log("_up_ directory contents:");
        if let Ok(entries) = std::fs::read_dir(&up_dir) {
            for entry in entries.flatten() {
                log(&format!("  - {:?}", entry.file_name()));
            }
        }
    }

    // 确定配置文件路径
    let config_path = if cfg!(debug_assertions) {
        PathBuf::from("config.toml")
    } else {
        // 尝试多个可能的位置
        let candidates = [
            exe_dir.join("_up_").join("config.toml"),
            exe_dir.join("config.toml"),
            PathBuf::from("config.toml"),
        ];

        let mut found = None;
        for candidate in &candidates {
            log(&format!("Checking config: {:?} (exists: {})", candidate, candidate.exists()));
            if candidate.exists() {
                found = Some(candidate.clone());
                break;
            }
        }

        found.unwrap_or_else(|| {
            log("No config found, will use default");
            PathBuf::from("config.toml")
        })
    };

    log(&format!("Using config path: {:?}", config_path));

    // 加载配置
    let config = if config_path.exists() {
        match Config::load(&config_path) {
            Ok(cfg) => {
                log("Config loaded successfully");
                cfg
            }
            Err(e) => {
                log(&format!("Config load error: {}, using default", e));
                Config::default()
            }
        }
    } else {
        log("Config not found, using default");
        Config::default()
    };

    log(&format!("Server will bind to {}:{}", config.server.host, config.server.port));

    // 创建数据库路径 - 开发模式和生产模式都使用用户目录
    // 避免在 src-tauri 目录下创建数据库文件导致 Tauri 热重载循环
    let db_path = {
        // 使用应用数据目录
        let db_filename = "kafka_manager.db";

        let data_dir = if cfg!(target_os = "windows") {
            dirs::data_local_dir().map(|d| d.join("Kafka Manager"))
        } else if cfg!(target_os = "macos") {
            dirs::home_dir().map(|d| d.join("Library/Application Support/Kafka Manager"))
        } else {
            dirs::data_local_dir().map(|d| d.join("kafka-manager"))
        };

        if let Some(dir) = data_dir {
            match std::fs::create_dir_all(&dir) {
                Ok(_) => {
                    let path = dir.join(db_filename);
                    log(&format!("Using database: {:?}", path));
                    path.to_string_lossy().to_string()
                }
                Err(e) => {
                    log(&format!("Failed to create data dir: {}, using exe dir", e));
                    exe_dir.join(db_filename).to_string_lossy().to_string()
                }
            }
        } else {
            log("Failed to get data dir, using exe dir");
            exe_dir.join(db_filename).to_string_lossy().to_string()
        }
    };

    log(&format!("Final database path: {}", db_path));

    // 创建数据库连接池
    log("Creating database pool...");
    let pool = match DbPool::new(&db_path).await {
        Ok(p) => {
            log("Database pool created OK");
            p
        }
        Err(e) => {
            log(&format!("FATAL: Failed to create database pool: {}", e));
            let _ = ready_tx.send(false);
            return;
        }
    };

    // 初始化数据库
    log("Initializing database...");
    if let Err(e) = pool.init().await {
        log(&format!("FATAL: Failed to init database: {}", e));
        let _ = ready_tx.send(false);
        return;
    }
    log("Database initialized OK");

    // 创建 Kafka 客户端
    log("Creating Kafka clients...");
    let clients = match KafkaClients::new(&config.clusters) {
        Ok(c) => {
            log("Kafka clients created OK");
            c
        }
        Err(e) => {
            log(&format!("FATAL: Failed to create Kafka clients: {}", e));
            let _ = ready_tx.send(false);
            return;
        }
    };
    let clients = Arc::new(ArcSwap::new(Arc::new(clients)));

    // 创建 Kafka 连接池
    log("Creating Kafka connection pools...");
    let kafka_pools = ClusterPools::new();
    if let Err(e) = kafka_pools.init(&config.clusters, &config.pool).await {
        log(&format!("FATAL: Failed to init Kafka pools: {}", e));
        let _ = ready_tx.send(false);
        return;
    }
    log("Kafka pools initialized OK");

    // 创建其他组件
    let cache = MetadataCache::new();
    let auth = AuthMiddleware::new(vec![], false);

    // 构建应用状态
    let state = Arc::new(AppState {
        db: pool,
        clients,
        config: config.clone(),
        auth,
        pools: kafka_pools,
        cache,
    });

    // 存储到全局变量
    let _ = GLOBAL_APP_STATE.set(state.clone());

    // 创建路由
    let app = create_router((**GLOBAL_APP_STATE.get().unwrap()).clone())
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive());

    // 绑定地址
    let addr_str = format!("{}:{}", config.server.host, config.server.port);
    log(&format!("Binding to: {}", addr_str));

    let addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            log(&format!("FATAL: Invalid address: {}", e));
            let _ = ready_tx.send(false);
            return;
        }
    };

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => {
            log(&format!("Successfully bound to {}", addr));
            l
        }
        Err(e) => {
            log(&format!("FATAL: Failed to bind: {}", e));
            let _ = ready_tx.send(false);
            return;
        }
    };

    // 通知前端后端已启动
    log("=========================================");
    log("SERVER READY - Starting HTTP service");
    log("=========================================");

    if let Err(e) = ready_tx.send(true) {
        log(&format!("Warning: Failed to send ready signal: {:?}", e));
    }

    // 启动服务器（阻塞）
    if let Err(e) = axum::serve(listener, app).await {
        log(&format!("Server error: {}", e));
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 取消消息查询
#[tauri::command]
async fn cancel_message_query(query_id: String) -> Result<bool, String> {
    let mut handles = CANCEL_HANDLES.lock().await;
    if let Some(sender) = handles.remove(&query_id) {
        let _ = sender.send(()).await;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// 使用SSE方式查询Kafka消息
#[tauri::command]
async fn query_messages_sse(
    window: tauri::Window,
    request: MessageQueryRequest,
    query_id: String,
) -> Result<(), String> {
    // 创建取消channel
    let (cancel_tx, mut cancel_rx) = tokio::sync::mpsc::channel::<()>(1);
    {
        let mut handles = CANCEL_HANDLES.lock().await;
        handles.insert(query_id.clone(), cancel_tx);
    }

    // 获取AppState
    let app_state: Arc<AppState> = GLOBAL_APP_STATE.get()
        .ok_or_else(|| "AppState not initialized".to_string())?
        .clone();

    // 在后台任务中执行查询
    tokio::spawn(async move {
        let result = execute_message_query(
            &window,
            app_state.clone(),
            &request,
            &mut cancel_rx,
            &query_id,
        ).await;

        // 发送错误或完成事件
        if let Err(e) = &result {
            let event = MessageEvent {
                event_type: "error".to_string(),
                data: None,
                current: None,
                total: None,
                count: None,
                message: Some(e.clone()),
            };
            let _ = window.emit(&format!("message-query-{}", query_id), event);
        }

        // 从cancel_handles中移除
        let _ = CANCEL_HANDLES.lock().await.remove(&query_id);

        result
    });

    Ok(())
}

/// 执行消息查询并发送事件
async fn execute_message_query(
    window: &tauri::Window,
    app_state: Arc<AppState>,
    request: &MessageQueryRequest,
    cancel_rx: &mut tokio::sync::mpsc::Receiver<()>,
    query_id: &str,
) -> Result<(), String> {
    use kafka_manager_api::db::cluster::ClusterStore;
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::ClientConfig;

    // 获取集群配置
    let cluster = ClusterStore::get_by_name(app_state.db.inner(), &request.cluster_id)
        .await
        .map_err(|e| format!("数据库错误: {}", e))?
        .ok_or_else(|| format!("集群 '{}' 不存在", request.cluster_id))?;

    let brokers = cluster.brokers;
    let max_messages = request.max_messages.unwrap_or(100);

    // 创建consumer配置
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers);
    cfg.set("group.id", &format!("kafka-mgr-query-{}", query_id));
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("session.timeout.ms", "3000");
    cfg.set("heartbeat.interval.ms", "500");
    cfg.set("fetch.min.bytes", "1");
    cfg.set("fetch.wait.max.ms", "1");
    cfg.set("fetch.max.bytes", "10485760");
    cfg.set("max.partition.fetch.bytes", "10485760");
    cfg.set("socket.nagle.disable", "true");
    cfg.set("enable.partition.eof", "false");
    cfg.set("connections.max.idle.ms", "540000");
    cfg.set("reconnect.backoff.ms", "50");
    cfg.set("reconnect.backoff.max.ms", "500");
    cfg.set("socket.connection.setup.timeout.ms", "3000");
    cfg.set("metadata.max.age.ms", "5000");
    cfg.set("partition.assignment.strategy", "");
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = cfg.create()
        .map_err(|e| format!("创建Consumer失败: {}", e))?;

    // 获取分区列表
    let partition_count: usize;
    let partitions: Vec<i32> = if let Some(p) = request.partition {
        partition_count = 1;
        vec![p]
    } else {
        // 获取topic的所有分区
        let metadata = consumer.fetch_metadata(Some(&request.topic), Duration::from_secs(5))
            .map_err(|e| format!("获取元数据失败: {}", e))?;
        let topic_metadata = metadata.topics()
            .iter()
            .find(|t| t.name() == request.topic)
            .ok_or_else(|| format!("Topic '{}' 不存在", request.topic))?;
        let parts: Vec<i32> = topic_metadata.partitions()
            .iter()
            .map(|p| p.id())
            .collect();
        partition_count = parts.len();
        parts
    };

    if partitions.is_empty() {
        let event = MessageEvent {
            event_type: "completed".to_string(),
            data: None,
            current: None,
            total: None,
            count: Some(0),
            message: None,
        };
        let _ = window.emit(&format!("message-query-{}", query_id), event);
        return Ok(());
    }

    let total_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let search_lower: Option<String> = request.search.as_ref().map(|s| s.to_lowercase());
    let fetch_mode: &str = request.fetch_mode.as_deref().unwrap_or("newest");
    let topic_name: String = request.topic.clone();
    let req_offset: Option<i64> = request.offset;
    let req_start_time: Option<i64> = request.start_time;
    let req_end_time: Option<i64> = request.end_time;

    // 并行处理每个分区
    let mut handles: Vec<tokio::task::JoinHandle<Result<(), String>>> = vec![];

    for partition in &partitions {
        let partition: i32 = *partition;
        let window: tauri::Window = window.clone();
        let query_id: String = query_id.to_string();
        let topic: String = topic_name.clone();
        let total_count: Arc<AtomicUsize> = total_count.clone();
        let search_lower: Option<String> = search_lower.clone();
        let brokers: String = brokers.clone();
        let max_msgs: usize = max_messages / partition_count.max(1);
        let offset: Option<i64> = req_offset;
        let start_time: Option<i64> = req_start_time;
        let end_time: Option<i64> = req_end_time;
        let mode: String = fetch_mode.to_string();

        let handle: tokio::task::JoinHandle<Result<(), String>> = tokio::spawn(async move {
            let result: Result<(), String> = fetch_partition_messages_sse(
                &window,
                &brokers,
                &topic,
                partition,
                max_msgs,
                offset,
                start_time,
                end_time,
                search_lower,
                &mode,
                &query_id,
                total_count,
            ).await;

            if let Err(ref e) = result {
                let event: MessageEvent = MessageEvent {
                    event_type: "error".to_string(),
                    data: None,
                    current: None,
                    total: None,
                    count: None,
                    message: Some(format!("分区 {} 查询失败: {}", partition, e)),
                };
                let _ = window.emit(&format!("message-query-{}", query_id), event);
            }

            result
        });

        handles.push(handle);
    }

    // 等待所有分区查询完成或取消信号
    tokio::select! {
        _ = async {
            for handle in handles {
                let _ = handle.await;
            }
        } => {
            // 所有任务完成
        }
        _ = cancel_rx.recv() => {
            // 收到取消信号
            log(&format!("Message query {} cancelled", query_id));
        }
    }

    // 发送完成事件
    let count = total_count.load(Ordering::SeqCst);
    let event = MessageEvent {
        event_type: "completed".to_string(),
        data: None,
        current: None,
        total: None,
        count: Some(count),
        message: None,
    };
    let _ = window.emit(&format!("message-query-{}", query_id), event);

    Ok(())
}

/// 从单个分区获取消息并发送SSE事件
async fn fetch_partition_messages_sse(
    window: &tauri::Window,
    brokers: &str,
    topic: &str,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search_lower: Option<String>,
    fetch_mode: &str,
    query_id: &str,
    total_count: Arc<AtomicUsize>,
) -> Result<(), String> {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::Message;
    use rdkafka::TopicPartitionList;
    use rdkafka::ClientConfig;

    // 创建consumer
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", brokers);
    cfg.set("group.id", &format!("kafka-mgr-query-partition-{}", query_id));
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("session.timeout.ms", "3000");
    cfg.set("fetch.min.bytes", "1");
    cfg.set("fetch.wait.max.ms", "1");
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = cfg.create()
        .map_err(|e| format!("创建Consumer失败: {}", e))?;

    // 计算起始offset
    let start_offset = if let Some(off) = offset {
        off
    } else if let Some(ts) = start_time {
        // 根据时间戳查找offset
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(ts))
            .map_err(|e| e.to_string())?;

        let result = consumer.offsets_for_times(tpl, Duration::from_secs(10))
            .map_err(|e| format!("时间戳查询失败: {}", e))?;

        result.elements()
            .iter()
            .find(|e| e.topic() == topic && e.partition() == partition)
            .and_then(|e| match e.offset() {
                rdkafka::Offset::Offset(o) => Some(o),
                _ => None,
            })
            .unwrap_or(0)
    } else if fetch_mode == "newest" {
        // 获取最新消息，计算起始offset
        match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
            Ok((low, high)) => {
                let start = std::cmp::max(low, high - max_messages as i64);
                start
            }
            Err(_) => 0,
        }
    } else if let Some(end_ts) = end_time {
        // 如果只指定了 end_time，从 end_time 对应的 offset 往前推 max_messages
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(end_ts))
            .map_err(|e| e.to_string())?;

        match consumer.offsets_for_times(tpl, Duration::from_secs(10)) {
            Ok(result) => {
                let end_offset_for_time = result.elements()
                    .iter()
                    .find(|e| e.topic() == topic && e.partition() == partition)
                    .and_then(|e| match e.offset() {
                        rdkafka::Offset::Offset(o) => Some(o),
                        _ => None,
                    })
                    .unwrap_or(0);
                // 从 end_offset 往前推 max_messages
                match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                    Ok((low, _)) => std::cmp::max(low, end_offset_for_time - max_messages as i64),
                    Err(_) => std::cmp::max(0, end_offset_for_time - max_messages as i64),
                }
            }
            Err(_) => 0,
        }
    } else {
        // 从最早开始
        match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
            Ok((low, _)) => low,
            Err(_) => 0,
        }
    };

    // 分配分区
    let mut tpl = TopicPartitionList::new();
    let seek_offset = if start_offset < 0 {
        rdkafka::Offset::Beginning
    } else {
        rdkafka::Offset::Offset(start_offset)
    };
    tpl.add_partition_offset(topic, partition, seek_offset)
        .map_err(|e| e.to_string())?;
    consumer.assign(&tpl).map_err(|e| e.to_string())?;

    // 计算结束offset（用于 newest 模式或 end_time）
    let end_offset = if let Some(end_ts) = end_time {
        // 根据结束时间戳查找offset
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(end_ts))
            .map_err(|e| e.to_string())?;

        match consumer.offsets_for_times(tpl, Duration::from_secs(10)) {
            Ok(result) => {
                result.elements()
                    .iter()
                    .find(|e| e.topic() == topic && e.partition() == partition)
                    .and_then(|e| match e.offset() {
                        rdkafka::Offset::Offset(o) => Some(o),
                        _ => None,
                    })
            }
            Err(_) => None,
        }
    } else if fetch_mode == "newest" {
        match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
            Ok((_, high)) => Some(high),
            Err(_) => None,
        }
    } else {
        None
    };

    let mut messages_sent = 0usize;
    let timeout = Duration::from_millis(100);
    let mut consecutive_timeouts = 0;
    const MAX_CONSECUTIVE_TIMEOUTS: u32 = 3;

    loop {
        if messages_sent >= max_messages {
            break;
        }

        match consumer.poll(timeout) {
            Some(Ok(msg)) => {
                consecutive_timeouts = 0;

                let msg_partition = msg.partition();
                let msg_offset = msg.offset();

                // 检查是否超过结束offset
                if let Some(end) = end_offset {
                    if msg_offset >= end {
                        break;
                    }
                }

                // 检查时间戳
                if let Some(end_ts) = end_time {
                    if let Some(ts) = msg.timestamp().to_millis() {
                        if ts > end_ts {
                            break;
                        }
                    }
                }

                let key = msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                let value = msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from));
                let timestamp = msg.timestamp().to_millis();

                // 搜索过滤
                if let Some(ref search) = search_lower {
                    let key_match = key.as_ref().map(|k| k.to_lowercase().contains(search)).unwrap_or(false);
                    let value_match = value.as_ref().map(|v| v.to_lowercase().contains(search)).unwrap_or(false);
                    if !key_match && !value_match {
                        continue;
                    }
                }

                let record = MessageRecord {
                    partition: msg_partition,
                    offset: msg_offset,
                    key,
                    value,
                    timestamp,
                };

                // 发送消息事件
                let event = MessageEvent {
                    event_type: "message".to_string(),
                    data: Some(record),
                    current: None,
                    total: None,
                    count: None,
                    message: None,
                };
                let _ = window.emit(&format!("message-query-{}", query_id), event);

                messages_sent += 1;
                total_count.fetch_add(1, Ordering::SeqCst);
            }
            Some(Err(_)) => {
                consecutive_timeouts += 1;
                if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                    break;
                }
            }
            None => {
                consecutive_timeouts += 1;
                if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                    break;
                }
            }
        }
    }

    Ok(())
}

pub fn run() {
    log("Tauri application starting...");

    // 创建通道
    let (ready_tx, ready_rx) = mpsc::channel::<bool>();

    // 在后台线程启动后端
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                log(&format!("FATAL: Failed to create tokio runtime: {}", e));
                let _ = ready_tx.send(false);
                return;
            }
        };

        rt.block_on(start_backend(ready_tx));
    });

    // 等待后端启动信号
    log("Waiting for backend to be ready...");
    let backend_ready = match ready_rx.recv_timeout(Duration::from_secs(30)) {
        Ok(ready) => {
            log(&format!("Backend ready signal received: {}", ready));
            ready
        }
        Err(_) => {
            log("Backend startup timed out after 30 seconds");
            false
        }
    };

    if backend_ready {
        log("Backend is ready, starting UI...");
    } else {
        log("WARNING: Backend failed to start or timed out");
    }

    // 启动 Tauri
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 当检测到另一个实例启动时，激活现有窗口
            log("Another instance detected, focusing existing window");
            if let Some(window) = app.webview_windows().values().next() {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![greet, get_app_version, query_messages_sse, cancel_message_query])
        .setup(|app| {
            // 创建托盘图标菜单
            let show_i = MenuItem::with_id(app, "show", "Show Kafka Manager", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            // 获取应用图标作为托盘图标
            let icon = app.default_window_icon().unwrap().clone();

            // 创建托盘图标
            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            // 显示主窗口
                            if let Some(window) = app.webview_windows().values().next() {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            log("Quit menu item clicked, exiting app and shutting down backend");
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // 阻止默认关闭行为，改为隐藏窗口到菜单栏
                    api.prevent_close();
                    let _ = window.hide();
                    log("Window hidden, app running in menu bar");
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("Failed to run Tauri application");

    // Tauri 应用退出后，清理资源
    log("Tauri application exited, backend will be terminated");

    // 退出进程，确保后端线程也被终止
    std::process::exit(0);
}
