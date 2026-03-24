<template>
  <div class="px-1.5 py-1 flex-shrink-0">
    <div class="relative">
      <input
        v-model="searchQuery"
        type="text"
        class="input input-bordered input-sm w-full pr-8"
        :placeholder="placeholder"
        @input="handleInput"
      />
      <svg
        v-if="!searchQuery"
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        stroke-width="1.5"
        stroke="currentColor"
        class="w-4 h-4 absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40"
      >
        <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
      </svg>
      <button
        v-else
        class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content"
        @click="clearSearch"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  modelValue: string;
  view: 'topics' | 'consumer-groups';
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
  search: [];
}>();

const searchQuery = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
});

const placeholder = computed(() => {
  return props.view === 'topics' ? '搜索 Topic...' : '搜索 Consumer Group...';
});

let debounceTimer: number | null = null;

function handleInput() {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
  debounceTimer = window.setTimeout(() => {
    emit('search');
  }, 300);
}

function clearSearch() {
  emit('update:modelValue', '');
  emit('search');
}
</script>
