<template>
  <div class="messages-view h-full flex flex-col">
    <!-- 根据模式显示不同界面 -->
    <template v-if="viewMode === 'simple'">
      <MessageQueryTool
        :key="`${currentCluster}-${currentTopic}`"
        :cluster="currentCluster"
        :topic="currentTopic"
        class="flex-1"
      />
    </template>
    <template v-else>
      <MessagesClassicView class="flex-1" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRoute } from 'vue-router';
import { apiClient } from '@/api/client';
import MessageQueryTool from '@/components/MessageQueryTool.vue';
import MessagesClassicView from './MessagesClassicView.vue';

const route = useRoute();

// 界面模式: 'classic' | 'simple'
const viewMode = ref<'classic' | 'simple'>('simple');

// 当前选中的 cluster 和 topic（从 URL 获取）
const currentCluster = computed(() => {
  const cluster = route.query.cluster;
  return typeof cluster === 'string' ? cluster : '';
});

const currentTopic = computed(() => {
  const topic = route.query.topic;
  return typeof topic === 'string' ? topic : '';
});

// 加载设置
async function loadViewModeSetting() {
  try {
    const settings = await apiClient.getSettings(['ui.message_view_mode']);
    const mode = settings.find(s => s.key === 'ui.message_view_mode')?.value;
    if (mode === 'classic' || mode === 'simple') {
      viewMode.value = mode;
    }
  } catch (e) {
    console.error('Failed to load view mode setting:', e);
  }
}

onMounted(() => {
  loadViewModeSetting();
});
</script>

<style scoped>
.messages-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}
</style>
