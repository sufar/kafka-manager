<template>
  <div class="messages-browser h-full flex flex-col">
    <!-- Top Toolbar -->
    <div class="toolbar flex items-center gap-2 p-2 border-b border-base-content/10 glass rounded-t-xl">
      <!-- Topic Indicator -->
      <div class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-gradient-to-r from-secondary/10 to-accent/10 glow-primary">
        <div class="w-6 h-6 rounded-lg bg-gradient-to-br from-secondary/20 to-accent/20 flex items-center justify-center animate-float">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-secondary">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
          </svg>
        </div>
        <span class="text-xs font-bold text-base-content/80">{{ t.topics.title }}</span>
        <span v-if="selectedTopic" class="text-xs font-mono text-accent">: {{ selectedTopic }}</span>
      </div>

      <div class="w-px h-6 bg-base-content/20" />

      <!-- Refresh/Stop Button -->
      <button class="btn btn-ghost btn-sm" @click="loading ? stopFetching() : fetchMessages()" :disabled="!selectedTopic" :title="loading ? t.common.cancel : t.common.refresh">
        <svg v-if="loading" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
        </svg>
        <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4" :class="{ 'animate-spin': loading }">
          <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
        </svg>
      </button>
      <button class="btn btn-ghost btn-sm" @click="openSendModal" :disabled="!selectedTopic" :title="t.messages.sendMessage">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
        </svg>
      </button>
      <button class="btn btn-ghost btn-sm" @click="exportMessages" :disabled="!selectedTopic" :title="t.topics.exportData">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
        </svg>
      </button>

      <div class="flex-1 min-w-0" />

      <!-- Topic Selector -->
      <select v-if="!topicParam" v-model="selectedTopic" class="select select-bordered select-sm max-w-xs" @change="fetchMessages">
        <option value="">{{ t.messages.selectTopic }}</option>
        <option v-for="topic in topics" :key="topic" :value="topic">{{ topic }}</option>
      </select>

      <!-- Partition Filter -->
      <select v-model.number="filters.partition" class="select select-bordered select-sm w-auto" @change="fetchMessages">
        <option :value="undefined">All</option>
        <option v-for="p in topicPartitions" :key="p" :value="p">{{ p }}</option>
      </select>

      <!-- Search -->
      <input v-model="filters.search" type="text" class="input input-bordered input-sm w-32" :placeholder="t.messages.filter" @change="fetchMessages" />

      <!-- Fetch Mode -->
      <select v-model="filters.fetchMode" class="select select-bordered select-sm w-auto" @change="fetchMessages">
        <option value="oldest">{{ t.messages.oldest }}</option>
        <option value="newest">{{ t.messages.newest }}</option>
      </select>

      <!-- Time Range Filter -->
      <div style="position: relative; display: inline-block;">
        <input v-model="filters.startTime" type="datetime-local" class="input input-bordered input-sm w-40" :placeholder="t.messages.startTime" @change="fetchMessages" />
        <button v-if="filters.startTime" style="position: absolute; right: 0.5rem; top: 50%; transform: translateY(-50%); background: transparent; border: none; cursor: pointer; padding: 0; display: flex; align-items: center; justify-content: center; opacity: 0.5;" class="hover:opacity-100" @click="filters.startTime = ''; fetchMessages()" title="Clear start time">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      <div style="position: relative; display: inline-block;">
        <input v-model="filters.endTime" type="datetime-local" class="input input-bordered input-sm w-40" :placeholder="t.messages.endTime" @change="fetchMessages" />
        <button v-if="filters.endTime" style="position: absolute; right: 0.5rem; top: 50%; transform: translateY(-50%); background: transparent; border: none; cursor: pointer; padding: 0; display: flex; align-items: center; justify-content: center; opacity: 0.5;" class="hover:opacity-100" @click="filters.endTime = ''; fetchMessages()" title="Clear end time">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Messages List (Top Panel) -->
    <div ref="messagesListRef" class="messages-list flex-1 overflow-y-auto min-h-0 relative">
      <div class="w-full bg-base-100/50 rounded-t-xl rounded-b-xl overflow-hidden">
        <table class="table table-sm w-full">
          <thead class="sticky top-0 glass z-10 backdrop-blur-md rounded-t-xl">
            <tr>
              <th class="text-left w-20 bg-gradient-to-r from-primary/10 to-transparent">{{ t.messages.offset }}</th>
              <th class="text-left w-32 bg-gradient-to-r from-secondary/10 to-transparent">{{ t.messages.partition }}</th>
              <th class="text-left w-40 bg-gradient-to-r from-accent/10 to-transparent cursor-pointer hover:bg-accent/5" @click="toggleTimestampSort">
                <div class="flex items-center gap-1">
                  <span>{{ t.messages.timestampLabel }}</span>
                  <svg v-if="sortOrder === 'asc'" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 15.75l7.5-7.5 7.5 7.5" />
                  </svg>
                  <svg v-else-if="sortOrder === 'desc'" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                  </svg>
                </div>
              </th>
              <th class="text-left w-48 bg-gradient-to-r from-info/10 to-transparent">{{ t.messages.key }}</th>
              <th class="text-left bg-gradient-to-r from-success/10 to-transparent">{{ t.messages.value }}</th>
            </tr>
          </thead>
          <tbody>
          <tr v-if="messages.length === 0">
            <td colspan="5" class="text-center py-8 text-base-content/60">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-12 h-12 mx-auto mb-2 opacity-50">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9.776c.112-.017.224-.026.336-.026h15.84c.112 0 .224.009.336.026m0-.026c.298.046.59.116.872.21l1.912.637a1.125 1.125 0 010 2.136l-1.912.637c-.282.094-.574.164-.872.21m-16.8.026c-.298.046.59.116-.872.21l1.912-.637a1.125 1.125 0 010-2.136l-1.912-.637c-.282-.094-.574-.164-.872-.21m12.078-6.053a3 3 0 00-2.974-2.723c-.624-.033-1.252.025-1.865.17-.64.151-1.247.382-1.808.683m6.647 1.873c.242.53.412 1.096.503 1.686m-12.078.026c.298-.046.59-.116-.872-.21l1.912-.637a1.125 1.125 0 010-2.136l-1.912-.637c-.282-.094-.574-.164-.872-.21m16.8-.026c-.298-.046.59-.116-.872-.21l-1.912-.637a1.125 1.125 0 010-2.136l1.912-.637c.282.094.574.164.872-.21" />
              </svg>
              {{ t.messages.noMessages }}
            </td>
          </tr>
          <tr
            v-for="(msg, index) in sortedMessages"
            :key="`${msg.partition}-${msg.offset}`"
            class="cursor-pointer transition-all duration-300 hover:bg-primary/5 hover:shadow-sm border-b border-base-content/5 last:border-0"
            :class="{ 'bg-primary/10 shadow-inner': selectedMessageIndex === index }"
            @click="selectMessage(index)"
          >
            <td class="font-mono text-xs px-4 first:rounded-bl-xl last-of-type:rounded-bl-xl">{{ msg.offset }}</td>
            <td class="py-1 px-3">
              <span class="badge badge-ghost badge-sm">{{ msg.partition }}</span>
            </td>
            <td class="text-xs text-base-content/60">{{ formatTimestamp(msg.timestamp) }}</td>
            <td class="font-mono text-xs px-4">{{ msg.key || '-' }}</td>
            <td class="font-mono text-xs px-4 last:rounded-br-xl">{{ formatMessagePreview(msg.value) }}</td>
          </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Resizer Handle -->
    <div
      class="h-2 flex items-center justify-center bg-base-300/50 backdrop-blur-sm cursor-row-resize select-none z-10 hover:bg-primary/30 transition-all duration-300 shadow-md"
      @mousedown="startResize"
    >
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 12 12" stroke-width="2" stroke="currentColor" class="w-3 h-3 text-base-content/60">
        <path stroke-linecap="round" stroke-linejoin="round" d="M6 1V11M6 1l-3 3m3-3 3 3M6 11l-3-3m3 3 3-3" />
      </svg>
    </div>

    <!-- Message Detail (Bottom Panel) -->
    <div
      class="message-detail overflow-auto glass min-h-0 backdrop-blur-md"
      :style="{ height: detailHeight + 'px', flex: 'none' }"
      @selectstart="handleSelectStart"
      @keydown.ctrl.a.prevent="handleSelectAll"
      @keydown.meta.a.prevent="handleSelectAll"
      tabindex="-1"
    >
      <div v-if="selectedMessage" class="p-2">
        <div class="flex items-center justify-between mb-2 pb-2 border-b border-base-content/10">
          <div class="flex items-center gap-4 text-xs">
            <span class="text-base-content/60">Offset: <span class="font-mono">{{ selectedMessage.offset }}</span></span>
            <span class="text-base-content/60">Partition: <span class="font-mono">{{ selectedMessage.partition }}</span></span>
            <span class="text-base-content/60">Timestamp: <span class="font-mono">{{ formatTimestamp(selectedMessage.timestamp) }}</span></span>
            <span class="text-base-content/60">Size: <span class="font-mono">{{ selectedMessageSize }} bytes</span></span>
          </div>
          <div class="flex items-center gap-2 whitespace-nowrap">
            <label class="text-xs text-base-content/60 whitespace-nowrap">View As:</label>
            <select v-model="messageViewFormat" class="select select-bordered select-xs">
              <option value="json">JSON</option>
              <option value="raw">Raw</option>
              <option value="hex">Hex</option>
            </select>
          </div>
        </div>

        <!-- Key -->
        <div v-if="selectedMessage.key" class="mb-3">
          <div class="flex items-center justify-between mb-1">
            <div class="text-xs font-semibold text-base-content/60">Key</div>
            <button class="btn btn-ghost btn-xs" @click="copyToClipboard(formatKeyValue(selectedMessage.key))" title="Copy Key">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
              </svg>
            </button>
          </div>
          <pre ref="keyPreRef" class="bg-base-200/50 backdrop-blur-sm p-2 rounded-xl text-xs font-mono overflow-auto cursor-text select-text border border-base-content/5" tabindex="0">{{ formatKeyValue(selectedMessage.key) }}</pre>
        </div>

        <!-- Value -->
        <div>
          <div class="flex items-center justify-between mb-1">
            <div class="text-xs font-semibold text-base-content/60">Value</div>
            <button class="btn btn-ghost btn-xs" @click="copyToClipboard(formatMessageValue(selectedMessage.value))" title="Copy Value">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
              </svg>
            </button>
          </div>
          <pre ref="valuePreRef" class="bg-base-200/50 backdrop-blur-sm p-2 rounded-xl text-xs font-mono overflow-auto cursor-text select-text whitespace-pre-wrap border border-base-content/5" tabindex="0">{{ formatMessageValue(selectedMessage.value) }}</pre>
        </div>
      </div>
      <div v-else class="flex items-center justify-center h-full text-base-content/40 text-sm">
        {{ t.messages.selectTopic }}
      </div>
    </div>

    <!-- Status Bar -->
    <div class="status-bar flex items-center justify-between px-3 py-2 text-xs border-t border-base-content/10 glass rounded-b-xl backdrop-blur-md">
      <div class="flex items-center gap-4">
        <span>{{ loading ? t.messages.sending : t.common.ready }}</span>
        <span>[{{ t.messages.messages }} = {{ messages.length }}]</span>
        <span v-if="fetchTime > 0">[{{ t.messages.time }} = {{ fetchTime }}ms]</span>
        <span v-if="selectedMessage">[{{ t.messages.selectedOffset }} = {{ selectedMessage.offset }}]</span>
      </div>
      <div class="flex items-center gap-2">
        <span>{{ t.messages.maxMessages }}</span>
        <input v-model.number="filters.max_messages" type="number" class="input input-bordered input-xs w-16" min="1" max="1000" @change="fetchMessages" />
      </div>
    </div>

    <!-- Send Message Modal -->
    <Teleport to="body">
      <dialog ref="sendModalRef" class="modal" @click.self="closeSendModal">
        <div class="modal-box max-w-lg">
          <form method="dialog">
            <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2">✕</button>
          </form>
          <h3 class="font-bold text-lg flex items-center gap-2 mb-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-info">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
            </svg>
            {{ t.messages.sendMessage }} <span class="font-mono">{{ selectedTopic }}</span>
          </h3>
          <form @submit.prevent="() => handleSendMessage(false)" class="flex flex-col gap-3">
            <!-- Partition Dropdown -->
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ t.messages.partition }}</span>
              </label>
              <select v-model.number="messageForm.partition" class="select select-bordered w-32" required :disabled="topicPartitions.length === 0">
                <option v-for="p in topicPartitions" :key="p" :value="p">{{ p }}</option>
              </select>
            </div>
            <!-- Key Input -->
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ t.messages.key }}</span>
                <span class="label-text-alt">{{ t.messages.optional }}</span>
              </label>
              <input v-model="messageForm.key" type="text" class="input input-bordered w-full" :placeholder="t.messages.optional" />
            </div>
            <!-- Value Textarea -->
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ t.messages.value }}</span>
                <span class="label-text-alt">{{ t.messages.required }}</span>
              </label>
              <textarea v-model="messageForm.value" class="textarea textarea-bordered h-32 font-mono text-sm w-full" required :placeholder="`{&quot;id&quot;: 1, &quot;data&quot;: &quot;example&quot;}`"></textarea>
            </div>
            <!-- Success Alert -->
            <div v-if="sendSuccess" class="alert alert-success py-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
              </svg>
              <span class="text-sm">{{ t.messages.messageSent }}! Offset: {{ lastOffset }}</span>
            </div>
            <!-- Actions -->
            <div class="modal-action">
              <button type="button" class="btn" @click="closeSendModal">{{ t.common.cancel }}</button>
              <button type="button" class="btn btn-primary" @click="handleSendMessage(true)" :disabled="sending">
                {{ t.messages.sendAndNew }}
              </button>
              <button type="submit" class="btn btn-primary" :disabled="sending">
                <svg v-if="sending" class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {{ sending ? t.messages.sending : t.messages.send }}
              </button>
            </div>
          </form>
        </div>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted, inject } from 'vue';
