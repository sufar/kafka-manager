<template>
  <div class="editor-container" :style="{ height: containerHeight }">
    <!-- 高亮显示层 -->
    <pre
      class="highlight-layer"
      v-html="highlightedJson"
    ></pre>
    <!-- 透明输入层 -->
    <textarea
      v-model="inputValue"
      class="input-layer"
      :placeholder="placeholder"
      :required="required"
      spellcheck="false"
      autocomplete="off"
      autocorrect="off"
      autocapitalize="off"
      @input="onInput(($event.target as HTMLTextAreaElement).value)"
    ></textarea>
    <!-- Format Button Slot -->
    <div class="format-button">
      <slot name="format-button"></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { formatJson, highlightJson } from '@/utils/json';

const props = withDefaults(defineProps<{
  modelValue: string;
  placeholder?: string;
  required?: boolean;
  height?: string;
}>(), {
  placeholder: '',
  required: false,
  height: 'h-64',
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const inputValue = ref(props.modelValue);

// 同步 props 变化
watch(() => props.modelValue, (newVal) => {
  inputValue.value = newVal;
});

// 容器高度
const containerHeight = computed(() => {
  if (props.height === 'h-64') return '16rem';
  // 支持 Tailwind h-* 类或自定义高度
  if (props.height?.startsWith('h-')) {
    const num = parseInt(props.height.replace('h-', ''), 10);
    return `${num * 0.25}rem`;
  }
  return props.height || '16rem';
});

// 高亮后的 JSON
const highlightedJson = computed(() => {
  if (!inputValue.value) return '';
  try {
    const parsed = JSON.parse(inputValue.value);
    const result = highlightJson(JSON.stringify(parsed, null, 2));
    console.log('[JsonEditor] Highlighted JSON:', result);
    return result;
  } catch (e) {
    // 不是有效 JSON，显示原始内容用于高亮尝试
    const result = highlightJson(inputValue.value);
    console.log('[JsonEditor] Highlighted raw:', result);
    return result;
  }
});

// 监听输入变化
function onInput(value: string) {
  emit('update:modelValue', value);
}

// 格式化 JSON
function handleFormat() {
  if (!inputValue.value) return;
  const formatted = formatJson(inputValue.value);
  inputValue.value = formatted;
  emit('update:modelValue', formatted);
}

// 暴露方法给父组件
defineExpose({
  format: handleFormat,
});
</script>

<style scoped>
.editor-container {
  position: relative;
  width: 100%;
  border: 1px solid oklch(var(--bc) / 0.1);
  border-radius: 0.5rem;
  overflow: hidden;
  background: transparent;
}

.highlight-layer,
.input-layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  margin: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.875rem;
  line-height: 1.5rem;
  padding: 0.625rem;
  padding-top: 2.5rem;
  padding-right: 3rem;
  white-space: pre-wrap;
  word-break: break-all;
  overflow: auto;
}

.highlight-layer {
  z-index: 0;
  pointer-events: none;
  color: oklch(var(--bc));
  background: transparent !important;
}

.input-layer {
  z-index: 1;
  background: transparent !important;
  border: none !important;
  outline: none !important;
  resize: none;
  color: transparent !important;
  caret-color: oklch(var(--bc));
  -webkit-text-fill-color: transparent !important;
  text-fill-color: transparent;
}

.input-layer::placeholder {
  color: oklch(var(--bc) / 0.4);
  -webkit-text-fill-color: oklch(var(--bc) / 0.4);
  text-fill-color: oklch(var(--bc) / 0.4);
}

.input-layer:focus {
  border-color: oklch(var(--p) / 0.5) !important;
  box-shadow: 0 0 0 2px oklch(var(--p) / 0.2);
}

.format-button {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  z-index: 10;
}

/* 语法高亮颜色 - 柔和现代风格 */
:deep(.text-secondary) {
  color: #7c3aed !important;
  font-weight: 600;
}
[data-theme="dark"] :deep(.text-secondary) {
  color: #a78bfa !important;
}
:deep(.text-accent) {
  color: #059669 !important;
}
[data-theme="dark"] :deep(.text-accent) {
  color: #34d399 !important;
}
:deep(.text-info) {
  color: #0284c7 !important;
  font-weight: 700;
}
[data-theme="dark"] :deep(.text-info) {
  color: #38bdf8 !important;
}
:deep(.text-warning) {
  color: #9ca3af !important;
  font-weight: 700;
}
[data-theme="dark"] :deep(.text-warning) {
  color: #6b7280 !important;
}
:deep(.text-base-content) {
  color: #475569 !important;
}
[data-theme="dark"] :deep(.text-base-content) {
  color: #94a3b8 !important;
}
</style>
