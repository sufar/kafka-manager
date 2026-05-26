//! Top Navigation Bar Component
//!
//! Displays the main header with logo, navigation controls, and action buttons.
//! Integrates with GlobalState Entity for toggle actions.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::state::{GlobalState, Language};
use crate::shortcuts::navigation::ToggleSidebar;

/// Top navigation bar component (static version)
pub struct TopNavBar {
    is_mobile: bool,
    is_dark: bool,
    sidebar_open: bool,
    translations: Arc<Translations>,
}

impl TopNavBar {
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

/// Top navigation bar with GlobalState Entity integration
pub struct TopNavBarWithState {
    state: Entity<GlobalState>,
}

impl TopNavBarWithState {
    pub fn new(state: Entity<GlobalState>) -> Self {
        Self { state }
    }
}

impl Render for TopNavBarWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let theme = &state.theme;
        let is_dark = theme.is_dark;
        let is_mobile = state.is_mobile;
        let sidebar_open = state.sidebar_open;
        let language = state.language;
        let tour_active = state.tour_active;
        let t = state.translations();

        div()
            .id("top-nav-bar-state")
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
                // Left section: Logo + Menu toggle (mobile) + Search
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .when(is_mobile, |this| {
                        this.child(
                            // Menu toggle button
                            div()
                                .id("menu-toggle")
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(32.0))
                                .h(px(32.0))
                                .rounded(px(6.0))
                                .bg(if sidebar_open { theme.primary.opacity(0.2) } else { theme.surface_raised })
                                .border(px(1.0))
                                .border_color(if sidebar_open { theme.primary } else { theme.border })
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface))
                                .child(
                                    div()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .rounded(px(3.0))
                                        .bg(if sidebar_open { theme.primary } else { theme.text_muted })
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.update(cx, |state, cx| {
                                        state.toggle_sidebar();
                                        cx.notify();
                                    });
                                }))
                        )
                    })
                    .child(
                        // Logo
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
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
                    .when(!is_mobile, |this| {
                        // Search button (desktop only)
                        this.child(
                            div()
                                .id("search-btn")
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .px(px(12.0))
                                .py(px(6.0))
                                .ml(px(16.0))
                                .rounded(px(6.0))
                                .bg(theme.surface_raised)
                                .border(px(1.0))
                                .border_color(theme.border)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface))
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child("🔍")
                                )
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_xs()
                                        .child(t.layout.search_placeholder.clone())
                                )
                                .child(
                                    div()
                                        .px(px(4.0))
                                        .py(px(2.0))
                                        .rounded(px(3.0))
                                        .bg(theme.surface)
                                        .child(
                                            div()
                                                .text_color(theme.text_muted)
                                                .text_xs()
                                                .child("⌘K")
                                        )
                                )
                        )
                    })
            )
            .child(
                // Right section: Actions
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    // Language toggle
                    .child(
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
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(match language {
                                        Language::Chinese => "中",
                                        Language::English => "En",
                                    })
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |state, cx| {
                                    state.toggle_language();
                                    cx.notify();
                                });
                            }))
                    )
                    // Theme toggle
                    .child(
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
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                // Sun/Moon icon
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(if is_dark { "🌙" } else { "☀️" })
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |state, cx| {
                                    state.toggle_theme();
                                    cx.notify();
                                });
                            }))
                    )
                    // Tour start button
                    .when(!tour_active, |this| {
                        this.child(
                            div()
                                .id("tour-btn")
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
                                .hover(|d| d.bg(theme.surface))
                                .child(
                                    div()
                                        .text_color(theme.text_secondary)
                                        .text_xs()
                                        .child("❓")
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.update(cx, |state, cx| {
                                        state.start_tour();
                                        cx.notify();
                                    });
                                }))
                        )
                    })
                    // Share button
                    .child(
                        div()
                            .id("share-btn")
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
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child("📤")
                            )
                    )
            )
    }
}