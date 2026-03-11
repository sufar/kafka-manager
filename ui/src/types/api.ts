// API 类型定义

export interface Cluster {
  id: number;
  name: string;
  brokers: string;
  request_timeout_ms: number;
  operation_timeout_ms: number;
  created_at: string;
  updated_at: string;
}

export interface ClusterListResponse {
  clusters: Cluster[];
}

export interface CreateClusterRequest {
  name: string;
  brokers: string;
  request_timeout_ms?: number;
  operation_timeout_ms?: number;
}

export interface UpdateClusterRequest {
  name?: string;
  brokers?: string;
  request_timeout_ms?: number;
  operation_timeout_ms?: number;
}

export interface TestConnectionResponse {
  success: boolean;
}

// Topic 相关类型
export interface TopicListResponse {
  topics: string[];
}

export interface CreateTopicRequest {
  name: string;
  num_partitions?: number;
  replication_factor?: number;
  config?: Record<string, string>;
}

export interface CreateTopicResponse {
  name: string;
}

export interface PartitionDetail {
  id: number;
  leader: number;
  replicas: number[];
  isr: number[];
}

export interface TopicDetailResponse {
  name: string;
  partitions: PartitionDetail[];
}

export interface TopicPartitionDetail {
  topic: string;
  partition: number;
  leader: number;
  replicas: number[];
  isr: number[];
  earliest_offset: number;
  latest_offset: number;
  first_commit_time?: number;
  last_commit_time?: number;
}

export interface BatchCreateTopicsRequest {
  topics: CreateTopicRequest[];
  continue_on_error?: boolean;
}

export interface BatchCreateTopicsResponse {
  success: boolean;
  created: string[];
  failed: FailedItem[];
}

export interface BatchDeleteTopicsRequest {
  topics: string[];
  continue_on_error?: boolean;
}

export interface BatchDeleteTopicsResponse {
  success: boolean;
  deleted: string[];
  failed: FailedItem[];
}

export interface FailedItem {
  name: string;
  error: string;
}

// Consumer Group 相关类型
export interface ConsumerGroupSummary {
  name: string;
  state: string;
}

export interface ConsumerGroupListResponse {
  groups: ConsumerGroupSummary[];
}

export interface ConsumerGroupMember {
  client_id: string;
  host: string;
}

export interface ConsumerGroupOffset {
  topic: string;
  partition: number;
  current_offset: number;
  log_end_offset: number;
  lag: number;
}

export interface ConsumerGroupDetailResponse {
  name: string;
  state: string;
  protocol?: string;
  members: ConsumerGroupMember[];
  offsets: ConsumerGroupOffset[];
}

export interface ConsumerGroupPartitionDetail {
  partition: number;
  current_offset: number;
  log_end_offset: number;
  lag: number;
  state: string;
  last_commit_time?: number;
  topic?: string;
  start_offset?: number;
}

export interface ConsumerGroupOffsetDetailResponse {
  group_name: string;
  topic: string;
  partitions: ConsumerGroupPartitionDetail[];
  total_lag: number;
}

export interface ResetConsumerGroupOffsetRequest {
  topic: string;
  partition?: number;
  offset: ResetOffsetType;
}

export interface ResetOffsetType {
  type: 'earliest' | 'latest' | 'value' | 'timestamp';
  value?: number;
}

export interface BatchDeleteConsumerGroupsRequest {
  group_names: string[];
  continue_on_error?: boolean;
}

export interface BatchDeleteConsumerGroupsResponse {
  success: boolean;
  deleted: string[];
  failed: FailedConsumerGroup[];
}

export interface FailedConsumerGroup {
  name: string;
  error: string;
}

// 集群统计相关类型
export interface BrokerStats {
  id: number;
  host: string;
  port: number;
  is_controller: boolean;
  leader_partitions: number;
  replica_partitions: number;
}

export interface ClusterStatsResponse {
  cluster_id: string;
  broker_count: number;
  controller_id?: number;
  topic_count: number;
  partition_count: number;
  under_replicated_partitions: number;
  consumer_group_count: number;
  total_lag: number;
  broker_stats: BrokerStats[];
}

// 健康检查
export interface HealthResponse {
  status: string;
  version: string;
}

// API 错误
export interface ApiError {
  message: string;
  status: number;
}

