//! Clusters View
//!
//! Main view for managing Kafka clusters with tree navigation.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{ClusterTreeNavigator, ClusterContextMenu, TopicsFolderContextMenu, ClusterAction, TopicsFolderAction};
use crate::state::ConnectionStatusType;
use crate::api::{ApiClient, ApiError, ClusterResponse, CreateClusterRequest, UpdateClusterRequest, ClusterGroupResponse, CreateClusterGroupRequest};

/// Clusters management view
pub struct ClustersView {
    theme: Theme,
    translations: Arc<Translations>,
    /// Mock data for demonstration
    clusters: Vec<ClusterDisplay>,
    selected_group: Option<i32>,
    /// Tree navigator for cluster/topics
    tree_nav: ClusterTreeNavigator,
    /// Context menu state
    context_menu: ClusterContextMenu,
    /// Topics folder context menu
    topics_folder_menu: TopicsFolderContextMenu,
    /// API client for backend communication
    api_client: ApiClient,
    /// Current error message
    error: Option<ApiError>,
}

/// Cluster display data
struct ClusterDisplay {
    name: String,
    brokers: String,
    status: ConnectionStatusType,
    group_name: Option<String>,
}

impl ClusterDisplay {
    /// Convert from API response
    fn from_response(response: &ClusterResponse) -> Self {
        Self {
            name: response.name.clone(),
            brokers: response.brokers.clone(),
            status: ConnectionStatusType::Connected,
            group_name: None,
        }
    }

    /// Convert from API response with group info
    fn from_response_with_group(response: &ClusterResponse, group: Option<&ClusterGroupResponse>) -> Self {
        Self {
            name: response.name.clone(),
            brokers: response.brokers.clone(),
            status: ConnectionStatusType::Connected,
            group_name: group.map(|g| g.name.clone()),
        }
    }
}

/// Cluster group display data
struct ClusterGroupDisplay {
    id: i32,
    name: String,
    description: Option<String>,
}

impl ClusterGroupDisplay {
    /// Convert from API response
    fn from_response(response: &ClusterGroupResponse) -> Self {
        Self {
            id: response.id,
            name: response.name.clone(),
            description: response.description.clone(),
        }
    }
}

impl ClustersView {
    /// Create new clusters view
    pub fn new(theme: Theme, translations: Arc<Translations>) -> Self {
        // Mock data for demonstration
        let clusters = vec![
            ClusterDisplay {
                name: "Production".to_string(),
                brokers: "kafka-prod-1:9092,kafka-prod-2:9092".to_string(),
                status: ConnectionStatusType::Connected,
                group_name: Some("Production".to_string()),
            },
            ClusterDisplay {
                name: "Development".to_string(),
                brokers: "localhost:9092".to_string(),
                status: ConnectionStatusType::Disconnected,
                group_name: Some("Dev".to_string()),
            },
            ClusterDisplay {
                name: "Staging".to_string(),
                brokers: "kafka-staging:9092".to_string(),
                status: ConnectionStatusType::Unknown,
                group_name: None,
            },
        ];

        Self {
            theme: theme.clone(),
            translations: translations.clone(),
            clusters,
            selected_group: None,
            tree_nav: ClusterTreeNavigator::new(theme.clone(), translations),
            context_menu: ClusterContextMenu::new(
                theme.clone(),
                "Production".to_string(),
                Point::new(px(100.0), px(100.0)),
            ),
            topics_folder_menu: TopicsFolderContextMenu::new(
                theme,
                "Production".to_string(),
                Point::new(px(200.0), px(200.0)),
            ),
            api_client: ApiClient::default_client(),
            error: None,
        }
    }

    /// Build create cluster request
    fn build_create_request(&self, name: String, brokers: String) -> CreateClusterRequest {
        CreateClusterRequest {
            name,
            brokers,
            request_timeout_ms: Some(30000),
            operation_timeout_ms: Some(30000),
            group_id: self.selected_group,
        }
    }

