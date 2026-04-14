#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use std::fs::OpenOptions;
use std::io::Write;

use arc_swap::ArcSwap;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};

// 引用主项目的 kafka-manager-api crate
use kafka_manager_api::{
    Config, DbPool, KafkaClients, ClusterPools,
    MetadataCache, RefreshState,
    AppState, create_router,
};
use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

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

/// 简单的日志函数，确保在 Windows 上也能看到输出
fn log(msg: &str) {
    eprintln!("[KAFKA-MANAGER] {}", msg);
    write_log(msg);
}

/// 写入日志到文件
fn write_log(msg: &str) {
    let log_path = dirs::cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));

    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
    }
}

/// 清理日志文件，只保留指定天数内的日志
fn cleanup_log(max_days: u32) {
    use chrono::NaiveDateTime;

    let log_path = dirs::cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));

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

    // 过滤出指定天数内的日志行
    // 日志格式: [2026-04-09 12:00:00] message
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
        // 无法解析日期的行也保留（可能是启动日志等）
        true
    }).collect();

    let _ = std::fs::write(&log_path, kept_lines.join("\n"));
    log(&format!("Log cleanup: kept {} lines, removed {} lines",
        kept_lines.len(),
        content.lines().count().saturating_sub(kept_lines.len())));
}

/// 启动时清理：只保留今天的日志
fn cleanup_today_start() {
    let log_path = dirs::cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));

    if !log_path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();

    // 只保留今天的日志（按日期前缀过滤）
    let today_lines: Vec<&str> = content.lines()
        .skip_while(|line| {
            // 跳过日志开头直到遇到今天的第一条日志
            !line.contains(&today_str)
        })
        .collect();

    if today_lines.len() < content.lines().count() {
        let _ = std::fs::write(&log_path, today_lines.join("\n"));
        log(&format!("Startup log cleanup: today's lines={}, removed={} lines",
            today_lines.len(),
            content.lines().count().saturating_sub(today_lines.len())));
    }
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

    // 初始化刷新状态跟踪
    let refresh_state = Arc::new(Mutex::new(RefreshState::default()));

    // 构建应用状态
    let state = AppState {
        db: pool,
        clients,
        config: config.clone(),
        pools: kafka_pools,
        cache,
        refresh_state,
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

/// 获取日志内容
#[tauri::command]
fn get_app_logs() -> Result<String, String> {
    let log_path = dirs::cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));

    std::fs::read_to_string(&log_path)
        .or_else(|_| Ok(String::new()))
}

/// 清除日志
#[tauri::command]
fn clear_app_logs() -> Result<(), String> {
    let log_path = dirs::cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));

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
fn is_portable_mode() -> bool {
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

    true
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

    let response = request.send().await.map_err(|e| format!("下载失败：{}", e))?;

    let final_response = if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        log("Received 416, removing and retrying");
        std::fs::remove_file(download_path).ok();
        let retry = client.get(url).header("User-Agent", "kafka-manager").send().await
            .map_err(|e| format!("下载失败：{}", e))?;
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

            // 清理上次更新遗留的旧 exe（如果存在）
            if old_exe.exists() {
                if let Err(e) = std::fs::remove_file(&old_exe) {
                    log(&format!("Warning: could not remove old exe from previous update: {}", e));
                } else {
                    log("Removed old exe from previous update");
                }
            }

            // Windows 允许重命名运行中的 exe，重命名后原路径即可用于写入新文件
            match std::fs::rename(&current_exe, &old_exe) {
                Ok(()) => {
                    log("Renamed current exe to .exe.old");
                    if let Err(e) = std::fs::copy(&new_exe, &current_exe) {
                        log(&format!("Failed to copy new exe: {}, reverting", e));
                        // 回退：把旧 exe 重命名回来
                        let _ = std::fs::rename(&old_exe, &current_exe);
                    } else {
                        log("New exe copied successfully");
                    }
                }
                Err(e) => {
                    log(&format!("Failed to rename current exe: {}", e));
                    // 回退：用批处理脚本等待进程退出后替换
                    let new_exe_str = new_exe.to_string_lossy().replace('/', "\\");
                    let bat_path = cache_dir.join("update_portable.bat");
                    let bat_content = format!(
                        r#"@echo off
cd /d "{current_dir_str}"
timeout /t 3 /nobreak >nul
if exist "{new_exe_str}" (
  copy /y "{new_exe_str}" "{current_dir_str}\kafka-manager.exe" >nul 2>&1
)
start "" "{current_dir_str}\kafka-manager.exe"
timeout /t 2 /nobreak >nul
rmdir /s /q "{temp_dir_str}" >nul 2>nul
del /q "%~dp0\update_portable.bat" >nul 2>nul
"#,
                    );
                    let _ = std::fs::write(&bat_path, bat_content);
                    std::process::Command::new("cmd")
                        .arg("/c")
                        .arg(&bat_path)
                        .creation_flags(0x00000200)
                        .spawn()
                        .ok();
                }
            }

            // 启动新进程：用 detached 完全独立启动，确保父进程退出后新进程仍在运行
            let new_exe_path = current_exe.to_string_lossy().to_string();
            std::process::Command::new(&current_exe)
                .current_dir(&current_dir)
                .detach()
                .spawn()
                .ok();

            log(&format!("Spawned new process: {}", new_exe_path));

            // 短暂等待确保新进程启动
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
                    let _ = app_handle.emit("download_error", error_msg);
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
                log("Opening DMG file and will exit app...");
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

                // 使用 status() 同步等待 open 命令完成，以便捕获错误
                match std::process::Command::new("open")
                    .arg(&download_path)
                    .status()
                {
                    Ok(status) => {
                        if status.success() {
                            log("Successfully opened DMG");
                        } else {
                            log(&format!("Failed to open DMG, exit status: {:?}", status.code()));
                        }
                    }
                    Err(e) => log(&format!("Failed to execute open command: {}", e)),
                }

                // 通知用户即将退出
                app_handle.dialog()
                    .message("安装包已打开，应用将在 3 秒后退出，请按照提示完成安装")
                    .title("下载完成")
                    .show(|_| {});

                // 延迟退出应用，让用户看到提示
                let app_handle_clone = app_handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
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
        .invoke_handler(tauri::generate_handler![greet, get_app_version, check_for_updates, install_update, get_app_logs, clear_app_logs, get_download_status, clear_download_status])
        .setup(|app| {
            // 启动时清理今天之前的日志
            cleanup_today_start();

            // 注册下载状态到 app state
            app.manage(Arc::new(Mutex::new(DownloadState::default())));

            // 启动定时日志清理任务（每小时清理3天前的日志）
            spawn_periodic_log_cleanup();

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
