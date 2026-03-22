// API 类型定义

export interface Cluster {
  id: number;
  name: string;
  brokers: string;
  request_timeout_ms: number;
  operation_timeout_ms: number;
  group_id?: number | null;
  created_at: string;
  updated_at: string;
}

export interface ClusterGroup {
  id: number;
  name: string;
  description?: string | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface ClusterListResponse {
  clusters: Cluster[];
}

export interface ClusterGroupListResponse {
  groups: ClusterGroup[];
}

export interface CreateClusterRequest {
  name: string;
  brokers: string;
  request_timeout_ms?: number;
  operation_timeout_ms?: number;
  group_id?: number;
}

export interface UpdateClusterRequest {
  name?: string;
  brokers?: string;
  request_timeout_ms?: number;
  operation_timeout_ms?: number;
  group_id?: number;
}

export interface TestConnectionResponse {
  success: boolean;
}

// Topic 相关类型
export interface TopicWithCluster {
  name: string;
  cluster: string;
}

export interface TopicListResponse {
  topics: string[];
}

export interface TopicListWithClusterResponse {
  topics: TopicWithCluster[];
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

// ==================== 消息管理 ====================

// 消息格式类型
export type MessageFormatType = 'json' | 'text' | 'binary_hex' | 'binary_base64' | 'avro' | 'protobuf' | 'unknown';

// 格式化后的消息内容
export interface FormattedMessage {
  raw: string;
  formatted: string;
  format: MessageFormatType;
  parsed?: any;
  metadata: MessageMetadata;
}

// 消息元数据
export interface MessageMetadata {
  size_bytes: number;
  is_empty: boolean;
  field_count?: number;
  nesting_depth?: number;
  is_binary: boolean;
}

export interface MessageRecord {
  partition: number;
  offset: number;
  key?: FormattedMessage;
  value?: FormattedMessage;
  timestamp?: number;
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
