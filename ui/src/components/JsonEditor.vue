<template>
  <div class="json-editor">
    <!-- Textarea -->
    <textarea
      :value="modelValue"
      @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
      class="textarea textarea-bordered font-mono text-sm w-full resize-none focus:outline-none focus:ring-2 focus:ring-primary/20"
      :class="textareaClass"
      :placeholder="placeholder"
      :required="required"
    ></textarea>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { formatJson } from '@/utils/json';

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

// 动态 class
const textareaClass = computed(() => {
  return props.height;
});

// 格式化 JSON
function handleFormat() {
  if (!props.modelValue) return;
  const formatted = formatJson(props.modelValue);
  emit('update:modelValue', formatted);
}

// 暴露方法给父组件
defineExpose({
  format: handleFormat,
});
</script>
