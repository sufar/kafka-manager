# Topic 导航实现指南

## 功能概述

本文档描述 Kafka Manager 中两种 Topic 导航方式的实现：
1. **顶部导航栏搜索** - 用户通过顶部搜索框搜索并点击 topic
2. **收藏双击跳转** - 用户在收藏页面双击收藏的 topic

## 行为一致性规范

两种导航方式在扁平模式（flat mode）下的 TopicNavigator 组件中应该表现一致：

| 行为 | 是否改变 |
|------|---------|
| 搜索框自动填充 | ❌ 否（保持用户当前选择） |
| 右下角 Cluster 选择器 | ❌ 否（保持用户当前选择） |
| Topic 列表过滤 | ✅ 是（根据右下角 cluster 选择器） |
| 高亮选中 Topic | ✅ 是 |
| 右侧消息界面 | ✅ 是（跳转到对应 cluster + topic） |

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

### 2. TopicNavigator.vue - 路由监听处理

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

### 3. 关键设计决策

#### 为什么不改变 Cluster 选择器？
- Cluster 选择器是用户手动控制的状态
- 自动改变会打断用户的操作流程
- 外部导航（顶部搜索、收藏双击）只影响右侧消息页面，不影响左侧 TopicNavigator 的 cluster 选择

#### 外部导航的处理逻辑
- 顶部搜索点击 topic → 跳转到 messages 页面，右下角 cluster 保持不变
- 收藏双击 topic → 跳转到 messages 页面，右下角 cluster 保持不变
- Topic 详情双击 topic → 跳转到 messages 页面，右下角 cluster 保持不变

#### 用户操作流程
1. 用户在右下角选择 cluster（例如 "cluster-a"）
2. 用户搜索并点击一个 topic（可能属于 "cluster-b"）
3. 右侧消息页面显示 "cluster-b" 的 topic 消息
4. 左侧 TopicNavigator 的 cluster 选择器仍然显示 "cluster-a"
5. 用户可以继续浏览 "cluster-a" 的其他 topics，不受影响

## 树形模式 vs 扁平模式

### 树形模式（Tree Mode）
- 使用 `ClusterTreeNavigator.vue`
- 双击收藏时展开树结构并高亮 topic
- 由 `highlightAndSelectTopic` 方法处理导航

### 扁平模式（Flat Mode）
- 使用 `TopicNavigator.vue`
- 通过 URL query 参数驱动状态
- 组件通过 `watch(route.query)` 响应变化
- Cluster 选择器完全由用户手动控制

## 注意事项

1. **Cluster 选择器只能手动切换**：不要使用任何内部状态来覆盖用户选择的 cluster

2. **刷新按钮行为**：只根据右下角选中的集群刷新，选"all clusters"时刷新所有集群

3. **搜索框行为**：根据右下角选中的 cluster 过滤 topics，选"all clusters"时搜索全部集群

4. **外部导航**：只设置 `pendingHighlight` 用于高亮 topic，不改变 cluster 选择器

## 相关文件

- `/src/layouts/ModernLayout.vue` - 导航处理和模式判断
- `/src/components/TopicNavigator.vue` - 扁平模式 topic 列表
- `/src/components/ClusterTreeNavigator.vue` - 树形模式 topic 树
- `/src/components/TopicFavorites.vue` - 收藏列表和双击事件
