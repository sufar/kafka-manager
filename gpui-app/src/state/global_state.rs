//! Global App State
//!
//! Centralized state management using GPUI Entity pattern.

use gpui::*;
use std::sync::Arc;
use std::collections::HashMap;

use crate::router::{Router, ViewType};
use crate::state::{AppState, MessageBuffer, MessageBufferConfig, BufferedMessage, ConnectionStatusType};
use crate::ui::theme::Theme;
use crate::i18n::Translations;
use crate::api::{ApiClient, ClusterApi, ClusterResponse, ClusterGroupResponse};

/// Toast notification message
#[derive(Debug, Clone)]
pub struct ToastMessage {
    pub id: usize,
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: u64,
    pub duration_ms: u64,
}

/// Toast type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

/// Global application state entity
pub struct GlobalState {
    /// API client for backend communication
    pub api_client: ApiClient,
    /// Router for navigation
    pub router: Router,
    /// Application data state
    pub app_state: AppState,
    /// Message buffer for query results
    pub message_buffer: MessageBuffer<BufferedMessage>,
    /// Current theme
    pub theme: Theme,
    /// Current language
    pub language: Language,
    /// Sidebar visibility (mobile)
    pub sidebar_open: bool,
    /// Sidebar width
    pub sidebar_width: Pixels,
    /// Is mobile layout
    pub is_mobile: bool,
    /// Tour active
    pub tour_active: bool,
    /// Tour current step
    pub tour_step: usize,
    /// Sidebar mode: tree or flat
    pub sidebar_mode: SidebarMode,
    /// Clusters loaded from backend
    pub clusters: Vec<ClusterResponse>,
    /// Cluster groups
    pub cluster_groups: Vec<ClusterGroupResponse>,
    /// Cluster health status
    pub cluster_health: HashMap<String, ClusterHealth>,
    /// Topics per cluster
    pub cluster_topics: HashMap<String, Vec<String>>,
    /// Consumer groups per cluster
    pub cluster_consumer_groups: HashMap<String, Vec<String>>,
    /// Selected cluster for navigation
    pub selected_cluster: Option<String>,
    /// Selected topic for navigation
    pub selected_topic: Option<(String, String)>, // (cluster, topic)
    /// Selected consumer group for navigation
    pub selected_consumer_group: Option<(String, String)>, // (cluster, group)
    /// Query partition filter for messages view
    pub query_partition: Option<i32>,
    /// Loading state
    pub loading: bool,
    /// Error message
    pub error: Option<String>,
    /// Toast messages
    pub toast_messages: Vec<ToastMessage>,
    /// Toast ID counter
    pub toast_counter: usize,
}

/// Cluster health status
#[derive(Debug, Clone)]
pub struct ClusterHealth {
    pub cluster_id: String,
    pub healthy: bool,
    pub last_checked: u64,
    pub error: Option<String>,
}

/// Sidebar display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarMode {
    #[default]
    Flat,
    Tree,
}

/// Language options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Chinese,
    English,
}

impl GlobalState {
    /// Create new global state
    pub fn new() -> Self {
        Self {
            api_client: ApiClient::default_client(),
            router: Router::new(),
            app_state: AppState::default(),
            message_buffer: MessageBuffer::new(),
            theme: Theme::dark(),
            language: Language::Chinese,
            sidebar_open: false,
            sidebar_width: px(280.0),
            is_mobile: false,
            tour_active: false,
            tour_step: 0,
            sidebar_mode: SidebarMode::Tree,
            clusters: Vec::new(),
            cluster_groups: Vec::new(),
            cluster_health: HashMap::new(),
            cluster_topics: HashMap::new(),
            cluster_consumer_groups: HashMap::new(),
            selected_cluster: None,
            selected_topic: None,
            selected_consumer_group: None,
            query_partition: None,
            loading: false,
            error: None,
            toast_messages: Vec::new(),
            toast_counter: 0,
        }
    }

    /// Create state with custom config
    pub fn with_config(config: MessageBufferConfig) -> Self {
        Self {
            api_client: ApiClient::default_client(),
            router: Router::new(),
            app_state: AppState::default(),
            message_buffer: MessageBuffer::with_config(config),
            theme: Theme::dark(),
            language: Language::Chinese,
            sidebar_open: false,
            sidebar_width: px(280.0),
            is_mobile: false,
            tour_active: false,
            tour_step: 0,
            sidebar_mode: SidebarMode::Tree,
            clusters: Vec::new(),
            cluster_groups: Vec::new(),
            cluster_health: HashMap::new(),
            cluster_topics: HashMap::new(),
            cluster_consumer_groups: HashMap::new(),
            selected_cluster: None,
            selected_topic: None,
            selected_consumer_group: None,
            query_partition: None,
            loading: false,
            error: None,
            toast_messages: Vec::new(),
            toast_counter: 0,
        }
    }

