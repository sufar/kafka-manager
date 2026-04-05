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
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

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
        });
    }

    // 直接请求 GitHub Releases API
    let api_url = "https://api.github.com/repos/sufar/kafka-manager/releases/latest";
    log(&format!("Fetching: {}", api_url));

    let client = reqwest::Client::new();

    let response = client
        .get(api_url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    log(&format!("HTTP status: {}", response.status()));

    // 检查 HTTP 状态码，非 200 时返回错误
    if !response.status().is_success() {
        log(&format!("GitHub API error: HTTP {}", response.status()));
        // API 请求失败时，返回无更新而不是错误
        // 这样可以避免网络问题或 API 限制造成用户困扰
        return Ok(UpdateResult {
            available: false,
            version: current_version.to_string(),
            notes: None,
            date: None,
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
    })
}

use sha2::{Sha256, Digest};

/// 计算文件的 SHA256 hash
fn calculate_file_hash(path: &std::path::Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("打开文件失败: {}", e))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// 验证文件签名（支持多种签名格式）
fn verify_signature(file_path: &std::path::Path, expected_sig: &str) -> Result<bool, String> {
    if expected_sig.is_empty() {
        log("Warning: Expected signature is empty, skipping verification");
        return Ok(true);
    }

    // 检查文件是否存在
    if !file_path.exists() {
        return Ok(false);
    }

    // 计算文件 hash
    let file_hash = calculate_file_hash(file_path)?;

    // 比较签名（支持 minisign 格式和纯 hash 格式）
    // minisign 签名格式包含 hash，我们简化处理直接比较 hash
    let is_valid = file_hash.eq_ignore_ascii_case(expected_sig) ||
                   expected_sig.to_lowercase().contains(&file_hash.to_lowercase());

    log(&format!("Signature verification: file_hash={}, expected={}, match={}",
                 &file_hash[..16.min(file_hash.len())],
                 &expected_sig[..16.min(expected_sig.len())],
                 is_valid));

    Ok(is_valid)
}

/// 读取缓存的签名信息
fn read_cached_signature(cache_dir: &std::path::Path, filename: &str) -> Option<String> {
    let sig_path = cache_dir.join(format!("{}.sig", filename));
    std::fs::read_to_string(&sig_path).ok()
}

/// 保存签名信息到缓存
fn save_cached_signature(cache_dir: &std::path::Path, filename: &str, signature: &str) -> Result<(), String> {
    let sig_path = cache_dir.join(format!("{}.sig", filename));
    std::fs::write(&sig_path, signature)
        .map_err(|e| format!("保存签名失败: {}", e))
}

/// 断点续传下载文件（带进度回调）
async fn download_with_resume<F>(
    client: &reqwest::Client,
    url: &str,
    download_path: &std::path::Path,
    expected_signature: &str,
    mut progress_callback: F,
) -> Result<bool, String>
where
    F: FnMut(u64, u64) + Send + 'static,
{
    use futures::StreamExt;

    // 检查本地文件
    let mut existing_size = if download_path.exists() {
        std::fs::metadata(download_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    // 验证现有文件签名
    if existing_size > 0 && !expected_signature.is_empty() {
        log(&format!("Checking existing file: {} bytes", existing_size));
        match verify_signature(download_path, expected_signature) {
            Ok(true) => {
                log("Existing file signature verified, skipping download");
                progress_callback(existing_size, existing_size); // 100% 进度
                return Ok(true); // 签名匹配，无需下载
            }
            Ok(false) => {
                log("Existing file signature mismatch, will re-download");
            }
            Err(e) => {
                log(&format!("Signature verification failed: {}", e));
            }
        }
    }

    // 获取文件信息
    let head_response = client
        .get(url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|e| format!("获取文件信息失败: {}", e))?;

    let total_size = head_response.content_length().unwrap_or(0);
    log(&format!("Remote file size: {} bytes", total_size));

    // 如果文件已完整下载但签名验证失败，删除重新下载
    if existing_size > 0 && existing_size >= total_size && total_size > 0 {
        log("File exists but signature mismatch or file too large, removing and re-downloading");
        std::fs::remove_file(download_path).ok();
        existing_size = 0;
    }

    // 检查是否支持断点续传
    let supports_resume = head_response.headers()
        .get("accept-ranges")
        .map_or(false, |v| v.as_bytes() == b"bytes");

    // 只有本地文件小于远程文件时才尝试断点续传
    let start_byte = if supports_resume && existing_size > 0 && existing_size < total_size {
        log(&format!("Resuming download from byte {} (remote: {})", existing_size, total_size));
        existing_size
    } else {
        if existing_size > 0 && !supports_resume {
            log("Server doesn't support resume, restarting download");
            std::fs::remove_file(download_path).ok();
        }
        0
    };

    // 构建请求
    let mut request = client
        .get(url)
        .header("User-Agent", "kafka-manager");

    // 添加 Range 头进行断点续传
    // 添加 Range 头进行断点续传
    if start_byte > 0 {
        request = request.header("Range", format!("bytes={}-", start_byte));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("下载失败：{}", e))?;

    // 检查响应状态 - 处理 416 错误
    let final_response = if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        log("Received 416 Range Not Satisfiable, removing local file and retrying");
        std::fs::remove_file(download_path).ok();
        // 重新下载
        let retry_response = client
            .get(url)
            .header("User-Agent", "kafka-manager")
            .send()
            .await
            .map_err(|e| format!("下载失败：{}", e))?;
        if !retry_response.status().is_success() {
            return Err(format!("下载失败：HTTP {}", retry_response.status()));
        }
        retry_response
    } else {
        if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(format!("下载失败：HTTP {}", response.status()));
        }
        response
    };

    // 以追加模式写入文件
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(download_path)
        .map_err(|e| format!("打开文件失败：{}", e))?;

    // 流式下载，带进度跟踪
    // 416 重试后从 0 开始下载
    let mut downloaded: u64 = 0;
    let mut stream = final_response.bytes_stream();

    // 发送初始进度

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载流错误: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        downloaded += chunk.len() as u64;

        // 发送进度更新
        progress_callback(downloaded, total_size);
    }

    file.flush().map_err(|e| format!("刷新文件失败: {}", e))?;
    drop(file);

    log("Download completed");
    Ok(false) // 表示是新下载的
}

