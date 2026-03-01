import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { apiClient } from '@/api/client';

export interface ConnectionStatus {
  cluster_id: string;
  status: string;
  error_message?: string;
}

export interface ConnectionMetrics {
  cluster_id: string;
  consumer_pool_size: number;
  producer_pool_size: number;
  consumer_pool_available: number;
  producer_pool_available: number;
}

export interface ConnectionHistoryEntry {
  status: string;
  error_message?: string;
  latency_ms?: number;
  checked_at: string;
}

export interface ConnectionStats {
  cluster_id: string;
  total_checks: number;
  successful_checks: number;
  failed_checks: number;
  success_rate: number;
  avg_latency_ms?: number;
  last_status: string;
  last_checked_at?: string;
}

export interface HealthCheckResult {
  cluster_id: string;
  healthy: boolean;
  status: string;
  error_message?: string;
}

export const useClusterConnectionStore = defineStore('clusterConnections', () => {
  const connections = ref<ConnectionStatus[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const metrics = ref<Record<string, ConnectionMetrics>>({});
  const stats = ref<Record<string, ConnectionStats>>({});
  const history = ref<Record<string, ConnectionHistoryEntry[]>>({});

  const connectionMap = computed(() => {
    const map: Record<string, ConnectionStatus> = {};
    for (const conn of connections.value) {
      map[conn.cluster_id] = conn;
    }
    return map;
  });

  async function fetchAllConnections() {
    loading.value = true;
    error.value = null;
    try {
      const data = await apiClient.getAllConnectionStatus();
      connections.value = data.connections;
    } catch (e) {
      error.value = (e as { message: string }).message;
    } finally {
      loading.value = false;
    }
  }

  async function fetchConnectionStatus(clusterId: string) {
    try {
      const data = await apiClient.getConnectionStatus(clusterId);
      const index = connections.value.findIndex((c) => c.cluster_id === clusterId);
      if (index !== -1) {
        connections.value[index] = data;
      } else {
        connections.value.push(data);
      }
      return data;
    } catch (e) {
      error.value = (e as { message: string }).message;
      throw e;
    }
  }

  async function disconnectCluster(clusterId: string) {
    return await apiClient.disconnectCluster(clusterId);
  }

  async function reconnectCluster(clusterId: string) {
    return await apiClient.reconnectCluster(clusterId);
  }

  async function healthCheck(clusterId: string): Promise<HealthCheckResult> {
    return await apiClient.healthCheckCluster(clusterId);
  }

  async function fetchMetrics(clusterId: string) {
    try {
      const data = await apiClient.getConnectionMetrics(clusterId);
      metrics.value[clusterId] = data;
      return data;
    } catch (e) {
      error.value = (e as { message: string }).message;
      throw e;
    }
  }

  async function fetchHistory(clusterId: string, limit: number = 100) {
    try {
      const data = await apiClient.getConnectionHistory(clusterId, limit);
      history.value[clusterId] = data.history;
      return data.history;
    } catch (e) {
      error.value = (e as { message: string }).message;
      throw e;
    }
  }

  async function fetchStats(clusterId: string) {
    try {
      const data = await apiClient.getConnectionStats(clusterId);
      stats.value[clusterId] = data;
      return data;
    } catch (e) {
      error.value = (e as { message: string }).message;
      throw e;
    }
  }

  async function batchDisconnect(clusterNames: string[]) {
    return await apiClient.batchDisconnect(clusterNames);
  }

  async function batchReconnect(clusterNames: string[]) {
    return await apiClient.batchReconnect(clusterNames);
  }

  function getConnectionStatus(clusterId: string): ConnectionStatus | undefined {
    return connectionMap.value[clusterId];
  }

  function isClusterConnected(clusterId: string): boolean {
    const status = connectionMap.value[clusterId];
    return status?.status === 'connected';
  }

  function clearError() {
    error.value = null;
  }

  return {
    connections,
    loading,
    error,
    metrics,
    stats,
    history,
    connectionMap,
    fetchAllConnections,
    fetchConnectionStatus,
    disconnectCluster,
    reconnectCluster,
    healthCheck,
    fetchMetrics,
    fetchHistory,
    fetchStats,
    batchDisconnect,
    batchReconnect,
    getConnectionStatus,
    isClusterConnected,
    clearError,
  };
});
