# Topic 导航实现指南

## 功能概述

本文档描述 Kafka Manager 中 TopicNavigator 组件的实现规范，包括 Cluster 选择器行为、Topic 搜索、外部导航处理以及状态持久化。

## 行为一致性规范

### Cluster 选择器行为

| 行为 | 是否改变 | 说明 |
|------|---------|------|
| 右下角 Cluster 选择器 | ❌ 否 | 只能手动点击切换，其他行为都不能改变 |
| 搜索框自动填充 | ❌ 否 | 保持用户当前选择 |
| Topic 列表过滤 | ✅ 是 | 根据右下角 cluster 选择器过滤 |
| 高亮选中 Topic | ✅ 是 | 外部导航时高亮对应 topic |
| 右侧消息界面 | ✅ 是 | 跳转到对应 cluster + topic |

### 外部导航场景

两种导航方式在扁平模式（flat mode）下的 TopicNavigator 组件中应该表现一致：

| 导航来源 | 行为 |
|---------|------|
| 顶部导航栏搜索点击 | 跳转到 messages 页面，右下角 cluster 保持不变 |
| 收藏页面双击 topic | 跳转到 messages 页面，右下角 cluster 保持不变 |
| Topic 详情页面双击 topic | 跳转到 messages 页面，右下角 cluster 保持不变 |

## 核心实现

### 1. ModernLayout.vue - 导航入口

#### 顶部导航栏搜索点击
```typescript
async function selectSearchResult(result: { cluster: string; topic: string }) {
  showSearchDropdown.value = false;
  searchQuery.value = '';

  // 统一跳转到 messages 页面，不管是 tree 还是 flat 模式
  router.push({ path: '/messages', query: { cluster: result.cluster, topic: result.topic } });

  if (sidebarMode.value === 'tree') {
    // Tree mode: expand tree and highlight topic
    clusterTreeNavigatorRef.value?.highlightAndSelectTopic(result.topic, result.cluster);
  }
  // Flat mode: TopicNavigator 通过 watch route.query 自动处理
}
```

#### 收藏双击跳转
```typescript
function handleNavigateFromFavorites(clusterId: string, topicName: string) {
  if (sidebarMode.value === 'tree') {
    // Tree mode: 展开树并选中 topic
    clusterTreeNavigatorRef.value?.highlightAndSelectTopic(topicName, clusterId);
  } else {
    // Flat mode: 和顶部导航栏搜索保持一致，跳转到 messages
    router.push({
      path: '/messages',
      query: { cluster: clusterId, topic: topicName },
    });
  }
}
```

### 2. TopicNavigator.vue - Cluster 选择器持久化

#### 状态定义
```typescript
const selectedClusterFilter = ref(''); // 空表示所有集群（UI 选择器，用户手动选择）
let hasLoadedSelectedCluster = false; // 标记是否已加载选中的集群
```

#### 加载选中的集群（从 SQLite 数据库）
```typescript
async function loadSelectedCluster() {
  if (hasLoadedSelectedCluster) return; // 只加载一次
  try {
    const settings = await apiClient.getSettings(['ui.selected_cluster']);
    const setting = settings.find((s: { key: string; value: string }) => s.key === 'ui.selected_cluster');
    if (setting && setting.value) {
      // 只有当集群列表可用且包含选中的集群时，才设置
      if (clusterStore.clusters.some(c => c.name === setting.value)) {
        selectedClusterFilter.value = setting.value;
        hasLoadedSelectedCluster = true;
        return;
      }
    }
    // 没有保存的设置或设置无效，标记为已加载
    hasLoadedSelectedCluster = true;
  } catch (e) {
    console.error('Failed to load selected cluster:', e);
    hasLoadedSelectedCluster = true;
  }
}
```

#### 保存选中的集群（到 SQLite 数据库）
```typescript
async function saveSelectedCluster() {
  try {
    await apiClient.updateSetting('ui.selected_cluster', selectedClusterFilter.value);
  } catch (e) {
    console.error('Failed to save selected cluster:', e);
  }
}
```

#### Cluster 过滤器变化处理
```typescript
function onClusterFilterChange() {
  // Clear search query when changing cluster filter
  searchQuery.value = '';
  // Reset pagination
  offset.value = 0;
  // Save selected cluster to settings
  saveSelectedCluster();
}
```

#### 监听集群列表变化
```typescript
// Watch for cluster list changes to ensure selectedClusterFilter is valid
watch(() => clusterStore.clusters, (newClusters) => {
  if (!hasLoadedSelectedCluster && newClusters.length > 0) {
    // 集群列表已加载，尝试恢复选中的集群
    loadSelectedCluster();
  }
  // If selected cluster is no longer in the list, reset to all clusters
  if (selectedClusterFilter.value && !newClusters.some(c => c.name === selectedClusterFilter.value)) {
    selectedClusterFilter.value = '';
  }
}, { deep: true });

onMounted(() => {
  // 如果集群列表已经加载，立即恢复选中的集群
  if (clusterStore.clusters.length > 0) {
    loadSelectedCluster();
  }
});
```

### 3. TopicNavigator.vue - 路由监听处理

```typescript
// Watch for route changes to handle cluster and topic query params
watch(
  () => route.query,
  (newQuery) => {
    const cluster = newQuery.cluster as string;
    const topic = newQuery.topic as string;

    // 如果有 cluster 和 topic，处理高亮
    if (cluster && topic) {
      pendingHighlight.value = { cluster, topic };
      // 外部导航不改变 cluster 下拉框，只设置 pendingHighlight 用于高亮选中的 topic
    } else {
      // 清除 pending highlight
      pendingHighlight.value = null;
    }
  },
  { immediate: true }
);
```

### 4. TopicNavigator.vue - 刷新和加载

