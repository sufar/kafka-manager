#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use std::fs::OpenOptions;

use arc_swap::ArcSwap;
use axum::extract::DefaultBodyLimit;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, compression::CompressionLayer};

// 引用主项目的 kafka-manager-api crate
use kafka_manager_api::{
    Config, DbPool, KafkaClients, ClusterPools,
    RefreshState, ImportExportLock,
    AppState, create_router,
};
use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem};

/// Windows 开机自启动：注册表路径
#[cfg(target_os = "windows")]
const RUN_REGISTRY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Windows 开机自启动：设置启动项（内部函数）
#[cfg(target_os = "windows")]
fn set_auto_launch(enable: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu.create_subkey(RUN_REGISTRY_PATH)
        .map_err(|e| format!("Failed to open Run registry: {}", e))?;

    if enable {
        // 获取当前可执行文件路径
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        let exe_str = exe_path.to_string_lossy().to_string();
        run.set_value("KafkaManager", &exe_str)
            .map_err(|e| format!("Failed to set registry value: {}", e))?;
    } else {
        // 删除启动项
        let _ = run.delete_value("KafkaManager");
    }
    Ok(())
}

/// 设置系统托盘图标
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

    let mut builder = tauri::tray::TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Kafka Manager")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.webview_windows().values().next() {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    log("Quit menu clicked, exiting app");
                    app.exit(0);
                }
                _ => {}
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    log("System tray created");
    Ok(())
}

/// 后台下载状态
#[derive(Clone, Debug, serde::Serialize)]
pub struct DownloadState {
    pub is_downloading: bool,
    pub downloaded: u64,
    pub total: u64,
    pub download_url: String,
    pub filename: String,
    pub download_path: Option<PathBuf>,
    #[serde(skip)]
    pub cancel_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            is_downloading: false,
            downloaded: 0,
            total: 0,
            download_url: String::new(),
            filename: String::new(),
            download_path: None,
            cancel_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

/// 简单的日志函数，通过 tracing 写入日志文件
fn log(msg: &str) {
    eprintln!("[KAFKA-MANAGER] {}", msg);
    tracing::info!("[tauri] {msg}");
}

/// 清理日志文件，只保留指定天数内的日志
fn cleanup_log(max_days: u32) {
    use chrono::NaiveDateTime;

    let log_path = kafka_manager_api::utils::app_log_path();

    if !log_path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let cutoff = chrono::Local::now()
        .checked_sub_days(chrono::Days::new(max_days as u64))
        .map(|dt| dt.naive_local())
        .unwrap_or_else(|| chrono::Local::now().naive_local());

    let kept_lines: Vec<&str> = content.lines().filter(|line| {
        if let Some(ts_start) = line.find('[') {
            if let Some(ts_end) = line.find(']') {
                if ts_end > ts_start {
                    let ts_str = &line[ts_start + 1..ts_end];
                    if let Ok(line_time) = NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S") {
                        return line_time >= cutoff;
                    }
                }
            }
        }
        true
    }).collect();

    let _ = std::fs::write(&log_path, kept_lines.join("\n"));
    log(&format!("Log cleanup: kept {} lines, removed {} lines",
        kept_lines.len(),
        content.lines().count().saturating_sub(kept_lines.len())));
}

/// 启动定时日志清理任务（每小时检查，清理3天前的日志）
fn spawn_periodic_log_cleanup() {
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(Duration::from_secs(3600));
            cleanup_log(3);
        }
    });
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

    #[cfg(target_os = "windows")]
    if !cfg!(debug_assertions) {
        // 清理上次更新遗留的旧 exe（进程已退出，文件锁释放，可以删除）
        let old_exe = exe_path.with_extension("exe.old");
        if old_exe.exists() {
            if let Err(e) = std::fs::remove_file(&old_exe) {
                log(&format!("Warning: could not remove old exe from previous update: {}", e));
            } else {
                log(&format!("Cleaned up old exe: {:?}", old_exe));
            }
        }
    }

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

    // 尝试从 exe 目录读取 config.toml（可选）
    let config = if cfg!(debug_assertions) {
        match Config::load("config.toml") {
            Ok(cfg) => { log("Config loaded successfully"); cfg }
            Err(_) => { log("Config not found, using default"); Config::default() }
        }
    } else {
        let config_path = exe_dir.join("config.toml");
        if config_path.exists() {
            match Config::load(&config_path) {
                Ok(cfg) => { log("Config loaded successfully"); cfg }
                Err(e) => { log(&format!("Config load error: {}, using default", e)); Config::default() }
            }
        } else {
            log("No config.toml found, using default");
            Config::default()
        }
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

    // 创建空的 Kafka 客户端（不建立连接，先启动应用）
    log("Creating empty Kafka clients (connections will be established in background)...");
    let clients = KafkaClients::default();
    let clients = Arc::new(ArcSwap::new(Arc::new(clients)));

    // 创建空的 Kafka 连接池（懒加载，首次使用时才连接）
    log("Creating empty Kafka connection pools (will be initialized in background)...");
    let kafka_pools = ClusterPools::new();

    // 初始化刷新状态跟踪
    let refresh_state = Arc::new(Mutex::new(RefreshState::default()));

    // 初始化导入导出全局锁
    let import_export_lock = Arc::new(Mutex::new(ImportExportLock::default()));

    // 构建应用状态
    let state = AppState {
        db: pool,
        clients: clients.clone(),
        config: config.clone(),
        pools: kafka_pools.clone(),
        refresh_state,
        import_export_lock,
    };

    // 初始化 tracing 日志（统一通过 tracing 写入日志文件）
    let log_path_tauri = kafka_manager_api::utils::app_log_path();

    // 确保日志目录存在
    if let Some(parent) = log_path_tauri.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(log_file) = OpenOptions::new().create(true).append(true).open(&log_path_tauri) {
        let (non_blocking, guard) = tracing_appender::non_blocking(log_file);
        // 将 guard 存入 static，防止被 drop 导致日志丢失
        static LOG_GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> = std::sync::OnceLock::new();
        LOG_GUARD.set(guard).ok();

        let _ = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                "%Y-%m-%d %H:%M:%S".to_string()
            ))
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info,rdkafka=warn".into())
            )
            .try_init();
    }

    // 自定义 TraceLayer：请求和响应都在 INFO 级别记录
    let trace_layer = tower_http::trace::TraceLayer::new_for_http()
        .on_request(|req: &axum::http::Request<axum::body::Body>, _span: &tracing::Span| {
            tracing::info!("[http] {} {}", req.method(), req.uri().path());
        })
        .on_response(|res: &axum::http::Response<_>, _latency: Duration, _span: &tracing::Span| {
            tracing::info!("[http] response {} ({:?})", res.status(), _latency);
        })
        .on_failure(|_error: tower_http::classify::ServerErrorsFailureClass, _latency: Duration, _span: &tracing::Span| {
            tracing::warn!("[http] failure ({:?})", _latency);
        });

    // 创建路由
    let app = create_router(state)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        .layer(trace_layer)
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

    let socket = match addr.ip() {
        std::net::IpAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
        std::net::IpAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
    };
    let socket = match socket {
        Ok(s) => s,
        Err(e) => {
            log(&format!("FATAL: Failed to create socket: {}", e));
            let _ = ready_tx.send(false);
            return;
        }
    };
    if let Err(e) = socket.set_reuseaddr(true) {
        log(&format!("Warning: Failed to set SO_REUSEADDR: {}", e));
    }
    if let Err(e) = socket.bind(addr) {
        log(&format!("FATAL: Failed to bind: {}", e));
        let _ = ready_tx.send(false);
        return;
    }
    let listener = match socket.listen(1024) {
        Ok(l) => {
            log(&format!("Successfully bound to {}", addr));
            l
        }
        Err(e) => {
            log(&format!("FATAL: Failed to listen: {}", e));
            let _ = ready_tx.send(false);
            return;
        }
    };

    // 通知前端后端已启动（Kafka 连接在后台建立中）
    log("=========================================");
    log("SERVER READY - Starting HTTP service");
    log("=========================================");

    if let Err(e) = ready_tx.send(true) {
        log(&format!("Warning: Failed to send ready signal: {:?}", e));
    }

    // 启动服务器（在后台任务中阻塞）
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log(&format!("Server error: {}", e));
        }
    });

    // 后台异步建立 Kafka 连接
    let cluster_count = config.clusters.len();
    if cluster_count > 0 {
        let clients_arc = clients.clone();
        let pools_arc = kafka_pools.clone();
        let clients_config = config.clusters.clone();
        let pool_config = config.pool.clone();

        tokio::spawn(async move {
            log(&format!("Background: Creating Kafka clients for {cluster_count} cluster(s)..."));

            // 并发创建 Kafka 客户端
            let clients_config_clone = clients_config.clone();
            let clients_result = tokio::task::spawn_blocking(move || {
                KafkaClients::new(&clients_config_clone)
            }).await;

            match clients_result {
                Ok(Ok(new_clients)) => {
                    log(&format!("Background: Kafka clients created OK ({cluster_count} cluster(s))"));
                    clients_arc.store(Arc::new(new_clients));
                }
                Ok(Err(e)) => {
                    log(&format!("Background: Failed to create Kafka clients: {}", e));
                }
                Err(e) => {
                    log(&format!("Background: Kafka client creation panicked: {}", e));
                }
            }

            // 初始化 Kafka 连接池
            log("Background: Initializing Kafka connection pools...");
            if let Err(e) = pools_arc.init(&clients_config, &pool_config).await {
                log(&format!("Background: Failed to init Kafka pools: {}", e));
            } else {
                log(&format!("Background: Kafka pools initialized OK ({cluster_count} cluster(s))"));
            }
        });
    }

    // 等待服务器结束
    let _ = server_handle.await;
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// 获取日志内容（只读最后 5000 行，避免大文件内存爆炸）
#[tauri::command]
fn get_app_logs() -> Result<String, String> {
    let log_path = kafka_manager_api::utils::app_log_path();

    // 用 BufReader + seek 从文件末尾往前读，只取最后 N 行
    let max_lines = 5000;
    let max_bytes = 5 * 1024 * 1024; // 最多读 5MB

    let file = match std::fs::File::open(&log_path) {
        Ok(f) => f,
        Err(_) => return Ok(String::new()),
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return Ok(String::new()),
    };

    let file_size = metadata.len();
    if file_size == 0 {
        return Ok(String::new());
    }

    // 从文件末尾往前读 max_bytes 或整个文件
    let read_from = if file_size > max_bytes {
        file_size - max_bytes
    } else {
        0
    };

    use std::io::{BufRead, BufReader, Seek};
    let mut reader = BufReader::new(file);
    reader.seek(std::io::SeekFrom::Start(read_from)).ok();

    let mut lines: Vec<String> = Vec::with_capacity(1000);
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        match reader.read_line(&mut line_buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                lines.push(line_buf.clone());
                if lines.len() >= max_lines {
                    // 已经超过最大行数，丢弃最旧的行
                    lines.remove(0);
                }
            }
            Err(_) => break,
        }
    }

    Ok(lines.join(""))
}

