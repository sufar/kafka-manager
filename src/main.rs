#![allow(dead_code)]

mod config;
mod db;
mod error;
mod kafka;
mod middleware;
mod models;
mod pool;
mod routes;
mod telemetry;
mod utils;

#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashSet;
use std::sync::Mutex;
use arc_swap::ArcSwap;
use axum::extract::DefaultBodyLimit;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::db::DbPool;
use crate::kafka::KafkaClients;
use crate::pool::ClusterPools;
use kafka_manager_api::utils::{app_log_path, ensure_log_dir};

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub clients: Arc<ArcSwap<KafkaClients>>,
    pub config: Config,
    /// Kafka 连接池
    pub pools: ClusterPools,
    /// 刷新状态跟踪（用于防止重复刷新）
    pub refresh_state: Arc<Mutex<RefreshState>>,
    /// 导入导出全局锁（同一时间只能有一个导入或导出在进行）
    pub import_export_lock: Arc<Mutex<ImportExportLock>>,
}

/// 刷新状态跟踪结构
#[derive(Debug, Default)]
pub struct RefreshState {
    /// 正在刷新的集群（每个集群同一时间只能有一个 consumer group 刷新）
    pub refreshing_clusters: HashSet<String>,
    /// 是否正在刷新所有集群的 topic
    pub refreshing_all_topics: bool,
    /// 是否正在刷新所有集群的 consumer group
    pub refreshing_all_consumer_groups: bool,
}

/// 导入导出全局状态（同一时间只能有一个导入或导出在进行）
#[derive(Debug, Default)]
pub struct ImportExportLock {
    pub is_busy: bool,
    pub operation: Option<String>, // "import" 或 "export"
}

impl AppState {
    /// 获取 Kafka 客户端（无锁读取）
    pub fn get_clients(&self) -> Arc<KafkaClients> {
        self.clients.load_full()
    }

    /// 更新 Kafka 客户端（原子操作）
    pub fn set_clients(&self, clients: KafkaClients) {
        self.clients.store(clients.into());
    }

