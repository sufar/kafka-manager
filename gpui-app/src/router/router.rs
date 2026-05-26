//! Router State Management
//!
//! Manages current view and navigation history with query params support.

use gpui::*;
use std::collections::HashMap;

/// View types for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewType {
    /// Cluster management view
    #[default]
    Clusters,
    /// Topic management view
    Topics,
    /// Message query view
    Messages,
    /// Consumer groups view
    ConsumerGroups,
    /// Schema registry view
    SchemaRegistry,
    /// Settings view
    Settings,
    /// Favorites view
    Favorites,
}

impl ViewType {
    /// Get the route path for this view
    pub fn path(&self) -> &'static str {
        match self {
            ViewType::Clusters => "/clusters",
            ViewType::Topics => "/topics",
            ViewType::Messages => "/messages",
            ViewType::ConsumerGroups => "/consumer-groups",
            ViewType::SchemaRegistry => "/schema-registry",
            ViewType::Settings => "/settings",
            ViewType::Favorites => "/favorites",
        }
    }

    /// Parse a path to get the view type
    pub fn from_path(path: &str) -> Self {
        // Strip query params if present
        let base_path = path.split('?').next().unwrap_or(path);
        match base_path {
            "/clusters" | "" => ViewType::Clusters,
            "/topics" => ViewType::Topics,
            "/messages" => ViewType::Messages,
            "/consumer-groups" => ViewType::ConsumerGroups,
            "/schema-registry" => ViewType::SchemaRegistry,
            "/settings" => ViewType::Settings,
            "/favorites" => ViewType::Favorites,
            _ => ViewType::Clusters,
        }
    }
}

/// Navigation query parameters
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NavigationParams {
    /// Selected cluster
    pub cluster: Option<String>,
    /// Selected topic
    pub topic: Option<String>,
    /// Selected partition
    pub partition: Option<i32>,
    /// Selected consumer group
    pub consumer_group: Option<String>,
    /// Additional params
    pub extra: HashMap<String, String>,
}

impl NavigationParams {
    /// Create new empty params
    pub fn new() -> Self {
        Self::default()
    }

    /// Create params with cluster
    pub fn with_cluster(cluster: String) -> Self {
        Self {
            cluster: Some(cluster),
            ..Default::default()
        }
    }

    /// Create params with cluster and topic
    pub fn with_cluster_and_topic(cluster: String, topic: String) -> Self {
        Self {
            cluster: Some(cluster),
            topic: Some(topic),
            ..Default::default()
        }
    }

    /// Parse query params from URL
    pub fn from_query_string(query: &str) -> Self {
        let mut params = Self::default();
        for pair in query.split('&') {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0];
                let value = parts[1];
                match key {
                    "cluster" => { params.cluster = Some(value.to_string()); }
                    "topic" => { params.topic = Some(value.to_string()); }
                    "partition" => { params.partition = value.parse().ok(); }
                    "consumer_group" => { params.consumer_group = Some(value.to_string()); }
                    _ => { params.extra.insert(key.to_string(), value.to_string()); }
                }
            }
        }
        params
    }

    /// Build query string
    pub fn to_query_string(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(cluster) = &self.cluster {
            parts.push(format!("cluster={}", cluster));
        }
        if let Some(topic) = &self.topic {
            parts.push(format!("topic={}", topic));
        }
        if let Some(partition) = self.partition {
            parts.push(format!("partition={}", partition));
        }
        if let Some(group) = &self.consumer_group {
            parts.push(format!("consumer_group={}", group));
        }
        for (key, value) in &self.extra {
            parts.push(format!("{}={}", key, value));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("?{}", parts.join("&"))
        }
    }
}

/// Navigation state combining view type and params
#[derive(Debug, Clone)]
pub struct NavigationState {
    /// Current view type
    pub view: ViewType,
    /// Query params
    pub params: NavigationParams,
}

impl NavigationState {
    pub fn new(view: ViewType) -> Self {
        Self {
            view,
            params: NavigationParams::default(),
        }
    }

    pub fn with_params(view: ViewType, params: NavigationParams) -> Self {
        Self { view, params }
    }

    /// Build full URL path
    pub fn to_path(&self) -> String {
        let base = self.view.path();
        let query = self.params.to_query_string();
        format!("{}{}", base, query)
    }

    /// Parse from URL path
    pub fn from_path(path: &str) -> Self {
        let parts: Vec<&str> = path.splitn(2, '?').collect();
        let view = ViewType::from_path(parts[0]);
        let params = if parts.len() > 1 {
            NavigationParams::from_query_string(parts[1])
        } else {
            NavigationParams::default()
        };
        Self { view, params }
    }
}

// Define navigation actions using GPUI's actions macro
actions!(router, [NavigateClusters, NavigateTopics, NavigateMessages, NavigateConsumerGroups, NavigateSchemaRegistry, NavigateSettings, NavigateFavorites]);

impl NavigateClusters {
    pub fn view_type() -> ViewType { ViewType::Clusters }
}

impl NavigateTopics {
    pub fn view_type() -> ViewType { ViewType::Topics }
}

impl NavigateMessages {
    pub fn view_type() -> ViewType { ViewType::Messages }
}

