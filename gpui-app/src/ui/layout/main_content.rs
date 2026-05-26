//! Main Content Component
//!
//! Displays the current view content based on router state.
//! Supports both static views and Entity-backed views with GlobalState.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::views::{ClustersView, ClustersViewWithState, TopicsView, MessagesView, MessagesViewWithState, ConsumerGroupsView, ConsumerGroupsViewWithState, TopicsViewWithState, SettingsView, SchemaRegistryView, FavoritesView};
use crate::router::ViewType;
use crate::state::GlobalState;

/// Main content area component (static version)
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

/// Main content with GlobalState Entity integration
pub struct MainContentWithState {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    messages_view: Entity<MessagesViewWithState>,
    topics_view: Entity<TopicsViewWithState>,
    consumer_groups_view: Entity<ConsumerGroupsViewWithState>,
    clusters_view: Entity<ClustersViewWithState>,
}

impl MainContentWithState {
    pub fn new(state: Entity<GlobalState>, translations: Arc<Translations>, cx: &mut Context<Self>) -> Self {
        let messages_view = cx.new(|cx| MessagesViewWithState::new(state.clone(), translations.clone()));
        let topics_view = cx.new(|cx| TopicsViewWithState::new(state.clone(), translations.clone()));
        let consumer_groups_view = cx.new(|cx| ConsumerGroupsViewWithState::new(state.clone(), translations.clone()));
        let clusters_view = cx.new(|cx| ClustersViewWithState::new(state.clone(), translations.clone(), cx));

        Self {
            state,
            translations,
            messages_view,
            topics_view,
            consumer_groups_view,
            clusters_view,
        }
    }
}

impl Render for MainContentWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let theme = &state.theme;
        let current_view = state.router.current_view();
        let t = &self.translations;

        let view_title = match current_view {
            ViewType::Clusters => t.clusters.title.clone(),
            ViewType::Topics => t.topics.title.clone(),
            ViewType::Messages => t.messages.title.clone(),
            ViewType::ConsumerGroups => t.consumer_groups.title.clone(),
            ViewType::SchemaRegistry => t.schema_registry.title.clone(),
            ViewType::Settings => t.settings.title.clone(),
            ViewType::Favorites => "Favorites".to_string(),
        };

        let view_description = match current_view {
            ViewType::Clusters => t.clusters.description.clone(),
            ViewType::Topics => t.topics.description.clone(),
            ViewType::Messages => t.messages.description.clone(),
            ViewType::ConsumerGroups => t.consumer_groups.title.clone(),
            ViewType::SchemaRegistry => t.schema_registry.title.clone(),
            ViewType::Settings => t.settings.title.clone(),
            ViewType::Favorites => "Manage your favorite topics".to_string(),
        };

        // Render view content based on current view
        div()
            .id("main-content-with-state")
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .bg(theme.background)
            .p(px(24.0))
            .child(
                // Page header
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(
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
                                    .hover(|d| d.bg(theme.surface))
                                    .child(
                                        div()
                                            .w(px(14.0))
                                            .h(px(14.0))
                                            .bg(theme.text_muted)
                                    )
                            )
                    )
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_sm()
                            .child(view_description)
                    )
            )
            // Render appropriate view
            .when(current_view == ViewType::Messages, |this| {
                this.child(self.messages_view.clone())
            })
            .when(current_view == ViewType::Clusters, |this| {
                this.child(self.clusters_view.clone())
            })
            .when(current_view == ViewType::Topics, |this| {
                this.child(self.topics_view.clone())
            })
            .when(current_view == ViewType::ConsumerGroups, |this| {
                this.child(self.consumer_groups_view.clone())
            })
            .when(current_view == ViewType::SchemaRegistry, |this| {
                this.child(SchemaRegistryView::new(theme.clone(), t.clone()))
            })
            .when(current_view == ViewType::Settings, |this| {
                this.child(SettingsView::new(theme.clone(), t.clone()))
            })
            .when(current_view == ViewType::Favorites, |this| {
                this.child(FavoritesView::new(theme.clone()))
            })
    }
}