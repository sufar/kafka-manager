//! Toast Component
//!
//! Notification toast messages with GlobalState integration.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::state::{GlobalState, ToastType};

/// Toast notification (static version)
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
}

impl IntoElement for Toast {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        if !self.is_visible {
            return div().id("toast-hidden");
        }

        let theme = &self.theme;
        let icon_color = match self.toast_type {
            ToastType::Success => theme.success,
            ToastType::Error => theme.error,
            ToastType::Warning => theme.warning,
            ToastType::Info => theme.primary,
        };
        let bg_color = match self.toast_type {
            ToastType::Success => theme.success.opacity(0.1),
            ToastType::Error => theme.error.opacity(0.1),
            ToastType::Warning => theme.warning.opacity(0.1),
            ToastType::Info => theme.primary.opacity(0.1),
        };

        div()
            .id("toast-container")
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
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded(px(8.0))
                    .bg(icon_color)
            )
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .child(self.message.clone())
            )
            .child(
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

/// Toast manager for handling multiple toasts (static version for backward compatibility)
pub struct ToastManager {
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

    /// Show a success toast (backward compatibility - creates a local Toast)
    pub fn success(&mut self, message: String) {
        println!("Toast success: {}", message);
    }

    /// Show an error toast
    pub fn error(&mut self, message: String) {
        println!("Toast error: {}", message);
    }

    /// Show a warning toast
    pub fn warning(&mut self, message: String) {
        println!("Toast warning: {}", message);
    }

    /// Show an info toast
    pub fn info(&mut self, message: String) {
        println!("Toast info: {}", message);
    }

    /// Clear all toasts
    pub fn clear(&mut self) {
        println!("Toast clear");
    }

    /// Remove expired toasts
    pub fn remove_expired(&mut self) {
        println!("Toast remove_expired");
    }
}

impl IntoElement for ToastManager {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        // Empty toast overlay container
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
    }
}

/// Toast manager with GlobalState Entity integration
pub struct ToastManagerWithState {
    state: Entity<GlobalState>,
    position: ToastPosition,
}

impl ToastManagerWithState {
    pub fn new(state: Entity<GlobalState>) -> Self {
        Self {
            state,
            position: ToastPosition::BottomRight,
        }
    }

    pub fn position_bottom_right(mut self) -> Self {
        self.position = ToastPosition::BottomRight;
        self
    }
}

impl Render for ToastManagerWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let theme = &state.theme;
        let toasts = state.get_toasts();

        div()
            .id("toast-manager-state")
            .absolute()
            .when(self.position == ToastPosition::BottomRight, |this| {
                this.bottom(px(16.0)).right(px(16.0))
            })
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(toasts.iter().map(|toast| {
                let icon_color = match toast.toast_type {
                    ToastType::Success => theme.success,
                    ToastType::Error => theme.error,
                    ToastType::Warning => theme.warning,
                    ToastType::Info => theme.primary,
                };
                let bg_color = match toast.toast_type {
                    ToastType::Success => theme.success.opacity(0.1),
                    ToastType::Error => theme.error.opacity(0.1),
                    ToastType::Warning => theme.warning.opacity(0.1),
                    ToastType::Info => theme.primary.opacity(0.1),
                };
                let toast_id_for_close = toast.id;

                div()
                    .id(format!("toast-{}", toast_id_for_close))
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
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(8.0))
                            .bg(icon_color)
                    )
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .child(toast.message.clone())
                    )
                    .child(
                        div()
                            .id(format!("toast-close-{}", toast_id_for_close))
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("×")
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.state.update(cx, |state, cx| {
                                    state.hide_toast(toast_id_for_close);
                                    cx.notify();
                                });
                            }))
                    )
            }))
    }
}