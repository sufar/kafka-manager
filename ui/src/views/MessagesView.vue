<template>
  <div class="flex flex-col h-full overflow-hidden">
    <MessageQueryTool
      :key="`${currentCluster}-${currentTopic}`"
      :cluster="currentCluster"
      :topic="currentTopic"
      class="flex-1"
      @navigate="handleNavigate"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import MessageQueryTool from '@/components/MessageQueryTool.vue';

const route = useRoute();
const router = useRouter();

// 当前选中的 cluster 和 topic（从 URL 获取）
const currentCluster = computed(() => {
  const cluster = route.query.cluster;
  return typeof cluster === 'string' ? cluster : '';
});

const currentTopic = computed(() => {
  const topic = route.query.topic;
  return typeof topic === 'string' ? topic : '';
});

// 处理导航事件
function handleNavigate(routeParam: { path: string; query?: Record<string, string> }) {
  router.push({ path: routeParam.path, query: routeParam.query });
}
</script>
