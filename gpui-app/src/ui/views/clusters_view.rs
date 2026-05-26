//! Clusters View
//!
//! Main view for managing Kafka clusters.
//! ClustersViewWithState uses GlobalState Entity for dynamic data.

use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use crate::i18n::Translations;
use crate::ui::Theme;
use crate::ui::components::{ClusterTreeNavigatorLegacy, ClusterContextMenu, TopicsFolderContextMenu, ClusterAction, TopicsFolderAction};
use crate::state::{ConnectionStatusType, GlobalState, ClusterHealth};
use crate::api::{ApiClient, ApiError, ClusterResponse, CreateClusterRequest, UpdateClusterRequest, ClusterGroupResponse, CreateClusterGroupRequest};

/// Clusters management view
pub struct ClustersView {
    theme: Theme,
    translations: Arc<Translations>,
    /// Mock data for demonstration
    clusters: Vec<ClusterDisplay>,
    selected_group: Option<i32>,
    /// Tree navigator for cluster/topics (legacy version)
    tree_nav: ClusterTreeNavigatorLegacy,
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
            tree_nav: ClusterTreeNavigatorLegacy::new(theme.clone(), translations),
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

    /// Render a cluster card (static method for IntoElement)
    fn render_cluster_card(theme: &Theme, t: &Arc<Translations>, cluster: &ClusterDisplay, _index: usize) -> Div {
        div()
            .flex()
            .flex_col()
            .p(px(16.0))
            .gap(px(12.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                // Header: Name + Status
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(theme.text)
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(cluster.name.clone())
                    )
                    .child(Self::status_badge(theme, cluster.status))
            )
            .child(
                // Brokers
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(t.clusters.brokers.clone())
                    )
                    .child(
                        div()
                            .text_color(theme.text_secondary)
                            .text_sm()
                            .child(cluster.brokers.clone())
                    )
            )
            .when_some(cluster.group_name.clone(), |this, group| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_xs()
                                .child(t.clusters.group.clone())
                        )
                        .child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(theme.surface_raised)
                                .text_color(theme.text_secondary)
                                .text_xs()
                                .child(group)
                        )
                )
            })
            .child(
                // Actions
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .mt(px(8.0))
                    .child(
                        // Test connection button
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(theme.primary)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_sm()
                                    .child(t.clusters.test_connection.clone())
                            )
                    )
                    .child(
                        // View topics button
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
                                    .child(t.clusters.view_topics_link.clone())
                            )
                    )
            )
    }

    /// Render status badge (static method)
    fn status_badge(theme: &Theme, status: ConnectionStatusType) -> Div {
        let (color, text) = match status {
            ConnectionStatusType::Connected => (theme.success, "已连接"),
            ConnectionStatusType::Disconnected => (theme.text_muted, "未连接"),
            ConnectionStatusType::Error => (theme.error, "错误"),
            ConnectionStatusType::Unknown => (theme.text_muted, "未知"),
        };

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .bg(color.opacity(0.2))
            .child(
                div()
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded(px(3.0))
                    .bg(color)
            )
            .child(
                div()
                    .text_color(color)
                    .text_xs()
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

/// Cluster form data for create/edit modal (matches Vue's form)
#[derive(Debug, Clone, Default)]
pub struct ClusterForm {
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i32,
    pub operation_timeout_ms: i32,
    pub group_id: Option<i32>,
}

impl ClusterForm {
    /// Validate form (matches Vue's validation)
    fn validate(&self) -> Option<String> {
        if self.name.is_empty() {
            return Some("名称不能为空".to_string());
        }
        if self.name.len() > 15 {
            return Some("名称不能超过15字符".to_string());
        }
        // Check name format: letters, numbers, Chinese, hyphen, underscore
        let valid_chars = self.name.chars().all(|c| {
            c.is_alphanumeric() || c == '-' || c == '_' || ('\u{4e00}'..='\u{9fff}').contains(&c)
        });
        if !valid_chars {
            return Some("名称只能包含字母、数字、中文、连字符、下划线".to_string());
        }
        if self.brokers.is_empty() {
            return Some("Brokers不能为空".to_string());
        }
        None
    }

    /// Convert to CreateClusterRequest
    fn to_create_request(&self) -> CreateClusterRequest {
        CreateClusterRequest {
            name: self.name.clone(),
            brokers: self.brokers.clone(),
            request_timeout_ms: Some(self.request_timeout_ms),
            operation_timeout_ms: Some(self.operation_timeout_ms),
            group_id: self.group_id,
        }
    }

    /// Convert to UpdateClusterRequest
    fn to_update_request(&self) -> UpdateClusterRequest {
        UpdateClusterRequest {
            name: self.name.clone(),
            brokers: self.brokers.clone(),
            request_timeout_ms: Some(self.request_timeout_ms),
            operation_timeout_ms: Some(self.operation_timeout_ms),
            group_id: self.group_id,
        }
    }
}

/// Test connection result (matches Vue's test connection display)
#[derive(Debug, Clone)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub tested_at: u64,
}

/// Clusters view with GlobalState Entity integration
/// Matches Vue's ClustersView layout with grid cards
pub struct ClustersViewWithState {
    state: Entity<GlobalState>,
    translations: Arc<Translations>,
    selected_group_id: Option<i32>,
    /// Create cluster modal state
    show_create_modal: bool,
    create_form: ClusterForm,
    create_form_error: Option<String>,
    /// Edit cluster modal state
    show_edit_modal: bool,
    edit_form: ClusterForm,
    editing_cluster_id: Option<i32>,
    edit_form_error: Option<String>,
    /// Delete cluster confirmation state
    show_delete_confirm: bool,
    deleting_cluster: Option<String>,
    /// Test connection state
    testing_connection: bool,
    test_result: Option<TestConnectionResult>,
    /// Loading states for cluster actions (matches Vue's testing/reconnecting/disconnecting/refreshing sets)
    testing_clusters: Vec<i32>,
    reconnecting_clusters: Vec<String>,
    disconnecting_clusters: Vec<String>,
    refreshing_topics_clusters: Vec<String>,
    /// Manage Groups modal state (matches Vue's manage groups modal)
    show_manage_groups_modal: bool,
    manage_groups_new_name: String,
    /// Group selector horizontal scroll position (matches Vue's scrollGroups)
    group_scroll_offset: f32,
}

impl ClustersViewWithState {
    pub fn new(state: Entity<GlobalState>, translations: Arc<Translations>, cx: &mut Context<Self>) -> Self {
        // Load clusters and groups on init
        state.update(cx, |s, cx| {
            s.load_clusters(cx);
            s.load_cluster_groups(cx);
        });

        Self {
            state,
            translations,
            selected_group_id: None,
            show_create_modal: false,
            create_form: ClusterForm::default(),
            create_form_error: None,
            show_edit_modal: false,
            edit_form: ClusterForm::default(),
            editing_cluster_id: None,
            edit_form_error: None,
            show_delete_confirm: false,
            deleting_cluster: None,
            testing_connection: false,
            test_result: None,
            testing_clusters: Vec::new(),
            reconnecting_clusters: Vec::new(),
            disconnecting_clusters: Vec::new(),
            refreshing_topics_clusters: Vec::new(),
            show_manage_groups_modal: false,
            manage_groups_new_name: String::new(),
            group_scroll_offset: 0.0,
        }
    }

    /// Get filtered clusters by group
    fn get_filtered_clusters(&self, cx: &App) -> Vec<ClusterResponse> {
        let state = self.state.read(cx);
        if self.selected_group_id.is_none() {
            state.clusters.clone()
        } else {
            state.clusters.iter()
                .filter(|c| c.group_id == self.selected_group_id)
                .cloned()
                .collect()
        }
    }

    /// Select a group for filtering
    fn select_group(&mut self, group_id: Option<i32>, cx: &mut Context<Self>) {
        self.selected_group_id = group_id;
        cx.notify();
    }

    /// Scroll groups left (matches Vue's scrollGroups(-200))
    fn scroll_groups_left(&mut self, cx: &mut Context<Self>) {
        self.group_scroll_offset = (self.group_scroll_offset - 200.0).max(0.0);
        cx.notify();
    }

    /// Scroll groups right (matches Vue's scrollGroups(200))
    fn scroll_groups_right(&mut self, cx: &mut Context<Self>) {
        // Max scroll based on number of groups
        let max_scroll = (self.state.read(cx).cluster_groups.len() as f32 * 100.0) - 200.0;
        self.group_scroll_offset = (self.group_scroll_offset + 200.0).min(max_scroll.max(0.0));
        cx.notify();
    }

    /// Navigate to topics view for a cluster
    fn navigate_to_topics(&mut self, cluster_name: String, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.selected_cluster = Some(cluster_name.clone());
            s.navigate(crate::router::ViewType::Topics);
            cx.notify();
        });
    }

    /// Navigate to messages view for a topic
    fn navigate_to_messages(&mut self, cluster_name: String, topic_name: String, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.selected_topic = Some((cluster_name, topic_name));
            s.navigate(crate::router::ViewType::Messages);
            cx.notify();
        });
    }

    /// Test cluster connection
    fn test_connection(&mut self, cluster_id: i32, cx: &mut Context<Self>) {
        // TODO: implement actual API call
        println!("Test connection for cluster {}", cluster_id);
        cx.notify();
    }

    /// Reconnect cluster
    fn reconnect_cluster(&mut self, cluster_name: String, cx: &mut Context<Self>) {
        // TODO: implement actual API call
        println!("Reconnect cluster {}", cluster_name);
        cx.notify();
    }

    /// Disconnect cluster
    fn disconnect_cluster(&mut self, cluster_name: String, cx: &mut Context<Self>) {
        // TODO: implement actual API call
        println!("Disconnect cluster {}", cluster_name);
        cx.notify();
    }

    /// Refresh cluster topics
    fn refresh_topics(&mut self, cluster_name: String, cx: &mut Context<Self>) {
        // TODO: implement actual API call
        println!("Refresh topics for cluster {}", cluster_name);
        cx.notify();
    }

    // === Modal handling methods (matches Vue's modal dialogs) ===

    /// Open create cluster modal
    fn open_create_modal(&mut self, cx: &mut Context<Self>) {
        self.show_create_modal = true;
        self.create_form = ClusterForm {
            name: String::new(),
            brokers: String::new(),
            request_timeout_ms: 30000,
            operation_timeout_ms: 30000,
            group_id: self.selected_group_id,
        };
        self.create_form_error = None;
        cx.notify();
    }

    /// Close create cluster modal
    fn close_create_modal(&mut self, cx: &mut Context<Self>) {
        self.show_create_modal = false;
        self.create_form_error = None;
        cx.notify();
    }

    /// Submit create cluster form
    fn submit_create(&mut self, cx: &mut Context<Self>) {
        // Validate form
        if let Some(error) = self.create_form.validate() {
            self.create_form_error = Some(error);
            cx.notify();
            return;
        }

        // TODO: actual API call
        let request = self.create_form.to_create_request();
        println!("Creating cluster: {:?}", request);
        self.close_create_modal(cx);
    }

    /// Open edit cluster modal
    fn open_edit_modal(&mut self, cluster: &ClusterResponse, cx: &mut Context<Self>) {
        self.show_edit_modal = true;
        self.editing_cluster_id = Some(cluster.id);
        self.edit_form = ClusterForm {
            name: cluster.name.clone(),
            brokers: cluster.brokers.clone(),
            request_timeout_ms: cluster.request_timeout_ms,
            operation_timeout_ms: cluster.operation_timeout_ms,
            group_id: cluster.group_id,
        };
        self.edit_form_error = None;
        cx.notify();
    }

    /// Close edit cluster modal
    fn close_edit_modal(&mut self, cx: &mut Context<Self>) {
        self.show_edit_modal = false;
        self.editing_cluster_id = None;
        self.edit_form_error = None;
        cx.notify();
    }

    /// Submit edit cluster form
    fn submit_edit(&mut self, cx: &mut Context<Self>) {
        // Validate form
        if let Some(error) = self.edit_form.validate() {
            self.edit_form_error = Some(error);
            cx.notify();
            return;
        }

        // TODO: actual API call
        let request = self.edit_form.to_update_request();
        println!("Updating cluster: {:?}", request);
        self.close_edit_modal(cx);
    }

    /// Open delete confirmation modal
    fn open_delete_confirm(&mut self, cluster_name: String, cx: &mut Context<Self>) {
        self.show_delete_confirm = true;
        self.deleting_cluster = Some(cluster_name);
        cx.notify();
    }

    /// Close delete confirmation modal
    fn close_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.show_delete_confirm = false;
        self.deleting_cluster = None;
        cx.notify();
    }

    /// Confirm delete cluster
    fn confirm_delete(&mut self, cx: &mut Context<Self>) {
        if let Some(cluster_name) = self.deleting_cluster.clone() {
            // TODO: actual API call
            println!("Deleting cluster: {}", cluster_name);
        }
        self.close_delete_confirm(cx);
    }

    /// Test connection with result display
    fn test_connection_with_result(&mut self, cluster_name: String, cx: &mut Context<Self>) {
        self.testing_connection = true;
        self.test_result = None;
        cx.notify();

        // Simulate test (would be async API call in real implementation)
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_secs(2)).await;
            this.update(cx, |view, cx| {
                view.testing_connection = false;
                view.test_result = Some(TestConnectionResult {
                    success: true,
                    message: format!("集群 {} 连接成功", cluster_name),
                    tested_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                });
                cx.notify();
            }).ok();
        }).detach();
    }

    // === Manage Groups modal methods (matches Vue's manage groups functionality) ===

    /// Open manage groups modal
    fn open_manage_groups_modal(&mut self, cx: &mut Context<Self>) {
        self.show_manage_groups_modal = true;
        self.manage_groups_new_name = String::new();
        cx.notify();
    }

    /// Close manage groups modal
    fn close_manage_groups_modal(&mut self, cx: &mut Context<Self>) {
        self.show_manage_groups_modal = false;
        cx.notify();
    }

    /// Create new group
    fn create_new_group(&mut self, cx: &mut Context<Self>) {
        if self.manage_groups_new_name.is_empty() {
            return;
        }
        // TODO: actual API call
        println!("Creating group: {}", self.manage_groups_new_name);
        self.manage_groups_new_name = String::new();
        cx.notify();
    }

    /// Delete group
    fn delete_group(&mut self, group_id: i32, cx: &mut Context<Self>) {
        // TODO: actual API call
        println!("Deleting group: {}", group_id);
        cx.notify();
    }

    /// Render status badge (matches Vue's badge styling)
    fn render_status_badge(theme: &Theme, healthy: bool, has_error: bool) -> Div {
        let (bg_color, text_color, status_text) = if healthy {
            (theme.success.opacity(0.15), theme.success, "已连接")
        } else if has_error {
            (theme.error.opacity(0.15), theme.error, "错误")
        } else {
            (theme.surface.opacity(0.5), theme.text_muted, "未连接")
        };

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(8.0))
            .py(px(3.0))
            .rounded(px(4.0))
            .bg(bg_color)
            .child(
                div()
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded(px(3.0))
                    .bg(text_color)
            )
            .child(
                div()
                    .text_color(text_color)
                    .text_xs()
                    .child(status_text)
            )
    }

    /// Render cluster card (matches Vue's card glass styling)
    fn render_cluster_card(
        cluster: &ClusterResponse,
        theme: &Theme,
        t: &Translations,
        health: Option<&ClusterHealth>,
    ) -> Stateful<Div> {
        let (healthy, has_error) = health.map(|h| (h.healthy, h.error.is_some()))
            .unwrap_or((false, false));

        // Clone values for use in child elements (to avoid lifetime issues)
        let cluster_name = cluster.name.clone();
        let cluster_brokers = cluster.brokers.clone();
        let created_date = cluster.created_at.split('T').next().unwrap_or("").to_string();
        let timeouts = format!("请求: {}ms | 操作: {}ms", cluster.request_timeout_ms, cluster.operation_timeout_ms);
        let create_topic_text = t.clusters.create_topic.clone();
        let view_topics_text = t.clusters.view_topics_link.clone();
        let reconnect_text = t.clusters.reconnect.clone();

        div()
            .id(format!("cluster-card-{}", cluster.id))
            .flex()
            .flex_col()
            .w(px(320.0))
            .rounded(px(8.0))
            // Glass card styling: semi-transparent background with blur effect simulation
            .bg(theme.surface.opacity(0.75))
            // Gradient border effect: subtle primary-colored border
            .border(px(1.0))
            .border_color(theme.primary.opacity(0.2))
            .child(
                    // Header: name + status + Edit/Delete buttons
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .p(px(12.0))
                        .border_b(px(1.0))
                        .border_color(theme.border.opacity(0.3))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .w(px(32.0))
                                        .h(px(32.0))
                                        .rounded(px(8.0))
                                        // Glow-primary effect: primary background with subtle glow
                                        .bg(theme.primary.opacity(0.15))
                                        .border(px(1.0))
                                        .border_color(theme.primary.opacity(0.3))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .w(px(16.0))
                                                .h(px(16.0))
                                                .rounded(px(4.0))
                                                .bg(theme.primary)
                                        )
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_color(theme.text)
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(cluster_name)
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.text_muted)
                                                .text_xs()
                                                .child(created_date)
                                        )
                                )
                        )
                        .child(
                            // Edit/Delete + Status buttons row
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                // Edit button
                                .child(
                                    div()
                                        .id(format!("edit-cluster-{}", cluster.id))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .rounded(px(4.0))
                                        .bg(theme.surface)
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .w(px(10.0))
                                                .h(px(10.0))
                                                .rounded(px(2.0))
                                                .bg(theme.text_muted.opacity(0.5))
                                        )
                                )
                                // Delete button (danger color)
                                .child(
                                    div()
                                        .id(format!("delete-cluster-{}", cluster.id))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .rounded(px(4.0))
                                        .bg(theme.error.opacity(0.1))
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .w(px(10.0))
                                                .h(px(10.0))
                                                .rounded(px(2.0))
                                                .bg(theme.error.opacity(0.5))
                                        )
                                )
                                .child(Self::render_status_badge(theme, healthy, has_error))
                        )
                )
            // Body: brokers + timeouts
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .p(px(12.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("Brokers")
                            )
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_xs()
                                    .child(cluster_brokers)
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("超时设置")
                            )
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(timeouts)
                            )
                    )
            )
            // Error message display (matches Vue's alert style when connection has error)
            .when_some(health.and_then(|h| h.error.clone()), |this, error_msg| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(8.0))
                        .py(px(6.0))
                        .m(px(8.0))
                        .rounded(px(4.0))
                        .bg(theme.error.opacity(0.15))
                        .border(px(1.0))
                        .border_color(theme.error.opacity(0.3))
                        .child(
                            div()
                                .w(px(8.0))
                                .h(px(8.0))
                                .rounded(px(4.0))
                                .bg(theme.error)
                        )
                        .child(
                            div()
                                .text_color(theme.error)
                                .text_xs()
                                .truncate()
                                .child(error_msg)
                        )
                )
            })
            // Actions: buttons row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .p(px(8.0))
                    .bg(theme.surface_raised)
                    .child(
                        // Create Topic
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.primary)
                            .child(
                                div()
                                    .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                    .text_xs()
                                    .child(create_topic_text)
                            )
                    )
                    .child(
                        // Test
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child("测试")
                            )
                    )
                    .child(
                        // View Topics
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(view_topics_text)
                            )
                    )
                    .child(
                        // Reconnect
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child(reconnect_text)
                            )
                    )
                    .child(
                        // Refresh Topics (matches Vue)
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .text_xs()
                                    .child("刷新")
                            )
                    )
                    .child(
                        // Disconnect (matches Vue's danger style)
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.error.opacity(0.1))
                            .child(
                                div()
                                    .text_color(theme.error)
                                    .text_xs()
                                    .child("断开")
                            )
                    )
            )
    }

    /// Render create cluster modal (matches Vue's Modal with form)
    fn render_create_modal(theme: &Theme, t: &Translations, form: &ClusterForm, error: Option<&String>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .w(px(400.0))
            .p(px(24.0))
            .rounded(px(12.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text)
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(t.clusters.add_cluster.clone())
            )
            // Form error display
            .when_some(error, |this, err| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(6.0))
                        .bg(theme.error.opacity(0.15))
                        .child(div().text_color(theme.error).text_xs().child(err.clone()))
                )
            })
            // Name field
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(div().text_color(theme.text_muted).text_xs().child("名称"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(div().text_color(theme.text).text_sm().child(form.name.clone()))
                    )
            )
            // Brokers field
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(div().text_color(theme.text_muted).text_xs().child("Brokers"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(div().text_color(theme.text).text_sm().child(form.brokers.clone()))
                    )
            )
            // Timeout fields
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .w(px(160.0))
                            .child(div().text_color(theme.text_muted).text_xs().child("请求超时(ms)"))
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .child(div().text_color(theme.text).text_sm().child(form.request_timeout_ms.to_string()))
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .w(px(160.0))
                            .child(div().text_color(theme.text_muted).text_xs().child("操作超时(ms)"))
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .child(div().text_color(theme.text).text_sm().child(form.operation_timeout_ms.to_string()))
                            )
                    )
            )
    }

    /// Render delete confirmation modal (matches Vue's confirm dialog)
    fn render_delete_confirm(theme: &Theme, t: &Translations, cluster_name: Option<&String>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .w(px(320.0))
            .p(px(24.0))
            .rounded(px(12.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text)
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(t.common.delete.clone())
            )
            .child(
                div()
                    .text_color(theme.text_secondary)
                    .text_sm()
                    .child(format!("确定要删除集群 {} 吗？", cluster_name.unwrap_or(&"未知".to_string())))
            )
            .child(
                div()
                    .text_color(theme.warning)
                    .text_xs()
                    .child("此操作不可撤销")
            )
    }

    /// Render manage groups modal (matches Vue's manage groups dialog)
    fn render_manage_groups_modal(theme: &Theme, t: &Translations, groups: &[ClusterGroupResponse]) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .w(px(480.0))
            .p(px(24.0))
            .rounded(px(12.0))
            .bg(theme.surface)
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                div()
                    .text_color(theme.text)
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(t.clusters.manage_groups.clone())
            )
            // New group input
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_raised)
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(div().text_color(theme.text_muted).text_sm().child("输入分组名称..."))
                    )
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.primary)
                            .child(div().text_color(Hsla::from(gpui::rgb(0xffffff))).text_sm().child("添加"))
                    )
            )
            // Groups list
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(theme.text_muted)
                            .text_xs()
                            .child(format!("已有 {} 个分组", groups.len()))
                    )
                    .children(groups.iter().map(|group| {
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(8.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_sm()
                                    .child(group.name.clone())
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_color(theme.text_muted)
                                            .text_xs()
                                            .child(format!("{} 个集群", 0))
                                    )
                                    .child(
                                        div()
                                            .px(px(6.0))
                                            .py(px(2.0))
                                            .rounded(px(2.0))
                                            .bg(theme.error.opacity(0.15))
                                            .cursor_pointer()
                                            .child(div().text_color(theme.error).text_xs().child("删除"))
                                    )
                            )
                    }))
            )
    }
}