    /// Load clusters from backend (async via spawn)
    pub fn load_clusters(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        cx.notify();

        let client = self.api_client.clone();
        cx.spawn(async move |this, cx| {
            let cluster_api = ClusterApi::new(client);
            match cluster_api.list().await {
                Ok(clusters) => {
                    this.update(cx, |state, cx| {
                        state.clusters = clusters;
                        state.loading = false;
                        state.error = None;
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    this.update(cx, |state, cx| {
                        state.loading = false;
                        state.error = Some(e.to_string());
                        cx.notify();
                    }).ok();
                }
            }
        }).detach();
    }

    /// Load cluster groups from backend
    pub fn load_cluster_groups(&mut self, cx: &mut Context<Self>) {
        let client = self.api_client.clone();
        cx.spawn(async move |this, cx| {
            let cluster_api = ClusterApi::new(client);
            match cluster_api.list_groups().await {
                Ok(groups) => {
                    this.update(cx, |state, cx| {
                        state.cluster_groups = groups;
                        cx.notify();
                    }).ok();
                }
                Err(_) => {}
            }
        }).detach();
    }

    /// Check cluster health
    pub fn check_cluster_health(&mut self, cluster_name: &str, cx: &mut Context<Self>) {
        let client = self.api_client.clone();
        let cluster = cluster_name.to_string();
        cx.spawn(async move |this, cx| {
            match client.get::<serde_json::Value>(&format!("/api/clusters/{}/health", cluster)).await {
                Ok(health) => {
                    let healthy = health.get("healthy").and_then(|v| v.as_bool()).unwrap_or(false);
                    let error_msg = health.get("error_message").and_then(|v| v.as_str()).map(|s| s.to_string());
                    this.update(cx, |state, cx| {
                        state.cluster_health.insert(cluster.clone(), ClusterHealth {
                            cluster_id: cluster.clone(),
                            healthy,
                            last_checked: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            error: error_msg,
                        });
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    this.update(cx, |state, cx| {
                        state.cluster_health.insert(cluster.clone(), ClusterHealth {
                            cluster_id: cluster.clone(),
                            healthy: false,
                            last_checked: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            error: Some(e.to_string()),
                        });
                        cx.notify();
                    }).ok();
                }
            }
        }).detach();
    }

    /// Load topics for a cluster
    pub fn load_cluster_topics(&mut self, cluster_name: &str, cx: &mut Context<Self>) {
        let client = self.api_client.clone();
        let cluster = cluster_name.to_string();
        cx.spawn(async move |this, cx| {
            match client.get::<Vec<String>>(&format!("/api/clusters/{}/topics/saved", cluster)).await {
                Ok(topics) => {
                    this.update(cx, |state, cx| {
                        state.cluster_topics.insert(cluster.clone(), topics);
                        cx.notify();
                    }).ok();
                }
                Err(_) => {
                    match client.get::<Vec<serde_json::Value>>(&format!("/api/clusters/{}/topics", cluster)).await {
                        Ok(topics) => {
                            let topic_names: Vec<String> = topics.iter()
                                .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                                .collect();
                            this.update(cx, |state, cx| {
                                state.cluster_topics.insert(cluster.clone(), topic_names);
                                cx.notify();
                            }).ok();
                        }
                        Err(_) => {}
                    }
                }
            }
        }).detach();
    }

    /// Load consumer groups for a cluster
    pub fn load_cluster_consumer_groups(&mut self, cluster_name: &str, cx: &mut Context<Self>) {
        let client = self.api_client.clone();
        let cluster = cluster_name.to_string();
        cx.spawn(async move |this, cx| {
            match client.get::<Vec<String>>(&format!("/api/clusters/{}/consumer-groups/saved", cluster)).await {
                Ok(groups) => {
                    this.update(cx, |state, cx| {
                        state.cluster_consumer_groups.insert(cluster.clone(), groups);
                        cx.notify();
                    }).ok();
                }
                Err(_) => {
                    match client.get::<Vec<serde_json::Value>>(&format!("/api/clusters/{}/consumer-groups", cluster)).await {
                        Ok(groups) => {
                            let group_names: Vec<String> = groups.iter()
                                .filter_map(|g| g.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                                .collect();
                            this.update(cx, |state, cx| {
                                state.cluster_consumer_groups.insert(cluster.clone(), group_names);
                                cx.notify();
                            }).ok();
                        }
                        Err(_) => {}
                    }
                }
            }
        }).detach();
    }

    /// Get cluster health status
    pub fn get_cluster_health(&self, cluster_name: &str) -> Option<&ClusterHealth> {
        self.cluster_health.get(cluster_name)
    }

    /// Get topics for a cluster
    pub fn get_cluster_topics(&self, cluster_name: &str) -> Vec<&String> {
        self.cluster_topics.get(cluster_name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get consumer groups for a cluster
    pub fn get_cluster_consumer_groups(&self, cluster_name: &str) -> Vec<&String> {
        self.cluster_consumer_groups.get(cluster_name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Toggle sidebar mode
    pub fn toggle_sidebar_mode(&mut self) {
        self.sidebar_mode = match self.sidebar_mode {
            SidebarMode::Flat => SidebarMode::Tree,
            SidebarMode::Tree => SidebarMode::Flat,
        };
    }

    /// Select a cluster
    pub fn select_cluster(&mut self, cluster: impl Into<Option<String>>) {
        self.selected_cluster = cluster.into();
    }

    /// Select a topic
    pub fn select_topic(&mut self, cluster: String, topic: String) {
        self.selected_topic = Some((cluster, topic));
    }

    /// Select a consumer group
    pub fn select_consumer_group(&mut self, cluster: String, group: String) {
        self.selected_consumer_group = Some((cluster, group));
    }

    /// Set query partition filter
    pub fn set_query_partition(&mut self, partition: Option<i32>) {
        self.query_partition = partition;
    }

    /// Navigate to messages view with cluster and topic
    pub fn navigate_to_messages(&mut self, cluster: &str, topic: &str) {
        self.selected_topic = Some((cluster.to_string(), topic.to_string()));
        self.router.navigate_to(ViewType::Messages);
    }

    /// Navigate to topics view with cluster
    pub fn navigate_to_topics(&mut self, cluster: &str) {
        self.selected_cluster = Some(cluster.to_string());
        self.router.navigate_to(ViewType::Topics);
    }

    /// Get current view type
    pub fn current_view(&self) -> ViewType {
        self.router.current_view()
    }

    /// Navigate to a view
    pub fn navigate(&mut self, view: ViewType) {
        self.router.navigate_to(view);
    }

    /// Toggle sidebar
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_open = !self.sidebar_open;
    }

    /// Toggle theme
    pub fn toggle_theme(&mut self) {
        if self.theme.is_dark {
            self.theme = Theme::light();
        } else {
            self.theme = Theme::dark();
        }
    }

    /// Toggle language
    pub fn toggle_language(&mut self) {
        self.language = match self.language {
            Language::Chinese => Language::English,
            Language::English => Language::Chinese,
        };
    }

    /// Get translations for current language
    pub fn translations(&self) -> Arc<Translations> {
        match self.language {
            Language::Chinese => Arc::new(Translations::zh()),
            Language::English => Arc::new(Translations::en()),
        }
    }

    /// Start tour
    pub fn start_tour(&mut self) {
        self.tour_active = true;
        self.tour_step = 0;
    }

    /// End tour
    pub fn end_tour(&mut self) {
        self.tour_active = false;
    }

    /// Next tour step
    pub fn next_tour_step(&mut self) {
        self.tour_step += 1;
        if self.tour_step >= 5 {
            self.end_tour();
        }
    }

    /// Previous tour step
    pub fn prev_tour_step(&mut self) {
        if self.tour_step > 0 {
            self.tour_step -= 1;
        }
    }

    /// Update mobile state based on window width
    pub fn update_mobile_state(&mut self, width: Pixels) {
        self.is_mobile = width < px(768.0);
    }

    /// Add message to buffer
    pub fn add_message(&mut self, message: BufferedMessage) {
        self.message_buffer.push(message);
    }

    /// Clear messages
    pub fn clear_messages(&mut self) {
        self.message_buffer.clear();
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.message_buffer.len()
    }

    /// Get total received messages
    pub fn total_received(&self) -> usize {
        self.message_buffer.total_received()
    }

    /// Get estimated memory usage
    pub fn estimated_memory(&self) -> usize {
        self.message_buffer.estimated_memory()
    }

    /// Show a toast notification
    pub fn show_toast(&mut self, message: String, toast_type: ToastType, cx: &mut Context<Self>) {
        let id = self.toast_counter;
        self.toast_counter += 1;
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let duration_ms = 5000; // 5 seconds default

        self.toast_messages.push(ToastMessage {
            id,
            message,
            toast_type,
            created_at,
            duration_ms,
        });
        cx.notify();

        // Schedule auto-dismiss
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(duration_ms)).await;
            this.update(cx, |state, cx| {
                state.hide_toast(id);
                cx.notify();
            }).ok();
        }).detach();
    }

    /// Show success toast
    pub fn show_success(&mut self, message: String, cx: &mut Context<Self>) {
        self.show_toast(message, ToastType::Success, cx);
    }

    /// Show error toast
    pub fn show_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.show_toast(message, ToastType::Error, cx);
    }

    /// Show warning toast
    pub fn show_warning(&mut self, message: String, cx: &mut Context<Self>) {
        self.show_toast(message, ToastType::Warning, cx);
    }

    /// Show info toast
    pub fn show_info(&mut self, message: String, cx: &mut Context<Self>) {
        self.show_toast(message, ToastType::Info, cx);
    }

    /// Hide a specific toast
    pub fn hide_toast(&mut self, id: usize) {
        self.toast_messages.retain(|t| t.id != id);
    }

    /// Clear all toasts
    pub fn clear_toasts(&mut self) {
        self.toast_messages.clear();
    }

    /// Get toast messages
    pub fn get_toasts(&self) -> &[ToastMessage] {
        &self.toast_messages
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for GlobalState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // This is a state-only entity, doesn't render directly
        // Use cx.notify() to trigger parent re-renders
        div().id("global-state-hidden")
    }
}