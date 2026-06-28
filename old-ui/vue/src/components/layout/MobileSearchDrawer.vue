<template>
  <div
    class="fixed inset-0 bg-base-100 z-[60] md:hidden flex flex-col"
  >
    <div class="flex items-center gap-2 p-2 border-b border-base-200">
      <button
        class="btn btn-ghost btn-circle btn-sm"
        @click="$emit('close')"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
      <input
        v-model="searchQuery"
        type="text"
        class="input input-bordered flex-1"
        :placeholder="t.layout.searchPlaceholder"
        @input="performSearch"
      />
    </div>
    <div class="flex-1 overflow-y-auto">
      <div v-if="searchLoading" class="p-4 text-center">
        <span class="loading loading-spinner loading-md"></span>
      </div>
      <div v-else-if="searchResults.length === 0 && searchQuery" class="p-4 text-center text-base-content/60">
        {{ t.layout.noTopicsFound }}
      </div>
      <div v-else>
        <div
          v-for="result in searchResults"
          :key="`${result.cluster}-${result.topic}`"
          class="flex items-center justify-between p-4 hover:bg-base-200 cursor-pointer border-b border-base-200"
          @click="$emit('select-topic', result.cluster, result.topic); $emit('close')"
        >
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-secondary flex-shrink-0">
                <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
              </svg>
              <span class="font-mono">{{ result.topic }}</span>
            </div>
          </div>
          <div class="badge badge-ghost badge-sm ml-2">
            {{ result.cluster }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const searchQuery = ref('');
const searchResults = ref<{ cluster: string; topic: string }[]>([]);
const searchLoading = ref(false);

defineEmits<{
  close: [];
  'select-topic': [cluster: string, topic: string];
}>();

async function performSearch() {
  const query = searchQuery.value.trim();
  if (!query) return;

  searchLoading.value = true;
  try {
    searchResults.value = await apiClient.searchTopics(query);
  } catch (e) {
    console.error('[mobile search] error:', e);
  } finally {
    searchLoading.value = false;
  }
}
</script>
