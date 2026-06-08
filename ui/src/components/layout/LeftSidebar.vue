<template>
  <div class="flex h-full flex-shrink-0">
    <!-- Left Sidebar - Navigator - Desktop Only -->
    <aside
      v-if="!isMobile"
      ref="leftSidebarRef"
      class="flex flex-col h-[calc(100%-1rem)] glass gradient-border relative rounded-xl ml-2 mt-2 mb-2"
      :style="{ width: leftSidebarWidth + 'px', maxWidth: '80vw' }"
      data-tour="sidebar"
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
      class="fixed left-2 bottom-2 top-12 h-[calc(100dvh-3.5rem)] w-72 overflow-hidden flex flex-col z-50 rounded-xl shadow-2xl"
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
      class="resizer w-1 cursor-col-resize bg-base-content/5 hover:bg-base-content/10 transition-all z-40 flex-shrink-0 group flex items-center justify-center"
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
import { apiClient } from '@/api/client';

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
const DEFAULT_SIDEBAR_WIDTH = 460;
const leftSidebarWidth = ref(DEFAULT_SIDEBAR_WIDTH);
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
  leftSidebarWidth.value = Math.max(133, Math.min(startWidth.value + delta, window.innerWidth * 0.8));
}

async function stopResize() {
  isResizing.value = false;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);

  // Save width to database
  try {
    await apiClient.updateSetting('ui.sidebar_width', leftSidebarWidth.value.toString());
  } catch (e) {
    console.error('Failed to save sidebar width:', e);
  }
}

// Load saved width from database
async function loadSidebarWidth() {
  try {
    const settings = await apiClient.getSettings(['ui.sidebar_width']);
    const saved = settings.find((s) => s.key === 'ui.sidebar_width');
    if (saved) {
      const width = parseInt(saved.value, 10);
      if (!isNaN(width) && width >= 133 && width <= window.innerWidth * 0.8) {
        leftSidebarWidth.value = width;
      }
    }
  } catch (e) {
    // Silent fail, use default
    console.debug('Failed to load sidebar width, using default');
  }
}

// Ref for ClusterTreeNavigator
const clusterTreeNavigatorRef = ref<InstanceType<typeof ClusterTreeNavigator>>();

// Expose methods to parent
defineExpose({
  clusterTreeNavigatorRef
});

// Load saved width on mount
onMounted(() => {
  if (!props.isMobile) {
    loadSidebarWidth();
  }
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