import { useRoute } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';

const route = useRoute();
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

// 注入全局 showToast 方法
const showToast = inject<(type: 'success' | 'error' | 'warning' | 'info', message: string, duration?: number) => void>(
  'showToast',
  (type, message) => {
    // 降级方案：如果是错误则 alert，其他类型忽略
    if (type === 'error') {
      alert(message);
    }
  }
);

// 从 URL 参数获取集群 ID
const clusterParam = computed(() => route.query.cluster as string || '');
const topicParam = computed(() => route.query.topic as string || '');
const actionParam = computed(() => route.query.action as string || '');
const partitionParam = computed(() => {
  const p = route.query.partition as string | undefined;
  return p ? parseInt(p, 10) : undefined;
});
const selectedClusterId = computed(() => clusterParam.value || clusterStore.selectedClusterId);
const topics = ref<string[]>([]);
const selectedTopic = ref<string>('');
const topicPartitions = ref<number[]>([]);

const loading = ref(false);
const fetchTime = ref<number>(0); // 获取消息耗时（毫秒）
const messages = ref<Array<{ partition: number; offset: number; key?: string; value?: string; timestamp?: number }>>([]);
const selectedMessageIndex = ref<number>(-1);
const messageViewFormat = ref<'json' | 'raw' | 'hex'>('json');
const sortOrder = ref<'asc' | 'desc' | ''>('desc'); // 默认按时间戳降序