#### 刷新 Topics（只刷新选中的集群）
```typescript
async function refreshTopics() {
  if (refreshing.value || isUnmounted.value) return;

  refreshing.value = true;
  try {
    const clusters = clusterStore.clusters;
    const clusterFilter = selectedClusterFilter.value;

    // If a specific cluster is selected, only refresh that cluster
    if (clusterFilter) {
      await apiClient.refreshTopics(clusterFilter);
    } else {
      // No cluster selected - refresh all clusters one by one
      for (const cluster of clusters) {
        try {
          await apiClient.refreshTopics(cluster.name);
        } catch (e) {
          // Silent failure - wait 5 seconds then continue to next cluster
          await new Promise(resolve => setTimeout(resolve, 5000));
        }
      }
      // Cleanup orphan topics
      await apiClient.cleanupOrphanTopics();
    }

    // Reload topics after refresh
    if (!isUnmounted.value) {
      await loadAllTopics();
    }
  } catch (e) {
    console.error('Failed to refresh topics:', e);
  } finally {
    refreshing.value = false;
  }
}
```

#### 加载 Topics（根据选中的 cluster 过滤）
```typescript
async function loadAllTopics() {
  if (isUnmounted.value) return;
  loading.value = true;
  offset.value = 0;
  allTopics.value = [];
  try {
    const clusters = clusterStore.clusters;
    const topics: TopicInfo[] = [];
    const clusterFilter = selectedClusterFilter.value;

    // If a specific cluster is selected, only load topics from that cluster
    if (clusterFilter) {
      const cluster = clusters.find(c => c.name === clusterFilter);
      if (cluster) {
        const result = await apiClient.getTopicsWithCluster(cluster.name, 0, limit.value);
        for (const topic of result.topics) {
          topics.push({ name: topic.name, cluster: topic.cluster });
        }
        total.value = result.total;
        hasMore.value = result.has_more;
      }
    } else {
      // No cluster selected - load all topics from all clusters
      const result = await apiClient.getTopicsWithCluster(undefined, 0, limit.value);
      for (const topic of result.topics) {
        topics.push({ name: topic.name, cluster: topic.cluster });
      }
      total.value = result.total;
      hasMore.value = result.has_more;
    }

    // Sort by cluster then by name
    topics.sort((a, b) => {
      if (a.cluster !== b.cluster) return a.cluster.localeCompare(b.cluster);
      return a.name.localeCompare(b.name);
    });

    if (!isUnmounted.value) {
      allTopics.value = topics;

      // Handle pending highlight
      if (pendingHighlight.value) {
        const { cluster, topic } = pendingHighlight.value;
        const targetTopic = allTopics.value.find(
          t => t.cluster === cluster && t.name === topic
        );
        if (targetTopic) {
          selectedTopic.value = targetTopic;
        }
        pendingHighlight.value = null;
      }
    }
  } catch (e) {
    console.error('Failed to load topics:', e);
  } finally {
    loading.value = false;
  }
}
```

## 关键设计决策

### 为什么不改变 Cluster 选择器？
- Cluster 选择器是用户手动控制的状态
- 自动改变会打断用户的操作流程
- 外部导航（顶部搜索、收藏双击）只影响右侧消息页面，不影响左侧 TopicNavigator 的 cluster 选择

### 用户操作流程
1. 用户在右下角选择 cluster（例如 "cluster-a"）
2. 用户搜索并点击一个 topic（可能属于 "cluster-b"）
3. 右侧消息页面显示 "cluster-b" 的 topic 消息
4. 左侧 TopicNavigator 的 cluster 选择器仍然显示 "cluster-a"
5. 用户可以继续浏览 "cluster-a" 的其他 topics，不受影响
6. 用户选择的 cluster 会被保存到数据库，下次访问时自动恢复

### Cluster 持久化机制
- 使用 SQLite 数据库存储 `ui.selected_cluster` 设置
- 用户每次切换 cluster 时自动保存
- 组件挂载时从数据库恢复上次的选择
- 如果保存的 cluster 不再存在，自动重置为 "all clusters"

## 树形模式 vs 扁平模式

### 树形模式（Tree Mode）
- 使用 `ClusterTreeNavigator.vue`
- 双击收藏时展开树结构并高亮 topic
- 由 `highlightAndSelectTopic` 方法处理导航

### 扁平模式（Flat Mode）
- 使用 `TopicNavigator.vue`
- Cluster 选择器持久化到数据库
- 通过 URL query 参数驱动状态
- 组件通过 `watch(route.query)` 响应变化
- Cluster 选择器完全由用户手动控制

## 注意事项

1. **Cluster 选择器只能手动切换**：不要使用任何内部状态来覆盖用户选择的 cluster

2. **刷新按钮行为**：只根据右下角选中的集群刷新，选"all clusters"时刷新所有集群

3. **搜索框行为**：根据右下角选中的 cluster 过滤 topics，选"all clusters"时搜索全部集群

4. **外部导航**：只设置 `pendingHighlight` 用于高亮 topic，不改变 cluster 选择器

5. **状态持久化**：用户选择的 cluster 会被保存到 `ui.selected_cluster` 设置，下次访问时自动恢复

6. **集群失效处理**：如果保存的 cluster 不再存在（被删除），自动重置为 "all clusters"

7. **加载时序**：先加载集群列表，再恢复选中的 cluster，确保 cluster 存在于列表中

## 相关文件

- `/src/layouts/ModernLayout.vue` - 导航处理和模式判断
- `/src/components/TopicNavigator.vue` - 扁平模式 topic 列表（包含 Cluster 持久化逻辑）
- `/src/components/ClusterTreeNavigator.vue` - 树形模式 topic 树
- `/src/components/TopicFavorites.vue` - 收藏列表和双击事件
