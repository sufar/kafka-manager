<template>
  <div class="flex flex-col h-full overflow-hidden">
    <div class="p-3 relative flex-1 flex flex-col min-h-0">
    <!-- Animated background -->
    <div class="absolute inset-0 pointer-events-none">
      <div class="particle particle-1"></div>
      <div class="particle particle-2"></div>
    </div>

    <!-- Page Header -->
    <div class="mb-4 relative flex-shrink-0">
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

    <!-- Content Area (scrollable) -->
    <div class="flex-1 overflow-auto min-h-0">
      <!-- Settings Cards -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 pb-4">
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
              <p class="text-xs text-base-content/60">{{ t.settings.themeDesc }}</p>
            </div>
          </div>
          <div class="flex items-center justify-between p-3 rounded-xl bg-base-100/50">
            <span class="text-sm font-medium">{{ isDark ? t.settings.darkMode : t.settings.lightMode }}</span>
            <button class="btn btn-toggle relative overflow-hidden" @click="handleToggleTheme">
              <input type="checkbox" :checked="isDark" class="toggle" />
            </button>
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

      <!-- Version Info -->
      <div class="card glass gradient-border hover:glow-primary transition-all duration-300">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center glow-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient">{{ t.settings.version }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.versionDesc }}</p>
            </div>
          </div>
          <div class="p-3 rounded-xl bg-base-100/50 flex items-center justify-between">
            <span class="text-sm font-medium">{{ t.settings.currentVersion }}</span>
            <div class="flex items-center gap-2">
              <span class="badge badge-primary font-mono">{{ appVersion }}</span>
              <button
                v-if="updateAvailable"
                class="btn btn-xs btn-warning text-white"
                @click="showUpdateModal = true"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M11.42 15.17 17.25 20A2.653 2.653 0 0 0 21 17.25l-4.662-4.662a7.467 7.467 0 0 0-.75-5.933m-6.75 5.511L12 15m0 0V3.75m0 11.25-2.822-2.822m0 0a7.467 7.467 0 0 1 5.933-.75m-5.933.75L5.25 15m0 0V3.75m0 11.25a2.653 2.653 0 0 1-3.75-3.75L6.127 6.375" />
                </svg>
                新版本 {{ updateInfo?.version }}
              </button>
            </div>
          </div>
          <div class="p-3 rounded-xl bg-base-100/50 flex items-center justify-between mt-2">
            <span class="text-sm font-medium">{{ t.settings.author }}</span>
            <span class="text-sm font-medium">朱占全</span>
          </div>
          <div class="p-3 rounded-xl bg-base-100/50 flex items-center justify-between mt-2">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium">检查更新</span>
              <span v-if="checking" class="loading loading-spinner loading-xs"></span>
              <span v-if="updateAvailable" class="text-warning text-xs">新版本 {{ updateInfo?.version }}</span>
            </div>
            <button class="btn btn-sm" @click="checkForUpdates(true)">
              {{ checking ? '检查中...' : '立即检查' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Update Modal -->
      <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle" :class="{ 'modal-open': showUpdateModal }">
        <div class="modal-box">
          <h3 class="font-bold text-lg mb-4">
            {{ t.update.available }}
          </h3>

          <div class="flex items-center gap-3 mb-4">
            <div class="w-12 h-12 rounded-lg bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center flex-shrink-0 glow-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M15.59 14.37a6 6 0 0 1-5.84 7.38c-4.38 0-7.21-4.79-5.43-8.96a7.46 7.46 0 0 1 3.43-3.69l.12-.06a6 6 0 1 1 7.72 5.33Zm-5.84-5.12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm6.54 6.04a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="m15 12-3.5-3.5m0 0L8 12m3.5-3.5v9" />
              </svg>
            </div>
            <div>
              <div class="text-sm text-base-content/60">{{ t.update.currentVersion }}</div>
              <div class="font-mono font-bold">{{ appVersion }}</div>
            </div>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-base-content/40">
              <path stroke-linecap="round" stroke-linejoin="round" d="M17 8.25 21 12m0 0-4 3.75M21 12H3" />
            </svg>
            <div>
              <div class="text-sm text-base-content/60">{{ t.update.newVersion }}</div>
              <div class="font-mono font-bold text-primary">{{ updateInfo?.version }}</div>
            </div>
          </div>

          <div v-if="updateInfo?.notes" class="mb-4 p-3 bg-base-200 rounded-lg">
            <h4 class="font-semibold text-sm mb-2">{{ t.update.releaseNotes }}</h4>
            <div class="text-sm whitespace-pre-wrap">{{ updateInfo.notes }}</div>
          </div>

          <div v-if="downloading" class="mb-4">
            <div class="flex justify-between text-xs mb-1">
              <span>{{ t.update.downloading }}</span>
              <span>{{ downloadProgress }}%</span>
            </div>
            <progress class="progress progress-primary w-full" :value="downloadProgress" max="100"></progress>
          </div>

          <div class="modal-action">
            <form method="dialog" class="flex gap-2 w-full">
              <button
                type="button"
                class="btn btn-ghost flex-1"
                :disabled="downloading"
                @click="showUpdateModal = false"
              >
                {{ t.common.cancel }}
              </button>
              <button
                type="button"
                class="btn btn-primary flex-1"
                :disabled="downloading"
                @click="installUpdate"
              >
                {{ downloading ? t.update.downloading : t.update.updateAndRestart }}
              </button>
            </form>
          </div>
        </div>

        <form method="dialog" class="modal-backdrop">
          <button @click="showUpdateModal = false">close</button>
        </form>
      </dialog>

      <!-- JSON Highlight Setting -->
      <div class="md:col-span-2">
        <JsonHighlightSelector />
      </div>
    </div>
    </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useThemeStore } from '@/stores/theme';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import { apiClient } from '@/api/client';
import { invoke } from '@tauri-apps/api/core';
import LanguageSelector from '@/components/Settings/LanguageSelector.vue';
import JsonHighlightSelector from '@/components/Settings/JsonHighlightSelector.vue';

const themeStore = useThemeStore();
const languageStore = useLanguageStore();
const toast = useToast();

const { isDark, toggleTheme } = themeStore;
const { t } = storeToRefs(languageStore);

// App version
const appVersion = ref('1.0.8');

// 更新相关状态
const updateAvailable = ref(false);
const updateInfo = ref<{ version: string; notes: string } | null>(null);
const showUpdateModal = ref(false);
const checking = ref(false);
const downloading = ref(false);
const installing = ref(false);
const downloadProgress = ref(0);

// Modal ref
const modalRef = ref<HTMLDialogElement>();

// 监听 modal 打开/关闭
watch(showUpdateModal, (newVal) => {
  if (modalRef.value) {
    if (newVal) {
      modalRef.value.showModal();
    } else {
      modalRef.value.close();
    }
  }
});

// 获取当前版本
async function getCurrentVersion() {
  try {
    appVersion.value = await invoke<string>('get_app_version');
  } catch (e) {
    console.error('Failed to get current version:', e);
  }
}

// 检查更新
async function checkForUpdates(manual = false) {
  if (checking.value) return;

  checking.value = true;
  try {
    const result = await invoke<{
      available: boolean;
      version: string;
      notes: string;
      date: string;
    }>('check_for_updates');

    if (result.available) {
      updateAvailable.value = true;
      updateInfo.value = {
        version: result.version,
        notes: result.notes || ''
      };
      if (manual) {
        showUpdateModal.value = true;
      }
    } else {
      if (manual) {
        // 使用 Toast 显示成功提示，替代系统 alert
        toast.showInfo(t.value.update.checkCompleteNoUpdate || '已是最新版本', 3000);
      }
    }
  } catch (e) {
    console.error('Failed to check for updates:', e);
    console.error('Error type:', typeof e);
    console.error('Error keys:', e ? Object.keys(e) : 'N/A');
    // 只在手动检查时显示错误提示
    if (manual) {
      const errorMsg = typeof e === 'string' ? e : (e as Error)?.message || '未知错误';
      toast.showError((t.value.update.checkFailed || '检查更新失败') + ': ' + errorMsg);
    }
    // 自动检查时静默失败，不弹出提示
  } finally {
    checking.value = false;
  }
}

// 安装更新
async function installUpdate() {
  if (downloading.value) return;

  downloading.value = true;
  downloadProgress.value = 0;

  try {
    // 模拟下载进度
    const progressInterval = setInterval(() => {
      downloadProgress.value = Math.min(downloadProgress.value + 10, 90);
    }, 500);

    await invoke('install_update');

    clearInterval(progressInterval);
    downloadProgress.value = 100;

    // 下载完成，重置状态并关闭 modal
    downloading.value = false;
    showUpdateModal.value = false;
    toast.showInfo(t.value.update.downloadComplete || '下载完成，正在打开安装包...', 5000);
  } catch (e) {
    downloading.value = false;
    console.error('Failed to install update:', e);
    const errorMsg = typeof e === 'string' ? e : (e as Error)?.message || '未知错误';
    toast.showError((t.value.update.installFailed || '安装失败') + ': ' + errorMsg);
  }
}

// Sidebar mode state
const sidebarMode = ref<'tree' | 'flat'>('flat');

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
  getCurrentVersion();
  loadSidebarModeSetting();
  // 延迟检查更新
  setTimeout(() => checkForUpdates(false), 2000);
});
</script>