// 显示错误提示
function showError(message: string) {
  showToast('error', message);
}

// 显示成功提示
function showSuccess(message: string) {
  showToast('success', message);
}

const filters = reactive({
  partition: undefined as number | undefined,
  max_messages: 100,
  search: '',
  fetchMode: 'newest' as 'oldest' | 'newest',
  startTime: '' as string,
  endTime: '' as string,
});

const showSendModal = ref(false);
const sending = ref(false);
const sendSuccess = ref(false);
const lastOffset = ref<number | null>(null);
const sentCount = ref(0); // 连续发送次数
const messageForm = reactive({
  partition: 0,
  key: '',
  value: '',
});

// Resizer
const messagesListRef = ref<HTMLElement>();
const sendModalRef = ref<HTMLDialogElement>();
const keyPreRef = ref<HTMLElement>();
const valuePreRef = ref<HTMLElement>();
const detailHeight = ref<number>(300);
const isResizing = ref(false);

// 在消息详情面板中处理 Ctrl+A，只选中 Key 或 Value 内容
function handleSelectStart(e: Event) {
  const target = e.target as HTMLElement;
  // 如果选中的是 pre 标签内的内容，允许默认行为
  if (target.tagName === 'PRE' || keyPreRef.value?.contains(target) || valuePreRef.value?.contains(target)) {
    return;
  }
  // 否则阻止默认行为
  e.preventDefault();
}