    /// 获取数据库连接池
    pub fn get_pool(&self) -> sqlx::Pool<sqlx::Sqlite> {
        self.db.inner().clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 与 Tauri 共用同一个日志文件
    // 确保日志目录存在
    ensure_log_dir();

    // 启动时清理旧日志文件（只保留今天的）
    cleanup_old_log_files();

    // 使用普通日志文件（不滚动，每次启动时清理）
    let log_path = app_log_path();
    let log_dir = log_path.parent().unwrap_or(std::path::Path::new("."));
    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        tracing_appender::rolling::Rotation::NEVER,
        log_dir,
        "kafka-manager.log"
    );
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // 初始化日志（同时输出到控制台和文件）
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kafka_manager_api=info,tower_http=info,rdkafka=warn".into()),
        )
        // 控制台输出带颜色
        .with(tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true))
        // 文件输出不带颜色（纯文本），使用自定义时间格式
        .with(tracing_subscriber::fmt::layer()
            .with_writer(non_blocking_file)
            .with_ansi(false)
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                "%Y-%m-%d %H:%M:%S".to_string()
            )))
        .init();

    // 测试日志是否正常工作
    tracing::info!("=== Kafka Manager API starting ===");
    tracing::debug!("Debug logging enabled");
    tracing::info!("Log directory: {:?}", log_dir);

    // 加载配置
    let config = Config::load("config.toml")?;
    tracing::info!(
        "Starting server on {}:{}",
        config.server.host,
        config.server.port
    );

    // 初始化数据库
    let db = DbPool::new("kafka_manager.db").await?;
    db.init().await?;
    tracing::info!("Database initialized: kafka_manager.db");

    // 清理过期的发送历史记录（保留最近 30 天）
    match crate::db::sent_message::cleanup_old_sent_messages(db.inner(), 30).await {
        Ok(count) => tracing::info!("Startup cleanup: cleaned up {} expired message records", count),
        Err(e) => tracing::warn!("Failed to cleanup old sent messages: {}", e),
    }

    // 从数据库加载集群配置
    let clusters = load_clusters_from_db(db.inner()).await?;

    // 创建空的 KafkaClients 和 ClusterPools，立即启动 HTTP 服务
    // Kafka 连接在后台异步建立，不阻塞服务启动
    let empty_clusters = std::collections::HashMap::new();
    let clients = KafkaClients::new(&empty_clusters)
        .expect("Failed to create empty KafkaClients");
    let clients: Arc<ArcSwap<KafkaClients>> = Arc::new(ArcSwap::new(clients.into()));
    tracing::info!("KafkaClients initialized (empty, connections will be established in background)");

    let pools = ClusterPools::new();
    tracing::info!("Kafka connection pools initialized (empty)");

    // 初始化刷新状态跟踪
    let refresh_state = Arc::new(Mutex::new(RefreshState::default()));
    tracing::info!("Refresh state tracking initialized");

    // 初始化导入导出全局锁
    let import_export_lock = Arc::new(Mutex::new(ImportExportLock::default()));

    // 应用状态
    let state = AppState {
        db: db.clone(),
        clients,
        config: config.clone(),
        pools: pools.clone(),
        refresh_state,
        import_export_lock,
    };

    // 构建路由
    let app = routes::create_router(state.clone())
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        .layer(CompressionLayer::new()
            .gzip(true)
            .br(true))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // 启动服务器（不等 Kafka 连接）
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .expect("Invalid address");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on {}", addr);

    // 服务启动后，后台任务：
    // 1. 逐个建立 Kafka 连接（客户端 + 连接池）
    // 2. 所有连接就绪后，并行同步所有集群的 topic 列表
    if !clusters.is_empty() {
        let state_for_bg = state.clone();
        let pool_config = config.pool.clone();
        let warmup_size = config.pool.min_size;
        tokio::spawn(async move {
            let mut connected = Vec::new();
            let mut failed = Vec::new();

            // 阶段 1：逐个建立 Kafka 连接
            for (cluster_id, kafka_config) in &clusters {
                // 创建 Kafka 客户端
                let new_clients = match state_for_bg.get_clients().with_added_cluster(cluster_id, kafka_config) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!("Failed to create clients for cluster '{}': {}", cluster_id, e);
                        failed.push((cluster_id.clone(), e.to_string()));
                        continue;
                    }
                };

                // 建立连接池
                if let Err(e) = state_for_bg.pools.add_cluster(cluster_id, kafka_config, &pool_config).await {
                    tracing::warn!("Failed to create pool for cluster '{}': {}", cluster_id, e);
                    failed.push((cluster_id.clone(), e.to_string()));
                    continue;
                }

                state_for_bg.set_clients(new_clients);
                connected.push(cluster_id.clone());
                tracing::info!("Connected to cluster: {}", cluster_id);
            }

            // 预热 Consumer Pool
            if warmup_size > 0 {
                for cluster_id in &connected {
                    if let Some(consumer_pool) = state_for_bg.pools.get_consumer_pool(cluster_id).await {
                        let size = warmup_size as usize;
                        let cid = cluster_id.clone();
                        tokio::spawn(async move {
                            let _ = crate::pool::kafka_consumer::warmup_consumer_pool(&consumer_pool, size).await;
                            tracing::debug!("Consumer pool warmup completed for {}", cid);
                        });
                    }
                }
            }

            if !connected.is_empty() {
                tracing::info!("Background Kafka connection completed: {} connected, {} failed", connected.len(), failed.len());
            }
            if !failed.is_empty() {
                for (id, err) in &failed {
                    tracing::warn!("Cluster '{}' connection failed: {}", id, err);
                }
            }

            // 阶段 2：所有连接就绪后，并行同步 topic 列表
            if !connected.is_empty() {
                use crate::routes::unified::refresh_single_cluster;

                tracing::info!("Starting background topic sync for {} clusters", connected.len());

                let mut tasks = Vec::with_capacity(connected.len());
                for cluster_name in &connected {
                    let state = state_for_bg.clone();
                    let name = cluster_name.clone();
                    tasks.push(tokio::spawn(async move {
                        refresh_single_cluster(state, name).await;
                    }));
                }

                for task in tasks {
                    let _ = task.await;
                }

                tracing::info!("Background topic sync completed for all clusters");
            }
        });
    }

    // 遥测后台任务：启动时检查 MySQL 连接并上报，每小时检查一次
    {
        let db_for_telemetry = db.clone();
        tokio::spawn(async move {
            // 启动时检查 MySQL TCP 连接
            tracing::info!("[Telemetry] Checking MySQL connection on startup...");
            let mysql_connected = telemetry::check_mysql_connection().await;

            if !mysql_connected {
                tracing::info!("[Telemetry] MySQL TCP connection not available, telemetry disabled");
                return;
            }

            // 尝试建立 MySQL 连接
            let mysql_pool = match telemetry::connect_mysql().await {
                Ok(pool) => pool,
                Err(e) => {
                    tracing::warn!("[Telemetry] MySQL connection failed: {}, telemetry disabled", e);
                    return;
                }
            };

            tracing::info!("[Telemetry] MySQL connection established, telemetry enabled");

            // 启动时立即上报一次
            if let Err(e) = telemetry::do_telemetry_report(db_for_telemetry.inner(), &mysql_pool).await {
                tracing::warn!("[Telemetry] Startup telemetry report failed: {}", e);
            }

            // 每小时检查一次
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // 每小时重新检查 MySQL 连接
                if !telemetry::check_mysql_connection().await {
                    tracing::debug!("[Telemetry] MySQL connection lost, skipping hourly report");
                    continue;
                }

                // 尝试上报
                tracing::debug!("[Telemetry] Hourly telemetry check...");
                if let Err(e) = telemetry::do_telemetry_report(db_for_telemetry.inner(), &mysql_pool).await {
                    tracing::warn!("[Telemetry] Hourly telemetry report failed: {}", e);
                }
            }
        });
    }

    axum::serve(listener, app).await?;

    Ok(())
}

/// 从数据库加载集群配置
async fn load_clusters_from_db(
    pool: &sqlx::SqlitePool,
) -> Result<std::collections::HashMap<String, crate::config::KafkaConfig>, crate::error::AppError> {
    use crate::config::KafkaConfig;
    use crate::db::cluster::ClusterStore;

    let db_clusters = ClusterStore::list(pool).await?;
    let mut clusters = std::collections::HashMap::with_capacity(db_clusters.len());

    for cluster in db_clusters {
        clusters.insert(
            cluster.name,
            KafkaConfig {
                brokers: cluster.brokers,
                request_timeout_ms: cluster.request_timeout_ms as u32,
                operation_timeout_ms: cluster.operation_timeout_ms as u32,
            },
        );
    }

    Ok(clusters)
}

/// 清理日志文件（启动时直接删除旧的日志文件）
fn cleanup_old_log_files() {
    let log_path = app_log_path();

    // 删除所有 kafka-manager 相关的日志文件
    let log_dir = log_path.parent().unwrap_or(std::path::Path::new("."));
    if let Ok(entries) = std::fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with("kafka-manager") && file_name.ends_with(".log") {
                if let Err(e) = std::fs::remove_file(entry.path()) {
                    eprintln!("Failed to remove log file {}: {}", file_name, e);
                } else {
                    println!("Removed log file: {}", file_name);
                }
            }
        }
    }
}
