import type {
  Cluster,
  ClusterListResponse,
  CreateClusterRequest,
  UpdateClusterRequest,
  TestConnectionResponse,
  TopicListResponse,
  CreateTopicRequest,
  CreateTopicResponse,
  TopicDetailResponse,
  TopicPartitionDetail,
  BatchCreateTopicsRequest,
  BatchCreateTopicsResponse,
  BatchDeleteTopicsRequest,
  BatchDeleteTopicsResponse,
  ConsumerGroupListResponse,
  ConsumerGroupDetailResponse,
  ConsumerGroupOffsetDetailResponse,
  ResetConsumerGroupOffsetRequest,
  BatchDeleteConsumerGroupsRequest,
  BatchDeleteConsumerGroupsResponse,
  ClusterStatsResponse,
  HealthResponse,
  ApiError,
  ConsumerGroupSummary,
} from '@/types/api';

// 检测是否在 Tauri 环境下运行
function isTauri(): boolean {
  // Tauri 2 使用 __TAURI__ 全局对象
  return typeof window !== 'undefined' &&
    ('__TAURI_INTERNALS__' in window ||
     '__TAURI__' in window ||
     !!window.navigator.userAgent.includes('Tauri'));
}

// 获取 API 基础 URL
function getBaseURL(): string {
  // 在 Tauri 环境下，使用 localhost:3000
  if (isTauri()) {
    console.log('[ApiClient] Running in Tauri environment, using http://localhost:3000');
    return 'http://localhost:3000';
  }
  // 在浏览器开发环境下，使用空字符串（通过 Vite 代理）
  console.log('[ApiClient] Running in web environment, using relative paths');
  return '';
}

class ApiClient {
  private baseURL: string;
  private apiKey: string | null;
  private currentAbortController: AbortController | null = null;

  constructor(baseURL: string = getBaseURL(), apiKey: string | null = null) {
    this.baseURL = baseURL;
    this.apiKey = apiKey;
  }

  setApiKey(apiKey: string) {
    this.apiKey = apiKey;
  }

  // 取消当前的 API 请求
  cancelRequest() {
    if (this.currentAbortController) {
      this.currentAbortController.abort();
      this.currentAbortController = null;
    }
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    // 创建新的 AbortController
    this.currentAbortController = new AbortController();
    const { signal } = this.currentAbortController;

    const url = `${this.baseURL}${endpoint}`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(options.headers as Record<string, string>),
    };

    if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    const response = await fetch(url, {
      ...options,
      headers,
      signal,
    });

    if (!response.ok) {
      let message = `HTTP ${response.status}: ${response.statusText}`;
      try {
        const errorData = await response.json();
        message = errorData.message || message;
      } catch {
        // 忽略解析错误
      }
      throw { message, status: response.status } as ApiError;
    }

    // 处理 204 No Content
    if (response.status === 204) {
      return {} as T;
    }

