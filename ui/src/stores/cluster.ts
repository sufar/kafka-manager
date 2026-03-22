import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Cluster, ClusterGroup } from '@/types/api';
import { apiClient } from '@/api/client';

export interface ClusterHealth {
  clusterId: string;
  healthy?: boolean;
  lastChecked?: number;
  error?: string;
}

export const useClusterStore = defineStore('clusters', () => {
  const clusters = ref<Cluster[]>([]);
  const groups = ref<ClusterGroup[]>([]);
  const loading = ref(false);
  const loadingGroups = ref(false);
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

  // 每个分组的集群数量
  const groupClusterCounts = computed(() => {
    const counts: Record<number, number> = {};
    clusters.value.forEach((cluster) => {
      if (cluster.group_id) {
        counts[cluster.group_id] = (counts[cluster.group_id] || 0) + 1;
      }
    });
    return counts;
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
    group_id?: number;
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
    cluster: { name?: string; brokers?: string; request_timeout_ms?: number; operation_timeout_ms?: number; group_id?: number }
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
          clusterHealth.value[cluster.name] = {
            clusterId: cluster.name,
            healthy: health.healthy,
            lastChecked: Date.now(),
            error: health.error_message,
          };
        } catch (e) {
          clusterHealth.value[cluster.name] = {
            clusterId: cluster.name,
            healthy: false,
            lastChecked: Date.now(),
            error: (e as { message: string }).message,
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

  // 获取所有健康集群的统计信息（简化版，不再获取 Topic 和 Partition 数据）
  const totalStats = computed(() => {
    const stats = {
      totalClusters: clusters.value.length,
      healthyClusters: Object.values(clusterHealth.value).filter((h) => h.healthy).length,
      totalTopics: 0,
      totalPartitions: 0,
      totalConsumerGroups: 0,
      totalLag: 0,
    };

    return stats;
  });

  // 按分组组织集群
  const clustersByGroup = computed(() => {
    const result: Record<number, Cluster[]> = {};
    for (const cluster of clusters.value) {
      const groupId = cluster.group_id ?? 0;
      if (!result[groupId]) {
        result[groupId] = [];
      }
      result[groupId].push(cluster);
    }
    return result;
  });

  async function fetchGroups() {
    loadingGroups.value = true;
    error.value = null;
    try {
      groups.value = await apiClient.getClusterGroups();
    } catch (e) {
      error.value = (e as { message: string }).message;
      console.error('[ClusterStore] Failed to fetch groups:', e);
    } finally {
      loadingGroups.value = false;
    }
  }

  async function createGroup(group: { name: string; description?: string | null; sort_order?: number }) {
    const newGroup = await apiClient.createClusterGroup(group);
    groups.value.push(newGroup);
    return newGroup;
  }

  async function updateGroup(id: number, group: { name?: string; description?: string | null; sort_order?: number }) {
    const updatedGroup = await apiClient.updateClusterGroup(id, group);
    const index = groups.value.findIndex((g) => g.id === id);
    if (index !== -1) {
      groups.value[index] = updatedGroup;
    }
    return updatedGroup;
  }

  async function deleteGroup(id: number) {
    await apiClient.deleteClusterGroup(id);
    groups.value = groups.value.filter((g) => g.id !== id);
  }

  async function assignClusterToGroup(clusterId: number, groupId: number) {
    await apiClient.assignClusterToGroup(clusterId, groupId);
    // 更新本地缓存
    const cluster = clusters.value.find((c) => c.id === clusterId);
    if (cluster) {
      cluster.group_id = groupId;
    }
  }

  return {
    clusters,
    groups,
    loading,
    loadingGroups,
    error,
    selectedClusterIds,
    selectedClusterId,
    selectedCluster,
    selectedClusters,
    clusterHealth,
    refreshingHealth,
    totalStats,
    clustersByGroup,
    groupClusterCounts,
    fetchClusters,
    fetchGroups,
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
    createGroup,
    updateGroup,
    deleteGroup,
    assignClusterToGroup,
  };
});
