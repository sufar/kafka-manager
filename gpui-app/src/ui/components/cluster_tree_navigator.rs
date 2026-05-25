//! Cluster Tree Navigator
//!
//! Tree view for clusters and topics navigation with search filtering.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::state::ConnectionStatusType;

/// Tree node state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeState {
    Expanded,
    Collapsed,
}

/// Cluster group for organizing clusters
#[derive(Debug, Clone)]
pub struct ClusterGroup {
    pub id: i64,
    pub name: String,
    pub color: Option<Hsla>,
}

impl Default for ClusterGroup {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Default".to_string(),
            color: None,
        }
    }
}

/// Tree navigator with search and grouping support
pub struct ClusterTreeNavigator {
    theme: Theme,
    translations: Arc<Translations>,
    /// Tree data with grouping
    groups: Vec<TreeGroup>,
    /// Selected node path (cluster_id -> topic_name)
    selected_node: Option<String>,
    /// Search filter string
    search_filter: String,
    /// Show all clusters expanded
    expand_all: bool,
}

/// Tree group containing clusters
struct TreeGroup {
    group: ClusterGroup,
    clusters: Vec<ClusterTreeNode>,
    state: NodeState,
}

/// Cluster tree node
struct ClusterTreeNode {
    id: String,
    name: String,
    status: ConnectionStatusType,
    group_id: Option<i64>,
    topics_state: NodeState,
    topics: Vec<TopicTreeNode>,
}

/// Topic tree node
struct TopicTreeNode {
    name: String,
    partition_count: i32,
    partitions_state: NodeState,
    partitions: Vec<PartitionTreeNode>,
}

/// Partition tree node
struct PartitionTreeNode {
    id: i32,
    leader: Option<i32>,
}

impl ClusterTreeNavigator {
    /// Create new tree navigator
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        // Mock data with grouping
        let groups = vec![
            TreeGroup {
                group: ClusterGroup {
                    id: 1,
                    name: "Production".to_string(),
                    color: Some(theme.success),
                },
                state: NodeState::Expanded,
                clusters: vec![
                    ClusterTreeNode {
                        id: "prod-1".to_string(),
                        name: "Main Cluster".to_string(),
                        status: ConnectionStatusType::Connected,
                        group_id: Some(1),
                        topics_state: NodeState::Expanded,
                        topics: vec![
                            TopicTreeNode {
                                name: "orders".to_string(),
                                partition_count: 3,
                                partitions_state: NodeState::Collapsed,
                                partitions: vec![
                                    PartitionTreeNode { id: 0, leader: Some(1) },
                                    PartitionTreeNode { id: 1, leader: Some(2) },
                                    PartitionTreeNode { id: 2, leader: Some(3) },
                                ],
                            },
                            TopicTreeNode {
                                name: "payments".to_string(),
                                partition_count: 1,
                                partitions_state: NodeState::Collapsed,
                                partitions: vec![PartitionTreeNode { id: 0, leader: Some(1) }],
                            },
                            TopicTreeNode {
                                name: "notifications".to_string(),
                                partition_count: 6,
                                partitions_state: NodeState::Collapsed,
                                partitions: vec![
                                    PartitionTreeNode { id: 0, leader: None },
                                    PartitionTreeNode { id: 1, leader: None },
                                    PartitionTreeNode { id: 2, leader: None },
                                    PartitionTreeNode { id: 3, leader: None },
                                    PartitionTreeNode { id: 4, leader: None },
                                    PartitionTreeNode { id: 5, leader: None },
                                ],
                            },
                        ],
                    },
                ],
            },
            TreeGroup {
                group: ClusterGroup {
                    id: 2,
                    name: "Development".to_string(),
                    color: Some(theme.warning),
                },
                state: NodeState::Expanded,
                clusters: vec![
                    ClusterTreeNode {
                        id: "dev-1".to_string(),
                        name: "Dev Cluster".to_string(),
                        status: ConnectionStatusType::Connected,
                        group_id: Some(2),
                        topics_state: NodeState::Collapsed,
                        topics: vec![
                            TopicTreeNode {
                                name: "test-topic".to_string(),
                                partition_count: 1,
                                partitions_state: NodeState::Collapsed,
                                partitions: vec![PartitionTreeNode { id: 0, leader: None }],
                            },
                            TopicTreeNode {
                                name: "debug-events".to_string(),
                                partition_count: 1,
                                partitions_state: NodeState::Collapsed,
                                partitions: vec![PartitionTreeNode { id: 0, leader: None }],
                            },
                        ],
                    },
                    ClusterTreeNode {
                        id: "dev-2".to_string(),
                        name: "Local Kafka".to_string(),
                        status: ConnectionStatusType::Disconnected,
                        group_id: Some(2),
                        topics_state: NodeState::Collapsed,
                        topics: vec![],
                    },
                ],
            },
            TreeGroup {
                group: ClusterGroup {
                    id: 0,
                    name: " Ungrouped".to_string(),
                    color: None,
                },
                state: NodeState::Collapsed,
                clusters: vec![
                    ClusterTreeNode {
                        id: "standalone-1".to_string(),
                        name: "Standalone".to_string(),
                        status: ConnectionStatusType::Error,
                        group_id: None,
                        topics_state: NodeState::Collapsed,
                        topics: vec![],
                    },
                ],
            },
        ];

