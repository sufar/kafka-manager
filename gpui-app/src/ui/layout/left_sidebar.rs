//! Left Sidebar Component
//!
//! Displays navigation items, cluster tree navigator, and resizer.
//! Matches Vue3 LeftSidebar.vue visual styling.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::router::ViewType;
use crate::ui::components::ClusterTreeNavigator;

/// Left sidebar component
pub struct LeftSidebar {
    width: Pixels,
    is_mobile: bool,
    sidebar_open: bool,
    theme: Theme,
    translations: Arc<Translations>,
    current_view: ViewType,
}

impl LeftSidebar {
    /// Create sidebar for mobile view (overlay)
    pub fn mobile(width: Pixels, sidebar_open: bool, theme: Theme, translations: Arc<Translations>, current_view: ViewType) -> Self {
        Self {
            width,
            is_mobile: true,
            sidebar_open,
            theme,
            translations,
            current_view,
        }
    }

    /// Create sidebar for desktop view (fixed)
    pub fn desktop(width: Pixels, theme: Theme, translations: Arc<Translations>, current_view: ViewType) -> Self {
        Self {
            width,
            is_mobile: false,
            sidebar_open: true,
            theme,
            translations,
            current_view,
        }
    }
}

impl IntoElement for LeftSidebar {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let theme = self.theme.clone();
        let translations = self.translations.clone();

        if self.is_mobile && !self.sidebar_open {
            // Mobile sidebar closed - return empty
            return div().id("mobile-sidebar-hidden");
        }

        if self.is_mobile {
            // Mobile: overlay sidebar - Vue3: fixed left-2 bottom-2 top-12 rounded-xl shadow-2xl
            // Vue3: h-[calc(100dvh-3.5rem)] w-72 (288px)
            div()
                .id("mobile-sidebar")
                .absolute()
                .left(px(8.0))   // Vue3: left-2 = 8px
                .bottom(px(8.0)) // Vue3: bottom-2 = 8px
                .top(px(48.0))   // Vue3: top-12 = 48px (3rem = 12 * 4px)
                .w(px(288.0))    // Vue3: w-72 = 288px (72 * 4px)
                .h(px(500.0))    // Approximate calc height
                .rounded(px(12.0))  // Vue3: rounded-xl = 12px
                .bg(theme.surface.opacity(0.95))  // Glass effect
                .overflow_y_scroll()
                .child(
                    // Mobile Sidebar Header - Vue3: p-3 border-b border-base-200
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .p(px(12.0))  // Vue3: p-3 = 12px
                        .border_b(px(1.0))
                        .border_color(theme.border)
                        .flex_shrink_0()
                        .child(
                            div()
                                .text_color(theme.text)
                                .font_weight(FontWeight::BOLD)
                                .child("Menu")
                        )
                        .child(
                            // Close button - Vue3: btn btn-ghost btn-circle btn-sm
                            div()
                                .w(px(32.0))
                                .h(px(32.0))
                                .rounded(px(16.0))  // btn-circle
                                .bg(theme.surface)
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    // X icon placeholder - Vue3: w-5 h-5 (20px)
                                    div()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .rounded(px(4.0))
                                        .bg(theme.text_muted)
                                )
                        )
                )
                .child(
                    // ClusterTreeNavigator
                    ClusterTreeNavigator::new(theme, translations)
                )
        } else {
            // Desktop: fixed sidebar - Vue3: glass gradient-border rounded-xl ml-2 mt-2 mb-2
            // Vue3: h-[calc(100%-1rem)] with resizer w-1
            div()
                .id("desktop-sidebar")
                .flex()
                .h(px(500.0))    // Approximate calc(100% - 1rem)
                .w(self.width)
                .ml(px(8.0))     // Vue3: ml-2 = 8px
                .mt(px(8.0))     // Vue3: mt-2 = 8px
                .mb(px(8.0))     // Vue3: mb-2 = 8px
                .flex_shrink_0()
                .rounded(px(12.0))  // Vue3: rounded-xl = 12px
                .bg(theme.surface.opacity(0.80))  // Glass effect: bg-base-100/80
                .border(px(1.0))
                .border_color(theme.border.opacity(0.50))  // gradient-border effect
                .overflow_hidden()
                .child(
                    // Main content
                    div()
                        .flex()
                        .flex_col()
                        .size_full()
                        .child(
                            // ClusterTreeNavigator
                            ClusterTreeNavigator::new(theme.clone(), translations)
                        )
                )
        }
    }
}

/// Resizer handle - Vue3: w-1 cursor-col-resize bg-base-content/5 hover:bg-base-content/10
pub fn render_resizer(theme: Theme) -> Div {
    div()
        .w(px(4.0))     // Vue3: w-1 = 4px
        .h_full()
        .cursor_col_resize()
        .bg(theme.text.opacity(0.05))  // Vue3: bg-base-content/5
        .hover(|d| d.bg(theme.text.opacity(0.10)))  // Vue3: hover:bg-base-content/10
        .flex()
        .items_center()
        .justify_center()
        .child(
            // Drag indicator - Vue3: w-px h-8 bg-base-content/20 group-hover:bg-primary/40
            div()
                .w(px(1.0))     // Vue3: w-px = 1px
                .h(px(32.0))    // Vue3: h-8 = 32px
                .rounded(px(1.0))
                .bg(theme.text.opacity(0.20))  // Vue3: bg-base-content/20
        )
}