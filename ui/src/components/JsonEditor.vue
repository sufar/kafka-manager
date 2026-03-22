<template>
  <div class="editor-container" :style="{ height: containerHeight }" ref="containerRef">
    <!-- 高亮显示层 -->
    <pre
      class="highlight-layer"
      ref="highlightRef"
      v-html="highlightedJson"
    ></pre>
    <!-- 透明输入层 -->
    <textarea
      v-model="inputValue"
      class="input-layer"
      ref="inputRef"
      :placeholder="placeholder"
      :required="required"
      spellcheck="false"
      autocomplete="off"
      autocorrect="off"
      autocapitalize="off"
      @input="onInput(($event.target as HTMLTextAreaElement).value)"
      @scroll="onScroll"
    ></textarea>
    <!-- Format Button -->
    <div class="format-button">
      <slot name="format-button"></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from 'vue';
import { formatJson } from '@/utils/json';
import { highlightJsonWithTemplate, clearTemplateCache } from '@/utils/json-highlight';

// 用于强制刷新的响应式变量
const refreshTrigger = ref(0);

// 监听模板变化事件，强制刷新
function handleTemplateChange() {
  clearTemplateCache();
  // 增加触发器值，强制 computed 重新计算
  refreshTrigger.value++;
}

onMounted(() => {
  window.addEventListener('json-highlight-changed', handleTemplateChange as EventListener);
});

onUnmounted(() => {
  window.removeEventListener('json-highlight-changed', handleTemplateChange as EventListener);
});

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
const highlightRef = ref<HTMLPreElement | null>(null);

// 同步 props 变化
watch(() => props.modelValue, (newVal) => {
  inputValue.value = newVal;
});

// 滚动同步
function onScroll(event: Event) {
  const target = event.target as HTMLTextAreaElement;
  if (highlightRef.value) {
    highlightRef.value.scrollTop = target.scrollTop;
    highlightRef.value.scrollLeft = target.scrollLeft;
  }
}

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
  // 读取 refreshTrigger 以建立依赖关系，当模板变化时强制重新计算
  void refreshTrigger.value;
  if (!inputValue.value) return '';
  // 始终显示原始内容的高亮，不自动格式化
  // 只有在用户点击格式化按钮时才会格式化
  // 不传递 isDark，让函数直接从 DOM 获取当前主题
  return highlightJsonWithTemplate(inputValue.value);
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

<style>
/* 深色主题 - 使用全局样式 */
:root[data-theme="dark"] .editor-container,
:root.dark .editor-container {
  border-color: rgba(255, 255, 255, 0.2) !important;
  background: rgba(26, 26, 46, 0.8) !important;
}

:root[data-theme="dark"] .editor-container:focus-within,
:root.dark .editor-container:focus-within {
  border-color: rgba(255, 255, 255, 0.35) !important;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.1);
}
</style>

<style scoped>
.editor-container {
  position: relative;
  width: 100%;
  border: 1px solid #a5a5b5 !important;
  border-radius: 0.5rem;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.9) !important;
}

.editor-container:focus-within {
  border-color: #6366f1 !important;
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.15);
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
  white-space: pre;
  overflow: auto;
  border: none;
}

.highlight-layer {
  z-index: 0;
  pointer-events: none;
  color: oklch(var(--bc));
  background: transparent !important;
  display: block;
  white-space: pre;
}

/* 重置 pre 默认样式 */
pre.highlight-layer {
  margin: 0;
  display: block;
}

/* 确保 span 不影响行高和字符宽度 */
.highlight-layer span {
  line-height: inherit;
  display: inline;
  font-family: inherit;
  font-size: inherit;
  letter-spacing: normal;
  word-spacing: normal;
}

.input-layer {
  z-index: 1;
  background: transparent !important;
  border: none !important;
  outline: none !important;
  resize: none;
  color: transparent !important;
  caret-color: #1e293b !important;
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

/* 深色模式光标颜色 */
[data-theme="dark"] .input-layer {
  caret-color: #f1f5f9 !important;
}

.format-button {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  z-index: 10;
  pointer-events: auto;
}
</style>