        Self {
            theme,
            translations,
            groups,
            selected_node: Some("orders".to_string()),
            search_filter: String::new(),
            expand_all: false,
        }
    }

    /// Set search filter
    pub fn set_search_filter(&mut self, filter: String) {
        self.search_filter = filter;
        // Auto-expand matching clusters/topics
        if !filter.is_empty() {
            self.expand_all = true;
        }
    }

    /// Get search filter
    pub fn search_filter(&self) -> &str {
        &self.search_filter
    }

    /// Clear search filter
    pub fn clear_search(&mut self) {
        self.search_filter.clear();
        self.expand_all = false;
    }

    /// Toggle all clusters expanded/collapsed
    pub fn toggle_expand_all(&mut self) {
        self.expand_all = !self.expand_all;
        for group in &mut self.groups {
            group.state = if self.expand_all { NodeState::Expanded } else { NodeState::Collapsed };
            for cluster in &mut group.clusters {
                cluster.topics_state = if self.expand_all { NodeState::Expanded } else { NodeState::Collapsed };
            }
        }
    }

    /// Check if topic matches search filter
    fn topic_matches_filter(&self, topic: &TopicTreeNode) -> bool {
        if self.search_filter.is_empty() {
            return true;
        }
        topic.name.to_lowercase().contains(&self.search_filter.to_lowercase())
    }

    /// Check if cluster matches search filter (has matching topics)
    fn cluster_matches_filter(&self, cluster: &ClusterTreeNode) -> bool {
        if self.search_filter.is_empty() {
            return true;
        }
        // Match cluster name or any topic name
        cluster.name.to_lowercase().contains(&self.search_filter.to_lowercase())
            || cluster.topics.iter().any(|t| self.topic_matches_filter(t))
    }

    /// Check if group matches search filter
    fn group_matches_filter(&self, group: &TreeGroup) -> bool {
        if self.search_filter.is_empty() {
            return true;
        }
        // Match group name or any cluster/topic
        group.group.name.to_lowercase().contains(&self.search_filter.to_lowercase())
            || group.clusters.iter().any(|c| self.cluster_matches_filter(c))
    }

    /// Render status indicator
    fn status_indicator(&self, status: ConnectionStatusType) -> Div {
        let theme = &self.theme;
        let color = match status {
            ConnectionStatusType::Connected => theme.success,
            ConnectionStatusType::Disconnected => theme.text_muted,
            ConnectionStatusType::Error => theme.error,
            ConnectionStatusType::Unknown => theme.warning,
        };

        div()
            .w(px(8.0))
            .h(px(8.0))
            .rounded(px(4.0))
            .bg(color)
    }

    /// Render expand/collapse icon with rotation based on state
    fn expand_icon(&self, state: NodeState) -> Div {
        let theme = &self.theme;

        div()
            .w(px(12.0))
            .h(px(12.0))
            .rounded(px(2.0))
            .bg(theme.text_muted.opacity(0.5))
    }

    /// Render group header
    fn group_header(&self, group: &TreeGroup) -> Div {
        let theme = &self.theme;
        let color = group.group.color.unwrap_or(theme.text_muted);

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .bg(theme.surface.opacity(0.3))
            .border_b(px(1.0))
            .border_color(theme.border)
            .cursor_pointer()
            .child(self.expand_icon(group.state))
            .when_some(group.group.color, |this, c| {
                this.child(
                    div()
                        .w(px(4.0))
                        .h(px(16.0))
                        .rounded(px(2.0))
                        .bg(c)
                )
            })
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(group.group.name.clone())
            )
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(format!("({})", group.clusters.len()))
            )
    }

    /// Render cluster node
    fn cluster_node(&self, cluster: &ClusterTreeNode, group_color: Option<Hsla>) -> Div {
        let theme = &self.theme;
        let filtered_topics: Vec<&TopicTreeNode> = cluster.topics.iter()
            .filter(|t| self.topic_matches_filter(t))
            .collect();

        // Don't render if no topics match filter
        if !self.search_filter.is_empty() && filtered_topics.is_empty() && !cluster.name.to_lowercase().contains(&self.search_filter.to_lowercase()) {
            return div();
        }

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .ml(px(8.0))
            .child(
                // Cluster header
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .bg(theme.surface_raised.opacity(0.5))
                    .cursor_pointer()
                    .child(self.status_indicator(cluster.status))
                    .child(self.expand_icon(cluster.topics_state))
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(cluster.name.clone())
                    )
                    .when(cluster.topics_state == NodeState::Expanded, |this| {
                        this.child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(format!("{} topics", cluster.topics.len()))
                        )
                    })
            )
            .when(cluster.topics_state == NodeState::Expanded || self.expand_all, |this| {
                this.child(
                    // Topics folder
                    div()
                        .flex()
                        .flex_col()
                        .ml(px(16.0))
                        .gap(px(2.0))
                        .children(filtered_topics.iter().map(|topic| {
                            self.topic_node(topic, group_color)
                        }))
                )
            })
    }

    /// Render topic node with partition expand/collapse
    fn topic_node(&self, topic: &TopicTreeNode, _group_color: Option<Hsla>) -> Div {
        let theme = &self.theme;
        let is_selected = self.selected_node == Some(topic.name.clone());

        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                // Topic row
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(if is_selected { theme.primary.opacity(0.2) } else { gpui::transparent_black() })
                    .border_l(px(2.0))
                    .border_color(if is_selected { theme.primary } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .child(
                        // Topic icon placeholder
                        div()
                            .w(px(10.0))
                            .h(px(10.0))
                            .rounded(px(2.0))
                            .bg(if is_selected { theme.primary } else { theme.text_muted })
                    )
                    .child(
                        div()
                            .text_color(if is_selected { theme.text } else { theme.text_secondary })
                            .text_xs()
                            .child(topic.name.clone())
                    )
                    .child(
                        // Partition count badge
                        div()
                            .px(px(4.0))
                            .py(px(2.0))
                            .rounded(px(3.0))
                            .bg(theme.surface_raised)
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child(format!("P{}", topic.partition_count))
                            )
                    )
                    .when(topic.partitions_state == NodeState::Expanded, |this| {
                        this.child(self.expand_icon(NodeState::Expanded))
                    })
            )
            .when(topic.partitions_state == NodeState::Expanded, |this| {
                this.child(
                    // Partitions list
                    div()
                        .flex()
                        .flex_col()
                        .ml(px(16.0))
                        .gap(px(1.0))
                        .children(topic.partitions.iter().map(|partition| {
                            self.partition_node(partition)
                        }))
                )
            })
    }

    /// Render partition node with leader info
    fn partition_node(&self, partition: &PartitionTreeNode) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(6.0))
            .py(px(2.0))
            .cursor_pointer()
            .child(
                div()
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded(px(2.0))
                    .bg(theme.text_muted.opacity(0.5))
            )
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(format!("P{}", partition.id))
            )
            .when_some(partition.leader, |this, leader| {
                this.child(
                    div()
                        .text_color(theme.success.opacity(0.7))
                        .text_xs()
                        .child(format!("L{}", leader))
                )
            })
    }

    /// Render search result count
    fn search_result_indicator(&self) -> Div {
        let theme = &self.theme;

        if self.search_filter.is_empty() {
            return div();
        }

        let count = self.groups.iter()
            .flat_map(|g| g.clusters.iter())
            .filter(|c| self.cluster_matches_filter(c))
            .count();

        div()
            .flex()
            .items_center()
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(theme.primary.opacity(0.1))
            .child(
                div()
                    .text_color(theme.primary)
                    .text_xs()
                    .child(format!("Found {} matches", count))
            )
    }
}

