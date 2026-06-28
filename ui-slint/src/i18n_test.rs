// 单元测试模块

#[cfg(test)]
mod tests {
    use crate::i18n::{Language, I18nStrings};

    #[test]
    fn test_language_conversion() {
        // 测试语言转换
        assert_eq!(Language::from_str("en"), Language::English);
        assert_eq!(Language::from_str("zh"), Language::Chinese);
        assert_eq!(Language::from_str("unknown"), Language::English); // 默认英文
    }

    #[test]
    fn test_english_strings() {
        let strings = I18nStrings::english();

        // 测试导航栏字符串
        assert_eq!(strings.clusters, "Clusters");
        assert_eq!(strings.topics, "Topics");
        assert_eq!(strings.messages, "Messages");
        assert_eq!(strings.consumer_groups, "Consumer Groups");
        assert_eq!(strings.favorites, "Favorites");
        assert_eq!(strings.settings, "Settings");

        // 测试页面标题
        assert_eq!(strings.clusters_title, "Clusters");
        assert_eq!(strings.settings_title, "Settings");

        // 测试 Sidebar
        assert_eq!(strings.statistics, "Statistics");
        assert_eq!(strings.logo_subtitle, "Slint Edition");
    }

    #[test]
    fn test_chinese_strings() {
        let strings = I18nStrings::chinese();

        // 测试导航栏字符串
        assert_eq!(strings.clusters, "集群");
        assert_eq!(strings.topics, "主题");
        assert_eq!(strings.messages, "消息");
        assert_eq!(strings.consumer_groups, "消费组");
        assert_eq!(strings.favorites, "收藏夹");
        assert_eq!(strings.settings, "设置");

        // 测试页面标题
        assert_eq!(strings.clusters_title, "集群管理");
        assert_eq!(strings.settings_title, "系统设置");

        // 测试 Sidebar
        assert_eq!(strings.statistics, "统计");
        assert_eq!(strings.logo_subtitle, "Slint 版");
    }

    #[test]
    fn test_i18n_for_language() {
        let en_strings = I18nStrings::for_language(Language::English);
        assert_eq!(en_strings.clusters, "Clusters");

        let zh_strings = I18nStrings::for_language(Language::Chinese);
        assert_eq!(zh_strings.clusters, "集群");
    }

    #[test]
    fn test_settings_strings() {
        let en_strings = I18nStrings::english();

        // 测试 Settings 页面字符串
        assert_eq!(en_strings.settings_system_tray, "System Tray");
        assert_eq!(en_strings.settings_enable_tray, "Enable system tray");
        assert_eq!(en_strings.settings_language, "Language");
        assert_eq!(en_strings.settings_theme, "Theme");
        assert_eq!(en_strings.settings_save, "💾 Save Settings");

        let zh_strings = I18nStrings::chinese();

        // 测试中文 Settings 页面字符串
        assert_eq!(zh_strings.settings_system_tray, "系统托盘");
        assert_eq!(zh_strings.settings_enable_tray, "启用系统托盘");
        assert_eq!(zh_strings.settings_language, "语言");
        assert_eq!(zh_strings.settings_theme, "主题");
        assert_eq!(zh_strings.settings_save, "💾 保存设置");
    }
}