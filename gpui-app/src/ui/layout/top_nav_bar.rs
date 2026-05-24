//! Top Navigation Bar Component
//!
//! Displays the main header with logo, navigation controls, and action buttons.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Top navigation bar component
pub struct TopNavBar {
    is_mobile: bool,
    is_dark: bool,
    sidebar_open: bool,
    translations: Arc<Translations>,
}

impl TopNavBar {
    /// Create new top nav bar
    pub fn new(is_mobile: bool, is_dark: bool, sidebar_open: bool, translations: Arc<Translations>) -> Self {
        Self {
            is_mobile,
            is_dark,
            sidebar_open,
            translations,
        }
    }
}

impl IntoElement for TopNavBar {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let theme = if self.is_dark { Theme::dark() } else { Theme::light() };
        let t = self.translations.clone();

        div()
            .id("top-nav-bar")
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .h(px(48.0))
            .px(px(16.0))
            .bg(theme.surface)
            .border_b(px(1.0))
            .border_color(theme.border)
            .child(
                // Left section: Logo + Menu toggle (mobile)
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .when(self.is_mobile, |this| {
                        this.child(
                            // Menu toggle button - shows different icon based on sidebar state
                            div()
                                .id("menu-toggle")
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(32.0))
                                .h(px(32.0))
                                .rounded(px(6.0))
                                .bg(if self.sidebar_open { theme.primary.opacity(0.2) } else { theme.surface_raised })
                                .border(px(1.0))
                                .border_color(if self.sidebar_open { theme.primary } else { theme.border })
                                .cursor_pointer()
                                .child(
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .bg(if self.sidebar_open { theme.primary } else { theme.text_muted })
                                )
                        )
                    })
                    .child(
                        // Logo
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                // Logo icon
                                div()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(4.0))
                                    .bg(theme.primary)
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Kafka Manager")
                            )
                    )
            )
            .child(
                // Right section: Language + Theme toggles
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // Language toggle
                        div()
                            .id("lang-toggle")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(32.0))
                            .h(px(32.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(t.layout.language_toggle.clone())
                            )
                    )
                    .child(
                        // Theme toggle
                        div()
                            .id("theme-toggle")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(32.0))
                            .h(px(32.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(
                                // Theme icon
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .rounded(px(8.0))
                                    .bg(Hsla::from(gpui::rgb(if self.is_dark {
                                        0xfbbf24
                                    } else {
                                        0x6366f1
                                    })))
                            )
                    )
            )
    }
}