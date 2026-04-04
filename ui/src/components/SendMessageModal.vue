<template>
  <Teleport to="body">
    <dialog ref="modalRef" class="modal" @click.self="close">
      <div class="modal-box w-full max-w-5xl mx-2 md:mx-auto p-5 max-h-[90vh] overflow-y-auto">
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2">
            <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
              </svg>
            </div>
            <div>
              <h3 class="font-bold text-base">{{ t.messages.sendMessage }}</h3>
              <span class="text-xs text-base-content/60 font-mono">{{ topicName }}</span>
            </div>
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="close">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <form @submit.prevent="handleSubmit(false)" class="space-y-4">
          <!-- Partition and Key Row -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- Partition Dropdown -->
            <div>
              <label class="block text-sm font-medium mb-1.5">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 inline mr-1.5 align-text-bottom">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6Z" />
                </svg>
                {{ t.messages.partition }}
              </label>
              <select v-model.number="form.partition" class="select select-bordered select-sm w-full max-w-[120px]" required :disabled="partitions.length === 0">
                <option v-for="p in partitions" :key="p" :value="p">{{ p }}</option>
              </select>
            </div>

            <!-- Key Input -->
            <div>
              <label class="block text-sm font-medium mb-1.5">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 inline mr-1.5 align-text-bottom">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z" />
                </svg>
                {{ t.messages.key }}
              </label>
              <input v-model="form.key" type="text" class="input input-bordered input-sm w-full" :placeholder="t.messages.keyOptional" />
            </div>
          </div>

          <!-- Value Textarea with Format Button -->
          <div>
            <div class="flex items-center justify-between mb-1.5">
              <label class="block text-sm font-medium">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 inline mr-1.5 align-text-bottom">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
                </svg>
                {{ t.messages.value }} <span class="text-error text-xs ml-1">{{ t.messages.required }}</span>
              </label>
            </div>
            <JsonEditor
              ref="jsonEditorRef"
              v-model="form.value"
              :height="isMobile ? 'h-48' : 'h-96'"
              :placeholder="`{&#10;  &quot;id&quot;: 1,&#10;  &quot;data&quot;: &quot;example&quot;&#10;}`"
              :required="true"
            >
              <template #format-button>
                <button
                  type="button"
                  class="btn btn-ghost hover:bg-primary/10 hover:text-primary h-8 w-8 p-0"
                  @click="() => jsonEditorRef?.format()"
                  :title="t.messages.formatJson"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09ZM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 0 0-2.456 2.456ZM16.894 20.567 16.5 21.75l-.394-1.183a2.25 2.25 0 0 0-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 0 0 1.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 0 0 1.423 1.423l1.183.394-1.183.394a2.25 2.25 0 0 0-1.423 1.423Z" />
                  </svg>
                </button>
              </template>
            </JsonEditor>
          </div>

          <!-- Headers Section -->
          <div class="border-t border-base-content/10 pt-4">
            <button type="button" class="btn btn-ghost btn-sm w-full justify-between group" @click="showHeaders = !showHeaders">
              <span class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 transition-transform group-hover:rotate-90" :class="{ 'rotate-90': showHeaders }">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                </svg>
                Headers
                <span v-if="form.headers.length > 0" class="badge badge-primary badge-xs">{{ form.headers.length }}</span>
              </span>
              <span class="text-xs text-base-content/50">{{ showHeaders ? t.messages.hide : t.messages.show }}</span>
            </button>

            <!-- Headers Input -->
            <div v-if="showHeaders" class="mt-3 space-y-2 animate-fadeIn">
              <div v-for="(header, index) in form.headers" :key="index" class="flex gap-2 items-center">
                <input
                  v-model="header.key"
                  type="text"
                  class="input input-bordered input-sm flex-1"
                  placeholder="Header key"
                />
                <span class="text-base-content/30 text-sm">→</span>
                <input
                  v-model="header.value"
                  type="text"
                  class="input input-bordered input-sm flex-1"
                  placeholder="Header value"
                />
                <button type="button" class="btn btn-ghost btn-xs btn-circle text-error hover:bg-error/10" @click="form.headers.splice(index, 1)">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <button type="button" class="btn btn-ghost btn-sm w-full text-primary hover:bg-primary/10" @click="form.headers.push({ key: '', value: '' })">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                </svg>
                {{ t.common.create }}
              </button>
            </div>
          </div>

          <!-- Success Alert -->
          <div v-if="sendSuccess" class="alert alert-success py-2 rounded-xl">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
            </svg>
            <div>
              <p class="font-medium text-sm">{{ t.messages.messageSent }}!</p>
              <p class="text-xs text-base-content/70">{{ offsetLabel }}: <span class="font-mono">{{ lastOffset }}</span></p>
            </div>
          </div>

          <!-- Actions -->
          <div class="modal-action flex-wrap gap-2 pt-3">
            <button type="button" class="btn btn-ghost btn-sm" @click="close">{{ t.messages.cancel }}</button>
            <button type="button" class="btn btn-outline btn-sm" @click="handleSubmit(true)" :disabled="sending">
              {{ continueButtonLabel }}
            </button>
            <button type="submit" class="btn btn-primary btn-sm gap-2" :disabled="sending">
              <svg v-if="!sending" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
              </svg>
              <svg v-else class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ sending ? t.messages.sending : sendButtonLabel }}
            </button>
          </div>
        </form>
      </div>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { useLanguageStore } from '@/stores/language';
