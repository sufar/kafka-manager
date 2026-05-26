//! Cluster Tree Navigator
//!
//! Tree view for clusters and topics navigation.
//! Matches Vue3 ClusterTreeNavigator.vue visual styling.

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
    expanded_clusters: Vec<String>,
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
            expanded_clusters: vec!["Production".to_string()],
        }
    }

    /// Render health indicator - Vue3: w-2 h-2 rounded-full (8px circle) with glow
    fn health_indicator(&self, status: ConnectionStatusType, refreshing: bool) -> Div {
        let theme = &self.theme;
        let (color, animate) = if refreshing {
            (theme.warning, true)
        } else {
            match status {
                ConnectionStatusType::Connected => (theme.success, false),
                ConnectionStatusType::Disconnected => (theme.text_muted, false),
                ConnectionStatusType::Error => (theme.error, false),
                ConnectionStatusType::Unknown => (theme.warning, false),
            }
        };

        // Vue3: w-2 h-2 rounded-full = 8px diameter circle
        div()
            .w(px(8.0))
            .h(px(8.0))
            .rounded(px(8.0))  // Full circle (radius = diameter/2)
            .bg(color)
            .when(animate, |d| {
                // Pulse animation for refreshing state
                d.opacity(0.7)
            })
    }

    /// Render expand/collapse icon - Vue3: w-3 h-3 (12px)
    fn expand_icon(&self, state: NodeState) -> Div {
        let theme = &self.theme;
        // Vue3: chevron icon placeholder, 12px
        div()
            .w(px(12.0))
            .h(px(12.0))
            .rounded(px(2.0))
            .bg(theme.text_muted.opacity(0.5))
            .when(state == NodeState::Expanded, |d| {
                // Rotation would be applied here in Vue3
                d
            })
    }

    /// Render header section - Vue3: icon + title + action buttons
    fn render_header(&self) -> Div {
        let theme = &self.theme;
        // Vue3: w-8 h-8 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20
        div()
            .flex()
            .items_center()
            .gap(px(8.0))  // Vue3: gap-2 = 8px
            .p(px(8.0))    // Vue3: p-2 = 8px
            .mb(px(8.0))   // Vue3: mb-2 = 8px
            .child(
                // Icon container - Vue3: w-8 h-8 rounded-xl (32px, 12px rounded)
                div()
                    .w(px(32.0))
                    .h(px(32.0))
                    .rounded(px(12.0))
                    .bg(theme.primary.opacity(0.20))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        // Icon placeholder - Vue3: w-5 h-5 (20px)
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(4.0))
                            .bg(theme.primary)
                    )
            )
            .child(
                // Title - Vue3: text-xs font-bold uppercase tracking-wider
                div()
                    .text_color(theme.text_muted)
                    .text_size(px(12.0))
                    .font_weight(FontWeight::BOLD)
                    .child("CLUSTERS")
            )
    }

    /// Render action buttons row - Vue3: collapse, clusters, favorites, schema, history
    fn render_action_buttons(&self) -> Div {
        let theme = &self.theme;
        // Vue3: btn btn-ghost btn-xs = 24px buttons with w-4 h-4 icons (16px)
        div()
            .flex()
            .gap(px(2.0))  // Vue3: gap-0.5 = 2px
            .child(
                // Collapse button - Vue3: btn btn-ghost btn-xs
                div()
                    .w(px(24.0))
                    .h(px(24.0))
                    .rounded(px(12.0))  // btn-circle
                    .bg(theme.surface)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.surface.opacity(0.80)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(3.0))
                            .bg(theme.text_muted)
                    )
            )
            .child(
                // Clusters button - Vue3: btn btn-ghost btn-xs
                div()
                    .w(px(24.0))
                    .h(px(24.0))
                    .rounded(px(12.0))
                    .bg(theme.surface)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.surface.opacity(0.80)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(3.0))
                            .bg(theme.primary)
                    )
            )
            .child(
                // Favorites button - Vue3: btn btn-ghost btn-xs
                div()
                    .w(px(24.0))
                    .h(px(24.0))
                    .rounded(px(12.0))
                    .bg(theme.surface)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.surface.opacity(0.80)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(3.0))
                            .bg(theme.text_muted)
                    )
            )
            .child(
                // Schema Registry button - Vue3: btn btn-ghost btn-xs
                div()
                    .w(px(24.0))
                    .h(px(24.0))
                    .rounded(px(12.0))
                    .bg(theme.surface)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.surface.opacity(0.80)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(3.0))
                            .bg(theme.text_muted)
                    )
            )
            .child(
                // History button - Vue3: btn btn-ghost btn-xs
                div()
                    .w(px(24.0))
                    .h(px(24.0))
                    .rounded(px(12.0))
                    .bg(theme.surface)
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.surface.opacity(0.80)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(3.0))
                            .bg(theme.text_muted)
                    )
            )
    }

    /// Render cluster node - Vue3: p-2 rounded-xl (8px padding, 12px rounded)
    fn cluster_node(&self, cluster: &ClusterTreeNode, is_expanded: bool) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .mb(px(4.0))  // Vue3: mb-1 = 4px
            .child(
                // Cluster header - Vue3: p-2 rounded-xl
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))  // Vue3: gap-1.5 = 6px
                    .p(px(8.0))    // Vue3: p-2 = 8px
                    .rounded(px(12.0))  // Vue3: rounded-xl = 12px
                    .bg(if is_expanded { theme.primary.opacity(0.10) } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.primary.opacity(0.05)))
                    .child(
                        // Expand/collapse button - Vue3: btn btn-ghost btn-xs p-0 w-5 h-5
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded(px(10.0))
                            .bg(theme.surface)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(self.expand_icon(if is_expanded { NodeState::Expanded } else { NodeState::Collapsed }))
                    )
                    .child(self.health_indicator(cluster.status, false))
                    .child(
                        // Cluster icon placeholder - Vue3: w-4 h-4 (16px)
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(theme.primary)
                    )
                    .child(
                        // Cluster name - Vue3: text-sm font-semibold
                        div()
                            .text_color(theme.text)
                            .text_size(px(14.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(cluster.name.clone())
                    )
            )
            .when(is_expanded, |this| {
                this.child(
                    // Topics folder
                    div()
                        .flex()
                        .flex_col()
                        .ml(px(8.0))  // Vue3: ml-2 = 8px (indentation)
                        .gap(px(2.0))
                        .child(self.render_topics_folder(cluster))
                        .child(self.render_consumer_groups_folder(cluster))
                )
            })
    }

    /// Render topics folder - Vue3: p-1.5 rounded-lg (6px padding, 8px rounded)
    fn render_topics_folder(&self, cluster: &ClusterTreeNode) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .mb(px(2.0))  // Vue3: mb-0.5 = 2px
            .child(
                // Topics folder header - Vue3: p-1.5 rounded-lg
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))  // Vue3: gap-1 = 4px
                    .p(px(6.0))    // Vue3: p-1.5 = 6px
                    .rounded(px(8.0))  // Vue3: rounded-lg = 8px
                    .bg(theme.secondary.opacity(0.10))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary.opacity(0.05)))
                    .child(
                        // Expand icon - Vue3: btn btn-ghost btn-xs p-0 w-4 h-4
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(8.0))
                            .bg(theme.surface)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(2.0))
                                    .bg(theme.text_muted.opacity(0.5))
                            )
                    )
                    .child(
                        // Folder icon placeholder - Vue3: w-4 h-4 (16px)
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(theme.secondary)
                    )
                    .child(
                        // Folder name - Vue3: text-sm font-medium
                        div()
                            .text_color(theme.text)
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Topics")
                    )
                    .child(
                        // Topic count badge - Vue3: badge badge-ghost badge-xs
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(6.0))
                            .bg(theme.surface.opacity(0.80))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_size(px(10.0))
                                    .child(format!("{}", cluster.topics.len()))
                            )
                    )
            )
            .child(
                // Topics list
                div()
                    .flex()
                    .flex_col()
                    .ml(px(8.0))  // Indentation
                    .gap(px(2.0))
                    .children(cluster.topics.iter().map(|topic| {
                        self.topic_node(topic)
                    }))
            )
    }

    /// Render consumer groups folder - Vue3: similar structure
    fn render_consumer_groups_folder(&self, _cluster: &ClusterTreeNode) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .child(
                // Consumer groups folder header
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .p(px(6.0))
                    .rounded(px(8.0))
                    .bg(theme.surface.opacity(0.50))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.surface.opacity(0.80)))
                    .child(
                        // Expand icon
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(8.0))
                            .bg(theme.surface)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .rounded(px(2.0))
                                    .bg(theme.text_muted.opacity(0.5))
                            )
                    )
                    .child(
                        // Icon placeholder
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(theme.text_muted)
                    )
                    .child(
                        // Folder name
                        div()
                            .text_color(theme.text)
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Consumer Groups")
                    )
            )
    }

    /// Render topic node - Vue3: p-1.5 rounded-lg
    fn topic_node(&self, topic: &TopicTreeNode) -> Div {
        let theme = &self.theme;
        let is_selected = self.selected_node == Some(topic.name.clone());

        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                // Topic row - Vue3: p-1.5 rounded-lg
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))  // Vue3: gap-1 = 4px
                    .p(px(6.0))    // Vue3: p-1.5 = 6px
                    .rounded(px(8.0))  // Vue3: rounded-lg = 8px
                    .bg(if is_selected { theme.primary.opacity(0.30) } else { gpui::transparent_black() })
                    .border_l(px(2.0))  // Vue3: border-l-2
                    .border_color(if is_selected { theme.primary } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.primary.opacity(0.10)))
                    .child(
                        // Topic icon placeholder - Vue3: w-4 h-4 (16px)
                        div()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(if is_selected { theme.primary } else { theme.text_muted })
                    )
                    .child(
                        // Topic name - Vue3: text-sm
                        div()
                            .text_color(if is_selected { theme.text } else { theme.text_secondary })
                            .text_size(px(14.0))
                            .child(topic.name.clone())
                    )
            )
            .child(
                // Partitions
                div()
                    .flex()
                    .flex_col()
                    .ml(px(12.0))  // Indentation
                    .gap(px(2.0))
                    .children(topic.partitions.iter().map(|partition| {
                        self.partition_node(partition)
                    }))
            )
    }

    /// Render partition node - Vue3: smaller styling
    fn partition_node(&self, partition: &PartitionTreeNode) -> Div {
        let theme = &self.theme;

        div()
            .flex()
            .items_center()
            .gap(px(4.0))  // Vue3: gap-1 = 4px
            .p(px(4.0))    // Vue3: p-1 = 4px
            .rounded(px(6.0))  // Vue3: rounded = 4px (base rounded)
            .cursor_pointer()
            .hover(|d| d.bg(theme.surface.opacity(0.50)))
            .child(
                // Partition icon placeholder - Vue3: w-3 h-3 (12px)
                div()
                    .w(px(12.0))
                    .h(px(12.0))
                    .rounded(px(3.0))
                    .bg(theme.text_muted.opacity(0.5))
            )
            .child(
                // Partition label - Vue3: text-xs
                div()
                    .text_color(theme.text_muted)
                    .text_size(px(12.0))
                    .child(format!("P{}", partition.id))
            )
    }
}

