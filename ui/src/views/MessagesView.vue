<template>
  <div class="flex flex-col h-full overflow-hidden">
    <!-- Header -->
    <div class="p-3 pb-0">
      <h1 class="text-xl font-bold flex items-center gap-2">
        <button class="btn btn-ghost btn-xs p-1 mr-2" @click="router.back()">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
          </svg>
        </button>
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
          <path stroke-linecap="round" stroke-linejoin="round" d="M8.625 12a.375.375 0 1 1-.75 0 .375.375 0 0 1 .75 0Zm0 0H8.25m4.125 0a.375.375 0 1 1-.75 0 .375.375 0 0 1 .75 0Zm0 0H12m4.125 0a.375.375 0 1 1-.75 0 .375.375 0 0 1 .75 0Zm0 0h-.375M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
        </svg>
        {{ t.messages?.title || '消息查询' }}
      </h1>
    </div>
    <div class="flex-1 overflow-hidden p-3 pt-2">
      <MessageQueryTool
        :key="`${currentCluster}-${currentTopic}`"
        :cluster="currentCluster"
        :topic="currentTopic"
        @navigate="handleNavigate"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { storeToRefs } from 'pinia';
import { useLanguageStore } from '@/stores/language';
import MessageQueryTool from '@/components/MessageQueryTool.vue';

const route = useRoute();
const router = useRouter();
const languageStore = useLanguageStore();
const { t } = storeToRefs(languageStore);

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
