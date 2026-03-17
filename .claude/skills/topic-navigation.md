# Topic 导航实现指南

## 功能概述

本文档描述 Kafka Manager 中两种 Topic 导航方式的实现：
1. **顶部导航栏搜索** - 用户通过顶部搜索框搜索并点击 topic
2. **收藏双击跳转** - 用户在收藏页面双击收藏的 topic

## 行为一致性规范

两种导航方式在扁平模式（flat mode）下的 TopicNavigator 组件中应该表现一致：

| 行为 | 是否改变 |
|------|---------|
| 搜索框自动填充 | ✅ 是（如果 topic 不在当前列表中） |
| 右下角 Cluster 选择器 | ❌ 否（保持用户当前选择） |
| Topic 列表过滤 | ✅ 是（根据搜索框内容） |
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
    const search = newQuery.search as string;

    // 如果有 search 参数，设置搜索框
    if (search) {
      searchQuery.value = search;
    }

    // 如果有 cluster 和 topic，处理高亮和搜索框
    if (cluster && topic) {
      pendingHighlight.value = { cluster, topic };

      // 检查是否已经在该 topic（内部点击）
      const isAlreadyOnThisTopic = selectedTopic.value?.cluster === cluster
        && selectedTopic.value?.name === topic;

      if (!isAlreadyOnThisTopic) {
        // 检查 topic 是否在当前过滤后的列表中可见
        const checkAndFillSearch = () => {
          const topicVisible = allTopics.value.some(
            t => t.cluster === cluster && t.name === topic
          );

          // 如果 topic 不在当前列表中，且搜索框为空，填入搜索框以便找到它
          if (!topicVisible && !searchQuery.value) {
            searchQuery.value = topic;
          }
        };

        // 如果数据已加载，立即检查
        if (!loading.value && allTopics.value.length > 0) {
          checkAndFillSearch();
        } else {
          // 等待加载完成
          const unwatch = watch(loading, (isLoading) => {
            if (!isLoading) {
              checkAndFillSearch();
              unwatch();
            }
          });
        }
      }
    }
  },
  { immediate: true }
);
```

### 3. 关键设计决策

#### 为什么不改变 Cluster 选择器？
- Cluster 选择器是用户手动控制的状态
- 自动改变会打断用户的操作流程
- 搜索框过滤可以跨集群工作（因为搜索会匹配 topic 名称和 cluster 名称）

#### 为什么只在 topic 不可见时填充搜索框？
- 如果 topic 已经在当前列表中，不需要额外过滤
- 避免不必要的搜索框内容变化
- 提供更好的用户体验

#### 内部点击 vs 外部导航
- 内部点击（点击列表中的 topic）：不改变搜索框，直接高亮
- 外部导航（收藏双击、顶部搜索）：检查是否需要填充搜索框

## 树形模式 vs 扁平模式

### 树形模式（Tree Mode）
- 使用 `ClusterTreeNavigator.vue`
- 双击收藏时展开树结构并高亮 topic
- 由 `highlightAndSelectTopic` 方法处理导航

### 扁平模式（Flat Mode）
- 使用 `TopicNavigator.vue`
- 通过 URL query 参数驱动状态
- 组件通过 `watch(route.query)` 响应变化

## 注意事项

1. **避免使用内部状态覆盖 UI 状态**：不要使用 `internalClusterFilter` 等内部状态来覆盖用户选择的 cluster，应该通过搜索框过滤来实现

2. **时序处理**：topics 数据可能尚未加载完成，需要使用 watch 或等待机制

3. **防止循环**：确保内部点击 topic 不会触发外部导航逻辑（通过 `isAlreadyOnThisTopic` 检查）

## 相关文件

- `/src/layouts/ModernLayout.vue` - 导航处理和模式判断
- `/src/components/TopicNavigator.vue` - 扁平模式 topic 列表
- `/src/components/ClusterTreeNavigator.vue` - 树形模式 topic 树
- `/src/components/TopicFavorites.vue` - 收藏列表和双击事件
