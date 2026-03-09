<template>
  <div class="p-6 relative overflow-hidden">
    <!-- Animated background particles -->
    <div class="absolute inset-0 overflow-hidden pointer-events-none">
      <div class="particle particle-1"></div>
      <div class="particle particle-2"></div>
    </div>

    <!-- Page Header -->
    <div class="mb-8 relative">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold text-gradient flex items-center gap-3">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-10 h-10 animate-float">
              <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
            </svg>
            {{ t.clusters.title }}
          </h1>
          <p class="text-base-content/60 mt-2 text-lg">{{ t.clusters.description }}</p>
        </div>
        <button class="btn btn-primary" @click="openCreateModal">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          {{ t.clusters.addCluster }}
        </button>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex justify-center items-center py-20">
      <div class="flex flex-col items-center">
        <span class="loading loading-spinner loading-lg text-primary"></span>
        <p class="mt-4 text-base-content/60">{{ t.common.loading }}...</p>
      </div>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="flex flex-col items-center justify-center py-16 text-center">
      <div class="text-base-content/40 mb-6">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-20 h-20">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
        </svg>
      </div>
      <h3 class="text-xl font-semibold mb-2">{{ t.clusters.connectionError }}</h3>
      <p class="text-base-content/60 mb-4 max-w-md">{{ error }}</p>
      <div class="flex gap-3">
        <button class="btn btn-primary" @click="refreshClusters">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
          </svg>
          {{ t.clusters.retry }}
        </button>
        <button class="btn btn-outline" @click="openCreateModal">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          {{ t.clusters.addCluster }}
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div v-else-if="clusters.length === 0" class="flex flex-col items-center justify-center py-16 text-center">
      <div class="text-base-content/40 mb-6">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-20 h-20">
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
        </svg>
      </div>
      <h3 class="text-xl font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-6">{{ t.clusters.description }}</p>
      <button class="btn btn-primary" @click="openCreateModal">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        {{ t.clusters.addCluster }}
      </button>
    </div>

    <!-- Clusters Grid -->
    <div v-else class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="cluster in clusters"
        :key="cluster.id"
        class="card glass gradient-border hover:shadow-xl hover:-translate-y-0.5 transition-all duration-200"
      >
        <div class="flex items-center justify-between p-5 border-b border-base-content/10">
          <div class="flex items-center gap-3">
            <div class="flex items-center justify-center w-10 h-10 rounded-xl bg-primary/10 glow-primary text-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
              </svg>
            </div>
            <div>
              <h3 class="font-semibold text-sm">{{ cluster.name }}</h3>
              <p class="text-xs text-base-content/60">Created {{ formatDate(cluster.created_at) }}</p>
            </div>
          </div>
          <div
            class="badge gap-1.5"
            :class="{
              'badge-success': getConnectionStatus(cluster.name)?.status === 'connected',
              'badge-error': getConnectionStatus(cluster.name)?.status === 'error',
              'badge-ghost': getConnectionStatus(cluster.name)?.status === 'disconnected',
            }"
          >
            <div
              class="w-2 h-2 rounded-full"
              :class="{
                'bg-success animate-pulse': getConnectionStatus(cluster.name)?.status === 'connected',
                'bg-error': getConnectionStatus(cluster.name)?.status === 'error',
              }"
            ></div>
            {{ getConnectionStatus(cluster.name)?.status || 'unknown' }}
          </div>
        </div>

        <div class="card-body p-5">
          <div class="mb-3">
            <div class="text-[10px] uppercase tracking-wider text-base-content/60 mb-1">Brokers</div>
            <div class="text-sm font-mono">{{ cluster.brokers }}</div>
          </div>
          <div class="mb-3">
            <div class="text-[10px] uppercase tracking-wider text-base-content/60 mb-1">Timeouts</div>
            <div class="text-sm">
              <div>Request: <span class="font-mono">{{ cluster.request_timeout_ms }}ms</span></div>
              <div>Operation: <span class="font-mono">{{ cluster.operation_timeout_ms }}ms</span></div>
            </div>
          </div>

          <div v-if="getConnectionStatus(cluster.name)?.error_message" class="alert alert-error py-2 px-3 mt-3">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
            </svg>
            <span class="text-sm">{{ getConnectionStatus(cluster.name)?.error_message }}</span>
          </div>
        </div>

        <div class="card-actions justify-start p-4 bg-base-200">
          <button
            class="btn btn-sm btn-outline"
            @click="testConnection(cluster.id)"
            :disabled="testing.has(cluster.id)"
          >
            <span v-if="testing.has(cluster.id)" class="loading loading-spinner loading-sm"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z" />
            </svg>
            Test
          </button>
          <button
            class="btn btn-sm btn-outline"
            @click="refreshConnectionStatus(cluster.name)"
            :disabled="refreshing.has(cluster.name)"
          >
            <span v-if="refreshing.has(cluster.name)" class="loading loading-spinner loading-sm"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            Refresh
          </button>
          <button
            class="btn btn-sm btn-ghost"
            @click="disconnectCluster(cluster.name)"
            :disabled="disconnecting.has(cluster.name)"
          >
            Disconnect
          </button>
          <button
            class="btn btn-sm btn-ghost"
            @click="reconnectCluster(cluster.name)"
            :disabled="reconnecting.has(cluster.name)"
          >
            <span v-if="reconnecting.has(cluster.name)" class="loading loading-spinner loading-sm"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            Reconnect
          </button>
          <button
            class="btn btn-sm btn-ghost"
            @click="editCluster(cluster)"
          >
            Edit
          </button>
          <button
            class="btn btn-sm btn-ghost text-error hover:bg-error/10"
            @click="confirmDelete(cluster)"
          >
            Delete
          </button>
        </div>
      </div>
    </div>

    <!-- Create/Edit Modal using Teleport and DaisyUI modal -->
    <Teleport to="body">
      <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle">
        <div class="modal-box">
          <!-- close button -->
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="closeModal">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
          <h3 class="font-bold text-xl mt-2 mb-4">{{ editingCluster ? t.clusters.editCluster : t.clusters.createCluster }}</h3>
          <form @submit.prevent="handleSubmit" class="flex flex-col gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ t.clusters.clusterName }}</span>
              </label>
              <input
                v-model="formData.name"
                type="text"
                placeholder="my-cluster"
                class="input input-bordered w-full"
                required
              />
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ t.clusters.brokers }}</span>
              </label>
              <input
                v-model="formData.brokers"
                type="text"
                placeholder="localhost:9092,localhost:9093"
                class="input input-bordered w-full"
                required
              />
              <label class="label">
                <span class="label-text-alt text-base-content/60">Comma-separated list of broker addresses</span>
              </label>
            </div>
            <div class="flex flex-wrap gap-4">
              <div class="form-control w-auto">
                <label class="label">
                  <span class="label-text font-medium">{{ t.clusters.requestTimeout }}</span>
                </label>
                <input
                  v-model.number="formData.request_timeout_ms"
                  type="number"
                  class="input input-bordered w-40"
                  placeholder="30000"
                />
              </div>
              <div class="form-control w-auto">
                <label class="label">
                  <span class="label-text font-medium">{{ t.clusters.operationTimeout }}</span>
                </label>
                <input
                  v-model.number="formData.operation_timeout_ms"
                  type="number"
                  class="input input-bordered w-40"
                  placeholder="30000"
                />
              </div>
            </div>
            <div class="modal-action mt-4">
              <button type="button" class="btn btn-outline" @click="closeModal">{{ t.common.cancel }}</button>
              <button type="submit" class="btn btn-primary" :disabled="submitting">
                <span v-if="submitting" class="loading loading-spinner loading-sm"></span>
                {{ editingCluster ? t.common.edit : t.common.create }}
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeModal">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useClusterConnectionStore } from '@/stores/clusterConnection';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import type { Cluster } from '@/types/api';

