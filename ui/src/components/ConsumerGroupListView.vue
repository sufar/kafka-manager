<template>
  <div class="flex-1 overflow-hidden px-2">
    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-8">
      <span class="loading loading-spinner loading-sm"></span>
    </div>

    <!-- Empty -->
    <div v-else-if="items.length === 0" class="text-center py-8 text-base-content/50">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 mx-auto mb-2 opacity-50">
        <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
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
      :buffer-size="5"
      v-slot="{ item }"
      @scroll="$emit('scroll', $event)"
    >
      <div
        class="group flex items-center gap-1.5 px-1.5 py-1 rounded cursor-pointer hover:bg-base-200/70"
        @click="handleClick((item as ConsumerGroupItem).group)"
      >
        <!-- Cluster Health Indicator -->
        <div
          class="w-1.5 h-1.5 rounded-full flex-shrink-0"
          :class="{
            'bg-success': getHealth((item as ConsumerGroupItem).group.cluster)?.healthy === true,
            'bg-error': getHealth((item as ConsumerGroupItem).group.cluster)?.healthy === false,
            'bg-warning': getHealth((item as ConsumerGroupItem).group.cluster)?.healthy === undefined
          }"
        ></div>

        <!-- Consumer Group Name with Tooltip -->
        <div class="flex-1 min-w-0 relative">
          <span
            class="text-xs truncate block"
            :title="`${(item as ConsumerGroupItem).group.name} (${(item as ConsumerGroupItem).group.cluster})`"
          >
            {{ (item as ConsumerGroupItem).group.name }}
          </span>
        </div>

        <!-- Cluster Badge -->
        <span class="badge badge-ghost badge-xs flex-shrink-0 truncate max-w-14 text-[10px] px-1">
          {{ (item as ConsumerGroupItem).group.cluster }}
        </span>
      </div>
    </RecycleScroller>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';

interface ConsumerGroupInfo {
  name: string;
  cluster: string;
}

interface ConsumerGroupItem {
  group: ConsumerGroupInfo;
  uid: string;
}

const props = defineProps<{
  items: ConsumerGroupInfo[];
  loading: boolean;
  getHealth: (clusterName: string) => { healthy?: boolean } | undefined;
}>();

const emit = defineEmits<{
  click: [group: ConsumerGroupInfo];
  scroll: [event: Event];
}>();

const itemsWithUid = computed((): ConsumerGroupItem[] => {
  return props.items.map(group => ({
    group,
    uid: `${group.cluster}-${group.name}`
  }));
});

const emptyText = computed(() => {
  return '暂无 Consumer Groups';
});

function handleClick(group: ConsumerGroupInfo) {
  emit('click', group);
}
</script>
