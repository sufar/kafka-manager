//! Button Component
//!
//! Reusable button component.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Button variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

/// Button component
pub struct Button {
    theme: Theme,
    label: String,
    variant: ButtonVariant,
}

impl Button {
    /// Create new button
    pub fn new(theme: Theme, label: String, variant: ButtonVariant) -> Self {
        Self {
            theme,
            label,
            variant,
        }
    }
}

impl IntoElement for Button {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        let (bg, bg_hover, text_color, border_color) = match self.variant {
            ButtonVariant::Primary => (
                theme.primary,
                theme.primary_hover,
                Hsla::from(gpui::rgb(0xffffff)),
                gpui::transparent_black()
            ),
            ButtonVariant::Secondary => (
                theme.surface_raised,
                theme.secondary.opacity(0.1),
                theme.text_secondary,
                theme.border
            ),
            ButtonVariant::Danger => (
                theme.error,
                theme.error.opacity(0.8),
                Hsla::from(gpui::rgb(0xffffff)),
                gpui::transparent_black()
            ),
            ButtonVariant::Ghost => (
                gpui::transparent_black(),
                theme.surface,
                theme.text_secondary,
                gpui::transparent_black()
            ),
        };

        div()
            .flex()
            .items_center()
            .justify_center()
            .px(px(16.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(bg)
            .border(px(1.0))
            .border_color(border_color)
            .cursor_pointer()
            .hover(|d| d.bg(bg_hover))
            .child(
                div()
                    .text_color(text_color)
                    .text_sm()
                    .child(self.label.clone())
            )
    }
}