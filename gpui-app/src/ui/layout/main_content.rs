//! Main Content Component
//!
//! Displays the current view content based on router state.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::views::{ClustersView, TopicsView, MessagesView, ConsumerGroupsView, SettingsView, SchemaRegistryView, FavoritesView};
use crate::router::ViewType;

/// Main content area component
pub struct MainContent {
    current_view: ViewType,
    theme: Theme,
    translations: Arc<Translations>,
}

impl MainContent {
    /// Create new main content component
    pub fn new(current_view: ViewType, theme: Theme, translations: Arc<Translations>) -> Self {
        Self {
            current_view,
            theme,
            translations,
        }
    }
}

impl IntoElement for MainContent {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let theme = self.theme.clone();
        let t = self.translations.clone();

        let view_title = match self.current_view {
            ViewType::Clusters => t.clusters.title.clone(),
            ViewType::Topics => t.topics.title.clone(),
            ViewType::Messages => t.messages.title.clone(),
            ViewType::ConsumerGroups => t.consumer_groups.title.clone(),
            ViewType::SchemaRegistry => t.schema_registry.title.clone(),
            ViewType::Settings => t.settings.title.clone(),
            ViewType::Favorites => "Favorites".to_string(),
        };

        let view_description = match self.current_view {
            ViewType::Clusters => t.clusters.description.clone(),
            ViewType::Topics => t.topics.description.clone(),
            ViewType::Messages => t.messages.description.clone(),
            ViewType::ConsumerGroups => t.consumer_groups.title.clone(),
            ViewType::SchemaRegistry => t.schema_registry.title.clone(),
            ViewType::Settings => t.settings.title.clone(),
            ViewType::Favorites => "Manage your favorite topics".to_string(),
        };

        // Render the appropriate view content
        let view_content = match self.current_view {
            ViewType::Clusters => {
                ClustersView::new(theme.clone(), t.clone())
                    .into_element()
                    .into_any_element()
            }
            ViewType::Topics => {
                TopicsView::new(theme.clone(), t.clone())
                    .into_element()
                    .into_any_element()
            }
            ViewType::Messages => {
                MessagesView::new(theme.clone(), t.clone())
                    .into_element()
                    .into_any_element()
            }
            ViewType::ConsumerGroups => {
                ConsumerGroupsView::new(theme.clone(), t.clone())
                    .into_element()
                    .into_any_element()
            }
            ViewType::SchemaRegistry => {
                SchemaRegistryView::new(theme.clone(), t.clone())
                    .into_element()
                    .into_any_element()
            }
            ViewType::Settings => {
                SettingsView::new(theme.clone(), t.clone())
                    .into_element()
                    .into_any_element()
            }
            ViewType::Favorites => {
                FavoritesView::new(theme.clone())
                    .into_element()
                    .into_any_element()
            }
        };

        div()
            .id("main-content")
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .bg(theme.background)
            .p(px(24.0))
            .child(
                // Page header
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(
                        // Title row
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child(view_title)
                            )
                            .child(
                                // Refresh button
                                div()
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
                                            .w(px(14.0))
                                            .h(px(14.0))
                                            .bg(theme.text_muted)
                                    )
                            )
                    )
                    .child(
                        // Description
                        div()
                            .text_color(theme.text_muted)
                            .text_sm()
                            .child(view_description)
                    )
            )
            .child(view_content)
    }
}