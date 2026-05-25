//! Tour Module
//!
//! New user onboarding/onboarding guide system with progress tracking.

mod definitions;

pub use definitions::{
    TourStep, TourDefinition, TourPosition, TourPlacement,
    TourStepPriority, TourProgress, TourManager
};

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Tour overlay state for UI rendering
pub struct TourOverlay {
    theme: Theme,
    translations: Arc<Translations>,
    current_step: usize,
    steps: Vec<TourStep>,
    is_active: bool,
    progress: Option<TourProgress>,
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
            progress: None,
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
            progress: None,
        }
    }

    /// Create tour overlay with progress tracking
    pub fn with_progress(theme: Theme, translations: Arc<Translations>, steps: Vec<TourStep>, progress: TourProgress) -> Self {
        Self {
            theme,
            translations,
            current_step: progress.current_step,
            steps,
            is_active: true,
            progress: Some(progress),
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
        if let Some(progress) = &mut self.progress {
            progress.finish();
        }
    }

    /// Go to next step
    pub fn next_step(&mut self) {
        if self.current_step < self.steps.len() - 1 {
            // Mark current step as completed
            if let Some(progress) = &mut self.progress {
                if let Some(step) = self.steps.get(self.current_step) {
                    progress.complete_step(&step.id);
                }
            }
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

    /// Skip current step
    pub fn skip_step(&mut self) -> bool {
        // Get step info first to avoid borrow conflict
        let step_info = self.current_step().map(|s| (s.id.clone(), s.skip_allowed));
        if let Some((step_id, can_skip)) = step_info {
            if can_skip {
                if let Some(progress) = &mut self.progress {
                    progress.skip_step(&step_id);
                }
                self.next_step();
                return true;
            }
        }
        false
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

    /// Get current step index
    pub fn current_step_index(&self) -> usize {
        self.current_step
    }

    /// Get total steps
    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    /// Get progress percentage
    pub fn progress_percentage(&self) -> f32 {
        if self.steps.is_empty() {
            return 100.0;
        }
        (self.current_step as f32 / self.steps.len() as f32) * 100.0
    }

    /// Get progress tracker
    pub fn progress(&self) -> Option<&TourProgress> {
        self.progress.as_ref()
    }

    /// Render spotlight position based on step
    fn calculate_spotlight_position(&self, step: &TourStep) -> (f32, f32, f32, f32) {
        // Use static position for now, but could be enhanced to find actual element
        let pos = &step.position;
        (pos.x as f32, pos.y as f32, pos.width as f32, pos.height as f32)
    }

    /// Calculate tooltip position based on spotlight and placement
    fn calculate_tooltip_position(&self, step: &TourStep) -> (f32, f32) {
        let (x, y, width, height) = self.calculate_spotlight_position(step);
        let (offset_x, offset_y) = step.position.placement.tooltip_offset();

        match step.position.placement {
            TourPlacement::Top => (x, y + offset_y as f32),
            TourPlacement::Bottom => (x, y + height + offset_y as f32),
            TourPlacement::Left => (x + offset_x as f32, y),
            TourPlacement::Right => (x + width + offset_x as f32, y),
            TourPlacement::Center => (x + width / 2.0 - 160.0, y + height + offset_y as f32),
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

        let (spot_x, spot_y, spot_w, spot_h) = self.calculate_spotlight_position(current);
        let (tooltip_x, tooltip_y) = self.calculate_tooltip_position(current);
        let progress_pct = self.progress_percentage();

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

        // Skip button (conditional on skip_allowed)
        let skip_button = if current.skip_allowed {
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
        } else {
            div()
        };

        // Priority indicator
        let priority_color = match current.priority {
            TourStepPriority::Required => theme.error,
            TourStepPriority::Optional => theme.warning,
            TourStepPriority::Info => theme.text_muted,
        };

        // Overlay with spotlight and tooltip
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
            .bg(theme.background.opacity(0.85))
            .child(
                // Spotlight box (highlights target element)
                div()
                    .absolute()
                    .top(px(spot_y))
                    .left(px(spot_x))
                    .w(px(spot_w))
                    .h(px(spot_h))
                    .rounded(px(8.0))
                    .border(px(3.0))
                    .border_color(theme.primary)
                    .bg(gpui::transparent_black())
            )
            .child(
                // Tooltip content
                div()
                    .absolute()
                    .top(px(tooltip_y))
                    .left(px(tooltip_x))
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
                                // Header with title and priority
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_color(theme.text)
                                            .text_lg()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(current.title.clone())
                                    )
                                    .when(current.priority != TourStepPriority::Info, |this| {
                                        this.child(
                                            div()
                                                .px(px(6.0))
                                                .py(px(2.0))
                                                .rounded(px(4.0))
                                                .bg(priority_color.opacity(0.2))
                                                .child(
                                                    div()
                                                        .text_color(priority_color)
                                                        .text_xs()
                                                        .child(match current.priority {
                                                            TourStepPriority::Required => "必读",
                                                            TourStepPriority::Optional => "可选",
                                                            TourStepPriority::Info => "",
                                                        })
                                                )
                                        )
                                    })
                            )
                            .child(
                                // Step description
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_sm()
                                    .child(current.description.clone())
                            )
                            .when_some(current.expected_action.clone(), |this, action| {
                                // Expected action hint
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(theme.primary.opacity(0.1))
                                        .child(
                                            div()
                                                .text_color(theme.primary)
                                                .text_xs()
                                                .child("💡 ".to_string() + &action)
                                        )
                                )
                            })
                            .child(
                                // Progress indicator
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        // Progress dots
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .children(self.steps.iter().enumerate().map(|(i, step)| {
                                                let is_current = i == self.current_step;
                                                let is_completed = i < self.current_step;
                                                div()
                                                    .w(px(if is_current { 10.0 } else { 6.0 }))
                                                    .h(px(if is_current { 10.0 } else { 6.0 }))
                                                    .rounded(px(if is_current { 5.0 } else { 3.0 }))
                                                    .bg(if is_completed {
                                                        theme.success
                                                    } else if is_current {
                                                        theme.primary
                                                    } else {
                                                        theme.surface_raised
                                                    })
                                            }))
                                    )
                                    .child(
                                        // Progress percentage
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(format!("{:.0}%", progress_pct))
                                    )
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
                                        // Right side buttons
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(skip_button)
                                            .child(
                                                // Next/Finish button
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