    return response.json();
  }

  // ==================== Health ====================
  async health(): Promise<HealthResponse> {
    return this.request('/api/health');
  }

  // ==================== Clusters ====================
  async getClusters(): Promise<Cluster[]> {
    const data = await this.request<ClusterListResponse>('/api/clusters');
    return data.clusters;
  }

  async getCluster(id: number): Promise<Cluster> {
    return this.request<Cluster>(`/api/clusters/${id}`);
  }

  async createCluster(cluster: CreateClusterRequest): Promise<Cluster> {
    return this.request('/api/clusters', {
      method: 'POST',
      body: JSON.stringify(cluster),
    });
  }

  async updateCluster(id: number, cluster: UpdateClusterRequest): Promise<Cluster> {
    return this.request(`/api/clusters/${id}`, {
      method: 'PUT',
      body: JSON.stringify(cluster),
    });
  }

  async deleteCluster(id: number): Promise<void> {
    return this.request(`/api/clusters/${id}`, {
      method: 'DELETE',
    });
  }

  async testCluster(id: number): Promise<TestConnectionResponse> {
    return this.request(`/api/clusters/_test/${id}`, {
      method: 'POST',
    });
  }

  // ==================== Topics ====================
  async getTopics(clusterId: string): Promise<string[]> {
    const data = await this.request<TopicListResponse>(`/api/clusters/${clusterId}/topics`);
    return data.topics;
  }

  async getTopicDetail(clusterId: string, topicName: string): Promise<TopicDetailResponse> {
    return this.request(`/api/clusters/${clusterId}/topics/${topicName}`);
  }

  async createTopic(clusterId: string, topic: CreateTopicRequest): Promise<CreateTopicResponse> {
    return this.request(`/api/clusters/${clusterId}/topics`, {
      method: 'POST',
      body: JSON.stringify(topic),
    });
  }

  async batchCreateTopics(
    clusterId: string,
    topics: BatchCreateTopicsRequest
  ): Promise<BatchCreateTopicsResponse> {
    return this.request(`/api/clusters/${clusterId}/topics/batch`, {
      method: 'POST',
      body: JSON.stringify(topics),
    });
  }

  async deleteTopic(clusterId: string, topicName: string): Promise<void> {
    return this.request(`/api/clusters/${clusterId}/topics/${topicName}`, {
      method: 'DELETE',
    });
  }

  async batchDeleteTopics(
    clusterId: string,
    topics: BatchDeleteTopicsRequest
  ): Promise<BatchDeleteTopicsResponse> {
    return this.request(`/api/clusters/${clusterId}/topics/batch`, {
      method: 'DELETE',
      body: JSON.stringify(topics),
    });
  }

  async getTopicOffsets(clusterId: string, topicName: string): Promise<TopicPartitionDetail[]> {
    return this.request(`/api/clusters/${clusterId}/topics/${topicName}/offsets`);
  }

  async addPartitions(clusterId: string, topicName: string, newPartitions: number): Promise<void> {
    return this.request(`/api/clusters/${clusterId}/topics/${topicName}/partitions`, {
      method: 'POST',
      body: JSON.stringify({ new_partitions: newPartitions }),
    });
  }

  async refreshTopics(clusterId: string): Promise<{ success: boolean; added: string[]; removed: string[]; total: number }> {
    return this.request(`/api/clusters/${clusterId}/topics/_refresh`, {
      method: 'POST',
    });
  }

  async searchTopics(): Promise<{ cluster: string; topic: string }[]> {
    const data = await this.request<{ results: { cluster: string; topic: string }[] }>('/api/topics/search');
    return data.results || [];
  }

  async getTopicCount(clusterId: string): Promise<number> {
    const data = await this.request<{ count: number }>(`/api/clusters/${clusterId}/topics/_count`);
    return data.count;
  }

  async getTopicConfig(clusterId: string, topicName: string): Promise<Record<string, string>> {
    return this.request(`/api/clusters/${clusterId}/topics/${topicName}/config`);
  }

  async alterTopicConfig(
    clusterId: string,
    topicName: string,
    config: Record<string, string>
  ): Promise<void> {
    return this.request(`/api/clusters/${clusterId}/topics/${topicName}/config`, {
      method: 'POST',
      body: JSON.stringify({ config }),
    });
  }

  // ==================== Consumer Groups ====================
  async getConsumerGroups(clusterId: string): Promise<ConsumerGroupSummary[]> {
    const data = await this.request<ConsumerGroupListResponse>(
      `/api/clusters/${clusterId}/consumer-groups`
    );
    return data.groups;
  }

  async getConsumerGroupDetail(
    clusterId: string,
    groupName: string
  ): Promise<ConsumerGroupDetailResponse> {
    return this.request(`/api/clusters/${clusterId}/consumer-groups/${groupName}`);
  }

  async getConsumerGroupOffsets(
    clusterId: string,
    groupName: string,
    topic?: string
  ): Promise<ConsumerGroupOffsetDetailResponse> {
    const params = new URLSearchParams();
    if (topic) params.append('topic', topic);

    const query = params.toString() ? `?${params}` : '';
    return this.request(`/api/clusters/${clusterId}/consumer-groups/${groupName}/_offsets${query}`);
  }

  async resetConsumerGroupOffset(
    clusterId: string,
    groupName: string,
    request: ResetConsumerGroupOffsetRequest
  ): Promise<void> {
    return this.request(`/api/clusters/${clusterId}/consumer-groups/${groupName}/_offsets/reset`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async deleteConsumerGroup(clusterId: string, groupName: string): Promise<void> {
    return this.request(`/api/clusters/${clusterId}/consumer-groups/${groupName}`, {
      method: 'DELETE',
    });
  }

  async batchDeleteConsumerGroups(
    clusterId: string,
    request: BatchDeleteConsumerGroupsRequest
  ): Promise<BatchDeleteConsumerGroupsResponse> {
    return this.request(`/api/clusters/${clusterId}/consumer-groups/_batch`, {
      method: 'DELETE',
      body: JSON.stringify(request),
    });
  }

  // ==================== Cluster Stats ====================
  async getClusterStats(clusterId: string): Promise<ClusterStatsResponse> {
    return this.request(`/api/clusters/${clusterId}/stats`);
  }

  // ==================== Schema Registry ====================
  async getSchemaSubjects(clusterId: string, schemaRegistryUrl: string): Promise<string[]> {
    const params = new URLSearchParams({ cluster_id: clusterId, schema_registry_url: schemaRegistryUrl });
    const data = await this.request<{ subjects: string[] }>(`/api/schema-registry/?${params}`);
    return data.subjects;
  }

  async getSchemaSubjectVersions(clusterId: string, subject: string, schemaRegistryUrl: string): Promise<number[]> {
    const params = new URLSearchParams({ cluster_id: clusterId, schema_registry_url: schemaRegistryUrl });
    const data = await this.request<{ versions: number[] }>(`/api/schema-registry/${subject}?${params}`);
    return data.versions;
  }

  async getSchema(clusterId: string, subject: string, version: string, schemaRegistryUrl: string): Promise<import('@/types/api').SchemaInfo> {
    const params = new URLSearchParams({ cluster_id: clusterId, schema_registry_url: schemaRegistryUrl });
    return this.request(`/api/schema-registry/${subject}/${version}?${params}`);
  }

  async registerSchema(clusterId: string, schema: import('@/types/api').RegisterSchemaRequest): Promise<{ id: number; subject: string; success: boolean }> {
    return this.request('/api/schema-registry/_register', {
      method: 'POST',
      body: JSON.stringify({ cluster_id: clusterId, ...schema }),
    });
  }

  async deleteSchema(clusterId: string, subject: string, schemaRegistryUrl: string): Promise<number[]> {
    const params = new URLSearchParams({ cluster_id: clusterId, schema_registry_url: schemaRegistryUrl });
    const data = await this.request<{ deleted_versions: number[] }>(`/api/schema-registry/${subject}?${params}`);
    return data.deleted_versions;
  }

  async deleteSchemaVersion(clusterId: string, subject: string, version: string, schemaRegistryUrl: string): Promise<number> {
    const params = new URLSearchParams({ cluster_id: clusterId, schema_registry_url: schemaRegistryUrl });
    const data = await this.request<{ deleted_version: number }>(`/api/schema-registry/${subject}/${version}?${params}`);
    return data.deleted_version;
  }

  async getCompatibilityLevel(clusterId: string, schemaRegistryUrl: string): Promise<string> {
    const params = new URLSearchParams({ cluster_id: clusterId, schema_registry_url: schemaRegistryUrl });
    const data = await this.request<{ level: string }>(`/api/schema-registry/_compatibility-level?${params}`);
    return data.level;
  }

  // ==================== 用户管理 ====================
  async getUsers(): Promise<import('@/types/api').UserResponse[]> {
    return this.request('/api/users');
  }

  async createUser(user: import('@/types/api').CreateUserRequest): Promise<{ id: number; username: string }> {
    return this.request('/api/users', {
      method: 'POST',
      body: JSON.stringify(user),
    });
  }

  async updateUser(id: number, user: { email?: string; role_id?: number; is_active?: boolean }): Promise<{ success: boolean }> {
    return this.request(`/api/users/${id}`, {
      method: 'PUT',
      body: JSON.stringify(user),
    });
  }

  async updatePassword(id: number, passwords: { old_password: string; new_password: string }): Promise<{ success: boolean }> {
    return this.request(`/api/users/${id}/password`, {
      method: 'PUT',
      body: JSON.stringify(passwords),
    });
  }

  // ==================== 角色管理 ====================
  async getRoles(): Promise<import('@/types/api').RoleResponse[]> {
    return this.request('/api/roles');
  }

  async createRole(role: import('@/types/api').CreateRoleRequest): Promise<{ id: number; name: string }> {
    return this.request('/api/roles', {
      method: 'POST',
      body: JSON.stringify(role),
    });
  }

  async updateRole(id: number, role: { name?: string; description?: string; permissions?: string[] }): Promise<{ success: boolean }> {
    return this.request(`/api/roles/${id}`, {
      method: 'PUT',
      body: JSON.stringify(role),
    });
  }

  // ==================== 通知管理 ====================
  async getNotifications(): Promise<import('@/types/api').NotificationConfig[]> {
    return this.request('/api/notifications');
  }

  async createNotification(notification: import('@/types/api').CreateNotificationConfigRequest): Promise<{ id: number; success: boolean }> {
    return this.request('/api/notifications', {
      method: 'POST',
      body: JSON.stringify(notification),
    });
  }

  async getNotification(id: number): Promise<import('@/types/api').NotificationConfig> {
    return this.request(`/api/notifications/${id}`);
  }

  async deleteNotification(id: number): Promise<void> {
    return this.request(`/api/notifications/${id}`, {
      method: 'DELETE',
    });
  }

  async enableNotification(id: number): Promise<{ success: boolean }> {
    return this.request(`/api/notifications/${id}/enable`, {
      method: 'POST',
    });
  }

  async disableNotification(id: number): Promise<{ success: boolean }> {
    return this.request(`/api/notifications/${id}/disable`, {
      method: 'POST',
    });
  }

  async getAlertHistory(query?: { limit?: number; severity?: string; notified?: boolean }): Promise<import('@/types/api').AlertHistoryItem[]> {
    const params = new URLSearchParams();
    if (query?.limit) params.append('limit', query.limit.toString());
    if (query?.severity) params.append('severity', query.severity);
    if (query?.notified !== undefined) params.append('notified', query.notified.toString());
    const queryString = params.toString() ? `?${params}` : '';
    return this.request(`/api/alerts/history${queryString}`);
  }

  // ==================== 消息管理 ====================
  async getMessages(clusterId: string, topic: string, params?: {
    partition?: number;
    offset?: number;
    max_messages?: number;
    limit?: number;
    search?: string;
    start_time?: number;
    end_time?: number;
  }): Promise<import('@/types/api').MessageRecord[]> {
    const queryParams = new URLSearchParams();
    if (params?.partition !== undefined) queryParams.append('partition', params.partition.toString());
    if (params?.offset !== undefined) queryParams.append('offset', params.offset.toString());
    if (params?.max_messages) queryParams.append('max_messages', params.max_messages.toString());
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.search) queryParams.append('search', params.search);
    if (params?.start_time) queryParams.append('start_time', params.start_time.toString());
    if (params?.end_time) queryParams.append('end_time', params.end_time.toString());
    const queryString = queryParams.toString();
    const url = queryString ? `?${queryString}` : '';
    // getMessages 不使用默认的 AbortController，由调用者自行管理
    const controller = new AbortController();
    this.currentAbortController = controller;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    const response = await fetch(`${this.baseURL}/api/clusters/${clusterId}/${topic}/messages${url}`, {
      headers,
      signal: controller.signal,
    });

    if (!response.ok) {
      let message = `HTTP ${response.status}: ${response.statusText}`;
      try {
        const errorData = await response.json();
        message = errorData.message || message;
      } catch {
        // 忽略解析错误
      }
      throw { message, status: response.status } as ApiError;
    }

    const data = await response.json();
    return data.messages;
  }

  // 取消 getMessages 请求
  cancelGetMessages() {
    if (this.currentAbortController) {
      this.currentAbortController.abort();
      this.currentAbortController = null;
    }
  }

  async sendMessage(clusterId: string, topic: string, message: import('@/types/api').SendMessageRequest): Promise<import('@/types/api').SendMessageResponse> {
    return this.request(`/api/clusters/${clusterId}/${topic}/messages`, {
      method: 'POST',
      body: JSON.stringify(message),
    });
  }

  async exportMessages(clusterId: string, topic: string, params?: {
    format?: 'json' | 'csv' | 'text';
    partition?: number;
    max_messages?: number;
    search?: string;
    fetchMode?: 'oldest' | 'newest';
    start_time?: number;
    end_time?: number;
  }): Promise<Blob> {
    const queryParams = new URLSearchParams();
    if (params?.format) queryParams.append('format', params.format);
    if (params?.partition !== undefined) queryParams.append('partition', params.partition.toString());
    if (params?.max_messages) queryParams.append('max_messages', params.max_messages.toString());
    if (params?.search) queryParams.append('search', params.search);
    if (params?.fetchMode) queryParams.append('fetch_mode', params.fetchMode);
    if (params?.start_time) queryParams.append('start_time', params.start_time.toString());
    if (params?.end_time) queryParams.append('end_time', params.end_time.toString());
    const queryString = queryParams.toString();
    const response = await fetch(`${this.baseURL}/api/clusters/${clusterId}/${topic}/messages/_export${queryString ? `?${queryString}` : ''}`, {
      headers: this.apiKey ? { 'X-API-Key': this.apiKey } : {},
    });

    if (!response.ok) {
      throw { message: `HTTP ${response.status}: ${response.statusText}`, status: response.status };
    }

    return response.blob();
  }

  // ==================== 集群监控 ====================
  async getClusterInfo(clusterId: string): Promise<import('@/types/api').ClusterInfoResponse> {
    return this.request(`/api/clusters/${clusterId}/info`);
  }

  async getClusterMetrics(clusterId: string): Promise<import('@/types/api').ClusterMetricsResponse> {
    return this.request(`/api/clusters/${clusterId}/metrics`);
  }

  async getBrokers(clusterId: string): Promise<import('@/types/api').BrokerInfo[]> {
    const data = await this.request<{ brokers: import('@/types/api').BrokerInfo[] }>(`/api/clusters/${clusterId}/brokers`);
    return data.brokers;
  }

  async getBrokerDetail(clusterId: string, brokerId: number): Promise<import('@/types/api').BrokerDetailResponse> {
    return this.request(`/api/clusters/${clusterId}/brokers/${brokerId}`);
  }

  // ==================== 集群连接管理 ====================
  async getAllConnectionStatus(): Promise<{ connections: { cluster_id: string; status: string; error_message?: string }[] }> {
    return this.request('/api/cluster-connections/');
  }

  async getConnectionStatus(clusterId: string): Promise<{ cluster_id: string; status: string; error_message?: string }> {
    return this.request(`/api/cluster-connections/${encodeURIComponent(clusterId)}/status`);
  }

  async disconnectCluster(clusterId: string): Promise<{ success: boolean; message: string }> {
    return this.request(`/api/cluster-connections/${encodeURIComponent(clusterId)}/disconnect`, {
      method: 'POST',
    });
  }

  async reconnectCluster(clusterId: string): Promise<{ success: boolean; message: string }> {
    return this.request(`/api/cluster-connections/${encodeURIComponent(clusterId)}/reconnect`, {
      method: 'POST',
    });
  }

  async healthCheckCluster(clusterId: string): Promise<{ cluster_id: string; healthy: boolean; status: string; error_message?: string }> {
    return this.request(`/api/cluster-connections/${encodeURIComponent(clusterId)}/health-check`, {
      method: 'POST',
    });
  }

  async getConnectionMetrics(clusterId: string): Promise<{ cluster_id: string; consumer_pool_size: number; producer_pool_size: number; consumer_pool_available: number; producer_pool_available: number }> {
    return this.request(`/api/cluster-connections/${encodeURIComponent(clusterId)}/metrics`);
  }

  async getConnectionHistory(clusterId: string, limit?: number): Promise<{ cluster_id: string; history: { status: string; error_message?: string; latency_ms?: number; checked_at: string }[] }> {
    const params = new URLSearchParams();
    if (limit) params.append('limit', limit.toString());
    const query = params.toString() ? `?${params}` : '';
    return this.request(`/api/cluster-connections/${encodeURIComponent(clusterId)}/history${query}`);
  }

  async getConnectionStats(clusterId: string): Promise<{ cluster_id: string; total_checks: number; successful_checks: number; failed_checks: number; success_rate: number; avg_latency_ms?: number; last_status: string; last_checked_at?: string }> {
    return this.request(`/api/cluster-connections/${encodeURIComponent(clusterId)}/stats`);
  }

  async batchDisconnect(clusterNames: string[]): Promise<{ total: number; successful: number; failed: number; results: { cluster_name: string; success: boolean; message?: string }[] }> {
    return this.request('/api/cluster-connections/batch/disconnect', {
      method: 'POST',
      body: JSON.stringify({ cluster_names: clusterNames }),
    });
  }

  async batchReconnect(clusterNames: string[]): Promise<{ total: number; successful: number; failed: number; results: { cluster_name: string; success: boolean; message?: string }[] }> {
    return this.request('/api/cluster-connections/batch/reconnect', {
      method: 'POST',
      body: JSON.stringify({ cluster_names: clusterNames }),
    });
  }

  // ==================== 全局设置 ====================
  async getSettings(keys?: string[]): Promise<{ key: string; value: string }[]> {
    const params = new URLSearchParams();
    if (keys && keys.length > 0) {
      params.append('keys', keys.join(','));
    }
    const query = params.toString() ? `?${params}` : '';
    const data = await this.request<{ settings: { key: string; value: string }[] }>(`/api/settings${query}`);
    return data.settings;
  }

  async updateSetting(key: string, value: string): Promise<{ key: string; value: string }> {
    return this.request('/api/settings', {
      method: 'PUT',
      body: JSON.stringify({ key, value }),
    });
  }
}

// 导出单例
export const apiClient = new ApiClient();

// 用于测试环境设置 API Key
export function setupApiClient(baseURL?: string, apiKey?: string) {
  return new ApiClient(baseURL, apiKey);
}
