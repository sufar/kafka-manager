//! Input Component
//!
//! Reusable input/text field component.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Input component
#[derive(Clone)]
pub struct Input {
    theme: Theme,
    placeholder: String,
    value: String,
}

impl Input {
    /// Create new input
    pub fn new(theme: Theme, placeholder: String) -> Self {
        Self {
            theme,
            placeholder,
            value: "".to_string(),
        }
    }

    /// Set input value
    pub fn with_value(mut self, value: String) -> Self {
        self.value = value;
        self
    }
}

impl IntoElement for Input {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .id("input-field")
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                if self.value.is_empty() {
                    div()
                        .text_color(theme.text_muted)
                        .text_sm()
                        .child(self.placeholder.clone())
                } else {
                    div()
                        .text_color(theme.text)
                        .text_sm()
                        .child(self.value.clone())
                }
            )
    }
}