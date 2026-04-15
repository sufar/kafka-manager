<template>
  <div class="p-3 overflow-auto min-h-full">
    <!-- Page Header -->
    <div class="mb-4">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2">
            <button class="btn btn-ghost btn-xs p-1 mr-2" @click="$router.back()" :title="t.favorites?.back || 'Back'">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
              </svg>
            </button>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-amber-400">
              <path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.563 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
            </svg>
            {{ t.favorites?.title || 'Topic 收藏' }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">
            {{ t.favorites?.description || '管理您收藏的 Topic，支持分组管理' }}
          </p>
        </div>
      </div>
    </div>

    <!-- Favorites Content -->
    <TopicFavorites :t="t" />
  </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useLanguageStore } from '@/stores/language';
import TopicFavorites from '@/components/TopicFavorites.vue';

const languageStore = useLanguageStore();
const { t } = storeToRefs(languageStore);
</script>
