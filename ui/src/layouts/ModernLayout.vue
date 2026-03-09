<template>
  <div class="min-h-screen bg-base-100">
    <!-- Top Navigation Bar -->
    <header class="navbar glass border-b border-base-200 fixed top-0 left-0 right-0 h-16 z-50 px-4">
      <div class="flex-1 flex items-center gap-4">
        <!-- Logo and Brand -->
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-primary flex items-center justify-center shadow-lg">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-primary-content">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
            </svg>
          </div>
          <h1 class="text-xl font-bold">Kafka Manager</h1>
        </div>

        <!-- Topic Search -->
        <div class="relative">
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            type="text"
            class="input input-bordered w-80"
            :placeholder="t.layout.searchPlaceholder"
            @focus="showSearchDropdown = true"
            @keydown="handleSearchKeydown"
          />

          <!-- Search Results Dropdown -->
          <div
            v-if="showSearchDropdown && (searchQuery || searchResults.length > 0)"
            class="absolute top-full mt-1 w-96 bg-base-100 border border-base-200 rounded-lg shadow-xl z-50 max-h-96 overflow-y-auto"
          >
            <div v-if="searchLoading" class="p-4 text-center">
              <span class="loading loading-spinner loading-sm"></span>
            </div>
            <div v-else-if="searchError" class="p-4 text-error text-sm">
              {{ searchError }}
            </div>
            <div v-else-if="searchResults.length === 0 && searchQuery" class="p-4 text-sm text-base-content/60">
              {{ t.layout.noTopicsFound }}
            </div>
            <div v-else>
              <div
                v-for="(result, index) in filteredSearchResults"
                :key="`${result.cluster}-${result.topic}`"
                class="flex items-center justify-between p-3 hover:bg-base-200 cursor-pointer border-b border-base-200 last:border-0"
                :class="{ 'bg-base-200': selectedIndex === index }"
                @click="selectSearchResult(result)"
                @mouseenter="selectedIndex = index"
              >
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-secondary flex-shrink-0">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                    </svg>
                    <span class="font-mono text-sm truncate" v-html="highlightMatch(result.topic)"></span>
                  </div>
                </div>
                <div class="badge badge-ghost badge-xs ml-2">
                  {{ result.cluster }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="flex-none flex items-center gap-2">
        <!-- Language Toggle -->
        <button
          class="btn btn-ghost btn-circle btn-sm"
          @click="toggleLanguage"
          :title="`Toggle language (Current: ${currentLanguage === 'zh' ? '中文' : 'EN'})`"
        >
          <span class="text-xs font-bold">{{ currentLanguage === 'zh' ? 'EN' : '中' }}</span>
        </button>

        <!-- Theme Toggle -->
        <button
          class="btn btn-ghost btn-circle btn-sm"
          @click="toggleTheme"
          title="Toggle theme"
        >
          <svg v-if="!isDark" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" />
          </svg>
          <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" />
          </svg>
        </button>

        <!-- Settings Button -->
        <router-link to="/settings" class="btn btn-ghost btn-circle btn-sm" :title="t.layout.settings">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 0 1 0-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281Z" />
            <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
          </svg>
        </router-link>
      </div>
    </header>

    <!-- Main Layout Container -->
    <div class="flex h-screen pt-[4.5rem] overflow-hidden p-2 gap-2">
      <!-- Left Sidebar - Tree Navigator -->
      <aside class="w-80 glass gradient-border overflow-hidden flex flex-col">
        <div class="flex-1 overflow-y-auto p-2">
          <ClusterTreeNavigator
            ref="clusterTreeNavigatorRef"
            @navigate="handleNavigate"
            @cluster-context-menu="showClusterMenuFromTree"
            @topic-context-menu="showTopicMenuFromTree"
            @partition-context-menu="showPartitionMenuFromTree"
            @topics-folder-context-menu="showTopicsFolderMenuFromTree"
            @toast="handleToast"
          />
        </div>
      </aside>

      <!-- Main Content -->
      <main class="flex-1 glass gradient-border overflow-hidden flex flex-col">
        <div class="flex-1 overflow-y-auto p-2">
          <router-view />
        </div>
      </main>
    </div>

    <!-- Cluster Context Menu -->
    <ClusterContextMenu
      v-if="contextMenus.cluster.visible"
      :visible="contextMenus.cluster.visible"
      :cluster-name="contextMenus.cluster.clusterName"
      :position="contextMenus.cluster.position"
      :refreshing="refreshingCluster === contextMenus.cluster.clusterName"
      :testing="testingCluster === contextMenus.cluster.clusterName"
      :disconnecting="disconnectingCluster === contextMenus.cluster.clusterName"
      :reconnecting="reconnectingCluster === contextMenus.cluster.clusterName"
      @close="closeClusterMenu"
      @action="handleClusterAction"
    />

    <!-- Topics Folder Context Menu -->
    <TopicsFolderContextMenu
      v-if="contextMenus.topicsFolder.visible"
      :key="`topics-folder-${contextMenus.topicsFolder.clusterName}`"
      :visible="contextMenus.topicsFolder.visible"
      :cluster-name="contextMenus.topicsFolder.clusterName"
      :position="contextMenus.topicsFolder.position"
      @close="closeTopicsFolderMenu"
      @action="handleTopicsFolderAction"
    />

    <!-- Topic Context Menu -->
    <TopicContextMenu
      v-if="contextMenus.topic.visible"
      :visible="contextMenus.topic.visible"
      :topic-name="contextMenus.topic.topicName"
      :cluster-name="contextMenus.topic.clusterName"
      :position="contextMenus.topic.position"
      @close="closeTopicMenu"
      @action="handleTopicAction"
    />

    <!-- Partition Context Menu -->
    <PartitionContextMenu
      v-if="contextMenus.partition.visible"
      :visible="contextMenus.partition.visible"
      :topic-name="contextMenus.partition.topicName"
      :cluster-name="contextMenus.partition.clusterName"
      :partition-id="contextMenus.partition.partitionId"
      :position="contextMenus.partition.position"
      @close="closePartitionMenu"
      @action="handlePartitionAction"
    />

    <!-- Global Toast Notifications -->
    <div class="toast toast-end z-50">
      <div v-for="toast in toasts" :key="toast.id" class="alert" :class="getToastClass(toast.type)">
        <svg v-if="toast.type === 'error'" xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <svg v-else-if="toast.type === 'success'" xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <svg v-else-if="toast.type === 'warning'" xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <svg v-else xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span>{{ toast.message }}</span>
        <button class="btn btn-ghost btn-xs" @click="removeToast(toast.id)">✕</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, computed, watch, provide } from 'vue';
