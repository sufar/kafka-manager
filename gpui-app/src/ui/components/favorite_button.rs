//! Favorite Button Component
//!
//! Toggle button for adding/removing topics from favorites.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;

/// Favorite button state
#[derive(Clone)]
pub struct FavoriteButton {
    theme: Theme,
    is_favorite: bool,
    cluster_id: String,
    topic_name: String,
}

impl FavoriteButton {
    /// Create new favorite button
    pub fn new(
        theme: Theme,
        cluster_id: String,
        topic_name: String,
        is_favorite: bool,
    ) -> Self {
        Self {
            theme,
            is_favorite,
            cluster_id,
            topic_name,
        }
    }

    /// Check if favorite
    pub fn is_favorite(&self) -> bool {
        self.is_favorite
    }

    /// Toggle favorite state
    pub fn toggle(&mut self) {
        self.is_favorite = !self.is_favorite;
    }

    /// Get cluster ID
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Get topic name
    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }
}

impl IntoElement for FavoriteButton {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let color = if self.is_favorite {
            Hsla::from(gpui::rgb(0xfbbf24)) // Amber color for favorites
        } else {
            theme.text_muted
        };

        div()
            .flex()
            .items_center()
            .justify_center()
            .w(px(24.0))
            .h(px(24.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .opacity(if self.is_favorite { 1.0 } else { 0.5 })
            .hover(|d| {
                d.opacity(1.0)
                    .bg(theme.primary.opacity(0.1))
            })
            .child(
                // Star icon (simplified)
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .bg(color)
            )
    }
}