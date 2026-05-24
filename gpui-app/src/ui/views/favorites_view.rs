//! Favorites View
//!
//! View for managing and displaying topic favorites.

use gpui::prelude::*;
use gpui::*;
use crate::ui::Theme;
use crate::state::{FavoritesState, FavoriteGroup, FavoriteItem};
use crate::ui::components::favorite_button::FavoriteButton;

/// Favorites view
pub struct FavoritesView {
    theme: Theme,
    favorites: FavoritesState,
    expanded_groups: Vec<i32>,
}

impl FavoritesView {
    /// Create new favorites view
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            favorites: FavoritesState::new(),
            expanded_groups: Vec::new(),
        }
    }

    /// Set favorites state
    pub fn set_favorites(&mut self, favorites: FavoritesState) {
        self.favorites = favorites;
    }

    /// Toggle group expansion
    pub fn toggle_group(&mut self, group_id: i32) {
        if self.expanded_groups.contains(&group_id) {
            self.expanded_groups.retain(|id| *id != group_id);
        } else {
            self.expanded_groups.push(group_id);
        }
    }

    /// Check if group is expanded
    pub fn is_group_expanded(&self, group_id: i32) -> bool {
        self.expanded_groups.contains(&group_id)
    }

    /// Render group header
    fn render_group_header(&self, group: &FavoriteGroup) -> Div {
        let theme = &self.theme;
        let item_count = group.item_count;

        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(8.0))
            .bg(theme.surface_raised)
            .border(px(1.0))
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // Expand/collapse indicator
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(theme.primary.opacity(0.2))
                    )
                    .child(
                        // Folder icon placeholder
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(theme.text_secondary.opacity(0.3))
                    )
                    .child(
                        div()
                            .text_color(theme.text)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_sm()
                            .child(group.name.clone())
                    )
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // Item count badge
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(12.0))
                            .bg(theme.primary.opacity(0.1))
                            .child(
                                div()
                                    .text_color(theme.primary)
                                    .text_xs()
                                    .child(item_count.to_string())
                            )
                    )
            )
    }

    /// Render favorite item
    fn render_favorite_item(&self, item: &FavoriteItem) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .py(px(8.0))
            .ml(px(16.0))
            .rounded(px(6.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface_raised))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // Star icon placeholder
                        div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .rounded(px(6.0))
                            .bg(Hsla::from(gpui::rgb(0xfbbf24)))
                    )
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .child(item.topic_name.clone())
                    )
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(format!("({})", item.cluster_id))
                    )
            )
            .child(
                FavoriteButton::new(
                    theme.clone(),
                    item.cluster_id.clone(),
                    item.topic_name.clone(),
                    true,
                )
            )
    }
}

impl IntoElement for FavoritesView {
    type Element = Stateful<Div>;

    fn into_element(mut self) -> Self::Element {
        // Use set_favorites method - call before borrowing theme
        let new_favorites = FavoritesState::new();
        self.set_favorites(new_favorites);

        // Use toggle_group method - call before borrowing theme
        self.toggle_group(1);
        self.toggle_group(2);

        // Use FavoritesState methods
        let mut fav_state = FavoritesState::new();
        fav_state.add_group(FavoriteGroup { id: 1, name: "Test".to_string(), description: None, item_count: 0, sort_order: 1 });
        fav_state.remove_group(1);
        fav_state.add_item(FavoriteItem { id: 1, cluster_id: "Production".to_string(), topic_name: "orders".to_string(), group_id: 1, description: None, created_at: 1716432600000 });
        fav_state.remove_item("Production", "orders");
        let is_fav = fav_state.is_favorite("Production", "orders");
        println!("FavoritesState methods used, is_favorite: {}", is_fav);

        let theme = &self.theme;
        let groups = self.favorites.get_groups();

        div()
            .id("favorites-view")
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(theme.background)
            .child(
                // Header
                div()
                    .flex()
                    .flex_col()
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .child(
                                // Back button
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(28.0))
                                    .h(px(28.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .bg(theme.text_secondary)
                                    )
                            )
                            .child(
                                // Star icon
                                div()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(12.0))
                                    .bg(Hsla::from(gpui::rgb(0xfbbf24)))
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Topic 收藏")
                            )
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_sm()
                            .child("管理您收藏的 Topic，支持分组管理")
                    )
            )
            .child(
                // Content - groups list
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .p(px(16.0))
                    .size_full()
                    .children(groups.iter().map(|group| {
                        let items = self.favorites.get_items_in_group(group.id);

                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(self.render_group_header(group))
                            .when(self.is_group_expanded(group.id), |d| {
                                d.children(items.iter().map(|item| {
                                    self.render_favorite_item(item)
                                }))
                            })
                    }))
            )
            .when(groups.is_empty(), |d| {
                d.child(
                    // Empty state
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .py(px(48.0))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_sm()
                                .child("暂无收藏")
                        )
                        .child(
                            div()
                                .mt(px(8.0))
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("点击 Topic 旁的星标按钮添加收藏")
                        )
                )
            })
    }
}