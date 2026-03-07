#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;
use std::sync::Arc;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use arc_swap::ArcSwap;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};

// 引用主项目的 kafka-manager-api crate
use kafka_manager_api::{
    Config, DbPool, KafkaClients, AuthMiddleware, ClusterPools,
    MetadataCache, TaskStore, HealthChecker, HealthCheckConfig,
    AppState, create_router,
};
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

/// 简单的日志函数，确保在 Windows 上也能看到输出
fn log(msg: &str) {
    eprintln!("[KAFKA-MANAGER] {}", msg);
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
    let task_store = TaskStore::new();
    let health_checker = HealthChecker::new(HealthCheckConfig::default());
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
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![greet, get_app_version])
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