impl NavigateConsumerGroups {
    pub fn view_type() -> ViewType { ViewType::ConsumerGroups }
}

impl NavigateSchemaRegistry {
    pub fn view_type() -> ViewType { ViewType::SchemaRegistry }
}

impl NavigateSettings {
    pub fn view_type() -> ViewType { ViewType::Settings }
}

impl NavigateFavorites {
    pub fn view_type() -> ViewType { ViewType::Favorites }
}

/// Router state with query params support
pub struct Router {
    /// Current navigation state (view + params)
    current: NavigationState,
    /// Navigation history
    history: Vec<NavigationState>,
}

impl Router {
    /// Create new router
    pub fn new() -> Self {
        Self {
            current: NavigationState::new(ViewType::default()),
            history: vec![NavigationState::new(ViewType::default())],
        }
    }

    /// Navigate to a new view by path (with optional query params)
    pub fn navigate(&mut self, path: &str) {
        let state = NavigationState::from_path(path);
        self.navigate_to_state(state);
    }

    /// Navigate directly to a view type (without params)
    pub fn navigate_to(&mut self, view: ViewType) {
        let state = NavigationState::new(view);
        self.navigate_to_state(state);
    }

    /// Navigate to a view with params
    pub fn navigate_with_params(&mut self, view: ViewType, params: NavigationParams) {
        let state = NavigationState::with_params(view, params);
        self.navigate_to_state(state);
    }

    /// Navigate to messages view with cluster and topic
    pub fn navigate_to_messages(&mut self, cluster: &str, topic: &str) {
        let params = NavigationParams::with_cluster_and_topic(cluster.to_string(), topic.to_string());
        self.navigate_with_params(ViewType::Messages, params);
    }

    /// Navigate to topics view with cluster
    pub fn navigate_to_topics(&mut self, cluster: &str) {
        let params = NavigationParams::with_cluster(cluster.to_string());
        self.navigate_with_params(ViewType::Topics, params);
    }

    /// Navigate to consumer groups view with cluster
    pub fn navigate_to_consumer_groups(&mut self, cluster: &str) {
        let params = NavigationParams::with_cluster(cluster.to_string());
        self.navigate_with_params(ViewType::ConsumerGroups, params);
    }

    /// Internal navigation to state
    fn navigate_to_state(&mut self, state: NavigationState) {
        if state.view != self.current.view || state.params != self.current.params {
            self.history.push(state.clone());
            self.current = state;
        }
    }

    /// Get current view
    pub fn current_view(&self) -> ViewType {
        self.current.view
    }

    /// Get current navigation params
    pub fn current_params(&self) -> &NavigationParams {
        &self.current.params
    }

    /// Get current cluster param
    pub fn current_cluster(&self) -> Option<&String> {
        self.current.params.cluster.as_ref()
    }

    /// Get current topic param
    pub fn current_topic(&self) -> Option<&String> {
        self.current.params.topic.as_ref()
    }

    /// Get current partition param
    pub fn current_partition(&self) -> Option<i32> {
        self.current.params.partition
    }

    /// Get current consumer group param
    pub fn current_consumer_group(&self) -> Option<&String> {
        self.current.params.consumer_group.as_ref()
    }

    /// Get current path (with query params)
    pub fn current_path(&self) -> String {
        self.current.to_path()
    }

    /// Go back in history
    pub fn go_back(&mut self) {
        if self.history.len() > 1 {
            self.history.pop();
            self.current = self.history.last().unwrap().clone();
        }
    }

    /// Check if can go back
    pub fn can_go_back(&self) -> bool {
        self.history.len() > 1
    }

    /// Get navigation history
    pub fn history(&self) -> &[NavigationState] {
        &self.history
    }

    /// Update current params without changing view
    pub fn update_params(&mut self, params: NavigationParams) {
        self.current.params = params;
    }

    /// Set cluster param
    pub fn set_cluster(&mut self, cluster: String) {
        self.current.params.cluster = Some(cluster);
    }

    /// Set topic param
    pub fn set_topic(&mut self, topic: String) {
        self.current.params.topic = Some(topic);
    }

    /// Clear topic param
    pub fn clear_topic(&mut self) {
        self.current.params.topic = None;
        self.current.params.partition = None;
    }

    /// Set partition param
    pub fn set_partition(&mut self, partition: i32) {
        self.current.params.partition = Some(partition);
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Use Navigate action view_type methods
fn use_navigate_view_types() {
    let clusters_view = crate::router::router::NavigateClusters::view_type();
    let topics_view = crate::router::router::NavigateTopics::view_type();
    let messages_view = crate::router::router::NavigateMessages::view_type();
    let consumer_groups_view = crate::router::router::NavigateConsumerGroups::view_type();
    let schema_registry_view = crate::router::router::NavigateSchemaRegistry::view_type();
    let settings_view = crate::router::router::NavigateSettings::view_type();
    let favorites_view = crate::router::router::NavigateFavorites::view_type();
    println!("View types: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
        clusters_view, topics_view, messages_view, consumer_groups_view,
        schema_registry_view, settings_view, favorites_view);
}

// Call the function to use view_type methods
#[allow(dead_code)]
fn _router_init() {
    use_navigate_view_types();
}