    /// Build update cluster request
    fn build_update_request(&self, cluster_name: String, new_brokers: String) -> UpdateClusterRequest {
        UpdateClusterRequest {
            name: cluster_name,
            brokers: new_brokers,
            request_timeout_ms: Some(30000),
            operation_timeout_ms: Some(30000),
            group_id: self.selected_group,
        }
    }

    /// Build create cluster group request
    fn build_create_group_request(name: String, description: Option<String>) -> CreateClusterGroupRequest {
        CreateClusterGroupRequest {
            name,
            description,
            sort_order: None,
        }
    }

    /// Handle cluster context menu action
    fn handle_cluster_action(action: ClusterAction, cluster_name: &str) {
        match action {
            ClusterAction::TestConnection => println!("Testing connection to {}", cluster_name),
            ClusterAction::RefreshStatus => println!("Refreshing status for {}", cluster_name),
            ClusterAction::Disconnect => println!("Disconnecting from {}", cluster_name),
            ClusterAction::Reconnect => println!("Reconnecting to {}", cluster_name),
            ClusterAction::ViewBrokers => println!("Viewing brokers for {}", cluster_name),
            ClusterAction::ViewTopics => println!("Viewing topics for {}", cluster_name),
            ClusterAction::RefreshTopics => println!("Refreshing topics for {}", cluster_name),
            ClusterAction::ViewConsumerGroups => println!("Viewing consumer groups for {}", cluster_name),
            ClusterAction::CreateTopic => println!("Creating topic in {}", cluster_name),
            ClusterAction::EditCluster => println!("Editing {}", cluster_name),
            ClusterAction::DeleteCluster => println!("Deleting {}", cluster_name),
        }
    }

    /// Handle topics folder context menu action
    fn handle_topics_folder_action(action: TopicsFolderAction, cluster_name: &str) {
        match action {
            TopicsFolderAction::RefreshTopics => println!("Refreshing topics for {}", cluster_name),
            TopicsFolderAction::CreateTopic => println!("Creating topic in {}", cluster_name),
            TopicsFolderAction::ViewAllTopics => println!("Viewing all topics in {}", cluster_name),
        }
    }

