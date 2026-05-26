//! Main Application Component
//!
//! This is the root view component that orchestrates the entire UI layout.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;

use crate::ui::theme::Theme;
use crate::ui::layout::{TopNavBar, LeftSidebar, MainContent};
use crate::ui::components::ToastManager;
use crate::i18n::Translations;
use crate::state::GlobalState;
use crate::tour::TourOverlay;

/// The main application view
pub struct KafkaManagerApp {
    /// Global state entity
    state: GlobalState,
}

impl KafkaManagerApp {
    pub fn new() -> Self {
        Self {
            state: GlobalState::new(),
        }
    }

    /// Get current theme
    fn theme(&self) -> Theme {
        self.state.theme.clone()
    }

    /// Get current translations
    fn translations(&self) -> Arc<Translations> {
        self.state.translations()
    }

    /// Get current view
    fn current_view(&self) -> crate::router::ViewType {
        self.state.current_view()
    }
}

impl Render for KafkaManagerApp {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.bounds();
        let width = bounds.size.width;
        self.state.update_mobile_state(width);

        let theme = self.theme();
        let translations = self.translations();
        let current_view = self.current_view();
        let sidebar_width = self.state.sidebar_width;
        let is_mobile = self.state.is_mobile;
        let sidebar_open = self.state.sidebar_open;

        // Root container with theme
        div()
            .id("app-root")
            .flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family(".SystemUIFont")
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
                                this.child(
                                    LeftSidebar::desktop(
                                        sidebar_width,
                                        theme.clone(),
                                        translations.clone(),
                                        current_view,
                                    )
                                )
                            })
                            .when(is_mobile && sidebar_open, |this| {
                                this.child(
                                    LeftSidebar::mobile(
                                        sidebar_width,
                                        sidebar_open,
                                        theme.clone(),
                                        translations.clone(),
                                        current_view,
                                    )
                                )
                            })
                            .child(
                                MainContent::new(current_view, theme.clone(), translations.clone())
                            )
                    )
            )
            // Toast notifications overlay
            .child(
                ToastManager::new(theme.clone())
                    .position_bottom_right()
            )
            // Tour overlay for onboarding
            .when(self.state.tour_active, |this| {
                let steps = crate::tour::TourDefinition::default_steps();
                let tour = TourOverlay::active(theme.clone(), translations.clone(), steps);
                this.child(tour)
            })
    }
}