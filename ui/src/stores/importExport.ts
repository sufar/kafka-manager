import { defineStore } from 'pinia';
import { ref, computed } from 'vue';

export const useImportExportStore = defineStore('importExport', () => {
  const importing = ref(false);
  const exporting = ref(false);
  const isBusy = computed(() => importing.value || exporting.value);

  return {
    importing,
    exporting,
    isBusy,
  };
});
