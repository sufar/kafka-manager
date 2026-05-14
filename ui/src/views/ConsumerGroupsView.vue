<template>
  <div class="p-3 overflow-y-auto min-h-screen relative">
    <!-- Detail View -->
    <div v-if="detailView" class="space-y-4">
      <!-- Header with group name and actions -->
      <div class="card glass gradient-border shadow-xl">
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 p-4">
          <div class="flex-1 min-w-0">
            <h2 class="text-xl font-bold flex items-center gap-2 flex-wrap">
              <button class="btn btn-ghost btn-xs p-1 mr-1" @click="router.back()" :title="t.common?.back || 'Back'">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
                </svg>
              </button>
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 flex-shrink-0">
                <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
              </svg>
              <span class="truncate">{{ t.consumerGroups.groupNamePrefix }}{{ currentGroup }}</span>
            </h2>
            <p class="text-base-content/60 text-sm mt-1 flex flex-wrap items-center gap-2">
              {{ t.clusters.clusters }}: <span class="font-medium">{{ clusterParam }}</span>
              <span v-if="groupState" class="badge" :class="getStateBadgeClass(groupState)">{{ groupState }}</span>
            </p>
          </div>
          <div class="flex gap-2 flex-shrink-0 w-full sm:w-auto min-w-0">
            <button
              class="btn btn-sm btn-outline flex-1 sm:flex-none flex-shrink-0"
              @click="refreshOffsets"
              :disabled="refreshing"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4" :class="{ 'animate-spin': refreshing }">
                <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
              </svg>
              {{ t.common.refresh }}
            </button>
            <div class="relative" ref="actionsMenuRef">
              <button @click="toggleActionsMenu" class="btn btn-sm btn-primary whitespace-nowrap flex-shrink-0">
                {{ t.common.actions }}
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 ml-1 transition-transform" :class="{ 'rotate-180': actionsMenuOpen }">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Actions Menu - Teleported to body -->
      <Teleport to="body">
        <ul v-if="actionsMenuOpen" ref="actionsMenuDropdown" :style="[menuStyle, { zIndex: 9999 }]" class="menu p-2 shadow-2xl bg-base-100 rounded-box w-52">
          <li><a @click.stop="handleResetOffset">{{ t.consumerGroups.resetOffset }}</a></li>
          <li><a @click.stop="handleDeleteGroup" class="text-error">{{ t.consumerGroups.deleteGroup }}</a></li>
        </ul>
      </Teleport>

      <!-- Loading state -->
      <div v-if="loadingDetail" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-md text-primary"></span>
        <p class="ml-4 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
      </div>

      <!-- Error state -->
      <div v-else-if="detailError" class="alert alert-error">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
        </svg>
        <span class="text-sm">{{ detailError }}</span>
      </div>

      <!-- Offsets Table -->
      <div v-else-if="offsets.length > 0" class="card glass gradient-border shadow-xl">
        <div class="p-3 bg-base-100 border-b border-base-200 flex items-center justify-between">
          <h3 class="font-semibold">{{ t.consumerGroups.offsets }}</h3>
          <span class="text-xs text-base-content/60">{{ offsets.length }} {{ t.consumerGroups.partitions }}</span>
        </div>
        <div class="overflow-y-auto max-h-[600px]">
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
              <tr>
                <th class="p-2">{{ t.consumerGroups.topic }}</th>
                <th class="p-2">{{ t.consumerGroups.partition }}
                  <div class="resizer"
                    @mousedown="startColumnResize('partition', $event)"></div>
                </th>
                <th class="p-2 text-right">{{ t.consumerGroups.startOffset }}
                  <div class="resizer"
                    @mousedown="startColumnResize('startOffset', $event)"></div>
                </th>
                <th class="p-2 text-right">{{ t.consumerGroups.endOffset }}
                  <div class="resizer"
                    @mousedown="startColumnResize('endOffset', $event)"></div>
                </th>
                <th class="p-2 text-right">{{ t.consumerGroups.committedOffset }}
                  <div class="resizer"
                    @mousedown="startColumnResize('committedOffset', $event)"></div>
                </th>
                <th class="p-2 text-right">{{ t.consumerGroups.lag }}
                  <div class="resizer"
                    @mousedown="startColumnResize('lag', $event)"></div>
                </th>
                <th class="p-2 text-right">{{ t.consumerGroups.lastCommit }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in offsets" :key="`${item.topic}-${item.partition}`" class="hover">
                <td class="font-medium truncate" :title="item.topic">{{ item.topic }}</td>
                <td><span class="badge badge-ghost badge-sm">{{ item.partition }}</span></td>
                <td class="text-right font-mono text-sm">{{ item.start_offset }}</td>
                <td class="text-right font-mono text-sm">{{ item.end_offset }}</td>
                <td class="text-right font-mono text-sm">{{ item.committed_offset }}</td>
                <td class="text-right">
                  <span :class="getLagClass(item.lag)" class="font-mono text-sm">{{ item.lag }}</span>
                </td>
                <td class="p-2 text-right text-xs text-base-content/60 whitespace-nowrap">{{ formatLastCommitTime(item.last_commit_time) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- No data state -->
      <div v-else class="card glass gradient-border shadow-xl">
        <div class="flex flex-col items-center justify-center py-12 text-center">
          <div class="text-base-content/30 mb-4">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
            </svg>
          </div>
          <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
          <p class="text-base-content/60 mb-4 text-sm">{{ t.consumerGroups.noOffsets }}</p>
          <button class="btn btn-sm btn-outline" @click="refreshOffsets">{{ t.common.refresh }}</button>
        </div>
      </div>

      <!-- Reset Offset Dialog -->
      <dialog ref="resetOffsetDialog" class="modal modal-bottom sm:modal-middle">
        <div class="modal-box">
          <h3 class="font-bold text-lg mb-4">{{ t.consumerGroups.resetOffset }}</h3>
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.selectTopic }}</span>
            </label>
            <select v-model="resetOffsetForm.topic" class="select select-bordered w-full">
              <option v-for="topic in uniqueTopics" :key="topic" :value="topic">{{ topic }}</option>
            </select>
          </div>
          <div class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.partition }}</span>
            </label>
            <select v-model="resetOffsetForm.partition" class="select select-bordered w-full">
              <option v-for="p in getPartitionsForTopic(resetOffsetForm.topic)" :key="p" :value="p">{{ p }}</option>
            </select>
          </div>
          <div class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.resetTo }}</span>
            </label>
            <select v-model="resetOffsetForm.resetTo" class="select select-bordered w-full">
              <option value="earliest">{{ t.consumerGroups.earliest }}</option>
              <option value="latest">{{ t.consumerGroups.latest }}</option>
              <option value="offset">{{ t.consumerGroups.specificOffset }}</option>
              <option value="timestamp">{{ t.consumerGroups.timestamp }}</option>
            </select>
          </div>
          <div v-if="resetOffsetForm.resetTo === 'offset'" class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.offsetValue }}</span>
            </label>
            <input v-model.number="resetOffsetForm.offsetValue" type="number" class="input input-bordered w-full" />
          </div>
          <div v-if="resetOffsetForm.resetTo === 'timestamp'" class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.timestampValue }}</span>
            </label>
            <input v-model="resetOffsetForm.timestampValue" type="datetime-local" class="input input-bordered w-full" />
          </div>
          <div class="modal-action">
            <button class="btn btn-ghost" @click="closeResetOffsetDialog">{{ t.common.cancel }}</button>
            <button class="btn btn-primary" @click="confirmResetOffset" :disabled="resettingOffset">{{ t.common.confirm }}</button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeResetOffsetDialog">
          <button class="hidden">close</button>
        </form>
      </dialog>
    </div>

    <!-- No group selected -->
    <div v-else class="flex flex-col items-center justify-center py-12 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-4 text-sm">{{ t.consumerGroups.selectFromNav }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

interface OffsetItem {
  topic: string;
  partition: number;
  start_offset: number;
  end_offset: number;
  committed_offset: number;
  lag: number;
  last_commit_time?: number | null;
}

const route = useRoute();
const router = useRouter();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess, confirm } = useToast();

