//! Cluster Tree Navigator
//!
//! Tree view for clusters and topics navigation.

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

/// Tree navigator
pub struct ClusterTreeNavigator {
    theme: Theme,
    translations: Arc<Translations>,
    /// Mock tree data
    clusters: Vec<ClusterTreeNode>,
    selected_node: Option<String>,
}

/// Cluster tree node
struct ClusterTreeNode {
    name: String,
    status: ConnectionStatusType,
    topics_state: NodeState,
    topics: Vec<TopicTreeNode>,
}

/// Topic tree node
struct TopicTreeNode {
    name: String,
    partitions: Vec<PartitionTreeNode>,
}

/// Partition tree node
struct PartitionTreeNode {
    id: i32,
}

impl ClusterTreeNavigator {
    /// Create new tree navigator
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        // Mock data
        let clusters = vec![
            ClusterTreeNode {
                name: "Production".to_string(),
                status: ConnectionStatusType::Connected,
                topics_state: NodeState::Expanded,
                topics: vec![
                    TopicTreeNode {
                        name: "orders".to_string(),
                        partitions: vec![
                            PartitionTreeNode { id: 0 },
                            PartitionTreeNode { id: 1 },
                            PartitionTreeNode { id: 2 },
                        ],
                    },
                    TopicTreeNode {
                        name: "payments".to_string(),
                        partitions: vec![PartitionTreeNode { id: 0 }],
                    },
                ],
            },
            ClusterTreeNode {
                name: "Development".to_string(),
                status: ConnectionStatusType::Disconnected,
                topics_state: NodeState::Collapsed,
                topics: vec![
                    TopicTreeNode {
                        name: "test-topic".to_string(),
                        partitions: vec![PartitionTreeNode { id: 0 }],
                    },
                ],
            },
        ];

        Self {
            theme,
            translations,
            clusters,
            selected_node: Some("orders".to_string()),
        }
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

    /// Render expand/collapse icon
    fn expand_icon(&self, _state: NodeState) -> Div {
        let theme = &self.theme;

        div()
            .w(px(12.0))
            .h(px(12.0))
            .rounded(px(2.0))
            .bg(theme.text_muted.opacity(0.5))
    }

    /// Render cluster node
    fn cluster_node(&self, cluster: &ClusterTreeNode) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
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
            )
            .when(cluster.topics_state == NodeState::Expanded, |this| {
                this.child(
                    // Topics folder
                    div()
                        .flex()
                        .flex_col()
                        .ml(px(20.0))
                        .gap(px(2.0))
                        .children(cluster.topics.iter().map(|topic| {
                            self.topic_node(topic)
                        }))
                )
            })
    }

    /// Render topic node
    fn topic_node(&self, topic: &TopicTreeNode) -> Div {
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
            )
            .child(
                // Partitions (collapsed by default)
                div()
                    .flex()
                    .flex_col()
                    .ml(px(16.0))
                    .gap(px(1.0))
                    .children(topic.partitions.iter().map(|partition| {
                        self.partition_node(partition)
                    }))
            )
    }

    /// Render partition node
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
    }
}

impl IntoElement for ClusterTreeNavigator {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;
        let t = &self.translations;

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(8.0))
            .child(
                // Search box
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(8.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .bg(theme.text_muted)
                    )
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(t.layout.search_placeholder.clone())
                    )
            )
            .child(
                // Tree content
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .size_full()
                    .children(self.clusters.iter().map(|cluster| {
                        self.cluster_node(cluster)
                    }))
            )
    }
}