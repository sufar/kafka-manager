<template>
  <div class="flex h-screen overflow-hidden" ref="layoutContainer">
    <!-- Sidebar -->
    <div
      class="bg-base-100 min-h-screen shadow-xl flex flex-col flex-shrink-0 transition-all duration-75"
      :style="{ width: `${sidebarWidth}px` }"
    >
      <!-- Logo -->
      <div class="p-4 border-b border-base-200 flex-shrink-0">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-primary flex items-center justify-center">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-primary-content">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
            </svg>
          </div>
          <div>
            <h1 class="text-xl font-bold text-primary">Kafka Manager</h1>
            <p class="text-xs text-base-content/60">{{ t.mainLayout.multiCluster }}</p>
          </div>
        </div>
      </div>

      <!-- Navigation -->
      <nav class="p-4 flex-shrink-0">
        <ul class="menu menu-md gap-1">
          <li>
            <router-link to="/dashboard" active-class="menu-active">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="m3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6ZM3.75 15.75A2.25 2.25 0 0 1 6 13.5h2.25a2.25 2.25 0 0 1 2.25 2.25V18a2.25 2.25 0 0 1-2.25 2.25H6A2.25 2.25 0 0 1 3.75 18v-2.25ZM13.5 6a2.25 2.25 0 0 1 2.25-2.25H18A2.25 2.25 0 0 1 20.25 6v2.25A2.25 2.25 0 0 1 18 10.5h-2.25a2.25 2.25 0 0 1-2.25-2.25V6ZM13.5 15.75a2.25 2.25 0 0 1 2.25-2.25H18a2.25 2.25 0 0 1 2.25 2.25V18A2.25 2.25 0 0 1 18 20.25h-2.25A2.25 2.25 0 0 1 13.5 18v-2.25Z" />
              </svg>
              {{ t.nav.dashboard }}
            </router-link>
          </li>
          <li>
            <router-link to="/clusters" active-class="menu-active">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
              </svg>
              {{ t.nav.clusters }}
            </router-link>
          </li>
        </ul>
      </nav>

      <!-- Cluster Navigation & Selection -->
      <div class="flex-1 overflow-y-auto px-4 py-2">
        <div class="flex items-center justify-between mb-2 flex-shrink-0">
          <span class="text-xs font-semibold text-base-content/60 uppercase">{{ t.mainLayout.clustersLabel }}</span>
          <div class="flex gap-1 flex-shrink-0">
            <button
              class="btn btn-xs btn-ghost"
              @click="selectAllClusters"
              :title="t.mainLayout.selectAll"
            >
              {{ t.mainLayout.selectAll }}
            </button>
            <button
              class="btn btn-xs btn-ghost"
              @click="clearSelection"
              :title="t.mainLayout.clearSelection"
            >
              {{ t.mainLayout.clearSelection }}
            </button>
          </div>
        </div>
        <ul class="menu w-full gap-1">
          <li v-for="cluster in clusters" :key="cluster.id">
            <details :open="isClusterExpanded(cluster.name)" @toggle="onClusterToggle(cluster.name, $event)">
              <summary class="cursor-pointer hover:bg-base-200 rounded-lg p-2 flex items-center gap-2">
                <input
                  type="checkbox"
                  class="checkbox checkbox-xs"
                  :checked="selectedClusterIds.includes(cluster.name)"
                  @change="toggleCluster(cluster.name)"
                  @click.stop
                />
                <span
                  class="w-2 h-2 rounded-full flex-shrink-0"
                  :class="getHealthClass(cluster.name)"
                ></span>
                <span class="font-medium truncate flex-1">{{ cluster.name }}</span>
                <button
                  class="btn btn-ghost btn-xs flex-shrink-0"
                  @click.stop="testConnection(cluster.id)"
                  :disabled="testing.has(cluster.id)"
                >
                  <span v-if="testing.has(cluster.id)" class="loading loading-spinner loading-xs"></span>
                  <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                  </svg>
                </button>
              </summary>
              <ul class="ml-4">
                <li>
                  <router-link :to="`/topics?cluster=${cluster.name}`" active-class="menu-active" @click.stop="connectCluster(cluster.name)">
                    {{ t.mainLayout.topics }}
                  </router-link>
                </li>
                <li>
                  <router-link :to="`/consumer-groups?cluster=${cluster.name}`" active-class="menu-active" @click.stop="connectCluster(cluster.name)">
                    {{ t.mainLayout.consumerGroups }}
                  </router-link>
                </li>
                <li>
                  <router-link :to="`/messages?cluster=${cluster.name}`" active-class="menu-active" @click.stop="connectCluster(cluster.name)">
                    {{ t.mainLayout.messages }}
                  </router-link>
                </li>
                <li>
                  <router-link :to="`/schema-registry?cluster=${cluster.name}`" active-class="menu-active" @click.stop="connectCluster(cluster.name)">
                    {{ t.mainLayout.schemaRegistry }}
                  </router-link>
                </li>
                <li>
                  <router-link :to="`/acls?cluster=${cluster.name}`" active-class="menu-active" @click.stop="connectCluster(cluster.name)">
                    {{ t.mainLayout.acls }}
                  </router-link>
                </li>
              </ul>
            </details>
          </li>
        </ul>
      </div>

      <!-- Add Cluster Button -->
      <div class="p-4 border-t border-base-200 flex-shrink-0">
        <router-link to="/clusters" class="btn btn-sm btn-block btn-outline">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          {{ t.mainLayout.manageClusters }}
        </router-link>
      </div>
    </div>

    <!-- Resizer -->
    <div
      class="resizer flex-shrink-0 cursor-col-resize hover:bg-primary hover:bg-opacity-20 transition-colors z-50"
      @mousedown="startResize"
      :class="{ 'bg-primary': isResizing }"
    >
      <div class="resizer-handle"></div>
    </div>

    <!-- Main content -->
    <div class="flex-1 flex flex-col min-h-screen bg-base-200 overflow-hidden">
      <!-- Navbar -->
      <header class="navbar bg-base-100 shadow-sm sticky top-0 z-40 flex-shrink-0">
        <div class="flex-1">
          <!-- Selected clusters info -->
          <div class="flex items-center gap-2">
            <span class="text-sm text-base-content/60">
              {{ selectedClusterIds.length }} {{ t.mainLayout.clustersSelected }}
            </span>
          </div>
        </div>
        <div class="flex-none gap-2">
          <button
            class="btn btn-sm btn-ghost"
            :class="{ 'btn-active': refreshingHealth }"
            @click="handleRefreshHealth"
            :title="t.mainLayout.refreshHealth"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              class="w-5 h-5"
              :class="{ 'animate-spin': refreshingHealth }"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.058A10.98 10.98 0 0112 3.325 10.98 10.98 0 0120 8.05m0 0v5.95m0-5.95h-5.95M4 15.95h5.95m-5.95 0V10" />
            </svg>
          </button>
          <slot name="navbar-end"></slot>
        </div>
      </header>

      <!-- Page content -->
      <main class="flex-1 p-6 overflow-auto">
        <router-view />
      </main>

      <!-- Footer -->
      <footer class="footer footer-center p-4 bg-base-100 text-base-content flex-shrink-0">
        <div>
          <p>{{ t.mainLayout.footerText }}</p>
        </div>
      </footer>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';

