import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useUpdateStore = defineStore('update', () => {
  const downloading = ref(false);
  const downloadProgress = ref(0);
  const downloadDownloaded = ref(0);
  const downloadTotal = ref(0);
  const downloadSpeed = ref(0); // bytes per second
  const isAutoDownload = ref(false); // 是否为后台自动更新（非用户手动触发）

  const setDownloading = (isDownloading: boolean, progress = 0, downloaded = 0, total = 0, speed = 0) => {
    downloading.value = isDownloading;
    downloadProgress.value = progress;
    downloadDownloaded.value = downloaded;
    downloadTotal.value = total;
    downloadSpeed.value = speed;
  };

  const setAutoDownload = (isAuto: boolean) => {
    isAutoDownload.value = isAuto;
  };

  const updateProgress = (progress: number, downloaded = 0, total = 0, speed = 0) => {
    downloadProgress.value = progress;
    downloadDownloaded.value = downloaded;
    downloadTotal.value = total;
    downloadSpeed.value = speed;
  };

  const clearState = () => {
    downloading.value = false;
    downloadProgress.value = 0;
    downloadDownloaded.value = 0;
    downloadTotal.value = 0;
    downloadSpeed.value = 0;
    isAutoDownload.value = false;
  };

  return {
    downloading,
    downloadProgress,
    downloadDownloaded,
    downloadTotal,
    downloadSpeed,
    isAutoDownload,
    setDownloading,
    setAutoDownload,
    updateProgress,
    clearState,
  };
});