/// 清除日志
#[tauri::command]
fn clear_app_logs() -> Result<(), String> {
    let log_path = kafka_manager_api::utils::app_log_path();

    std::fs::write(&log_path, "")
        .map_err(|e| format!("清除日志失败：{}", e))
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 更新检查结果
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UpdateResult {
    pub available: bool,
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
    /// 是否为绿色免安装版（便携版）
    pub is_portable: bool,
    /// 便携版更新下载 URL（仅当 is_portable=true 时有效）
    pub portable_download_url: Option<String>,
}

/// 检查更新 - 直接使用 GitHub API
#[tauri::command]
async fn check_for_updates(_app: tauri::AppHandle) -> Result<UpdateResult, String> {
    do_check_updates().await
}

/// 后台自动下载更新（静默，不安装）
/// 每小时调用一次，有新版本时自动下载安装包到缓存目录
async fn auto_download_update(app: &tauri::AppHandle) {
    // 开发模式下跳过
    if cfg!(debug_assertions) {
        return;
    }

    log("Auto-download: checking for updates...");
    let update_result = match do_check_updates().await {
        Ok(r) => r,
        Err(e) => {
            log(&format!("Auto-download: check failed, {}", e));
            return;
        }
    };

    if !update_result.available {
        log("Auto-download: no updates available");
        return;
    }

    log(&format!("Auto-download: update available v{}, starting download", update_result.version));

    // 原子地检查并设置下载状态，防止与手动下载并发
    let can_download = if let Some(state) = app.try_state::<Arc<Mutex<DownloadState>>>() {
        if let Ok(mut guard) = state.lock() {
            if guard.is_downloading {
                log("Auto-download: already downloading, skipping");
                false
            } else {
                guard.cancel_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                guard.is_downloading = true;
                true
            }
        } else {
            true // 锁获取失败，继续下载
        }
    } else {
        true // 状态不存在，继续下载
    };

    if !can_download {
        return;
    }

    // 获取缓存目录
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("kafka-manager"))
        .unwrap_or_else(|| std::env::temp_dir().join("kafka-manager-cache"));

    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        log(&format!("Auto-download: failed to create cache dir: {}", e));
        return;
    }

    // 从 GitHub API 获取最新版本信息以确定下载 URL
    let api_url = "https://api.github.com/repos/sufar/kafka-manager/releases/latest";
    let client = match reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log(&format!("Auto-download: failed to create client: {}", e));
            return;
        }
    };

    let response = match client.get(api_url).header("User-Agent", "kafka-manager").send().await {
        Ok(r) => r,
        Err(e) => {
            log(&format!("Auto-download: failed to fetch release info: {}", e));
            return;
        }
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(e) => {
            log(&format!("Auto-download: failed to parse response: {}", e));
            return;
        }
    };

    let tag_name = json["tag_name"].as_str().unwrap_or("");
    if tag_name.is_empty() {
        log("Auto-download: empty tag_name, skipping");
        return;
    }

    // 根据平台确定文件
    let target = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "darwin-aarch64"
    } else if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else if cfg!(target_os = "linux") {
        "linux-x86_64"
    } else if cfg!(target_os = "windows") {
        "windows-x86_64"
    } else {
        return;
    };

    let portable = is_portable_mode();
    let assets = match json["assets"].as_array() {
        Some(a) => a,
        None => {
            log("Auto-download: no assets found");
            return;
        }
    };

    let (download_url, filename) = if portable {
        assets.iter().find_map(|asset| {
            let name = asset["name"].as_str()?;
            if name.contains("Portable") || name.contains("portable") || name.contains(".zip") {
                return Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()));
            }
            None
        }).or_else(|| {
            assets.iter().find_map(|asset| {
                let name = asset["name"].as_str()?;
                if name.ends_with(".exe") {
                    Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                } else { None }
            })
        }).unwrap_or_else(|| {
            log("Auto-download: no suitable asset found for portable mode");
            (String::new(), String::new())
        })
    } else {
        assets.iter().find_map(|asset| {
            let name = asset["name"].as_str()?;
            if name.contains(target) {
                return Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()));
            }
            None
        }).or_else(|| {
            assets.iter().find_map(|asset| {
                let name = asset["name"].as_str()?;
                if cfg!(target_os = "macos") && name.ends_with(".dmg") {
                    Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                } else if cfg!(target_os = "linux") && (name.ends_with(".AppImage") || name.ends_with(".deb")) {
                    Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                } else if cfg!(target_os = "windows") && (name.ends_with(".msi") || name.ends_with(".exe")) {
                    Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                } else { None }
            })
        }).unwrap_or_else(|| {
            log(&format!("Auto-download: no suitable asset found for {}", target));
            (String::new(), String::new())
        })
    };

    if filename.is_empty() {
        return;
    }

    let download_path = cache_dir.join(&filename);

    // 如果文件已存在，后续会通过 download_with_resume_background 自动续传

    // 获取总大小
    let head_response = match client.get(&download_url).header("User-Agent", "kafka-manager").send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(_) | Err(_) => {
            log("Auto-download: HEAD request failed");
            return;
        }
    };

    let total_size = head_response.content_length().unwrap_or(0);

    // 检查是否已完整下载
    if let Ok(meta) = std::fs::metadata(&download_path) {
        if meta.len() == total_size && total_size > 0 {
            log(&format!("Auto-download: file already downloaded ({} bytes), skipping", meta.len()));
            if let Some(state) = app.try_state::<Arc<Mutex<DownloadState>>>() {
                if let Ok(mut guard) = state.lock() {
                    guard.is_downloading = false;
                }
            }
            return;
        }
    }

    log(&format!("Auto-download: downloading {} ({})", filename, format_size(total_size)));

    // 更新下载状态详情（is_downloading 已在顶部原子设置）
    if let Some(state) = app.try_state::<Arc<Mutex<DownloadState>>>() {
        if let Ok(mut guard) = state.lock() {
            guard.downloaded = 0;
            guard.total = total_size;
            guard.download_url = download_url.clone();
            guard.filename = filename.clone();
            guard.download_path = Some(download_path.clone());
        }
    }

    // 执行下载
    let download_client = match reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log(&format!("Auto-download: failed to create download client: {}", e));
            if let Some(state) = app.try_state::<Arc<Mutex<DownloadState>>>() {
                if let Ok(mut guard) = state.lock() {
                    guard.is_downloading = false;
                }
            }
            return;
        }
    };

    let download_state = match app.try_state::<Arc<Mutex<DownloadState>>>() {
        Some(s) => s.inner().clone(),
        None => {
            log("Auto-download: DownloadState not managed");
            return;
        }
    };

    match download_with_resume_background(
        &download_client,
        &download_url,
        &download_path,
        download_state.clone(),
        total_size,
    ).await {
        Ok(skipped) => {
            if let Ok(mut guard) = download_state.lock() {
                guard.is_downloading = false;
            }
            if skipped {
                log("Auto-download: using cached file");
            } else {
                log(&format!("Auto-download: {} downloaded successfully", filename));
            }
        }
        Err(e) => {
            log(&format!("Auto-download: download failed: {}", e));
            if let Ok(mut guard) = download_state.lock() {
                guard.is_downloading = false;
            }
        }
    }
}

