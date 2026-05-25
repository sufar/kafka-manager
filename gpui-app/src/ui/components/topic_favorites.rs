//! Topic Favorites Component
//!
//! Panel showing favorite topics list with management features.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;

/// Favorite topic item
#[derive(Debug, Clone)]
pub struct FavoriteTopicItem {
    /// Topic name
    pub name: String,
    /// Cluster name
    pub cluster: String,
    /// Group name (for grouping favorites)
    pub group: Option<String>,
    /// Added timestamp
    pub added_at: i64,
    /// Notes/description
    pub notes: Option<String>,
}

/// Favorite group
#[derive(Debug, Clone)]
pub struct FavoriteGroup {
    /// Group name
    pub name: String,
    /// Group description
    pub description: Option<String>,
    /// Item count
    pub count: i32,
}

/// Topic Favorites - Favorite topics panel
pub struct TopicFavorites {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Favorite groups
    groups: Vec<FavoriteGroup>,
    /// Favorite items
    items: Vec<FavoriteTopicItem>,
    /// Selected group filter
    selected_group: Option<String>,
    /// Selected item for details
    selected_item: Option<usize>,
    /// Search query
    search_query: String,
    /// Is collapsed
    is_collapsed: bool,
}

impl TopicFavorites {
    /// Create new topic favorites panel
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        let groups = vec![
            FavoriteGroup {
                name: "常用".to_string(),
                description: Some("日常使用的Topic".to_string()),
                count: 3,
            },
            FavoriteGroup {
                name: "监控".to_string(),
                description: Some("需要监控的Topic".to_string()),
                count: 2,
            },
        ];

        let items = vec![
            FavoriteTopicItem {
                name: "orders".to_string(),
                cluster: "Production".to_string(),
                group: Some("常用".to_string()),
                added_at: 1716432600000,
                notes: Some("订单消息Topic".to_string()),
            },
            FavoriteTopicItem {
                name: "payments".to_string(),
                cluster: "Production".to_string(),
                group: Some("常用".to_string()),
                added_at: 1716432601000,
                notes: None,
            },
            FavoriteTopicItem {
                name: "notifications".to_string(),
                cluster: "Production".to_string(),
                group: Some("常用".to_string()),
                added_at: 1716432602000,
                notes: Some("推送通知".to_string()),
            },
            FavoriteTopicItem {
                name: "user-events".to_string(),
                cluster: "Production".to_string(),
                group: Some("监控".to_string()),
                added_at: 1716432603000,
                notes: Some("用户行为事件".to_string()),
            },
            FavoriteTopicItem {
                name: "error-logs".to_string(),
                cluster: "Production".to_string(),
                group: Some("监控".to_string()),
                added_at: 1716432604000,
                notes: Some("错误日志收集".to_string()),
            },
        ];