import { useRouter } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useClusterConnectionStore } from '@/stores/clusterConnection';
import { useThemeStore } from '@/stores/theme';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import ClusterTreeNavigator from '@/components/ClusterTreeNavigator.vue';
import ClusterContextMenu from '@/components/ContextMenus/ClusterContextMenu.vue';
import TopicContextMenu from '@/components/ContextMenus/TopicContextMenu.vue';
import TopicsFolderContextMenu from '@/components/ContextMenus/TopicsFolderContextMenu.vue';
import PartitionContextMenu from '@/components/ContextMenus/PartitionContextMenu.vue';

const router = useRouter();
const clusterStore = useClusterStore();
const connectionStore = useClusterConnectionStore();
const themeStore = useThemeStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const refreshing = ref(false);
const refreshingCluster = ref<string | null>(null);
const testingCluster = ref<string | null>(null);
const disconnectingCluster = ref<string | null>(null);
const reconnectingCluster = ref<string | null>(null);

// Ref for ClusterTreeNavigator
const clusterTreeNavigatorRef = ref<InstanceType<typeof ClusterTreeNavigator>>();

const { isDark, toggleTheme } = themeStore;
const toggleLanguage = languageStore.toggleLanguage;
const currentLanguage = computed(() => languageStore.currentLanguage);

// ==================== Topic Search ====================
const searchQuery = ref('');
const searchResults = ref<{ cluster: string; topic: string }[]>([]);
const searchLoading = ref(false);
const searchError = ref<string | null>(null);
const showSearchDropdown = ref(false);
const selectedIndex = ref(-1);
const searchInputRef = ref<HTMLInputElement>();

// 搜索时自动获取结果
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchQuery, (newQuery) => {
  const trimmed = newQuery.trim();
  if (trimmed) {
    // 显示下拉框
    showSearchDropdown.value = true;
    // 防抖 300ms（避免频繁请求）
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
      performSearch();
    }, 300);
  } else {
    searchResults.value = [];
  }
});

async function performSearch() {
  const query = searchQuery.value.trim();
  if (!query) return;

  searchLoading.value = true;
  searchError.value = null;
  try {
    searchResults.value = await apiClient.searchTopics(query);
  } catch (e) {
    console.error('[search] error:', e);
    searchError.value = (e as { message: string }).message;
  } finally {
    searchLoading.value = false;
  }
}

