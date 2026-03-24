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
  HealthResponse,
  ApiError,
  ClusterGroup,
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
  private currentAbortController: AbortController | null = null;
  private backendReady: boolean | null = null;

  constructor(baseURL: string = getBaseURL()) {
    this.baseURL = baseURL;
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

  // Test cluster connection with temporary configuration
  async testClusterConfig(config: {
    name: string;
    brokers: string;
    request_timeout_ms?: number;
    operation_timeout_ms?: number;
  }): Promise<{ success: boolean; error?: string }> {
    return this.request('cluster.test_config', config);
  }

  // ==================== Cluster Groups ====================
  async getClusterGroups(): Promise<ClusterGroup[]> {
    return this.request('cluster_group.list', {});
  }

  async getClusterGroup(id: number): Promise<ClusterGroup> {
    return this.request('cluster_group.get', { id });
  }

  async createClusterGroup(group: { name: string; description?: string | null; sort_order?: number }): Promise<ClusterGroup> {
    return this.request('cluster_group.create', group);
  }

  async updateClusterGroup(id: number, group: { name?: string; description?: string | null; sort_order?: number }): Promise<ClusterGroup> {
    return this.request('cluster_group.update', { id, ...group });
  }

  async deleteClusterGroup(id: number): Promise<void> {
    return this.request('cluster_group.delete', { id });
  }

  async getClustersInGroup(groupId: number): Promise<Cluster[]> {
    return this.request('cluster_group.clusters', { group_id: groupId });
  }

  async assignClusterToGroup(clusterId: number, groupId: number): Promise<void> {
    return this.request('cluster_group.assign_cluster', { cluster_id: clusterId, group_id: groupId });
  }

  // ==================== Topics ====================
  async getTopics(clusterId?: string): Promise<string[]> {
    const params: any = {};
    if (clusterId) {
      params.cluster_id = clusterId;
    }
    const data = await this.request<TopicListResponse>('topic.list', params);
    return data.topics;
  }

  async getTopicsWithCluster(clusterId?: string, offset?: number, limit?: number): Promise<{ topics: { name: string; cluster: string }[]; total: number; has_more: boolean }> {
    const params: any = {};
    if (clusterId) {
      params.cluster_id = clusterId;
    }
    if (offset !== undefined) {
      params.offset = offset;
    }
    if (limit !== undefined) {
      params.limit = limit;
    }
    return this.request('topic.list_with_cluster', params);
  }

  async getTopicsWithClusters(clusterIds: string[], offset?: number, limit?: number, search?: string): Promise<{ topics: { name: string; cluster: string }[]; total: number; has_more: boolean }> {
    const params: any = {};
    params.cluster_ids = clusterIds;
    if (offset !== undefined) {
      params.offset = offset;
    }
    if (limit !== undefined) {
      params.limit = limit;
    }
    if (search !== undefined && search.trim() !== '') {
      params.search = search.trim();
    }
    return this.request('topic.list_with_cluster', params);
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

  async refreshTopics(clusterId?: string): Promise<{ success: boolean; added: string[]; removed: string[]; total: number }> {
    // 如果不传 clusterId，则刷新所有集群
    const params: Record<string, any> = {};
    if (clusterId) {
      params.cluster_id = clusterId;
    }
    return this.request('topic.refresh', params);
  }

  async cleanupOrphanTopics(): Promise<{ success: boolean; removed: [string, string][]; count: number }> {
    return this.request('topic.cleanup_orphans', {});
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
  /**
   * Get consumer groups list with topics information from database
   * Returns paginated results with total, offset, limit, and has_more
   */
  async getConsumerGroupsList(clusterIds?: string[], offset: number = 0, limit: number = 10000): Promise<{
    groups: Array<{
      id: number;
      cluster_id: string;
      group_name: string;
      topics: string[];
      fetched_at: string;
    }>;
    total: number;
    offset: number;
    limit: number;
    has_more: boolean;
  }> {
    const params: any = { offset, limit };
    if (clusterIds) {
      params.cluster_ids = clusterIds;
    }
    return await this.request<{
      groups: Array<{
        id: number;
        cluster_id: string;
        group_name: string;
        topics: string[];
        fetched_at: string;
      }>;
      total: number;
      offset: number;
      limit: number;
      has_more: boolean;
    }>('consumer_group.list', params);
  }

  /**
   * @deprecated Use getConsumerGroupsList instead which returns paginated results
   */
  async getConsumerGroups(clusterId?: string): Promise<string[]> {
    const params: any = {};
    if (clusterId) {
      params.cluster_id = clusterId;
    }
    const data = await this.request<{ groups: string[] }>('consumer_group.list', params);
    return data.groups || [];
  }

  async getSavedConsumerGroups(clusterId: string): Promise<string[]> {
    const data = await this.request<{ groups: string[] }>('consumer_group.saved', { cluster_id: clusterId });
    return data.groups || [];
  }

  async refreshConsumerGroups(clusterId?: string): Promise<{ success: boolean; added: string[]; removed: string[]; total: number }> {
    const params: Record<string, any> = {};
    if (clusterId) {
      params.cluster_id = clusterId;
    }
    return this.request('consumer_group.refresh', params);
  }

  async getConsumerGroupInfo(clusterId: string, groupName: string): Promise<{ name: string; cluster: string; topics: string[]; state: string }> {
    return this.request('consumer_group.get', { cluster_id: clusterId, group: groupName });
  }

  async getConsumerGroupOffsets(clusterId: string, groupName: string): Promise<Array<{
    topic: string;
    partition: number;
    start_offset: number;
    end_offset: number;
    committed_offset: number;
    lag: number;
  }>> {
    const data = await this.request<{ offsets: Array<{
      topic: string;
      partition: number;
      start_offset: number;
      end_offset: number;
      committed_offset: number;
      lag: number;
    }> }>('consumer_group.offsets', { cluster_id: clusterId, group: groupName });
    return data.offsets || [];
  }

  async refreshConsumerGroupOffsets(clusterId: string, groupName: string): Promise<{
    topic: string;
    partition: number;
    start_offset: number;
    end_offset: number;
    committed_offset: number;
    lag: number;
  }[]> {
    const data = await this.request<{ offsets: Array<{
      topic: string;
      partition: number;
      start_offset: number;
      end_offset: number;
      committed_offset: number;
      lag: number;
    }> }>('consumer_group.refresh_offsets', { cluster_id: clusterId, group: groupName });
    return data.offsets || [];
  }

  async resetConsumerGroupOffset(clusterId: string, groupName: string, topic: string, partition: number, offset: number): Promise<void> {
    return this.request('consumer_group.reset_offset', {
      cluster_id: clusterId,
      group: groupName,
      topic,
      partition,
      offset
    });
  }

  async resetConsumerGroupOffsetToEarliest(clusterId: string, groupName: string, topic: string, partition: number): Promise<{ offset: number }> {
    return this.request('consumer_group.reset_offset_earliest', {
      cluster_id: clusterId,
      group: groupName,
      topic,
      partition
    });
  }

  async resetConsumerGroupOffsetToLatest(clusterId: string, groupName: string, topic: string, partition: number): Promise<{ offset: number }> {
    return this.request('consumer_group.reset_offset_latest', {
      cluster_id: clusterId,
      group: groupName,
      topic,
      partition
    });
  }

  async resetConsumerGroupOffsetToTimestamp(clusterId: string, groupName: string, topic: string, partition: number, timestamp: number): Promise<{ offset: number }> {
    return this.request('consumer_group.reset_offset_timestamp', {
      cluster_id: clusterId,
      group: groupName,
      topic,
      partition,
      timestamp
    });
  }

  async deleteConsumerGroup(clusterId: string, groupName: string): Promise<void> {
    return this.request('consumer_group.delete', { cluster_id: clusterId, group: groupName });
  }

  async searchConsumerGroups(keyword?: string): Promise<{ cluster: string; group: string }[]> {
    const params: Record<string, any> = {};
    if (keyword && keyword.trim()) {
      params.keyword = keyword.trim();
    }
    const data = await this.request<{ results: { cluster: string; group: string }[] }>('consumer_group.search', params);
    return data.results || [];
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

  /**
   * 使用 SSE 流式获取消息
   * @param clusterId 集群ID
   * @param topic 主题名
   * @param params 查询参数
   * @param callbacks 回调函数
   * @returns EventSource 实例，可用于取消请求
   */
  getMessagesStream(
    clusterId: string,
    topic: string,
    params?: {
      partition?: number;
      offset?: number;
      max_messages?: number;
      order_by?: 'timestamp' | 'offset';
      sort?: 'asc' | 'desc';
      search?: string;
      search_in?: 'key' | 'value' | 'all';
      start_time?: number;
      end_time?: number;
      fetchMode?: 'oldest' | 'newest';
    },
    callbacks?: {
      onStart?: (data: { partitions: number; total_target: number }) => void;
      onBatch?: (messages: import('@/types/api').MessageRecord[], progress: number, total: number) => void;
      onOrder?: (sort: string) => void;
      onComplete?: (data?: { actual_total?: number }) => void;
      onError?: (error: string) => void;
    }
  ): AbortController {
    const url = `${this.baseURL}/api/stream`;

    const body = {
      cluster_id: clusterId,
      topic,
      ...params
    };

    // 使用 fetch 获取 SSE 流（EventSource 不支持 POST）
    const controller = new AbortController();
    const { signal } = controller;

    fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-API-Method': 'message.list'
      },
      body: JSON.stringify(body),
      signal
    }).then(async (response) => {
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        callbacks?.onError?.(errorData.error || `HTTP ${response.status}`);
        return;
      }

      const reader = response.body?.getReader();
      if (!reader) {
        callbacks?.onError?.('No response body');
        return;
      }

      const decoder = new TextDecoder();
      let buffer = '';
      let batchCount = 0;
      let totalMessagesReceived = 0;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });

        // 使用 \n\n 分割完整的 SSE 事件
        const parts = buffer.split('\n\n');
        buffer = parts.pop() || '';

        for (const part of parts) {
          const trimmed = part.trim();
          if (!trimmed) continue;

          // 解析 event 和 data 行
          const lines = trimmed.split('\n');
          let event = '';
          let data = '';

          for (const line of lines) {
            if (line.startsWith('event: ')) {
              event = line.slice(7).trim();
            } else if (line.startsWith('data: ')) {
              data = line.slice(6).trim();
            }
          }

          if (!event || !data) continue;

          try {
            const parsed = JSON.parse(data);
            switch (event) {
              case 'start':
                console.log(`[SSE Client] Start: partitions=${parsed.partitions}, total_target=${parsed.total_target}`);
                callbacks?.onStart?.(parsed);
                break;
              case 'batch':
                batchCount++;
                const msgCount = parsed.messages?.length || 0;
                totalMessagesReceived += msgCount;
                console.log(`[SSE Client] Batch #${batchCount}: ${msgCount} messages, total received: ${totalMessagesReceived}, progress=${parsed.progress}/${parsed.total}`);
                callbacks?.onBatch?.(parsed.messages, parsed.progress, parsed.total);
                break;
              case 'order':
                console.log(`[SSE Client] Order: sort=${parsed.sort}`);
                callbacks?.onOrder?.(parsed.sort);
                break;
              case 'complete':
                console.log(`[SSE Client] Complete event received: actual_total=${parsed.actual_total}, target_total=${parsed.target_total}`);
                callbacks?.onComplete?.(parsed);
                break;
              case 'error':
                callbacks?.onError?.(parsed.error);
                break;
            }
          } catch (e) {
            console.error('[SSE Client] Failed to parse event:', trimmed, e);
          }
        }
      }

      // 处理流结束后剩余的 buffer 数据
      if (buffer.trim()) {
        const trimmed = buffer.trim();
        const lines = trimmed.split('\n');
        let event = '';
        let data = '';

        for (const line of lines) {
          if (line.startsWith('event: ')) {
            event = line.slice(7).trim();
          } else if (line.startsWith('data: ')) {
            data = line.slice(6).trim();
          }
        }

        if (event && data) {
          try {
            const parsed = JSON.parse(data);
            switch (event) {
              case 'start':
                callbacks?.onStart?.(parsed);
                break;
              case 'batch': {
                const msgCount = parsed.messages?.length || 0;
                totalMessagesReceived += msgCount;
                console.log(`[SSE Client] Final buffer batch: ${msgCount} messages, total: ${totalMessagesReceived}`);
                callbacks?.onBatch?.(parsed.messages, parsed.progress, parsed.total);
                break;
              }
              case 'order':
                callbacks?.onOrder?.(parsed.sort);
                break;
              case 'error':
                callbacks?.onError?.(parsed.error);
                break;
            }
          } catch (e) {
            console.error('[SSE Client] Failed to parse final buffer:', buffer, e);
          }
        }
      }

      console.log(`[SSE Client] Stream complete. Total batches: ${batchCount}, Total messages: ${totalMessagesReceived}`);
      // 传递 complete 事件数据（包括 actual_total）
      callbacks?.onComplete?.({ actual_total: totalMessagesReceived });
    }).catch((error) => {
      if (error.name !== 'AbortError') {
        callbacks?.onError?.(error.message || 'Network error');
      }
    });

    return controller;
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

  // ==================== 集群连接管理 ====================
  async getConnections(): Promise<{ cluster_id: string; status: string; error_message?: string }[]> {
    const data = await this.request<{ connections: { cluster_id: string; status: string; error_message?: string }[] }>('connection.list', {});
    return data.connections || [];
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

  // ==================== 全局设置 ====================
  async getSettings(keys?: string[]): Promise<{ key: string; value: string }[]> {
    const data = await this.request<{ settings: { key: string; value: string }[] }>('settings.get', { keys });
    return data.settings;
  }

  async updateSetting(key: string, value: string): Promise<{ key: string; value: string }> {
    return this.request('settings.update', { key, value });
  }

  // ==================== Topic 收藏 ====================
  async getFavoriteGroups(): Promise<Array<{ id: number; name: string; description?: string; sort_order: number; item_count: number; created_at: string; updated_at: string }>> {
    return this.request('favorite.group.list', {});
  }

  async createFavoriteGroup(params: { name: string; description?: string; sort_order?: number }): Promise<{ id: number; name: string; description?: string; sort_order: number; created_at: string; updated_at: string }> {
    return this.request('favorite.group.create', params);
  }

  async updateFavoriteGroup(id: number, params: { name?: string; description?: string; sort_order?: number }): Promise<{ id: number; name: string; description?: string; sort_order: number; created_at: string; updated_at: string }> {
    return this.request('favorite.group.update', { id, ...params });
  }

  async deleteFavoriteGroup(id: number): Promise<void> {
    return this.request('favorite.group.delete', { id });
  }

  async getFavorites(): Promise<Array<{ id: number; name: string; description?: string; sort_order: number; items: Array<{ id: number; group_id: number; cluster_id: string; topic_name: string; description?: string; sort_order: number; created_at: string; updated_at: string }> }>> {
    return this.request('favorite.list', {});
  }

  async createFavorite(params: { group_id: number; cluster_id: string; topic_name: string; description?: string; sort_order?: number }): Promise<{ id: number; group_id: number; cluster_id: string; topic_name: string; description?: string; sort_order: number; created_at: string; updated_at: string }> {
    return this.request('favorite.create', params);
  }

  async updateFavorite(id: number, params: { group_id?: number; description?: string; sort_order?: number }): Promise<{ id: number; group_id: number; cluster_id: string; topic_name: string; description?: string; sort_order: number; created_at: string; updated_at: string }> {
    return this.request('favorite.update', { id, ...params });
  }

  async deleteFavorite(id: number): Promise<void> {
    return this.request('favorite.delete', { id });
  }

  async checkFavorite(clusterId: string, topicName: string): Promise<boolean> {
    const data = await this.request<{ is_favorite: boolean }>('favorite.check', { cluster_id: clusterId, topic_name: topicName });
    return data.is_favorite;
  }

  async deleteFavoriteByTopic(clusterId: string, topicName: string): Promise<void> {
    return this.request('favorite.delete_by_topic', { cluster_id: clusterId, topic_name: topicName });
  }

  // ==================== JSON 高亮模板 ====================
  async getJsonHighlightTemplates(): Promise<Array<{ id: number | null; name: string; description: string; is_builtin: boolean; style_json: string; created_at: string; updated_at: string }>> {
    const data = await this.request<{ templates: Array<{ id: number | null; name: string; description: string; is_builtin: boolean; style_json: string; created_at: string; updated_at: string }> }>('json_highlight.list', {});
    return data.templates;
  }

  async getCurrentJsonHighlightTemplate(): Promise<{ name: string }> {
    return this.request('json_highlight.get_current', {});
  }

  async setCurrentJsonHighlightTemplate(name: string): Promise<{ name: string }> {
    return this.request('json_highlight.set_current', { name });
  }

  async createJsonHighlightTemplate(params: { name: string; description: string; style_json: string }): Promise<{ id: number }> {
    return this.request('json_highlight.create', params);
  }

  async updateJsonHighlightTemplate(id: number, params: { description: string; style_json: string }): Promise<{ success: boolean }> {
    return this.request('json_highlight.update', { id, ...params });
  }

  async deleteJsonHighlightTemplate(id: number): Promise<{ success: boolean }> {
    return this.request('json_highlight.delete', { id });
  }
}

// 导出单例
export const apiClient = new ApiClient();

// 用于测试环境设置 baseURL
export function setupApiClient(baseURL?: string) {
  return new ApiClient(baseURL);
}