const route = useRoute();
const router = useRouter();
const clusterStore = useClusterStore();
const connectionStore = useClusterConnectionStore();
const languageStore = useLanguageStore();
const { showError, showSuccess } = useToast();

const clusters = computed(() => clusterStore.clusters);
const loading = computed(() => clusterStore.loading);
const error = computed(() => clusterStore.error);

// 翻译
const t = computed(() => languageStore.t);

const editingCluster = ref<Cluster | null>(null);
const testing = ref(new Set<number>());
const refreshing = ref(new Set<string>());
const disconnecting = ref(new Set<string>());
const reconnecting = ref(new Set<string>());
const submitting = ref(false);

const formData = reactive({
  name: '',
  brokers: '',
  request_timeout_ms: 30000,
  operation_timeout_ms: 30000,
});

const modalRef = ref<HTMLDialogElement>();

function openCreateModal() {
  editingCluster.value = null;
  formData.name = '';
  formData.brokers = '';
  formData.request_timeout_ms = 30000;
  formData.operation_timeout_ms = 30000;
  modalRef.value?.showModal();
}

function editCluster(cluster: Cluster) {
  editingCluster.value = cluster;
  formData.name = cluster.name;
  formData.brokers = cluster.brokers;
  formData.request_timeout_ms = cluster.request_timeout_ms;
  formData.operation_timeout_ms = cluster.operation_timeout_ms;
  modalRef.value?.showModal();
}

