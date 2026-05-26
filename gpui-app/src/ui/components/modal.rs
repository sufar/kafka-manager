//! Modal Component
//!
//! Reusable modal/dialog component.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Modal overlay container
pub struct Modal {
    title: String,
    is_open: bool,
}

impl Modal {
    /// Create new modal
    pub fn new(title: String) -> Self {
        Self {
            title,
            is_open: false,
        }
    }

    /// Open the modal
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Close the modal
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Check if modal is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Get modal title
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// Modal content wrapper
pub struct ModalContent {
    theme: Theme,
    title: String,
    children: Vec<Div>,
}

impl ModalContent {
    /// Create modal content
    pub fn new(theme: Theme, title: String) -> Self {
        Self {
            theme,
            title,
            children: Vec::new(),
        }
    }

    /// Add a child element
    pub fn child(mut self, child: Div) -> Self {
        self.children.push(child);
        self
    }
}

impl IntoElement for ModalContent {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        // Modal overlay
        div()
            .id("modal-overlay")
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background.opacity(0.8))
            .child(
                // Modal container
                div()
                    .flex()
                    .flex_col()
                    .w(px(480.0))
                    .max_h(px(600.0))
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        // Modal header
                        ModalHeader::new(theme.clone(), self.title.clone())
                    )
                    .children(self.children)
            )
    }
}

/// Modal header component
pub struct ModalHeader {
    theme: Theme,
    title: String,
}

impl ModalHeader {
    /// Create modal header
    pub fn new(theme: Theme, title: String) -> Self {
        Self {
            theme,
            title,
        }
    }
}

impl IntoElement for ModalHeader {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .py(px(12.0))
            .border_b(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text)
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title.clone())
            )
            .child(
                // Close button
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(28.0))
                    .h(px(28.0))
                    .rounded(px(6.0))
                    .bg(theme.surface_raised)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .bg(theme.text_muted)
                    )
            )
    }
}

/// Modal footer component
pub struct ModalFooter {
    theme: Theme,
    cancel_text: String,
    confirm_text: String,
}

impl ModalFooter {
    /// Create modal footer
    pub fn new(theme: Theme, cancel_text: String, confirm_text: String) -> Self {
        Self {
            theme,
            cancel_text,
            confirm_text,
        }
    }
}

impl IntoElement for ModalFooter {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(12.0))
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
                    .bg(theme.primary)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                            .text_sm()
                            .child(self.confirm_text.clone())
                    )
            )
    }
}