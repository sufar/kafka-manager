//! Router State Management
//!
//! Manages current view and navigation history.

use gpui::*;

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
        match path {
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

/// Router state
pub struct Router {
    /// Current view
    current: ViewType,
    /// Navigation history
    history: Vec<ViewType>,
}

impl Router {
    /// Create new router
    pub fn new() -> Self {
        Self {
            current: ViewType::default(),
            history: vec![ViewType::default()],
        }
    }

    /// Navigate to a new view by path
    pub fn navigate(&mut self, path: &str) {
        let new_view = ViewType::from_path(path);
        self.navigate_to(new_view);
    }

    /// Navigate directly to a view type
    pub fn navigate_to(&mut self, view: ViewType) {
        if view != self.current {
            self.history.push(view);
            self.current = view;
        }
    }

    /// Get current view
    pub fn current_view(&self) -> ViewType {
        self.current
    }

    /// Go back in history
    pub fn go_back(&mut self) {
        if self.history.len() > 1 {
            self.history.pop();
            self.current = *self.history.last().unwrap();
        }
    }

    /// Check if can go back
    pub fn can_go_back(&self) -> bool {
        self.history.len() > 1
    }

    /// Get navigation history
    pub fn history(&self) -> &[ViewType] {
        &self.history
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Use Navigate action view_type methods
fn use_navigate_view_types() {
    // Access the action structs directly - they are created by actions! macro
    // Note: The actions! macro creates them in a submodule, so we use them via crate::router
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