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

    // 在 Tauri 环境下，增加重试次数和超时时间
    const maxRetries = isTauri() ? 60 : 1; // Tauri 下最多尝试 60 次（30 秒）
    const retryDelay = 500; // 每次等待 500ms

    for (let i = 0; i < maxRetries; i++) {
      try {
        const response = await this.request('health', {});
        if (response) {
          this.backendReady = true;
          return true;
        }
      } catch (e) {
        const error = e as { message?: string };
        console.warn(`[ApiClient] Backend health check attempt ${i + 1}/${maxRetries} failed:`, error.message || e);

        // 如果不是最后一次，等待后重试
        if (i < maxRetries - 1) {
          await new Promise(resolve => setTimeout(resolve, retryDelay));
        }
      }
    }

    console.error('[ApiClient] Backend health check failed after all retries');
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
  private async request<T>(method: string, body: Record<string, any>, timeoutMs?: number): Promise<T> {
    this.currentAbortController = new AbortController();
    const { signal } = this.currentAbortController;

    // 设置请求超时（默认 30 秒，消息查询可能需要更长时间）
    const requestTimeout = timeoutMs || 30000;
    const timeoutId = setTimeout(() => {
      if (this.currentAbortController) {
        this.currentAbortController.abort();
      }
    }, requestTimeout);

    const url = `${this.baseURL}/api`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'X-API-Method': method,
    };

    if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    // 在 Tauri 环境下，添加更多重试
    const maxRetries = isTauri() ? 3 : 1; // Tauri 下最多重试 3 次（原来是 10 次）
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
          clearTimeout(timeoutId);
          throw { message, status: response.status } as ApiError;
        }

        const data = await response.json();

        if (!data.success) {
          clearTimeout(timeoutId);
          throw { message: data.error || 'Unknown error', status: response.status } as ApiError;
        }

        clearTimeout(timeoutId);
        return data.data as T;
      } catch (e) {
        const error = e as { message?: string; type?: string; name?: string };
        // 处理超时错误
        if (error.name === 'AbortError') {
          clearTimeout(timeoutId);
          throw { message: `请求超时（${requestTimeout / 1000}秒），请检查网络连接或减少查询数据量`, status: 408 } as ApiError;
        }
        if (isTauri() && attempt < maxRetries - 1) {
          if (error.message?.includes('Failed to fetch') ||
              error.message?.includes('NetworkError') ||
              error.message?.includes('fetch') ||
              error.type === 'TypeError') {
            await new Promise(resolve => setTimeout(resolve, 100)); // 从 1 秒减少到 100ms
            lastError = e as Error;
            continue;
          }
        }
        clearTimeout(timeoutId);
        throw e;
      }
    }

    clearTimeout(timeoutId);
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
    const data = await this.request<TopicListResponse>('topic.list', { cluster_id: clusterId });
    return data.topics;
  }

  async getTopicDetail(clusterId: string, topicName: string): Promise<TopicDetailResponse> {
    return this.request('topic.get', { cluster_id: clusterId, name: topicName });
  }

  async createTopic(clusterId: string, topic: CreateTopicRequest): Promise<CreateTopicResponse> {
    return this.request('topic.create', { cluster_id: clusterId, ...topic });
  }

  async batchCreateTopics(
    clusterId: string,
    topics: BatchCreateTopicsRequest
  ): Promise<BatchCreateTopicsResponse> {
    return this.request('topic.batch_create', { cluster_id: clusterId, ...topics });
  }

  async deleteTopic(clusterId: string, topicName: string): Promise<void> {
    return this.request('topic.delete', { cluster_id: clusterId, topic: topicName });
  }

  async batchDeleteTopics(
    clusterId: string,
    topics: BatchDeleteTopicsRequest
  ): Promise<BatchDeleteTopicsResponse> {
    return this.request('topic.batch_delete', { cluster_id: clusterId, ...topics });
  }

  async getTopicOffsets(clusterId: string, topicName: string): Promise<TopicPartitionDetail[]> {
    return this.request('topic.offsets', { cluster_id: clusterId, topic: topicName });
  }

  async addPartitions(clusterId: string, topicName: string, newPartitions: number): Promise<void> {
    return this.request('topic.partitions.add', { cluster_id: clusterId, topic: topicName, new_partitions: newPartitions });
  }

  async refreshTopics(clusterId: string): Promise<{ success: boolean; added: string[]; removed: string[]; total: number }> {
    return this.request('topic.refresh', { cluster_id: clusterId });
  }

  async getSavedTopics(clusterId: string): Promise<string[]> {
    const data = await this.request<{ topics: string[] }>('topic.saved', { cluster_id: clusterId });
    return data.topics || [];
  }

  async searchTopics(keyword?: string): Promise<{ cluster: string; topic: string }[]> {
    const params: Record<string, any> = {};
    if (keyword && keyword.trim()) {
      params.keyword = keyword.trim();
    }
    const data = await this.request<{ results: { cluster: string; topic: string }[] }>('topic.search', params);
    return data.results || [];
  }

  async getTopicCount(clusterId: string): Promise<number> {
    const data = await this.request<{ count: number }>('topic.count', { cluster_id: clusterId });
    return data.count;
  }

  async getTopicConfig(clusterId: string, topicName: string): Promise<Record<string, string>> {
    return this.request('topic.config.get', { cluster_id: clusterId, topic: topicName });
  }

  async alterTopicConfig(
    clusterId: string,
    topicName: string,
    config: Record<string, string>
  ): Promise<void> {
    return this.request('topic.config.alter', { cluster_id: clusterId, topic: topicName, config });
  }

  async updateTopicConfig(
    clusterId: string,
    topicName: string,
    config: Record<string, string>
  ): Promise<void> {
    return this.request('topic.config.alter', { cluster_id: clusterId, topic: topicName, config });
  }

  async getPartitionWatermarks(clusterId: string, topicName: string, partition: number): Promise<{ lowOffset: number; highOffset: number }> {
    return this.request('topic.partition.watermarks', { cluster_id: clusterId, topic: topicName, partition });
  }

  // Topic 标签管理
  async getTopicTags(clusterId: string, topicName: string): Promise<Array<{ id: number; name: string; color: string }>> {
    const data = await this.request<{ tags: Array<{ id: number; name: string; color: string }> }>('tag.list', {
      cluster_id: clusterId,
      resource_type: 'topic',
      resource_name: topicName
    });
    return data.tags || [];
  }

  async createTopicTag(clusterId: string, topicName: string, tagName: string, color: string): Promise<{ id: number; name: string; color: string }> {
    return this.request('tag.create', {
      cluster_id: clusterId,
      resource_type: 'topic',
      resource_name: topicName,
      tag_name: tagName,
      color
    });
  }

  async deleteTopicTag(clusterId: string, topicName: string, tagId: number): Promise<void> {
    return this.request('tag.delete', {
      cluster_id: clusterId,
      resource_type: 'topic',
      resource_name: topicName,
      tag_id: tagId
    });
  }

  // ==================== Consumer Groups ====================
  async getConsumerGroups(clusterId: string): Promise<ConsumerGroupSummary[]> {
    const data = await this.request<ConsumerGroupListResponse>('consumer_group.list', { cluster_id: clusterId });
    return data.groups;
  }

  async getAllConsumerOffsets(clusterId: string): Promise<ConsumerOffsetsListResponse> {
    return this.request('consumer_group.consumer_offsets', { cluster_id: clusterId });
  }

  async getConsumerGroupDetail(
    clusterId: string,
    groupName: string
  ): Promise<ConsumerGroupDetailResponse> {
    return this.request('consumer_group.get', { cluster_id: clusterId, name: groupName });
  }

  async getConsumerGroupOffsets(
    clusterId: string,
    groupName: string,
    topic?: string
  ): Promise<ConsumerGroupOffsetDetailResponse> {
    return this.request('consumer_group.offsets', { cluster_id: clusterId, group_name: groupName, topic });
  }

  async resetConsumerGroupOffset(
    clusterId: string,
    groupName: string,
    request: ResetConsumerGroupOffsetRequest
  ): Promise<void> {
    return this.request('consumer_group.offsets.reset', {
      cluster_id: clusterId,
      group_name: groupName,
      ...request
    });
  }

  async deleteConsumerGroup(clusterId: string, groupName: string): Promise<void> {
    return this.request('consumer_group.delete', { cluster_id: clusterId, name: groupName });
  }

  async batchDeleteConsumerGroups(
    clusterId: string,
    request: BatchDeleteConsumerGroupsRequest
  ): Promise<BatchDeleteConsumerGroupsResponse> {
    return this.request('consumer_group.batch_delete', { cluster_id: clusterId, ...request });
  }

  async getConsumerLag(clusterId: string, topic: string): Promise<{
    topic: string;
    total_lag: number;
    consumer_groups: {
      name: string;
      total_lag: number;
      partitions: {
        partition: number;
        current_offset: number;
        log_end_offset: number;
        lag: number;
        state: string;
      }[];
    }[];
  }> {
    return this.request('consumer_lag.get', { cluster_id: clusterId, topic });
  }

  async getConsumerLagHistory(clusterId: string, topic: string): Promise<{
    topic: string;
    start_time: number;
    end_time: number;
    data_points: number;
    timestamps: number[];
    consumer_groups: {
      name: string;
      lag_series: number[];
      consumed_series: number[];
      produced_series: number[];
    }[];
  }> {
    return this.request('consumer_lag.history', { cluster_id: clusterId, topic });
  }

  // ==================== Cluster Stats ====================
  async getClusterStats(clusterId: string): Promise<ClusterStatsResponse> {
    return this.request('cluster.stats', { cluster_id: clusterId });
  }

  // ==================== Schema Registry ====================
  async getSchemaSubjects(clusterId: string, schemaRegistryUrl: string): Promise<string[]> {
    const data = await this.request<{ subjects: string[] }>('schema.subjects', {
      cluster_id: clusterId,
      schema_registry_url: schemaRegistryUrl
    });
    return data.subjects;
  }

  async getSchemaSubjectVersions(clusterId: string, subject: string, schemaRegistryUrl: string): Promise<number[]> {
    const data = await this.request<{ versions: number[] }>('schema.versions', {
      cluster_id: clusterId,
      subject,
      schema_registry_url: schemaRegistryUrl
    });
    return data.versions;
  }

  async getSchema(clusterId: string, subject: string, version: string, schemaRegistryUrl: string): Promise<import('@/types/api').SchemaInfo> {
    return this.request('schema.get', {
      cluster_id: clusterId,
      subject,
      version,
      schema_registry_url: schemaRegistryUrl
    });
  }

  async registerSchema(clusterId: string, schema: import('@/types/api').RegisterSchemaRequest): Promise<{ id: number; subject: string; success: boolean }> {
    return this.request('schema.register', { cluster_id: clusterId, ...schema });
  }

  async deleteSchema(clusterId: string, subject: string, schemaRegistryUrl: string): Promise<number[]> {
    const data = await this.request<{ deleted_versions: number[] }>('schema.delete', {
      cluster_id: clusterId,
      subject,
      schema_registry_url: schemaRegistryUrl
    });
    return data.deleted_versions;
  }

  async deleteSchemaVersion(clusterId: string, subject: string, version: string, schemaRegistryUrl: string): Promise<number> {
    const data = await this.request<{ deleted_version: number }>('schema.version.delete', {
      cluster_id: clusterId,
      subject,
      version,
      schema_registry_url: schemaRegistryUrl
    });
    return data.deleted_version;
  }

  async getCompatibilityLevel(clusterId: string, schemaRegistryUrl: string): Promise<string> {
    const data = await this.request<{ level: string }>('schema.compatibility_level', {
      cluster_id: clusterId,
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
    max_messages?: number;  // 每个分区最大获取消息数（从Kafka获取的最大数量）
    order_by?: 'timestamp' | 'offset';
    sort?: 'asc' | 'desc';
    search?: string;
    search_in?: 'key' | 'value' | 'all';
    start_time?: number;
    end_time?: number;
    fetchMode?: 'oldest' | 'newest';
  }): Promise<import('@/types/api').MessageRecord[]> {
    // 消息查询可能需要较长时间，设置 60 秒超时
    const data = await this.request<{ messages: import('@/types/api').MessageRecord[] }>('message.list', {
      cluster_id: clusterId,
      topic,
      ...params
    }, 60000);
    return data.messages;
  }

  cancelGetMessages() {
    this.cancelRequest();
  }

  async sendMessage(clusterId: string, topic: string, message: import('@/types/api').SendMessageRequest): Promise<import('@/types/api').SendMessageResponse> {
    return this.request('message.send', {
      cluster_id: clusterId,
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
  }): Promise<{ topic: string; format: string; messages: any[]; count: number }> {
    return this.request('message.export', {
      cluster_id: clusterId,
      topic,
      partition: params?.partition,
      max_messages: params?.max_messages,
      search: params?.search,
      fetch_mode: params?.fetchMode,
      start_time: params?.start_time,
      end_time: params?.end_time,
    });
  }
  // ==================== 集群监控 ====================
  async getClusterInfo(clusterId: string): Promise<import('@/types/api').ClusterInfoResponse> {
    return this.request('cluster.info', { cluster_id: clusterId });
  }

  async getClusterMetrics(clusterId: string): Promise<import('@/types/api').ClusterMetricsResponse> {
    return this.request('cluster.metrics', { cluster_id: clusterId });
  }

  async getBrokers(clusterId: string): Promise<import('@/types/api').BrokerInfo[]> {
    const data = await this.request<{ brokers: import('@/types/api').BrokerInfo[] }>('cluster.brokers', { cluster_id: clusterId });
    return data.brokers;
  }

  async getBrokerDetail(clusterId: string, brokerId: number): Promise<import('@/types/api').BrokerDetailResponse> {
    return this.request('cluster.broker.get', { cluster_id: clusterId, broker_id: brokerId });
  }

  // ==================== 集群连接管理 ====================
  async getAllConnectionStatus(): Promise<{ connections: { cluster_id: string; status: string; error_message?: string }[] }> {
    return this.request('connection.list', {});
  }

  async getConnectionStatus(clusterId: string): Promise<{ cluster_id: string; status: string; error_message?: string }> {
    return this.request('connection.get', { cluster_id: clusterId });
  }

  async disconnectCluster(clusterId: string): Promise<{ success: boolean; message: string }> {
    return this.request('connection.disconnect', { cluster_id: clusterId });
  }

  async reconnectCluster(clusterId: string): Promise<{ success: boolean; message: string }> {
    return this.request('connection.reconnect', { cluster_name: clusterId });
  }

  async healthCheckCluster(clusterId: string): Promise<{ cluster_id: string; healthy: boolean; status: string; error_message?: string }> {
    return this.request('connection.health_check', { cluster_id: clusterId });
  }

  async getConnectionMetrics(clusterId: string): Promise<{ cluster_id: string; consumer_pool_size: number; producer_pool_size: number; consumer_pool_available: number; producer_pool_available: number }> {
    return this.request('connection.metrics', { cluster_id: clusterId });
  }

  async getConnectionHistory(clusterId: string, limit?: number): Promise<{ cluster_id: string; history: { status: string; error_message?: string; latency_ms?: number; checked_at: string }[] }> {
    return this.request('connection.history', { cluster_id: clusterId, limit });
  }

  async getConnectionStats(clusterId: string): Promise<{ cluster_id: string; total_checks: number; successful_checks: number; failed_checks: number; success_rate: number; avg_latency_ms?: number; last_status: string; last_checked_at?: string }> {
    return this.request('connection.stats', { cluster_id: clusterId });
  }

  async batchDisconnect(clusterNames: string[]): Promise<{ total: number; successful: number; failed: number; results: { cluster_name: string; success: boolean; message?: string }[] }> {
    return this.request('connection.batch_disconnect', { cluster_names: clusterNames });
  }

  async batchReconnect(clusterNames: string[]): Promise<{ total: number; successful: number; failed: number; results: { cluster_name: string; success: boolean; message?: string }[] }> {
    return this.request('connection.batch_reconnect', { cluster_names: clusterNames });
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
