<template>
  <div class="flex flex-col h-full overflow-hidden">
    <div class="p-3 relative flex-1 flex flex-col min-h-0">
    <!-- Page Header -->
    <div class="mb-4 relative">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2">
            <button class="btn btn-ghost btn-xs p-1 mr-2" :disabled="!canGoBack" :class="{ 'opacity-50 cursor-not-allowed': !canGoBack }" @click="goBack" :title="t.common?.back || 'Back'">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
              </svg>
            </button>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
            </svg>
            {{ t.topics.title }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">
            <span v-if="clusterParam">{{ t.clusters.clusters }}: <span class="font-medium">{{ clusterParam }}</span></span>
            <span v-else>{{ t.topics.description }}</span>
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            class="btn btn-xs btn-outline"
            @click="refreshAllTopics"
            :disabled="refreshing || !clusterParam"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            <span class="hidden md:inline ml-1">{{ t.common.refresh }}</span>
          </button>
          <button
            class="btn btn-xs btn-primary"
            @click="openCreateTopicDialog"
            :disabled="!clusterParam"
            data-tour="topics-create"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
            <span class="hidden md:inline ml-1">{{ t.common.create }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- No cluster selected -->
    <div v-if="!clusterParam && selectedClusterIds.length === 0" class="flex flex-col items-center justify-center py-8 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-4 text-sm">{{ t.topics.description }}</p>
    </div>

    <!-- Single cluster view - empty state -->
    <div v-else-if="clusterParam && filteredClusterTopics.length === 0 && !loading" class="card glass gradient-border shadow-xl">
      <div class="px-3 py-2 bg-base-100 border-b border-base-200 flex items-center gap-2">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-base-content/40">
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
        </svg>
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t.common.search"
          class="input input-bordered input-sm w-64"
        />
      </div>
      <div class="flex flex-col items-center justify-center py-8 text-center">
        <div class="text-base-content/30 mb-3">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-12 h-12">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
          </svg>
        </div>
        <h3 class="text-base font-semibold mb-1">{{ searchQuery ? t.topics.noSearchResults : t.common.noData }}</h3>
        <p class="text-base-content/60 mb-3 text-xs">
          <span v-if="searchQuery">{{ t.common.search }}: "{{ searchQuery }}"</span>
          <span v-else>{{ t.topics.description }}</span>
        </p>
        <button v-if="searchQuery" class="btn btn-xs btn-outline" @click="searchQuery = ''">{{ t.topics.clearSearch }}</button>
      </div>
    </div>

    <!-- Loading state -->
    <div v-else-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-md text-primary"></span>
      <p class="ml-4 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
      </svg>
      <span class="text-sm">{{ error }}</span>
    </div>

    <!-- Single cluster view (from URL param) -->
    <div v-else-if="clusterParam && filteredClusterTopics.length > 0" class="card glass gradient-border shadow-xl flex-1 flex flex-col overflow-hidden" data-tour="topics-list">
      <!-- Header with search and count -->
      <div class="px-3 py-2 bg-base-100 border-b border-base-200 flex items-center justify-between flex-shrink-0">
        <div class="flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-base-content/40">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
          </svg>
          <input
            v-model="searchQuery"
            type="text"
            :placeholder="t.common.search"
            class="input input-bordered input-sm w-64"
          />
        </div>
        <span class="text-xs text-base-content/50">{{ filteredClusterTopics.length }} topics</span>
      </div>
      <!-- Table header - fixed -->
      <div class="bg-base-100 border-b border-base-200 flex-shrink-0">
        <table class="table w-full table-sm">
          <thead>
            <tr>
              <th class="p-2 text-xs">{{ t.topics.topicName }}</th>
              <th class="p-2 text-xs text-right w-16">{{ t.common.actions }}</th>
            </tr>
          </thead>
        </table>
      </div>
      <!-- Table content - scrollable -->
      <div ref="singleClusterContainerRef" class="overflow-y-auto flex-1" @scroll="handleSingleClusterScroll">
        <table class="table w-full table-sm">
          <tbody>
            <!-- 虚拟滚动：顶部占位 -->
            <tr v-if="singleClusterVirtualStartIndex > 0" :style="{ height: singleClusterVirtualStartIndex * ROW_HEIGHT + 'px' }">
              <td colspan="2" style="padding: 0; border: 0;"></td>
            </tr>
            <!-- 可见区域的行 -->
            <tr v-for="topic in singleClusterVisibleTopics" :key="topic.name" @dblclick="selectTopicInTree(clusterParam, topic)" class="hover cursor-pointer">
              <td class="p-2">
                <div class="flex items-center gap-2">
                  <FavoriteButton
                    :cluster-id="clusterParam"
                    :topic-name="topic.name"
                    :t="t"
                    :favorite-cache="favoriteCache"
                    @change="handleFavoriteChange(clusterParam, topic.name, $event)"
                  />
                  <span class="text-xs truncate" :title="topic.name">{{ topic.name }}</span>
                </div>
              </td>
              <td class="p-2 text-right">
                <button class="btn btn-ghost btn-xs text-error hover:bg-error/10" @click="confirmDelete(clusterParam, topic.name)">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                  </svg>
                </button>
              </td>
            </tr>
            <!-- 虚拟滚动：底部占位 -->
            <tr v-if="singleClusterVirtualStartIndex + singleClusterVisibleTopics.length < filteredClusterTopics.length" :style="{ height: (filteredClusterTopics.length - singleClusterVirtualStartIndex - singleClusterVisibleTopics.length) * ROW_HEIGHT + 'px' }">
              <td colspan="2" style="padding: 0; border: 0;"></td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Loading state -->
    <div v-else-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-md text-primary"></span>
      <p class="ml-4 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
    </div>

    </div>

    <!-- Create Topic Dialog -->
    <CreateTopicDialog
      ref="createTopicDialogRef"
      :cluster-name="clusterParam"
      @created="fetchTopics"
    />

    <!-- Delete Topic Confirmation Dialog -->
    <DeleteTopicDialog
      ref="deleteTopicDialogRef"
      :cluster="deleteTarget.cluster"
      :topic="deleteTarget.topic"
      @deleted="handleTopicDeleted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useRoute, onBeforeRouteUpdate, useRouter } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';
import { useCanGoBack } from '@/composables/useCanGoBack';
import DeleteTopicDialog from '@/components/DeleteTopicDialog.vue';
import FavoriteButton from '@/components/FavoriteButton.vue';
import CreateTopicDialog from '@/components/CreateTopicDialog.vue';

// 定义本地类型
interface TopicItem {
  name: string;
  cluster: string;
  partition_count?: number;
  replication_factor?: number;
  status?: string;
}

const route = useRoute();
const router = useRouter();
const { canGoBack, goBack } = useCanGoBack();
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

const selectedClusterIds = computed(() => clusterStore.selectedClusterIds);

const clusterParam = computed((): string => {
  const val = route.query.cluster;
  return Array.isArray(val) ? (val[0] ?? '') : (val ?? '');
});


// 从URL读取搜索参数
const searchParam = computed(() => {
  const val = route.query.search;
  return Array.isArray(val) ? val[0] : (val || '');
});

const loading = ref(false);
const error = ref<string | null>(null);
const refreshing = ref(false);
const searchQuery = ref('');

const topicsByCluster = ref<Record<string, TopicItem[]>>({});
const clusterTopics = ref<TopicItem[]>([]);

// Favorite cache: Set of "clusterId-topicName" keys
const favoriteCache = ref<Set<string>>(new Set());

// Create Topic Dialog
const createTopicDialogRef = ref<InstanceType<typeof CreateTopicDialog> | null>(null);

function openCreateTopicDialog() {
  createTopicDialogRef.value?.open();
}

const deleteTarget = reactive({ cluster: '', topic: '' });

// Delete Topic Dialog
const deleteTopicDialogRef = ref<InstanceType<typeof DeleteTopicDialog> | null>(null);

const favoritesCacheTime = 5 * 60 * 1000; // 5 分钟缓存
let lastFavoritesFetchTime = 0;
let favoritesFetchPromise: Promise<void> | null = null;


// 虚拟滚动相关
const ROW_HEIGHT = 28; // 每行高度（像素），紧凑样式
const VISIBLE_OFFSET = 5; // 额外渲染的行数（减少以优化性能）

// Single cluster 模式虚拟滚动
const singleClusterContainerRef = ref<HTMLElement | null>(null);
void singleClusterContainerRef; // prevent ts-unused warning
const singleClusterScrollTop = ref(0);
const singleClusterContainerHeight = ref(0);

function handleSingleClusterScroll(event: Event) {
  const target = event.target as HTMLElement;
  if (!target) return;
  singleClusterScrollTop.value = target.scrollTop;
  singleClusterContainerHeight.value = target.clientHeight;
}

// Single cluster 模式虚拟滚动
const singleClusterVirtualStartIndex = computed(() => {
  return Math.max(0, Math.floor(singleClusterScrollTop.value / ROW_HEIGHT) - VISIBLE_OFFSET);
});

const singleClusterVisibleTopics = computed(() => {
  const allTopics = filteredClusterTopics.value;
  if (!allTopics.length) return [];

  const startIndex = singleClusterVirtualStartIndex.value;
  // 使用容器实际高度或默认值（确保有可见行）
  const containerH = singleClusterContainerHeight.value > 0 ? singleClusterContainerHeight.value : 400;
  const visibleCount = Math.ceil(containerH / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
  const endIndex = Math.min(allTopics.length, startIndex + visibleCount);

  return allTopics.slice(startIndex, endIndex);
});

// Filtered topics based on search query (使用防抖优化)
const searchQueryDebounced = ref('');
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchQuery, (newVal) => {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  searchDebounceTimer = setTimeout(() => {
    searchQueryDebounced.value = newVal;
  }, 150); // 150ms 防抖
});

const filteredClusterTopics = computed(() => {
  if (!searchQueryDebounced.value) return clusterTopics.value;

  const query = searchQueryDebounced.value.toLowerCase();
  return clusterTopics.value.filter(topic =>
    topic.name.toLowerCase().includes(query)
  );
});

onBeforeRouteUpdate((to) => {
  // 处理 search 参数变化
  const searchVal = Array.isArray(to.query.search) ? to.query.search[0] : (to.query.search || '');
  if (searchVal) {
    searchQuery.value = searchVal;
  } else {
    searchQuery.value = '';
  }
});

// 使用 watch 替代 watchEffect，精确监听 clusterParam 变化
watch(clusterParam, (newClusterParam) => {
  if (newClusterParam) {
    fetchTopics();
  }
}, { immediate: true });

async function fetchTopics() {
  loading.value = true;
  error.value = null;

  if (!clusterParam.value) {
    loading.value = false;
    return;
  }

  try {
    const topicNames = await apiClient.getTopics(clusterParam.value);
    clusterTopics.value = topicNames.map((name) => ({
      name,
      cluster: clusterParam.value as string,
      partition_count: undefined,
    }));
    topicsByCluster.value = {};
    // Fetch favorites
    await fetchFavorites();
  } catch (e) {
    console.error('[TopicsView] Error fetching topics:', e);
    error.value = (e as { message: string }).message || 'Failed to load topics';
  } finally {
    loading.value = false;
  }
}

// 双击 Topic 时触发：通知父组件展开并选中左侧树中的对应 Topic
function selectTopicInTree(clusterName: string, topic: TopicItem) {
  // 触发自定义事件，由 ModernLayout 捕获并处理
  window.dispatchEvent(new CustomEvent('select-topic-in-tree', {
    detail: { topicName: topic.name, clusterName }
  }));
}

async function confirmDelete(clusterId: string, topicName: string) {
  deleteTarget.cluster = clusterId;
  deleteTarget.topic = topicName;
  deleteTopicDialogRef.value?.open();
}

function handleTopicDeleted(_cluster: string, _topic: string) {
  fetchTopics();
}

async function refreshAllTopics() {
  if (!clusterParam.value) return;

  refreshing.value = true;

  // 如果搜索框有内容，只刷新该单个 topic
  const topicQuery = searchQuery.value.trim();
  const isSingleTopic = topicQuery !== '';

  // 立即提示，不等待后端响应
  if (isSingleTopic) {
    showSuccess(`Refreshing topic "${topicQuery}"...`, 3000);
  } else {
    showSuccess(t.value.topics.refreshingBg, 3000);
  }

  // Fire-and-forget
  apiClient.refreshTopics(clusterParam.value, isSingleTopic ? topicQuery : undefined).catch(() => {});

  // 立即释放按钮状态
  refreshing.value = false;

  // 短暂延迟后重新加载
  setTimeout(async () => {
    await fetchTopics();
  }, 500);
}


async function fetchFavorites() {
  // 如果正在请求中，等待完成
  if (favoritesFetchPromise) {
    return favoritesFetchPromise;
  }

  // 如果缓存未过期，使用缓存
  const now = Date.now();
  if (now - lastFavoritesFetchTime < favoritesCacheTime) {
    return;
  }

  favoritesFetchPromise = (async () => {
    try {
      const groups = await apiClient.getFavorites();

      // Always fetch all favorites and cache them
      const newCache = new Set<string>();

      for (const group of groups) {
        for (const item of group.items) {
          newCache.add(`${item.cluster_id}-${item.topic_name}`);
        }
      }

      favoriteCache.value = newCache;
      lastFavoritesFetchTime = Date.now();
    } catch (e) {
      console.error('[TopicsView] Error fetching favorites:', e);
      // 超时等错误不阻断主流程
    } finally {
      favoritesFetchPromise = null;
    }
  })();

  return favoritesFetchPromise;
}

// Handle favorite change event to update cache
function handleFavoriteChange(clusterId: string, topicName: string, isFavorite: boolean) {
  const key = `${clusterId}-${topicName}`;
  if (isFavorite) {
    favoriteCache.value.add(key);
  } else {
    favoriteCache.value.delete(key);
  }
}

onMounted(() => {
  // 从URL读取搜索参数
  if (searchParam.value) {
    searchQuery.value = searchParam.value;
  }
  // 初始化容器高度
  if (singleClusterContainerRef.value) {
    singleClusterContainerHeight.value = singleClusterContainerRef.value.clientHeight || 400;
  }
});
</script>

<style scoped>
/* 紧凑表格样式 */
.table-sm :deep(td) {
  padding: 0.5rem;
  vertical-align: middle;
}

.table-sm :deep(th) {
  padding: 0.5rem;
  font-size: 0.75rem;
  text-transform: none;
  letter-spacing: normal;
  font-weight: 600;
}

/* 确保表头和内容列宽一致 */
.table-sm :deep(td:last-child),
.table-sm :deep(th:last-child) {
  width: 64px;
  min-width: 64px;
}

/* 移除背景模糊效果 */
:global(.modal:has(.modal-box)::backdrop) {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:global(dialog[open]::backdrop) {
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
</style>