const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const { t } = storeToRefs(languageStore);

const clusters = computed(() => clusterStore.clusters);
const selectedClusterIds = computed(() => clusterStore.selectedClusterIds);
const refreshingHealth = computed(() => clusterStore.refreshingHealth);

const testing = ref(new Set<number>());

// 侧边栏宽度控制和调整大小
const SIDEBAR_MIN_WIDTH = 200;
const SIDEBAR_MAX_WIDTH = 600;
const SIDEBAR_DEFAULT_WIDTH = 280;

const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH);
const isResizing = ref(false);

// 从 localStorage 加载宽度
function loadSidebarWidth() {
  const saved = localStorage.getItem('kafka-manager-sidebar-width');
  if (saved) {
    const width = parseInt(saved, 10);
    if (width >= SIDEBAR_MIN_WIDTH && width <= SIDEBAR_MAX_WIDTH) {
      sidebarWidth.value = width;
    }
  }
}

// 保存宽度到 localStorage
function saveSidebarWidth() {
  localStorage.setItem('kafka-manager-sidebar-width', sidebarWidth.value.toString());
}

// 开始调整大小
function startResize(e: MouseEvent) {
  isResizing.value = true;
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';

  const startX = e.clientX;
  const startWidth = sidebarWidth.value;

  const handleMouseMove = (event: MouseEvent) => {
    const delta = event.clientX - startX;
    let newWidth = startWidth + delta;
    newWidth = Math.max(SIDEBAR_MIN_WIDTH, Math.min(SIDEBAR_MAX_WIDTH, newWidth));
    sidebarWidth.value = newWidth;
  };

  const handleMouseUp = () => {
    isResizing.value = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    saveSidebarWidth();
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
  };

  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', handleMouseUp);
}

