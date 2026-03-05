import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Cluster, ClusterStatsResponse } from '@/types/api';
import { apiClient } from '@/api/client';

export interface ClusterHealth {
  clusterId: string;
  healthy?: boolean;
  lastChecked?: number;
  error?: string;
  stats?: ClusterStatsResponse;
}

export const useClusterStore = defineStore('clusters', () => {
  const clusters = ref<Cluster[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // 选中的集群列表（支持多选）
  const selectedClusterIds = ref<string[]>([]);

  // 集群健康状态
  const clusterHealth = ref<Record<string, ClusterHealth>>({});

  // 是否正在刷新健康状态
  const refreshingHealth = ref(false);

  // 选中的集群列表
  const selectedClusters = computed(() => {
    return clusters.value.filter((c) => selectedClusterIds.value.includes(c.name));
  });

  // 第一个选中的集群（用于向后兼容）
  const selectedCluster = computed(() => {
    if (selectedClusterIds.value.length === 0) return null;
    return clusters.value.find((c) => c.name === selectedClusterIds.value[0]) || null;
  });

  // 选中的集群 ID（单个，用于向后兼容）
  const selectedClusterId = computed({
    get: () => selectedClusterIds.value[0] || null,
    set: (value) => {
      if (value) {
        selectedClusterIds.value = [value];
      } else {
        selectedClusterIds.value = [];
      }
    },
  });

  async function fetchClusters() {
    // 如果正在加载，跳过重复请求
    if (loading.value) return;

    loading.value = true;
    error.value = null;
    try {
      clusters.value = await apiClient.getClusters();
      // 如果没有选中集群且存在集群，自动选择第一个
      if (selectedClusterIds.value.length === 0 && clusters.value.length > 0) {
        const firstName = clusters.value[0]?.name;
        if (firstName) {
          selectedClusterIds.value = [firstName];
        }
      }
      // 初始化健康状态（不阻塞，并行执行）
      for (const cluster of clusters.value) {
        if (!clusterHealth.value[cluster.name]) {
          clusterHealth.value[cluster.name] = {
            clusterId: cluster.name,
            healthy: undefined,
          };
        }
      }
      // 并行获取统计数据（使用 Promise.all 限制并发）
      const statsPromises = clusters.value.slice(0, 5).map(async (cluster) => { // 最多同时获取5个集群
        try {
          const stats = await apiClient.getClusterStats(cluster.name);
          const health = clusterHealth.value[cluster.name];
          if (health) {
            health.stats = stats;
          }
        } catch (e) {
          // 静默失败，不阻塞其他请求
        }
      });
      await Promise.all(statsPromises);
    } catch (e) {
      error.value = (e as { message: string }).message;
      console.error('[ClusterStore] Failed to fetch clusters:', e);
    } finally {
      loading.value = false;
    }
  }

  async function createCluster(cluster: {
    name: string;
    brokers: string;
    request_timeout_ms?: number;
    operation_timeout_ms?: number;
  }) {
    const newCluster = await apiClient.createCluster(cluster);
    clusters.value.push(newCluster);
    clusterHealth.value[newCluster.name] = {
      clusterId: newCluster.name,
      healthy: true,
    };
    return newCluster;
  }

  async function updateCluster(
    id: number,
    cluster: { name?: string; brokers?: string; request_timeout_ms?: number; operation_timeout_ms?: number }
  ) {
    const updatedCluster = await apiClient.updateCluster(id, cluster);
    const index = clusters.value.findIndex((c) => c.id === id);
    if (index !== -1) {
      clusters.value[index] = updatedCluster;
    }
    return updatedCluster;
  }

  async function deleteCluster(id: number) {
    const clusterName = clusters.value.find((c) => c.id === id)?.name;
    await apiClient.deleteCluster(id);
    clusters.value = clusters.value.filter((c) => c.id !== id);
    if (clusterName) {
      selectedClusterIds.value = selectedClusterIds.value.filter((id) => id !== clusterName);
      delete clusterHealth.value[clusterName];
    }
    // 如果没有选中集群且存在集群，自动选择第一个
    if (selectedClusterIds.value.length === 0 && clusters.value.length > 0) {
      const firstName = clusters.value[0]?.name;
      if (firstName) {
        selectedClusterIds.value = [firstName];
      }
    }
  }

  async function testCluster(id: number): Promise<{ success: boolean; error?: string }> {
    try {
      const result = await apiClient.testCluster(id);
      const cluster = clusters.value.find((c) => c.id === id);
      if (cluster) {
        clusterHealth.value[cluster.name] = {
          clusterId: cluster.name,
          healthy: result.success,
          lastChecked: Date.now(),
        };
      }
      return { success: result.success };
    } catch (e) {
      const cluster = clusters.value.find((c) => c.id === id);
      if (cluster) {
        clusterHealth.value[cluster.name] = {
          clusterId: cluster.name,
          healthy: false,
          lastChecked: Date.now(),
          error: (e as { message: string }).message,
        };
      }
      return { success: false, error: (e as { message: string }).message };
    }
  }

  // 刷新所有集群的健康状态（使用心跳检查）
  async function refreshAllHealth() {
    refreshingHealth.value = true;
    try {
      const promises = clusters.value.map(async (cluster) => {
        try {
          // 使用 health-check API 主动检查 Kafka 连接状态（心跳）
          const health = await apiClient.healthCheckCluster(cluster.name);
          const healthEntry: ClusterHealth = {
            clusterId: cluster.name,
            healthy: health.healthy,
            lastChecked: Date.now(),
            error: health.error_message,
          };

          // 保留之前的 stats 数据（如果存在）
          const previousStats = clusterHealth.value[cluster.name]?.stats;
          if (previousStats) {
            healthEntry.stats = previousStats;
          }

          // 如果健康检查成功，获取集群统计数据
          if (health.healthy) {
            try {
              const stats = await apiClient.getClusterStats(cluster.name);
              healthEntry.stats = stats;
            } catch (e) {
              console.warn(`Failed to fetch stats for cluster ${cluster.name}:`, e);
              // 如果获取 stats 失败但有之前的缓存，保留缓存
              if (previousStats) {
                healthEntry.stats = previousStats;
              }
            }
          }

          clusterHealth.value[cluster.name] = healthEntry;
        } catch (e) {
          // 如果健康检查失败，保留之前的 stats 数据
          const previousStats = clusterHealth.value[cluster.name]?.stats;
          clusterHealth.value[cluster.name] = {
            clusterId: cluster.name,
            healthy: false,
            lastChecked: Date.now(),
            error: (e as { message: string }).message,
            stats: previousStats, // 保留之前的 stats
          };
        }
      });
      await Promise.all(promises);
    } finally {
      refreshingHealth.value = false;
    }
  }

  function selectCluster(clusterId: string | null) {
    if (clusterId) {
      selectedClusterIds.value = [clusterId];
    } else {
      selectedClusterIds.value = [];
    }
  }

  // 切换集群选中状态（支持多选）
  function toggleClusterSelection(clusterId: string, selected?: boolean) {
    const index = selectedClusterIds.value.indexOf(clusterId);
    if (selected === true) {
      if (!selectedClusterIds.value.includes(clusterId)) {
        selectedClusterIds.value.push(clusterId);
      }
    } else if (selected === false) {
      selectedClusterIds.value = selectedClusterIds.value.filter((id) => id !== clusterId);
    } else {
      if (index > -1) {
        selectedClusterIds.value.splice(index, 1);
      } else {
        selectedClusterIds.value.push(clusterId);
      }
    }
  }

  // 选择所有集群
  function selectAllClusters() {
    selectedClusterIds.value = clusters.value.map((c) => c.name);
  }

  // 清除所有选择
  function clearSelection() {
    selectedClusterIds.value = [];
  }

  // 获取集群的健康状态
  function getClusterHealth(clusterId: string): ClusterHealth | undefined {
    return clusterHealth.value[clusterId];
  }

  // 获取所有健康集群的统计信息
  const totalStats = computed(() => {
    const stats = {
      totalClusters: clusters.value.length,
      healthyClusters: Object.values(clusterHealth.value).filter((h) => h.healthy).length,
      totalTopics: 0,
      totalPartitions: 0,
      totalConsumerGroups: 0,
      totalLag: 0,
    };

    for (const health of Object.values(clusterHealth.value)) {
      if (health.stats) {
        stats.totalTopics += health.stats.topic_count || 0;
        stats.totalPartitions += health.stats.partition_count || 0;
        stats.totalConsumerGroups += health.stats.consumer_group_count || 0;
        stats.totalLag += health.stats.total_lag || 0;
      }
    }

    return stats;
  });

  return {
    clusters,
    loading,
    error,
    selectedClusterIds,
    selectedClusterId,
    selectedCluster,
    selectedClusters,
    clusterHealth,
    refreshingHealth,
    totalStats,
    fetchClusters,
    createCluster,
    updateCluster,
    deleteCluster,
    testCluster,
    refreshAllHealth,
    selectCluster,
    toggleClusterSelection,
    selectAllClusters,
    clearSelection,
    getClusterHealth,
  };
});
