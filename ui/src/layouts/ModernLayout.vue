<template>
  <div class="h-[100dvh] bg-base-100 flex flex-col pt-10">
    <!-- Top Navigation Bar -->
    <TopNavBar
      :is-mobile="isMobile"
      :is-dark="isDark"
      :sidebar-mode="sidebarMode"
      @toggle-sidebar="sidebarOpen = true"
      @toggle-language="toggleLanguage"
      @toggle-theme="toggleTheme"
      @open-mobile-search="showMobileSearch = true"
      @select-topic="handleSelectTopicInTree"
    />

    <!-- Mobile Search Drawer -->
    <MobileSearchDrawer
      v-if="isMobile && showMobileSearch"
      @close="showMobileSearch = false"
      @select-topic="handleSearchTopicSelect"
    />

    <!-- Main Layout Container -->
    <div class="flex flex-1 overflow-hidden">
      <LeftSidebar
        :is-mobile="isMobile"
        :sidebar-mode="sidebarMode"
        :sidebar-open="sidebarOpen"
        @navigate="handleNavigate"
        @cluster-context-menu="showClusterMenuFromTree"
        @topic-context-menu="showTopicMenuFromTree"
        @partition-context-menu="showPartitionMenuFromTree"
        @topics-folder-context-menu="showTopicsFolderMenuFromTree"
        @toast="handleToast"
        @close-sidebar="sidebarOpen = false"
      />

      <!-- Main Content -->
      <main class="flex-1 glass gradient-border overflow-auto flex flex-col min-w-0 rounded-xl mr-2 h-[calc(100dvh-0.5rem)]">
        <router-view />
      </main>
    </div>

    <!-- Context Menus -->
    <ContextMenus
      ref="contextMenusRef"
      @cluster-action="handleClusterAction"
      @topics-folder-action="handleTopicsFolderAction"
      @topic-action="handleTopicAction"
      @partition-action="handlePartitionAction"
    />

    <!-- Toast Notifications and Confirm Dialog -->
    <ToastAndConfirm ref="toastRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useClusterConnectionStore } from '@/stores/clusterConnection';
import { useThemeStore } from '@/stores/theme';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';

// Import layout components
import TopNavBar from '@/components/layout/TopNavBar.vue';
import MobileSearchDrawer from '@/components/layout/MobileSearchDrawer.vue';
import LeftSidebar from '@/components/layout/LeftSidebar.vue';
import ContextMenus from '@/components/layout/ContextMenus.vue';
import ToastAndConfirm from '@/components/layout/ToastAndConfirm.vue';

const router = useRouter();
const route = useRoute();
const clusterStore = useClusterStore();
const connectionStore = useClusterConnectionStore();
const themeStore = useThemeStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

// Refs
const contextMenusRef = ref<InstanceType<typeof ContextMenus>>();
const toastRef = ref<InstanceType<typeof ToastAndConfirm>>();
const clusterTreeNavigatorRef = ref<InstanceType<typeof import('@/components/ClusterTreeNavigator.vue').default>>();

// Sidebar mode: 'tree' | 'flat'
const sidebarMode = ref<'tree' | 'flat'>('flat');

// Mobile responsive state
const isMobile = ref(false);
const sidebarOpen = ref(false);
const showMobileSearch = ref(false);
const MOBILE_BREAKPOINT = 768;

// Check if mobile
function checkMobile() {
  isMobile.value = window.innerWidth < MOBILE_BREAKPOINT;
  if (!isMobile.value) {
    sidebarOpen.value = false;
    showMobileSearch.value = false;
  }
}

// Theme and language
const { isDark, toggleTheme } = themeStore;
const toggleLanguage = languageStore.toggleLanguage;
const currentLanguage = computed(() => languageStore.currentLanguage);

// Load sidebar mode setting
async function loadSidebarModeSetting() {
  try {
    const settings = await apiClient.getSettings(['ui.sidebar_mode']);
    const mode = settings.find((s: { key: string; value: string }) => s.key === 'ui.sidebar_mode')?.value;
    if (mode === 'tree' || mode === 'flat') {
      sidebarMode.value = mode;
    }
  } catch (e) {
    console.error('Failed to load sidebar mode setting:', e);
  }
}

