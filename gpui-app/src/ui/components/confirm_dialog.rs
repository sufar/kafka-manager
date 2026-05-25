//! Confirm Dialog Component
//!
//! Confirmation dialog for user actions like delete, reset, etc.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Confirm dialog variant (severity level)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ConfirmVariant {
    #[default]
    Default,
    Warning,
    Danger,
    Success,
}

impl ConfirmVariant {
    /// Get accent color for this variant
    pub fn accent_color(&self, theme: &Theme) -> Hsla {
        match self {
            ConfirmVariant::Default => theme.primary,
            ConfirmVariant::Warning => theme.warning,
            ConfirmVariant::Danger => theme.error,
            ConfirmVariant::Success => theme.success,
        }
    }

    /// Get icon indicator
    pub fn icon(&self) -> String {
        match self {
            ConfirmVariant::Default => "?".to_string(),
            ConfirmVariant::Warning => "!".to_string(),
            ConfirmVariant::Danger => "×".to_string(),
            ConfirmVariant::Success => "✓".to_string(),
        }
    }
}

/// Confirm Dialog - Confirmation dialog component
pub struct ConfirmDialog {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Dialog title
    title: String,
    /// Dialog message
    message: String,
    /// Confirm variant
    variant: ConfirmVariant,
    /// Confirm button text
    confirm_text: String,
    /// Cancel button text
    cancel_text: String,
    /// Is open
    is_open: bool,
    /// Has details section
    has_details: bool,
    /// Details content
    details: Option<String>,
}

impl ConfirmDialog {
    /// Create new confirm dialog
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            title: "确认操作".to_string(),
            message: "确定要执行此操作吗？".to_string(),
            variant: ConfirmVariant::default(),
            confirm_text: "确认".to_string(),
            cancel_text: "取消".to_string(),
            is_open: false,
            has_details: false,
            details: None,
        }
    }

    /// Create danger variant dialog
    pub fn danger(theme: Theme, translations: Arc<Translations>, title: String, message: String) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            title,
            message,
            variant: ConfirmVariant::Danger,
            confirm_text: "删除".to_string(),
            cancel_text: "取消".to_string(),
            is_open: false,
            has_details: false,
            details: None,
        }
    }

    /// Create warning variant dialog
    pub fn warning(theme: Theme, translations: Arc<Translations>, title: String, message: String) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            title,
            message,
            variant: ConfirmVariant::Warning,
            confirm_text: "继续".to_string(),
            cancel_text: "取消".to_string(),
            is_open: false,
            has_details: false,
            details: None,
        }
    }

    /// Open dialog
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Open with specific content
    pub fn open_with(&mut self, title: String, message: String, variant: ConfirmVariant) {
        self.title = title;
        self.message = message;
        self.variant = variant;
        self.is_open = true;
    }

    /// Close dialog
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Toggle dialog
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Is dialog open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Set title
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Set message
    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }

    /// Set variant
    pub fn set_variant(&mut self, variant: ConfirmVariant) {
        self.variant = variant;
    }

    /// Set confirm text
    pub fn set_confirm_text(&mut self, text: String) {
        self.confirm_text = text;
    }

    /// Set cancel text
    pub fn set_cancel_text(&mut self, text: String) {
        self.cancel_text = text;
    }

    /// Set details
    pub fn set_details(&mut self, details: Option<String>) {
        let has_details = details.is_some();
        self.details = details;
        self.has_details = has_details;
    }

    /// Render icon indicator
    fn render_icon(&self) -> Div {
        let theme = &self.theme;
        let color = self.variant.accent_color(theme);

        div()
            .flex()
            .items_center()
            .justify_center()
            .w(px(48.0))
            .h(px(48.0))
            .rounded(px(24.0))
            .bg(color.opacity(0.15))
            .child(
                div()
                    .text_color(color)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(self.variant.icon())
            )
    }

    /// Render backdrop
    fn render_backdrop(&self) -> Div {
        let theme = &self.theme;

        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(px(2000.0))
            .h(px(2000.0))
            .bg(theme.background.opacity(0.7))
    }

    /// Render dialog panel
    fn render_panel(&self) -> Div {
        let theme = &self.theme;
        let accent = self.variant.accent_color(theme);

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .w(px(400.0))
            .p(px(24.0))
            .rounded(px(12.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Header with icon and title
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child(self.render_icon())
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(self.title.clone())
                            )
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(self.message.clone())
                            )
                    )
            )
            .when_some(self.details.clone(), |this, details| {
                this.child(
                    // Details section
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .p(px(12.0))
                        .rounded(px(8.0))
                        .bg(theme.surface_raised)
                        .border_l(px(3.0))
                        .border_color(accent.opacity(0.3))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("详情")
                        )
                        .child(
                            div()
                                .text_color(theme.text_secondary)
                                .text_xs()
                                .child(details)
                        )
                )
            })
            .child(
                // Actions row
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(12.0))
                    .pt(px(12.0))
                    .border_t(px(1.0))
                    .border_color(theme.border)
                    .child(
                        // Cancel button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(self.cancel_text.clone())
                            )
                    )
                    .child(
                        // Confirm button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(accent)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.confirm_text.clone())
                            )
                    )
            )
    }
}

impl IntoElement for ConfirmDialog {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div()
            .relative()
            .when(self.is_open, |this| {
                this.child(self.render_backdrop())
                    .child(
                        // Centered dialog
                        div()
                            .absolute()
                            .top(px(200.0))
                            .left(px(400.0))
                            .child(self.render_panel())
                    )
            })
    }
}

impl Clone for ConfirmDialog {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            title: self.title.clone(),
            message: self.message.clone(),
            variant: self.variant,
            confirm_text: self.confirm_text.clone(),
            cancel_text: self.cancel_text.clone(),
            is_open: self.is_open,
            has_details: self.has_details,
            details: self.details.clone(),
        }
    }
}