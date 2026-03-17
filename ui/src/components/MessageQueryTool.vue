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

      <!-- 发送消息 -->
      <button class="btn btn-ghost btn-sm" @click="openSendModal" title="发送消息">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
        </svg>
      </button>
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
          <button class="btn btn-ghost btn-xs ml-2" :disabled="messages.length === 0" @click="exportMessages" title="导出消息">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
            </svg>
          </button>
        </span>
        <span v-if="error" class="text-error">{{ error }}</span>
      </div>
    </div>

    <!-- 消息列表 -->
    <div class="flex-1 overflow-hidden bg-base-100">
      <!-- Desktop Table with Virtual Scroll -->
      <div class="hidden md:flex md:flex-col h-full">
        <!-- Table Header -->
        <div class="flex bg-base-200 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide w-full">
          <div class="w-12 flex-shrink-0">分区</div>
          <div class="w-16 flex-shrink-0">Offset</div>
          <div class="w-28 flex-shrink-0">时间戳</div>
          <div class="w-20 flex-shrink-0">Key</div>
          <div class="flex-1">Value</div>
          <div class="w-10 flex-shrink-0 text-center">操作</div>
        </div>
        <!-- Virtual Scroll List -->
        <RecycleScroller
          v-if="messages.length > 0"
          class="flex-1 overflow-auto w-full"
          :items="messages"
          :item-size="24"
          key-field="uid"
          v-slot="{ item }"
        >
          <div
            class="flex items-center px-2 py-0.5 hover:bg-base-200/50 transition-colors border-b border-base-200/30 cursor-pointer w-full"
            :class="{ 'bg-primary/20': selectedMessage?.partition === getMsgPartition(item) && selectedMessage?.offset === getMsgOffset(item) }"
            style="height: 24px;"
            @click="selectedMessage = (item as any)"
          >
            <div class="w-12 flex-shrink-0 text-[10px]">
              <span class="badge badge-ghost badge-xs scale-90">{{ getMsgPartition(item) }}</span>
            </div>
            <div class="w-16 flex-shrink-0 text-[10px] font-mono">{{ getMsgOffset(item) }}</div>
            <div class="w-28 flex-shrink-0 text-[10px] text-base-content/60 whitespace-nowrap">{{ formatTime(getMsgTimestamp(item)) }}</div>
            <div class="w-20 flex-shrink-0 text-[10px] font-mono truncate">{{ getMsgKey(item) || '-' }}</div>
            <div class="flex-1 text-[10px] font-mono truncate pr-2" style="min-width: 0;">{{ getMsgValue(item) }}</div>
            <div class="w-10 flex-shrink-0 text-[10px] flex items-center justify-center">
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click.stop="copyMessageValue(item as any)" title="复制 Value (JSON 格式化)">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
          </div>
        </RecycleScroller>
        <div v-if="messages.length === 0 && !loading" class="flex-1 flex flex-col items-center justify-center text-base-content/50">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50 mb-2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
          </svg>
          <span>暂无消息</span>
        </div>
      </div>

      <!-- Mobile Card View with Virtual Scroll -->
      <RecycleScroller
        v-if="messages.length > 0"
        class="md:hidden overflow-auto p-2 pb-20"
        :items="messages"
        :item-size="70"
        key-field="uid"
        v-slot="{ item }"
      >
        <div
          class="card bg-base-100 border border-base-200 p-2 shadow-sm mb-2 cursor-pointer"
          :class="{ 'bg-primary/20 border-primary/50': selectedMessage?.partition === getMsgPartition(item) && selectedMessage?.offset === getMsgOffset(item) }"
          style="height: 62px;"
          @click="selectedMessage = (item as any)"
        >
          <div class="flex items-center justify-between mb-1">
            <div class="flex items-center gap-2">
              <span class="badge badge-ghost badge-xs">P{{ getMsgPartition(item) }}</span>
              <span class="text-xs font-mono text-base-content/70">#{{ getMsgOffset(item) }}</span>
            </div>
            <span class="text-xs text-base-content/50">{{ formatTime(getMsgTimestamp(item)) }}</span>
          </div>
          <div v-if="getMsgKey(item)" class="text-xs font-mono text-secondary mb-1 truncate">
            Key: {{ getMsgKey(item) }}
          </div>
          <div class="text-xs font-mono truncate text-base-content/80">
            {{ truncate(getMsgValue(item), 100) }}
          </div>
        </div>
      </RecycleScroller>
      <div v-else-if="messages.length === 0 && !loading" class="md:hidden h-full flex flex-col items-center justify-center text-base-content/50 p-2">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50 mb-2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
        </svg>
        <span>暂无消息</span>
      </div>
    </div>

    <!-- Detail Panel (Sticky at bottom with resize handle) -->
    <div v-if="selectedMessage" class="detail-panel border-t border-base-300 bg-base-200/30 flex flex-col" :style="{ height: panelHeight + 'px' }">
      <!-- Resize Handle -->
      <div class="resize-handle h-1 cursor-row-resize bg-base-300 hover:bg-primary/50 transition-colors flex items-center justify-center" @mousedown="startResize">
        <div class="w-8 h-0.5 bg-base-content/20 rounded-full"></div>
      </div>
      <div class="flex-1 overflow-hidden p-2">
        <div class="flex items-center justify-between mb-1.5 pb-1 border-b border-base-content/10">
          <div class="flex items-center gap-2 flex-wrap">
            <h4 class="text-xs font-bold">消息详情</h4>
            <span class="text-[10px] text-base-content/50">Partition: <span class="font-mono">{{ selectedMessage.partition }}</span></span>
            <span class="text-[10px] text-base-content/50">Offset: <span class="font-mono">{{ selectedMessage.offset }}</span></span>
            <span class="text-[10px] text-base-content/50">{{ formatTime(selectedMessage.timestamp) }}</span>
          </div>
          <div class="flex items-center gap-1">
            <button class="btn btn-ghost btn-xs px-1" @click="selectedMessage = null">关闭</button>
          </div>
        </div>
        <div class="space-y-1.5 text-[10px] h-[calc(100%-32px)] overflow-auto flex flex-col">
          <div v-if="selectedMessage.key" class="mb-1">
            <div class="flex items-center justify-between mb-0.5">
              <div class="text-base-content/50 text-[10px] font-semibold">Key:</div>
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="copyKey" title="复制 Key">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
            <div class="bg-base-100 p-1 rounded text-[10px] font-mono border border-base-content/5 truncate">{{ selectedMessage.key }}</div>
          </div>
          <div class="flex flex-col flex-1">
            <div class="flex items-center justify-between mb-0.5">
              <div class="text-base-content/50 text-[10px] font-semibold flex items-center gap-1">
                Value:
                <select v-model="valueViewFormat" class="select select-bordered select-xs scale-90 origin-left">
                  <option value="json">JSON</option>
                  <option value="raw">Raw</option>
                  <option value="hex">Hex</option>
                </select>
              </div>
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="copyValue" title="复制 Value">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
            <pre class="bg-base-100 p-1.5 rounded text-[10px] font-mono overflow-auto whitespace-pre-wrap border border-base-content/5 flex-1">{{ formatValue(selectedMessage.value, valueViewFormat) }}</pre>
          </div>
        </div>
      </div>
    </div>

    <!-- Send Message Modal -->
    <Teleport to="body">
      <dialog ref="sendModalRef" class="modal" @click.self="closeSendModal">
        <div class="modal-box w-full max-w-lg mx-2 md:mx-auto">
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="closeSendModal">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
          <h3 class="font-bold text-lg flex items-center gap-2 mb-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-info">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
            </svg>
            发送消息 <span class="font-mono text-sm truncate max-w-[150px] md:max-w-xs">{{ selectedTopic }}</span>
          </h3>
          <form @submit.prevent="() => handleSendMessage(false)" class="flex flex-col gap-3">
            <!-- Partition Dropdown -->
            <div>
              <label class="label">
                <span class="label-text font-medium">分区</span>
              </label>
              <select v-model.number="messageForm.partition" class="select select-bordered w-full sm:w-32" required :disabled="partitions.length === 0">
                <option v-for="p in partitions" :key="p" :value="p">{{ p }}</option>
              </select>
            </div>
            <!-- Key Input -->
            <div>
              <label class="label">
                <span class="label-text font-medium">Key</span>
                <span class="label-text-alt">可选</span>
              </label>
              <input v-model="messageForm.key" type="text" class="input input-bordered w-full" placeholder="可选" />
            </div>
            <!-- Value Textarea -->
            <div>
              <label class="label">
                <span class="label-text font-medium">Value</span>
                <span class="label-text-alt">必填</span>
              </label>
              <textarea v-model="messageForm.value" class="textarea textarea-bordered h-24 sm:h-32 font-mono text-sm w-full" required placeholder='{"id": 1, "data": "example"}'></textarea>
            </div>
            <!-- Success Alert -->
            <div v-if="sendSuccess" class="alert alert-success py-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
              </svg>
              <span class="text-sm">消息发送成功! Offset: {{ lastOffset }}</span>
            </div>
            <!-- Actions -->
            <div class="modal-action flex-wrap">
              <button type="button" class="btn" @click="closeSendModal">取消</button>
              <button type="button" class="btn btn-primary" @click="handleSendMessage(true)" :disabled="sending">
                发送并继续
              </button>
              <button type="submit" class="btn btn-primary" :disabled="sending">
                <svg v-if="sending" class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {{ sending ? '发送中...' : '发送' }}
              </button>
            </div>
          </form>
        </div>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import { RecycleScroller } from 'vue-virtual-scroller';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const route = useRoute();
