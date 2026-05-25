//! Theme Selector Component
//!
//! Component for selecting application theme (Light/Dark/System).

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Theme mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemeMode {
    /// Get display label
    pub fn label(&self) -> String {
        match self {
            ThemeMode::Dark => "深色".to_string(),
            ThemeMode::Light => "浅色".to_string(),
            ThemeMode::System => "跟随系统".to_string(),
        }
    }

    /// Get icon/indicator
    pub fn icon(&self) -> String {
        match self {
            ThemeMode::Dark => "🌙".to_string(),
            ThemeMode::Light => "☀".to_string(),
            ThemeMode::System => "💻".to_string(),
        }
    }
}

/// Theme Selector - Theme switching component
pub struct ThemeSelector {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Current theme mode
    current_mode: ThemeMode,
    /// Available modes
    modes: Vec<ThemeMode>,
    /// Is dropdown open
    is_open: bool,
    /// Is compact mode
    is_compact: bool,
}

impl ThemeSelector {
    /// Create new theme selector
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            current_mode: ThemeMode::default(),
            modes: vec![ThemeMode::Dark, ThemeMode::Light, ThemeMode::System],
            is_open: false,
            is_compact: false,
        }
    }

    /// Create compact mode
    pub fn compact(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            current_mode: ThemeMode::default(),
            modes: vec![ThemeMode::Dark, ThemeMode::Light, ThemeMode::System],
            is_open: false,
            is_compact: true,
        }
    }

    /// Set current mode
    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.current_mode = mode;
    }

    /// Toggle dropdown
    pub fn toggle_dropdown(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Close dropdown
    pub fn close_dropdown(&mut self) {
        self.is_open = false;
    }

    /// Get current mode
    pub fn current(&self) -> ThemeMode {
        self.current_mode
    }

    /// Render trigger button
    fn render_trigger(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(6.0))
            .bg(if self.is_open { theme.surface_raised } else { theme.surface })
            .border(px(1.0))
            .border_color(theme.border)
            .cursor_pointer()
            .child(
                // Theme icon
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .child(self.current_mode.icon())
            )
            .when(!self.is_compact, |this| {
                this.child(
                    // Theme name
                    div()
                        .text_color(theme.text_secondary)
                        .text_xs()
                        .child(self.current_mode.label())
                )
            })
            .child(
                // Dropdown indicator
                div()
                    .w(px(12.0))
                    .h(px(12.0))
                    .rounded(px(2.0))
                    .bg(theme.text_muted.opacity(0.3))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(if self.is_open { "↑" } else { "↓" })
                    )
            )
    }

    /// Render theme mode item
    fn render_mode_item(&self, mode: ThemeMode, is_selected: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(6.0))
            .bg(if is_selected { theme.primary.opacity(0.15) } else { gpui::transparent_black() })
            .border_l(px(2.0))
            .border_color(if is_selected { theme.primary } else { gpui::transparent_black() })
            .cursor_pointer()
            .child(
                // Icon
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .child(mode.icon())
            )
            .child(
                // Mode name
                div()
                    .flex_1()
                    .text_color(if is_selected { theme.text } else { theme.text_secondary })
                    .text_sm()
                    .child(mode.label())
            )
            .when(is_selected, |this| {
                this.child(
                    // Selected indicator
                    div()
                        .w(px(10.0))
                        .h(px(10.0))
                        .rounded(px(5.0))
                        .bg(theme.primary)
                )
            })
    }

    /// Render dropdown menu
    fn render_dropdown(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .mt(px(4.0))
            .p(px(8.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Header
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .px(px(6.0))
                    .child("主题")
            )
            .children(self.modes.iter().map(|mode| {
                let is_selected = *mode == self.current_mode;
                self.render_mode_item(*mode, is_selected)
            }))
            .child(
                // Preview section
                div()
                    .mt(px(8.0))
                    .pt(px(8.0))
                    .border_t(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("预览")
                    )
                    .child(
                        // Preview colors
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .mt(px(4.0))
                            .child(
                                // Background preview
                                div()
                                    .w(px(24.0))
                                    .h(px(12.0))
                                    .rounded(px(3.0))
                                    .bg(if self.current_mode == ThemeMode::Light {
                                        gpui::rgb(0xffffff)
                                    } else {
                                        gpui::rgb(0x1e1e1e)
                                    })
                                    .border(px(1.0))
                                    .border_color(theme.border)
                            )
                            .child(
                                // Text preview
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .w(px(40.0))
                                            .h(px(4.0))
                                            .rounded(px(1.0))
                                            .bg(if self.current_mode == ThemeMode::Light {
                                                gpui::rgb(0x333333)
                                            } else {
                                                gpui::rgb(0xffffff)
                                            })
                                    )
                                    .child(
                                        div()
                                            .w(px(30.0))
                                            .h(px(3.0))
                                            .rounded(px(1.0))
                                            .bg(if self.current_mode == ThemeMode::Light {
                                                gpui::rgb(0x666666)
                                            } else {
                                                gpui::rgb(0xaaaaaa)
                                            })
                                    )
                            )
                    )
            )
    }
}

impl IntoElement for ThemeSelector {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(self.render_trigger())
            .when(self.is_open, |this| {
                this.child(self.render_dropdown())
            })
    }
}

impl Clone for ThemeSelector {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            current_mode: self.current_mode,
            modes: self.modes.clone(),
            is_open: self.is_open,
            is_compact: self.is_compact,
        }
    }
}