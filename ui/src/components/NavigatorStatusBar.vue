<template>
  <div class="p-1.5 text-xs text-base-content/50 border-t border-base-200 flex-shrink-0">
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-1 flex-shrink-0">
        <!-- Load More Button (Topics only) -->
        <button
          v-if="view === 'topics' && hasMore && itemCount < 10000"
          class="btn btn-ghost btn-xs text-primary"
          :disabled="loadingMore"
          @click="$emit('loadMore')"
        >
          <span v-if="loadingMore" class="loading loading-spinner loading-xs"></span>
          加载更多
        </button>
        <span class="text-xs">Cluster:</span>
        <!-- Cluster Selector -->
        <slot name="cluster-selector"></slot>
      </div>
      <div class="flex items-center gap-2 min-w-0">
        <span class="text-xs text-base-content/50 truncate flex-1 min-w-0">
          <template v-if="view === 'topics'">
            {{ itemCount }} / {{ total }} topics
          </template>
          <template v-else>
            {{ itemCount }} / {{ itemCount }} consumer groups
          </template>
        </span>
        <!-- Refresh Button -->
        <button
          class="btn btn-ghost btn-xs flex-shrink-0"
          :disabled="refreshing"
          @click="$emit('refresh')"
          :title="refreshTitle"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  view: 'topics' | 'consumer-groups';
  itemCount: number;
  total?: number;
  hasMore?: boolean;
  loadingMore?: boolean;
  refreshing?: boolean;
}>();

const emit = defineEmits<{
  refresh: [];
  loadMore: [];
}>();

const refreshTitle = computed(() => {
  return props.view === 'topics' ? '刷新 Topics' : '刷新 Consumer Groups';
});
</script>