import JsonEditor from '@/components/JsonEditor.vue';

const languageStore = useLanguageStore();
const jsonEditorRef = ref<InstanceType<typeof JsonEditor> | null>(null);
const t = computed(() => languageStore.t);

// 检测是否为移动端
const isMobile = ref(false);
onMounted(() => {
  isMobile.value = window.innerWidth < 768;
  const handleResize = () => {
    isMobile.value = window.innerWidth < 768;
  };
  window.addEventListener('resize', handleResize);
  onUnmounted(() => window.removeEventListener('resize', handleResize));
});

const props = defineProps<{
  modelValue: boolean;
  topicName: string;
  clusterName: string;
  partitions: number[];
  offsetLabel?: string; // "Offset" or t.messages.offset
  continueButtonLabel?: string; // "继续" or "Send and Continue"
  sendButtonLabel?: string; // "发送" or "Send"
  initialPartition?: number | null;
  initialKey?: string;
  initialValue?: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  submit: [data: SendMessageData, keepOpen: boolean];
}>();

interface MessageHeader {
  key: string;
  value: string;
}

interface SendMessageData {
  partition: number;
  key: string | undefined;
  value: string;
  headers: MessageHeader[];
}

const modalRef = ref<HTMLDialogElement | null>(null);

const form = reactive<SendMessageData>({
  partition: 0,
  key: '',
  value: '',
  headers: [],
});

const showHeaders = ref(false);
const sending = ref(false);
const sendSuccess = ref(false);
const lastOffset = ref<number | null>(null);

const offsetLabel = computed(() => props.offsetLabel || t.value.messages.offset);
const continueButtonLabel = computed(() => props.continueButtonLabel || t.value.messages.continue);
const sendButtonLabel = computed(() => props.sendButtonLabel || t.value.messages.send);

// 监听 props 变化，初始化表单
watch(() => props.modelValue, async (newVal) => {
  if (newVal) {
    // 打开弹框时初始化表单
    const defaultPartition = props.partitions.length > 0 ? props.partitions[0]! : 0;
    form.partition = props.initialPartition ?? defaultPartition;
    form.key = props.initialKey ?? '';
    form.value = props.initialValue ?? '';
    form.headers = [];
    showHeaders.value = false;
    sending.value = false;
    sendSuccess.value = false;
    lastOffset.value = null;

    // 如果 initialValue 存在且是有效 JSON，自动格式化
    if (props.initialValue) {
      try {
        const parsed = JSON.parse(props.initialValue);
        form.value = JSON.stringify(parsed, null, 2);
      } catch {
        // 不是有效 JSON，保持原样
      }
    }

    await nextTick();
    if (modalRef.value) {
      modalRef.value.showModal();
    }
  } else {
    modalRef.value?.close();
  }
}, { immediate: true });

// 关闭弹框
function close() {
  emit('update:modelValue', false);
}

// 提交
async function handleSubmit(keepOpen: boolean) {
  if (!form.value.trim()) return;

  sending.value = true;

  try {
    emit('submit', {
      partition: form.partition,
      key: form.key || undefined,
      value: form.value,
      headers: form.headers,
    }, keepOpen);

    sendSuccess.value = true;
    lastOffset.value = null; // 由父组件设置

    if (!keepOpen) {
      form.key = '';
      form.value = '';
      form.headers = [];
      showHeaders.value = false;
      setTimeout(() => {
        close();
      }, 500);
    }
    // keepOpen 为 true 时不清空任何内容，方便重复发送相同消息
  } catch (e) {
    console.error('Failed to send message:', e);
  } finally {
    sending.value = false;
  }
}

// 暴露方法给父组件
function setLastOffset(offset: number) {
  lastOffset.value = offset;
}

function setSending(value: boolean) {
  sending.value = value;
}

defineExpose({
  setLastOffset,
  setSending,
});
</script>

<style scoped>
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fadeIn {
  animation: fadeIn 0.2s ease-out;
}

/* 移除背景模糊效果 */
:global(.modal:has(.modal-box)::backdrop) {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:global(dialog[open]::backdrop) {
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
</style>