/// 后台检查更新（启动时自动执行），有更新时通知前端
async fn auto_check_updates(app: &tauri::AppHandle) {
    log("Auto-checking for updates...");
    match do_check_updates().await {
        Ok(result) => {
            if result.available {
                log(&format!("Auto-check: update available v{}", result.version));
                // 通知前端有新版本
                let _ = app.emit("update-available", &result);
            } else {
                log("Auto-check: no updates available");
            }
        }
        Err(e) => {
            // 静默忽略网络错误、403 等
            log(&format!("Auto-check failed: {}", e));
        }
    }
}

/// 核心更新检查逻辑
async fn do_check_updates() -> Result<UpdateResult, String> {
    log("=========================================");
    log("Checking for updates via GitHub API...");

    let current_version = env!("CARGO_PKG_VERSION");
    log(&format!("Current version: {}", current_version));

    // 在开发模式下跳过更新检查
    if cfg!(debug_assertions) {
        log("Skipping update check in debug mode");
        return Ok(UpdateResult {
            available: false,
            version: current_version.to_string(),
            notes: None,
            date: None,
            is_portable: false,
            portable_download_url: None,
        });
    }

    // 直接请求 GitHub Releases API
    let api_url = "https://api.github.com/repos/sufar/kafka-manager/releases/latest";
    log(&format!("Fetching: {}", api_url));

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    let response = client
        .get(api_url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|e| {
            log(&format!("请求失败：{}", e));
            format!("网络错误：{}", e)
        })?;

    log(&format!("HTTP status: {}", response.status()));

    // 检查 HTTP 状态码，403 时返回特殊错误信息
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        log("GitHub API rate limit exceeded (403 Forbidden)");
        return Err("403 Forbidden".to_string());
    }

    if !response.status().is_success() {
        log(&format!("GitHub API error: HTTP {}", response.status()));
        return Ok(UpdateResult {
            available: false,
            version: current_version.to_string(),
            notes: None,
            date: None,
            is_portable: false,
            portable_download_url: None,
        });
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    // 提取版本信息
    let tag_name = json["tag_name"].as_str().unwrap_or("");
    log(&format!("Latest tag: {}", tag_name));

    // 如果 tag_name 为空，说明响应格式异常，返回无更新
    if tag_name.is_empty() {
        log("Warning: tag_name is empty, response may be malformed");
        return Ok(UpdateResult {
            available: false,
            version: current_version.to_string(),
            notes: None,
            date: None,
            is_portable: is_portable_mode(),
            portable_download_url: None,
        });
    }

    // 提取发布说明，并处理重复内容
    let body_str = json["body"].as_str().unwrap_or("").to_string();
    let notes = if body_str.is_empty() {
        None
    } else {
        // 检查是否有重复内容（GitHub 有时会重复）
        let lines: Vec<&str> = body_str.lines().collect();
        let mut unique_lines: Vec<&str> = Vec::new();
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

        for line in lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !seen.contains(trimmed) {
                seen.insert(trimmed);
                unique_lines.push(line);
            } else if trimmed.is_empty() && !unique_lines.is_empty() && !unique_lines.last().map_or(false, |l| l.trim().is_empty()) {
                // 保留空行，但不保留连续空行
                unique_lines.push(line);
            }
        }

        Some(unique_lines.join("\n"))
    };

    // 提取发布时间
    let published_at = json["published_at"].as_str().unwrap_or("");

    // 去掉 v 前缀进行比较
    let remote_version = tag_name.strip_prefix('v').unwrap_or(tag_name);
    log(&format!("Remote version (normalized): {}", remote_version));

    // 使用语义化版本比较
    let has_update = if let (Ok(remote_ver), Ok(current_ver)) = (
        semver::Version::parse(remote_version),
        semver::Version::parse(current_version),
    ) {
        remote_ver > current_ver
    } else {
        // 如果版本解析失败，回退到字符串比较（只有远程版本字符串 > 当前版本字符串才认为有更新）
        // 这种回退逻辑较为粗糙，仅在语义化版本解析失败时使用
        remote_version > current_version
    };
    log(&format!("Has update: {}", has_update));

    if has_update {
        log(&format!("Update available: {} -> {}", current_version, remote_version));
    } else {
        log("No updates available");
    }

    Ok(UpdateResult {
        available: has_update,
        version: remote_version.to_string(),
        notes: notes.clone(),
        date: if published_at.is_empty() { None } else { Some(published_at.to_string()) },
        is_portable: is_portable_mode(),
        portable_download_url: if is_portable_mode() {
            // 从 assets 中查找便携版下载链接
            find_portable_download_url(&json)
        } else {
            None
        },
    })
}

