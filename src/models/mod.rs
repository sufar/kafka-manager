use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub name: String,
    #[serde(default = "default_partitions")]
    pub num_partitions: i32,
    #[serde(default = "default_replication")]
    pub replication_factor: i32,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

fn default_partitions() -> i32 {
    1
}

fn default_replication() -> i32 {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTopicResponse {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicListResponse {
    pub topics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicDetailResponse {
    pub name: String,
    pub partitions: Vec<PartitionDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionDetail {
    pub id: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
}

// 消息相关模型
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub key: Option<String>,
    pub value: String,
    pub partition: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub partition: i32,
    pub offset: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MessageRecord {
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageListResponse {
    pub messages: Vec<MessageRecord>,
}

/// Topic Partition 详细信息（包含 offset 信息）
#[derive(Debug, Serialize, Deserialize)]
pub struct TopicPartitionDetail {
    pub topic: String,
    pub partition: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
    pub earliest_offset: i64,
    pub latest_offset: i64,
    pub first_commit_time: Option<i64>,  // 最早消息的时间戳 (毫秒)
    pub last_commit_time: Option<i64>,   // 最新消息的时间戳 (毫秒)
}

// 集群监控相关模型
#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterInfoResponse {
    pub brokers: Vec<BrokerInfo>,
    pub controller_id: Option<i32>,
    pub cluster_id: Option<String>,
    pub topic_count: i32,
    pub total_partitions: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerInfo {
    pub id: i32,
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerListResponse {
    pub brokers: Vec<BrokerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerDetailResponse {
    pub id: i32,
    pub host: String,
    pub port: i32,
    pub is_controller: bool,
    pub leader_partitions: i32,
    pub replica_partitions: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterMetricsResponse {
    pub broker_count: i32,
    pub controller_id: Option<i32>,
    pub topic_count: i32,
    pub partition_count: i32,
    pub under_replicated_partitions: i32,
}

/// 吞吐量统计（消息/秒）
#[derive(Debug, Serialize, Deserialize)]
pub struct ThroughputStats {
    /// 消息速率（条/秒）
    pub messages_per_second: f64,
    /// 字节速率（字节/秒）
    pub bytes_per_second: Option<f64>,
    /// 统计时间窗口（秒）
    pub window_seconds: i64,
}

/// Topic 吞吐量响应（生产速度）
#[derive(Debug, Serialize, Deserialize)]
pub struct TopicThroughputResponse {
    pub topic: String,
    /// 生产吞吐量
    pub produce_throughput: ThroughputStats,
    /// 总消息数
    pub total_messages: i64,
    pub partitions: Vec<PartitionThroughput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionThroughput {
    pub partition: i32,
    pub earliest_offset: i64,
    pub latest_offset: i64,
    pub message_count: i64,
    /// 生产速率（条/秒）
    pub produce_rate: f64,
    /// 最早消息时间戳
    pub first_message_time: Option<i64>,
    /// 最新消息时间戳
    pub last_message_time: Option<i64>,
}
