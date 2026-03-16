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

// API 错误
export interface ApiError {

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

// ==================== 消息管理 ====================

export interface MessageRecord {
  partition: number;
  offset: number;
  key?: string;
  value?: string;
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