/// 检测是否为绿色免安装版
#[allow(unreachable_code)]
fn is_portable_mode() -> bool {
    // macOS: 从 DMG 安装的应用通常在 /Applications 目录
    // macOS 上不存在绿色版概念，统一走 DMG 更新流程
    #[cfg(target_os = "macos")]
    {
        return false;
    }

    // 以下逻辑仅用于 Windows/Linux 等其他平台，保持不变
    #[cfg(not(target_os = "macos"))]
    {
        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return true,
        };

        // 检查是否在标准安装目录（Program Files, AppData 等）
        let path_str = exe_path.to_string_lossy().to_lowercase();
        let standard_locations = [
            "program files",
            "program files (x86)",
            "appdata\\local",
            "appdata\\roaming",
        ];

        for loc in &standard_locations {
            if path_str.contains(loc) {
                return false;
            }
        }

        return true;
    }

    unreachable!()
}

/// 从 GitHub release JSON 中提取便携版下载链接
fn find_portable_download_url(json: &serde_json::Value) -> Option<String> {
    let assets = json["assets"].as_array()?;
    for asset in assets {
        let name = asset["name"].as_str()?;
        // 优先匹配 Portable.zip
        if name.contains("Portable") || name.contains("portable") {
            return asset["browser_download_url"].as_str().map(String::from);
        }
    }
    // 回退：返回 .exe 下载链接（NSIS 单文件安装器）
    for asset in assets {
        let name = asset["name"].as_str()?;
        if name.ends_with(".exe") {
            return asset["browser_download_url"].as_str().map(String::from);
        }
    }
    None
}

/// 获取下载状态
#[tauri::command]
fn get_download_status(app: tauri::AppHandle) -> Result<DownloadState, String> {
    match app.try_state::<Arc<Mutex<DownloadState>>>() {
        Some(state) => {
            let guard = state.lock().map_err(|e| format!("Lock error: {}", e))?;
            let mut result = guard.clone();

            // 如果状态中没有进度，但缓存目录中有当前下载的文件，检查文件大小
            // 注意：只检查当前正在下载的文件名，避免匹配到旧版本的缓存文件
            if result.downloaded == 0 && result.total == 0 && result.is_downloading && !result.filename.is_empty() {
                let cache_dir = dirs::cache_dir()
                    .map(|d| d.join("kafka-manager"))
                    .unwrap_or_else(|| std::env::temp_dir().join("kafka-manager-cache"));

                let target_path = cache_dir.join(&result.filename);
                if let Ok(meta) = std::fs::metadata(&target_path) {
                    let file_size = meta.len();
                    if file_size > 0 {
                        result.downloaded = file_size;
                        result.total = std::cmp::max(result.total, file_size);
                        result.download_path = Some(target_path);
                    }
                }
            }

            Ok(result)
        }
        None => {
            Ok(DownloadState::default())
        }
    }
}

/// 清除下载状态
#[tauri::command]
fn clear_download_status(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<Arc<Mutex<DownloadState>>>() {
        let mut guard = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.cancel_requested.store(true, std::sync::atomic::Ordering::Relaxed);
        guard.is_downloading = false;
        guard.downloaded = 0;
        guard.total = 0;
    }
    // 注意：不清除缓存的下载文件，保留以便断点续传
    // install_update() 会检查现有文件并自动续传
    Ok(())
}

/// 设置开机自启动（仅 Windows 生效）
#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
#[tauri::command]
fn set_auto_launch(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        let exe_str = exe_path.to_string_lossy().to_string();

        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run, _) = hkcu.create_subkey(RUN_REGISTRY_PATH)
            .map_err(|e| format!("Failed to open Run registry: {}", e))?;

        if enabled {
            run.set_value("KafkaManager", &exe_str)
                .map_err(|e| format!("Failed to set registry value: {}", e))?;
        } else {
            let _ = run.delete_value("KafkaManager");
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
    }
    Ok(())
}

/// 检测是否为 Windows 平台
#[tauri::command]
fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// 获取开机自启动状态（仅 Windows 生效）
#[tauri::command]
fn get_auto_launch() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(run) = hkcu.open_subkey(RUN_REGISTRY_PATH) {
            let result: Result<String, _> = run.get_value("KafkaManager");
            return Ok(result.is_ok());
        }
    }
    Ok(false)
}

