import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useUpdateStore = defineStore('update', () => {
  const downloading = ref(false);
  const downloadProgress = ref(0);

  const setDownloading = (isDownloading: boolean, progress = 0) => {
    downloading.value = isDownloading;
    downloadProgress.value = progress;
  };

  const updateProgress = (progress: number) => {
    downloadProgress.value = progress;
  };

  const clearState = () => {
    downloading.value = false;
    downloadProgress.value = 0;
  };

  return {
    downloading,
    downloadProgress,
    setDownloading,
    updateProgress,
    clearState,
  };
});