function startResize(e: MouseEvent) {
  isResizing.value = true;
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
  e.preventDefault();
}

function handleResize(e: MouseEvent) {
  if (!isResizing.value || !messagesListRef.value) return;

  const container = messagesListRef.value.parentElement;
  if (!container) return;

  const rect = container.getBoundingClientRect();
  const newDetailHeight = rect.bottom - e.clientY - 28; // 28px for status bar

  // Min/max constraints
  if (newDetailHeight >= 100 && newDetailHeight <= rect.height - 100) {
    detailHeight.value = newDetailHeight;
  }
}

function stopResize() {
  isResizing.value = false;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
}

const selectedMessage = computed(() => {
  if (selectedMessageIndex.value < 0 || selectedMessageIndex.value >= sortedMessages.value.length) {
    return null;
  }
  return sortedMessages.value[selectedMessageIndex.value];
});

const selectedMessageSize = computed(() => {
  if (!selectedMessage.value) return 0;
  const valueSize = selectedMessage.value.value?.length || 0;
  const keySize = selectedMessage.value.key?.length || 0;
  return valueSize + keySize;
});

// 切换时间戳排序
function toggleTimestampSort() {
  if (sortOrder.value === 'desc') {
    sortOrder.value = 'asc';
  } else if (sortOrder.value === 'asc') {
    sortOrder.value = '';
  } else {
    sortOrder.value = 'desc';
  }
}