// 前端不再需要过滤，因为后端已经返回了匹配的结果
const filteredSearchResults = computed(() => {
  return searchResults.value;
});

function highlightMatch(topic: string) {
  if (!searchQuery.value.trim()) return topic;
  const query = searchQuery.value.toLowerCase();
  const topicLower = topic.toLowerCase();
  const index = topicLower.indexOf(query);
  if (index === -1) return topic;
  const before = topic.slice(0, index);
  const match = topic.slice(index, index + query.length);
  const after = topic.slice(index + query.length);
  return `${before}<mark class="bg-warning text-warning-content rounded px-0.5">${match}</mark>${after}`;
}

function handleSearchKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, filteredSearchResults.value.length - 1);
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
  } else if (e.key === 'Enter' && selectedIndex.value >= 0) {
    e.preventDefault();
    const result = filteredSearchResults.value[selectedIndex.value];
    if (result) selectSearchResult(result);
  } else if (e.key === 'Escape') {
    showSearchDropdown.value = false;
    searchInputRef.value?.blur();
  }
}

async function selectSearchResult(result: { cluster: string; topic: string }) {
  showSearchDropdown.value = false;
  searchQuery.value = '';
  // 先立即导航到消息页面，让用户立即看到结果
  router.push({ path: '/messages', query: { cluster: result.cluster, topic: result.topic } });
  // 然后异步展开左侧树（不阻塞导航）
  clusterTreeNavigatorRef.value?.highlightAndSelectTopic(result.topic, result.cluster);
}

// Ctrl+K 快捷键聚焦搜索框
function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
    e.preventDefault();
    searchInputRef.value?.focus();
    showSearchDropdown.value = true;
  }
}

// Context menu state
const contextMenus = reactive({
  cluster: {
    visible: false,
    clusterName: '',
    position: { x: 0, y: 0 }
  },
  topicsFolder: {
    visible: false,
    clusterName: '',
    position: { x: 0, y: 0 }
  },
  topic: {
    visible: false,
    topicName: '',
    clusterName: '',
    position: { x: 0, y: 0 }
  },
  partition: {
    visible: false,
    topicName: '',
    clusterName: '',
    partitionId: 0,
    position: { x: 0, y: 0 }
  }
});

// Toast notifications
interface Toast {
  id: number;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
}

const toasts = ref<Toast[]>([]);
let toastId = 0;

function showToast(type: 'success' | 'error' | 'warning' | 'info', message: string, duration: number = 5000) {
  const id = toastId++;
  toasts.value.push({ id, type, message });
  if (duration > 0) {
    setTimeout(() => removeToast(id), duration);
  }
}

function removeToast(id: number) {
  const index = toasts.value.findIndex(t => t.id === id);
  if (index > -1) {
    toasts.value.splice(index, 1);
  }
}

function getToastClass(type: string): string {
  const classes = {
    success: 'alert-success',
    error: 'alert-error',
    warning: 'alert-warning',
    info: 'alert-info'
  };
  return classes[type as keyof typeof classes] || 'alert-info';
}

// 提供给所有子组件使用
provide('showToast', showToast);

function handleNavigate(route: { path: string; query?: Record<string, string> }) {
  router.push({ path: route.path, query: route.query });
}

function handleToast(type: 'success' | 'error' | 'warning' | 'info', message: string) {
  showToast(type, message);
}

// 从 TopicsView 调用：展开并选中左侧树中的特定 Topic
function handleSelectTopicInTree(topicName: string, clusterName: string) {
  clusterTreeNavigatorRef.value?.highlightAndSelectTopic(topicName, clusterName);
}

function handleClusterAction(action: string, cluster: string) {
  switch (action) {
    case 'viewBrokers':
      router.push({ path: '/dashboard', query: { cluster } });
      break;
    case 'viewTopics':
      router.push({ path: '/topics', query: { cluster } });
      break;
    case 'refreshTopics':
      refreshClusterTopics(cluster);
      break;
    case 'refreshConnection':
      refreshConnectionStatus(cluster);
      break;
    case 'viewConsumers':
      router.push({ path: '/consumer-groups', query: { cluster } });
      break;
    case 'createTopic':
      router.push({ path: '/topics', query: { cluster, action: 'create' } });
      break;
    case 'testConnection':
      testConnection(cluster);
      break;
    case 'disconnect':
      disconnectCluster(cluster);
      break;
    case 'reconnect':
      reconnectCluster(cluster);
      break;
    case 'edit':
      editCluster(cluster);
      break;
    case 'deleteCluster':
      if (confirm(t.value.layout.confirmDeleteCluster.replace('{cluster}', cluster))) {
        // 通过集群名称找到对应的 ID
        const clusterId = clusterStore.clusters.find((c) => c.name === cluster)?.id;
        if (clusterId) {
          clusterStore.deleteCluster(clusterId);
        } else {
          showToast('error', t.value.layout.clusterNotFound);
        }
      }
      break;
  }
}

