<template>
  <div class="sent-message-history">
    <!-- 头部 -->
    <div class="flex items-center justify-between mb-3">
      <h3 class="font-bold text-sm flex items-center gap-1.5">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-base-content/60">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
        </svg>
        {{ t.sentMessageHistory?.title || '发送历史' }}
      </h3>
      <div class="flex items-center gap-1">
        <button
          v-if="messages.length > 0"
          class="btn btn-ghost btn-xs text-error"
          @click="clearHistory"
          :title="t.sentMessageHistory?.clearAll || '清空历史'"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
          </svg>
        </button>
        <button
          class="btn btn-ghost btn-xs"
          @click="loadHistory"
          :title="t.common?.refresh || '刷新'"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
          </svg>
        </button>
        <button
          class="btn btn-ghost btn-xs"
          @click="emit('close')"
          :title="t.common?.close || '关闭'"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- 搜索框 -->
    <div class="relative mb-2">
      <input
        v-model="searchQuery"
        type="text"
        class="input input-bordered input-sm w-full pl-8"
        :placeholder="t.sentMessageHistory?.searchPlaceholder || '搜索 Topic...'"
      />
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 absolute left-2.5 top-1/2 -translate-y-1/2 text-base-content/40">
        <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
      </svg>
      <button
        v-if="searchQuery"
        class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content"
        @click="searchQuery = ''"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- 内容 -->
    <div class="history-list">
      <div v-if="loading" class="flex items-center justify-center py-8">
        <span class="loading loading-spinner loading-md text-primary"></span>
      </div>

      <div v-else-if="filteredMessages.length === 0" class="text-center py-8 text-base-content/50">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-10 h-10 mx-auto mb-2 opacity-50">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
        </svg>
        <p class="text-sm">{{ searchQuery ? (t.sentMessageHistory?.noSearchResults || '无匹配的历史记录') : (t.sentMessageHistory?.empty || '暂无发送历史') }}</p>
        <p v-if="!searchQuery" class="text-xs mt-1">{{ t.sentMessageHistory?.emptyHint || '发送消息时会自动记录到这里' }}</p>
      </div>

      <div v-else>
        <div
          v-for="item in filteredMessages"
          :key="item.id"
          class="history-item"
          @dblclick="selectMessage(item)"
        >
          <div class="flex items-center gap-2 flex-1 min-w-0">
            <div class="w-5 h-5 rounded bg-primary/10 flex items-center justify-center flex-shrink-0">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
              </svg>
            </div>
            <span class="font-medium text-xs truncate flex-shrink-0 min-w-0" :title="item.topic_name">{{ item.topic_name }}</span>
            <span class="text-[10px] text-base-content/40 flex-shrink-0 hidden sm:inline">·</span>
            <span class="badge badge-ghost badge-[10px] text-[9px] px-1 flex-shrink-0 truncate max-w-[60px]" :title="item.cluster_id">{{ item.cluster_id }}</span>
            <span class="badge badge-primary badge-[10px] text-[9px] px-1 flex-shrink-0">P{{ item.partition }}</span>
            <span v-if="item.message_key" class="text-[10px] text-base-content/50 flex-shrink-0">
              <span class="text-base-content/30">K:</span> <span class="truncate max-w-[80px] inline-block align-bottom" :title="item.message_key">{{ item.message_key }}</span>
            </span>
            <span class="text-[10px] text-base-content/50 flex-1 min-w-0 truncate" :title="item.message_value">
              <span class="text-base-content/30">V:</span> {{ item.message_value }}
            </span>
          </div>
          <div class="flex items-center gap-1 flex-shrink-0">
            <span class="text-[10px] text-base-content/40 whitespace-nowrap" :title="formatFullTime(item.sent_at)">{{ formatTime(item.sent_at) }}</span>
            <button
              class="btn btn-ghost btn-xs px-0.5 h-auto text-error"
              @click.stop="deleteHistory(item.id)"
              :title="t.sentMessageHistory?.delete || '删除记录'"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const { showSuccess, showError, confirm } = useToast();

// Props
const props = defineProps<{
  t: Record<string, any>;
  cluster?: string;
  topic?: string;
}>();

// Emits
const emit = defineEmits<{
  select: [message: { cluster_id: string; topic_name: string; partition: number; message_key: string | null; message_value: string; headers?: any }];
  close: [];
}>();

// State
const loading = ref(false);
const messages = ref<Array<{
  id: number;
  cluster_id: string;
  topic_name: string;
  partition: number;
  message_key: string | null;
  message_value: string;
  headers?: any;
  offset?: number;
  sent_at: string;
}>>([]);
const searchQuery = ref('');

// Filter messages by search query (only search within current topic if cluster/topic props are set)
const filteredMessages = computed(() => {
  if (!searchQuery.value || searchQuery.value.trim() === '') {
    return messages.value;
  }

  const query = searchQuery.value.toLowerCase();
  return messages.value.filter((item) => {
    return item.topic_name?.toLowerCase().includes(query);
  });
});

// Load history
async function loadHistory() {
  loading.value = true;
  try {
    const result = await apiClient.getSentMessageHistory(100, 0, props.cluster, props.topic);
    messages.value = result.messages || [];
  } catch (error: any) {
    showError(error.message || '加载历史失败');
  } finally {
    loading.value = false;
  }
}

// Delete history
async function deleteHistory(id: number) {
  try {
    await apiClient.deleteSentMessageHistory(id);
    messages.value = messages.value.filter((m) => m.id !== id);
    showSuccess('历史记录已删除');
  } catch (error: any) {
    showError(error.message || '删除失败');
  }
}

// Clear all history
async function clearHistory() {
  if (!await confirm(props.t.sentMessageHistory?.confirmClear || '确定要清空所有发送历史吗？')) {
    return;
  }
  try {
    await apiClient.clearSentMessageHistory();
    messages.value = [];
    showSuccess('发送历史已清空');
  } catch (error: any) {
    showError(error.message || '清空失败');
  }
}

// Select message to fill form
function selectMessage(item: typeof messages.value[number]) {
  emit('select', {
    cluster_id: item.cluster_id,
    topic_name: item.topic_name,
    partition: item.partition,
    message_key: item.message_key,
    message_value: item.message_value,
    headers: item.headers,
  });
}

// Format full time for tooltip
function formatFullTime(sentAt: string): string {
  try {
    const date = new Date(sentAt);
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  } catch {
    return sentAt;
  }
}

// Format time
function formatTime(sentAt: string): string {
  try {
    const date = new Date(sentAt);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) {
      return props.t.sentMessageHistory?.justNow || '刚刚';
    } else if (minutes < 60) {
      return `${minutes}${props.t.sentMessageHistory?.minutesAgo || '分钟前'}`;
    } else if (hours < 24) {
      return `${hours}${props.t.sentMessageHistory?.hoursAgo || '小时前'}`;
    } else if (days < 7) {
      return `${days}${props.t.sentMessageHistory?.daysAgo || '天前'}`;
    } else {
      return date.toLocaleDateString();
    }
  } catch {
    return sentAt;
  }
}

onMounted(() => {
  loadHistory();
});

watch(() => [props.cluster, props.topic], () => {
  if (props.cluster && props.topic) {
    loadHistory();
  }
});
</script>

<style scoped>
.sent-message-history {
  padding: 0.5rem;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.history-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.375rem;
  padding: 0.375rem 0.5rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.history-item:hover {
  background: rgba(99, 102, 241, 0.1);
}

.history-item:active {
  transform: scale(0.98);
}
</style>
