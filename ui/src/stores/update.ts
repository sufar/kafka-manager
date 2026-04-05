import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useUpdateStore = defineStore('update', () => {
  // 下载状态 - 使用 localStorage 持久化
  const downloading = ref(false);
  const downloadProgress = ref(0);
  const downloadFilename = ref('');

  // 从 localStorage 加载状态
  const loadState = () => {
    if (typeof window === 'undefined') return;
    const saved = localStorage.getItem('kafka-manager-update-state');
    if (saved) {
      try {
        const state = JSON.parse(saved);
        downloading.value = state.downloading || false;
        downloadProgress.value = state.downloadProgress || 0;
        downloadFilename.value = state.downloadFilename || '';
      } catch (e) {
        console.error('Failed to load update state:', e);
      }
    }
  };

  // 保存状态到 localStorage
  const saveState = () => {
    if (typeof window === 'undefined') return;
    localStorage.setItem('kafka-manager-update-state', JSON.stringify({
      downloading: downloading.value,
      downloadProgress: downloadProgress.value,
      downloadFilename: downloadFilename.value,
    }));
  };

  // 设置下载状态
  const setDownloading = (isDownloading: boolean, progress = 0, filename = '') => {
    downloading.value = isDownloading;
    downloadProgress.value = progress;
    downloadFilename.value = filename;
    saveState();
  };

  // 更新下载进度
  const updateProgress = (progress: number) => {
    downloadProgress.value = progress;
    saveState();
  };

  // 清除下载状态（下载完成或失败后）
  const clearState = () => {
    downloading.value = false;
    downloadProgress.value = 0;
    downloadFilename.value = '';
    saveState();
    if (typeof window !== 'undefined') {
      localStorage.removeItem('kafka-manager-update-state');
    }
  };

  return {
    downloading,
    downloadProgress,
    downloadFilename,
    loadState,
    setDownloading,
    updateProgress,
    clearState,
  };
});