async function testConnection(clusterName: string) {
  const cluster = clusterStore.clusters.find((c) => c.name === clusterName);
  if (!cluster) return;

  testingCluster.value = clusterName;
  try {
    const result = await clusterStore.testCluster(cluster.id);
    if (result.success) {
      showToast('success', t.value.clusters.connected);
    } else {
      showToast('error', t.value.clusters.connectionError);
    }
  } catch (e) {
    showToast('error', `${t.value.clusters.connectionError}: ${(e as { message: string }).message}`);
  } finally {
    testingCluster.value = null;
  }
}

async function refreshConnectionStatus(clusterName: string) {
  refreshingCluster.value = clusterName;
  try {
    const health = await apiClient.healthCheckCluster(clusterName);
    clusterStore.clusterHealth[clusterName] = {
      clusterId: clusterName,
      healthy: health.healthy,
      lastChecked: Date.now(),
      error: health.error_message,
    };
    if (health.healthy) {
      showToast('success', 'Cluster status refreshed');
    } else {
      showToast('error', `Cluster connection issue: ${health.error_message || 'Unknown error'}`);
    }
  } catch (e) {
    showToast('error', `Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshingCluster.value = null;
  }
}

async function disconnectCluster(clusterName: string) {
  if (confirm(t.value.clusters.disconnectConfirm.replace('{cluster}', clusterName))) {
    disconnectingCluster.value = clusterName;
    try {
      await connectionStore.disconnectCluster(clusterName);
      // 更新集群健康状态为断开 - 这样会同步更新左侧菜单的绿点状态
      clusterStore.clusterHealth[clusterName] = {
        clusterId: clusterName,
        healthy: false,
        lastChecked: Date.now(),
        error: 'Disconnected',
      };
      await connectionStore.fetchAllConnections();
      showToast('success', 'Cluster disconnected successfully');
    } catch (e) {
      showToast('error', `Disconnect failed: ${(e as { message: string }).message}`);
    } finally {
      disconnectingCluster.value = null;
    }
  }
}

async function reconnectCluster(clusterName: string) {
  reconnectingCluster.value = clusterName;
  try {
    await connectionStore.reconnectCluster(clusterName);
    // 重新检查集群健康状态 - 这样会同步更新左侧菜单的绿点状态
    const health = await apiClient.healthCheckCluster(clusterName);
    clusterStore.clusterHealth[clusterName] = {
      clusterId: clusterName,
      healthy: health.healthy,
      lastChecked: Date.now(),
      error: health.error_message,
    };
    await connectionStore.fetchAllConnections();
    await clusterStore.fetchClusters();
    showToast('success', 'Cluster reconnected successfully');
  } catch (e) {
    showToast('error', `Reconnect failed: ${(e as { message: string }).message}`);
  } finally {
    reconnectingCluster.value = null;
  }
}

function editCluster(clusterName: string) {
  // 导航到 clusters 页面并打开编辑弹窗
  router.push({ path: '/clusters', query: { action: 'edit', cluster: clusterName } });
}

function handleTopicsFolderAction(action: string, cluster: string) {
  switch (action) {
    case 'refreshTopics':
      refreshClusterTopics(cluster);
      break;
    case 'createTopic':
      router.push({ path: '/topics', query: { cluster, action: 'create' } });
      break;
    case 'viewAllTopics':
      router.push({ path: '/topics', query: { cluster } });
      break;
  }
}

async function refreshClusterTopics(cluster: string) {
  if (refreshing.value) {
    // 如果正在刷新，点击则取消当前刷新
    apiClient.cancelRequest();
    refreshing.value = false;
    refreshingCluster.value = null;
    return;
  }

  refreshing.value = true;
  refreshingCluster.value = cluster;
  try {
    await apiClient.refreshTopics(cluster);
    // 通知 ClusterTreeNavigator 刷新
    window.dispatchEvent(new CustomEvent('cluster-topics-refreshed', { detail: { cluster } }));
  } catch (e) {
    const error = e as { message: string };
    // 如果是取消请求，不显示错误
    if (error.message === 'AbortError' || error.message.includes('aborted')) {
    } else {
      showToast('error', `${t.value.layout.refreshFailed}: ${error.message}`);
    }
  } finally {
    refreshing.value = false;
    refreshingCluster.value = null;
  }
}

function handleTopicAction(action: string, topic: string, cluster: string) {

  switch (action) {
    case 'viewMessages':
      router.push({ path: '/messages', query: { cluster, topic } });
      break;
    case 'viewDetails':
      router.push({ path: '/topics', query: { cluster, topic } });
      break;
    case 'viewPartitions':
      router.push({ path: '/topics', query: { cluster, topic, tab: 'partitions' } });
      break;
    case 'viewConsumerLag':
      router.push({ path: '/consumer-lag', query: { cluster, topic } });
      break;
    case 'sendMessage':
      router.push({ path: '/messages', query: { cluster, topic, action: 'send' } });
      break;
    case 'deleteTopic':
      if (confirm(t.value.layout.confirmDeleteTopic.replace('{topic}', topic))) {
        // Handle delete
      }
      break;
  }
}

function handlePartitionAction(action: string, topic: string, cluster: string, partitionId: number) {

  switch (action) {
    case 'viewMessages':
      router.push({ path: '/messages', query: { cluster, topic, partition: String(partitionId) } });
      break;
    case 'sendMessage':
      router.push({ path: '/messages', query: { cluster, topic, partition: String(partitionId), action: 'send' } });
      break;
  }
}

function closeClusterMenu() {
  contextMenus.cluster.visible = false;
}

function closeTopicsFolderMenu() {
  contextMenus.topicsFolder.visible = false;
}

function closeTopicMenu() {
  contextMenus.topic.visible = false;
}

function closePartitionMenu() {
  contextMenus.partition.visible = false;
}

function showClusterMenuFromTree(event: MouseEvent, clusterName: string) {
  contextMenus.cluster = {
    visible: true,
    clusterName,
    position: { x: event.clientX, y: event.clientY }
  };
}

function showTopicsFolderMenuFromTree(event: MouseEvent, clusterName: string) {
  contextMenus.topicsFolder = {
    visible: true,
    clusterName,
    position: { x: event.clientX, y: event.clientY }
  };
}

function showTopicMenuFromTree(event: MouseEvent, topicName: string, clusterName: string) {
  contextMenus.topic = {
    visible: true,
    topicName,
    clusterName,
    position: { x: event.clientX, y: event.clientY }
  };
}

function showPartitionMenuFromTree(event: MouseEvent, topicName: string, clusterName: string, partitionId: number) {
  contextMenus.partition = {
    visible: true,
    topicName,
    clusterName,
    partitionId,
    position: { x: event.clientX, y: event.clientY }
  };
}

onMounted(async () => {
  themeStore.initTheme();
  languageStore.initLanguage();
  await clusterStore.fetchClusters();

  // 注册全局键盘事件
  document.addEventListener('keydown', handleGlobalKeydown);

  // 监听点击外部关闭搜索下拉框
  document.addEventListener('click', handleClickOutside);

  // 监听 TopicsView 的双击 Topic 事件
  window.addEventListener('select-topic-in-tree', ((event: CustomEvent) => {
    const { topicName, clusterName } = event.detail;
    handleSelectTopicInTree(topicName, clusterName);
  }) as EventListener);

  // 监听打开创建集群弹窗事件
  window.addEventListener('open-create-cluster-modal', handleOpenCreateClusterModal);
});

function handleOpenCreateClusterModal() {
  // 导航到 clusters 页面并打开创建弹窗
  // 添加时间戳避免路由参数相同时无法触发更新
  router.push({ path: '/clusters', query: { action: 'create', t: String(Date.now()) } });
}

// 清理事件监听
onUnmounted(() => {
  window.removeEventListener('open-create-cluster-modal', handleOpenCreateClusterModal);
  document.removeEventListener('keydown', handleGlobalKeydown);
  document.removeEventListener('click', handleClickOutside);
});

function handleClickOutside(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 检查是否点击在搜索框或下拉框外部
  const searchInput = searchInputRef.value;
  const isClickInsideSearch = searchInput && target.closest('input') === searchInput;
  const isClickInsideDropdown = target.closest('.absolute') !== null;

  if (!isClickInsideSearch && !isClickInsideDropdown) {
    showSearchDropdown.value = false;
  }
}
</script>