// ==================== Navigation ====================
function handleNavigate(routeParam: { path: string; query?: Record<string, string> }) {
  router.push({ path: routeParam.path, query: routeParam.query });
  if (isMobile.value) {
    sidebarOpen.value = false;
  }
}

function handleToast(type: 'success' | 'error' | 'warning' | 'info', message: string) {
  toastRef.value?.showToast(type, message);
}

// Handle topic selection from search
async function handleSearchTopicSelect(cluster: string, topic: string) {
  router.push({ path: '/messages', query: { cluster, topic } });
  if (sidebarMode.value === 'tree') {
    clusterTreeNavigatorRef.value?.highlightAndSelectTopic(topic, cluster);
  }
}

// Handle topic selection in tree mode
function handleSelectTopicInTree(topicName: string, clusterName: string) {
  if (sidebarMode.value === 'tree') {
    clusterTreeNavigatorRef.value?.highlightAndSelectTopic(topicName, clusterName);
  } else {
    router.push({
      path: '/messages',
      query: { cluster: clusterName, topic: topicName },
    });
  }
}

// Handle settings changes
function handleSettingsChanged(event: CustomEvent) {
  const { key, value } = event.detail;
  if (key === 'ui.sidebar_mode' && (value === 'tree' || value === 'flat')) {
    sidebarMode.value = value;
  }
}

// Handle navigate from favorites
function handleNavigateFromFavorites(clusterId: string, topicName: string) {
  if (sidebarMode.value === 'tree') {
    clusterTreeNavigatorRef.value?.highlightAndSelectTopic(topicName, clusterId);
  } else {
    router.push({
      path: '/messages',
      query: { cluster: clusterId, topic: topicName },
    });
    if (isMobile.value) {
      sidebarOpen.value = false;
    }
  }
}

// ==================== Cluster Actions ====================
const refreshingCluster = ref<string | null>(null);
const disconnectingCluster = ref<string | null>(null);
const reconnectingCluster = ref<string | null>(null);

async function handleClusterAction(action: string, cluster: string) {
  switch (action) {
    case 'viewTopics':
      router.push({ path: '/topics', query: { cluster } });
      break;
    case 'refreshTopics':
      refreshClusterTopics(cluster);
      break;
    case 'refreshConnection':
      refreshConnectionStatus(cluster);
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
    case 'editCluster':
      editCluster(cluster);
      break;
    case 'deleteCluster':
      if (await toastRef.value?.showConfirm(t.value.layout.confirmDeleteCluster.replace('{cluster}', cluster))) {
        const clusterId = clusterStore.clusters.find((c) => c.name === cluster)?.id;
        if (clusterId) {
          clusterStore.deleteCluster(clusterId);
        } else {
          toastRef.value?.showToast('error', t.value.layout.clusterNotFound);
        }
      }
      break;
  }
}

async function testConnection(clusterName: string) {
  const cluster = clusterStore.clusters.find((c) => c.name === clusterName);
  if (!cluster) return;

  contextMenusRef.value!.testingCluster = clusterName;
  try {
    const result = await clusterStore.testCluster(cluster.id);
    if (result.success) {
      toastRef.value?.showToast('success', t.value.clusters.connected);
    } else {
      toastRef.value?.showToast('error', t.value.clusters.connectionError);
    }
  } catch (e) {
    toastRef.value?.showToast('error', `${t.value.clusters.connectionError}: ${(e as { message: string }).message}`);
  } finally {
    contextMenusRef.value!.testingCluster = null;
  }
}

