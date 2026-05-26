//! Top Navigation Bar Component
//!
//! Displays the main header with logo, search, and action buttons.
//! Matches Vue3 TopNavBar.vue visual styling.

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

        // Vue3: h-10 bg-base-100/80 border-b border-base-200 px-2
        div()
            .id("top-nav-bar")
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .h(px(40.0))    // Vue3: h-10 = 40px
            .px(px(8.0))    // Vue3: px-2 = 8px
            .bg(theme.surface.opacity(0.80))  // Vue3: bg-base-100/80 = glass effect
            .border_b(px(1.0))
            .border_color(theme.border)
            .child(
                // Left section: Menu toggle (mobile) + Logo + Search
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))  // Vue3: gap-2 = 8px
                    .when(self.is_mobile, |this| {
                        // Mobile menu button - Vue3: btn btn-ghost btn-circle btn-xs h-7 w-7
                        this.child(
                            div()
                                .w(px(28.0))   // Vue3: h-7 w-7 = 28px (mobile)
                                .h(px(28.0))
                                .rounded(px(14.0))  // btn-circle
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    // Menu icon placeholder - Vue3: w-4 h-4 = 16px
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(3.0))
                                        .bg(theme.text_muted)
                                )
                        )
                    })
                    .child(
                        // Logo and Brand - Vue3: gap-1.5 = 6px
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                // Logo icon - Vue3: w-6 h-6 rounded = 24px, 4px rounded
                                div()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(4.0))
                                    .bg(theme.primary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        // Inner icon placeholder - Vue3: w-3.5 h-3.5 = 14px
                                        div()
                                            .w(px(14.0))
                                            .h(px(14.0))
                                            .rounded(px(3.0))
                                            .bg(Hsla::from(gpui::rgb(0xffffff)))  // primary-content
                                    )
                            )
                            .child(
                                // Title - Vue3: text-base font-bold = 16px
                                div()
                                    .text_color(theme.text)
                                    .text_size(px(16.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Kafka Manager")
                            )
                    )
                    .when(!self.is_mobile, |this| {
                        // Search input - Vue3: input input-bordered input-xs w-56 h-7
                        // Desktop only
                        this.child(
                            div()
                                .ml(px(8.0))  // Vue3: ml-2 = 8px
                                .w(px(224.0)) // Vue3: w-56 = 224px
                                .h(px(28.0))  // Vue3: h-7 = 28px
                                .rounded(px(6.0))  // input-xs rounded
                                .bg(theme.surface)
                                .border(px(1.0))
                                .border_color(theme.border)
                                .px(px(12.0))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    // Search icon placeholder - Vue3: w-4 h-4 = 16px
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(3.0))
                                        .bg(theme.text_muted.opacity(0.50))
                                )
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_size(px(12.0))
                                        .child(t.layout.search_placeholder.clone())
                                )
                        )
                    })
                    .when(self.is_mobile, |this| {
                        // Mobile search button - Vue3: btn btn-ghost btn-circle btn-xs h-7 w-7
                        this.child(
                            div()
                                .ml(px(4.0))  // Vue3: ml-1 = 4px
                                .w(px(28.0))
                                .h(px(28.0))
                                .rounded(px(14.0))  // btn-circle
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    // Search icon placeholder - Vue3: w-4 h-4 = 16px
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(3.0))
                                        .bg(theme.text_muted)
                                )
                        )
                    })
            )
            .child(
                // Right section: Action buttons - Vue3: gap-0.5 = 2px
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))  // Vue3: gap-0.5 = 2px
                    .flex_shrink_0()
                    .when(self.is_mobile, |this| {
                        // Mobile more menu - Vue3: btn btn-ghost btn-circle btn-xs h-7 w-7
                        this.child(
                            div()
                                .w(px(28.0))
                                .h(px(28.0))
                                .rounded(px(14.0))  // btn-circle
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    // More icon placeholder - Vue3: w-4 h-4 = 16px
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(3.0))
                                        .bg(theme.text_muted)
                                )
                        )
                    })
                    .when(!self.is_mobile, |this| {
                        // Desktop buttons - Vue3: btn btn-ghost btn-circle btn-xs = 24px
                        // Language selector
                        this.child(
                            div()
                                .w(px(24.0))   // Vue3: btn-xs = 24px (desktop)
                                .h(px(24.0))
                                .rounded(px(12.0))  // btn-circle
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .text_color(theme.text_secondary)
                                        .text_size(px(10.0))
                                        .child(t.layout.language_toggle.clone())
                                )
                        )
                        .child(
                            // Theme toggle - Vue3: btn btn-ghost btn-circle btn-xs
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded(px(12.0))  // btn-circle
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    // Theme icon placeholder - Vue3: w-4 h-4 = 16px
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(8.0))  // circle for sun/moon
                                        .bg(Hsla::from(gpui::rgb(if self.is_dark {
                                            0xfbbf24  // yellow-400 (sun)
                                        } else {
                                            0x6366f1  // indigo-500 (moon)
                                        })))
                                )
                        )
                        .child(
                            // Share button - Vue3: btn btn-ghost btn-circle btn-xs
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded(px(12.0))
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(3.0))
                                        .bg(theme.text_muted)
                                )
                        )
                        .child(
                            // Tour button - Vue3: btn btn-ghost btn-circle btn-xs
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded(px(12.0))
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(3.0))
                                        .bg(theme.text_muted)
                                )
                        )
                    })
            )
    }
}