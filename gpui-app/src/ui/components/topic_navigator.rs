//! Topic Navigator Component
//!
//! Navigation component for selecting and switching between topics.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{FavoriteButton, NavigatorSearch};

/// Topic navigation item
#[derive(Debug, Clone)]
pub struct TopicNavItem {
    /// Topic name
    pub name: String,
    /// Cluster name
    pub cluster: String,
    /// Partition count
    pub partitions: i32,
    /// Is favorite
    pub is_favorite: bool,
    /// Has unread messages (notification indicator)
    pub has_notification: bool,
}

/// Topic Navigator - Topic selection and switching
pub struct TopicNavigator {
    /// Theme
    theme: Theme,
    /// Translations
    translations: Arc<Translations>,
    /// Available topics
    topics: Vec<TopicNavItem>,
    /// Selected topic index
    selected_index: Option<usize>,
    /// Search component
    search: NavigatorSearch,
    /// Show favorites only filter
    show_favorites_only: bool,
    /// Sort mode
    sort_mode: TopicSortMode,
    /// Expanded cluster groups
    expanded_clusters: Vec<String>,
}

/// Sort mode for topics
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TopicSortMode {
    #[default]
    Alphabetical,
    ByPartitions,
    ByActivity,
}

impl TopicSortMode {
    /// Get display label
    pub fn label(&self) -> String {
        match self {
            TopicSortMode::Alphabetical => "字母排序".to_string(),
            TopicSortMode::ByPartitions => "分区数".to_string(),
            TopicSortMode::ByActivity => "活跃度".to_string(),
        }
    }
}

impl TopicNavigator {
    /// Create new topic navigator
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        let topics = vec![
            TopicNavItem {
                name: "orders".to_string(),
                cluster: "Production".to_string(),
                partitions: 12,
                is_favorite: true,
                has_notification: false,
            },
            TopicNavItem {
                name: "payments".to_string(),
                cluster: "Production".to_string(),
                partitions: 6,
                is_favorite: true,
                has_notification: true,
            },
            TopicNavItem {
                name: "notifications".to_string(),
                cluster: "Production".to_string(),
                partitions: 3,
                is_favorite: false,
                has_notification: false,
            },
            TopicNavItem {
                name: "user-events".to_string(),
                cluster: "Production".to_string(),
                partitions: 24,
                is_favorite: false,
                has_notification: false,
            },
            TopicNavItem {
                name: "test-topic".to_string(),
                cluster: "Development".to_string(),
                partitions: 1,
                is_favorite: false,
                has_notification: false,
            },
            TopicNavItem {
                name: "debug-logs".to_string(),
                cluster: "Development".to_string(),
                partitions: 1,
                is_favorite: false,
                has_notification: true,
            },
        ];

