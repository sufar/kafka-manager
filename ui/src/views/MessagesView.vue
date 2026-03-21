<template>
  <div class="messages-view h-full flex flex-col">
    <MessageQueryTool
      :key="`${currentCluster}-${currentTopic}`"
      :cluster="currentCluster"
      :topic="currentTopic"
      class="flex-1"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import MessageQueryTool from '@/components/MessageQueryTool.vue';

const route = useRoute();

// 当前选中的 cluster 和 topic（从 URL 获取）
const currentCluster = computed(() => {
  const cluster = route.query.cluster;
  return typeof cluster === 'string' ? cluster : '';
});

const currentTopic = computed(() => {
  const topic = route.query.topic;
  return typeof topic === 'string' ? topic : '';
});
</script>

<style scoped>
.messages-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}
</style>