/// 后台下载函数（使用状态对象而不是 Channel）
async fn download_with_resume_background(
    client: &reqwest::Client,
    url: &str,
    download_path: &std::path::Path,
    state: Arc<Mutex<DownloadState>>,
    total_size: u64,
) -> Result<bool, String> {
    use futures::StreamExt;
    use std::io::Write;

    // 检查本地文件
    let mut existing_size = if download_path.exists() {
        std::fs::metadata(download_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    // 发送 HEAD 请求检查文件信息和是否支持断点续传
    let head_response = client
        .get(url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|_| "network_error".to_string())?;

    let remote_size = head_response.content_length().unwrap_or(total_size);

    // 检查是否支持断点续传
    let supports_resume = head_response.headers()
        .get("accept-ranges")
        .map_or(false, |v| v.as_bytes() == b"bytes");

    // 验证缓存文件：检查远程 ETag 是否与本地缓存一致
    // 如果 ETag 不同，说明远程文件已更新（例如新版本发布），需要重新下载
    let cached_etag_path = download_path.with_extension("etag");
    let cached_etag = cached_etag_path.exists()
        .then(|| std::fs::read_to_string(&cached_etag_path).ok())
        .flatten();
    let remote_etag = head_response.headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ETag 比较结果：
    // - 有缓存 ETag 且匹配：文件有效，可能跳过
    // - 有缓存 ETag 但不匹配：远程文件已变，删除旧文件
    // - 没有缓存 ETag：可能是中断的下载，保留文件尝试续传
    let etag_mismatch = if let (Some(cached), Some(remote)) = (&cached_etag, &remote_etag) {
        cached.as_str() != remote.as_str()
    } else {
        false // 没有缓存 ETag 视为不确定，不删除，留给后续逻辑处理
    };

    // 如果 ETag 明确不匹配（说明远程文件已变化），清理旧缓存
    if existing_size > 0 && etag_mismatch {
        log(&format!("ETag mismatch, remote file has changed, removing cached file ({} bytes)", existing_size));
        std::fs::remove_file(download_path).ok();
        std::fs::remove_file(&cached_etag_path).ok();
        existing_size = 0;
    } else if etag_mismatch && existing_size > 0 && existing_size >= remote_size && remote_size > 0 {
        // ETag 不匹配且文件大小超过远程（确定是旧版本文件），清理
        log(&format!("Cached file size exceeds remote, removing ({} bytes)", existing_size));
        std::fs::remove_file(download_path).ok();
        std::fs::remove_file(&cached_etag_path).ok();
        existing_size = 0;
    }

    // 检查文件是否已完整下载，避免重复下载
    if existing_size > 0 && remote_size > 0 && existing_size == remote_size {
        log(&format!("File already fully downloaded ({} bytes), skipping download", existing_size));
        return Ok(true);
    }

    // 如果本地文件大于远程文件（可能是之前断点续传bug导致的追加文件），清理后重新下载
    if existing_size > 0 && remote_size > 0 && existing_size > remote_size {
        log(&format!("Local file ({} bytes) exceeds remote ({} bytes), removing corrupted file", existing_size, remote_size));
        std::fs::remove_file(download_path).ok();
        std::fs::remove_file(&cached_etag_path).ok();
        existing_size = 0;
    }

    let start_byte = if supports_resume && existing_size > 0 && existing_size < remote_size {
        log(&format!("Resuming download from byte {} (remote: {})", existing_size, remote_size));
        existing_size
    } else {
        if existing_size > 0 {
            if !supports_resume {
                log("Server doesn't support resume, restarting download");
            }
            std::fs::remove_file(download_path).ok();
        }
        0
    };

    let mut request = client.get(url).header("User-Agent", "kafka-manager");
    if start_byte > 0 {
        request = request.header("Range", format!("bytes={}-", start_byte));
    }

    let response = request.send().await.map_err(|e| {
        let mut msg = e.to_string();
        // 隐藏 URL 信息
        if let Some(pos) = msg.find("url (") {
            msg = msg[..pos].trim_end().to_string();
        }
        format!("下载失败：{}", msg)
    })?;

    let final_response = if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        log("Received 416, removing and retrying");
        std::fs::remove_file(download_path).ok();
        let retry = client.get(url).header("User-Agent", "kafka-manager").send().await
            .map_err(|e| {
                let mut msg = e.to_string();
                if let Some(pos) = msg.find("url (") {
                    msg = msg[..pos].trim_end().to_string();
                }
                format!("下载失败：{}", msg)
            })?;
        if !retry.status().is_success() {
            return Err(format!("下载失败：HTTP {}", retry.status()));
        }
        retry
    } else {
        if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(format!("下载失败：HTTP {}", response.status()));
        }
        response
    };

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(download_path)
        .map_err(|e| format!("打开文件失败：{}", e))?;

    // 如果请求了 Range 但服务器返回 200（不是 PARTIAL_CONTENT），说明服务器忽略了 Range
    // 此时需要删除旧文件，从头开始写入
    if start_byte > 0 && final_response.status() == reqwest::StatusCode::OK {
        log("Server ignored Range header, removing partial file and starting fresh");
        std::fs::remove_file(download_path).ok();
        std::fs::remove_file(&download_path.with_extension("etag")).ok();
        file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(download_path)
            .map_err(|e| format!("打开文件失败：{}", e))?;
    }

    let mut downloaded: u64 = 0;
    let mut stream = final_response.bytes_stream();

    // 用于每 10 秒打印进度
    let mut last_progress_log = std::time::Instant::now();
    let mut last_logged_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        // 检查取消请求
        if state.lock().map_or(false, |g| g.cancel_requested.load(std::sync::atomic::Ordering::Relaxed)) {
            log("Download cancelled");
            return Err("下载已取消".to_string());
        }

        let chunk = chunk.map_err(|_e| "network_error".to_string())?;
        file.write_all(&chunk).map_err(|e| format!("写入文件失败：{}", e))?;
        downloaded += chunk.len() as u64;

        // 更新状态
        {
            let mut guard = state.lock().map_err(|e| format!("Lock error: {}", e))?;
            guard.downloaded = existing_size + downloaded;
        }

        // 每 10 秒打印一次下载进度
        if last_progress_log.elapsed().as_secs() >= 10 {
            let total = if total_size > 0 { total_size } else { existing_size + downloaded };
            let progress = if total > 0 { (existing_size + downloaded) as f64 / total as f64 * 100.0 } else { 0.0 };
            let speed_bytes = downloaded - last_logged_downloaded;
            let speed_mb = speed_bytes as f64 / 1024.0 / 1024.0 / 10.0; // MB/s
            log(&format!("Download progress: {}/{} ({:.1}%), speed: {:.2} MB/s",
                format_size(existing_size + downloaded),
                format_size(total),
                progress,
                speed_mb));
            last_progress_log = std::time::Instant::now();
            last_logged_downloaded = downloaded;
        }
    }

    file.flush().map_err(|e| format!("刷新文件失败：{}", e))?;

    // 保存 ETag 以便下次下载时校验
    if let Some(etag) = &remote_etag {
        let etag_path = download_path.with_extension("etag");
        if let Err(e) = std::fs::write(&etag_path, etag) {
            log(&format!("Warning: failed to save ETag: {}", e));
        }
    }

    log("Download completed");
    Ok(false)
}

/// 格式化文件大小
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 重置下载状态，使前端可以重试
fn reset_download_state(app: &tauri::AppHandle) {
    if let Ok(mut state) = app.state::<Arc<Mutex<DownloadState>>>().lock() {
        state.cancel_requested.store(true, std::sync::atomic::Ordering::Relaxed);
        state.cancel_requested.store(false, std::sync::atomic::Ordering::Relaxed);
        state.is_downloading = false;
    }
}