// 集群展开状态（localStorage 持久化）
const expandedClusters = ref<Set<string>>(new Set());

function initExpandedClusters() {
  const saved = localStorage.getItem('kafka-manager-expanded-clusters');
  if (saved) {
    try {
      const names = JSON.parse(saved);
      expandedClusters.value = new Set(names);
    } catch {
      expandedClusters.value = new Set();
    }
  }
}

function isClusterExpanded(clusterName: string): boolean {
  return expandedClusters.value.has(clusterName);
}

function onClusterToggle(clusterName: string, event: Event) {
  const target = event.target as HTMLDetailsElement;
  // 使用 setTimeout 等待 DOM 更新后读取 open 状态
  setTimeout(() => {
    if (target.open) {
      expandedClusters.value.add(clusterName);
    } else {
      expandedClusters.value.delete(clusterName);
    }
    localStorage.setItem('kafka-manager-expanded-clusters', JSON.stringify([...expandedClusters.value]));
  }, 0);
}

function getHealthClass(clusterId: string): string {
  const health = clusterStore.getClusterHealth(clusterId);
  if (!health) return 'bg-base-300';
  if (health.healthy) return 'bg-success';
  return 'bg-error';
}

function toggleCluster(clusterId: string) {
  clusterStore.toggleClusterSelection(clusterId);
}

function selectAllClusters() {
  clusterStore.selectAllClusters();
}

function clearSelection() {
  clusterStore.clearSelection();
}

async function handleRefreshHealth() {
  await clusterStore.refreshAllHealth();
}

async function testConnection(id: number) {
  testing.value.add(id);
  try {
    await clusterStore.testCluster(id);
  } finally {
    testing.value.delete(id);
  }
}

// 连接集群（如果未连接）
async function connectCluster(clusterName: string): Promise<void> {
  const health = clusterStore.getClusterHealth(clusterName);
  // 如果集群未连接或不健康，尝试重连
  if (!health || !health.healthy) {
    try {
      await apiClient.reconnectCluster(clusterName);
      // 重连后刷新健康状态
      await clusterStore.refreshAllHealth();
    } catch (e) {
      console.error(`Failed to connect cluster ${clusterName}:`, e);
    }
  }
}

onMounted(() => {
  loadSidebarWidth();
  initExpandedClusters();
  clusterStore.fetchClusters();
});
</script>

<style scoped>
.resizer {
  width: 6px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  transition: background-color 0.2s;
}

.resizer:hover {
  background: rgba(128, 128, 128, 0.1);
}

.resizer-handle {
  width: 2px;
  height: 40px;
  background: rgba(128, 128, 128, 0.3);
  border-radius: 1px;
  transition: background-color 0.2s;
}

.resizer:hover .resizer-handle {
  background: rgba(128, 128, 128, 0.6);
}

.resizer:active .resizer-handle {
  background: currentColor;
}
</style>