// 排序后的消息列表
const sortedMessages = computed(() => {
  if (sortOrder.value === '') {
    return messages.value;
  }
  return [...messages.value].sort((a, b) => {
    const tsA = a.timestamp || 0;
    const tsB = b.timestamp || 0;
    return sortOrder.value === 'asc' ? tsA - tsB : tsB - tsA;
  });
});

function selectMessage(index: number) {
  selectedMessageIndex.value = index;
}

function formatMessagePreview(value?: string): string {
  if (!value) return 'null';
  try {
    const parsed = JSON.parse(value);
    return JSON.stringify(parsed).substring(0, 100);
  } catch {
    return value.substring(0, 100);
  }
}

function formatKeyValue(key: string): string {
  try {
    const parsed = JSON.parse(key);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return key;
  }
}

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    showSuccess(t.value.messages.copied || 'Copied!');
  } catch (e) {
    // 降级方案：使用传统的 select + execCommand
    const textArea = document.createElement('textarea');
    textArea.value = text;
    textArea.style.position = 'fixed';
    textArea.style.left = '-999999px';
    document.body.appendChild(textArea);
    textArea.select();
    try {
      document.execCommand('copy');
      showSuccess(t.value.messages.copied || 'Copied!');
    } catch (e) {
      console.error('Failed to copy:', e);
      showError(t.value.toast?.copyFailed || 'Failed to copy');
    }
    document.body.removeChild(textArea);
  }
}

