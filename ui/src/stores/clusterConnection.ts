import { defineStore } from 'pinia';
import { ref } from 'vue';
import { apiClient } from '@/api/client';

export interface ConnectionStatus {
  cluster_id: string;
  status: string;
  error_message?: string;
}

export const useClusterConnectionStore = defineStore('clusterConnections', () => {
  const connections = ref<ConnectionStatus[]>([]);
  const error = ref<string | null>(null);

  async function fetchAllConnections() {
    error.value = null;
    try {
      const data = await apiClient.getConnections();
      connections.value = data;
    } catch (e) {
      error.value = (e as { message: string }).message;
    }
  }

  async function disconnectCluster(clusterId: string) {
    return await apiClient.disconnectCluster(clusterId);
  }

  async function reconnectCluster(clusterId: string) {
    return await apiClient.reconnectCluster(clusterId);
  }

  function getConnectionStatus(clusterId: string): ConnectionStatus | undefined {
    return connections.value.find((c) => c.cluster_id === clusterId);
  }

  function updateConnectionStatus(clusterId: string, status: string, errorMessage?: string) {
    const index = connections.value.findIndex((c) => c.cluster_id === clusterId);
    if (index !== -1) {
      connections.value[index] = {
        cluster_id: clusterId,
        status,
        error_message: errorMessage,
      };
    } else {
      connections.value.push({
        cluster_id: clusterId,
        status,
        error_message: errorMessage,
      });
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    connections,
    error,
    fetchAllConnections,
    disconnectCluster,
    reconnectCluster,
    getConnectionStatus,
    updateConnectionStatus,
    clearError,
  };
});
