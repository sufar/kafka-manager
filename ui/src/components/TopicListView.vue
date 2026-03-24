<template>
  <div class="flex-1 overflow-hidden px-2">
    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-8">
      <span class="loading loading-spinner loading-sm"></span>
    </div>

    <!-- Empty -->
    <div v-else-if="items.length === 0" class="text-center py-8 text-base-content/50">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 mx-auto mb-2 opacity-50">
        <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
      </svg>
      <p class="text-xs">{{ emptyText }}</p>
    </div>

    <!-- Virtual Scroll Items -->
    <RecycleScroller
      v-else
      class="h-full overflow-auto"
      :items="itemsWithUid"
      :item-size="28"
      key-field="uid"
      v-slot="{ item, index }"
      @scroll="$emit('scroll', $event)"
    >
      <div
        class="group flex items-center gap-1.5 px-1.5 py-1 rounded cursor-pointer transition-all duration-200 hover:bg-base-200"
        :class="{ 'bg-primary/10': hoveredIndex === index }"
        @click="handleClick((item as TopicItem).topic)"
        @mouseenter="hoveredIndex = index"
      >
        <!-- Cluster Health Indicator -->
        <div
          class="w-1.5 h-1.5 rounded-full flex-shrink-0"
          :class="{
            'bg-success': getHealth((item as TopicItem).topic.cluster)?.healthy === true,
            'bg-error': getHealth((item as TopicItem).topic.cluster)?.healthy === false,
            'bg-warning': getHealth((item as TopicItem).topic.cluster)?.healthy === undefined
          }"
        ></div>

        <!-- Topic Name with Tooltip -->
        <div class="flex-1 min-w-0 relative">
          <span
            class="text-xs truncate block"
            :title="`${(item as TopicItem).topic.name} (${(item as TopicItem).topic.cluster})`"
          >
            {{ (item as TopicItem).topic.name }}
          </span>
        </div>

        <!-- Cluster Badge -->
        <span class="badge badge-ghost badge-xs flex-shrink-0 truncate max-w-14 text-[10px] px-1">
          {{ (item as TopicItem).topic.cluster }}
        </span>
      </div>
    </RecycleScroller>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';

interface TopicInfo {
  name: string;
  cluster: string;
}

interface TopicItem {
  topic: TopicInfo;
  uid: string;
}

const props = defineProps<{
  items: TopicInfo[];
  loading: boolean;
  emptyText?: string;
  getHealth: (clusterName: string) => { healthy?: boolean } | undefined;
}>();

const emit = defineEmits<{
  click: [topic: TopicInfo];
  scroll: [event: Event];
}>();

const hoveredIndex = ref(-1);

const itemsWithUid = computed((): TopicItem[] => {
  return props.items.map(topic => ({
    topic,
    uid: `${topic.cluster}-${topic.name}`
  }));
});

const emptyText = computed(() => {
  return props.emptyText || '暂无 Topics';
});

function handleClick(topic: TopicInfo) {
  emit('click', topic);
}
</script>
