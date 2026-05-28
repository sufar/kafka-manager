//! Main Application Component
//!
//! This is the root view component that orchestrates the entire UI layout.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;

use crate::ui::theme::Theme;
use crate::ui::layout::{TopNavBar, MainContentWithState, LeftSidebarWithState};
use crate::ui::components::ToastManagerWithState;
use crate::i18n::Translations;
use crate::state::GlobalState;
use crate::tour::TourOverlay;
use crate::router::ViewType;
use crate::shortcuts::navigation::{GoToClusters, GoToTopics, GoToMessages, GoToConsumerGroups, GoToSchemaRegistry, ToggleSidebar};

/// The main application view
pub struct KafkaManagerApp {
    /// Global state entity
    state: Entity<GlobalState>,
    /// Sidebar component with state
    sidebar: Entity<LeftSidebarWithState>,
    /// Toast manager with state
    toast_manager: Entity<ToastManagerWithState>,
    /// Main content with state
    main_content: Entity<MainContentWithState>,
}

impl KafkaManagerApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        tracing::info!("Initializing KafkaManagerApp...");

        // Create global state entity
        let state = cx.new(|cx| {
            tracing::info!("Creating GlobalState...");
            let mut global_state = GlobalState::new();
            // Load clusters on initialization (async, won't block)
            tracing::info!("Loading clusters and groups...");
            global_state.load_clusters(cx);
            global_state.load_cluster_groups(cx);
            tracing::info!("GlobalState initialized");
            global_state
        });

        tracing::info!("Creating sidebar...");
        // Create sidebar with state reference
        let sidebar = cx.new(|cx| LeftSidebarWithState::new(state.clone(), false));

        tracing::info!("Creating toast manager...");
        // Create toast manager with state reference
        let toast_manager = cx.new(|_| ToastManagerWithState::new(state.clone()));

        tracing::info!("Creating main content...");
        // Create main content with state reference
        let translations = state.read(cx).translations();
        let main_content = cx.new(|cx| MainContentWithState::new(state.clone(), translations, cx));

        tracing::info!("KafkaManagerApp fully initialized");

        Self {
            state,
            sidebar,
            toast_manager,
            main_content,
        }
    }

    /// Get current theme
    fn theme(&self, cx: &App) -> Theme {
        self.state.read(cx).theme.clone()
    }

    /// Get current translations
    fn translations(&self, cx: &App) -> Arc<Translations> {
        self.state.read(cx).translations()
    }

    /// Get current view
    fn current_view(&self, cx: &App) -> ViewType {
        self.state.read(cx).current_view()
    }
}

impl Render for KafkaManagerApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.bounds();
        let width = bounds.size.width;

        // Update mobile state
        self.state.update(cx, |state, cx| {
            state.update_mobile_state(width);
            cx.notify();
        });

        let theme = self.theme(cx);
        let translations = self.translations(cx);
        let current_view = self.current_view(cx);
        let is_mobile = self.state.read(cx).is_mobile;
        let sidebar_open = self.state.read(cx).sidebar_open;
        let tour_active = self.state.read(cx).tour_active;

        // Root container with theme and action handlers
        div()
            .id("app-root")
            .flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family(".SystemUIFont")
            .on_action(cx.listener(|this, _: &GoToClusters, _, cx| {
                this.state.update(cx, |state, cx| {
                    state.navigate(ViewType::Clusters);
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &GoToTopics, _, cx| {
                this.state.update(cx, |state, cx| {
                    state.navigate(ViewType::Topics);
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &GoToMessages, _, cx| {
                this.state.update(cx, |state, cx| {
                    state.navigate(ViewType::Messages);
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &GoToConsumerGroups, _, cx| {
                this.state.update(cx, |state, cx| {
                    state.navigate(ViewType::ConsumerGroups);
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &GoToSchemaRegistry, _, cx| {
                this.state.update(cx, |state, cx| {
                    state.navigate(ViewType::SchemaRegistry);
                    cx.notify();
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.state.update(cx, |state, cx| {
                    state.toggle_sidebar();
                    cx.notify();
                });
            }))
            .child(
                // Main layout container
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(
                        // Top Navigation Bar
                        TopNavBar::new(is_mobile, theme.is_dark, sidebar_open, translations.clone())
                    )
                    .child(
                        // Content area (sidebar + main content)
                        div()
                            .flex()
                            .flex_1()
                            .overflow_hidden()
                            .when(!is_mobile, |this| {
                                this.child(self.sidebar.clone())
                            })
                            .when(is_mobile && sidebar_open, |this| {
                                this.child(self.sidebar.clone())
                            })
                            .child(self.main_content.clone())
                    )
            )
            // Toast notifications overlay
            .child(self.toast_manager.clone())
            // Tour overlay for onboarding
            .when(tour_active, |this| {
                let steps = crate::tour::TourDefinition::default_steps();
                let tour = TourOverlay::active(theme.clone(), translations.clone(), steps);
                this.child(tour)
            })
    }
}