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

/// 增强的消息列表响应，包含统计信息
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageListResponseWithStats {
    pub messages: Vec<MessageRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<QueryStatsWrapper>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryStatsWrapper {
    pub partitions_queried: usize,
    pub total_scanned: usize,
    pub matched: usize,
    pub query_time_ms: u64,
}

// Consumer Group 相关模型
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupListResponse {
    pub groups: Vec<ConsumerGroupSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupSummary {
    pub name: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupDetailResponse {
    pub name: String,
    pub state: String,
    pub protocol: Option<String>,
    pub members: Vec<ConsumerGroupMember>,
    pub offsets: Vec<ConsumerGroupOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupMember {
    pub client_id: String,
    pub host: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupOffset {
    pub topic: String,
    pub partition: i32,
    pub current_offset: i64,
    pub log_end_offset: i64,
    pub lag: i64,
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

/// Consumer Group 带 offset 详情的响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupOffsetDetailResponse {
    pub group_name: String,
    pub topic: String,
    pub partitions: Vec<ConsumerGroupPartitionDetail>,
    pub total_lag: i64,
}

/// Consumer Group Partition 详情
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupPartitionDetail {
    pub partition: i32,
    pub current_offset: i64,
    pub log_end_offset: i64,
    pub lag: i64,
    pub state: String, // "Active", "Empty", etc.
    pub last_commit_time: Option<i64>,  // 最后提交时间 (毫秒)
    pub topic: String,
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

/// Consumer Group offset 重置请求
#[derive(Debug, Serialize, Deserialize)]
pub struct ResetConsumerGroupOffsetRequest {
    pub topic: String,
    pub partition: Option<i32>,
    pub offset: ResetOffsetType,
}

/// 重置 offset 类型
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "lowercase")]
pub enum ResetOffsetType {
    Earliest,
    Latest,
    Value(i64),
    Timestamp(i64),
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

/// Consumer Group 吞吐量响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupThroughputResponse {
    pub group_name: String,
    pub topic: String,
    /// 消费吞吐量
    pub consume_throughput: ThroughputStats,
    /// 积压消息总数
    pub total_lag: i64,
    /// 预估消费完成时间（秒）
    pub estimated_time_to_catch_up: Option<f64>,
    pub partitions: Vec<ConsumerGroupPartitionThroughput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupPartitionThroughput {
    pub partition: i32,
    pub current_offset: i64,
    pub log_end_offset: i64,
    pub lag: i64,
    /// 消费速率（条/秒）
    pub consume_rate: f64,
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

/// Topic 所有 Consumer Group 积压情况响应
#[derive(Debug, Serialize, Deserialize)]
pub struct TopicConsumerLagResponse {
    pub topic: String,
    /// 总积压消息数（所有 consumer group 累加）
    pub total_lag: i64,
    /// Consumer Group 列表
    pub consumer_groups: Vec<ConsumerGroupLagSummary>,
}

/// Consumer Group 积压摘要
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupLagSummary {
    pub group_name: String,
    /// 该 group 的总积压数
    pub total_lag: i64,
    /// 分区积压详情
    pub partitions: Vec<PartitionLagDetail>,
}

/// 分区积压详情
#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionLagDetail {
    pub partition: i32,
    /// 当前已消费 offset
    pub current_offset: i64,
    /// 最新 offset
    pub log_end_offset: i64,
    /// 积压数
    pub lag: i64,
    /// 状态
    pub state: String,
}

/// Consumer Offsets 列表响应（用于 Consumer Group 列表页面）
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerOffsetsListResponse {
    /// 集群 ID
    pub cluster_id: String,
    /// Consumer Group 列表
    pub consumer_groups: Vec<ConsumerGroupOffsetsSummary>,
}

/// Consumer Group offsets 摘要
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupOffsetsSummary {
    /// Consumer Group 名称
    pub group_name: String,
    /// 状态
    pub state: String,
    /// Topic 列表
    pub topics: Vec<TopicOffsetsSummary>,
    /// 总 lag
    pub total_lag: i64,
}

/// Topic offsets 摘要
#[derive(Debug, Serialize, Deserialize)]
pub struct TopicOffsetsSummary {
    /// Topic 名称
    pub topic: String,
    /// Partition offsets 详情
    pub partitions: Vec<PartitionOffsetDetail>,
    /// 该 topic 的总 lag
    pub total_lag: i64,
}

/// Partition offset 详情
#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionOffsetDetail {
    /// Partition ID
    pub partition: i32,
    /// Start Offset (earliest)
    pub start_offset: i64,
    /// End Offset (latest)
    pub end_offset: i64,
    /// 当前消费 offset
    pub current_offset: i64,
    /// Lag
    pub lag: i64,
    /// 状态 (Active/Empty)
    pub state: String,
}

// ==================== 历史积压数据（用于折线图） ====================

/// 历史积压数据响应（用于折线图）
#[derive(Debug, Serialize, Deserialize)]
pub struct TopicConsumerLagHistoryResponse {
    pub topic: String,
    /// 时间范围起点（毫秒时间戳）
    pub start_time: i64,
    /// 时间范围终点（毫秒时间戳）
    pub end_time: i64,
    /// 采样点数
    pub data_points: i32,
    /// X 轴时间标签（毫秒时间戳数组）
    pub timestamps: Vec<i64>,
    /// 每个 Consumer Group 的历史数据
    pub consumer_groups: Vec<ConsumerGroupLagHistory>,
}

/// Consumer Group 历史积压数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumerGroupLagHistory {
    pub group_name: String,
    /// Y 轴数据：每个时间点的积压数（与 timestamps 对应）
    pub lag_series: Vec<i64>,
    /// 每个时间点的消费进度（已消费 offset 总和）
    pub consumed_series: Vec<i64>,
    /// 每个时间点的生产进度（log end offset 总和）
    pub produced_series: Vec<i64>,
}

/// 历史积压数据查询参数
#[derive(Debug, Serialize, Deserialize)]
pub struct LagHistoryQuery {
    /// Topic 名称（可选，如果从路径获取）
    pub topic: Option<String>,
    /// 开始时间戳（毫秒），默认 1 小时前
    pub start_time: Option<i64>,
    /// 结束时间戳（毫秒），默认当前时间
    pub end_time: Option<i64>,
    /// 采样点数，默认 60 个点
    pub data_points: Option<i32>,
}
