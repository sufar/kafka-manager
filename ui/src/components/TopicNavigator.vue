<template>
  <div class="topic-navigator h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between p-1.5 flex-shrink-0 border-b border-base-200">
      <div class="flex items-center gap-1.5">
        <div class="w-6 h-6 rounded bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-primary">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
          </svg>
        </div>
        <span class="text-xs font-bold text-base-content/60 uppercase tracking-wider">Topics</span>
      </div>
      <div class="flex items-center gap-0.5">
        <button
          class="btn btn-ghost btn-xs"
          @click="goToFavorites"
          title="Topic Favorites"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.563 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
          </svg>
        </button>
        <button
          class="btn btn-ghost btn-xs"
          @click="goToClusters"
          title="Manage Clusters"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 6h9.75M10.5 6a1.5 1.5 0 11-3 0m3 0a1.5 1.5 0 10-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-9.75 0h9.75" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Search Box -->
    <div class="px-1.5 py-1 flex-shrink-0">
      <div class="relative">
        <input
          v-model="searchQuery"
          type="text"
          class="input input-bordered input-sm w-full pr-8"
          placeholder="搜索 Topic..."
          @input="onSearchInput"
        />
        <svg
          v-if="!searchQuery"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          stroke-width="1.5"
          stroke="currentColor"
          class="w-4 h-4 absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40"
        >
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
        </svg>
        <button
          v-else
          class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content"
          @click="clearSearch"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Topic List with Virtual Scroll -->
    <div class="flex-1 overflow-hidden px-2">
      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center py-8">
        <span class="loading loading-spinner loading-sm"></span>
      </div>

      <!-- Empty -->
      <div v-else-if="filteredTopics.length === 0" class="text-center py-8 text-base-content/50">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 mx-auto mb-2 opacity-50">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
        <p class="text-xs">{{ searchQuery ? '无匹配结果' : '暂无 Topics' }}</p>
      </div>

      <!-- Virtual Scroll Topic Items -->
      <RecycleScroller
        v-else
        class="h-full overflow-auto"
        :items="filteredTopicsWithUid"
        :item-size="28"
        key-field="uid"
        v-slot="{ item }"
      >
        <div
          class="group flex items-center gap-1.5 px-1.5 py-1 rounded cursor-pointer transition-all duration-200 hover:bg-base-200"
          :class="{ 'bg-primary/10': selectedTopic?.cluster === (item as TopicItem).topic.cluster && selectedTopic?.name === (item as TopicItem).topic.name }"
          @click="selectTopic((item as TopicItem).topic)"
        >
          <!-- Cluster Health Indicator -->
          <div
            class="w-1.5 h-1.5 rounded-full flex-shrink-0"
            :class="{
              'bg-success': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === true,
              'bg-error': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === false,
              'bg-warning': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === undefined
            }"
          ></div>

          <!-- Topic Name with Tooltip -->
          <div class="flex-1 min-w-0 relative">
            <span
              class="text-xs truncate block"
              :title="`${(item as TopicItem).topic.name} (${(item as TopicItem).topic.cluster})`"
            >
              {{ (item as TopicItem).topic.name }}
            </span>
          </div>

          <!-- Cluster Badge -->
          <span class="badge badge-ghost badge-xs flex-shrink-0 truncate max-w-14 text-[10px] px-1">
            {{ (item as TopicItem).topic.cluster }}
          </span>
        </div>
      </RecycleScroller>
    </div>

    <!-- Status Bar -->
    <div class="p-1.5 text-xs text-base-content/50 border-t border-base-200 flex-shrink-0">
      <div class="flex items-center justify-between gap-2">
        <span>{{ filteredTopics.length }} topics</span>
        <div class="flex items-center gap-1">
          <span class="text-xs">Cluster:</span>
          <select
            v-model="selectedClusterFilter"
            class="select select-bordered select-xs"
            @change="onClusterFilterChange"
          >
            <option value="">{{ t.navigator.allClusters }}</option>
            <option v-for="cluster in clusterStore.clusters" :key="cluster.name" :value="cluster.name">
              {{ cluster.name }}
            </option>
          </select>
          <!-- Refresh Button -->
          <button
            class="btn btn-ghost btn-xs"
            :disabled="refreshing"
            @click="refreshTopics"
            title="刷新 Topics"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="w-3.5 h-3.5"
              :class="{ 'animate-spin': refreshing }"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import { useRoute } from 'vue-router';
