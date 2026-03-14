<template>
  <div class="message-query-tool h-full flex flex-col">
    <!-- 简洁搜索栏 -->
    <div class="toolbar flex flex-wrap items-center gap-2 p-2 border-b border-base-300 bg-base-100">
      <!-- 分区选择 -->
      <select v-model="selectedPartition" class="select select-bordered select-sm w-28">
        <option value="all">全部分区</option>
        <option v-for="p in partitions" :key="p" :value="p">分区 {{ p }}</option>
      </select>

      <!-- 查询模式 -->
      <select v-model="fetchMode" class="select select-bordered select-sm w-24">
        <option value="newest">最新</option>
        <option value="oldest">最早</option>
      </select>

      <!-- 数量 -->
      <input v-model.number="maxMessages" type="number" class="input input-bordered input-sm w-16" min="1" max="1000" title="消息数量" />

      <!-- 搜索 -->
      <div class="flex-1 min-w-[120px] relative">
        <input v-model="searchKeyword" type="text" class="input input-bordered input-sm w-full pr-8" placeholder="搜索消息内容..." @keyup.enter="queryMessages" />
        <button v-if="searchKeyword" class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content" @click="searchKeyword = ''; queryMessages()">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- 查询按钮 -->
      <button class="btn btn-primary btn-sm" :class="{ 'loading': loading }" :disabled="!canQuery || loading" @click="queryMessages">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
        </svg>
      </button>

      <!-- 停止按钮 -->
      <button v-if="loading" class="btn btn-error btn-sm" @click="stopQuery">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      <!-- 导出 -->
      <button class="btn btn-ghost btn-sm" :disabled="messages.length === 0" @click="exportMessages" title="导出消息">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
        </svg>
      </button>

      <!-- 自动刷新 -->
      <label class="flex items-center gap-1 cursor-pointer ml-2">
        <input v-model="autoRefresh" type="checkbox" class="checkbox checkbox-xs" />
        <span class="text-xs text-base-content/70">自动刷新</span>
      </label>
      <select v-if="autoRefresh" v-model="refreshInterval" class="select select-bordered select-xs w-16">
        <option :value="5000">5s</option>
        <option :value="10000">10s</option>
        <option :value="30000">30s</option>
        <option :value="60000">1m</option>
      </select>
    </div>

    <!-- 状态栏 -->
    <div class="status-bar flex items-center justify-between px-3 py-1 text-xs border-b border-base-300 bg-base-200/50">
      <div class="flex items-center gap-4">
        <span v-if="selectedTopic" class="text-base-content/70">
          Topic: <span class="font-mono font-bold text-primary">{{ selectedTopic }}</span>
        </span>
        <span v-if="lastQueryTime > 0" class="text-base-content/70">
          耗时: <span class="font-mono font-bold">{{ lastQueryTime }}ms</span>
        </span>
        <span v-if="messages.length > 0" class="text-base-content/70">
          共 <span class="font-mono font-bold text-success">{{ messages.length }}</span> 条
        </span>
        <span v-if="error" class="text-error">{{ error }}</span>
      </div>
    </div>

    <!-- 消息列表 -->
    <div class="flex-1 overflow-auto bg-base-100">
      <!-- Desktop Table -->
      <table class="table table-sm w-full min-w-[600px] hidden md:table">
        <thead class="sticky top-0 bg-base-200 z-10">
          <tr>
            <th class="w-14 text-xs">分区</th>
            <th class="w-20 text-xs">Offset</th>
            <th class="w-32 text-xs">时间戳</th>
            <th class="w-24 text-xs">Key</th>
            <th class="text-xs">Value</th>
            <th class="w-14 text-xs">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="msg in messages" :key="`${msg.partition}-${msg.offset}`" class="hover:bg-base-200/50 transition-colors">
            <td class="text-xs">
              <span class="badge badge-ghost badge-sm">{{ msg.partition }}</span>
            </td>
            <td class="text-xs font-mono">{{ msg.offset }}</td>
            <td class="text-xs text-base-content/70 whitespace-nowrap">{{ formatTime(msg.timestamp) }}</td>
            <td class="text-xs font-mono truncate max-w-[80px]">{{ msg.key || '-' }}</td>
            <td class="text-xs font-mono truncate">{{ truncate(msg.value, 80) }}</td>
            <td class="text-xs">
              <button class="btn btn-ghost btn-xs" @click="copyMessage(msg)">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </td>
          </tr>
          <tr v-if="messages.length === 0 && !loading">
            <td colspan="6" class="text-center py-8 text-base-content/50">
              <div class="flex flex-col items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
                </svg>
                <span>暂无消息</span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>

      <!-- Mobile Card View -->
      <div class="md:hidden space-y-2 p-2">
        <div
          v-for="msg in messages"
          :key="`${msg.partition}-${msg.offset}`"
          class="card bg-base-100 border border-base-200 p-3 shadow-sm"
          @click="selectedMessage = msg"
        >
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2">
              <span class="badge badge-ghost badge-sm">P{{ msg.partition }}</span>
              <span class="text-xs font-mono text-base-content/70">#{{ msg.offset }}</span>
            </div>
            <span class="text-xs text-base-content/50">{{ formatTime(msg.timestamp) }}</span>
          </div>
          <div v-if="msg.key" class="text-xs font-mono text-secondary mb-1 truncate">
            Key: {{ msg.key }}
          </div>
          <div class="text-sm font-mono truncate text-base-content/80">
            {{ truncate(msg.value, 100) }}
          </div>
        </div>
        <div v-if="messages.length === 0 && !loading" class="text-center py-8 text-base-content/50">
          <div class="flex flex-col items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
            </svg>
            <span>暂无消息</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 详情面板（可选） -->
    <div v-if="selectedMessage" class="detail-panel border-t border-base-300 bg-base-200/30 p-3 h-48 overflow-auto">
      <div class="flex items-center justify-between mb-2">
        <h4 class="text-sm font-bold">消息详情</h4>
        <button class="btn btn-ghost btn-xs" @click="selectedMessage = null">关闭</button>
      </div>
      <div class="space-y-2 text-xs">
        <div><span class="text-base-content/50">Partition:</span> {{ selectedMessage.partition }}</div>
        <div><span class="text-base-content/50">Offset:</span> {{ selectedMessage.offset }}</div>
        <div><span class="text-base-content/50">Timestamp:</span> {{ formatTime(selectedMessage.timestamp) }}</div>
        <div v-if="selectedMessage.key"><span class="text-base-content/50">Key:</span> <pre class="bg-base-100 p-1 rounded mt-1">{{ selectedMessage.key }}</pre></div>
        <div><span class="text-base-content/50">Value:</span> <pre class="bg-base-100 p-1 rounded mt-1 overflow-auto">{{ selectedMessage.value }}</pre></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import { apiClient } from '@/api/client';
