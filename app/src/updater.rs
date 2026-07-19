//! 自动更新：GitHub Releases 检查、下载、绿色版安装与重启
//!
//! 从原 Tauri 壳移植（src-tauri/src/lib.rs），去掉了 Tauri 依赖。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const API_URL: &str = "https://api.github.com/repos/sufar/kafka-manager/releases/latest";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn log(msg: &str) {
    tracing::info!("[updater] {}", msg);
}

/// 更新检查结果
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateResult {
    pub available: bool,
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
    pub is_portable: bool,
    pub portable_download_url: Option<String>,
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("kafka-manager"))
        .unwrap_or_else(|| std::env::temp_dir().join("kafka-manager-cache"))
}

/// 检测是否为绿色免安装版
pub fn is_portable_mode() -> bool {
    #[cfg(target_os = "macos")]
    {
        return false;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return true,
        };
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
}

/// 检查更新
pub async fn do_check_updates() -> Result<UpdateResult, String> {
    log("Checking for updates via GitHub API...");

    // 开发模式下跳过
    if cfg!(debug_assertions) {
        return Ok(UpdateResult {
            available: false,
            version: CURRENT_VERSION.to_string(),
            notes: None,
            date: None,
            is_portable: false,
            portable_download_url: None,
        });
    }

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    let response = client
        .get(API_URL)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|e| format!("网络错误：{}", e))?;

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err("403 Forbidden".to_string());
    }
    if !response.status().is_success() {
        return Ok(UpdateResult {
            available: false,
            version: CURRENT_VERSION.to_string(),
            notes: None,
            date: None,
            is_portable: is_portable_mode(),
            portable_download_url: None,
        });
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let tag_name = json["tag_name"].as_str().unwrap_or("");
    if tag_name.is_empty() {
        return Ok(UpdateResult {
            available: false,
            version: CURRENT_VERSION.to_string(),
            notes: None,
            date: None,
            is_portable: is_portable_mode(),
            portable_download_url: None,
        });
    }

    // 发布说明（去重）
    let body_str = json["body"].as_str().unwrap_or("").to_string();
    let notes = if body_str.is_empty() {
        None
    } else {
        let lines: Vec<&str> = body_str.lines().collect();
        let mut unique_lines: Vec<&str> = Vec::new();
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for line in lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !seen.contains(trimmed) {
                seen.insert(trimmed);
                unique_lines.push(line);
            } else if trimmed.is_empty()
                && !unique_lines.is_empty()
                && !unique_lines.last().map_or(false, |l| l.trim().is_empty())
            {
                unique_lines.push(line);
            }
        }
        Some(unique_lines.join("\n"))
    };

    let published_at = json["published_at"].as_str().unwrap_or("");
    let remote_version = tag_name.strip_prefix('v').unwrap_or(tag_name);

    let has_update = if let (Ok(remote_ver), Ok(current_ver)) = (
        semver::Version::parse(remote_version),
        semver::Version::parse(CURRENT_VERSION),
    ) {
        remote_ver > current_ver
    } else {
        remote_version > CURRENT_VERSION
    };

    Ok(UpdateResult {
        available: has_update,
        version: remote_version.to_string(),
        notes,
        date: if published_at.is_empty() {
            None
        } else {
            Some(published_at.to_string())
        },
        is_portable: is_portable_mode(),
        portable_download_url: if is_portable_mode() {
            find_portable_download_url(&json)
        } else {
            None
        },
    })
}

/// 从 release assets 中提取便携版下载链接
fn find_portable_download_url(json: &serde_json::Value) -> Option<String> {
    let assets = json["assets"].as_array()?;
    for asset in assets {
        let name = asset["name"].as_str()?;
        if name.contains("Portable") || name.contains("portable") {
            return asset["browser_download_url"].as_str().map(String::from);
        }
    }
    for asset in assets {
        let name = asset["name"].as_str()?;
        if name.ends_with(".exe") {
            return asset["browser_download_url"].as_str().map(String::from);
        }
    }
    None
}