impl IntoElement for ClusterTreeNavigator {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let t = &self.translations;

        // Filter groups
        let filtered_groups: Vec<&TreeGroup> = self.groups.iter()
            .filter(|g| self.group_matches_filter(g))
            .collect();

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(8.0))
            .child(
                // Search box with filter
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(8.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(if !self.search_filter.is_empty() { theme.primary } else { theme.border })
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(6.0))
                                    .bg(theme.text_muted)
                            )
                            .child(
                                div()
                                    .text_color(if !self.search_filter.is_empty() { theme.text } else { theme.text_muted })
                                    .text_xs()
                                    .child(if self.search_filter.is_empty() {
                                        t.layout.search_placeholder.clone()
                                    } else {
                                        self.search_filter.clone()
                                    })
                            )
                            .when(!self.search_filter.is_empty(), |this| {
                                this.child(
                                    div()
                                        .px(px(4.0))
                                        .py(px(2.0))
                                        .rounded(px(3.0))
                                        .bg(theme.surface_raised)
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .text_color(theme.text_muted)
                                                .text_xs()
                                                .child("×")
                                        )
                                )
                            })
                    )
                    .child(self.search_result_indicator())
            )
            .child(
                // Expand all/collapse all button
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(8.0))
                    .py(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child("Expand/Collapse")
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(3.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(if self.expand_all { "Collapse" } else { "Expand" })
                            )
                    )
            )
            .child(
                // Tree content with groups
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .size_full()
                    .overflow_y_scroll()
                    .children(filtered_groups.iter().map(|group| {
                        let group_color = group.group.color;
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(self.group_header(group))
                            .when(group.state == NodeState::Expanded, |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.0))
                                        .children(group.clusters.iter()
                                            .filter(|c| self.cluster_matches_filter(c))
                                            .map(|cluster| {
                                                self.cluster_node(cluster, group_color)
                                            }))
                                )
                            })
                    }))
            )
    }
}