//! Application State
//!
//! The central state container for the entire application.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Main application state
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// List of clusters
    pub clusters: Vec<Cluster>,
    /// List of cluster groups
    pub cluster_groups: Vec<ClusterGroup>,
    /// Connection status for each cluster
    pub connections: HashMap<String, ConnectionStatus>,
    /// Currently selected cluster ID
    pub selected_cluster: Option<String>,
    /// Loading state
    pub loading: bool,
    /// Error message if any
    pub error: Option<String>,
}

/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: i32,
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i32,
    pub operation_timeout_ms: i32,
    pub group_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Cluster group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterGroup {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Connection status for a cluster
#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub cluster_name: String,
    pub status: ConnectionStatusType,
    pub error_message: Option<String>,
    pub last_checked: i64,
}

/// Connection status type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatusType {
    Connected,
    Disconnected,
    Error,
    Unknown,
}

impl ConnectionStatusType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionStatusType::Connected => "connected",
            ConnectionStatusType::Disconnected => "disconnected",
            ConnectionStatusType::Error => "error",
            ConnectionStatusType::Unknown => "unknown",
        }
    }
}