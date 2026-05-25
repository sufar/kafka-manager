//! Language Selector Component
//!
//! Component for switching between languages (Chinese/English).

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::state::Language;

/// Language option
#[derive(Debug, Clone)]
pub struct LanguageOption {
    /// Language code
    pub code: String,
    /// Language display name
    pub name: String,
    /// Language enum
    pub language: Language,
}

impl LanguageOption {
    /// Get flag emoji
    pub fn flag(&self) -> String {
        match self.language {
            Language::Chinese => "🇨🇳".to_string(),
            Language::English => "🇺🇸".to_string(),
        }
    }
}

/// Language Selector - Language switching component
pub struct LanguageSelector {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Current language
    current_language: Language,
    /// Available languages
    languages: Vec<LanguageOption>,
    /// Is dropdown open
    is_open: bool,
    /// Is compact mode (icon only)
    is_compact: bool,
}

impl LanguageSelector {
    /// Create new language selector
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            current_language: Language::Chinese,
            languages: vec![
                LanguageOption {
                    code: "zh".to_string(),
                    name: "中文".to_string(),
                    language: Language::Chinese,
                },
                LanguageOption {
                    code: "en".to_string(),
                    name: "English".to_string(),
                    language: Language::English,
                },
            ],
            is_open: false,
            is_compact: false,
        }
    }

    /// Create compact mode (icon only)
    pub fn compact(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            current_language: Language::Chinese,
            languages: vec![
                LanguageOption {
                    code: "zh".to_string(),
                    name: "中文".to_string(),
                    language: Language::Chinese,
                },
                LanguageOption {
                    code: "en".to_string(),
                    name: "English".to_string(),
                    language: Language::English,
                },
            ],
            is_open: false,
            is_compact: true,
        }
    }

    /// Set current language
    pub fn set_language(&mut self, language: Language) {
        self.current_language = language;
    }

    /// Toggle dropdown
    pub fn toggle_dropdown(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Close dropdown
    pub fn close_dropdown(&mut self) {
        self.is_open = false;
    }

    /// Get current language
    pub fn current(&self) -> Language {
        self.current_language
    }

    /// Get current language option
    fn current_option(&self) -> Option<&LanguageOption> {
        self.languages.iter().find(|l| l.language == self.current_language)
    }

    /// Render trigger button
    fn render_trigger(&self) -> Div {
        let theme = &self.theme;
        let current = self.current_option();

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
            .when_some(current, |this, opt| {
                this.child(
                    // Flag
                    div()
                        .text_color(theme.text)
                        .text_sm()
                        .child(opt.flag())
                )
                .when(!self.is_compact, |this| {
                    this.child(
                        // Language name
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child(opt.name.clone())
                    )
                })
            })
            .child(
                // Dropdown arrow indicator
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
            .shadow(px(0.0), px(4.0), px(12.0), theme.border.opacity(0.3))
            .children(self.languages.iter().map(|opt| {
                let is_selected = opt.language == self.current_language;

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
                        // Flag
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .child(opt.flag())
                    )
                    .child(
                        // Language name
                        div()
                            .flex_1()
                            .text_color(if is_selected { theme.text } else { theme.text_secondary })
                            .text_sm()
                            .child(opt.name.clone())
                    )
                    .child(
                        // Language code
                        div()
                            .px(px(4.0))
                            .py(px(2.0))
                            .rounded(px(3.0))
                            .bg(theme.surface_raised)
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(opt.code.clone())
                            )
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
            }))
    }
}

impl IntoElement for LanguageSelector {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div()
            .flex()
            .flex_col()
            .relative()
            .child(self.render_trigger())
            .when(self.is_open, |this| {
                this.child(self.render_dropdown())
            })
    }
}

impl Clone for LanguageSelector {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            current_language: self.current_language,
            languages: self.languages.clone(),
            is_open: self.is_open,
            is_compact: self.is_compact,
        }
    }
}