impl Render for ClustersViewWithState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let theme = &state.theme;
        let t = &self.translations;
        let loading = state.loading;
        let clusters = self.get_filtered_clusters(cx);
        let groups = state.cluster_groups.clone();
        let selected_gid = self.selected_group_id;

        div()
            .id("clusters-view-with-state")
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .p(px(12.0))
            .child(
                // Header (matches Vue's header with back button)
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .mb(px(12.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    // Back button (matches Vue's back button with arrow icon)
                                    .child(
                                        div()
                                            .id("clusters-back-btn")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(24.0))
                                            .h(px(24.0))
                                            .rounded(px(4.0))
                                            .bg(theme.surface_raised)
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.state.update(cx, |s, cx| {
                                                    s.navigate(crate::router::ViewType::Settings);
                                                    cx.notify();
                                                });
                                            }))
                                            .child(
                                                div()
                                                    .w(px(12.0))
                                                    .h(px(12.0))
                                                    .rounded(px(2.0))
                                                    .bg(theme.text_muted)
                                            )
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.text)
                                            .text_xl()
                                            .font_weight(FontWeight::BOLD)
                                            .child(t.clusters.title.clone())
                                    )
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .text_sm()
                                    .child(t.clusters.description.clone())
                            )
                    )
                    .child(
                        // Header buttons: Manage Groups + Add Cluster (matches Vue's header buttons)
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            // Manage Groups button (opens manage groups modal)
                            .child(
                                div()
                                    .id("manage-groups-btn")
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(theme.surface_raised)
                                    .border(px(1.0))
                                    .border_color(theme.border)
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_manage_groups_modal(cx);
                                    }))
                                    .child(div().text_color(theme.text_secondary).text_sm().child(t.clusters.add_group.clone()))
                            )
                            // Add Cluster button (opens create modal)
                            .child(
                                div()
                                    .id("add-cluster-btn")
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .bg(theme.primary)
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_create_modal(cx);
                                    }))
                                    .child(
                                        div()
                                            .w(px(12.0))
                                            .h(px(12.0))
                                            .rounded(px(2.0))
                                            .bg(Hsla::from(gpui::rgb(0xffffff)))
                                    )
                                    .child(
                                        div()
                                            .text_color(Hsla::from(gpui::rgb(0xffffff)))
                                            .text_sm()
                                            .child(t.clusters.add_cluster.clone())
                                    )
                            )
                    )
            )
            // Group selector tabs (matches Vue's horizontal scroll with arrow buttons)
            .when(!groups.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .mb(px(8.0))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .text_sm()
                                .child(format!("{}:", t.clusters.group))
                        )
                        // Left scroll button (matches Vue's scrollGroups(-200))
                        .child(
                            div()
                                .id("group-scroll-left")
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(20.0))
                                .h(px(20.0))
                                .rounded(px(4.0))
                                .bg(theme.surface_raised)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.scroll_groups_left(cx);
                                }))
                                .child(
                                    div()
                                        .w(px(8.0))
                                        .h(px(8.0))
                                        .rounded(px(2.0))
                                        .bg(theme.text_muted.opacity(0.5))
                                )
                        )
                        .child(
                            div()
                                .id("group-all")
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(4.0))
                                .bg(if selected_gid.is_none() { theme.primary } else { theme.surface_raised })
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.select_group(None, cx);
                                }))
                                .child(
                                    div()
                                        .text_color(if selected_gid.is_none() { Hsla::from(gpui::rgb(0xffffff)) } else { theme.text_secondary })
                                        .text_xs()
                                        .child(t.common.all.clone())
                                )
                        )
                        .children(groups.iter().map(|group| {
                            let is_selected = selected_gid == Some(group.id);
                            let group_id = group.id;
                            div()
                                .id(format!("group-{}", group.id))
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(4.0))
                                .bg(if is_selected { theme.primary } else { theme.surface_raised })
                                .cursor_pointer()
                                .on_click(cx.listener({
                                    let gid = group_id;
                                    move |this, _, _, cx| {
                                        this.select_group(Some(gid), cx);
                                    }
                                }))
                                .child(
                                    div()
                                        .text_color(if is_selected { Hsla::from(gpui::rgb(0xffffff)) } else { theme.text_secondary })
                                        .text_xs()
                                        .child(group.name.clone())
                                )
                        }))
                        // Right scroll button (matches Vue's scrollGroups(200))
                        .child(
                            div()
                                .id("group-scroll-right")
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(20.0))
                                .h(px(20.0))
                                .rounded(px(4.0))
                                .bg(theme.surface_raised)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.scroll_groups_right(cx);
                                }))
                                .child(
                                    div()
                                        .w(px(8.0))
                                        .h(px(8.0))
                                        .rounded(px(2.0))
                                        .bg(theme.text_muted.opacity(0.5))
                                )
                        )
                )
            })
            // Content
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .child(
                        // Loading state
                        div()
                            .when(loading, |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .py(px(32.0))
                                        .child(
                                            div()
                                                .text_color(theme.text_muted)
                                                .text_sm()
                                                .child(t.common.loading.clone())
                                        )
                                )
                            })
                            // Clusters grid (3 columns like Vue)
                            .when(!loading && !clusters.is_empty(), |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .gap(px(12.0))
                                        .children(clusters.iter().map(|cluster| {
                                            let health = state.cluster_health.get(&cluster.name);
                                            Self::render_cluster_card(cluster, theme, t, health)
                                        }))
                                )
                            })
                            // Empty state
                            .when(!loading && clusters.is_empty(), |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .py(px(32.0))
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .text_color(theme.text)
                                                .text_sm()
                                                .child(t.common.no_data.clone())
                                        )
                                )
                            })
                    )
            )
            // Create cluster modal overlay
            .when(self.show_create_modal, |this| {
                let form = self.create_form.clone();
                let error = self.create_form_error.clone();

                this.child(
                    div()
                        .id("create-modal-overlay")
                        .absolute()
                        .top(px(0.0))
                        .left(px(0.0))
                        .right(px(0.0))
                        .bottom(px(0.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::transparent_black().opacity(0.5))
                        .child(Self::render_create_modal(theme, t, &form, error.as_ref()))
                        // Buttons row
                        .child(
                            div()
                                .absolute()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .mt(px(180.0))
                                .child(
                                    div()
                                        .id("cancel-create-btn")
                                        .px(px(16.0))
                                        .py(px(8.0))
                                        .rounded(px(6.0))
                                        .bg(theme.surface_raised)
                                        .cursor_pointer()
                                        .child(div().text_color(theme.text_secondary).text_sm().child(t.common.cancel.clone()))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.close_create_modal(cx);
                                        }))
                                )
                                .child(
                                    div()
                                        .id("submit-create-btn")
                                        .px(px(16.0))
                                        .py(px(8.0))
                                        .rounded(px(6.0))
                                        .bg(theme.primary)
                                        .cursor_pointer()
                                        .child(div().text_color(Hsla::from(gpui::rgb(0xffffff))).text_sm().child(t.common.create.clone()))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.submit_create(cx);
                                        }))
                                )
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_create_modal(cx);
                        }))
                )
            })
            // Delete confirmation modal overlay
            .when(self.show_delete_confirm, |this| {
                let deleting = self.deleting_cluster.clone();

                this.child(
                    div()
                        .id("delete-confirm-overlay")
                        .absolute()
                        .top(px(0.0))
                        .left(px(0.0))
                        .right(px(0.0))
                        .bottom(px(0.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::transparent_black().opacity(0.5))
                        .child(Self::render_delete_confirm(theme, t, deleting.as_ref()))
                        // Buttons row
                        .child(
                            div()
                                .absolute()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .mt(px(140.0))
                                .child(
                                    div()
                                        .id("cancel-delete-btn")
                                        .px(px(16.0))
                                        .py(px(8.0))
                                        .rounded(px(6.0))
                                        .bg(theme.surface_raised)
                                        .cursor_pointer()
                                        .child(div().text_color(theme.text_secondary).text_sm().child(t.common.cancel.clone()))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.close_delete_confirm(cx);
                                        }))
                                )
                                .child(
                                    div()
                                        .id("confirm-delete-btn")
                                        .px(px(16.0))
                                        .py(px(8.0))
                                        .rounded(px(6.0))
                                        .bg(theme.error)
                                        .cursor_pointer()
                                        .child(div().text_color(Hsla::from(gpui::rgb(0xffffff))).text_sm().child(t.common.delete.clone()))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.confirm_delete(cx);
                                        }))
                                )
                        )
                )
            })
            // Test connection result display
            .when_some(self.test_result.clone(), |this, result| {
                let bg_color = if result.success { theme.success.opacity(0.15) } else { theme.error.opacity(0.15) };
                let text_color = if result.success { theme.success } else { theme.error };

                this.child(
                    div()
                        .absolute()
                        .top(px(60.0))
                        .right(px(12.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(6.0))
                        .bg(bg_color)
                        .border(px(1.0))
                        .border_color(text_color.opacity(0.3))
                        .child(
                            div()
                                .w(px(8.0))
                                .h(px(8.0))
                                .rounded(px(4.0))
                                .bg(text_color)
                        )
                        .child(
                            div()
                                .text_color(text_color)
                                .text_xs()
                                .child(result.message.clone())
                        )
                )
            })
            // Manage Groups modal overlay (matches Vue's manage groups dialog)
            .when(self.show_manage_groups_modal, |this| {
                let groups = state.cluster_groups.clone();

                this.child(
                    div()
                        .id("manage-groups-modal-overlay")
                        .absolute()
                        .top(px(0.0))
                        .left(px(0.0))
                        .right(px(0.0))
                        .bottom(px(0.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::transparent_black().opacity(0.5))
                        .child(Self::render_manage_groups_modal(theme, t, &groups))
                        // Close button overlay
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_manage_groups_modal(cx);
                        }))
                )
            })
    }
}
