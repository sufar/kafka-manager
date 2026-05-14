<template>
  <div class="flex flex-col h-screen overflow-hidden">
    <!-- Header -->
    <div class="p-3 pb-2 flex-shrink-0">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2 flex-wrap">
            <!-- Back button -->
            <button
              class="btn btn-ghost btn-sm"
              @click="goBack"
              title="Back to Messages"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
              </svg>
            </button>
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
                  <path stroke-linecap="round" stroke-linejoin="round" d="M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z" />
                </svg>
              </button>
            </span>
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <!-- Refresh offsets button -->
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

    <!-- Content Area (scrollable) -->
    <div class="flex-1 overflow-y-auto px-3 pb-3">
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
      <div class="overflow-y-auto">
        <table class="table w-full table-fixed">
          <colgroup>
            <col class="w-full">
            <col :style="{ width: columnWidths.partition + 'px' }">
            <col :style="{ width: columnWidths.startOffset + 'px' }">
            <col :style="{ width: columnWidths.endOffset + 'px' }">
            <col :style="{ width: columnWidths.committedOffset + 'px' }">
            <col :style="{ width: columnWidths.lag + 'px' }">
            <col :style="{ width: columnWidths.lastCommit + 'px' }">
          </colgroup>
          <thead>
            <tr class="bg-base-100 sticky top-0 z-10">
              <th class="p-2">{{ t.topicConsumerGroups.groupName }}</th>
              <th class="p-2">{{ t.consumerGroups.partition }}
                <div class="resizer" @mousedown="startColumnResize('partition', $event)"></div>
              </th>
              <th class="p-2 text-right">{{ t.consumerGroups.startOffset }}
                <div class="resizer" @mousedown="startColumnResize('startOffset', $event)"></div>
              </th>
              <th class="p-2 text-right">{{ t.consumerGroups.endOffset }}
                <div class="resizer" @mousedown="startColumnResize('endOffset', $event)"></div>
              </th>
              <th class="p-2 text-right">{{ t.consumerGroups.committedOffset }}
                <div class="resizer" @mousedown="startColumnResize('committedOffset', $event)"></div>
              </th>
              <th class="p-2 text-right">{{ t.consumerGroups.lag }}
                <div class="resizer" @mousedown="startColumnResize('lag', $event)"></div>
              </th>
              <th class="p-2 text-right">{{ t.consumerGroups.lastCommit }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in offsets" :key="`${item.group}-${item.partition}`" class="hover">
              <td class="p-2">
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-secondary flex-shrink-0">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                  </svg>
                  <span class="truncate">{{ item.group }}</span>
                </div>
              </td>
              <td class="p-2"><span class="badge badge-ghost badge-sm">{{ item.partition }}</span></td>
              <td class="p-2 text-right font-mono text-sm">{{ item.start_offset }}</td>
              <td class="p-2 text-right font-mono text-sm">{{ item.end_offset }}</td>
              <td class="p-2 text-right font-mono text-sm">{{ item.committed_offset }}</td>
              <td class="p-2 text-right">
                <span :class="getLagClass(item.lag)" class="font-mono text-sm">{{ item.lag }}</span>
              </td>
              <td class="p-2 text-right text-xs text-base-content/60 whitespace-nowrap">{{ formatLastCommitTime(item.last_commit_time) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const route = useRoute();
const router = useRouter();
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

// Column widths for the offsets table
type ColumnKey = 'partition' | 'startOffset' | 'endOffset' | 'committedOffset' | 'lag' | 'lastCommit';
const columnWidths = ref<Record<ColumnKey, number>>({
  partition: 60,
  startOffset: 80,
  endOffset: 80,
  committedOffset: 90,
  lag: 70,
  lastCommit: 120,
});
const columnResizing = ref(false);
const resizeColumn = ref<ColumnKey | null>(null);
const resizeStartX = ref(0);
const resizeStartWidth = ref(0);

function startColumnResize(col: ColumnKey, e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
  columnResizing.value = true;
  resizeColumn.value = col;
  resizeStartX.value = e.clientX;
  resizeStartWidth.value = columnWidths.value[col];
  document.addEventListener('mousemove', onColumnResize);
  document.addEventListener('mouseup', stopColumnResize);
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function onColumnResize(e: MouseEvent) {
  if (!resizeColumn.value) return;
  const delta = e.clientX - resizeStartX.value;
  const newWidth = Math.max(30, resizeStartWidth.value + delta);
  columnWidths.value[resizeColumn.value] = newWidth;
}

function stopColumnResize() {
  columnResizing.value = false;
  resizeColumn.value = null;
  document.removeEventListener('mousemove', onColumnResize);
  document.removeEventListener('mouseup', stopColumnResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

// Go back to messages page
function goBack() {
  router.push({
    path: '/messages',
    query: {
      cluster: clusterParam.value,
      topic: topicParam.value
    }
  });
}

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
  position: relative;
}

.resizer {
  position: absolute;
  top: 0;
  bottom: 0;
  right: 0;
  width: 4px;
  cursor: col-resize;
  z-index: 1;
}

.resizer:hover {
  background-color: oklch(var(--p) / 0.4);
}
</style>