const { showSuccess } = useToast();

interface Message {
  partition: number;
  offset: number;
  key: string | null;
  value: string | null;
  timestamp: number | null;
  uid: string;
}

// Props - 从父组件接收 cluster 和 topic
const props = defineProps<{
  cluster?: string;
  topic?: string;
}>();

// 状态
const partitions = ref<number[]>([]);
const messages = ref<Message[]>([]);
const selectedMessage = ref<any>(null);
const valueViewFormat = ref<'json' | 'raw' | 'hex'>('json');
const panelHeight = ref(280); // 默认高度增加到 280px

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

// 发送消息弹框状态
const sendModalRef = ref<HTMLDialogElement | null>(null);
const sending = ref(false);
const sendSuccess = ref(false);
const lastOffset = ref<number | null>(null);
const messageForm = ref({
  partition: 0,
  key: '',
  value: '',
});

// 计算属性
const canQuery = computed(() => {
  return !!selectedCluster.value && !!selectedTopic.value && !loading.value;
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

    messages.value = result.map((msg: any, index: number) => ({
      partition: msg.partition,
      offset: msg.offset,
      key: msg.key,
      value: msg.value,
      timestamp: msg.timestamp,
      uid: `${msg.partition}-${msg.offset}-${index}`,
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

// 发送消息弹框控制
function openSendModal() {
  // 如果没有选中分区，默认选第一个
  if (partitions.value.length > 0 && messageForm.value.partition === 0) {
    messageForm.value.partition = partitions.value[0] ?? 0;
  }
  sendModalRef.value?.showModal();
}

function closeSendModal() {
  sendModalRef.value?.close();
  sendSuccess.value = false;
  lastOffset.value = null;
}

async function handleSendMessage(keepOpen: boolean) {
  if (!selectedCluster.value || !selectedTopic.value) return;
  if (!messageForm.value.value.trim()) return;

  sending.value = true;
  sendSuccess.value = false;

  try {
    const result = await apiClient.sendMessage(selectedCluster.value, selectedTopic.value, {
      partition: messageForm.value.partition,
      key: messageForm.value.key || undefined,
      value: messageForm.value.value,
    });

    lastOffset.value = result.offset;
    sendSuccess.value = true;
    showSuccess('消息发送成功');

    if (!keepOpen) {
      // 清空表单并关闭弹框
      setTimeout(() => {
        messageForm.value.key = '';
        messageForm.value.value = '';
        closeSendModal();
      }, 500);
    } else {
      // 只清空 value，保留 key 和 partition
      messageForm.value.value = '';
    }
  } catch (e) {
    console.error('Failed to send message:', e);
  } finally {
    sending.value = false;
  }
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

// 辅助函数：获取消息属性
function getMsgPartition(item: any): number { return item?.partition ?? 0; }
function getMsgOffset(item: any): number { return item?.offset ?? 0; }
function getMsgKey(item: any): string | null { return item?.key; }
function getMsgValue(item: any): string | null { return item?.value; }
function getMsgTimestamp(item: any): number | null { return item?.timestamp; }

// 格式化值为不同格式
function formatValue(value: string | null, format: 'json' | 'raw' | 'hex'): string {
  if (!value) return 'null';

  if (format === 'json') {
    try {
      const parsed = JSON.parse(value);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return value;
    }
  } else if (format === 'hex') {
    try {
      const bytes = new TextEncoder().encode(value);
      return Array.from(bytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ');
    } catch {
      return value;
    }
  }
  return value;
}

function copyKey() {
  if (!selectedMessage.value?.key) return;
  navigator.clipboard.writeText(selectedMessage.value.key).then(() => {
    showSuccess('Key 已复制到剪贴板', 2000);
  });
}

function copyValue() {
  if (!selectedMessage.value?.value) return;
  const text = formatValue(selectedMessage.value.value, valueViewFormat.value);
  navigator.clipboard.writeText(text).then(() => {
    showSuccess('Value 已复制到剪贴板', 2000);
  });
}

function copyMessageValue(msg: any) {
  if (!msg?.value) return;
  // 默认按 JSON 格式化复制
  const text = formatValue(msg.value, 'json');
  navigator.clipboard.writeText(text).then(() => {
    showSuccess('Value 已复制到剪贴板', 2000);
  });
}

onMounted(async () => {
  // 加载设置（包括 max_messages）
  await loadSettings();

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
  if (loading.value) {
    apiClient.cancelGetMessages();
  }
  stopResize();
});

// 加载设置（从数据库）
async function loadSettings() {
  try {
    const settings = await apiClient.getSettings(['messages.max_messages']);
    for (const setting of settings) {
      if (setting.key === 'messages.max_messages') {
        const savedMax = parseInt(setting.value, 10);
        if (!isNaN(savedMax) && savedMax >= 1 && savedMax <= 10000) {
          maxMessages.value = savedMax;
        }
      }
    }
  } catch (e) {
    // 静默失败 - 使用默认值
    console.debug('Settings load failed (using defaults):', (e as { message?: string }).message);
  }
}

// 保存 max_messages 设置到数据库
async function saveMaxMessagesSetting() {
  try {
    await apiClient.updateSetting('messages.max_messages', maxMessages.value.toString());
  } catch (e) {
    console.error('Failed to save max_messages setting:', e);
  }
}

// 监听 max_messages 变化，自动保存
watch(() => maxMessages.value, () => {
  saveMaxMessagesSetting();
});

// 拖动调整高度
let isResizing = false;
let startY = 0;
let startHeight = 0;

function startResize(e: MouseEvent) {
  isResizing = true;
  startY = e.clientY;
  startHeight = panelHeight.value;
  document.addEventListener('mousemove', onResize);
  document.addEventListener('mouseup', stopResize);
  e.preventDefault();
}

function onResize(e: MouseEvent) {
  if (!isResizing) return;
  const delta = startY - e.clientY; // 向上拖动增加高度
  const newHeight = startHeight + delta;
  panelHeight.value = Math.max(150, Math.min(600, newHeight)); // 限制高度范围 150-600px
}

function stopResize() {
  isResizing = false;
  document.removeEventListener('mousemove', onResize);
  document.removeEventListener('mouseup', stopResize);
}
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

.resize-handle {
  user-select: none;
}

pre {
  white-space: pre-wrap;
  word-break: break-all;
}
.vue-recycle-scroller {
  position: relative;
}

.vue-recycle-scroller__item-wrapper {
  flex: 1;
}

.vue-recycle-scroller__item-view {
  position: absolute;
  top: 0;
  left: 0;
  will-change: transform;
}
</style>
