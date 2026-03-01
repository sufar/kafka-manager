<template>
  <div v-if="visible" class="fixed inset-0 z-[9999]" @click="close" @contextmenu.prevent="close">
    <div
      class="absolute z-[10000] min-w-[180px] rounded-lg bg-base-100 border border-base-200 shadow-xl p-1"
      :style="{ top: position.y + 'px', left: position.x + 'px' }"
      @click.stop
    >
      <!-- Menu Title -->
      <div class="px-3 py-2 text-sm font-semibold text-base-content border-b border-base-200 mb-1">
        <span class="font-mono">{{ topicName }}</span>
      </div>
      <!-- Menu Items -->
      <ul class="menu menu-sm bg-base-100 w-full">
        <li>
          <a @click="handleAction('viewMessages')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9.776c.112-.017.224-.026.336-.026h15.84c.112 0 .224.009.336.026m0-.026c.298.046.59.116.872.21l1.912.637a1.125 1.125 0 010 2.136l-1.912.637c-.282.094-.574.164-.872.21m-16.8.026c-.298.046-.59.116-.872-.21l-1.912-.637a1.125 1.125 0 010 2.136l1.912.637c.282.094.574.164-.872.21m12.078-6.053a3 3 0 00-2.974-2.723c-.624-.033-1.252.025-1.865.17-.64.151-1.247.382-1.808.683m6.647 1.873c.242.53.412 1.096.503 1.686m-12.078.026c.298-.046.59-.116.872-.21l1.912-.637a1.125 1.125 0 010-2.136l-1.912-.637c-.282-.094-.574-.164-.872-.21m16.8-.026c-.298-.046.59-.116-.872-.21l-1.912-.637a1.125 1.125 0 010-2.136l1.912-.637c.282-.094.574-.164-.872-.21" />
            </svg>
            View Messages
          </a>
        </li>
        <li>
          <a @click="handleAction('viewDetails')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.09.27.09.56 0 .83C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" />
              <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            View Details
          </a>
        </li>
        <li>
          <a @click="handleAction('viewPartitions')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
            </svg>
            View Partitions
          </a>
        </li>
        <li>
          <a @click="handleAction('viewConsumerLag')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125z" />
            </svg>
            View Consumer Lag
          </a>
        </li>
      </ul>
      <hr class="h-px bg-base-200 my-1" />
      <ul class="menu menu-sm bg-base-100 w-full">
        <li>
          <a @click="handleAction('sendMessage')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
            </svg>
            Send Message
          </a>
        </li>
        <li>
          <a @click="handleAction('exportData')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" />
            </svg>
            Export Data
          </a>
        </li>
      </ul>
      <hr class="h-px bg-base-200 my-1" />
      <ul class="menu menu-sm bg-base-100 w-full">
        <li>
          <a class="text-error hover:bg-error/10" @click="handleAction('deleteTopic')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
            </svg>
            Delete Topic
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
  topicName: string;
  clusterName: string;
  position: { x: number; y: number };
}>();

const emit = defineEmits<{
  close: [];
  action: [action: string, topicName: string, clusterName: string];
}>();

const visible = ref(props.visible);

watch(() => props.visible, (val) => {
  visible.value = val;
});

function close() {
  emit('close');
}

function handleAction(action: string) {
  emit('action', action, props.topicName, props.clusterName);
  close();
}
</script>
