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
      <div class="flex items-center justify-between">
        <span>{{ filteredTopics.length }} topics</span>
        <span v-if="searchQuery">搜索中</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import { apiClient } from '@/api/client';
import { useClusterStore } from '@/stores/cluster';

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
}>();

const clusterStore = useClusterStore();

// State
const searchQuery = ref('');
const allTopics = ref<TopicInfo[]>([]);
const loading = ref(false);
const selectedTopic = ref<TopicInfo | null>(null);

// Debounce timer
let searchTimer: number | null = null;

// Filtered topics
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

// Load all topics from all clusters
async function loadAllTopics() {
  loading.value = true;
  try {
    const clusters = clusterStore.clusters;
    const topics: TopicInfo[] = [];

    for (const cluster of clusters) {
      try {
        const clusterTopics = await apiClient.getTopics(cluster.name);
        for (const topicName of clusterTopics) {
          topics.push({
            name: topicName,
            cluster: cluster.name
          });
        }
      } catch (e) {
        console.error(`Failed to load topics for cluster ${cluster.name}:`, e);
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

// Search input handler with debounce
function onSearchInput() {
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  searchTimer = window.setTimeout(() => {
    // Search is reactive via computed property
  }, 300);
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

onMounted(() => {
  loadAllTopics();
});
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