const clusterParam = computed(() => {
  const val = route.query.cluster;
  return Array.isArray(val) ? val[0] : (val || '');
});

const groupParam = computed(() => {
  const val = route.query.group;
  return Array.isArray(val) ? val[0] : (val || '');
});

// Detail view state
const detailView = computed(() => !!groupParam.value && !!clusterParam.value);
const currentGroup = ref('');
const groupState = ref('');
const loadingDetail = ref(false);
const detailError = ref<string | null>(null);
const offsets = ref<OffsetItem[]>([]);
const refreshing = ref(false);

// Column widths for the offsets table
type ColumnKey = 'partition' | 'startOffset' | 'endOffset' | 'committedOffset' | 'lag' | 'lastCommit';
const columnWidths = ref<Record<ColumnKey, number>>({
  partition: 60,
  startOffset: 80,
  endOffset: 80,
  committedOffset: 90,
  lag: 70,
  lastCommit: 150,
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

// Reset offset dialog
const resetOffsetDialog = ref<HTMLDialogElement | null>(null);
const resettingOffset = ref(false);
const resetOffsetForm = ref<{
  topic: string;
  partition: number;
  resetTo: 'earliest' | 'latest' | 'offset' | 'timestamp';
  offsetValue: number;
  timestampValue: string;
}>({
  topic: '',
  partition: -1,
  resetTo: 'earliest',
  offsetValue: 0,
  timestampValue: '',
});

// Actions menu
const actionsMenuOpen = ref(false);
const actionsMenuRef = ref<HTMLElement | null>(null);
const actionsMenuDropdown = ref<HTMLElement | null>(null);
const menuStyle = ref({});

// Toggle menu and calculate position
function toggleActionsMenu() {
  actionsMenuOpen.value = !actionsMenuOpen.value;
  if (actionsMenuOpen.value && actionsMenuRef.value) {
    updateMenuPosition();
  }
}

// Update menu position
function updateMenuPosition() {
  if (actionsMenuRef.value) {
    const rect = actionsMenuRef.value.getBoundingClientRect();
    menuStyle.value = {
      position: 'fixed',
      top: `${rect.bottom + 8}px`,
      right: `${window.innerWidth - rect.right}px`,
    };
  }
}

// Close menu when clicking outside
function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (actionsMenuOpen.value && actionsMenuRef.value && !actionsMenuRef.value.contains(target)) {
    actionsMenuOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
  window.addEventListener('scroll', updateMenuPosition, true);
  window.addEventListener('resize', updateMenuPosition);
  if (detailView.value) {
    loadConsumerGroupDetail();
  }
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  window.removeEventListener('scroll', updateMenuPosition, true);
  window.removeEventListener('resize', updateMenuPosition);
});

// Unique topics from offsets
const uniqueTopics = computed(() => {
  const topics = new Set<string>();
  offsets.value.forEach(o => topics.add(o.topic));
  return Array.from(topics).sort();
});

// Watch for group parameter to show detail view
watch([() => groupParam.value, () => clusterParam.value], ([newGroup, newCluster], [oldGroup, oldCluster]) => {
  if (newGroup && newCluster) {
    currentGroup.value = newGroup;
    loadConsumerGroupDetail();
  }
}, { immediate: true });

async function loadConsumerGroupDetail() {
  if (!clusterParam.value || !currentGroup.value) return;

  loadingDetail.value = true;
  detailError.value = null;

  try {
    // Get group info
    const groupInfo = await apiClient.getConsumerGroupInfo(clusterParam.value, currentGroup.value);
    groupState.value = groupInfo.state || 'Unknown';

    // Get offsets
    await loadOffsets();
  } catch (e) {
    console.error('[ConsumerGroupsView] Error loading group detail:', e);
    detailError.value = (e as { message: string }).message || 'Failed to load consumer group details';
  } finally {
    loadingDetail.value = false;
  }
}

async function loadOffsets() {
  if (!clusterParam.value || !currentGroup.value) return;

  try {
    const offsetsData = await apiClient.getConsumerGroupOffsets(clusterParam.value, currentGroup.value);
    offsets.value = offsetsData;
  } catch (e) {
    console.error('[ConsumerGroupsView] Error loading offsets:', e);
    throw e;
  }
}

function goBack() {
  router.push({ path: '/topics', query: { cluster: clusterParam.value } });
}

async function refreshOffsets() {
  if (!clusterParam.value || !currentGroup.value) return;

  refreshing.value = true;
  try {
    await loadOffsets();
    showSuccess(t.value.consumerGroups.offsetsRefreshed);
  } catch (e) {
    console.error('[ConsumerGroupsView] Refresh offsets error:', e);
    showError(`Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value = false;
  }
}

function openResetOffsetDialog() {
  resetOffsetForm.value = {
    topic: uniqueTopics.value[0] || '',
    partition: -1,
    resetTo: 'earliest',
    offsetValue: 0,
    timestampValue: '',
  };
  resetOffsetDialog.value?.showModal();
}

function handleResetOffset() {
  actionsMenuOpen.value = false;
  // 使用 groupParam 而不是 currentGroup，确保获取最新的值
  if (!groupParam.value) {
    showError('Consumer group name is missing');
    return;
  }
  currentGroup.value = groupParam.value;
  openResetOffsetDialog();
}

function closeResetOffsetDialog() {
  resetOffsetDialog.value?.close();
}

function getPartitionsForTopic(topic: string): number[] {
  const topicOffsets = offsets.value.filter(o => o.topic === topic);
  const partitions = topicOffsets.map(o => o.partition);
  // Remove duplicates and sort
  return [...new Set(partitions)].sort((a, b) => a - b);
}

async function confirmResetOffset() {
  if (!clusterParam.value || !currentGroup.value) {
    console.error('[confirmResetOffset] Missing clusterParam or currentGroup');
    return;
  }

  const { topic, partition, resetTo, offsetValue, timestampValue } = resetOffsetForm.value;
  console.log('[confirmResetOffset] Form data:', { topic, partition, resetTo, offsetValue, timestampValue });
  console.log('[confirmResetOffset] Cluster:', clusterParam.value, 'Group:', currentGroup.value);

  if (partition < 0) {
    showError('Please select a partition');
    return;
  }

  if (resetTo === 'offset' && offsetValue < 0) {
    showError('Please enter a valid offset value');
    return;
  }

  if (resetTo === 'timestamp' && !timestampValue) {
    showError('Please enter a timestamp');
    return;
  }

  resettingOffset.value = true;
  try {
    let resetToParam: 'earliest' | 'latest' | 'offset' | 'timestamp' = resetTo;
    let timestampParam: number | undefined = undefined;
    let offsetParam: number | undefined = undefined;

    if (resetTo === 'offset') {
      offsetParam = offsetValue;
    } else if (resetTo === 'timestamp') {
      timestampParam = new Date(timestampValue).getTime();
    }

    console.log('[confirmResetOffset] Calling API with params:', {
      clusterId: clusterParam.value,
      groupName: currentGroup.value,
      topic,
      partition,
      resetTo: resetToParam,
      timestamp: timestampParam,
      offset: offsetParam
    });

    await apiClient.resetConsumerGroupOffset(
      clusterParam.value,
      currentGroup.value,
      topic,
      partition,
      resetToParam,
      timestampParam,
      offsetParam
    );

    console.log('[confirmResetOffset] API call succeeded');
    showSuccess(t.value.consumerGroups.offsetResetSuccess);
    closeResetOffsetDialog();
    await loadOffsets();
  } catch (e) {
    console.error('[ConsumerGroupsView] Reset offset error:', e);
    const errorMsg = (e as { message: string }).message;
    let userMessage = `Reset failed: ${errorMsg}`;

    // 处理特定的错误消息
    if (errorMsg.includes('UnknownMemberId') || errorMsg.includes('Unknown member')) {
      userMessage = '重置失败：当前消费者组没有活跃成员。请确保有消费者连接到该组后再尝试重置偏移量。';
    } else if (errorMsg.includes('group_name')) {
      userMessage = '重置失败：消费者组名称无效或不存在。';
    }

    showError(userMessage);
  } finally {
    resettingOffset.value = false;
  }
}

async function deleteConsumerGroup() {
  if (!clusterParam.value || !currentGroup.value) return;

  const confirmed = await confirm(`Are you sure you want to delete consumer group "${currentGroup.value}"?`);
  if (!confirmed) {
    return;
  }

  try {
    await apiClient.deleteConsumerGroup(clusterParam.value, currentGroup.value);
    showSuccess(t.value.consumerGroups.deleted);
    goBack();
  } catch (e) {
    console.error('[ConsumerGroupsView] Delete error:', e);
    showError(`Delete failed: ${(e as { message: string }).message}`);
  }
}

function handleDeleteGroup() {
  actionsMenuOpen.value = false;
  setTimeout(() => {
    deleteConsumerGroup();
  }, 100);
}

// Utility functions
function getStateBadgeClass(state: string): string {
  const lowerState = state.toLowerCase();
  if (lowerState === 'stable' || lowerState === 'empty') return 'badge-success';
  if (lowerState === 'preparing_rebalance' || lowerState === 'completing_rebalance') return 'badge-warning';
  if (lowerState === 'dead' || lowerState === 'unknown') return 'badge-error';
  return 'badge-ghost';
}

function getLagClass(lag: number): string {
  if (lag === 0) return 'text-success';
  if (lag < 1000) return 'text-warning';
  return 'text-error';
}

function formatLastCommitTime(timestamp?: number | null): string {
  if (!timestamp) return '-';
  try {
    const date = new Date(timestamp);
    return date.toLocaleString();
  } catch {
    return '-';
  }
}

onMounted(() => {
  if (detailView.value) {
    loadConsumerGroupDetail();
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