        Self {
            theme: theme.clone(),
            translations,
            groups,
            items,
            selected_group: None,
            selected_item: None,
            search_query: String::new(),
            is_collapsed: false,
        }
    }

    /// Set favorite items
    pub fn set_items(&mut self, items: Vec<FavoriteTopicItem>) {
        self.items = items;
    }

    /// Set favorite groups
    pub fn set_groups(&mut self, groups: Vec<FavoriteGroup>) {
        self.groups = groups;
    }

    /// Set selected group filter
    pub fn set_selected_group(&mut self, group: Option<String>) {
        self.selected_group = group;
    }

    /// Set search query
    pub fn set_search(&mut self, query: String) {
        self.search_query = query;
    }

    /// Toggle collapsed
    pub fn toggle_collapsed(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    /// Add favorite
    pub fn add_favorite(&mut self, item: FavoriteTopicItem) {
        self.items.push(item);
    }

    /// Remove favorite by name and cluster
    pub fn remove_favorite(&mut self, name: &str, cluster: &str) {
        self.items.retain(|i| i.name != name || i.cluster != cluster);
    }

    /// Get filtered items
    fn filtered_items(&self) -> Vec<&FavoriteTopicItem> {
        self.items.iter()
            .filter(|item| {
                // Group filter
                if let Some(group) = &self.selected_group {
                    if item.group.as_ref() != Some(group) {
                        return false;
                    }
                }
                // Search filter
                if !self.search_query.is_empty() {
                    let query_lower = self.search_query.to_lowercase();
                    if !item.name.to_lowercase().contains(&query_lower) &&
                        !item.cluster.to_lowercase().contains(&query_lower) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Render collapsed view
    fn render_collapsed(&self) -> Div {
        let theme = &self.theme;
        let count = self.items.len();

        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .w(px(50.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .bg(theme.surface)
            .child(
                div()
                    .w(px(12.0))
                    .h(px(12.0))
                    .rounded(px(2.0))
                    .bg(theme.warning.opacity(0.3))
            )
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child(format!("{}", count))
            )
    }

    /// Render group selector
    fn render_group_selector(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(
                // "All" option
                div()
                    .px(px(6.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(if self.selected_group.is_none() { theme.primary.opacity(0.2) } else { theme.surface })
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(if self.selected_group.is_none() { theme.primary } else { theme.text_muted })
                            .text_xs()
                            .child("全部")
                    )
            )
            .children(self.groups.iter().map(|group| {
                let is_selected = self.selected_group == Some(group.name.clone());
                div()
                    .px(px(6.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(if is_selected { theme.primary.opacity(0.2) } else { theme.surface })
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(if is_selected { theme.primary } else { theme.text_muted })
                            .text_xs()
                            .child(format!("{} ({})", group.name, group.count))
                    )
            }))
    }

    /// Render favorite item
    fn render_favorite_item(&self, item: &FavoriteTopicItem, index: usize, is_selected: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .bg(if is_selected { theme.primary.opacity(0.1) } else { theme.surface })
            .border_l(px(2.0))
            .border_color(if is_selected { theme.primary } else { gpui::transparent_black() })
            .cursor_pointer()
            .child(
                // Main row
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        // Favorite icon
                        div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .rounded(px(2.0))
                            .bg(theme.warning.opacity(0.3))
                    )
                    .child(
                        // Topic name
                        div()
                            .flex_1()
                            .text_color(theme.text)
                            .text_sm()
                            .child(item.name.clone())
                    )
                    .child(
                        // Cluster badge
                        div()
                            .px(px(4.0))
                            .py(px(2.0))
                            .rounded(px(3.0))
                            .bg(theme.surface_raised)
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(item.cluster.clone())
                            )
                    )
            )
            .when_some(item.notes.clone(), |this, notes| {
                this.child(
                    // Notes
                    div()
                        .ml(px(20.0))
                        .text_color(theme.text_muted)
                        .text_xs()
                        .child(notes)
                )
            })
            .child(
                // Added time
                div()
                    .ml(px(20.0))
                    .text_color(theme.text_muted.opacity(0.5))
                    .text_xs()
                    .child(format_added_time(item.added_at))
            )
    }
}

/// Format added time to human readable
fn format_added_time(ts_ms: i64) -> String {
    use chrono::{Utc, TimeZone};
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .map(|dt| dt.format("%m-%d").to_string())
        .unwrap_or_else(|| "".to_string())
}

impl IntoElement for TopicFavorites {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        if self.is_collapsed {
            return self.render_collapsed();
        }

        let theme = &self.theme;
        let filtered = self.filtered_items();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w_full()
            .h_full()
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .p(px(8.0))
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .w(px(14.0))
                                    .h(px(14.0))
                                    .rounded(px(2.0))
                                    .bg(theme.warning.opacity(0.3))
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("收藏")
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(format!("({})", filtered.len()))
                            )
                    )
                    .child(
                        // Collapse button
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("−")
                            )
                    )
            )
            .child(self.render_group_selector())
            .child(
                // Search input placeholder
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(theme.surface_raised)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .w(px(10.0))
                            .h(px(10.0))
                            .rounded(px(2.0))
                            .bg(theme.text_muted.opacity(0.5))
                    )
                    .child(
                        div()
                            .text_color(if self.search_query.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text
                            })
                            .text_xs()
                            .child(if self.search_query.is_empty() {
                                "搜索...".to_string()
                            } else {
                                self.search_query.clone()
                            })
                    )
            )
            .child(
                // Favorites list
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .flex_1()
                    
                    .when(filtered.is_empty(), |this| {
                        this.child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("无收藏")
                        )
                    })
                    .children(filtered.iter().enumerate().map(|(idx, item)| {
                        let global_idx = self.items.iter().position(|i| i.name == item.name).unwrap_or(0);
                        let is_selected = self.selected_item == Some(global_idx);
                        self.render_favorite_item(item, idx, is_selected)
                    }))
            )
            .child(
                // Footer with add button
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .pt(px(4.0))
                    .border_t(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.primary.opacity(0.1))
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.primary)
                                    .text_xs()
                                    .child("+ 新建分组")
                            )
                    )
            )
    }
}

impl Clone for TopicFavorites {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            groups: self.groups.clone(),
            items: self.items.clone(),
            selected_group: self.selected_group.clone(),
            selected_item: self.selected_item,
            search_query: self.search_query.clone(),
            is_collapsed: self.is_collapsed,
        }
    }
}