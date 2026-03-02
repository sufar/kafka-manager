#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::time::Duration;
use std::path::PathBuf;
use std::sync::mpsc;

// 引用主项目的 kafka-manager-api crate
use kafka_manager_api::{
    Config, DbPool, KafkaClients, AuthMiddleware, ClusterPools,
    MetadataCache, TaskStore, HealthChecker, HealthCheckConfig,
    AppState, create_router,
};

/// 启动后端服务器
async fn start_backend(ready_tx: mpsc::Sender<bool>) -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 确定配置文件路径
    let config_path = if cfg!(debug_assertions) {
        // 开发模式：使用项目根目录的配置文件
        PathBuf::from("config.toml")
    } else {
        // 生产模式：使用可执行文件所在目录的配置文件
        let exe_dir = std::env::current_exe()?
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        exe_dir.join("config.toml")
    };

    // 加载配置
    let config = Config::load(&config_path)?;

    // 创建数据库连接池
    let db_path = if cfg!(debug_assertions) {
        "kafka_manager.db".to_string()
    } else {
        let data_dir = std::env::current_exe()?
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        data_dir.join("kafka_manager.db").to_string_lossy().to_string()
    };
    let pool = DbPool::new(&db_path).await?;

    // 创建 Kafka 客户端管理器
    let clients = Arc::new(RwLock::new(KafkaClients::new(&config.clusters)?));

    // 创建 Kafka 连接池
    let kafka_pools = ClusterPools::new();
    kafka_pools.init(&config.clusters).await?;

    // 创建元数据缓存
    let cache = MetadataCache::new();

    // 创建任务存储和健康检查器
    let task_store = TaskStore::new();
    let health_check_config = HealthCheckConfig::default();
    let health_checker = HealthChecker::new(health_check_config);

    // 创建认证中间件（默认禁用认证）
    let auth = AuthMiddleware::new(vec![], false);

    // 构建应用状态
    let state = AppState {
        db: pool,
        clients,
        config: config.clone(),
        auth,
        pools: kafka_pools,
        cache,
        task_store,
        health_checker,
    };

    // 创建路由
    let app = create_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive());

    // 从配置中读取服务器地址
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Starting backend server on http://{}", addr);

    // 通知后端已启动
    let _ = ready_tx.send(true);

    axum::serve(listener, app).await?;

    Ok(())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn run() {
    // 使用通道等待后端服务器启动
    let (ready_tx, ready_rx) = mpsc::channel::<bool>();

    // 在后台启动后端服务器
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = start_backend(ready_tx).await {
                eprintln!("Failed to start backend: {}", e);
            }
        });
    });

    // 等待后端服务器启动信号
    if ready_rx.recv_timeout(Duration::from_secs(10)).is_ok() {
        tracing::info!("Backend server is ready, starting Tauri application...");
    } else {
        eprintln!("Warning: Backend server startup timed out, starting Tauri application anyway");
    }

    // 启动 Tauri 应用
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![greet, get_app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