function closeModal() {
  modalRef.value?.close();
  editingCluster.value = null;
  formData.name = '';
  formData.brokers = '';
  formData.request_timeout_ms = 30000;
  formData.operation_timeout_ms = 30000;
  // 清除路由参数
  router.replace({ path: '/clusters', query: {} });
}

async function handleSubmit() {
  submitting.value = true;
  try {
    if (editingCluster.value) {
      await clusterStore.updateCluster(editingCluster.value.id, {
        name: formData.name,
        brokers: formData.brokers,
        request_timeout_ms: formData.request_timeout_ms,
        operation_timeout_ms: formData.operation_timeout_ms,
      });
    } else {
      await clusterStore.createCluster({
        name: formData.name,
        brokers: formData.brokers,
        request_timeout_ms: formData.request_timeout_ms,
        operation_timeout_ms: formData.operation_timeout_ms,
      });
    }
    showSuccess(editingCluster.value ? t.value.clusters.updated : t.value.clusters.created);
    closeModal();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    submitting.value = false;
  }
}

async function testConnection(id: number) {
  testing.value.add(id);
  try {
    const result = await clusterStore.testCluster(id);
    if (result.success) {
      showSuccess(t.value.clusters.connected);
    } else {
      showError(t.value.clusters.connectionError);
    }
  } catch (e) {
    showError(`${t.value.clusters.connectionError}: ${(e as { message: string }).message}`);
  } finally {
    testing.value.delete(id);
  }
}

function confirmDelete(cluster: Cluster) {
  if (confirm(`Are you sure you want to delete cluster "${cluster.name}"?`)) {
    clusterStore.deleteCluster(cluster.id);
  }
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

function getConnectionStatus(clusterName: string) {
  return connectionStore.getConnectionStatus(clusterName);
}

async function refreshConnectionStatus(clusterName: string) {
  refreshing.value.add(clusterName);
  try {
    const status = await connectionStore.fetchConnectionStatus(clusterName);
    const statusText = status.status === 'connected' ? t.value.clusters.connected :
                       status.status === 'error' ? t.value.clusters.connectionError :
                       status.status;
    showSuccess(`Status: ${statusText}`);
  } catch (e) {
    showError(`Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value.delete(clusterName);
  }
}

async function disconnectCluster(clusterName: string) {
  if (confirm(t.value.clusters.disconnectConfirm.replace('{cluster}', clusterName))) {
    disconnecting.value.add(clusterName);
    try {
      await connectionStore.disconnectCluster(clusterName);
      await connectionStore.fetchAllConnections();
      showSuccess('Cluster disconnected successfully');
    } catch (e) {
      showError(`Disconnect failed: ${(e as { message: string }).message}`);
    } finally {
      disconnecting.value.delete(clusterName);
    }
  }
}

async function reconnectCluster(clusterName: string) {
  reconnecting.value.add(clusterName);
  try {
    await connectionStore.reconnectCluster(clusterName);
    await connectionStore.fetchAllConnections();
    await clusterStore.fetchClusters();
    const status = connectionStore.getConnectionStatus(clusterName);
    const statusText = status?.status === 'connected' ? t.value.clusters.connected :
                       status?.status === 'error' ? t.value.clusters.connectionError :
                       status?.status || 'unknown';
    showSuccess(`Reconnected: ${statusText}`);
  } catch (e) {
    showError(`Reconnect failed: ${(e as { message: string }).message}`);
  } finally {
    reconnecting.value.delete(clusterName);
  }
}

async function refreshClusters() {
  await clusterStore.fetchClusters();
  await connectionStore.fetchAllConnections();
}

onMounted(() => {
  clusterStore.fetchClusters();
  connectionStore.fetchAllConnections();

  // 检查路由参数，如果 action=create 则打开创建模态框
  if (route.query.action === 'create') {
    setTimeout(() => {
      openCreateModal();
    }, 50);
  }
  // 检查路由参数，如果 action=edit 则打开编辑模态框
  if (route.query.action === 'edit' && route.query.cluster) {
    const clusterToEdit = clusters.value.find(c => c.name === route.query.cluster);
    if (clusterToEdit) {
      setTimeout(() => {
        editCluster(clusterToEdit);
      }, 50);
    }
  }
});

// 监听路由参数变化
watch(() => route.fullPath, (newPath, oldPath) => {
  if (newPath !== oldPath && route.query.action === 'create') {
    // 确保 modal 完全关闭后再打开
    setTimeout(() => {
      openCreateModal();
    }, 50);
  }
  // 监听编辑集群
  if (newPath !== oldPath && route.query.action === 'edit' && route.query.cluster) {
    const clusterToEdit = clusters.value.find(c => c.name === route.query.cluster);
    if (clusterToEdit) {
      setTimeout(() => {
        editCluster(clusterToEdit);
      }, 50);
    }
  }
});
</script>
