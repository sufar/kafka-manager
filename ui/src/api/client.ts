import type {
  Cluster,
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
  ConsumerOffsetsListResponse,
} from '@/types/api';

// 检测是否在 Tauri 环境下运行
function isTauri(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }
  const win = window as any;
  return !!(
    win.__TAURI__ ||
    win.__TAURI_INTERNALS__ ||
    win.__TAURI_IPC__ ||
    win._TAURI_VERSION_ ||
    win.navigator?.userAgent?.includes('Tauri')
  );
}

// 获取 API 基础 URL
function getBaseURL(): string {
  if (isTauri()) {
    return 'http://localhost:9732';
  }
  return '';
}

class ApiClient {
  private baseURL: string;
  private apiKey: string | null;
  private currentAbortController: AbortController | null = null;
  private backendReady: boolean | null = null;

  constructor(baseURL: string = getBaseURL(), apiKey: string | null = null) {
    this.baseURL = baseURL;
    this.apiKey = apiKey;
  }

  setApiKey(apiKey: string) {
    this.apiKey = apiKey;
  }

  // 检查后端是否就绪
  async checkBackendReady(): Promise<boolean> {
    if (this.backendReady === true) {
      return true;
    }

    try {
      const response = await this.request('health', {});
      if (response) {
        this.backendReady = true;
        return true;
      }
    } catch (e) {
      console.warn('[ApiClient] Backend health check failed:', e);
    }

    this.backendReady = false;
    return false;
  }

  // 取消当前的 API 请求
  cancelRequest() {
    if (this.currentAbortController) {
      this.currentAbortController.abort();
      this.currentAbortController = null;
    }
  }

  // 统一请求方法
  private async request<T>(method: string, body: Record<string, any>): Promise<T> {
    this.currentAbortController = new AbortController();
    const { signal } = this.currentAbortController;

    const url = `${this.baseURL}/api`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'X-API-Method': method,
    };

