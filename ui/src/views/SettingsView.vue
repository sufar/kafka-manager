<template>
  <div class="p-3 relative overflow-hidden">
    <!-- Animated background -->
    <div class="absolute inset-0 overflow-hidden pointer-events-none">
      <div class="particle particle-1"></div>
      <div class="particle particle-2"></div>
    </div>

    <!-- Page Header -->
    <div class="mb-4 relative">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-xl font-bold text-gradient flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 animate-float">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281Z" />
              <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
            </svg>
            {{ t.settings.title }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">{{ t.settings.description }}</p>
        </div>
      </div>
    </div>

    <!-- Settings Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- Language Setting -->
      <div class="card glass gradient-border hover:glow-primary transition-all duration-300">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center glow-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="m10.5 21 5.25-13.5m0 0L10.5 21m0 0 5.25 5.25M15.75 7.5H2.25a.75.75 0 0 1-.75-.75v-1.5h15v1.5a.75.75 0 0 1-.75.75Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M15 19.5H2.25a.75.75 0 0 1-.75-.75v-1.5h15v1.5a.75.75 0 0 1-.75.75Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M21.75 10.5v3a.75.75 0 0 1-.75.75h-1.5a.75.75 0 0 1-.75-.75v-3a.75.75 0 0 1 .75-.75h1.5a.75.75 0 0 1 .75.75Z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient">{{ t.settings.language }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.selectLanguage }}</p>
            </div>
          </div>
          <LanguageSelector />
        </div>
      </div>

      <!-- Theme Setting -->
      <div class="card glass gradient-border hover:glow-secondary transition-all duration-300">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center glow-secondary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-secondary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient-warm">{{ t.settings.theme }}</h2>
              <p class="text-xs text-base-content/60">Toggle light or dark mode</p>
            </div>
          </div>
          <div class="flex items-center justify-between p-3 rounded-xl bg-base-100/50">
            <span class="text-sm font-medium">{{ isDark ? 'Dark Mode' : 'Light Mode' }}</span>
            <button class="btn btn-toggle relative overflow-hidden" @click="handleToggleTheme">
              <input type="checkbox" :checked="isDark" class="toggle" />
            </button>
          </div>
        </div>
      </div>

      <!-- Message View Mode Setting -->
      <div class="card glass gradient-border hover:glow-primary transition-all duration-300">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center glow-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient">{{ t.settings.messageViewMode }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.selectMessageViewMode }}</p>
            </div>
          </div>
          <div class="flex flex-col gap-2 p-3 rounded-xl bg-base-100/50">
            <button
              class="btn btn-sm"
              :class="{ 'btn-active btn-primary': messageViewMode === 'classic', 'btn-ghost': messageViewMode !== 'classic' }"
              @click="setMessageViewMode('classic')"
            >
              <span class="font-medium">{{ t.settings.classicMode }}</span>
            </button>
            <p class="text-xs text-base-content/50 px-1">{{ t.settings.classicModeDesc }}</p>
            <div class="divider my-1"></div>
            <button
              class="btn btn-sm"
              :class="{ 'btn-active btn-primary': messageViewMode === 'simple', 'btn-ghost': messageViewMode !== 'simple' }"
              @click="setMessageViewMode('simple')"
            >
              <span class="font-medium">{{ t.settings.simpleMode }}</span>
            </button>
            <p class="text-xs text-base-content/50 px-1">{{ t.settings.simpleModeDesc }}</p>
          </div>
        </div>
      </div>

      <!-- Sidebar Mode Setting -->
      <div class="card glass gradient-border hover:glow-accent transition-all duration-300">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-accent/10 flex items-center justify-center glow-accent">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-accent">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient">{{ t.settings.sidebarMode }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.selectSidebarMode }}</p>
            </div>
          </div>
          <div class="flex flex-col gap-2 p-3 rounded-xl bg-base-100/50">
            <button
              class="btn btn-sm"
              :class="{ 'btn-active btn-primary': sidebarMode === 'tree', 'btn-ghost': sidebarMode !== 'tree' }"
              @click="setSidebarMode('tree')"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
              </svg>
              <span class="font-medium">{{ t.settings.treeMode }}</span>
            </button>
            <p class="text-xs text-base-content/50 px-1">{{ t.settings.treeModeDesc }}</p>
            <div class="divider my-1"></div>
            <button
              class="btn btn-sm"
              :class="{ 'btn-active btn-primary': sidebarMode === 'flat', 'btn-ghost': sidebarMode !== 'flat' }"
              @click="setSidebarMode('flat')"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 6.75h12M8.25 12h12m-12 5.25h12M3.75 6.75h.007v.008H3.75V6.75zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zM3.75 12h.007v.008H3.75V12zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm-.375 5.25h.007v.008H3.75v-.008zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
              </svg>
              <span class="font-medium">{{ t.settings.flatMode }}</span>
            </button>
            <p class="text-xs text-base-content/50 px-1">{{ t.settings.flatModeDesc }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { useThemeStore } from '@/stores/theme';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import LanguageSelector from '@/components/Settings/LanguageSelector.vue';

const themeStore = useThemeStore();
const languageStore = useLanguageStore();

const { isDark, toggleTheme } = themeStore;
const { t } = storeToRefs(languageStore);

// Message view mode state
const messageViewMode = ref<'classic' | 'simple'>('simple');

// Sidebar mode state
const sidebarMode = ref<'tree' | 'flat'>('flat');

// Load message view mode setting
async function loadMessageViewModeSetting() {
  try {
    const settings = await apiClient.getSettings(['ui.message_view_mode']);
    const mode = settings.find((s: { key: string; value: string }) => s.key === 'ui.message_view_mode')?.value;
    if (mode === 'classic' || mode === 'simple') {
      messageViewMode.value = mode;
    }
  } catch (e) {
    console.error('Failed to load message view mode setting:', e);
  }
}

// Set message view mode
async function setMessageViewMode(mode: 'classic' | 'simple') {
  messageViewMode.value = mode;
  try {
    await apiClient.updateSetting('ui.message_view_mode', mode);
  } catch (e) {
    console.error('Failed to save message view mode setting:', e);
  }
}

// Load sidebar mode setting
async function loadSidebarModeSetting() {
  try {
    const settings = await apiClient.getSettings(['ui.sidebar_mode']);
    const mode = settings.find((s: { key: string; value: string }) => s.key === 'ui.sidebar_mode')?.value;
    if (mode === 'tree' || mode === 'flat') {
      sidebarMode.value = mode;
    }
  } catch (e) {
    console.error('Failed to load sidebar mode setting:', e);
  }
}

// Set sidebar mode
async function setSidebarMode(mode: 'tree' | 'flat') {
  sidebarMode.value = mode;
  try {
    await apiClient.updateSetting('ui.sidebar_mode', mode);
    // 触发设置变化事件通知其他组件
    window.dispatchEvent(new CustomEvent('settings-changed', {
      detail: { key: 'ui.sidebar_mode', value: mode }
    }));
  } catch (e) {
    console.error('Failed to save sidebar mode setting:', e);
  }
}

function handleToggleTheme() {
  toggleTheme();
}

onMounted(() => {
  loadMessageViewModeSetting();
  loadSidebarModeSetting();
});
</script>
