<template>
  <div class="flex h-[100dvh] flex-shrink-0 ml-2">
    <!-- Left Sidebar - Navigator - Desktop Only -->
    <aside
      v-if="!isMobile"
      ref="leftSidebarRef"
      class="h-full glass gradient-border overflow-hidden flex flex-col relative rounded-xl"
      :style="{ width: leftSidebarWidth + 'px', minWidth: '200px', maxWidth: '80vw' }"
    >
      <div class="flex-1 flex flex-col min-h-0">
        <!-- Tree Mode -->
        <ClusterTreeNavigator
          v-if="sidebarMode === 'tree'"
          ref="clusterTreeNavigatorRef"
          @navigate="(route) => $emit('navigate', route)"
          @cluster-context-menu="(event, clusterName) => $emit('cluster-context-menu', event, clusterName)"
          @topic-context-menu="(event, topicName, clusterName) => $emit('topic-context-menu', event, topicName, clusterName)"
          @partition-context-menu="(event, topicName, clusterName, partitionId) => $emit('partition-context-menu', event, topicName, clusterName, partitionId)"
          @topics-folder-context-menu="(event, clusterName) => $emit('topics-folder-context-menu', event, clusterName)"
          @toast="(type, message) => $emit('toast', type, message)"
        />
        <!-- Flat Mode -->
        <TopicNavigator
          v-else
          @navigate="(route) => $emit('navigate', route)"
        />
      </div>
    </aside>

    <!-- Mobile Sidebar Drawer -->
    <aside
      v-if="isMobile && sidebarOpen"
      class="fixed left-0 top-10 h-[calc(100dvh-2.5rem)] w-72 overflow-hidden flex flex-col z-50"
    >
      <!-- Mobile Sidebar Header -->
      <div class="flex items-center justify-between p-3 border-b border-base-200 flex-shrink-0">
        <span class="font-bold">Menu</span>
        <button
          class="btn btn-ghost btn-circle btn-sm"
          @click="$emit('close-sidebar')"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="flex-1 flex flex-col min-h-0">
        <!-- Tree Mode -->
        <ClusterTreeNavigator
          v-if="sidebarMode === 'tree'"
          ref="clusterTreeNavigatorRef"
          @navigate="(route) => $emit('navigate', route)"
          @cluster-context-menu="(event, clusterName) => $emit('cluster-context-menu', event, clusterName)"
          @topic-context-menu="(event, topicName, clusterName) => $emit('topic-context-menu', event, topicName, clusterName)"
          @partition-context-menu="(event, topicName, clusterName, partitionId) => $emit('partition-context-menu', event, topicName, clusterName, partitionId)"
          @topics-folder-context-menu="(event, clusterName) => $emit('topics-folder-context-menu', event, clusterName)"
          @toast="(type, message) => $emit('toast', type, message)"
        />
        <!-- Flat Mode -->
        <TopicNavigator
          v-else
          @navigate="(route) => $emit('navigate', route)"
        />
      </div>
    </aside>

    <!-- Resizer Handle - Desktop Only -->
    <div
      v-if="!isMobile"
      ref="resizerRef"
      class="resizer w-1 cursor-col-resize bg-base-content/5 hover:bg-base-content/10 transition-all z-50 flex-shrink-0 group flex items-center justify-center"
      @mousedown="startResize"
      :title="t.mainLayout.dragToResize || '拖动以调整侧边栏宽度'"
    >
      <!-- 拖动指示器 -->
      <div class="w-px h-8 bg-base-content/20 group-hover:bg-primary/40 transition-all rounded"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import ClusterTreeNavigator from '@/components/ClusterTreeNavigator.vue';
import TopicNavigator from '@/components/TopicNavigator.vue';
import { useLanguageStore } from '@/stores/language';

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const props = defineProps<{
  isMobile: boolean;
  sidebarMode: 'tree' | 'flat';
  sidebarOpen: boolean;
}>();

const emit = defineEmits<{
  navigate: [route: { path: string; query?: Record<string, string> }];
  'cluster-context-menu': [event: MouseEvent, clusterName: string];
  'topic-context-menu': [event: MouseEvent, topicName: string, clusterName: string];
  'partition-context-menu': [event: MouseEvent, topicName: string, clusterName: string, partitionId: number];
  'topics-folder-context-menu': [event: MouseEvent, clusterName: string];
  toast: [type: 'success' | 'error' | 'warning' | 'info', message: string];
  'close-sidebar': [];
}>();

// Sidebar width management
const leftSidebarWidth = ref(280);
const leftSidebarRef = ref<HTMLElement>();
const resizerRef = ref<HTMLElement>();
const isResizing = ref(false);
const startX = ref(0);
const startWidth = ref(0);

function startResize(e: MouseEvent) {
  e.preventDefault();
  isResizing.value = true;
  startX.value = e.pageX;
  startWidth.value = leftSidebarWidth.value;
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
}

function handleResize(e: MouseEvent) {
  const delta = e.pageX - startX.value;
  leftSidebarWidth.value = Math.max(200, Math.min(startWidth.value + delta, window.innerWidth * 0.8));
}

function stopResize() {
  isResizing.value = false;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
}

// Ref for ClusterTreeNavigator
const clusterTreeNavigatorRef = ref<InstanceType<typeof ClusterTreeNavigator>>();

// Expose methods to parent
defineExpose({
  clusterTreeNavigatorRef
});

// Cleanup on unmount
onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
});
</script>

<style scoped>
.resizer {
  touch-action: none;
}
</style>
