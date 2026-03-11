/// 消息查询 Composable
/// 提供统一的消息查询接口，支持进度跟踪、取消、缓存等功能

import { ref } from 'vue';
import type { MessageRecord, QueryStats } from '@/types/api';
import { apiClient } from '@/api/client';

export interface MessageQueryParams {
  clusterId: string;
  topic: string;
  partition?: number;
  fetchMode: 'oldest' | 'newest';
  timeRange?: {
    start: number;
    end: number;
  };
  search?: {
    keyword: string;
    scope: 'key' | 'value' | 'all';
  };
  limit: number;
  perPartitionMax: boolean;
  orderBy?: 'timestamp' | 'offset';
  sort?: 'asc' | 'desc';
  scanDepth?: number;
}

export interface QueryResult {
  messages: MessageRecord[];
  stats?: QueryStats;
  fetchTime: number;
}

export function useMessageQuery() {
  const loading = ref(false);
  const progress = ref(0);
  const error = ref<string | null>(null);
  const result = ref<QueryResult | null>(null);
  const currentAbortController = ref<AbortController | null>(null);

  // 查询缓存
  const cache = new Map<string, {
    result: QueryResult;
    timestamp: number;
    params: string;
  }>();

  const CACHE_TTL = 5000; // 5 秒缓存

  function getCacheKey(params: MessageQueryParams): string {
    // newest 模式不缓存，确保总是获取最新消息
    if (params.fetchMode === 'newest') {
      return '';
    }
    return `${params.clusterId}:${params.topic}:${JSON.stringify({
      partition: params.partition,
      fetchMode: params.fetchMode,
      timeRange: params.timeRange,
      search: params.search,
      limit: params.limit,
      perPartitionMax: params.perPartitionMax,
    })}`;
  }

  function getCached(key: string): QueryResult | null {
    if (!key) return null;
    const cached = cache.get(key);
    if (cached && Date.now() - cached.timestamp < CACHE_TTL) {
      return cached.result;
    }
    if (cached) {
      cache.delete(key);
    }
    return null;
  }

  function setCache(key: string, result: QueryResult, paramsStr: string) {
    if (!key) return;
    cache.set(key, {
      result,
      timestamp: Date.now(),
      params: paramsStr,
    });
    // 限制缓存大小
    if (cache.size > 50) {
      const oldestKey = Array.from(cache.keys()).sort((a, b) => {
        const aTime = cache.get(a)!.timestamp;
        const bTime = cache.get(b)!.timestamp;
        return aTime - bTime;
      })[0];
      if (oldestKey) {
        cache.delete(oldestKey);
      }
    }
  }

  async function execute(params: MessageQueryParams, forceRefresh = false): Promise<QueryResult> {
    // 取消之前的请求
    cancel();

    loading.value = true;
    progress.value = 0;
    error.value = null;

    const startTime = performance.now();
    const cacheKey = getCacheKey(params);

    // 尝试从缓存获取
    if (!forceRefresh) {
      const cached = getCached(cacheKey);
      if (cached) {
        loading.value = false;
        result.value = cached;
        return cached;
      }
    }

    try {
      currentAbortController.value = new AbortController();

      const apiParams: Record<string, any> = {
        max_messages: params.limit,
        per_partition_max: params.perPartitionMax,
        fetchMode: params.fetchMode,
        start_time: params.timeRange?.start,
        end_time: params.timeRange?.end,
        order_by: params.orderBy,
        sort: params.sort,
        scan_depth: params.scanDepth,
      };

      if (params.partition !== undefined) {
        apiParams.partition = params.partition;
      }

      if (params.search) {
        apiParams.search = params.search.keyword;
        apiParams.search_in = params.search.scope;
      }

      // 执行查询
      const data = await apiClient.getMessages(params.clusterId, params.topic, apiParams);

      const fetchTime = Math.round(performance.now() - startTime);
      const queryResult: QueryResult = {
        messages: data.messages,
        stats: data.stats,
        fetchTime,
      };

      // 缓存结果
      setCache(cacheKey, queryResult, JSON.stringify(apiParams));

      result.value = queryResult;
      progress.value = 100;

      return queryResult;
    } catch (e) {
      const err = e as { message?: string };
      if (err.message?.includes('aborted')) {
        // 请求被取消，不设置错误
        throw new Error('cancelled');
      }
      error.value = err.message || 'Query failed';
      throw e;
    } finally {
      loading.value = false;
      currentAbortController.value = null;
    }
  }

  function cancel() {
    if (currentAbortController.value) {
      currentAbortController.value.abort();
      currentAbortController.value = null;
    }
    apiClient.cancelGetMessages();
    loading.value = false;
  }

  function reset() {
    cancel();
    result.value = null;
    error.value = null;
    progress.value = 0;
  }

  function clearCache() {
    cache.clear();
  }

  return {
    loading,
    progress,
    error,
    result,
    execute,
    cancel,
    reset,
    clearCache,
  };
}
