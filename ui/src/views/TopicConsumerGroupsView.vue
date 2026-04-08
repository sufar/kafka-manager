<template>
  <div class="p-3 overflow-y-auto min-h-full relative">
    <!-- Header -->
    <div class="mb-4">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2 flex-wrap">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
            </svg>
            {{ t.topicConsumerGroups?.title || 'Topic Consumer Groups' }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm flex flex-wrap items-center gap-1">
            <span>{{ t.clusters.clusters }}:</span>
            <span class="font-medium">{{ clusterParam }}</span>
            <span>•</span>
            <span>{{ t.topicConsumerGroups.topicNamePrefix }}</span>
            <span class="font-medium">{{ topicParam }}</span>
            <span class="tooltip tooltip-right" :data-tip="t.topicConsumerGroups?.dataNotice">
              <button class="btn btn-ghost btn-xs btn-circle ml-1">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.458.775-2.072 1.404-.297.305-.567.644-.796 1.013-.23.369-.427.774-.586 1.21-.254.696-.384 1.437-.384 2.202 0 .414.336.75.75.75s.75-.336.75-.75c0-.53.087-1.04.248-1.516.16-.476.395-.914.696-1.298.3-.384.659-.724 1.067-1.003.408-.279.858-.5 1.34-.646.396-.12.776-.29 1.13-.512.354-.222.68-.496.968-.814.923-1.022 1.486-2.376 1.486-3.868 0-2.95-2.4-5.35-5.35-5.35S5.5 5.05 5.5 8c0 1.492.563 2.846 1.486 3.868.288.318.614.592.968.814.354.222.734.392 1.13.512.482.146.932.367 1.34.646.408.279.767.619 1.067 1.003.3.384.536.822.696 1.298.16.476.248.986.248 1.516 0 .414.336.75.75.75s.75-.336.75-.75c0-.765-.13-1.506-.384-2.202-.159-.436-.356-.841-.586-1.21-.229-.369-.499-.708-.796-1.013-.614-.629-1.327-1.043-2.072-1.404-.24-.116-.467-.263-.67-.442-1.172-1.025-1.172-2.687 0-3.712Z" />
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 16a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM12 4a.75.75 0 0 0 0 1.5.75.75 0 0 0 0-1.5Z" />
                </svg>
              </button>
            </span>
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            class="btn btn-xs btn-outline"
            @click="refreshAll"
            :disabled="refreshing"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            <span class="hidden md:inline ml-1">{{ t.common.refresh }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading" class="flex justify-center py-8">
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

    <!-- Empty state -->
    <div v-else-if="offsets.length === 0" class="flex flex-col items-center justify-center py-12 text-center">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16 text-base-content/30 mb-4">
        <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
      </svg>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 text-sm">{{ t.topicConsumerGroups?.noData }}</p>
    </div>

    <!-- Offsets Table -->
    <div v-else class="card glass gradient-border shadow-xl">
      <!-- Table Header -->
      <div class="bg-base-100 border-b border-base-200 sticky top-0 z-10">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="p-2">{{ t.consumerGroups.groupName }}</th>
              <th class="p-2">{{ t.consumerGroups.partition }}</th>
              <th class="p-2 text-right">{{ t.consumerGroups.startOffset }}</th>
              <th class="p-2 text-right">{{ t.consumerGroups.endOffset }}</th>
              <th class="p-2 text-right">{{ t.consumerGroups.committedOffset }}</th>
              <th class="p-2 text-right">{{ t.consumerGroups.lag }}</th>
              <th class="p-2 text-right">{{ t.consumerGroups.lastCommit }}</th>
            </tr>
          </thead>
        </table>
      </div>
      <!-- Table Body -->
      <div class="overflow-auto" style="max-height: calc(100vh - 280px);">
        <table class="table w-full">
          <tbody>
            <tr v-for="item in offsets" :key="`${item.group}-${item.partition}`" class="hover">
              <td class="font-medium">
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-secondary flex-shrink-0">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                  </svg>
                  {{ item.group }}
                </div>
              </td>
              <td><span class="badge badge-ghost badge-sm">{{ item.partition }}</span></td>
              <td class="text-right font-mono text-sm">{{ item.start_offset }}</td>
              <td class="text-right font-mono text-sm">{{ item.end_offset }}</td>
              <td class="text-right font-mono text-sm">{{ item.committed_offset }}</td>
              <td class="text-right">
                <span :class="getLagClass(item.lag)" class="font-mono text-sm">{{ item.lag }}</span>
              </td>
              <td class="text-right text-xs text-base-content/60">{{ formatLastCommitTime(item.last_commit_time) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRoute } from 'vue-router';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const route = useRoute();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const currentLang = computed(() => languageStore.currentLanguage);
const { showError, showSuccess } = useToast();

// Get cluster and topic from URL params
const clusterParam = computed(() => {
  const cluster = route.query.cluster;
  return typeof cluster === 'string' ? cluster : '';
});

const topicParam = computed(() => {
  const topic = route.query.topic;
  return typeof topic === 'string' ? topic : '';
});

// State
const loading = ref(false);
const error = ref<string | null>(null);
const refreshing = ref(false);

interface OffsetRow {
  group: string;
  topic: string;
  partition: number;
  start_offset: number;
  end_offset: number;
  committed_offset: number;
  lag: number;
  last_commit_time: number | null;
}

const offsets = ref<OffsetRow[]>([]);

// Get lag class
function getLagClass(lag: number): string {
  if (lag === 0) return 'text-success';
  if (lag < 100) return 'text-warning';
  return 'text-error';
}

// Format last commit time
function formatLastCommitTime(timestamp: number | null): string {
  if (!timestamp) return '-';
  const date = new Date(timestamp);
  const locale = currentLang.value === 'zh' ? 'zh-CN' : 'en-US';
  return date.toLocaleString(locale, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

// Load consumer groups and their offsets for topic
async function loadOffsets() {
  if (!clusterParam.value || !topicParam.value) {
    error.value = t.value.common.error || 'Error';
    return;
  }

  loading.value = true;
  error.value = null;

  try {
    // Get list of consumer groups consuming this topic with offsets
    const offsetData = await apiClient.getConsumerGroupsByTopic(clusterParam.value, topicParam.value);

    // Map to OffsetRow format
    const allOffsets: OffsetRow[] = offsetData.map(o => ({
      group: o.group,
      topic: o.topic,
      partition: o.partition,
      start_offset: o.start_offset,
      end_offset: o.end_offset,
      committed_offset: o.committed_offset,
      lag: o.lag,
      last_commit_time: o.last_commit_time,
    }));

    offsets.value = allOffsets;
  } catch (e) {
    console.error('[TopicConsumerGroupsView] Error loading consumer groups:', e);
    error.value = t.value.common.error || 'Failed to load consumer groups';
  } finally {
    loading.value = false;
  }
}

// Refresh all offsets
async function refreshAll() {
  if (!clusterParam.value || !topicParam.value) return;

  refreshing.value = true;
  try {
    // Get fresh consumer groups list with offsets
    const offsetData = await apiClient.getConsumerGroupsByTopic(clusterParam.value, topicParam.value);

    // Map to OffsetRow format
    const allOffsets: OffsetRow[] = offsetData.map(o => ({
      group: o.group,
      topic: o.topic,
      partition: o.partition,
      start_offset: o.start_offset,
      end_offset: o.end_offset,
      committed_offset: o.committed_offset,
      lag: o.lag,
      last_commit_time: o.last_commit_time,
    }));

    offsets.value = allOffsets;
    showSuccess(t.value.topicConsumerGroups?.refreshed || 'Consumer groups refreshed');
  } catch (e) {
    console.error('[TopicConsumerGroupsView] Error refreshing consumer groups:', e);
    showError(`${t.value.common.refresh} ${t.value.common.failed}: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value = false;
  }
}

onMounted(() => {
  loadOffsets();
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
