/// Health handlers for unified API
use crate::error::Result;
use serde_json::Value;

pub async fn handle_health() -> Result<Value> {
    Ok(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn handle_app_version() -> Result<Value> {
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn handle_app_logs() -> Result<Value> {
    use std::fs;
    use dirs::cache_dir;
    use std::path::PathBuf;

    let log_path = cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));

    let logs_content = fs::read_to_string(&log_path).unwrap_or_default();

    Ok(serde_json::json!({
        "logs": logs_content,
        "log_file": log_path.to_string_lossy()
    }))
}

pub async fn handle_app_logs_clear() -> Result<Value> {
    use std::fs;
    use dirs::cache_dir;
    use std::path::PathBuf;

    let tauri_log_path = cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));
    let _ = fs::write(&tauri_log_path, "");

    let log_dir = cache_dir()
        .map(|d| d.join("kafka-manager").join("logs"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager/logs"));

    if let Ok(entries) = fs::read_dir(&log_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let _ = fs::remove_file(path);
            }
        }
    }

    Ok(serde_json::json!({
        "success": true
    }))
}
