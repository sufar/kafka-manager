<template>
  <div v-if="visible" class="fixed inset-0 z-[9999]" @click="close" @contextmenu.prevent="close">
    <div
      class="absolute z-[10000] min-w-[180px] rounded-lg bg-base-100 border border-base-200 shadow-xl p-1"
      :style="{ top: position.y + 'px', left: position.x + 'px' }"
      @click.stop
    >
      <!-- Menu Title -->
      <div class="px-3 py-2 text-sm font-semibold text-base-content border-b border-base-200 mb-1">
        <span class="font-mono">{{ clusterName }} #{{ partitionId }}</span>
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
          <a @click="handleAction('sendMessage')">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
            </svg>
            Send Message
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
  partitionId: number;
  position: { x: number; y: number };
}>();

const emit = defineEmits<{
  close: [];
  action: [action: string, topicName: string, clusterName: string, partitionId: number];
}>();

const visible = ref(props.visible);

watch(() => props.visible, (val) => {
  visible.value = val;
});

function close() {
  emit('close');
}

function handleAction(action: string) {
  emit('action', action, props.topicName, props.clusterName, props.partitionId);
  close();
}
</script>
