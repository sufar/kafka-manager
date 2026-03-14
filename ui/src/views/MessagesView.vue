<template>
  <div class="messages-view h-full flex flex-col">
    <!-- 界面模式切换器 -->
    <div class="mode-switcher flex items-center justify-between px-3 py-2 border-b border-base-300 bg-base-200/50">
      <div class="flex items-center gap-2">
        <span class="text-xs text-base-content/60">消息界面:</span>
        <div class="btn-group btn-group-sm">
          <button
            class="btn btn-sm"
            :class="{ 'btn-active': viewMode === 'classic' }"
            @click="setViewMode('classic')"
          >
            经典模式
          </button>
          <button
            class="btn btn-sm"
            :class="{ 'btn-active': viewMode === 'simple' }"
            @click="setViewMode('simple')"
          >
            简洁模式
          </button>
        </div>
      </div>
      <div class="text-xs text-base-content/50">
        {{ viewMode === 'classic' ? '功能完整，适合复杂操作' : '轻量快速，适合日常查询' }}
      </div>
    </div>

    <!-- 根据模式显示不同界面 -->
    <template v-if="viewMode === 'simple'">
      <MessageQueryTool class="flex-1" />
    </template>
    <template v-else>
      <MessagesClassicView class="flex-1" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { apiClient } from '@/api/client';
import MessageQueryTool from '@/components/MessageQueryTool.vue';
import MessagesClassicView from './MessagesClassicView.vue';

// 界面模式: 'classic' | 'simple'
const viewMode = ref<'classic' | 'simple'>('classic');

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

// 保存设置
async function setViewMode(mode: 'classic' | 'simple') {
  viewMode.value = mode;
  try {
    await apiClient.updateSetting('ui.message_view_mode', mode);
  } catch (e) {
    console.error('Failed to save view mode setting:', e);
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

.mode-switcher {
  flex-shrink: 0;
}
</style>
