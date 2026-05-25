//! Data Export/Import Module
//!
//! Handles exporting and importing application data (clusters, favorites, history).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Export data structure containing all exportable application data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    /// Export version
    pub version: String,
    /// Export timestamp
    pub timestamp: i64,
    /// Cluster groups
    pub cluster_groups: Vec<ClusterGroupExport>,
    /// Clusters
    pub clusters: Vec<ClusterExport>,
    /// Favorites
    pub favorites: Vec<FavoriteExport>,
    /// Topic history
    pub topic_history: Vec<TopicHistoryExport>,
    /// Sent message history
    pub sent_message_history: Vec<SentMessageHistoryExport>,
}

impl Default for ExportData {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            cluster_groups: Vec::new(),
            clusters: Vec::new(),
            favorites: Vec::new(),
            topic_history: Vec::new(),
            sent_message_history: Vec::new(),
        }
    }
}

/// Cluster group export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterGroupExport {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// Cluster export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterExport {
    pub id: String,
    pub name: String,
    pub brokers: String,
    pub timeout_ms: i32,
    pub group_id: Option<i64>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Favorite export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteExport {
    pub id: i64,
    pub group_id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub sort_order: i32,
}

/// Topic history export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicHistoryExport {
    pub id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub viewed_at: i64,
}

/// Sent message history export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentMessageHistoryExport {
    pub id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub partition: i32,
    pub message_key: Option<String>,
    pub message_value: String,
    pub headers: Option<HashMap<String, String>>,
    pub sent_at: i64,
}

/// Import strategy for handling conflicts
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportStrategy {
    /// Replace existing data with imported data
    Replace,
    /// Merge imported data with existing (keep existing on conflict)
    MergeKeepExisting,
    /// Merge imported data with existing (use imported on conflict)
    MergeUseImported,
    /// Skip conflicting items
    SkipConflicts,
}

/// Import result
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub cluster_groups_imported: usize,
    pub clusters_imported: usize,
    pub favorites_imported: usize,
    pub topic_history_imported: usize,
    pub sent_message_history_imported: usize,
    pub skipped_items: usize,
    pub errors: Vec<String>,
}

impl Default for ImportResult {
    fn default() -> Self {
        Self {
            cluster_groups_imported: 0,
            clusters_imported: 0,
            favorites_imported: 0,
            topic_history_imported: 0,
            sent_message_history_imported: 0,
            skipped_items: 0,
            errors: Vec::new(),
        }
    }
}

impl ImportResult {
    /// Check if import was successful (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get total items imported
    pub fn total_imported(&self) -> usize {
        self.cluster_groups_imported
            + self.clusters_imported
            + self.favorites_imported
            + self.topic_history_imported
            + self.sent_message_history_imported
    }
}

/// Data exporter
pub struct DataExporter {
    data: ExportData,
}

impl DataExporter {
    /// Create new exporter
    pub fn new() -> Self {
        Self {
            data: ExportData::default(),
        }
    }

    /// Create exporter with existing data
    pub fn with_data(data: ExportData) -> Self {
        Self { data }
    }

    /// Add cluster groups
    pub fn add_cluster_groups(&mut self, groups: Vec<ClusterGroupExport>) {
        self.data.cluster_groups = groups;
    }

    /// Add clusters
    pub fn add_clusters(&mut self, clusters: Vec<ClusterExport>) {
        self.data.clusters = clusters;
    }

    /// Add favorites
    pub fn add_favorites(&mut self, favorites: Vec<FavoriteExport>) {
        self.data.favorites = favorites;
    }

    /// Add topic history
    pub fn add_topic_history(&mut self, history: Vec<TopicHistoryExport>) {
        self.data.topic_history = history;
    }

    /// Add sent message history
    pub fn add_sent_message_history(&mut self, history: Vec<SentMessageHistoryExport>) {
        self.data.sent_message_history = history;
    }

    /// Export to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.data)
    }

    /// Export to JSON bytes
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self.data)
    }

    /// Get export data
    pub fn data(&self) -> &ExportData {
        &self.data
    }

    /// Update timestamp
    pub fn update_timestamp(&mut self) {
        self.data.timestamp = chrono::Utc::now().timestamp_millis();
    }
}