import { apiClient } from '@/api/client';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';

interface TopicInfo {
  name: string;
  cluster: string;
}

interface TopicItem {
  topic: TopicInfo;
  uid: string;
}

const emit = defineEmits<{
  navigate: [{ path: string; query?: Record<string, string> }];
  update: [];
}>();

const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const route = useRoute();
const t = computed(() => languageStore.t);

// State
const searchQuery = ref('');
const selectedClusterFilter = ref(''); // 空表示所有集群（UI 选择器，用户手动选择）
const internalClusterFilter = ref(''); // 内部使用，收藏跳转时临时设置，不改变 UI
const allTopics = ref<TopicInfo[]>([]);
const loading = ref(false);
const refreshing = ref(false);
const selectedTopic = ref<TopicInfo | null>(null);
const isUnmounted = ref(false);

// Debounce timer
let searchTimer: number | null = null;

// Cleanup on unmount to prevent blocking navigation
onUnmounted(() => {
  isUnmounted.value = true;
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  // Cancel any pending API requests
  apiClient.cancelRequest();
});

// Filtered topics - search only (cluster filtering is done on backend)
const filteredTopics = computed(() => {
  if (!searchQuery.value.trim()) {
    return allTopics.value;
  }
  const query = searchQuery.value.toLowerCase();
  return allTopics.value.filter(
    t => t.name.toLowerCase().includes(query) || t.cluster.toLowerCase().includes(query)
  );
});

// Topics with uid for virtual scroll
const filteredTopicsWithUid = computed((): TopicItem[] => {
  return filteredTopics.value.map(topic => ({
    topic,
    uid: `${topic.cluster}-${topic.name}`
  }));
});

// Get cluster health
function getClusterHealth(clusterName: string) {
  return clusterStore.clusterHealth[clusterName];
}

// Track pending highlight for after topics load
const pendingHighlight = ref<{ cluster: string; topic: string } | null>(null);