/// 下载更新包（带进度回调）
pub async fn download_update(
    url: &str,
    filename: &str,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<PathBuf, String> {
    use futures::StreamExt;

    let cache = cache_dir();
    std::fs::create_dir_all(&cache).map_err(|e| format!("创建缓存目录失败：{}", e))?;
    let target = cache.join(filename);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    let response = client
        .get(url)
        .header("User-Agent", "kafka-manager")
        .send()
        .await
        .map_err(|e| format!("下载失败：{}", e))?;

    if !response.status().is_success() {
        return Err(format!("下载失败：HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(&target).map_err(|e| format!("创建文件失败：{}", e))?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断：{}", e))?;
        file.write_all(&chunk).map_err(|e| format!("写入失败：{}", e))?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }

    log(&format!("Downloaded {} bytes to {:?}", downloaded, target));
    Ok(target)
}

/// 安装绿色版更新并重启（Windows：exe 替换 + PowerShell 脚本；其他平台暂不支持自动安装）
#[cfg(target_os = "windows")]
pub fn install_portable_update(zip_path: &Path) -> Result<(), String> {
    use std::io::Cursor;

    log("Installing portable update...");

    let zip_data = std::fs::read(zip_path).map_err(|e| format!("读取更新文件失败：{}", e))?;
    let cache = cache_dir();
    let extract_dir = cache.join("_portable_update");
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir).map_err(|e| format!("清理旧解压目录失败：{}", e))?;
    }
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("创建解压目录失败：{}", e))?;

    let cursor = Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("解压更新包失败：{}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("解压文件失败：{}", e))?;
        let outpath = extract_dir.join(file.name());
        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p).ok();
            }
            let mut outfile = std::fs::File::create(&outpath).map_err(|e| format!("创建文件失败：{}", e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("写入文件失败：{}", e))?;
        }
    }

    let current_exe = std::env::current_exe().map_err(|e| format!("获取当前程序路径失败：{}", e))?;
    let current_dir = current_exe.parent().ok_or("无法获取程序所在目录")?;
    let exe_name = current_exe.file_name().map(|n| n.to_string_lossy().to_string());
    let exe_name = exe_name.unwrap_or_else(|| "kafka-manager.exe".to_string());

    // 复制除当前 exe 之外的所有文件
    fn copy_dir_contents_skip(src: &Path, dst: &Path, skip_name: &str) -> Result<(), String> {
        for entry in std::fs::read_dir(src).map_err(|e| format!("读取目录失败：{}", e))? {
            let entry = entry.map_err(|e| format!("读取条目失败：{}", e))?;
            let src_path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name == skip_name {
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

    use std::os::windows::process::CommandExt;

    let new_exe = extract_dir.join(&exe_name);
    let old_exe = current_exe.with_extension("exe.old");
    let current_dir_str = current_dir.to_string_lossy().replace('/', "\\");
    let temp_dir_str = extract_dir.to_string_lossy().replace('/', "\\");
    let new_exe_str = new_exe.to_string_lossy().replace('/', "\\");
    let old_exe_str = old_exe.to_string_lossy().replace('/', "\\");
    let exe_path_str = current_exe.to_string_lossy().replace('/', "\\");

    if old_exe.exists() {
        let _ = std::fs::remove_file(&old_exe);
    }

    // Windows 允许重命名运行中的 exe
    let rename_ok = match std::fs::rename(&current_exe, &old_exe) {
        Ok(()) => {
            if let Err(e) = std::fs::copy(&new_exe, &current_exe) {
                log(&format!("Failed to copy new exe: {}, reverting", e));
                let _ = std::fs::rename(&old_exe, &current_exe);
                false
            } else {
                true
            }
        }
        Err(e) => {
            log(&format!("Failed to rename current exe: {}", e));
            false
        }
    };

    let ps1_path = cache.join("update_portable.ps1");
    let ps1_content = if rename_ok {
        format!(
            "$ErrorActionPreference = 'SilentlyContinue'\nStart-Process \"{exe_path_str}\"\nStart-Sleep -Seconds 2\nRemove-Item -Path \"{temp_dir_str}\" -Recurse -Force -ErrorAction SilentlyContinue\nif (Test-Path \"{old_exe_str}\") {{\n    Remove-Item -Path \"{old_exe_str}\" -Force -ErrorAction SilentlyContinue\n}}\nRemove-Item -Path $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue\n"
        )
    } else {
        format!(
            "$ErrorActionPreference = 'SilentlyContinue'\nStart-Sleep -Seconds 2\nif (Test-Path \"{new_exe_str}\") {{\n    Copy-Item -Path \"{new_exe_str}\" -Destination \"{exe_path_str}\" -Force -ErrorAction SilentlyContinue\n}}\nStart-Sleep -Seconds 1\nStart-Process \"{exe_path_str}\"\nStart-Sleep -Seconds 2\nRemove-Item -Path \"{temp_dir_str}\" -Recurse -Force -ErrorAction SilentlyContinue\nRemove-Item -Path $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue\n"
        )
    };

    std::fs::write(&ps1_path, ps1_content).map_err(|e| format!("写入更新脚本失败：{}", e))?;

    // 后台运行 PowerShell 脚本并退出当前进程
    std::process::Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File"])
        .arg(&ps1_path)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("启动更新脚本失败：{}", e))?;

    let _ = current_dir_str;
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(1));
        std::process::exit(0);
    });

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_portable_update(_zip_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持自动安装，请手动下载更新".to_string())
}
