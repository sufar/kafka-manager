use std::path::PathBuf;

/// 获取应用日志文件路径（Tauri 和独立服务器共享）
/// 统一路径，避免写入和读取不一致
pub fn app_log_path() -> PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"))
}

/// 确保日志文件所在目录存在
pub fn ensure_log_dir() {
    if let Some(parent) = app_log_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}