impl IntoElement for ClusterTreeNavigator {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap(px(8.0))  // Vue3: gap-2 = 8px
            .p(px(8.0))    // Vue3: p-2 = 8px
            .child(
                // Header section
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .flex_shrink_0()
                    .child(
                        // Header row: icon + title + buttons
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        // Icon container - Vue3: w-8 h-8 rounded-xl
                                        div()
                                            .w(px(32.0))
                                            .h(px(32.0))
                                            .rounded(px(12.0))
                                            .bg(theme.primary.opacity(0.20))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .w(px(20.0))
                                                    .h(px(20.0))
                                                    .rounded(px(4.0))
                                                    .bg(theme.primary)
                                            )
                                    )
                                    .child(
                                        // Title - Vue3: text-xs font-bold uppercase tracking-wider
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_size(px(12.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child("CLUSTERS")
                                    )
                            )
                            .child(self.render_action_buttons())
                    )
            )
            .child(
                // Tree content - scrollable
                div()
                    .id("tree-content")
                    .flex()
                    .flex_col()
                    .gap(px(4.0))  // Vue3: gap-1 = 4px
                    .size_full()
                    .overflow_y_scroll()
                    .children(self.clusters.iter().map(|cluster| {
                        let is_expanded = self.expanded_clusters.contains(&cluster.name);
                        self.cluster_node(cluster, is_expanded)
                    }))
            )
    }
}