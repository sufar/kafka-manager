//! Toast Component
//!
//! Notification toast messages.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Toast type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

/// Toast notification
#[derive(Clone)]
pub struct Toast {
    theme: Theme,
    message: String,
    toast_type: ToastType,
    is_visible: bool,
}

impl Toast {
    /// Create new toast
    pub fn new(theme: Theme, message: String, toast_type: ToastType) -> Self {
        Self {
            theme,
            message,
            toast_type,
            is_visible: true,
        }
    }

    /// Hide the toast
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Get icon color based on type
    fn icon_color(&self) -> Hsla {
        match self.toast_type {
            ToastType::Success => self.theme.success,
            ToastType::Error => self.theme.error,
            ToastType::Warning => self.theme.warning,
            ToastType::Info => self.theme.primary,
        }
    }

    /// Get background color based on type
    fn bg_color(&self) -> Hsla {
        match self.toast_type {
            ToastType::Success => self.theme.success.opacity(0.1),
            ToastType::Error => self.theme.error.opacity(0.1),
            ToastType::Warning => self.theme.warning.opacity(0.1),
            ToastType::Info => self.theme.primary.opacity(0.1),
        }
    }
}

impl IntoElement for Toast {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        if !self.is_visible {
            return div().id("toast-hidden");
        }

        let theme = &self.theme;
        let icon_color = self.icon_color();
        let bg_color = self.bg_color();

        div()
            .id("toast-container")
            .absolute()
            .top(px(60.0))
            .right(px(16.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(16.0))
            .py(px(12.0))
            .rounded(px(8.0))
            .bg(bg_color)
            .border(px(1.0))
            .border_color(icon_color.opacity(0.3))
            .child(
                // Icon
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded(px(8.0))
                    .bg(icon_color)
            )
            .child(
                // Message
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .child(self.message.clone())
            )
            .child(
                // Close button
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(4.0))
                    .bg(theme.surface_raised)
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .bg(theme.text_muted)
                    )
            )
    }
}

/// Toast manager for handling multiple toasts
pub struct ToastManager {
    toasts: Vec<Toast>,
    theme: Theme,
    position: ToastPosition,
}

/// Toast position
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastPosition {
    TopRight,
    BottomRight,
    BottomLeft,
}

impl ToastManager {
    /// Create new toast manager
    pub fn new(theme: Theme) -> Self {
        Self {
            toasts: Vec::new(),
            theme,
            position: ToastPosition::TopRight,
        }
    }

    /// Set position to bottom right
    pub fn position_bottom_right(mut self) -> Self {
        self.position = ToastPosition::BottomRight;
        self
    }

    /// Set position to bottom left
    pub fn position_bottom_left(mut self) -> Self {
        self.position = ToastPosition::BottomLeft;
        self
    }

    /// Show a success toast
    pub fn success(&mut self, message: String) {
        self.toasts.push(Toast::new(self.theme.clone(), message, ToastType::Success));
    }

    /// Show an error toast
    pub fn error(&mut self, message: String) {
        self.toasts.push(Toast::new(self.theme.clone(), message, ToastType::Error));
    }

    /// Show a warning toast
    pub fn warning(&mut self, message: String) {
        self.toasts.push(Toast::new(self.theme.clone(), message, ToastType::Warning));
    }

    /// Show an info toast
    pub fn info(&mut self, message: String) {
        self.toasts.push(Toast::new(self.theme.clone(), message, ToastType::Info));
    }

    /// Clear all toasts
    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    /// Remove expired toasts
    pub fn remove_expired(&mut self) {
        self.toasts.retain(|t| t.is_visible());
    }
}

impl IntoElement for ToastManager {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        // Toast overlay container
        div()
            .id("toast-manager")
            .absolute()
            .when(self.position == ToastPosition::TopRight, |this| {
                this.top(px(60.0)).right(px(16.0))
            })
            .when(self.position == ToastPosition::BottomRight, |this| {
                this.bottom(px(16.0)).right(px(16.0))
            })
            .when(self.position == ToastPosition::BottomLeft, |this| {
                this.bottom(px(16.0)).left(px(16.0))
            })
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(self.toasts)
    }
}