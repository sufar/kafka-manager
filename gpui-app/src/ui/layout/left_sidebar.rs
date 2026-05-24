//! Left Sidebar Component
//!
//! Displays navigation items, cluster list, and quick actions.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::router::ViewType;

/// Left sidebar component
pub struct LeftSidebar {
    width: Pixels,
    is_mobile: bool,
    theme: Theme,
    translations: Arc<Translations>,
    current_view: ViewType,
}

impl LeftSidebar {
    /// Create sidebar for mobile view (overlay)
    pub fn mobile(width: Pixels, theme: Theme, translations: Arc<Translations>, current_view: ViewType) -> Self {
        Self {
            width,
            is_mobile: true,
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
        let t = self.translations.clone();
        let current = self.current_view;

        let nav_item = |text: String, _view_type: ViewType, theme: Theme, active: bool| -> Div {
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(6.0))
                .bg(if active { theme.surface_raised } else { gpui::transparent_black() })
                .border(px(1.0))
                .border_color(if active { theme.border_focused } else { gpui::transparent_black() })
                .cursor_pointer()
                .hover(|d| d.bg(theme.surface))
                .child(
                    div()
                        .w(px(16.0))
                        .h(px(16.0))
                        .rounded(px(3.0))
                        .bg(if active { theme.primary } else { theme.text_muted })
                )
                .child(
                    div()
                        .text_color(if active { theme.text } else { theme.text_secondary })
                        .text_sm()
                        .child(text)
                )
        };

        let content = div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(16.0))
            .gap(px(16.0))
            .child(
                // Navigation items
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(nav_item(t.clusters.title.clone(), ViewType::Clusters, theme.clone(), current == ViewType::Clusters))
                    .child(nav_item(t.topics.title.clone(), ViewType::Topics, theme.clone(), current == ViewType::Topics))
                    .child(nav_item(t.messages.title.clone(), ViewType::Messages, theme.clone(), current == ViewType::Messages))
                    .child(nav_item(t.consumer_groups.title.clone(), ViewType::ConsumerGroups, theme.clone(), current == ViewType::ConsumerGroups))
                    .child(nav_item(t.schema_registry.title.clone(), ViewType::SchemaRegistry, theme.clone(), current == ViewType::SchemaRegistry))
                    .child(nav_item("Favorites".to_string(), ViewType::Favorites, theme.clone(), current == ViewType::Favorites))
            )
            .child(
                // Separator
                div()
                    .h(px(1.0))
                    .w_full()
                    .bg(theme.border)
            )
            .child(
                // Quick actions section
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.common.actions.clone())
                    )
                    .child(
                        // Add cluster button
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.primary)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.primary.opacity(0.9)))
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_sm()
                                    .child(t.clusters.add_cluster.clone())
                            )
                    )
            );

        if self.is_mobile {
            // Mobile: overlay sidebar
            div()
                .id("mobile-sidebar")
                .absolute()
                .left(px(0.0))
                .top(px(48.0))
                .bottom(px(0.0))
                .w(self.width)
                .bg(theme.surface)
                .border_r(px(1.0))
                .border_color(theme.border)
                .child(content)
        } else {
            // Desktop: fixed sidebar
            div()
                .id("desktop-sidebar")
                .flex()
                .flex_col()
                .w(self.width)
                .h_full()
                .bg(theme.surface)
                .border_r(px(1.0))
                .border_color(theme.border)
                .child(content)
        }
    }
}