        Self {
            theme: theme.clone(),
            translations,
            topics,
            selected_index: Some(0),
            search: NavigatorSearch::new(theme.clone(), Arc::new(Translations::default())),
            show_favorites_only: false,
            sort_mode: TopicSortMode::default(),
            expanded_clusters: vec!["Production".to_string()],
        }
    }

    /// Create with specific topics
    pub fn with_topics(theme: Theme, translations: Arc<Translations>, topics: Vec<TopicNavItem>) -> Self {
        Self {
            theme: theme.clone(),
            translations,
            topics,
            selected_index: None,
            search: NavigatorSearch::new(theme.clone(), Arc::new(Translations::default())),
            show_favorites_only: false,
            sort_mode: TopicSortMode::default(),
            expanded_clusters: Vec::new(),
        }
    }

    /// Set topics list
    pub fn set_topics(&mut self, topics: Vec<TopicNavItem>) {
        self.topics = topics;
    }

    /// Set selected topic
    pub fn set_selected(&mut self, index: usize) {
        self.selected_index = Some(index);
    }

    /// Set search query
    pub fn set_search(&mut self, query: String) {
        self.search.set_query(query);
    }

    /// Toggle favorites filter
    pub fn toggle_favorites_filter(&mut self) {
        self.show_favorites_only = !self.show_favorites_only;
    }

    /// Set sort mode
    pub fn set_sort_mode(&mut self, mode: TopicSortMode) {
        self.sort_mode = mode;
    }

    /// Toggle cluster expansion
    pub fn toggle_cluster(&mut self, cluster: String) {
        if self.expanded_clusters.contains(&cluster) {
            self.expanded_clusters.retain(|c| c != &cluster);
        } else {
            self.expanded_clusters.push(cluster);
        }
    }

    /// Get selected topic
    pub fn selected_topic(&self) -> Option<&TopicNavItem> {
        self.selected_index.map(|idx| &self.topics[idx])
    }

    /// Get filtered and sorted topics grouped by cluster
    fn grouped_topics(&self) -> Vec<(String, Vec<&TopicNavItem>)> {
        // Filter
        let filtered: Vec<&TopicNavItem> = self.topics.iter()
            .filter(|t| {
                // Favorites filter
                if self.show_favorites_only && !t.is_favorite {
                    return false;
                }
                // Search filter
                if !self.search.query().is_empty() {
                    let query_lower = self.search.query().to_lowercase();
                    if !t.name.to_lowercase().contains(&query_lower) {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Sort
        let mut sorted = filtered;
        match self.sort_mode {
            TopicSortMode::Alphabetical => {
                sorted.sort_by(|a, b| a.name.cmp(&b.name));
            }
            TopicSortMode::ByPartitions => {
                sorted.sort_by(|a, b| b.partitions.cmp(&a.partitions));
            }
            TopicSortMode::ByActivity => {
                // Sort by notification then name
                sorted.sort_by(|a, b| {
                    b.has_notification.cmp(&a.has_notification)
                        .then_with(|| a.name.cmp(&b.name))
                });
            }
        }

        // Group by cluster
        let clusters: Vec<String> = sorted.iter()
            .map(|t| t.cluster.clone())
            .collect();

        let unique_clusters: Vec<String> = clusters.iter()
            .fold(Vec::new(), |mut acc, c| {
                if !acc.contains(c) {
                    acc.push(c.clone());
                }
                acc
            });

        unique_clusters.iter()
            .map(|cluster| {
                let topics: Vec<&TopicNavItem> = sorted.iter()
                    .filter(|t| &t.cluster == cluster)
                    .copied()
                    .collect();
                (cluster.clone(), topics)
            })
            .collect()
    }

    /// Render cluster group header
    fn render_cluster_header(&self, cluster: &str, topic_count: usize, is_expanded: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .bg(theme.surface_raised)
            .cursor_pointer()
            .child(
                // Expand/collapse icon
                div()
                    .w(px(12.0))
                    .h(px(12.0))
                    .rounded(px(2.0))
                    .bg(theme.text_muted.opacity(0.5))
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_xs()
                            .child(if is_expanded { "−" } else { "+" })
                    )
            )
            .child(
                div()
                    .text_color(theme.text)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(cluster.to_string())
            )
            .child(
                div()
                    .px(px(4.0))
                    .py(px(2.0))
                    .rounded(px(3.0))
                    .bg(theme.surface)
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(format!("{}", topic_count))
                    )
            )
    }

    /// Render topic item
    fn render_topic_item(&self, topic: &TopicNavItem, index: usize, is_selected: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(6.0))
            .ml(px(12.0))
            .rounded(px(4.0))
            .bg(if is_selected { theme.primary.opacity(0.15) } else { gpui::transparent_black() })
            .border_l(px(2.0))
            .border_color(if is_selected { theme.primary } else { gpui::transparent_black() })
            .cursor_pointer()
            .child(
                // Favorite indicator
                div()
                    .when(topic.is_favorite, |this| {
                        this.w(px(10.0))
                            .h(px(10.0))
                            .rounded(px(2.0))
                            .bg(theme.warning.opacity(0.3))
                    })
            )
            .child(
                // Notification indicator
                div()
                    .when(topic.has_notification, |this| {
                        this.w(px(8.0))
                            .h(px(8.0))
                            .rounded(px(4.0))
                            .bg(theme.error)
                    })
            )
            .child(
                // Topic icon
                div()
                    .w(px(10.0))
                    .h(px(10.0))
                    .rounded(px(2.0))
                    .bg(if is_selected { theme.primary } else { theme.text_muted })
            )
            .child(
                // Topic name
                div()
                    .flex_1()
                    .text_color(if is_selected { theme.text } else { theme.text_secondary })
                    .text_xs()
                    .child(topic.name.clone())
            )
            .child(
                // Partition count
                div()
                    .px(px(4.0))
                    .py(px(2.0))
                    .rounded(px(3.0))
                    .bg(theme.surface)
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(format!("{}P", topic.partitions))
                    )
            )
    }

    /// Render toolbar
    fn render_toolbar(&self) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .pb(px(8.0))
            .child(
                // Favorites filter toggle
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(24.0))
                    .h(px(24.0))
                    .rounded(px(4.0))
                    .bg(if self.show_favorites_only { theme.warning.opacity(0.2) } else { theme.surface })
                    .border(px(1.0))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(if self.show_favorites_only { theme.warning } else { theme.text_muted })
                            .text_xs()
                            .child("★")
                    )
            )
            .child(
                // Sort dropdown
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .px(px(6.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .cursor_pointer()
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child(self.sort_mode.label())
                    )
            )
    }
}

impl IntoElement for TopicNavigator {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let grouped = self.grouped_topics();

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
            .child(self.render_toolbar())
            .child(self.search.clone())
            .child(
                // Topics list grouped by cluster
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .flex_1()
                    
                    .children(grouped.iter().map(|(cluster, topics)| {
                        let is_expanded = self.expanded_clusters.contains(cluster);
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(self.render_cluster_header(cluster, topics.len(), is_expanded))
                            .when(is_expanded, |this| {
                                this.children(topics.iter().enumerate().map(|(idx, topic)| {
                                    // Calculate global index for selection check
                                    let global_idx = self.topics.iter().position(|t| t.name == topic.name).unwrap_or(0);
                                    let is_selected = self.selected_index == Some(global_idx);
                                    self.render_topic_item(topic, idx, is_selected)
                                }))
                            })
                    }))
            )
            // Selection status
            .when_some(self.selected_topic(), |this, topic| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .p(px(6.0))
                        .rounded(px(4.0))
                        .bg(theme.surface_raised)
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child("已选择:")
                        )
                        .child(
                            div()
                                .text_color(theme.text)
                                .text_xs()
                                .child(topic.name.clone())
                        )
                )
            })
    }
}

impl Clone for TopicNavigator {
    fn clone(&self) -> Self {
        Self {
            theme: self.theme.clone(),
            translations: self.translations.clone(),
            topics: self.topics.clone(),
            selected_index: self.selected_index,
            search: self.search.clone(),
            show_favorites_only: self.show_favorites_only,
            sort_mode: self.sort_mode,
            expanded_clusters: self.expanded_clusters.clone(),
        }
    }
}