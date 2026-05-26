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

        // TopNavBar styling matching Vue3:
        // Vue: fixed top-0 left-0 right-0 h-10 bg-base-100/80 border-b border-base-200 z-50 px-2
        div()
            .id("top-nav-bar")
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .h(px(40.0))  // Vue: h-10 = 40px
            .px(px(8.0))  // Vue: px-2 = 8px
            .bg(theme.surface.opacity(0.80))  // Vue: bg-base-100/80 for glass effect
            .border_b(px(1.0))
            .border_color(theme.border.opacity(0.10))  // Vue: border-base-200
            .child(
                // Left section: Logo + Menu toggle (mobile)
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))  // Vue: gap-2 = 8px
                    .when(self.is_mobile, |this| {
                        this.child(
                            // Mobile menu button - Vue: btn btn-ghost btn-circle btn-xs h-7 w-7
                            div()
                                .id("menu-toggle")
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(28.0))  // Vue: w-7 = 28px
                                .h(px(28.0))  // Vue: h-7 = 28px
                                .rounded(px(14.0))  // Vue: btn-circle = fully rounded
                                .bg(if self.sidebar_open { theme.primary.opacity(0.10) } else { gpui::transparent_black() })
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface))
                                .child(
                                    // Hamburger icon placeholder
                                    div()
                                        .text_color(if self.sidebar_open { theme.primary } else { theme.text_muted })
                                        .text_size(px(16.0))  // Vue: w-4 h-4 = 16px
                                        .child("☰")
                                )
                        )
                    })
                    .child(
                        // Logo - Vue: w-6 h-6 rounded bg-primary shadow-lg
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))  // Vue: gap-1.5 = 6px
                            .child(
                                div()
                                    .w(px(24.0))  // Vue: w-6 = 24px
                                    .h(px(24.0))  // Vue: h-6 = 24px
                                    .rounded(px(4.0))  // Vue: rounded = 4px
                                    .bg(theme.primary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                            .text_size(px(14.0))  // Vue: w-3.5 h-3.5 = 14px
                                            .child("📊")
                                    )
                            )
                            .child(
                                // Title - Vue: text-base font-bold (16px)
                                div()
                                    .text_color(theme.text)
                                    .text_size(px(16.0))  // Vue: text-base = 16px
                                    .font_weight(FontWeight::BOLD)
                                    .child("Kafka Manager")
                            )
                    )
            )
            .child(
                // Right section: Language + Theme toggles - Vue: gap-0.5 = 2px
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))  // Vue: gap-0.5 = 2px
                    .child(
                        // Language toggle - Vue: btn btn-ghost btn-circle btn-xs h-6 w-6
                        div()
                            .id("lang-toggle")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(24.0))  // Vue: w-6 = 24px
                            .h(px(24.0))  // Vue: h-6 = 24px
                            .rounded(px(12.0))  // Vue: btn-circle = fully rounded
                            .bg(gpui::transparent_black())
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_size(px(10.0))  // Vue: text-[10px]
                                    .font_weight(FontWeight::BOLD)
                                    .child("EN")
                            )
                    )
                    .child(
                        // Theme toggle - Vue: btn btn-ghost btn-circle btn-xs h-6 w-6
                        div()
                            .id("theme-toggle")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(24.0))  // Vue: w-6 = 24px
                            .h(px(24.0))  // Vue: h-6 = 24px
                            .rounded(px(12.0))  // Vue: btn-circle = fully rounded
                            .bg(gpui::transparent_black())
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                // Sun/Moon icon placeholder
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_size(px(14.0))  // Vue: w-3.5 h-3.5 = 14px
                                    .child(if self.is_dark { "☀️" } else { "🌙" })
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

        // TopNavBar styling matching Vue3:
        // Vue: fixed top-0 left-0 right-0 h-10 bg-base-100/80 border-b border-base-200 z-50 px-2
        div()
            .id("top-nav-bar-state")
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .h(px(40.0))  // Vue: h-10 = 40px
            .px(px(8.0))  // Vue: px-2 = 8px
            .bg(theme.surface.opacity(0.80))  // Vue: bg-base-100/80 for glass effect
            .border_b(px(1.0))
            .border_color(theme.border.opacity(0.10))  // Vue: border-base-200
            .child(
                // Left section: Logo + Menu toggle (mobile) + Search
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))  // Vue: gap-2 = 8px
                    .when(is_mobile, |this| {
                        this.child(
                            // Menu toggle button - Vue: btn btn-ghost btn-circle btn-xs h-7 w-7
                            div()
                                .id("menu-toggle")
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(28.0))  // Vue: w-7 = 28px
                                .h(px(28.0))  // Vue: h-7 = 28px
                                .rounded(px(14.0))  // Vue: btn-circle = fully rounded
                                .bg(if sidebar_open { theme.primary.opacity(0.10) } else { gpui::transparent_black() })
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface))
                                .child(
                                    // Hamburger icon placeholder
                                    div()
                                        .text_color(if sidebar_open { theme.primary } else { theme.text_muted })
                                        .text_size(px(16.0))  // Vue: w-4 h-4 = 16px
                                        .child("☰")
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
                        // Logo - Vue: w-6 h-6 rounded bg-primary shadow-lg
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))  // Vue: gap-1.5 = 6px
                            .child(
                                div()
                                    .w(px(24.0))  // Vue: w-6 = 24px
                                    .h(px(24.0))  // Vue: h-6 = 24px
                                    .rounded(px(4.0))  // Vue: rounded = 4px
                                    .bg(theme.primary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                            .text_size(px(14.0))  // Vue: w-3.5 h-3.5 = 14px
                                            .child("📊")
                                    )
                            )
                            .child(
                                // Title - Vue: text-base font-bold (16px)
                                div()
                                    .text_color(theme.text)
                                    .text_size(px(16.0))  // Vue: text-base = 16px
                                    .font_weight(FontWeight::BOLD)
                                    .child("Kafka Manager")
                            )
                    )
                    .when(!is_mobile, |this| {
                        // Search button (desktop only) - Vue: input input-bordered input-xs w-56 h-7
                        this.child(
                            div()
                                .id("search-btn")
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .w(px(224.0))  // Vue: w-56 = 224px
                                .h(px(28.0))   // Vue: h-7 = 28px
                                .ml(px(8.0))   // Vue: ml-2 = 8px
                                .px(px(12.0))  // Vue: px-3 = 12px
                                .rounded(px(6.0))  // Vue: input rounded
                                .bg(theme.surface)
                                .border(px(1.0))
                                .border_color(theme.border.opacity(0.20))  // Vue: input-bordered
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface_raised))
                                .child(
                                    // Search icon placeholder
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_size(px(14.0))
                                        .child("🔍")
                                )
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .text_size(px(12.0))  // Vue: text-xs = 12px
                                        .child(t.layout.search_placeholder.clone())
                                )
                                .child(
                                    // Keyboard shortcut badge
                                    div()
                                        .px(px(4.0))  // Vue: px-1 = 4px
                                        .py(px(2.0))  // small padding
                                        .rounded(px(3.0))
                                        .bg(theme.surface_raised)
                                        .child(
                                            div()
                                                .text_color(theme.text_muted)
                                                .text_size(px(10.0))
                                                .child("⌘K")
                                        )
                                )
                        )
                    })
            )
            .child(
                // Right section: Actions - Vue: gap-0.5 = 2px between buttons
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))  // Vue: gap-0.5 = 2px
                    // Language toggle - Vue: btn btn-ghost btn-circle btn-xs h-6 w-6
                    .child(
                        div()
                            .id("lang-toggle")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(24.0))  // Vue: w-6 = 24px
                            .h(px(24.0))  // Vue: h-6 = 24px
                            .rounded(px(12.0))  // Vue: btn-circle
                            .bg(gpui::transparent_black())
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_size(px(10.0))  // Vue: text-[10px]
                                    .font_weight(FontWeight::BOLD)
                                    .child(match language {
                                        Language::Chinese => "EN",
                                        Language::English => "中",
                                    })
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |state, cx| {
                                    state.toggle_language();
                                    cx.notify();
                                });
                            }))
                    )
                    // Theme toggle - Vue: btn btn-ghost btn-circle btn-xs h-6 w-6
                    .child(
                        div()
                            .id("theme-toggle")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(24.0))  // Vue: w-6 = 24px
                            .h(px(24.0))  // Vue: h-6 = 24px
                            .rounded(px(12.0))  // Vue: btn-circle
                            .bg(gpui::transparent_black())
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                // Sun/Moon icon placeholder - Vue: w-3.5 h-3.5 = 14px
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_size(px(14.0))
                                    .child(if is_dark { "☀️" } else { "🌙" })
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |state, cx| {
                                    state.toggle_theme();
                                    cx.notify();
                                });
                            }))
                    )
                    // Tour start button - Vue: btn btn-ghost btn-circle btn-xs h-6 w-6
                    .when(!tour_active, |this| {
                        this.child(
                            div()
                                .id("tour-btn")
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(24.0))  // Vue: w-6 = 24px
                                .h(px(24.0))  // Vue: h-6 = 24px
                                .rounded(px(12.0))  // Vue: btn-circle
                                .bg(gpui::transparent_black())
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface))
                                .child(
                                    // Help icon placeholder
                                    div()
                                        .text_color(theme.text_secondary)
                                        .text_size(px(14.0))
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
                    // Share button - Vue: btn btn-ghost btn-circle btn-xs h-6 w-6
                    .child(
                        div()
                            .id("share-btn")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(24.0))  // Vue: w-6 = 24px
                            .h(px(24.0))  // Vue: h-6 = 24px
                            .rounded(px(12.0))  // Vue: btn-circle
                            .bg(gpui::transparent_black())
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface))
                            .child(
                                // Share icon placeholder
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_size(px(14.0))
                                    .child("📤")
                            )
                    )
            )
    }
}