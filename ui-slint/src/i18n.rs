// 国际化模块 - 提供中英文支持

use slint::ComponentHandle;

/// 语言类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s {
            "zh" => Language::Chinese,
            "en" | _ => Language::English,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }
}

/// 国际化字符串结构
pub struct I18nStrings {
    // 导航栏
    pub clusters: String,
    pub topics: String,
    pub messages: String,
    pub consumer_groups: String,
    pub favorites: String,
    pub settings: String,

    // 页面标题
    pub clusters_title: String,
    pub topics_title: String,
    pub messages_title: String,
    pub consumer_groups_title: String,
    pub favorites_title: String,
    pub settings_title: String,

    // Sidebar
    pub statistics: String,
    pub logo_subtitle: String,

    // Settings 页面
    pub settings_system_tray: String,
    pub settings_enable_tray: String,
    pub settings_tray_desc: String,
    pub settings_language: String,
    pub settings_language_desc: String,
    pub settings_theme: String,
    pub settings_theme_desc: String,
    pub settings_theme_not_impl: String,
    pub settings_save: String,

    // 操作按钮
    pub refresh: String,
    pub search: String,
    pub query: String,
    pub stop: String,
    pub send: String,
}

impl I18nStrings {
    /// 获取英文字符串
    pub fn english() -> Self {
        Self {
            // 导航栏
            clusters: "Clusters".to_string(),
            topics: "Topics".to_string(),
            messages: "Messages".to_string(),
            consumer_groups: "Consumer Groups".to_string(),
            favorites: "Favorites".to_string(),
            settings: "Settings".to_string(),

            // 页面标题
            clusters_title: "Clusters".to_string(),
            topics_title: "Topics".to_string(),
            messages_title: "Messages".to_string(),
            consumer_groups_title: "Consumer Groups".to_string(),
            favorites_title: "Favorites".to_string(),
            settings_title: "Settings".to_string(),

            // Sidebar
            statistics: "Statistics".to_string(),
            logo_subtitle: "Slint Edition".to_string(),

            // Settings 页面
            settings_system_tray: "System Tray".to_string(),
            settings_enable_tray: "Enable system tray".to_string(),
            settings_tray_desc: "Show app in system tray for quick access".to_string(),
            settings_language: "Language".to_string(),
            settings_language_desc: "Select your preferred language".to_string(),
            settings_theme: "Theme".to_string(),
            settings_theme_desc: "Choose light or dark theme".to_string(),
            settings_theme_not_impl: "not implemented yet".to_string(),
            settings_save: "💾 Save Settings".to_string(),

            // 操作按钮
            refresh: "🔄 Refresh".to_string(),
            search: "Search topics...".to_string(),
            query: "🔍 Query".to_string(),
            stop: "✕ Stop".to_string(),
            send: "📨 Send".to_string(),
        }
    }

    /// 获取中文字符串
    pub fn chinese() -> Self {
        Self {
            // 导航栏
            clusters: "集群".to_string(),
            topics: "主题".to_string(),
            messages: "消息".to_string(),
            consumer_groups: "消费组".to_string(),
            favorites: "收藏夹".to_string(),
            settings: "设置".to_string(),

            // 页面标题
            clusters_title: "集群管理".to_string(),
            topics_title: "主题管理".to_string(),
            messages_title: "消息查询".to_string(),
            consumer_groups_title: "消费组管理".to_string(),
            favorites_title: "收藏夹".to_string(),
            settings_title: "系统设置".to_string(),

            // Sidebar
            statistics: "统计".to_string(),
            logo_subtitle: "Slint 版".to_string(),

            // Settings 页面
            settings_system_tray: "系统托盘".to_string(),
            settings_enable_tray: "启用系统托盘".to_string(),
            settings_tray_desc: "在系统托盘显示应用以便快速访问".to_string(),
            settings_language: "语言".to_string(),
            settings_language_desc: "选择您的首选语言".to_string(),
            settings_theme: "主题".to_string(),
            settings_theme_desc: "选择浅色或深色主题".to_string(),
            settings_theme_not_impl: "尚未实现".to_string(),
            settings_save: "💾 保存设置".to_string(),

            // 操作按钮
            refresh: "🔄 刷新".to_string(),
            search: "搜索主题...".to_string(),
            query: "🔍 查询".to_string(),
            stop: "✕ 停止".to_string(),
            send: "📨 发送".to_string(),
        }
    }

    /// 根据语言获取字符串
    pub fn for_language(lang: Language) -> Self {
        match lang {
            Language::English => Self::english(),
            Language::Chinese => Self::chinese(),
        }
    }
}

/// 应用国际化字符串到 UI
pub fn apply_i18n(app: &crate::App, lang: Language) {
    let strings = I18nStrings::for_language(lang);

    // 更新导航栏国际化字符串
    app.set_i18n_clusters(slint::SharedString::from(strings.clusters));
    app.set_i18n_topics(slint::SharedString::from(strings.topics));
    app.set_i18n_messages(slint::SharedString::from(strings.messages));
    app.set_i18n_consumer_groups(slint::SharedString::from(strings.consumer_groups));
    app.set_i18n_favorites(slint::SharedString::from(strings.favorites));
    app.set_i18n_settings(slint::SharedString::from(strings.settings));

    // 更新页面标题
    app.set_page_title_clusters(slint::SharedString::from(strings.clusters_title));
    app.set_page_title_topics(slint::SharedString::from(strings.topics_title));
    app.set_page_title_messages(slint::SharedString::from(strings.messages_title));
    app.set_page_title_consumer_groups(slint::SharedString::from(strings.consumer_groups_title));
    app.set_page_title_favorites(slint::SharedString::from(strings.favorites_title));
    app.set_page_title_settings(slint::SharedString::from(strings.settings_title));

    // 更新 Sidebar
    app.set_sidebar_statistics(slint::SharedString::from(strings.statistics));
    app.set_sidebar_subtitle(slint::SharedString::from(strings.logo_subtitle));

    // 更新 Settings 页面
    app.set_settings_system_tray_title(slint::SharedString::from(strings.settings_system_tray));
    app.set_settings_enable_tray_label(slint::SharedString::from(strings.settings_enable_tray));
    app.set_settings_tray_desc(slint::SharedString::from(strings.settings_tray_desc));
    app.set_settings_language_title(slint::SharedString::from(strings.settings_language));
    app.set_settings_language_desc(slint::SharedString::from(strings.settings_language_desc));
    app.set_settings_theme_title(slint::SharedString::from(strings.settings_theme));
    app.set_settings_theme_desc(slint::SharedString::from(strings.settings_theme_desc));
    app.set_settings_theme_not_impl(slint::SharedString::from(strings.settings_theme_not_impl));
    app.set_settings_save_button(slint::SharedString::from(strings.settings_save));

    println!("Applied i18n for language: {:?}", lang);
}