import { useClusterStore } from '@/stores/cluster';

const route = useRoute();

interface Message {
  partition: number;
  offset: number;
  key: string | null;
  value: string | null;
  timestamp: number | null;
}

// Store
const clusterStore = useClusterStore();

// Props - 从父组件接收 cluster 和 topic
const props = defineProps<{
  cluster?: string;
  topic?: string;
}>();

// 状态
const partitions = ref<number[]>([]);
const messages = ref<Message[]>([]);
const selectedMessage = ref<Message | null>(null);

// 查询参数
const selectedCluster = ref('');
const selectedTopic = ref('');
const selectedPartition = ref<string | number>('all');
const fetchMode = ref<'newest' | 'oldest'>('newest');
const maxMessages = ref(100);
const searchKeyword = ref('');

// UI 状态
const loading = ref(false);
const error = ref('');
const lastQueryTime = ref(0);

// 自动刷新
const autoRefresh = ref(false);
const refreshInterval = ref(10000);
let refreshTimer: number | null = null;

// 计算属性
const canQuery = computed(() => {
  return !!selectedCluster.value && !!selectedTopic.value && !loading.value;
});

// 监听自动刷新
watch(autoRefresh, (enabled) => {
  if (enabled) {
    startAutoRefresh();
  } else {
    stopAutoRefresh();
  }
});

watch(refreshInterval, () => {
  if (autoRefresh.value) {
    stopAutoRefresh();
    startAutoRefresh();
  }
});

