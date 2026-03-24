<template>
  <div class="p-3">
    <!-- Page Header -->
    <div class="mb-4">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
            </svg>
            {{ t.consumerGroups.title }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">
            <span v-if="clusterParam">{{ t.clusters.clusters }}: <span class="font-medium">{{ clusterParam }}</span></span>
            <span v-else>{{ t.consumerGroups.description }}</span>
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            class="btn btn-xs btn-outline"
            @click="refreshConsumerGroups"
            :disabled="refreshing || !clusterParam"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            <span class="hidden md:inline ml-1">{{ t.common.refresh }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- No cluster selected -->
    <div v-if="!clusterParam" class="flex flex-col items-center justify-center py-8 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-4 text-sm">{{ t.consumerGroups.description }}</p>
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

    <!-- Consumer Groups list -->
    <div v-else-if="clusterParam && filteredConsumerGroups.length > 0" class="card glass gradient-border shadow-xl">
      <!-- Search Bar -->
      <div class="p-3 bg-base-100">
        <div class="relative w-full">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
          </svg>
          <input
            v-model="searchQuery"
            type="text"
            :placeholder="t.common.search"
            class="input input-bordered w-full pl-10"
          />
        </div>
      </div>
      <!-- Table Header -->
      <div class="bg-base-100 border-b border-base-200">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="p-2">
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                  </svg>
                  <span class="text-sm font-semibold">{{ t.consumerGroups.groupName }}</span>
                </div>
              </th>
              <th class="p-2">
                <span class="text-sm font-semibold">{{ t.consumerGroups.topics }}</span>
              </th>
              <th class="p-2 text-right pr-4">{{ t.common.actions }}</th>
            </tr>
          </thead>
        </table>
      </div>
      <!-- Table Content -->
      <div ref="containerRef" class="overflow-y-auto" @scroll="handleScroll" style="max-height: calc(100vh - 350px);">
        <table class="table w-full">
          <tbody>
            <!-- Virtual scroll: top spacer -->
            <tr v-if="virtualStartIndex > 0" :style="{ height: virtualStartIndex * ROW_HEIGHT + 'px' }">
              <td colspan="3" style="padding: 0; border: 0;"></td>
            </tr>
            <!-- Visible rows -->
            <tr v-for="group in visibleConsumerGroups" :key="group.name" @dblclick="selectConsumerGroup(clusterParam, group)" class="hover cursor-pointer" :style="{ height: `${ROW_HEIGHT}px`, minHeight: `${ROW_HEIGHT}px` }">
              <td>
                <div class="flex items-center gap-2">
                  <div class="grid h-5 w-5 place-items-center rounded bg-base-300 text-base-content/70">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                    </svg>
                  </div>
                  <span class="font-medium text-sm">{{ group.name }}</span>
                </div>
              </td>
              <td>
                <div class="flex flex-wrap gap-1 max-w-xs">
                  <span v-for="topic in group.topics.slice(0, 3)" :key="topic" class="badge badge-ghost badge-xs">
                    {{ topic }}
                  </span>
                  <span v-if="group.topics.length > 3" class="badge badge-ghost badge-xs">
                    +{{ group.topics.length - 3 }}
                  </span>
                </div>
              </td>
              <td class="p-2">
                <div class="flex justify-end gap-0.5">
                  <button class="btn btn-ghost btn-xs" @click="viewConsumerGroup(group.name)" title="View">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" />
                      <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
                    </svg>
                  </button>
                  <button class="btn btn-ghost btn-xs" @click="refreshOffsets(group.name)" :title="t.common.refresh">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                    </svg>
                  </button>
                </div>
              </td>
            </tr>
            <!-- Virtual scroll: bottom spacer -->
            <tr v-if="virtualStartIndex + visibleConsumerGroups.length < filteredConsumerGroups.length" :style="{ height: (filteredConsumerGroups.length - virtualStartIndex - visibleConsumerGroups.length) * ROW_HEIGHT + 'px' }">
              <td colspan="3" style="padding: 0; border: 0;"></td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- No data state -->
    <div v-else-if="clusterParam && consumerGroups.length === 0" class="card glass gradient-border shadow-xl">
      <div class="flex flex-col items-center justify-center py-12 text-center">
        <div class="text-base-content/30 mb-4">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
            <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
          </svg>
        </div>
        <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
        <p class="text-base-content/60 mb-4 text-sm">{{ t.consumerGroups.emptyHelp }}</p>
        <button class="btn btn-sm btn-outline" @click="refreshConsumerGroups">{{ t.common.refresh }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

interface ConsumerGroupItem {
  name: string;
  cluster: string;
  topics: string[];
}

const route = useRoute();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

const clusterParam = computed(() => {
  const val = route.query.cluster;
  return Array.isArray(val) ? val[0] : (val || '');
});

const searchQuery = ref('');
const loading = ref(false);
const error = ref<string | null>(null);
const refreshing = ref(false);

const consumerGroups = ref<ConsumerGroupItem[]>([]);

// Virtual scroll
const ROW_HEIGHT = 40;
const VISIBLE_OFFSET = 5;
const containerRef = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const containerHeight = ref(0);

function handleScroll(event: Event) {
  const target = event.target as HTMLElement;
  if (!target) return;
  scrollTop.value = target.scrollTop;
  containerHeight.value = target.clientHeight;
}

const virtualStartIndex = computed(() => {
  return Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - VISIBLE_OFFSET);
});

const visibleConsumerGroups = computed(() => {
  const allGroups = filteredConsumerGroups.value;
  if (!allGroups.length) return [];

  const startIndex = virtualStartIndex.value;
  const containerH = containerHeight.value > 0 ? containerHeight.value : 400;
  const visibleCount = Math.ceil(containerH / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
  const endIndex = Math.min(allGroups.length, startIndex + visibleCount);

  return allGroups.slice(startIndex, endIndex);
});

const filteredConsumerGroups = computed(() => {
  if (!searchQuery.value) return consumerGroups.value;

  const query = searchQuery.value.toLowerCase();
  return consumerGroups.value.filter(group =>
    group.name.toLowerCase().includes(query) ||
    group.topics.some(topic => topic.toLowerCase().includes(query))
  );
});

watch(clusterParam, (newClusterParam) => {
  if (newClusterParam) {
    fetchConsumerGroups();
  }
}, { immediate: true });

async function fetchConsumerGroups() {
  loading.value = true;
  error.value = null;

  if (!clusterParam.value) {
    loading.value = false;
    return;
  }

  try {
    // Use consumer_group.list API to get groups with topics information
    const result = await apiClient.getConsumerGroupsList([clusterParam.value]);
    consumerGroups.value = result.groups.map((g) => ({
      name: g.group_name,
      cluster: g.cluster_id,
      topics: g.topics || [],
    }));
  } catch (e) {
    console.error('[ConsumerGroupsView] Error fetching consumer groups:', e);
    error.value = (e as { message: string }).message || 'Failed to load consumer groups';
  } finally {
    loading.value = false;
  }
}

function selectConsumerGroup(clusterName: string, group: ConsumerGroupItem) {
  window.dispatchEvent(new CustomEvent('select-consumer-group-in-tree', {
    detail: { groupName: group.name, clusterName }
  }));
}

function viewConsumerGroup(groupName: string) {
  window.dispatchEvent(new CustomEvent('view-consumer-group', {
    detail: { groupName, clusterName: clusterParam.value }
  }));
}

function refreshOffsets(groupName: string) {
  window.dispatchEvent(new CustomEvent('refresh-consumer-group-offsets', {
    detail: { groupName, clusterName: clusterParam.value }
  }));
}

async function refreshConsumerGroups() {
  if (!clusterParam.value) return;

  refreshing.value = true;
  try {
    // First, refresh data from Kafka to database
    const refreshResult = await apiClient.refreshConsumerGroups(clusterParam.value);
    console.log('[ConsumerGroupsView] Refresh result:', refreshResult);

    // Wait for sync to complete
    await new Promise(resolve => setTimeout(resolve, 500));

    // Then fetch the updated data from database
    await fetchConsumerGroups();

    if (refreshResult && refreshResult.success) {
      showSuccess(t.value.consumerGroups.refreshed);
    } else {
      showError('Refresh completed but no data was updated');
    }
  } catch (e) {
    console.error('[ConsumerGroupsView] Refresh error:', e);
    showError(`Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value = false;
  }
}

onMounted(() => {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight || 400;
  }
});
</script>

<style scoped>
.table :deep(tbody tr) {
  height: 40px;
}

.table :deep(td) {
  padding: 0.25rem 0.5rem;
  vertical-align: middle;
}

.table :deep(th) {
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  text-transform: none;
  letter-spacing: normal;
}
</style>
