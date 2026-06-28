// Settings 事件处理器

use kafka_manager_api::{AppState, db::settings::SettingStore};
use slint::{ComponentHandle, Weak};
use tokio::sync::RwLock;
use std::sync::Arc;

// 从 crate root 引用 i18n（main.rs中声明）
use crate::i18n::{Language, apply_i18n};

/// 加载用户设置（从数据库）
pub async fn load_settings(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
) {
    println!("Loading user settings from database");

    let state_guard = state.read().await;
    let db = state_guard.db.inner();

    // 加载各项设置
    let tray_enabled = SettingStore::get(db, "system_tray_enabled")
        .await
        .unwrap_or(None)
        .map(|v| v == "true")
        .unwrap_or(false);

    let language = SettingStore::get(db, "language")
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "en".to_string());

    let theme = SettingStore::get(db, "theme")
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "light".to_string());

    println!("Loaded settings: tray={}, lang={}, theme={}", tray_enabled, language, theme);

    // 更新 UI（包括国际化）
    let _ = app.upgrade_in_event_loop(move |app| {
        app.set_system_tray_enabled(tray_enabled);
        app.set_language(slint::SharedString::from(language.clone()));
        app.set_theme(slint::SharedString::from(theme));

        // 应用国际化
        let lang_enum = Language::from_str(&language);
        apply_i18n(&app, lang_enum);
    });
}

/// 保存系统托盘设置
pub async fn save_tray_enabled(
    state: Arc<RwLock<AppState>>,
    enabled: bool,
) {
    println!("Saving tray enabled: {}", enabled);

    let state_guard = state.read().await;
    let db = state_guard.db.inner();

    match SettingStore::set(db, "system_tray_enabled", &enabled.to_string()).await {
        Ok(_) => println!("Tray setting saved successfully"),
        Err(e) => eprintln!("Failed to save tray setting: {}", e),
    }
}

/// 保存语言设置
pub async fn save_language(
    state: Arc<RwLock<AppState>>,
    language: String,
) {
    println!("Saving language: {}", language);

    let state_guard = state.read().await;
    let db = state_guard.db.inner();

    match SettingStore::set(db, "language", &language).await {
        Ok(_) => println!("Language setting saved successfully"),
        Err(e) => eprintln!("Failed to save language setting: {}", e),
    }
}

/// 保存主题设置
pub async fn save_theme(
    state: Arc<RwLock<AppState>>,
    theme: String,
) {
    println!("Saving theme: {}", theme);

    let state_guard = state.read().await;
    let db = state_guard.db.inner();

    match SettingStore::set(db, "theme", &theme).await {
        Ok(_) => println!("Theme setting saved successfully"),
        Err(e) => eprintln!("Failed to save theme setting: {}", e),
    }
}

/// 保存所有设置
pub async fn save_all_settings(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
) {
    println!("Saving all settings");

    // 获取当前 UI 状态
    let (tray_enabled, language, theme) = {
        let app_strong = app.upgrade().unwrap();
        let tray = app_strong.get_system_tray_enabled();
        let lang = app_strong.get_language();
        let theme_str = app_strong.get_theme();
        (tray, lang.to_string(), theme_str.to_string())
    };

    let state_guard = state.read().await;
    let db = state_guard.db.inner();

    // 保存所有设置
    let results = futures::future::join3(
        SettingStore::set(db, "system_tray_enabled", &tray_enabled.to_string()),
        SettingStore::set(db, "language", &language),
        SettingStore::set(db, "theme", &theme),
    ).await;

    match results {
        (Ok(_), Ok(_), Ok(_)) => println!("All settings saved successfully"),
        _ => eprintln!("Some settings failed to save"),
    }
}