function formatMessageValue(value?: string): string {
  if (!value) return 'null';

  if (messageViewFormat.value === 'json') {
    try {
      const parsed = JSON.parse(value);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return value;
    }
  } else if (messageViewFormat.value === 'hex') {
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

function formatTimestamp(ts?: number): string {
  if (!ts) return '-';
  return new Date(ts).toLocaleString();
}

async function fetchTopics() {
  if (!selectedClusterId.value) return;
  try {
    topics.value = await apiClient.getTopics(selectedClusterId.value);
  } catch (e) {
    console.error('Failed to fetch topics:', e);
  }
}

async function fetchTopicPartitions() {
  if (!selectedClusterId.value || !selectedTopic.value) {
    console.warn('[fetchTopicPartitions] Missing cluster or topic:', { cluster: selectedClusterId.value, topic: selectedTopic.value });
    topicPartitions.value = [0]; // 默认提供 partition 0
    return;
  }
  try {
    const topicDetail = await apiClient.getTopicDetail(selectedClusterId.value, selectedTopic.value);
    const partitions = topicDetail.partitions?.map((p: { id: number }) => p.id) || [];
    topicPartitions.value = partitions.length > 0 ? partitions : [0];
    // 清空 partition 过滤，因为旧值可能不在新的分区列表中
    filters.partition = undefined;
  } catch (e) {
    console.error('Failed to fetch topic partitions:', e);
    topicPartitions.value = [0]; // 降级方案：默认提供 partition 0
  }
}

async function fetchMessages() {
  if (!selectedClusterId.value || !selectedTopic.value) return;

  // 取消上一次的请求，避免并发请求导致超时
  apiClient.cancelGetMessages();

  loading.value = true;
  selectedMessageIndex.value = -1;
  const startTime = performance.now();
  try {
    const params: {
      partition?: number;
      max_messages: number;
      search: string;
      fetchMode?: 'oldest' | 'newest';
      start_time?: number;
      end_time?: number;
    } = {
      max_messages: filters.max_messages,
      search: filters.search,
      fetchMode: filters.fetchMode,
    };

    // 只有当 partition 有值时才传递，不传递表示不过滤
    if (filters.partition !== undefined) {
      params.partition = filters.partition;
    }

    // 传递时间范围过滤（转换为毫秒时间戳）
    if (filters.startTime) {
      params.start_time = new Date(filters.startTime).getTime();
    }
    if (filters.endTime) {
      params.end_time = new Date(filters.endTime).getTime();
    }

    messages.value = await apiClient.getMessages(selectedClusterId.value, selectedTopic.value, params);
    fetchTime.value = Math.round(performance.now() - startTime);
  } catch (e) {
    const error = e as { message: string };
    // 如果是取消请求，不显示错误
    if (error.message === 'AbortError' || error.message.includes('aborted')) {
    } else {
      showError(error.message);
    }
  } finally {
    loading.value = false;
  }
}

function stopFetching() {
  apiClient.cancelGetMessages();
  loading.value = false;
}

// 处理键盘 Ctrl+A 事件，选中 Key 或 Value 内容
function handleSelectAll() {
  if (!selectedMessage.value) return;

  // 根据当前焦点位置决定选中 Key 还是 Value
  let preElement: HTMLElement | null | undefined = null;

  // 检查当前焦点是否在 Key 或 Value 区域
  const activeElement = document.activeElement;
  const isKeyFocused = keyPreRef.value && (activeElement === keyPreRef.value || keyPreRef.value.contains(activeElement));
  const isValueFocused = valuePreRef.value && (activeElement === valuePreRef.value || valuePreRef.value.contains(activeElement));

  // 如果 Key 区域有焦点且有 Key 内容，选中 Key
  if (isKeyFocused && selectedMessage.value.key) {
    preElement = keyPreRef.value;
  }
  // 如果 Value 区域有焦点，选中 Value
  else if (isValueFocused) {
    preElement = valuePreRef.value;
  }
  // 没有焦点时，默认选中 Value 内容
  else if (valuePreRef.value) {
    preElement = valuePreRef.value;
  }
  // 降级方案：如果没有 Value 但有 Key，选中 Key
  else if (keyPreRef.value && selectedMessage.value.key) {
    preElement = keyPreRef.value;
  }

  if (preElement) {
    const range = document.createRange();
    range.selectNodeContents(preElement);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
  }
}

async function openSendModal() {
  // 先获取分区列表（无论之前是否有数据，都重新获取以确保数据最新）
  if (selectedClusterId.value && selectedTopic.value) {
    await fetchTopicPartitions();
  }

  // 如果有 partition 参数，使用它作为默认值
  let partition: number = 0;
  if (partitionParam.value !== undefined && topicPartitions.value.includes(partitionParam.value)) {
    partition = Number(partitionParam.value);
  } else if (topicPartitions.value.length > 0) {
    partition = Number(topicPartitions.value[0]) || 0;
  }
  messageForm.partition = partition;
  messageForm.key = '';
  messageForm.value = '';
  sendSuccess.value = false;
  lastOffset.value = null;
  sentCount.value = 0;
  showSendModal.value = true;
  // 使用 DaisyUI 的 showModal() 方法
  sendModalRef.value?.showModal();
}

function closeSendModal() {
  showSendModal.value = false;
  sendSuccess.value = false;
  sendModalRef.value?.close();
}

async function handleSendMessage(keepOpen: boolean = false) {
  if (!selectedClusterId.value || !selectedTopic.value) return;

  sending.value = true;
  sendSuccess.value = false;
  try {
    const result = await apiClient.sendMessage(selectedClusterId.value, selectedTopic.value, {
      partition: messageForm.partition,
      key: messageForm.key || undefined,
      value: messageForm.value,
    });
    lastOffset.value = result.offset;
    sentCount.value += 1;

    if (keepOpen) {
      // 不清空输入框，保留当前值方便继续发送相同内容
      sendSuccess.value = true;
    } else {
      // 关闭弹框，刷新消息列表
      closeSendModal();
      fetchMessages();
    }
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    sending.value = false;
  }
}

async function exportMessages() {
  if (!selectedClusterId.value || !selectedTopic.value) return;

  // 导出当前已加载的消息，按照当前页面的排序方式排序
  const messagesToExport = sortOrder.value === ''
    ? messages.value
    : [...messages.value].sort((a, b) => {
        const tsA = a.timestamp || 0;
        const tsB = b.timestamp || 0;
        return sortOrder.value === 'asc' ? tsA - tsB : tsB - tsA;
      });

  if (!messagesToExport || messagesToExport.length === 0) {
    showError('No messages to export');
    return;
  }

  try {
    // 检测是否在 Tauri 环境下运行（使用与 apiClient 相同的检测逻辑）
    const isTauriApp = !!(
      (window as any).__TAURI__ ||
      (window as any).__TAURI_INTERNALS__ ||
      (window as any).__TAURI_IPC__ ||
      (window as any)._TAURI_VERSION_ ||
      window.navigator?.userAgent?.includes('Tauri')
    );
    console.log('[exportMessages] Is Tauri:', isTauriApp);

    if (isTauriApp) {
      // Tauri 桌面应用：使用文件系统保存
      console.log('[exportMessages] Attempting Tauri save...');
      try {
        const dialog = await import('@tauri-apps/plugin-dialog');
        console.log('[exportMessages] Dialog module loaded, save function exists:', typeof dialog.save);

        const filePath = await dialog.save({
          filters: [{
            name: 'JSON Files',
            extensions: ['json']
          }],
          defaultPath: `${selectedTopic.value}_export_${Date.now()}.json`
        });

        console.log('[exportMessages] File path:', filePath);

        if (filePath) {
          const fs = await import('@tauri-apps/plugin-fs');
          await fs.writeTextFile(filePath, JSON.stringify(messagesToExport, null, 2));
          showSuccess('Export successful');
        }
      } catch (tauriError) {
        console.error('[exportMessages] Tauri save failed, falling back to download:', tauriError);
        // Tauri 失败时降级到浏览器下载
        throw tauriError;
      }
    } else {
      // 浏览器环境：使用 Blob 下载
      console.log('[exportMessages] Using browser download mode');
      const blob = new Blob([JSON.stringify(messagesToExport, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${selectedTopic.value}_export_${Date.now()}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      showSuccess('Export successful');
    }
  } catch (e) {
    console.error('[exportMessages] Full error:', e);
    console.error('[exportMessages] Error name:', (e as Error).name);
    console.error('[exportMessages] Error message:', (e as Error).message);
    console.error('[exportMessages] Error stack:', (e as Error).stack);

    // 如果 Tauri 保存失败，降级到浏览器下载
    const isTauriApp = !!(
      (window as any).__TAURI__ ||
      (window as any).__TAURI_INTERNALS__ ||
      (window as any).__TAURI_IPC__ ||
      (window as any)._TAURI_VERSION_ ||
      window.navigator?.userAgent?.includes('Tauri')
    );

    if (isTauriApp) {
      console.log('[exportMessages] Falling back to browser download mode');
      try {
        const blob = new Blob([JSON.stringify(messagesToExport, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${selectedTopic.value}_export_${Date.now()}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        showSuccess('Export successful (browser download)');
        return;
      } catch (fallbackError) {
        console.error('[exportMessages] Fallback download also failed:', fallbackError);
      }
    }

    const error = e as { message?: string; code?: string; stack?: string };
    const errorMessage = error.message || error.code || 'Unknown error';
    showError(`Export failed: ${errorMessage}`);
  }
}

watch(selectedClusterId, () => {
  selectedTopic.value = '';
  topicPartitions.value = [];
  messages.value = [];
  selectedMessageIndex.value = -1;
  fetchTopics();
});

// 标志设置是否已加载
let settingsLoaded = false;

// 监听 topic 和 cluster 参数变化（支持跨集群切换）
watch([topicParam, clusterParam], async ([newTopic]) => {
  if (newTopic) {
    selectedTopic.value = newTopic;
    fetchTopicPartitions();
    // 确保设置已加载后再获取消息
    if (!settingsLoaded) {
      await loadSettings();
    }
    fetchMessages();
  }
}, { immediate: true });

// 监听 selectedTopic 变化（从下拉框选择）
watch(selectedTopic, () => {
  if (selectedTopic.value) {
    fetchTopicPartitions();
    fetchMessages();
  } else {
    topicPartitions.value = [];
  }
});

// 监听 partition 参数变化（从树形菜单点击）
watch(partitionParam, (newPartition) => {
  if (newPartition !== undefined) {
    filters.partition = newPartition;
    // 如果是从右键菜单发送消息，预填充 partition 字段
    if (actionParam.value === 'send') {
      messageForm.partition = newPartition;
    }
    if (selectedTopic.value) {
      fetchMessages();
    }
  }
}, { immediate: true });

// 监听 topicPartitions 变化，更新 messageForm.partition 默认值
watch(topicPartitions, (newPartitions) => {
  if (newPartitions.length > 0 && messageForm.partition === 0) {
    messageForm.partition = Number(newPartitions[0]);
  }
});

// 监听 action 参数，自动打开消息发送框
watch(actionParam, (newAction) => {
  if (newAction === 'send' && selectedTopic.value) {
    openSendModal();
  }
}, { immediate: true });

// 保存 max_messages 设置到数据库
async function saveMaxMessagesSetting() {
  try {
    await apiClient.updateSetting('messages.max_messages', filters.max_messages.toString());
  } catch (e) {
    console.error('Failed to save max_messages setting:', e);
  }
}

// 加载设置
async function loadSettings() {
  if (settingsLoaded) return;
  try {
    const settings = await apiClient.getSettings(['messages.max_messages', 'ui.language']);
    for (const setting of settings) {
      if (setting.key === 'messages.max_messages') {
        const savedMax = parseInt(setting.value, 10);
        if (!isNaN(savedMax) && savedMax >= 1 && savedMax <= 1000) {
          filters.max_messages = savedMax;
        }
      }
    }
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
  settingsLoaded = true;
}

// 监听 max_messages 变化，自动保存
watch(() => filters.max_messages, () => {
  saveMaxMessagesSetting();
});

onMounted(async () => {
  // 加载全局设置
  await loadSettings();

  fetchTopics();
});
</script>
