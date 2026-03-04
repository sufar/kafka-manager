#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;
use std::sync::Arc;
use arc_swap::ArcSwap;
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
    // 初始化日志（使用 try_init 避免重复初始化错误）
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_target(true).with_level(true))
        .try_init();

    // 记录启动日志
    tracing::info!("Starting backend server initialization...");
    eprintln!("[Backend] Starting backend server initialization...");

    // 确定配置文件路径
    let config_path = if cfg!(debug_assertions) {
        // 开发模式：使用项目根目录的配置文件
        PathBuf::from("config.toml")
    } else {
        // 生产模式：使用资源目录中的配置文件
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        tracing::info!("Executable path: {:?}", exe_path);
        tracing::info!("Executable directory: {:?}", exe_dir);
        eprintln!("[Backend] Executable directory: {:?}", exe_dir);

        // 尝试多个可能的路径
        let possible_paths = [
            // Windows: 资源文件在可执行文件目录的 _up_ 子目录中
            exe_dir.join("_up_").join("config.toml"),
            // Windows MSI 安装程序可能将资源放在不同位置
            exe_dir.join("../_up_/config.toml"),
            // 与可执行文件同目录
            exe_dir.join("config.toml"),
            // 当前工作目录
            PathBuf::from("config.toml"),
        ];

        let mut found_path = None;
        for path in &possible_paths {
            tracing::info!("Checking config path: {:?}", path);
            eprintln!("[Backend] Checking config path: {:?}", path);
            if path.exists() {
                tracing::info!("Found config at: {:?}", path);
                eprintln!("[Backend] Found config at: {:?}", path);
                found_path = Some(path.clone());
                break;
            }
        }

        found_path.unwrap_or_else(|| {
            tracing::warn!("No config file found, using default");
            eprintln!("[Backend] No config file found, using default");
            PathBuf::from("config.toml")
        })
    };

    tracing::info!("Using config file path: {:?}", config_path);
    eprintln!("[Backend] Using config file path: {:?}", config_path);

    // 加载配置（如果不存在，使用默认配置）
    let config = if config_path.exists() {
        match Config::load(&config_path) {
            Ok(cfg) => {
                tracing::info!("Config loaded successfully");
                eprintln!("[Backend] Config loaded successfully");
                cfg
            }
            Err(e) => {
                tracing::error!("Failed to load config: {}, using default", e);
                eprintln!("[Backend] Failed to load config: {}, using default", e);
                Config::default()
            }
        }
    } else {
        tracing::warn!("Config file not found, using default configuration");
        eprintln!("[Backend] Config file not found, using default configuration");
        Config::default()
    };

    tracing::info!("Server config: {}:{}", config.server.host, config.server.port);
    eprintln!("[Backend] Server config: {}:{}", config.server.host, config.server.port);

    // 创建数据库连接池
    let db_path = if cfg!(debug_assertions) {
        "kafka_manager.db".to_string()
    } else {
        // 生产模式：将数据库放在用户的系统特定应用数据目录
        let app_name = "Kafka Manager";
        let db_filename = "kafka_manager.db";

        // 使用 dirs crate 获取正确的系统特定目录
        let app_data_dir = if cfg!(target_os = "windows") {
            // Windows: C:\\Users\\Username\\AppData\\Roaming\\Kafka Manager
            dirs::data_local_dir()
                .map(|d| d.join(app_name))
                .or_else(|| dirs::home_dir().map(|d| d.join("AppData").join("Roaming").join(app_name)))
        } else if cfg!(target_os = "macos") {
            // macOS: ~/Library/Application Support/Kafka Manager
            dirs::home_dir()
                .map(|d| d.join("Library/Application Support").join(app_name))
        } else {
            // Linux/Other: ~/.local/share/Kafka Manager 或 ~/.kafka-manager
            dirs::data_local_dir()
                .map(|d| d.join(app_name))
                .or_else(|| dirs::home_dir().map(|d| d.join(".local/share").join(app_name)))
                .or_else(|| dirs::home_dir().map(|d| d.join(".kafka-manager")))
        };

        if let Some(app_data_dir) = app_data_dir {
            // 确保目录存在
            if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                eprintln!("Failed to create app data directory: {}", e);
                tracing::error!("Failed to create app data directory: {}", e);
                // 回退到可执行文件所在目录
                let exe_dir = std::env::current_exe()?
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                exe_dir.join(db_filename).to_string_lossy().to_string()
            } else {
                tracing::info!("Using database path: {:?}", app_data_dir.join(db_filename));
                app_data_dir.join(db_filename).to_string_lossy().to_string()
            }
        } else {
            // 回退到可执行文件所在目录
            let exe_dir = std::env::current_exe()?
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            let fallback_path = exe_dir.join(db_filename);
            tracing::warn!("Failed to get home directory, using fallback: {:?}", fallback_path);
            fallback_path.to_string_lossy().to_string()
        }
    };
    let pool = DbPool::new(&db_path).await?;

    // 初始化数据库表
    pool.init().await?;

    // 创建 Kafka 客户端管理器（使用 ArcSwap 实现无锁读取）
    let clients = Arc::new(ArcSwap::new(Arc::new(KafkaClients::new(&config.clusters)?)));

    // 创建 Kafka 连接池
    let kafka_pools = ClusterPools::new();
    kafka_pools.init(&config.clusters, &config.pool).await?;

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
            if let Err(e) = start_backend(ready_tx.clone()).await {
                tracing::error!("Failed to start backend: {}", e);
                eprintln!("[Backend] Failed to start backend: {}", e);
                // 确保通知前端启动失败
                let _ = ready_tx.send(false);
            }
        });
    });

    // 等待后端服务器启动信号
    let backend_ready = match ready_rx.recv_timeout(Duration::from_secs(30)) {
        Ok(ready) => ready,
        Err(_) => {
            tracing::warn!("Backend server startup timed out after 30 seconds");
            eprintln!("[Backend] Startup timed out after 30 seconds");
            false
        }
    };

    if backend_ready {
        tracing::info!("Backend server is ready, starting Tauri application...");
        eprintln!("[Backend] Backend server is ready, starting Tauri application...");
    } else {
        tracing::warn!("Backend server failed to start or timed out, starting Tauri application anyway...");
        eprintln!("[Backend] Backend server failed to start or timed out, starting Tauri application anyway...");
    }

    // 启动 Tauri 应用
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![greet, get_app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