// ==================== Schema Registry ====================

export interface SchemaInfo {
  subject: string;
  version: number;
  id: number;
  schema_type: string;
  schema: string;
  references?: SchemaReference[];
}

export interface SchemaReference {
  name: string;
  subject: string;
  version: number;
}

export interface SubjectVersionsResponse {
  cluster_id: string;
  subject: string;
  versions: number[];
}

export interface CompatibilityCheck {
  is_compatible: boolean;
  messages: string[];
}

export interface RegisterSchemaRequest {
  subject: string;
  schema: string;
  schema_type?: string;
}

// ==================== 用户和角色管理 ====================

export interface UserResponse {
  id: number;
  username: string;
  email?: string;
  role_id?: number;
  role_name?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateUserRequest {
  username: string;
  password: string;
  email?: string;
  role_id?: number;
}

export interface RoleResponse {
  id: number;
  name: string;
  description?: string;
  permissions: string[];
  created_at: string;
}

export interface CreateRoleRequest {
  name: string;
  description?: string;
  permissions: string[];
}

// ==================== 通知管理 ====================

export interface NotificationConfig {
  id: number;
  name: string;
  type: string;
  config: Record<string, string>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateNotificationConfigRequest {
  name: string;
  type: string;
  config: Record<string, string>;
  enabled?: boolean;
}

export interface AlertHistoryItem {
  id: number;
  rule_id: number;
  cluster_id: string;
  alert_type: string;
  alert_message: string;
  alert_value: string;
  threshold: number;
  severity: string;
  notified: boolean;
  created_at: string;
}

// ==================== 消息管理 ====================

export interface MessageRecord {
  partition: number;
  offset: number;
  key?: string;
  value?: string;
  timestamp?: number;
}

export interface QueryStats {
  partitions_queried: number;
  total_scanned: number;
  matched: number;
  query_time_ms: number;
  per_partition_stats?: Record<number, PartitionQueryStats>;
}

export interface PartitionQueryStats {
  scanned: number;
  matched: number;
  start_offset: number;
  end_offset: number;
}

export interface MessageListResponse {
  messages: MessageRecord[];
  stats?: QueryStats;
}

export interface SendMessageRequest {
  partition: number;
  key?: string;
  value: string;
  headers?: Record<string, string>;
}

export interface SendMessageResponse {
  partition: number;
  offset: number;
}

export interface EnhancedMessageRecord {
  partition: number;
  offset: number;
  key?: string;
  value?: string;
  timestamp?: number;
  key_raw?: string;
  value_raw?: string;
  value_json?: Record<string, unknown>;
  value_hex?: string;
  content_type?: string;
  size: {
    key_size: number;
    value_size: number;
    total_size: number;
  };
}

// ==================== 集群监控 ====================

export interface BrokerInfo {
  id: number;
  host: string;
  port: number;
}

export interface ClusterInfoResponse {
  brokers: BrokerInfo[];
  controller_id?: number;
  cluster_id?: string;
  topic_count: number;
  total_partitions: number;
}

export interface ClusterMetricsResponse {
  broker_count: number;
  controller_id?: number;
  topic_count: number;
  partition_count: number;
  under_replicated_partitions: number;
}

export interface BrokerDetailResponse {
  id: number;
  host: string;
  port: number;
  rack?: string;
  log_dirs: LogDirInfo[];
}

export interface LogDirInfo {
  log_dir: string;
  error?: string;
  partitions: LogDirPartitionInfo[];
}

export interface LogDirPartitionInfo {
  topic: string;
  partition: number;
  size: number;
  offset_lag?: number;
  is_future?: boolean;
}

// ==================== Consumer Offsets ====================

export interface PartitionOffsetDetail {
  partition: number;
  start_offset: number;
  end_offset: number;
  current_offset: number;
  lag: number;
  state: string;
}

export interface TopicOffsetsSummary {
  topic: string;
  partitions: PartitionOffsetDetail[];
  total_lag: number;
}

export interface ConsumerGroupOffsetsSummary {
  group_name: string;
  state: string;
  topics: TopicOffsetsSummary[];
  total_lag: number;
}

export interface ConsumerOffsetsListResponse {
  cluster_id: string;
  consumer_groups: ConsumerGroupOffsetsSummary[];
}
