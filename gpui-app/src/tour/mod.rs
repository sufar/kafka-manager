//! Tour Module
//!
//! New user onboarding/onboarding guide system.

mod definitions;

pub use definitions::{TourStep, TourDefinition, TourPosition};

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Tour overlay state
pub struct TourOverlay {
    theme: Theme,
    translations: Arc<Translations>,
    current_step: usize,
    steps: Vec<TourStep>,
    is_active: bool,
}

impl TourOverlay {
    /// Create new tour overlay
    pub fn new(theme: Theme, translations: Arc<Translations>, steps: Vec<TourStep>) -> Self {
        Self {
            theme,
            translations,
            current_step: 0,
            steps,
            is_active: false,
        }
    }

    /// Create new tour overlay that is immediately active
    pub fn active(theme: Theme, translations: Arc<Translations>, steps: Vec<TourStep>) -> Self {
        Self {
            theme,
            translations,
            current_step: 0,
            steps,
            is_active: true,
        }
    }

    /// Start the tour
    pub fn start(&mut self) {
        self.is_active = true;
        self.current_step = 0;
    }

    /// End the tour
    pub fn end(&mut self) {
        self.is_active = false;
    }

    /// Go to next step
    pub fn next_step(&mut self) {
        if self.current_step < self.steps.len() - 1 {
            self.current_step += 1;
        } else {
            self.end();
        }
    }

    /// Go to previous step
    pub fn prev_step(&mut self) {
        if self.current_step > 0 {
            self.current_step -= 1;
        }
    }

    /// Check if tour is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get current step
    pub fn current_step(&self) -> Option<&TourStep> {
        if self.is_active && self.current_step < self.steps.len() {
            Some(&self.steps[self.current_step])
        } else {
            None
        }
    }
}

impl IntoElement for TourOverlay {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        if !self.is_active {
            return div().id("tour-overlay-inactive");
        }

        let theme = &self.theme;
        let current = &self.steps[self.current_step];
        let t = &self.translations;

        // Previous button (conditional)
        let prev_button = if self.current_step > 0 {
            div()
                .flex()
                .items_center()
                .justify_center()
                .px(px(12.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(theme.surface_raised)
                .border(px(1.0))
                .border_color(theme.border)
                .cursor_pointer()
                .child(
                    div()
                        .text_color(theme.text_secondary)
                        .text_sm()
                        .child(t.common.back.clone())
                )
        } else {
            div()
        };

        // Overlay with spotlight
        div()
            .id("tour-overlay")
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background.opacity(0.9))
            .child(
                // Spotlight box (simulated)
                div()
                    .absolute()
                    .top(px(current.position.y as f32))
                    .left(px(current.position.x as f32))
                    .w(px(current.position.width as f32))
                    .h(px(current.position.height as f32))
                    .rounded(px(8.0))
                    .border(px(3.0))
                    .border_color(theme.primary)
                    .bg(gpui::transparent_black())
            )
            .child(
                // Step content
                div()
                    .absolute()
                    .top(px(current.position.y as f32 + current.position.height as f32 + 16.0))
                    .left(px(current.position.x as f32))
                    .w(px(320.0))
                    .p(px(16.0))
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(
                                // Step title
                                div()
                                    .text_color(theme.text)
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(current.title.clone())
                            )
                            .child(
                                // Step description
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(current.description.clone())
                            )
                            .child(
                                // Progress indicator
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .children(self.steps.iter().enumerate().map(|(i, _)| {
                                        div()
                                            .w(px(8.0))
                                            .h(px(8.0))
                                            .rounded(px(4.0))
                                            .bg(if i == self.current_step { theme.primary } else { theme.surface_raised })
                                    }))
                            )
                            .child(
                                // Navigation buttons
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.0))
                                    .child(prev_button)
                                    .child(
                                        // Next/Skip buttons
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                // Skip button
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .px(px(12.0))
                                                    .py(px(6.0))
                                                    .rounded(px(6.0))
                                                    .bg(gpui::transparent_black())
                                                    .cursor_pointer()
                                                    .child(
                                                        div()
                                                            .text_color(theme.text_muted)
                                                            .text_sm()
                                                            .child("跳过")
                                                    )
                                            )
                                            .child(
                                                // Next button
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .px(px(16.0))
                                                    .py(px(6.0))
                                                    .rounded(px(6.0))
                                                    .bg(theme.primary)
                                                    .cursor_pointer()
                                                    .child(
                                                        div()
                                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                                            .text_sm()
                                                            .child(if self.current_step < self.steps.len() - 1 {
                                                                "下一步".to_string()
                                                            } else {
                                                                t.common.confirm.clone()
                                                            })
                                                    )
                                            )
                                    )
                            )
                    )
            )
    }
}