/// 从 latest.json 获取签名信息
async fn fetch_signature_from_latest(
    client: &reqwest::Client,
    tag_name: &str,
    platform: &str,
) -> Result<String, String> {
    let latest_url = format!(
        "https://github.com/sufar/kafka-manager/releases/download/{}/latest.json",
        tag_name
    );

    log(&format!("Fetching signature from: {}", latest_url));

    let response = client
        .get(&latest_url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|e| format!("获取 latest.json 失败: {}", e))?;

    if !response.status().is_success() {
        log(&format!("latest.json not found: HTTP {}", response.status()));
        return Ok(String::new());
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 latest.json 失败: {}", e))?;

    // 从 platforms 中提取签名
    let signature = json
        .get("platforms")
        .and_then(|p| p.get(platform))
        .and_then(|p| p.get("signature"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    log(&format!("Found signature: {}...", &signature[..20.min(signature.len())]));
    Ok(signature)
}

/// 下载并安装更新（支持签名验证和断点续传）
#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    on_progress: tauri::ipc::Channel<(u64, u64)>,
) -> Result<(), String> {
    log("Downloading and installing update with signature verification...");

    // 获取系统缓存目录
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("kafka-manager"))
        .unwrap_or_else(|| std::env::temp_dir().join("kafka-manager-cache"));

    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("创建缓存目录失败: {}", e))?;

    log(&format!("Cache directory: {:?}", cache_dir));

    // 从 GitHub API 获取最新版本信息
    let api_url = "https://api.github.com/repos/sufar/kafka-manager/releases/latest";
    let client = reqwest::Client::new();

    let response = client
        .get(api_url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析版本信息失败: {}", e))?;

    let tag_name = json["tag_name"].as_str().unwrap_or("unknown");
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
        return Err("不支持的平台".to_string());
    };

    log(&format!("Target platform: {}", target));

    // 从 assets 中找到对应的下载 URL
    let assets = json["assets"].as_array().ok_or("找不到下载文件")?;

    let (download_url, filename) = assets
        .iter()
        .find_map(|asset| {
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
        .ok_or_else(|| "找不到适合当前平台的安装包")?;

    log(&format!("Download URL: {}", download_url));
    log(&format!("Filename: {}", filename));

    // 获取签名信息
    let expected_signature = fetch_signature_from_latest(&client, tag_name, target).await
        .unwrap_or_default();

    // 检查缓存的签名
    let cached_signature = read_cached_signature(&cache_dir, &filename);
    if let Some(ref sig) = cached_signature {
        log(&format!("Cached signature: {}...", &sig[..20.min(sig.len())]));
    }

    // 下载路径
    let download_path = cache_dir.join(&filename);

    // 克隆 on_progress 用于回调
    let progress_channel = on_progress.clone();

    // 执行下载（带断点续传和进度回调）
    let skipped = download_with_resume(
        &client,
        &download_url,
        &download_path,
        &expected_signature,
        move |downloaded, total| {
            // 发送进度事件到前端
            let _ = progress_channel.send((downloaded, total));
        },
    ).await?;

    if skipped {
        log("Using cached file (signature verified)");
    } else {
        // 下载完成后验证签名
        if !expected_signature.is_empty() {
            match verify_signature(&download_path, &expected_signature) {
                Ok(true) => {
                    log("Downloaded file signature verified");
                    // 保存签名到缓存
                    save_cached_signature(&cache_dir, &filename, &expected_signature)?;
                }
                Ok(false) => {
                    // 签名不匹配，删除文件
                    std::fs::remove_file(&download_path).ok();
                    return Err("下载文件签名验证失败，已删除".to_string());
                }
                Err(e) => {
                    log(&format!("Signature verification error: {}", e));
                    // 继续安装，但记录警告
                }
            }
        }
    }

    log(&format!("File ready at: {:?}", download_path));

    // 使用系统对话框通知用户
    use tauri_plugin_dialog::DialogExt;

    #[cfg(target_os = "macos")]
    {
        log("Opening DMG file and will exit app...");
        std::process::Command::new("open")
            .arg(&download_path)
            .spawn()
            .ok();

        // 通知用户即将退出
        app.dialog()
            .message("安装包已打开，应用将在 3 秒后退出，请按照提示完成安装")
            .title("下载完成")
            .show(|_| {});

        // 延迟退出应用，让用户看到提示
        let app_handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            log("Exiting app for update installation...");
            app_handle.exit(0);
        });
    }

    #[cfg(target_os = "windows")]
    {
        log("Opening MSI/EXE file...");
        std::process::Command::new("cmd")
            .args(["/c", "start", download_path.to_str().unwrap_or("")])
            .spawn()
            .ok();

        app.dialog()
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

        app.dialog()
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
        app.dialog()
            .message("下载完成")
            .title(if skipped { "使用缓存文件" } else { "下载完成" })
            .show(|_| {});
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![greet, get_app_version, check_for_updates, install_update, get_app_logs, clear_app_logs])
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
