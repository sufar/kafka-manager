//! 本地 UI 偏好持久化（主题、语言、侧栏宽度/模式、集群选择等），
//! 存于缓存目录 ui_prefs.json；小数据集同步整表重写。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPrefs {
    pub theme: String,          // "light" | "dark"
    pub language: String,       // "zh" | "en"
    pub sidebar_mode: String,   // "list" | "tree"
    pub sidebar_width: f32,     // px
    pub update_notify: bool,    // 检查更新后仍提示
    pub last_route: String,     // 启动恢复路由（如 "/messages?cluster=a&topic=b"）
    pub dev_unlocked: bool,     // 版本号连击 5 次解锁开发者功能
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            language: "zh".into(),
            sidebar_mode: "tree".into(),
            sidebar_width: 224.0,
            update_notify: false,
            last_route: String::new(),
            dev_unlocked: false,
        }
    }
}

fn prefs_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kafka-manager")
        .join("ui_prefs.json")
}

pub fn load() -> UiPrefs {
    let path = prefs_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(prefs: &UiPrefs) {
    let path = prefs_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(&path, json);
    }
}
