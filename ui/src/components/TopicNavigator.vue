<template>
  <div class="topic-navigator h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between p-2 mb-2 flex-shrink-0 border-b border-base-200">
      <div class="flex items-center gap-2">
        <div class="w-7 h-7 rounded-lg bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
          </svg>
        </div>
        <span class="text-xs font-bold text-base-content/60 uppercase tracking-wider">Topics</span>
      </div>
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

    <!-- Search Box -->
    <div class="px-2 mb-2 flex-shrink-0">
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
        :item-size="36"
        key-field="uid"
        v-slot="{ item }"
      >
        <div
          class="group flex items-center gap-2 px-2 py-1.5 rounded-lg cursor-pointer transition-all duration-200 hover:bg-base-200"
          :class="{ 'bg-primary/10': selectedTopic?.cluster === (item as TopicItem).topic.cluster && selectedTopic?.name === (item as TopicItem).topic.name }"
          @click="selectTopic((item as TopicItem).topic)"
        >
          <!-- Cluster Health Indicator -->
          <div
            class="w-2 h-2 rounded-full flex-shrink-0"
            :class="{
              'bg-success': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === true,
              'bg-error': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === false,
              'bg-warning': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === undefined
            }"
          ></div>

          <!-- Topic Icon -->
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-secondary flex-shrink-0">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
          </svg>

          <!-- Topic Name with Tooltip -->
          <div class="flex-1 min-w-0 relative">
            <span
              class="text-sm truncate block"
              :title="`${(item as TopicItem).topic.name} (${(item as TopicItem).topic.cluster})`"
            >
              {{ (item as TopicItem).topic.name }}
            </span>
          </div>

          <!-- Cluster Badge -->
          <span class="badge badge-ghost badge-xs flex-shrink-0 truncate max-w-16">
            {{ (item as TopicItem).topic.cluster }}
          </span>
        </div>
      </RecycleScroller>
    </div>

    <!-- Status Bar -->
    <div class="p-2 text-xs text-base-content/50 border-t border-base-200 flex-shrink-0">
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
import { ref, computed, watch } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import { useRoute, useRouter } from 'vue-router';
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
const router = useRouter();
const t = computed(() => languageStore.t);

// State
const searchQuery = ref('');
const selectedClusterFilter = ref(''); // 空表示所有集群
const allTopics = ref<TopicInfo[]>([]);
const loading = ref(false);
const refreshing = ref(false);
const selectedTopic = ref<TopicInfo | null>(null);

// Debounce timer
let searchTimer: number | null = null;

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

// Load all topics from all clusters or selected cluster
async function loadAllTopics() {
  loading.value = true;
  try {
    const clusters = clusterStore.clusters;
    const topics: TopicInfo[] = [];

    // If a specific cluster is selected, only load topics from that cluster
    if (selectedClusterFilter.value) {
      const cluster = clusters.find(c => c.name === selectedClusterFilter.value);
      if (cluster) {
        const clusterTopics = await apiClient.getTopics(cluster.name);
        for (const topicName of clusterTopics) {
          topics.push({
            name: topicName,
            cluster: cluster.name
          });
        }
      }
    } else {
      // No cluster selected - load all topics from all clusters with a single API call
      const allTopicNames = await apiClient.getTopics();
      for (const topicName of allTopicNames) {
        topics.push({
          name: topicName,
          cluster: '' // Will be resolved later when user expands cluster nodes
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

    allTopics.value = topics;
  } catch (e) {
    console.error('Failed to load topics:', e);
  } finally {
    loading.value = false;
  }
}

// Refresh topics from Kafka to SQLite database
async function refreshTopics() {
  if (refreshing.value) return;

  refreshing.value = true;
  try {
    const clusters = clusterStore.clusters;

    // If a specific cluster is selected, only refresh that cluster
    if (selectedClusterFilter.value) {
      await apiClient.refreshTopics(selectedClusterFilter.value);
    } else {
      // No cluster selected - refresh all clusters one by one
      // If a cluster is unreachable, wait 5 seconds silently and continue to next cluster
      for (const cluster of clusters) {
        try {
          await apiClient.refreshTopics(cluster.name);
        } catch (e) {
          // Silent failure - wait 5 seconds then continue to next cluster
          console.warn(`Failed to refresh topics for cluster ${cluster.name}, waiting 5s before continuing...`);
          await new Promise(resolve => setTimeout(resolve, 5000));
        }
      }
    }

    // Reload topics after refresh
    await loadAllTopics();
  } catch (e) {
    console.error('Failed to refresh topics:', e);
  } finally {
    refreshing.value = false;
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
  // Clear search query when changing cluster filter
  searchQuery.value = '';
}

// Clear search
function clearSearch() {
  searchQuery.value = '';
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
  emit('navigate', {
    path: '/clusters'
  });
}

watch([() => clusterStore.clusters.length, selectedClusterFilter], () => {
  loadAllTopics();
}, { immediate: true });

// Watch for route changes to handle cluster and search query params
watch(
  () => route.query,
  (newQuery) => {
    const cluster = newQuery.cluster as string;
    const search = newQuery.search as string;

    if (cluster && clusterStore.clusters.some(c => c.name === cluster)) {
      selectedClusterFilter.value = cluster;
    }

    if (search) {
      searchQuery.value = search;
    }

    // If both cluster and topic are provided, select the topic and navigate to messages
    const topic = newQuery.topic as string;
    if (cluster && topic) {
      // Wait for topics to load
      setTimeout(() => {
        const targetTopic = allTopics.value.find(
          t => t.cluster === cluster && t.name === topic
        );
        if (targetTopic) {
          selectedTopic.value = targetTopic;
          // Navigate to messages view
          router.replace({
            path: '/messages',
            query: { cluster, topic }
          });
        }
      }, 100);
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
