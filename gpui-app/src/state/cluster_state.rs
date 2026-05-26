//! Cluster-specific state operations
//!
//! Contains operations for managing cluster state.

use super::{AppState, Cluster, ClusterGroup, ConnectionStatus, ConnectionStatusType};

impl AppState {
    /// Add a cluster to the state
    pub fn add_cluster(&mut self, cluster: Cluster) {
        self.clusters.push(cluster);
    }

    /// Remove a cluster from the state
    pub fn remove_cluster(&mut self, id: i32) {
        self.clusters.retain(|c| c.id != id);
    }

    /// Update a cluster in the state
    pub fn update_cluster(&mut self, id: i32, cluster: Cluster) {
        if let Some(idx) = self.clusters.iter().position(|c| c.id == id) {
            self.clusters[idx] = cluster;
        }
    }

    /// Add a cluster group
    pub fn add_cluster_group(&mut self, group: ClusterGroup) {
        self.cluster_groups.push(group);
    }

    /// Remove a cluster group
    pub fn remove_cluster_group(&mut self, id: i32) {
        self.cluster_groups.retain(|g| g.id != id);
    }

    /// Update connection status for a cluster
    pub fn update_connection_status(&mut self, cluster_name: &str, status: ConnectionStatusType, error: Option<String>) {
        self.connections.insert(cluster_name.to_string(), ConnectionStatus {
            cluster_name: cluster_name.to_string(),
            status,
            error_message: error,
            last_checked: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        });
    }

    /// Get connection status for a cluster
    pub fn get_connection_status(&self, cluster_name: &str) -> Option<&ConnectionStatus> {
        self.connections.get(cluster_name)
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Set error message
    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    /// Select a cluster
    pub fn select_cluster(&mut self, cluster_name: Option<String>) {
        self.selected_cluster = cluster_name;
    }

    /// Get clusters filtered by group
    pub fn get_clusters_in_group(&self, group_id: Option<i32>) -> Vec<&Cluster> {
        self.clusters
            .iter()
            .filter(|c| c.group_id == group_id)
            .collect()
    }
}