//! JSON Highlight Selector Component
//!
//! Component for selecting JSON syntax highlighting themes.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// JSON highlight theme
#[derive(Debug, Clone)]
pub struct JsonHighlightTheme {
    /// Theme name
    pub name: String,
    /// Theme ID
    pub id: String,
    /// Is default theme
    pub is_default: bool,
    /// Preview colors
    pub preview_colors: ThemePreviewColors,
}

/// Preview colors for a theme
#[derive(Debug, Clone)]
pub struct ThemePreviewColors {
    /// Key color
    pub key: Hsla,
    /// String value color
    pub string: Hsla,
    /// Number value color
    pub number: Hsla,
    /// Boolean value color
    pub boolean: Hsla,
    /// Null value color
    pub null: Hsla,
}

/// Json Highlight Selector - Theme selection component
pub struct JsonHighlightSelector {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Available themes
    themes: Vec<JsonHighlightTheme>,
    /// Selected theme
    selected_theme: String,
    /// Is dropdown open
    is_open: bool,
    /// Is compact mode
    is_compact: bool,
}

impl JsonHighlightSelector {
    /// Create new highlight selector
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            themes: Self::default_themes(),
            selected_theme: "default".to_string(),
            is_open: false,
            is_compact: false,
        }
    }

    /// Create compact mode
    pub fn compact(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            themes: Self::default_themes(),
            selected_theme: "default".to_string(),
            is_open: false,
            is_compact: true,
        }
    }

    /// Get default themes
    fn default_themes() -> Vec<JsonHighlightTheme> {
        vec![
            JsonHighlightTheme {
                name: "默认".to_string(),
                id: "default".to_string(),
                is_default: true,
                preview_colors: ThemePreviewColors {
                    key: gpui::rgb(0x6a9955),
                    string: gpui::rgb(0xce9178),
                    number: gpui::rgb(0xb5cea8),
                    boolean: gpui::rgb(0x569cd6),
                    null: gpui::rgb(0x569cd6),
                },
            },
            JsonHighlightTheme {
                name: "深色".to_string(),
                id: "dark".to_string(),
                is_default: false,
                preview_colors: ThemePreviewColors {
                    key: gpui::rgb(0x9cdcfe),
                    string: gpui::rgb(0xf9aeae),
                    number: gpui::rgb(0xbcd4e6),
                    boolean: gpui::rgb(0x89ddff),
                    null: gpui::rgb(0x89ddff),
                },
            },
            JsonHighlightTheme {
                name: "高亮".to_string(),
                id: "bright".to_string(),
                is_default: false,
                preview_colors: ThemePreviewColors {
                    key: gpui::rgb(0x569cd6),
                    string: gpui::rgb(0xdcdcaa),
                    number: gpui::rgb(0x4ec9b0),
                    boolean: gpui::rgb(0xc586c0),
                    null: gpui::rgb(0xc586c0),
                },
            },
            JsonHighlightTheme {
                name: "简约".to_string(),
                id: "minimal".to_string(),
                is_default: false,
                preview_colors: ThemePreviewColors {
                    key: gpui::rgb(0x555555),
                    string: gpui::rgb(0x333333),
                    number: gpui::rgb(0x333333),
                    boolean: gpui::rgb(0x333333),
                    null: gpui::rgb(0x333333),
                },
            },
        ]
    }

    /// Set selected theme
    pub fn set_theme(&mut self, theme_id: String) {
        self.selected_theme = theme_id;
    }

    /// Toggle dropdown
    pub fn toggle_dropdown(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Close dropdown
    pub fn close_dropdown(&mut self) {
        self.is_open = false;
    }

    /// Get current theme
    pub fn current_theme(&self) -> Option<&JsonHighlightTheme> {
        self.themes.iter().find(|t| t.id == self.selected_theme)
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
                // Preview icon
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        div()
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded(px(1.0))
                            .bg(self.current_theme().map(|t| t.preview_colors.key).unwrap_or(theme.text_muted))
                    )
                    .child(
                        div()
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded(px(1.0))
                            .bg(self.current_theme().map(|t| t.preview_colors.string).unwrap_or(theme.text_muted))
                    )
            )
            .when(!self.is_compact, |this| {
                this.child(
                    div()
                        .text_color(theme.text_secondary)
                        .text_xs()
                        .child(self.current_theme().map(|t| t.name.clone()).unwrap_or_default())
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

    /// Render theme item
    fn render_theme_item(&self, item: &JsonHighlightTheme, is_selected: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .bg(if is_selected { theme.primary.opacity(0.15) } else { gpui::transparent_black() })
            .border_l(px(2.0))
            .border_color(if is_selected { theme.primary } else { gpui::transparent_black() })
            .cursor_pointer()
            .child(
                // Preview swatches
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .w(px(36.0))
                    .child(
                        div()
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded(px(1.0))
                            .bg(item.preview_colors.key)
                    )
                    .child(
                        div()
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded(px(1.0))
                            .bg(item.preview_colors.string)
                    )
                    .child(
                        div()
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded(px(1.0))
                            .bg(item.preview_colors.number)
                    )
                    .child(
                        div()
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded(px(1.0))
                            .bg(item.preview_colors.boolean)
                    )
            )
            .child(
                // Theme name
                div()
                    .flex_1()
                    .text_color(if is_selected { theme.text } else { theme.text_secondary })
                    .text_sm()
                    .child(item.name.clone())
            )
            .when(item.is_default, |this| {
                this.child(
                    div()
                        .px(px(4.0))
                        .py(px(2.0))
                        .rounded(px(3.0))
                        .bg(theme.surface_raised)
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("默认")
                        )
                )
            })
            .when(is_selected, |this| {
                this.child(
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
                    .child("JSON高亮主题")
            )
            .children(self.themes.iter().map(|item| {
                let is_selected = item.id == self.selected_theme;
                self.render_theme_item(item, is_selected)
            }))
    }

    /// Render preview JSON snippet
    fn render_preview(&self) -> Div {
        let theme = &self.theme;
        let colors = self.current_theme().map(|t| t.preview_colors).unwrap_or_default();

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(8.0))
            .rounded(px(4.0))
            .bg(theme.surface_raised)
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child("预览示例")
            )
            .child(
                // Sample JSON
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(div().text_color(colors.key).text_xs().child("\"name\""))
                            .child(div().text_color(theme.text_muted).text_xs().child(":"))
                            .child(div().text_color(colors.string).text_xs().child("\"Kafka\""))
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(div().text_color(colors.key).text_xs().child("\"count\""))
                            .child(div().text_color(theme.text_muted).text_xs().child(":"))
                            .child(div().text_color(colors.number).text_xs().child("100"))
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(div().text_color(colors.key).text_xs().child("\"active\""))
                            .child(div().text_color(theme.text_muted).text_xs().child(":"))
                            .child(div().text_color(colors.boolean).text_xs().child("true"))
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(div().text_color(colors.key).text_xs().child("\"data\""))
                            .child(div().text_color(theme.text_muted).text_xs().child(":"))
                            .child(div().text_color(colors.null).text_xs().child("null"))
                    )
            )
    }
}

impl Default for ThemePreviewColors {
    fn default() -> Self {
        Self {
            key: gpui::rgb(0x6a9955),
            string: gpui::rgb(0xce9178),
            number: gpui::rgb(0xb5cea8),
            boolean: gpui::rgb(0x569cd6),
            null: gpui::rgb(0x569cd6),
        }
    }
}

impl IntoElement for JsonHighlightSelector {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .relative()
            .child(self.render_trigger())
            .when(self.is_open, |this| {
                this.child(self.render_dropdown())
                    .child(self.render_preview())
            })
    }
}

impl Clone for JsonHighlightSelector {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            themes: self.themes.clone(),
            selected_theme: self.selected_theme.clone(),
            is_open: self.is_open,
            is_compact: self.is_compact,
        }
    }
}