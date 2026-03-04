<template>
  <div class="form-control w-full">
    <label class="label">
      <span class="label-text font-medium">{{ t.settings.language }}</span>
      <span class="label-text-alt">{{ t.settings.selectLanguage }}</span>
    </label>
    <select
      :value="currentLanguage"
      class="select select-bordered"
      @change="handleLanguageChange"
    >
      <option value="zh">{{ t.settings.languageZh }}</option>
      <option value="en">{{ t.settings.languageEn }}</option>
    </select>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { useLanguageStore } from '@/stores/language';

const languageStore = useLanguageStore();
const { t, setLanguage } = languageStore;
const { currentLanguage } = storeToRefs(languageStore);

async function handleLanguageChange(event: Event) {
  const target = event.target as HTMLSelectElement;
  await setLanguage(target.value as 'zh' | 'en');
}

onMounted(() => {
  languageStore.initLanguage();
});
</script>