/// 绿色便携版更新：解压并覆盖当前目录的所有文件
fn install_portable_update(
    app_handle: &tauri::AppHandle,
    zip_path: &std::path::Path,
    cache_dir: &std::path::Path,
) -> Result<(), String> {
    use std::io::Cursor;

    log("Installing portable update...");

    // 读取 zip 文件
    let zip_data = std::fs::read(zip_path)
        .map_err(|e| format!("读取更新文件失败：{}", e))?;

    // 创建临时解压目录
    let extract_dir = cache_dir.join("_portable_update");
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)
            .map_err(|e| format!("清理旧解压目录失败：{}", e))?;
    }
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("创建解压目录失败：{}", e))?;

    // 解压 zip
    let cursor = Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("解压更新包失败：{}", e))?;

    log(&format!("Zip contains {} files", archive.len()));

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("解压文件失败：{}", e))?;
        let outpath = extract_dir.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p).ok();
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("创建文件失败：{}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("写入文件失败：{}", e))?;
        }
    }

    log(&format!("Extracted to {:?}", extract_dir));

    // 找到当前 exe 所在目录
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("获取当前程序路径失败：{}", e))?;
    let current_dir = current_exe.parent().ok_or("无法获取程序所在目录")?;

    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_dialog::DialogExt;

        // 复制所有文件到当前目录（覆盖旧文件）
        fn copy_dir_contents(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
            for entry in std::fs::read_dir(src).map_err(|e| format!("读取目录失败：{}", e))? {
                let entry = entry.map_err(|e| format!("读取条目失败：{}", e))?;
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());

                if src_path.is_dir() {
                    std::fs::create_dir_all(&dst_path).ok();
                    copy_dir_contents(&src_path, &dst_path)?;
                } else {
                    std::fs::copy(&src_path, &dst_path)
                        .map_err(|e| format!("复制文件 {} 失败：{}", src_path.display(), e))?;
                }
            }
            Ok(())
        }

        // macOS 上需要替换整个 .app bundle
        // 当前 exe 路径如: /path/to/Kafka Manager.app/Contents/MacOS/Kafka Manager
        // .app bundle 路径: /path/to/Kafka Manager.app
        let app_bundle = current_dir
            .parent()  // Contents
            .and_then(|p| p.parent())  // Kafka Manager.app
            .ok_or("无法定位 .app bundle")?;

        // 在解压目录中找到 .app bundle
        let mut new_app = None;
        for entry in std::fs::read_dir(&extract_dir).map_err(|e| format!("读取解压目录失败：{}", e))? {
            let entry = entry.map_err(|e| format!("读取条目失败：{}", e))?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "app") {
                new_app = Some(path);
                break;
            }
        }

        if let Some(app_path) = new_app {
            log(&format!("Replacing .app bundle: {:?}", app_bundle));
            // 先删除旧的 .app bundle
            std::fs::remove_dir_all(app_bundle)
                .map_err(|e| format!("删除旧 .app bundle 失败：{}", e))?;
            // 复制新的 .app bundle
            copy_dir_contents(&app_path, app_bundle)?;
            log("Successfully replaced .app bundle");
        } else {
            // 回退：直接覆盖当前目录
            copy_dir_contents(&extract_dir, current_dir)?;
            log(&format!("All files copied to {:?}", current_dir));
        }

        // 重启应用
        app_handle.dialog()
            .message("更新已完成，应用即将重启")
            .title("更新成功")
            .show(|_| {});

        let app_handle_clone = app_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            log("Restarting app for portable update...");
            let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
            let _ = std::process::Command::new(&current_exe).spawn();
            app_handle_clone.exit(0);
        });

        // 清理临时目录
        let extract_dir_clone = extract_dir.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let _ = std::fs::remove_dir_all(&extract_dir_clone);
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        // 先复制除当前 exe 之外的所有文件
        let exe_name = current_exe
            .file_name()
            .map(|n| n.to_string_lossy().to_string());

        fn copy_dir_contents_skip(
            src: &std::path::Path,
            dst: &std::path::Path,
            skip_name: &Option<String>,
        ) -> Result<(), String> {
            for entry in std::fs::read_dir(src).map_err(|e| format!("读取目录失败：{}", e))? {
                let entry = entry.map_err(|e| format!("读取条目失败：{}", e))?;
                let src_path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                // 跳过当前正在运行的 exe
                if skip_name.as_ref().map_or(false, |name| file_name == *name) {
                    continue;
                }

                let dst_path = dst.join(&file_name);

                if src_path.is_dir() {
                    std::fs::create_dir_all(&dst_path).ok();
                    copy_dir_contents_skip(&src_path, &dst_path, skip_name)?;
                } else {
                    std::fs::copy(&src_path, &dst_path)
                        .map_err(|e| format!("复制文件 {} 失败：{}", src_path.display(), e))?;
                }
            }
            Ok(())
        }

        copy_dir_contents_skip(&extract_dir, current_dir, &exe_name)?;
        log(&format!("All files copied to {:?} (except current exe)", current_dir));

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use tauri_plugin_dialog::DialogExt;

            let new_exe = extract_dir.join(current_exe.file_name().unwrap_or_else(|| std::ffi::OsStr::new("kafka-manager.exe")));
            let current_dir = current_exe.parent().ok_or("无法获取程序所在目录")?;
            let old_exe = current_exe.with_extension("exe.old");
            let current_dir_str = current_dir.to_string_lossy().replace('/', "\\");
            let temp_dir_str = extract_dir.to_string_lossy().replace('/', "\\");
            let new_exe_str = new_exe.to_string_lossy().replace('/', "\\");
            let old_exe_str = old_exe.to_string_lossy().replace('/', "\\");

            // 清理上次更新遗留的旧 exe（如果存在）
            if old_exe.exists() {
                if let Err(e) = std::fs::remove_file(&old_exe) {
                    log(&format!("Warning: could not remove old exe from previous update: {}", e));
                } else {
                    log("Removed old exe from previous update");
                }
            }

            // Windows 允许重命名运行中的 exe，重命名后原路径即可用于写入新文件
            let rename_ok = match std::fs::rename(&current_exe, &old_exe) {
                Ok(()) => {
                    log("Renamed current exe to .exe.old");
                    if let Err(e) = std::fs::copy(&new_exe, &current_exe) {
                        log(&format!("Failed to copy new exe: {}, reverting", e));
                        let _ = std::fs::rename(&old_exe, &current_exe);
                        false
                    } else {
                        log("New exe copied successfully");
                        true
                    }
                }
                Err(e) => {
                    log(&format!("Failed to rename current exe: {}", e));
                    false
                }
            };

            // PowerShell 脚本：启动应用
            let ps1_path = cache_dir.join("update_portable.ps1");
            let ps1_content = if rename_ok {
                // 文件已替换，直接启动
                format!(
                    r#"$ErrorActionPreference = 'SilentlyContinue'
Start-Process "{current_dir_str}\kafka-manager.exe"
Start-Sleep -Seconds 2
Remove-Item -Path "{temp_dir_str}" -Recurse -Force -ErrorAction SilentlyContinue
if (Test-Path "{old_exe_str}") {{
    Remove-Item -Path "{old_exe_str}" -Force -ErrorAction SilentlyContinue
}}
Remove-Item -Path $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue
"#,
                )
            } else {
                // 文件未替换，先替换再启动
                format!(
                    r#"$ErrorActionPreference = 'SilentlyContinue'
Start-Sleep -Seconds 2
if (Test-Path "{new_exe_str}") {{
    Copy-Item -Path "{new_exe_str}" -Destination "{current_dir_str}\kafka-manager.exe" -Force -ErrorAction SilentlyContinue
}}
Start-Sleep -Seconds 1
Start-Process "{current_dir_str}\kafka-manager.exe"
Start-Sleep -Seconds 2
Remove-Item -Path "{temp_dir_str}" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue
"#,
                )
            };

            let _ = std::fs::write(&ps1_path, ps1_content);
            std::process::Command::new("powershell.exe")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-File")
                .arg(&ps1_path)
                .creation_flags(0x00000200)
                .spawn()
                .ok();

            log("Spawned PowerShell script to start new process");

            // 短暂等待确保脚本执行
            std::thread::sleep(std::time::Duration::from_millis(1000));

            // 清理临时目录
            let _ = std::fs::remove_dir_all(&extract_dir);

            // 强制退出
            std::process::exit(0);
        }

        #[cfg(not(target_os = "windows"))]
        {
            use tauri_plugin_dialog::DialogExt;

            // Linux/macOS 上同样需要独立进程来替换文件
            let new_exe = extract_dir.join(current_exe.file_name().unwrap_or_else(|| std::ffi::OsStr::new("kafka-manager")));
            let exe_path = current_exe.to_string_lossy();
            let new_exe_path = new_exe.to_string_lossy();
            let temp_dir = extract_dir.to_string_lossy();

            let sh_path = cache_dir.join("update_portable.sh");
            let sh_content = format!(
                r#"#!/bin/sh
sleep 3
if [ -f "{new_exe_path}" ]; then
  cp -f "{new_exe_path}" "{exe_path}"
  if [ $? -eq 0 ]; then
    {exe_path} &
  fi
fi
rm -rf "{temp_dir}"
rm -f "{sh_path}"
"#,
                sh_path = sh_path.to_string_lossy(),
            );

            std::fs::write(&sh_path, sh_content)
                .map_err(|e| format!("创建更新脚本失败：{}", e))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&sh_path).unwrap().permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&sh_path, perms).ok();
            }

            app_handle.dialog()
                .message("文件已准备就绪，应用即将退出并完成更新")
                .title("更新中")
                .show(|_| {});

            // 启动独立的 shell 脚本（detached）
            std::process::Command::new("sh")
                .arg(sh_path.to_string_lossy().as_ref())
                .spawn()
                .map_err(|e| format!("启动更新脚本失败：{}", e))?;

            std::thread::sleep(std::time::Duration::from_secs(1));
            app_handle.exit(0);
        }
    }

    Ok(())
}

