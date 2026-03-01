<template>
  <div v-if="visible" class="fixed inset-0 z-[9999]" @click="close" @contextmenu.prevent="close">
    <div
      class="absolute z-[10000] min-w-[180px] rounded-lg bg-base-100 border border-base-200 shadow-xl p-1"
      :style="{ top: position.y + 'px', left: position.x + 'px' }"
      @click.stop
    >
      <!-- Menu Title -->
      <div class="px-3 py-2 text-sm font-semibold text-base-content border-b border-base-200 mb-1">
        <span>Topics - {{ clusterName }}</span>
      </div>
      <!-- Menu Items -->
      <ul class="menu menu-sm bg-base-100 w-full">
        <li>
          <a @click="handleAction('refreshTopics')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            Refresh Topics
          </a>
        </li>
        <li>
          <a @click="handleAction('createTopic')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
            Create Topic
          </a>
        </li>
        <li>
          <a @click="handleAction('viewAllTopics')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
            </svg>
            View All Topics
          </a>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';

const props = defineProps<{
  visible: boolean;
  clusterName: string;
  position: { x: number; y: number };
}>();

const emit = defineEmits<{
  close: [];
  action: [action: string, clusterName: string];
}>();

const visible = ref(props.visible);

watch(() => props.visible, (val) => {
  visible.value = val;
});

function close() {
  emit('close');
}

function handleAction(action: string) {
  emit('action', action, props.clusterName);
  close();
}
</script>
