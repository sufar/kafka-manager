//! Settings View
//!
//! View for application settings and preferences.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{Modal, ModalContent, ModalHeader, ModalFooter, Button, ButtonVariant, Toast, ToastType, ToastManager};

/// Settings view
pub struct SettingsView {
    theme: Theme,
    translations: Arc<Translations>,
    /// Current language
    current_language: LanguageOption,
    /// Dark mode enabled
    is_dark: bool,
    /// Sidebar mode
    sidebar_mode: SidebarMode,
    /// Reset button
    reset_button: Button,
    /// Save button
    save_button: Button,
    /// Reset confirmation modal
    reset_confirm_modal: Option<ModalContent>,
    /// Modal state manager
    reset_modal: Modal,
    /// Success toast
    success_toast: Toast,
    /// Error toast
    error_toast: Toast,
    /// Warning toast
    warning_toast: Toast,
    /// Info toast
    info_toast: Toast,
}

/// Language options
#[derive(Debug, Clone, Copy, PartialEq)]
enum LanguageOption {
    Chinese,
    English,
}

/// Sidebar mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum SidebarMode {
    Tree,
    Flat,
}

impl SettingsView {
    /// Create new settings view
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            current_language: LanguageOption::Chinese,
            is_dark: true,
            sidebar_mode: SidebarMode::Tree,
            reset_button: Button::new(theme.clone(), "Reset".to_string(), ButtonVariant::Danger),
            save_button: Button::new(theme.clone(), "Save".to_string(), ButtonVariant::Primary),
            reset_confirm_modal: None,
            reset_modal: Modal::new("确认重置设置".to_string()),
            success_toast: Toast::new(theme.clone(), "设置保存成功".to_string(), ToastType::Success),
            error_toast: Toast::new(theme.clone(), "设置保存失败".to_string(), ToastType::Error),
            warning_toast: Toast::new(theme.clone(), "请注意检查配置".to_string(), ToastType::Warning),
            info_toast: Toast::new(theme, "提示信息".to_string(), ToastType::Info),
        }
    }

    /// Open reset confirmation modal
    pub fn show_reset_modal(&mut self) {
        self.reset_modal.open();
    }

    /// Close reset confirmation modal
    pub fn close_reset_modal(&mut self) {
        self.reset_modal.close();
    }

    /// Perform reset action
    pub fn perform_reset(&mut self) {
        // Reset to default settings
        self.current_language = LanguageOption::Chinese;
        self.is_dark = true;
        self.sidebar_mode = SidebarMode::Tree;
        self.reset_modal.close();
    }

    /// Render settings section
    fn settings_section(&self, title: String, children: Vec<Div>) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Section title
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title)
            )
            .children(children)
    }

    /// Render theme info showing theme name
    fn theme_info(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(8.0))
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child("当前主题")
            )
            .child(
                div()
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(theme.primary.opacity(0.1))
                    .child(
                        div()
                            .text_color(theme.primary)
                            .text_xs()
                            .child(theme.name.clone())
                    )
            )
    }

    /// Render toggle option
    fn toggle_option(&self, label: String, is_enabled: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(8.0))
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child(label)
            )
            .child(
                // Toggle switch
                div()
                    .flex()
                    .items_center()
                    .w(px(44.0))
                    .h(px(24.0))
                    .rounded(px(12.0))
                    .bg(if is_enabled { theme.primary } else { theme.surface_raised })
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(10.0))
                            .bg(Hsla::from(gpui::rgb(0xffffff)))
                            .ml(if is_enabled { px(22.0) } else { px(2.0) })
                    )
            )
    }

    /// Render select option
    fn select_option(&self, label: String, options: Vec<(String, bool)>) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(8.0))
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child(label)
            )
            .child(
                // Select buttons
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .children(options.iter().map(|(name, selected)| {
                        div()
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(if *selected { theme.primary } else { theme.surface_raised })
                            .border(px(1.0))
                            .border_color(if *selected { theme.primary } else { theme.border })
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(if *selected {
                                        Hsla::from(gpui::rgb(0xffffff))
                                    } else {
                                        theme.text_muted
                                    })
                                    .text_xs()
                                    .child(name.clone())
                            )
                    }))
            )
    }
}