    /// Render a cluster card - Vue3: card glass gradient-border p-3
    fn render_cluster_card(theme: &Theme, t: &Arc<Translations>, cluster: &ClusterDisplay, _index: usize) -> Div {
        div()
            .flex()
            .flex_col()
            .p(px(12.0))   // Vue3: p-3 = 12px
            .gap(px(12.0))
            .rounded(px(12.0))  // Vue3: rounded-xl for glass cards
            .bg(theme.surface.opacity(0.80))  // Vue3: glass effect
            .border(px(1.0))
            .border_color(theme.border.opacity(0.10))  // Vue3: gradient-border
            .child(
                // Header: Icon + Name + Status - Vue3: p-3 border-b border-base-content/10
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb(px(12.0))  // Vue3: border-b padding
                    .border_b(px(1.0))
                    .border_color(theme.border.opacity(0.10))
                    .child(
                        // Icon + Name container - Vue3: gap-2 = 8px
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                // Cluster icon - Vue3: w-8 h-8 rounded-lg bg-primary/10
                                div()
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .rounded(px(8.0))
                                    .bg(theme.primary.opacity(0.10))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        // Inner icon placeholder - Vue3: w-5 h-5 = 20px
                                        div()
                                            .w(px(20.0))
                                            .h(px(20.0))
                                            .rounded(px(4.0))
                                            .bg(theme.primary)
                                    )
                            )
                            .child(
                                // Name container
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        // Title - Vue3: font-semibold text-sm = 14px
                                        div()
                                            .text_color(theme.text)
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(cluster.name.clone())
                                    )
                                    .child(
                                        // Created date - Vue3: text-[10px] = 10px
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_size(px(10.0))
                                            .child("Created: 2024-01-01")
                                    )
                            )
                    )
                    .child(
                        // Edit/Delete buttons + Status badge
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))  // Vue3: gap-1 = 4px
                            .child(
                                // Edit button - Vue3: btn btn-xs btn-ghost h-auto p-1
                                div()
                                    .w(px(24.0))   // Vue3: btn-xs = 24px
                                    .h(px(24.0))
                                    .rounded(px(12.0))  // btn-circle
                                    .bg(theme.surface)
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.surface.opacity(0.80)))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        // Edit icon placeholder - Vue3: w-4 h-4 = 16px
                                        div()
                                            .w(px(16.0))
                                            .h(px(16.0))
                                            .rounded(px(3.0))
                                            .bg(theme.text_muted)
                                    )
                            )
                            .child(
                                // Delete button - Vue3: btn btn-xs btn-ghost text-error
                                div()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(12.0))
                                    .bg(theme.surface)
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.error.opacity(0.10)))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .w(px(16.0))
                                            .h(px(16.0))
                                            .rounded(px(3.0))
                                            .bg(theme.error)
                                    )
                            )
                            .child(Self::status_badge(theme, cluster.status))
                    )
            )
            // Card body - Vue3: card-body p-3
            .child(
                // Brokers section - Vue3: mb-2
                div()
                    .mb(px(8.0))
                    .child(
                        // Label - Vue3: text-[10px] uppercase tracking-wider
                        div()
                            .text_color(theme.text_muted)
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.clusters.brokers.clone().to_uppercase())
                    )
                    .child(
                        // Value - Vue3: text-xs font-mono
                        div()
                            .text_color(theme.text)
                            .text_size(px(12.0))
                            .font_weight(FontWeight::NORMAL)
                            .child(cluster.brokers.clone())
                    )
            )
            // Timeouts section - Vue3: mb-2
            .child(
                div()
                    .mb(px(8.0))
                    .child(
                        // Label - Vue3: text-[10px] uppercase tracking-wider
                        div()
                            .text_color(theme.text_muted)
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child(t.clusters.timeouts.clone().to_uppercase())
                    )
                    .child(
                        // Values - Vue3: text-xs
                        div()
                            .text_color(theme.text)
                            .text_size(px(12.0))
                            .child(format!("Request: {}ms | Operation: {}ms", 30000, 60000))
                    )
            )
            .when_some(cluster.group_name.clone(), |this, group| {
                this.child(
                    // Group section
                    div()
                        .mb(px(8.0))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_size(px(10.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(t.clusters.group.clone().to_uppercase())
                        )
                        .child(
                            // Group badge - Vue3: badge badge-ghost badge-xs
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(6.0))
                                .bg(theme.surface.opacity(0.80))
                                .text_color(theme.text_muted)
                                .text_size(px(10.0))
                                .child(group)
                        )
                )
            })
            // Actions - Vue3: mt-4 flex gap-2
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))  // Vue3: gap-2 = 8px
                    .mt(px(16.0))  // Vue3: mt-4 = 16px
                    .child(
                        // Test connection button - Vue3: btn btn-primary btn-sm
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))  // Vue3: gap-1 = 4px
                            .h(px(32.0))   // Vue3: btn-sm h-8 = 32px
                            .px(px(12.0))
                            .rounded(px(8.0))  // Vue3: rounded-lg = 8px
                            .bg(theme.primary)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.primary.opacity(0.90)))
                            .child(
                                // Icon placeholder - Vue3: w-4 h-4 = 16px
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .rounded(px(3.0))
                                    .bg(Hsla::from(gpui::rgb(0xffffff)))
                            )
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_size(px(14.0))
                                    .child(t.clusters.test_connection.clone())
                            )
                    )
                    .child(
                        // View topics button - Vue3: btn btn-outline btn-sm
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .h(px(32.0))
                            .px(px(12.0))
                            .rounded(px(8.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.surface.opacity(0.80)))
                            .child(
                                // Icon placeholder
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .rounded(px(3.0))
                                    .bg(theme.text_muted)
                            )
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_size(px(14.0))
                                    .child(t.clusters.view_topics_link.clone())
                            )
                    )
            )
    }

    /// Render status badge - Vue3: badge gap-1 badge-xs, w-1.5 h-1.5 rounded-full
    fn status_badge(theme: &Theme, status: ConnectionStatusType) -> Div {
        let (bg_color, indicator_color, text_color, text) = match status {
            ConnectionStatusType::Connected => (
                theme.success.opacity(0.20),  // badge-success
                theme.success,                 // bg-success
                theme.success,                 // text-success
                "已连接"
            ),
            ConnectionStatusType::Disconnected => (
                theme.surface.opacity(0.80),   // badge-ghost
                theme.text_muted,              // muted
                theme.text_muted,
                "未连接"
            ),
            ConnectionStatusType::Error => (
                theme.error.opacity(0.20),     // badge-error
                theme.error,                    // bg-error
                theme.error,
                "错误"
            ),
            ConnectionStatusType::Unknown => (
                theme.surface.opacity(0.80),
                theme.text_muted,
                theme.text_muted,
                "未知"
            ),
        };

        // Vue3: badge gap-1 badge-xs = 12px height, gap-1 = 4px
        div()
            .flex()
            .items_center()
            .gap(px(4.0))    // Vue3: gap-1 = 4px
            .h(px(20.0))     // Vue3: badge-xs h-5 = 20px
            .px(px(8.0))     // Vue3: badge-xs px-2 = 8px
            .rounded(px(6.0))  // Vue3: badge rounded = 4px-8px
            .bg(bg_color)
            .child(
                // Indicator dot - Vue3: w-1.5 h-1.5 rounded-full = 6px circle
                div()
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded(px(3.0))  // Full circle (radius = half diameter)
                    .bg(indicator_color)
            )
            .child(
                div()
                    .text_color(text_color)
                    .text_size(px(12.0))  // Vue3: badge-xs text-xs = 12px
                    .child(text.to_string())
            )
    }
}