// 方法
async function loadPartitions() {
  if (!selectedCluster.value || !selectedTopic.value) return;

  try {
    const detail = await apiClient.getTopicDetail(selectedCluster.value, selectedTopic.value);
    partitions.value = detail.partitions?.map((p: { id: number }) => p.id) || [];
  } catch (e) {
    console.error('Failed to fetch partitions:', e);
  }
}

async function queryMessages() {
  if (!canQuery.value) return;

  loading.value = true;
  error.value = '';
  const startTime = performance.now();

  try {
    const params: any = {
      max_messages: maxMessages.value,
      fetchMode: fetchMode.value,
      sort: fetchMode.value === 'newest' ? 'desc' : 'asc',
    };

    if (selectedPartition.value !== 'all') {
      params.partition = selectedPartition.value;
    }

    if (searchKeyword.value.trim()) {
      params.search = searchKeyword.value.trim();
      params.search_in = 'value';
    }

    const result = await apiClient.getMessages(
      selectedCluster.value,
      selectedTopic.value,
      params
    );

    messages.value = result.map((msg: any) => ({
      partition: msg.partition,
      offset: msg.offset,
      key: msg.key,
      value: msg.value,
      timestamp: msg.timestamp,
    }));

    lastQueryTime.value = Math.round(performance.now() - startTime);
  } catch (e: any) {
    console.error('Query failed:', e);
    error.value = e.message || '查询失败';
    messages.value = [];
  } finally {
    loading.value = false;
  }
}

function stopQuery() {
  apiClient.cancelGetMessages();
  loading.value = false;
}

function startAutoRefresh() {
  if (refreshTimer) return;
  refreshTimer = window.setInterval(() => {
    if (!loading.value) {
      queryMessages();
    }
  }, refreshInterval.value);
}

function stopAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }
}

function exportMessages() {
  if (messages.value.length === 0) return;

  const data = JSON.stringify(messages.value, null, 2);
  const blob = new Blob([data], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${selectedTopic.value}_messages_${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function copyMessage(msg: Message) {
  const text = JSON.stringify(msg, null, 2);
  navigator.clipboard.writeText(text).then(() => {
    // 可以在这里显示一个 toast
    alert('已复制到剪贴板');
  });
}

function formatTime(ts: number | null): string {
  if (!ts) return '-';
  const date = new Date(ts);
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function truncate(str: string | null, len: number): string {
  if (!str) return '';
  return str.length > len ? str.slice(0, len) + '...' : str;
}

onMounted(async () => {
  // 优先使用 props 传入的 cluster 和 topic
  if (props.cluster) {
    selectedCluster.value = props.cluster;
  }
  if (props.topic) {
    selectedTopic.value = props.topic;
  }

  // 如果 props 没有，从 URL 参数获取
  if (!selectedCluster.value || !selectedTopic.value) {
    const { cluster, topic, partition } = route.query;
    if (cluster && typeof cluster === 'string') {
      selectedCluster.value = cluster;
    }
    if (topic && typeof topic === 'string') {
      selectedTopic.value = topic;
    }
    if (partition && typeof partition === 'string') {
      const partitionNum = parseInt(partition, 10);
      if (!isNaN(partitionNum)) {
        selectedPartition.value = partitionNum;
      }
    }
  }

  // 加载分区信息并自动查询
  if (selectedCluster.value && selectedTopic.value) {
    await loadPartitions();
    await queryMessages();
  }
});

// 监听 props 变化
watch(() => props.cluster, async (newCluster) => {
  if (newCluster && newCluster !== selectedCluster.value) {
    selectedCluster.value = newCluster;
    await loadPartitions();
    if (selectedCluster.value && selectedTopic.value) {
      await queryMessages();
    }
  }
});

watch(() => props.topic, async (newTopic) => {
  if (newTopic && newTopic !== selectedTopic.value) {
    selectedTopic.value = newTopic;
    selectedPartition.value = 'all';
    partitions.value = [];
    messages.value = [];
    await loadPartitions();
    if (selectedCluster.value && selectedTopic.value) {
      await queryMessages();
    }
  }
});

onUnmounted(() => {
  stopAutoRefresh();
  if (loading.value) {
    apiClient.cancelGetMessages();
  }
});
</script>

<style scoped>
.message-query-tool {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  flex-shrink: 0;
}

.status-bar {
  flex-shrink: 0;
}

.detail-panel {
  flex-shrink: 0;
}

pre {
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