/// 下载并安装更新（支持签名验证和断点续传）- 后台线程运行
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    log("Downloading and installing update with signature verification...");

    // 立即检查并设置下载状态，防止并发
    {
        let state = app.state::<Arc<Mutex<DownloadState>>>();
        let mut guard = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        if guard.is_downloading {
            log("Cancelling existing download to start a new one");
            guard.cancel_requested.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        // 立即标记为下载中，阻止后续并发调用
        guard.cancel_requested.store(false, std::sync::atomic::Ordering::Relaxed);
        guard.is_downloading = true;
    }

    // 获取系统缓存目录
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("kafka-manager"))
        .unwrap_or_else(|| std::env::temp_dir().join("kafka-manager-cache"));

    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| { reset_download_state(&app); format!("创建缓存目录失败：{}", e) })?;

    log(&format!("Cache directory: {:?}", cache_dir));

    // 从 GitHub API 获取最新版本信息
    let api_url = "https://api.github.com/repos/sufar/kafka-manager/releases/latest";

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| { reset_download_state(&app); format!("创建 HTTP 客户端失败：{}", e) })?;

    let response = client
        .get(api_url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|_e| { reset_download_state(&app); "network_error".to_string() })?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|_e| { reset_download_state(&app); "network_error".to_string() })?;

    let tag_name = json["tag_name"].as_str().unwrap_or("");
    if tag_name.is_empty() || tag_name == "unknown" {
        reset_download_state(&app);
        return Err("network_error".to_string());
    }
    log(&format!("Latest release: {}", tag_name));

    // 根据平台确定要下载的文件和签名 key
    let (target, _file_ext) = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        ("darwin-aarch64", ".dmg")
    } else if cfg!(target_os = "macos") {
        ("darwin-x86_64", ".dmg")
    } else if cfg!(target_os = "linux") {
        ("linux-x86_64", ".AppImage")
    } else if cfg!(target_os = "windows") {
        ("windows-x86_64", ".msi")
    } else {
        reset_download_state(&app);
        return Err("不支持的平台".to_string());
    };

    log(&format!("Target platform: {}", target));

    // 检测是否为绿色便携版
    let portable = is_portable_mode();
    log(&format!("Portable mode: {}", portable));

    // 从 assets 中找到对应的下载 URL
    let assets = json["assets"].as_array().ok_or_else(|| {
        reset_download_state(&app);
        "找不到下载文件".to_string()
    })?;

    // 根据模式选择不同的下载文件
    let (download_url, filename) = if portable {
        // 便携版：优先找 Portable.zip，其次是 .exe
        assets
            .iter()
            .find_map(|asset| {
                let name = asset["name"].as_str()?;
                if name.contains("Portable") || name.contains("portable") || name.contains(".zip") {
                    return Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()));
                }
                None
            })
            .or_else(|| {
                assets.iter().find_map(|asset| {
                    let name = asset["name"].as_str()?;
                    if name.ends_with(".exe") {
                        Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| {
                reset_download_state(&app);
                "找不到适合当前平台的安装包".to_string()
            })?
    } else {
        // 安装版：原有的逻辑
        // 先精确匹配目标平台+架构（优先），再回退到按扩展名匹配
        assets
            .iter()
            .find_map(|asset| {
                let name = asset["name"].as_str()?;
                // 精确匹配：文件名包含目标平台标识
                if name.contains(target) {
                    return Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()));
                }
                None
            })
            .or_else(|| {
                // 回退：按扩展名匹配
                assets.iter().find_map(|asset| {
                    let name = asset["name"].as_str()?;
                    if cfg!(target_os = "macos") && name.ends_with(".dmg") {
                        Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                    } else if cfg!(target_os = "linux") && (name.ends_with(".AppImage") || name.ends_with(".deb")) {
                        Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                    } else if cfg!(target_os = "windows") && (name.ends_with(".msi") || name.ends_with(".exe")) {
                        Some((asset["browser_download_url"].as_str()?.to_string(), name.to_string()))
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| {
                reset_download_state(&app);
                "找不到适合当前平台的安装包".to_string()
            })?
    };

    log(&format!("Filename: {}", filename));

    // 下载路径
    let download_path = cache_dir.join(&filename);

    // 检查是否有缓存文件
    let existing_size = if download_path.exists() {
        std::fs::metadata(&download_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    // 为下载创建一个独立的 client（无代理、无超时限制，但有连接超时）
    let download_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| { reset_download_state(&app); format!("创建下载客户端失败：{}", e) })?;

    // 获取文件总大小（使用独立 client，不走代理），重试 3 次
    let head_response = {
        let mut attempts = 0;
        loop {
            attempts += 1;
            match download_client
                .get(&download_url)
                .header("User-Agent", "kafka-manager")
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => break resp,
                Ok(_) | Err(_) => {}
            }
            if attempts >= 3 {
                if let Some(state) = app.try_state::<Arc<Mutex<DownloadState>>>() {
                    if let Ok(mut guard) = state.lock() {
                        guard.cancel_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                        guard.is_downloading = false;
                    }
                }
                return Err("network_error".to_string());
            }
            log(&format!("HEAD request failed (attempt {attempts}/3), retrying in 2s..."));
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    };

    let total_size = head_response.content_length().unwrap_or(0);
    log(&format!("Remote file size: {} bytes", total_size));

    // 更新下载状态
    {
        let state = app.state::<Arc<Mutex<DownloadState>>>();
        let mut guard = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.downloaded = existing_size;
        guard.total = total_size;
        guard.download_url = download_url.clone();
        guard.filename = filename.clone();
        guard.download_path = Some(download_path.clone());
        if existing_size > 0 {
            log(&format!("Found existing file: {} bytes, will resume download", existing_size));
        }
    }

    // 克隆状态和 app 用于后台线程
    let state = app.state::<Arc<Mutex<DownloadState>>>().inner().clone();
    let app_handle = app.clone();
    let download_client = download_client.clone();
    let portable = portable;
    let cache_dir = cache_dir.clone();

    // 在独立线程中执行下载，不依赖前端生命周期
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            log("Starting background download in independent thread...");

            // 执行下载（带断点续传，使用状态对象更新进度）
            let skipped = match download_with_resume_background(
                &download_client,
                &download_url,
                &download_path,
                state.clone(),
                total_size,
            ).await {
                Ok(skipped) => skipped,
                Err(e) => {
                    let error_msg = e.clone();
                    log(&format!("Download error: {}", error_msg));
                    {
                        let mut guard = state.lock().expect("Lock error");
                        guard.cancel_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                        guard.is_downloading = false;
                    }
                    // 向前端发送错误事件
                    // 网络错误使用 network_error 标识，前端可以显示翻译后的提示
                    let emit_msg = if error_msg.contains("error sending request")
                        || error_msg.contains("network")
                        || error_msg.contains("timeout")
                        || error_msg.contains("connection")
                    {
                        "network_error"
                    } else {
                        &error_msg
                    };
                    let _ = app_handle.emit("download_error", emit_msg);
                    return;
                }
            };

            // 下载完成，更新状态
            {
                let mut guard = state.lock().expect("Lock error");
                guard.cancel_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                guard.is_downloading = false;
            }

            if skipped {
                log("Using cached file");
            }

            log(&format!("File ready at: {:?}", download_path));

            // 绿色便携版：解压并覆盖当前 exe
            if portable {
                match install_portable_update(&app_handle, &download_path, &cache_dir) {
                    Ok(()) => {
                        log("Portable update completed successfully");
                        let _ = app_handle.emit("update_complete", ());
                        return;
                    }
                    Err(e) => {
                        log(&format!("Portable update failed: {}", e));
                        let mut guard = state.lock().expect("Lock error");
                        guard.cancel_requested.store(false, std::sync::atomic::Ordering::Relaxed);
                        guard.is_downloading = false;
                        let _ = app_handle.emit("download_error", e);
                        return;
                    }
                }
            }

            // 使用系统对话框通知用户
            use tauri_plugin_dialog::DialogExt;

            #[cfg(target_os = "macos")]
            {
                log("Opening DMG file...");
                let dmg_path_str = download_path.to_string_lossy();
                log(&format!("DMG path: {}", dmg_path_str));

                // 检查文件是否存在及其大小
                if download_path.exists() {
                    match std::fs::metadata(&download_path) {
                        Ok(meta) => log(&format!("DMG file exists, size: {} bytes", meta.len())),
                        Err(e) => log(&format!("ERROR: Cannot read DMG metadata: {}", e)),
                    }
                } else {
                    log("ERROR: DMG file does not exist!");
                }

                // 使用 spawn() 立即返回，不阻塞后续对话框
                let _ = std::process::Command::new("open")
                    .arg(&download_path)
                    .spawn();

                // 通知用户
                app_handle.dialog()
                    .message("安装包已挂载，请将 Kafka Manager 拖拽到 Applications 文件夹完成安装。")
                    .title("下载完成")
                    .show(|_| {});

                // 立即退出应用，用户可继续在安装界面拖拽图标安装
                let app_handle_clone = app_handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    log("Exiting app for update installation...");
                    app_handle_clone.exit(0);
                });
            }

            #[cfg(target_os = "windows")]
            {
                log("Opening installer file...");
                let path_str = download_path.to_string_lossy();
                log(&format!("Installer path: {}", path_str));

                if !download_path.exists() {
                    log("ERROR: Installer file does not exist!");
                } else {
                    match std::fs::metadata(&download_path) {
                        Ok(meta) => log(&format!("Installer file exists, size: {} bytes", meta.len())),
                        Err(e) => log(&format!("ERROR: Cannot read file metadata: {}", e)),
                    }
                }

                // 使用 explorer.exe 打开文件，对路径中的空格更友好
                match std::process::Command::new("explorer")
                    .arg(&download_path)
                    .spawn()
                {
                    Ok(_) => log("Explorer launched successfully"),
                    Err(e) => {
                        log(&format!("Failed to open with explorer: {}", e));
                        // 回退到 cmd /c start
                        std::process::Command::new("cmd")
                            .args(["/c", "start", "\"\"", &path_str])
                            .spawn()
                            .ok();
                    }
                }

                app_handle.dialog()
                    .message("安装程序已启动，应用将在 3 秒后退出，请按照提示完成安装")
                    .title("下载完成")
                    .show(|_| {});

                // 延迟退出应用
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    log("Exiting app for update installation...");
                    std::process::exit(0);
                });
            }

            #[cfg(target_os = "linux")]
            {
                if download_path.extension().map_or(false, |ext| ext == "deb") {
                    log("Installing deb package...");
                    std::process::Command::new("gdebi")
                        .arg(&download_path)
                        .spawn()
                        .ok();
                } else {
                    log("Opening AppImage...");
                    std::process::Command::new("chmod")
                        .args(["+x", download_path.to_str().unwrap_or("")])
                        .spawn()
                        .ok();
                    std::process::Command::new(&download_path)
                        .spawn()
                        .ok();
                }

                app_handle.dialog()
                    .message("安装包已打开，应用将在 3 秒后退出，请按照提示完成安装")
                    .title("下载完成")
                    .show(|_| {});

                // 延迟退出应用
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    log("Exiting app for update installation...");
                    std::process::exit(0);
                });
            }

            #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
            {
                app_handle.dialog()
                    .message("下载完成")
                    .title(if skipped { "使用缓存文件" } else { "下载完成" })
                    .show(|_| {});
            }
        });
    });

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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![greet, get_app_version, check_for_updates, install_update, get_app_logs, clear_app_logs, get_download_status, clear_download_status, set_auto_launch, get_auto_launch, is_windows])
        .setup(|app| {
            // 启动时清理3天前的日志（与定时清理保持一致）
            cleanup_log(3);

            // 注册下载状态到 app state
            app.manage(Arc::new(Mutex::new(DownloadState::default())));

            // 启动定时日志清理任务（每小时清理3天前的日志）
            spawn_periodic_log_cleanup();

            // 读取系统托盘设置，异步创建托盘（默认开启）
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let system_tray_enabled = {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        match sqlx::query_as::<_, (String,)>(
                            "SELECT value FROM user_settings WHERE key = ?"
                        )
                        .bind("ui.system_tray")
                        .fetch_one(state.db.inner())
                        .await {
                            Ok((val,)) => val == "true",
                            Err(_) => false,
                        }
                    } else {
                        false
                    }
                };

                if system_tray_enabled {
                    if let Err(e) = setup_tray(&app_handle) {
                        log(&format!("Failed to setup tray: {}", e));
                    }
                }
            });

            // 启动时自动检查更新（后台静默，有更新时通知前端）
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 延迟 3 秒，等 UI 完全加载
                tokio::time::sleep(Duration::from_secs(3)).await;
                auto_check_updates(&app_handle).await;
                // 启动时自动下载更新（如果有的话）
                auto_download_update(&app_handle).await;
            });

            // 每小时自动检查并下载更新
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                    auto_download_update(&app_handle).await;
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // 先阻止关闭，然后根据设置决定行为
                    api.prevent_close();
                    let app_handle = window.app_handle().clone();
                    let window_clone = window.clone();

                    tauri::async_runtime::spawn(async move {
                        let system_tray_enabled = {
                            if let Some(state) = app_handle.try_state::<AppState>() {
                                match sqlx::query_as::<_, (String,)>(
                                    "SELECT value FROM user_settings WHERE key = ?"
                                )
                                .bind("ui.system_tray")
                                .fetch_one(state.db.inner())
                                .await {
                                    Ok((val,)) => val == "true",
                                    Err(_) => false,
                                }
                            } else {
                                false
                            }
                        };

                        if system_tray_enabled {
                            // 隐藏窗口到系统托盘
                            let _ = window_clone.hide();
                            log("Window hidden, app running in system tray");
                        } else {
                            // 直接退出应用
                            app_handle.exit(0);
                        }
                    });
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