    if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    // 在 Tauri 环境下，添加重试逻辑
    const maxRetries = isTauri() ? 5 : 1;
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
      try {
        const response = await fetch(url, {
          method: 'POST',
          headers,
          body: JSON.stringify(body),
          signal,
        });

        if (!response.ok) {
          let message = `HTTP ${response.status}: ${response.statusText}`;
          try {
            const errorData = await response.json();
            message = errorData.error || message;
          } catch {
            // 忽略解析错误
          }
          throw { message, status: response.status } as ApiError;
        }

        const data = await response.json();

        if (!data.success) {
          throw { message: data.error || 'Unknown error', status: response.status } as ApiError;
        }

        return data.data as T;
      } catch (e) {
        const error = e as { message?: string; type?: string };
        if (isTauri() && attempt < maxRetries - 1) {
          if (error.message?.includes('Failed to fetch') || error.message?.includes('NetworkError') || error.type === 'TypeError') {
            await new Promise(resolve => setTimeout(resolve, 500 * (attempt + 1)));
            lastError = e as Error;
            continue;
          }
        }
        throw e;
      }
    }

    if (lastError) {
      throw lastError;
    }

    throw new Error('Request failed');
  }

  // ==================== Health ====================
  async health(): Promise<HealthResponse> {
    return this.request('health', {});
  }

  // ==================== Clusters ====================
  async getClusters(): Promise<Cluster[]> {
    return this.request('cluster.list', {});
  }

  async getCluster(id: number): Promise<Cluster> {
    return this.request('cluster.get', { id });
  }

  async createCluster(cluster: CreateClusterRequest): Promise<Cluster> {
    if (isTauri()) {
      const ready = await this.checkBackendReady();
      if (!ready) {
        throw new Error('后端服务未就绪，请等待应用完全启动后再试（约 5-10 秒）');
      }
    }
    return this.request('cluster.create', cluster);
  }

  async updateCluster(id: number, cluster: UpdateClusterRequest): Promise<Cluster> {
    return this.request('cluster.update', { id, ...cluster });
  }

  async deleteCluster(id: number): Promise<void> {
    return this.request('cluster.delete', { id });
  }

  async testCluster(id: number): Promise<TestConnectionResponse> {
    return this.request('cluster.test', { id });
  }

  // ==================== Topics ====================
  async getTopics(clusterId: string): Promise<string[]> {
    const data = await this.request<TopicListResponse>('topic.list', { cluster: clusterId });
    return data.topics;
  }

  async getTopicDetail(clusterId: string, topicName: string): Promise<TopicDetailResponse> {
    return this.request('topic.get', { cluster: clusterId, topic: topicName });
  }

  async createTopic(clusterId: string, topic: CreateTopicRequest): Promise<CreateTopicResponse> {
    return this.request('topic.create', { cluster: clusterId, ...topic });
  }

  async batchCreateTopics(
    clusterId: string,
    topics: BatchCreateTopicsRequest
  ): Promise<BatchCreateTopicsResponse> {
    return this.request('topic.batch_create', { cluster: clusterId, ...topics });
  }

  async deleteTopic(clusterId: string, topicName: string): Promise<void> {
    return this.request('topic.delete', { cluster: clusterId, topic: topicName });
  }

  async batchDeleteTopics(
    clusterId: string,
    topics: BatchDeleteTopicsRequest
  ): Promise<BatchDeleteTopicsResponse> {
    return this.request('topic.batch_delete', { cluster: clusterId, ...topics });
  }

  async getTopicOffsets(clusterId: string, topicName: string): Promise<TopicPartitionDetail[]> {
    return this.request('topic.offsets', { cluster: clusterId, topic: topicName });
  }

  async addPartitions(clusterId: string, topicName: string, newPartitions: number): Promise<void> {
    return this.request('topic.partitions.add', { cluster: clusterId, topic: topicName, new_partitions: newPartitions });
  }

  async refreshTopics(clusterId: string): Promise<{ success: boolean; added: string[]; removed: string[]; total: number }> {
    return this.request('topic.refresh', { cluster: clusterId });
  }

  async searchTopics(): Promise<{ cluster: string; topic: string }[]> {
    const data = await this.request<{ results: { cluster: string; topic: string }[] }>('topic.search', {});
    return data.results || [];
  }

  async getTopicCount(clusterId: string): Promise<number> {
    const data = await this.request<{ count: number }>('topic.count', { cluster: clusterId });
    return data.count;
  }

  async getTopicConfig(clusterId: string, topicName: string): Promise<Record<string, string>> {
    return this.request('topic.config.get', { cluster: clusterId, topic: topicName });
  }

  async alterTopicConfig(
    clusterId: string,
    topicName: string,
    config: Record<string, string>
  ): Promise<void> {
    return this.request('topic.config.alter', { cluster: clusterId, topic: topicName, config });
  }

  // ==================== Consumer Groups ====================
  async getConsumerGroups(clusterId: string): Promise<ConsumerGroupSummary[]> {
    const data = await this.request<ConsumerGroupListResponse>('consumer_group.list', { cluster: clusterId });
    return data.groups;
  }

  async getAllConsumerOffsets(clusterId: string): Promise<ConsumerOffsetsListResponse> {
    return this.request('consumer_group.consumer_offsets', { cluster: clusterId });
  }

  async getConsumerGroupDetail(
    clusterId: string,
    groupName: string
  ): Promise<ConsumerGroupDetailResponse> {
    return this.request('consumer_group.get', { cluster: clusterId, group: groupName });
  }

  async getConsumerGroupOffsets(
    clusterId: string,
    groupName: string,
    topic?: string
  ): Promise<ConsumerGroupOffsetDetailResponse> {
    return this.request('consumer_group.offsets', { cluster: clusterId, group: groupName, topic });
  }

  async resetConsumerGroupOffset(
    clusterId: string,
    groupName: string,
    request: ResetConsumerGroupOffsetRequest
  ): Promise<void> {
    return this.request('consumer_group.offsets.reset', {
      cluster: clusterId,
      group: groupName,
      ...request
    });
  }

  async deleteConsumerGroup(clusterId: string, groupName: string): Promise<void> {
    return this.request('consumer_group.delete', { cluster: clusterId, group: groupName });
  }

  async batchDeleteConsumerGroups(
    clusterId: string,
    request: BatchDeleteConsumerGroupsRequest
  ): Promise<BatchDeleteConsumerGroupsResponse> {
    return this.request('consumer_group.batch_delete', { cluster: clusterId, ...request });
  }

  // ==================== Cluster Stats ====================
  async getClusterStats(clusterId: string): Promise<ClusterStatsResponse> {
    return this.request('cluster.stats', { cluster: clusterId });
  }

  // ==================== Schema Registry ====================
  async getSchemaSubjects(clusterId: string, schemaRegistryUrl: string): Promise<string[]> {
    const data = await this.request<{ subjects: string[] }>('schema.subjects', {
      cluster: clusterId,
      schema_registry_url: schemaRegistryUrl
    });
    return data.subjects;
  }

  async getSchemaSubjectVersions(clusterId: string, subject: string, schemaRegistryUrl: string): Promise<number[]> {
    const data = await this.request<{ versions: number[] }>('schema.versions', {
      cluster: clusterId,
      subject,
      schema_registry_url: schemaRegistryUrl
    });
    return data.versions;
  }

  async getSchema(clusterId: string, subject: string, version: string, schemaRegistryUrl: string): Promise<import('@/types/api').SchemaInfo> {
    return this.request('schema.get', {
      cluster: clusterId,
      subject,
      version,
      schema_registry_url: schemaRegistryUrl
    });
  }

  async registerSchema(clusterId: string, schema: import('@/types/api').RegisterSchemaRequest): Promise<{ id: number; subject: string; success: boolean }> {
    return this.request('schema.register', { cluster: clusterId, ...schema });
  }

  async deleteSchema(clusterId: string, subject: string, schemaRegistryUrl: string): Promise<number[]> {
    const data = await this.request<{ deleted_versions: number[] }>('schema.delete', {
      cluster: clusterId,
      subject,
      schema_registry_url: schemaRegistryUrl
    });
    return data.deleted_versions;
  }

  async deleteSchemaVersion(clusterId: string, subject: string, version: string, schemaRegistryUrl: string): Promise<number> {
    const data = await this.request<{ deleted_version: number }>('schema.version.delete', {
      cluster: clusterId,
      subject,
      version,
      schema_registry_url: schemaRegistryUrl
    });
    return data.deleted_version;
  }

  async getCompatibilityLevel(clusterId: string, schemaRegistryUrl: string): Promise<string> {
    const data = await this.request<{ level: string }>('schema.compatibility_level', {
      cluster: clusterId,
      schema_registry_url: schemaRegistryUrl
    });
    return data.level;
  }

  // ==================== 用户管理 ====================
  async getUsers(): Promise<import('@/types/api').UserResponse[]> {
    return this.request('user.list', {});
  }

  async createUser(user: import('@/types/api').CreateUserRequest): Promise<{ id: number; username: string }> {
    return this.request('user.create', user);
  }

  async updateUser(id: number, user: { email?: string; role_id?: number; is_active?: boolean }): Promise<{ success: boolean }> {
    return this.request('user.update', { id, ...user });
  }

  async updatePassword(id: number, passwords: { old_password: string; new_password: string }): Promise<{ success: boolean }> {
    return this.request('user.password.update', { id, ...passwords });
  }

  // ==================== 角色管理 ====================
  async getRoles(): Promise<import('@/types/api').RoleResponse[]> {
    return this.request('role.list', {});
  }

  async createRole(role: import('@/types/api').CreateRoleRequest): Promise<{ id: number; name: string }> {
    return this.request('role.create', role);
  }

  async updateRole(id: number, role: { name?: string; description?: string; permissions?: string[] }): Promise<{ success: boolean }> {
    return this.request('role.update', { id, ...role });
  }

  // ==================== 通知管理 ====================
  async getNotifications(): Promise<import('@/types/api').NotificationConfig[]> {
    return this.request('notification.list', {});
  }

  async createNotification(notification: import('@/types/api').CreateNotificationConfigRequest): Promise<{ id: number; success: boolean }> {
    return this.request('notification.create', notification);
  }

  async getNotification(id: number): Promise<import('@/types/api').NotificationConfig> {
    return this.request('notification.get', { id });
  }

  async deleteNotification(id: number): Promise<void> {
    return this.request('notification.delete', { id });
  }

  async enableNotification(id: number): Promise<{ success: boolean }> {
    return this.request('notification.enable', { id });
  }

  async disableNotification(id: number): Promise<{ success: boolean }> {
    return this.request('notification.disable', { id });
  }

  async getAlertHistory(query?: { limit?: number; severity?: string; notified?: boolean }): Promise<import('@/types/api').AlertHistoryItem[]> {
    return this.request('alert.history', query || {});
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
    fetchMode?: 'oldest' | 'newest';
  }): Promise<import('@/types/api').MessageRecord[]> {
    const data = await this.request<{ messages: import('@/types/api').MessageRecord[] }>('message.list', {
      cluster: clusterId,
      topic,
      ...params
    });
    return data.messages;
  }

  cancelGetMessages() {
    this.cancelRequest();
  }

  async sendMessage(clusterId: string, topic: string, message: import('@/types/api').SendMessageRequest): Promise<import('@/types/api').SendMessageResponse> {
    return this.request('message.send', {
      cluster: clusterId,
      topic,
      ...message
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
    // 导出功能需要通过原始URL获取
    const queryParams = new URLSearchParams();
    if (params?.format) queryParams.append('format', params.format);
    if (params?.partition !== undefined) queryParams.append('partition', params.partition.toString());
    if (params?.max_messages) queryParams.append('max_messages', params.max_messages.toString());
    if (params?.search) queryParams.append('search', params.search);
    if (params?.fetchMode) queryParams.append('fetch_mode', params.fetchMode);
    if (params?.start_time) queryParams.append('start_time', params.start_time.toString());
    if (params?.end_time) queryParams.append('end_time', params.end_time.toString());

    const queryString = queryParams.toString();
    const url = `${this.baseURL}/api/clusters/${clusterId}/${topic}/messages/_export${queryString ? `?${queryString}` : ''}`;

    const response = await fetch(url, {
      headers: this.apiKey ? { 'X-API-Key': this.apiKey } : {},
    });

    if (!response.ok) {
      throw { message: `HTTP ${response.status}: ${response.statusText}`, status: response.status };
    }

    return response.blob();
  }

  // ==================== 集群监控 ====================
  async getClusterInfo(clusterId: string): Promise<import('@/types/api').ClusterInfoResponse> {
    return this.request('cluster.info', { cluster: clusterId });
  }

  async getClusterMetrics(clusterId: string): Promise<import('@/types/api').ClusterMetricsResponse> {
    return this.request('cluster.metrics', { cluster: clusterId });
  }

  async getBrokers(clusterId: string): Promise<import('@/types/api').BrokerInfo[]> {
    const data = await this.request<{ brokers: import('@/types/api').BrokerInfo[] }>('cluster.brokers', { cluster: clusterId });
    return data.brokers;
  }

  async getBrokerDetail(clusterId: string, brokerId: number): Promise<import('@/types/api').BrokerDetailResponse> {
    return this.request('cluster.broker.get', { cluster: clusterId, broker_id: brokerId });
  }

  // ==================== 集群连接管理 ====================
  async getAllConnectionStatus(): Promise<{ connections: { cluster_id: string; status: string; error_message?: string }[] }> {
    return this.request('cluster_connection.list', {});
  }

  async getConnectionStatus(clusterId: string): Promise<{ cluster_id: string; status: string; error_message?: string }> {
    return this.request('cluster_connection.get', { cluster: clusterId });
  }

  async disconnectCluster(clusterId: string): Promise<{ success: boolean; message: string }> {
    return this.request('cluster_connection.disconnect', { cluster: clusterId });
  }

  async reconnectCluster(clusterId: string): Promise<{ success: boolean; message: string }> {
    return this.request('cluster_connection.reconnect', { cluster: clusterId });
  }

  async healthCheckCluster(clusterId: string): Promise<{ cluster_id: string; healthy: boolean; status: string; error_message?: string }> {
    return this.request('cluster_connection.health_check', { cluster: clusterId });
  }

  async getConnectionMetrics(clusterId: string): Promise<{ cluster_id: string; consumer_pool_size: number; producer_pool_size: number; consumer_pool_available: number; producer_pool_available: number }> {
    return this.request('cluster_connection.metrics', { cluster: clusterId });
  }

  async getConnectionHistory(clusterId: string, limit?: number): Promise<{ cluster_id: string; history: { status: string; error_message?: string; latency_ms?: number; checked_at: string }[] }> {
    return this.request('cluster_connection.history', { cluster: clusterId, limit });
  }

  async getConnectionStats(clusterId: string): Promise<{ cluster_id: string; total_checks: number; successful_checks: number; failed_checks: number; success_rate: number; avg_latency_ms?: number; last_status: string; last_checked_at?: string }> {
    return this.request('cluster_connection.stats', { cluster: clusterId });
  }

  async batchDisconnect(clusterNames: string[]): Promise<{ total: number; successful: number; failed: number; results: { cluster_name: string; success: boolean; message?: string }[] }> {
    return this.request('cluster_connection.batch_disconnect', { cluster_names: clusterNames });
  }

  async batchReconnect(clusterNames: string[]): Promise<{ total: number; successful: number; failed: number; results: { cluster_name: string; success: boolean; message?: string }[] }> {
    return this.request('cluster_connection.batch_reconnect', { cluster_names: clusterNames });
  }

  // ==================== 全局设置 ====================
  async getSettings(keys?: string[]): Promise<{ key: string; value: string }[]> {
    const data = await this.request<{ settings: { key: string; value: string }[] }>('settings.get', { keys });
    return data.settings;
  }

  async updateSetting(key: string, value: string): Promise<{ key: string; value: string }> {
    return this.request('settings.update', { key, value });
  }
}

// 导出单例
export const apiClient = new ApiClient();

// 用于测试环境设置 API Key
export function setupApiClient(baseURL?: string, apiKey?: string) {
  return new ApiClient(baseURL, apiKey);
}