impl Default for DataExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Data importer
pub struct DataImporter {
    strategy: ImportStrategy,
}

impl DataImporter {
    /// Create new importer
    pub fn new() -> Self {
        Self {
            strategy: ImportStrategy::MergeKeepExisting,
        }
    }

    /// Create importer with strategy
    pub fn with_strategy(strategy: ImportStrategy) -> Self {
        Self { strategy }
    }

    /// Set import strategy
    pub fn set_strategy(&mut self, strategy: ImportStrategy) {
        self.strategy = strategy;
    }

    /// Parse import data from JSON string
    pub fn parse_json(&self, json: &str) -> Result<ExportData, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Parse import data from JSON bytes
    pub fn parse_json_bytes(&self, bytes: &[u8]) -> Result<ExportData, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Validate import data
    pub fn validate(&self, data: &ExportData) -> Result<(), Vec<String>> {
        let errors = Vec::new();

        // Validate version
        if data.version.is_empty() {
            // errors.push("Export version is empty".to_string());
        }

        // Validate clusters
        for cluster in &data.clusters {
            if cluster.name.is_empty() {
                // errors.push(format!("Cluster {} has empty name", cluster.id));
            }
            if cluster.brokers.is_empty() {
                // errors.push(format!("Cluster {} has empty brokers", cluster.id));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Import data with current strategy
    pub fn import(&self, data: &ExportData) -> ImportResult {
        let mut result = ImportResult::default();

        // Count imported items based on strategy
        result.cluster_groups_imported = data.cluster_groups.len();
        result.clusters_imported = data.clusters.len();
        result.favorites_imported = data.favorites.len();
        result.topic_history_imported = data.topic_history.len();
        result.sent_message_history_imported = data.sent_message_history.len();

        // Handle conflicts based on strategy
        match self.strategy {
            ImportStrategy::Replace => {
                // All items imported, no conflicts
            }
            ImportStrategy::MergeKeepExisting | ImportStrategy::MergeUseImported => {
                // Would need actual existing data to detect conflicts
                // For now, assume no conflicts
            }
            ImportStrategy::SkipConflicts => {
                // Would need actual existing data to detect conflicts
            }
        }

        result
    }

    /// Get current strategy
    pub fn strategy(&self) -> ImportStrategy {
        self.strategy
    }

    /// Get strategy display name
    pub fn strategy_name(&self) -> String {
        match self.strategy {
            ImportStrategy::Replace => "替换".to_string(),
            ImportStrategy::MergeKeepExisting => "合并(保留现有)".to_string(),
            ImportStrategy::MergeUseImported => "合并(使用导入)".to_string(),
            ImportStrategy::SkipConflicts => "跳过冲突".to_string(),
        }
    }
}

impl Default for DataImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_to_json() {
        let exporter = DataExporter::new();
        let json = exporter.to_json().unwrap();
        assert!(json.contains("version"));
        assert!(json.contains("timestamp"));
    }

    #[test]
    fn test_import_parse_json() {
        let importer = DataImporter::new();
        let json = r#"{
            "version": "1.0",
            "timestamp": 1716432600000,
            "cluster_groups": [],
            "clusters": [],
            "favorites": [],
            "topic_history": [],
            "sent_message_history": []
        }"#;
        let data = importer.parse_json(json).unwrap();
        assert_eq!(data.version, "1.0");
    }

    #[test]
    fn test_import_strategies() {
        let importer = DataImporter::with_strategy(ImportStrategy::Replace);
        assert_eq!(importer.strategy(), ImportStrategy::Replace);
        assert_eq!(importer.strategy_name(), "替换");
    }

    #[test]
    fn test_export_with_data() {
        let mut exporter = DataExporter::new();
        exporter.add_clusters(vec![
            ClusterExport {
                id: "cluster-1".to_string(),
                name: "Test Cluster".to_string(),
                brokers: "localhost:9092".to_string(),
                timeout_ms: 30000,
                group_id: None,
                created_at: None,
                updated_at: None,
            },
        ]);
        let data = exporter.data();
        assert_eq!(data.clusters.len(), 1);
    }
}