// Load all topics from all clusters or selected cluster
async function loadAllTopics() {
  if (isUnmounted.value) return;
  loading.value = true;
  try {
    const clusters = clusterStore.clusters;
    const topics: TopicInfo[] = [];

    // 优先使用内部过滤器（收藏跳转等场景），否则使用 UI 选择器
    const clusterFilter = internalClusterFilter.value || selectedClusterFilter.value;

    // If a specific cluster is selected, only load topics from that cluster
    if (clusterFilter) {
      const cluster = clusters.find(c => c.name === clusterFilter);
      if (cluster) {
        const clusterTopics = await apiClient.getTopics(cluster.name);
        if (isUnmounted.value) return; // Check after async
        for (const topicName of clusterTopics) {
          topics.push({
            name: topicName,
            cluster: cluster.name
          });
        }
      }
    } else {
      // No cluster selected - load all topics from all clusters with cluster info
      const allTopicsWithCluster = await apiClient.getTopicsWithCluster();
      if (isUnmounted.value) return; // Check after async
      for (const topic of allTopicsWithCluster) {
        topics.push({
          name: topic.name,
          cluster: topic.cluster
        });
      }
    }

    // Sort by cluster then by name
    topics.sort((a, b) => {
      if (a.cluster !== b.cluster) {
        return a.cluster.localeCompare(b.cluster);
      }
      return a.name.localeCompare(b.name);
    });

    if (!isUnmounted.value) {
      allTopics.value = topics;

      // Check if there's a pending highlight after topics load
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
    if (!isUnmounted.value) {
      console.error('Failed to load topics:', e);
    }
  } finally {
    if (!isUnmounted.value) {
      loading.value = false;
    }
  }
}

// Refresh topics from Kafka to SQLite database
async function refreshTopics() {
  if (refreshing.value || isUnmounted.value) return;

  refreshing.value = true;
  try {
    const clusters = clusterStore.clusters;

    // 优先使用内部 cluster 过滤器，否则使用 UI 选择器
    const clusterFilter = internalClusterFilter.value || selectedClusterFilter.value;

    // If a specific cluster is selected, only refresh that cluster
    if (clusterFilter) {
      await apiClient.refreshTopics(clusterFilter);
      if (isUnmounted.value) return;
    } else {
      // No cluster selected - refresh all clusters one by one
      // If a cluster is unreachable, wait 5 seconds silently and continue to next cluster
      for (const cluster of clusters) {
        if (isUnmounted.value) return;
        try {
          await apiClient.refreshTopics(cluster.name);
        } catch (e) {
          if (isUnmounted.value) return;
          // Silent failure - wait 5 seconds then continue to next cluster
          console.warn(`Failed to refresh topics for cluster ${cluster.name}, waiting 5s before continuing...`);
          await new Promise(resolve => setTimeout(resolve, 5000));
          if (isUnmounted.value) return;
        }
      }

      // 清理孤儿 Topic（所属集群已被删除的 Topic）
      try {
        const result = await apiClient.cleanupOrphanTopics();
        if (isUnmounted.value) return;
        if (result.count > 0) {
          console.log(`Cleaned up ${result.count} orphan topics:`, result.removed);
        }
      } catch (e) {
        if (!isUnmounted.value) {
          console.warn('Failed to cleanup orphan topics:', e);
        }
      }
    }

    // Reload topics after refresh
    if (!isUnmounted.value) {
      await loadAllTopics();
    }
  } catch (e) {
    if (!isUnmounted.value) {
      console.error('Failed to refresh topics:', e);
    }
  } finally {
    if (!isUnmounted.value) {
      refreshing.value = false;
    }
  }
}

// Search input handler with debounce
function onSearchInput() {
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  searchTimer = window.setTimeout(() => {
    // Search is reactive via computed property
  }, 300);
}

// Cluster filter change handler
function onClusterFilterChange() {
  // 清除内部 cluster 过滤器，让用户手动选择生效
  internalClusterFilter.value = '';
  // Clear search query when changing cluster filter
  searchQuery.value = '';
}

// Clear search
function clearSearch() {
  searchQuery.value = '';
  internalClusterFilter.value = '';
}

// Select topic
function selectTopic(topic: TopicInfo) {
  selectedTopic.value = topic;
  emit('navigate', {
    path: '/messages',
    query: {
      cluster: topic.cluster,
      topic: topic.name
    }
  });
}

// Go to clusters page
function goToClusters() {
  // Cancel any pending operations before navigation
  if (searchTimer) {
    clearTimeout(searchTimer);
    searchTimer = null;
  }
  apiClient.cancelRequest();

  // Emit navigation event immediately
  emit('navigate', {
    path: '/clusters'
  });
}

// Go to favorites page
function goToFavorites() {
  emit('navigate', {
    path: '/favorites'
  });
}

watch([() => clusterStore.clusters.length, selectedClusterFilter], () => {
  loadAllTopics();
}, { immediate: true });

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
      const isAlreadyOnThisTopic = selectedTopic.value?.cluster === cluster && selectedTopic.value?.name === topic;

      if (!isAlreadyOnThisTopic) {
        // 来自外部导航（如收藏双击、顶部搜索）
        // 设置内部 cluster 过滤器（不改变 UI 选择器）
        internalClusterFilter.value = cluster;
        // 填入搜索框
        searchQuery.value = topic;
        // 重新加载 topics（根据 internalClusterFilter）
        loadAllTopics();
      }
    } else {
      // 清除内部 cluster 过滤器
      internalClusterFilter.value = '';
    }
  },
  { immediate: true }
);
</script>

<style scoped>
.topic-navigator {
  display: flex;
  flex-direction: column;
  height: 100%;
}

/* Custom scrollbar */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}

/* Vue virtual scroller styles */
.vue-recycle-scroller {
  position: relative;
}

.vue-recycle-scroller__item-wrapper {
  overflow: hidden;
}

.vue-recycle-scroller__item-view {
  position: absolute;
  top: 0;
  left: 0;
  will-change: transform;
}
</style>