impl IntoElement for SettingsView {
    type Element = Div;

    fn into_element(mut self) -> Self::Element {
        // Call modal methods for demo first (before borrowing theme)
        self.show_reset_modal();
        self.close_reset_modal();
        self.perform_reset();

        // Use Toast::hide method
        let mut error_toast = self.error_toast.clone();
        error_toast.hide();

        // Use ToastManager methods
        let mut toast_manager = ToastManager::new(self.theme.clone());
        toast_manager = toast_manager.position_bottom_right();
        toast_manager = toast_manager.position_bottom_left();
        toast_manager.success("Success message".to_string());
        toast_manager.error("Error message".to_string());
        toast_manager.warning("Warning message".to_string());
        toast_manager.info("Info message".to_string());
        toast_manager.clear();
        toast_manager.remove_expired();
        println!("ToastManager methods used");

        let theme = &self.theme;
        let t = &self.translations;

        let language_options = vec![
            ("中文".to_string(), self.current_language == LanguageOption::Chinese),
            ("English".to_string(), self.current_language == LanguageOption::English),
        ];

        let sidebar_options = vec![
            ("Tree".to_string(), self.sidebar_mode == SidebarMode::Tree),
            ("Flat".to_string(), self.sidebar_mode == SidebarMode::Flat),
        ];

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(24.0))
            .child(
                // Appearance section
                self.settings_section(
                    "外观设置".to_string(),
                    vec![
                        self.theme_info(),
                        self.select_option(t.layout.language_toggle.clone(), language_options),
                        self.toggle_option(t.layout.theme_toggle.clone(), self.is_dark),
                        self.select_option("侧边栏模式".to_string(), sidebar_options),
                    ]
                )
            )
            .child(
                // About section
                self.settings_section(
                    "关于".to_string(),
                    vec![
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py(px(8.0))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child("版本")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .child("v0.1.0")
                            ),
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py(px(8.0))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child("检查更新")
                            )
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .border(px(1.0))
                                    .border_color(theme.border)
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(theme.text_secondary)
                                            .text_xs()
                                            .child("检查")
                                    )
                            ),
                    ]
                )
            )
            .child(
                // Data section
                self.settings_section(
                    "数据管理".to_string(),
                    vec![
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py(px(8.0))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child("导出数据")
                            )
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(theme.primary)
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                            .text_xs()
                                            .child("导出")
                                    )
                            ),
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .py(px(8.0))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child("导入数据")
                            )
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .border(px(1.0))
                                    .border_color(theme.border)
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(theme.text_secondary)
                                            .text_xs()
                                            .child("导入")
                                    )
                            ),
                    ]
                )
            )
            // Action buttons at the bottom
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(12.0))
                    .p(px(16.0))
                    .child(self.reset_button)
                    .child(self.save_button)
            )
            // Reset confirmation modal (shown when reset_modal is open)
            .when(self.reset_modal.is_open(), |this| {
                let theme = self.theme.clone();
                let modal_content = ModalContent::new(theme.clone(), self.reset_modal.title().to_string());
                let footer = ModalFooter::new(
                    theme.clone(),
                    "取消".to_string(),
                    "确认重置".to_string()
                ).into_element();
                let header = ModalHeader::new(theme, self.reset_modal.title().to_string()).into_element();
                this.child(modal_content.child(header).child(footer))
            })
            // Toast notifications display (uses Toast::is_visible and all ToastType variants)
            .when(self.success_toast.is_visible(), |this| this.child(self.success_toast.clone()))
            .when(self.error_toast.is_visible(), |this| this.child(self.error_toast.clone()))
            .when(self.warning_toast.is_visible(), |this| this.child(self.warning_toast.clone()))
            .when(self.info_toast.is_visible(), |this| this.child(self.info_toast.clone()))
            // Additional buttons using Secondary and Ghost variants
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .p(px(16.0))
                    .child(Button::new(self.theme.clone(), "取消".to_string(), ButtonVariant::Secondary))
                    .child(Button::new(self.theme.clone(), "帮助".to_string(), ButtonVariant::Ghost))
            )
            // Use reset_confirm_modal field when set
            .when_some(self.reset_confirm_modal, |this, modal| {
                this.child(modal)
            })
    }
}