async function refreshConnectionStatus(clusterName: string) {
  contextMenusRef.value!.refreshingCluster = clusterName;
  try {
    const health = await apiClient.healthCheckCluster(clusterName);
    clusterStore.clusterHealth[clusterName] = {
      clusterId: clusterName,
      healthy: health.healthy,
      lastChecked: Date.now(),
      error: health.error_message,
    };
    if (health.healthy) {
      toastRef.value?.showToast('success', 'Cluster status refreshed');
    } else {
      toastRef.value?.showToast('error', `Cluster connection issue: ${health.error_message || 'Unknown error'}`);
    }
  } catch (e) {
    toastRef.value?.showToast('error', `Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    contextMenusRef.value!.refreshingCluster = null;
  }
}

async function disconnectCluster(clusterName: string) {
  if (await toastRef.value?.showConfirm(t.value.clusters.disconnectConfirm.replace('{cluster}', clusterName))) {
    contextMenusRef.value!.disconnectingCluster = clusterName;
    try {
      await connectionStore.disconnectCluster(clusterName);
      clusterStore.clusterHealth[clusterName] = {
        clusterId: clusterName,
        healthy: false,
        lastChecked: Date.now(),
        error: 'Disconnected',
      };
      await connectionStore.fetchAllConnections();
      toastRef.value?.showToast('success', 'Cluster disconnected successfully');
    } catch (e) {
      toastRef.value?.showToast('error', `Disconnect failed: ${(e as { message: string }).message}`);
    } finally {
      contextMenusRef.value!.disconnectingCluster = null;
    }
  }
}

async function reconnectCluster(clusterName: string) {
  contextMenusRef.value!.reconnectingCluster = clusterName;
  try {
    await connectionStore.reconnectCluster(clusterName);
    const health = await apiClient.healthCheckCluster(clusterName);
    clusterStore.clusterHealth[clusterName] = {
      clusterId: clusterName,
      healthy: health.healthy,
      lastChecked: Date.now(),
      error: health.error_message,
    };
    await connectionStore.fetchAllConnections();
    await clusterStore.fetchClusters();
    toastRef.value?.showToast('success', 'Cluster reconnected successfully');
  } catch (e) {
    toastRef.value?.showToast('error', `Reconnect failed: ${(e as { message: string }).message}`);
  } finally {
    contextMenusRef.value!.reconnectingCluster = null;
  }
}

function editCluster(clusterName: string) {
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
  if (contextMenusRef.value!.refreshingCluster === cluster) {
    apiClient.cancelRequest();
    contextMenusRef.value!.refreshingCluster = null;
    return;
  }

  contextMenusRef.value!.refreshingCluster = cluster;
  try {
    await apiClient.refreshTopics(cluster);
    window.dispatchEvent(new CustomEvent('cluster-topics-refreshed', { detail: { cluster } }));
  } catch (e) {
    const error = e as { message: string };
    if (!error.message.includes('aborted')) {
      toastRef.value?.showToast('error', `${t.value.layout.refreshFailed}: ${error.message}`);
    }
  } finally {
    contextMenusRef.value!.refreshingCluster = null;
  }
}

async function handleTopicAction(action: string, topic: string, cluster: string) {
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
      if (await toastRef.value?.showConfirm(t.value.layout.confirmDeleteTopic.replace('{topic}', topic))) {
        try {
          await apiClient.deleteTopic(cluster, topic);
          toastRef.value?.showToast('success', 'Topic deleted successfully');
          window.dispatchEvent(new CustomEvent('topic-deleted', { detail: { cluster, topic } }));
        } catch (e) {
          toastRef.value?.showToast('error', `${t.value.toast.operationFailed}: ${(e as { message: string }).message}`);
        }
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

// Context menu handlers
function showClusterMenuFromTree(event: MouseEvent, clusterName: string) {
  contextMenusRef.value?.showClusterMenu(clusterName, event.clientX, event.clientY);
}

function showTopicsFolderMenuFromTree(event: MouseEvent, clusterName: string) {
  contextMenusRef.value?.showTopicsFolderMenu(clusterName, event.clientX, event.clientY);
}

function showTopicMenuFromTree(event: MouseEvent, topicName: string, clusterName: string) {
  contextMenusRef.value?.showTopicMenu(topicName, clusterName, event.clientX, event.clientY);
}

function showPartitionMenuFromTree(event: MouseEvent, topicName: string, clusterName: string, partitionId: number) {
  contextMenusRef.value?.showPartitionMenu(topicName, clusterName, partitionId, event.clientX, event.clientY);
}

// ==================== State Persistence ====================
const STORAGE_KEY = 'kafka-manager-last-state';

interface LastState {
  path: string;
  query?: Record<string, string>;
  timestamp: number;
}

function saveCurrentState() {
  try {
    const state: LastState = {
      path: router.currentRoute.value.path,
      query: router.currentRoute.value.query as Record<string, string>,
      timestamp: Date.now(),
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch (e) {
    console.warn('[State] Failed to save state:', e);
  }
}

function restorePreviousState() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (!saved) return;

    const state: LastState = JSON.parse(saved);
    if (state.path && state.path !== '/' && state.path !== '/clusters') {
      router.replace({ path: state.path, query: state.query });
    }
  } catch (e) {
    console.warn('[State] Failed to restore state:', e);
  }
}

router.afterEach(() => {
  saveCurrentState();
});

// Global keydown handler
function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
    e.preventDefault();
    // Focus search input in TopNavBar
  }
}

// Handle open create cluster modal
function handleOpenCreateClusterModal() {
  router.push({ path: '/clusters', query: { action: 'create', t: String(Date.now()) } });
}

// Lifecycle
onMounted(async () => {
  themeStore.initTheme();
  languageStore.initLanguage();
  await clusterStore.fetchClusters();

  restorePreviousState();
  checkMobile();
  window.addEventListener('resize', checkMobile);
  document.addEventListener('keydown', handleGlobalKeydown);

  window.addEventListener('select-topic-in-tree', ((e: Event) => {
    const event = e as CustomEvent<{ topicName: string; clusterName: string }>;
    const { topicName, clusterName } = event.detail;
    handleSelectTopicInTree(topicName, clusterName);
  }) as EventListener);

  window.addEventListener('open-create-cluster-modal', handleOpenCreateClusterModal);
  window.addEventListener('settings-changed', ((e: Event) => {
    const event = e as CustomEvent<{ key: string; value: any }>;
    handleSettingsChanged(event);
  }) as EventListener);
  window.addEventListener('navigate-to-topic-from-favorites', ((e: Event) => {
    const event = e as CustomEvent<{ clusterId: string; topicName: string }>;
    const { clusterId, topicName } = event.detail;
    handleNavigateFromFavorites(clusterId, topicName);
  }) as EventListener);
});

onUnmounted(() => {
  window.removeEventListener('resize', checkMobile);
  document.removeEventListener('keydown', handleGlobalKeydown);
  window.removeEventListener('open-create-cluster-modal', handleOpenCreateClusterModal);
  window.removeEventListener('select-topic-in-tree', ((e: Event) => {
    const event = e as CustomEvent<{ topicName: string; clusterName: string }>;
    const { topicName, clusterName } = event.detail;
    handleSelectTopicInTree(topicName, clusterName);
  }) as EventListener);
  window.removeEventListener('settings-changed', ((e: Event) => {
    const event = e as CustomEvent<{ key: string; value: any }>;
    handleSettingsChanged(event);
  }) as EventListener);
  window.removeEventListener('navigate-to-topic-from-favorites', ((e: Event) => {
    const event = e as CustomEvent<{ clusterId: string; topicName: string }>;
    const { clusterId, topicName } = event.detail;
    handleNavigateFromFavorites(clusterId, topicName);
  }) as EventListener);
});
</script>

<style scoped>
/* Resizer 样式 */
.resizer:hover {
  background: rgba(128, 128, 128, 0.2);
}

/* Glass morphism effect */
.glass {
  background: rgba(var(--glass-bg, 255, 255, 255), 0.1);
  backdrop-filter: blur(10px);
}

/* Gradient border */
.gradient-border {
  position: relative;
  background: linear-gradient(135deg, var(--gradient-from, #667eea), var(--gradient-to, #764ba2));
  padding: 1px;
}

.gradient-border > * {
  background: var(--glass-bg, base-100);
  height: 100%;
}
</style>
