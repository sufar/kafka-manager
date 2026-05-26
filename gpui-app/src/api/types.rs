//! API Types
//!
//! Request and response types for API communication.

use serde::{Deserialize, Serialize};

// ==================== Cluster Types ====================

/// Cluster response from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterResponse {
    pub id: i32,
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i32,
    pub operation_timeout_ms: i32,
    pub group_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Create cluster request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClusterRequest {
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: Option<i32>,
    pub operation_timeout_ms: Option<i32>,
    pub group_id: Option<i32>,
}

/// Update cluster request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateClusterRequest {
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: Option<i32>,
    pub operation_timeout_ms: Option<i32>,
    pub group_id: Option<i32>,
}

/// Cluster group response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterGroupResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Create cluster group request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClusterGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

// ==================== Topic Types ====================

/// Topic response from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicResponse {
    pub name: String,
    pub partitions: i32,
    pub replication_factor: i32,
    pub config: Option<serde_json::Value>,
}

/// Create topic request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub name: String,
    pub partitions: i32,
    pub replication_factor: i32,
    pub config: Option<serde_json::Value>,
}

/// Topic partition info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub partition: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
}

// ==================== Message Types ====================

/// Message query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueryRequest {
    pub topic: String,
    pub partition: Option<i32>,
    pub start_offset: Option<i64>,
    pub end_offset: Option<i64>,
    pub max_messages: i32,
    pub search_value: Option<String>,
}

/// Kafka message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessage {
    pub partition: i32,
    pub offset: i64,
    pub timestamp: i64,
    pub key: Option<String>,
    pub value: String,
    pub headers: Option<serde_json::Value>,
}

/// Send message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub topic: String,
    pub partition: Option<i32>,
    pub key: Option<String>,
    pub value: String,
    pub headers: Option<serde_json::Value>,
}

// ==================== Consumer Group Types ====================

/// Consumer group response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroupResponse {
    pub group_id: String,
    pub state: String,
    pub members: i32,
    pub topics: Vec<String>,
}

/// Consumer group member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroupMember {
    pub member_id: String,
    pub client_id: String,
    pub host: String,
    pub assignments: Vec<TopicPartitionAssignment>,
}

/// Topic partition assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicPartitionAssignment {
    pub topic: String,
    pub partition: i32,
}

// ==================== Schema Registry Types ====================

/// Schema subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSubject {
    pub subject: String,
    pub versions: Vec<i32>,
}

/// Schema version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub subject: String,
    pub version: i32,
    pub schema_type: String,
    pub schema: String,
    pub id: i32,
}

/// Register schema request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSchemaRequest {
    pub subject: String,
    pub schema_type: String,
    pub schema: String,
}