#[cfg(test)]
mod tests {
    /// 测试版本比较逻辑
    fn compare_versions(remote_version: &str, current_version: &str) -> bool {
        if let (Ok(remote_ver), Ok(current_ver)) = (
            semver::Version::parse(remote_version),
            semver::Version::parse(current_version),
        ) {
            remote_ver > current_ver
        } else {
            // 回退到字符串比较
            remote_version > current_version
        }
    }

    #[test]
    fn test_version_compare_equal() {
        // 相同版本应该返回 false（无更新）
        assert_eq!(compare_versions("1.0.20", "1.0.20"), false);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), false);
        assert_eq!(compare_versions("0.1.0", "0.1.0"), false);
    }

    #[test]
    fn test_version_compare_newer() {
        // 远程版本更新应该返回 true
        assert_eq!(compare_versions("1.0.21", "1.0.20"), true);
        assert_eq!(compare_versions("1.1.0", "1.0.20"), true);
        assert_eq!(compare_versions("2.0.0", "1.0.20"), true);
        assert_eq!(compare_versions("1.0.20", "1.0.19"), true);
    }

    #[test]
    fn test_version_compare_older() {
        // 远程版本更旧应该返回 false
        assert_eq!(compare_versions("1.0.19", "1.0.20"), false);
        assert_eq!(compare_versions("1.0.0", "1.0.20"), false);
        assert_eq!(compare_versions("0.9.0", "1.0.20"), false);
    }

    #[test]
    fn test_version_with_v_prefix() {
        // 测试带 v 前缀的版本号（模拟 GitHub API 返回的格式）
        let remote_with_v = "v1.0.21";
        let remote_normalized = remote_with_v.strip_prefix('v').unwrap_or(remote_with_v);
        assert_eq!(compare_versions(remote_normalized, "1.0.20"), true);
        assert_eq!(compare_versions("1.0.20", "1.0.20"), false);
    }

    #[test]
    fn test_version_empty_remote() {
        // 测试远程版本为空的情况（API 失败时）
        // 空字符串应该返回 false（无更新）
        assert_eq!(compare_versions("", "1.0.20"), false);
        assert_eq!(compare_versions("", "0.0.1"), false);
    }
}