impl IntoElement for ClustersView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let theme = self.theme.clone();
        let t = self.translations.clone();
        // Build requests demo (uses build_create_request, build_update_request) - must be before moving api_client
        let build_create = self.build_create_request("NewCluster".to_string(), "localhost:9092".to_string());
        let build_update = self.build_update_request("Production".to_string(), "new-broker:9092".to_string());
        println!("Create request: {:?}", build_create);
        println!("Update request: {:?}", build_update);

        let tree_nav = self.tree_nav;
        let clusters = self.clusters;
        let selected_group = self.selected_group;
        let api_client = self.api_client.clone();
        let error = self.error.clone();

        // Call handler functions for demo (using all variants)
        Self::handle_cluster_action(ClusterAction::TestConnection, "Production");
        Self::handle_cluster_action(ClusterAction::DeleteCluster, "OldCluster");
        Self::handle_cluster_action(ClusterAction::EditCluster, "Staging");
        Self::handle_cluster_action(ClusterAction::RefreshStatus, "Production");
        Self::handle_cluster_action(ClusterAction::Disconnect, "Dev");
        Self::handle_cluster_action(ClusterAction::Reconnect, "Dev");
        Self::handle_cluster_action(ClusterAction::ViewBrokers, "Production");
        Self::handle_cluster_action(ClusterAction::ViewConsumerGroups, "Production");
        Self::handle_cluster_action(ClusterAction::CreateTopic, "Production");
        Self::handle_topics_folder_action(TopicsFolderAction::RefreshTopics, "Production");
        Self::handle_topics_folder_action(TopicsFolderAction::CreateTopic, "Production");
        Self::handle_topics_folder_action(TopicsFolderAction::ViewAllTopics, "Production");

        // Demo from_response conversions (uses from_response, from_response_with_group)
        let mock_response = ClusterResponse {
            id: 1,
            name: "Demo".to_string(),
            brokers: "localhost:9092".to_string(),
            request_timeout_ms: 30000,
            operation_timeout_ms: 30000,
            group_id: Some(1),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        };
        let mock_group = ClusterGroupResponse {
            id: 1,
            name: "Production".to_string(),
            description: Some("Prod cluster".to_string()),
            sort_order: 1,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        };
        let _display = ClusterDisplay::from_response(&mock_response);
        let _display_with_group = ClusterDisplay::from_response_with_group(&mock_response, Some(&mock_group));
        let _group_display = ClusterGroupDisplay::from_response(&mock_group);
        println!("ClusterGroupDisplay: id={}, name={}, description={:?}",
            _group_display.id, _group_display.name, _group_display.description);

        // Build create group request (uses build_create_group_request)
        let _create_group_req = Self::build_create_group_request("NewGroup".to_string(), Some("Description".to_string()));

        // Use ConnectionStatusType::as_str method
        let status_str = ConnectionStatusType::Connected.as_str();
        let status_str2 = ConnectionStatusType::Disconnected.as_str();
        let status_str3 = ConnectionStatusType::Error.as_str();
        let status_str4 = ConnectionStatusType::Unknown.as_str();
        println!("Status strings: {}, {}, {}, {}", status_str, status_str2, status_str3, status_str4);

        // Use ClusterContextMenu setter methods
        let mut ctx_menu = self.context_menu.clone();
        ctx_menu.set_testing(true);
        ctx_menu.set_refreshing(false);
        ctx_menu.set_disconnecting(true);
        ctx_menu.set_reconnecting(false);
        println!("ClusterContextMenu setters used");

        // Use ApiClient methods (base_url, path)
        println!("ApiClient base_url: {}", api_client.base_url());

        // Use ViewType::path method
        let clusters_path = crate::router::ViewType::Clusters.path();
        let topics_path = crate::router::ViewType::Topics.path();
        println!("ViewType paths: {}, {}", clusters_path, topics_path);

        // Group filter tabs
        let groups = vec![
            ("全部", None),
            ("Production", Some(1)),
            ("Dev", Some(2)),
        ];

        // Two-column layout: tree navigator + cluster cards
        div()
            .flex()
            .flex_row()
            .size_full()
            .gap(px(16.0))
            // API status info (uses api_client)
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child(format!("API客户端状态: {:?}", api_client))
            )
            // Error display (uses error field)
            .when_some(error, |this, err| {
                this.child(
                    div()
                        .text_color(theme.error)
                        .text_xs()
                        .child(format!("错误: {:?}", err))
                )
            })
            .child(
                // Left side: Tree navigator
                div()
                    .flex()
                    .flex_col()
                    .w(px(280.0))
                    .h_full()
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .border(px(1.0))
                    .border_color(theme.border)
                    .p(px(8.0))
                    .child(tree_nav)
            )
            .child(
                // Right side: Cluster management
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap(px(16.0))
                    .child(
                        // Group filter tabs
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .children(groups.iter().map(|(name, id)| {
                                let is_selected = selected_group == *id;
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(if is_selected { theme.primary } else { theme.surface_raised })
                                    .border(px(1.0))
                                    .border_color(if is_selected { theme.primary } else { theme.border })
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_color(if is_selected {
                                                Hsla::from(gpui::rgb(0xffffff))
                                            } else {
                                                theme.text_secondary
                                            })
                                            .text_sm()
                                            .child(name.to_string())
                                    )
                            }))
                    )
                    .child(
                        // Add cluster button
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(16.0))
                            .py(px(10.0))
                            .rounded(px(8.0))
                            .bg(theme.primary.opacity(0.1))
                            .border(px(1.0))
                            .border_color(theme.primary.opacity(0.3))
                            .cursor_pointer()
                            .child(
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .rounded(px(4.0))
                                    .bg(theme.primary)
                            )
                            .child(
                                div()
                                    .text_color(theme.primary)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(t.clusters.add_cluster.clone())
                            )
                    )
                    .child(
                        // Clusters grid
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(16.0))
                            .children(clusters.iter().enumerate().map(|(index, cluster)| {
                                Self::render_cluster_card(&theme, &t, cluster, index)
                            }))
                    )
            )
            // Context menu overlay (for right-click on clusters)
            .child(self.context_menu)
            // Topics folder context menu overlay
            .child(self.topics_folder_menu)
    }
}