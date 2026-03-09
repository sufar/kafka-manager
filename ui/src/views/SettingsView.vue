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
    </div>
  </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useThemeStore } from '@/stores/theme';
import { useLanguageStore } from '@/stores/language';
import LanguageSelector from '@/components/Settings/LanguageSelector.vue';

const themeStore = useThemeStore();
const languageStore = useLanguageStore();

const { isDark, toggleTheme } = themeStore;
const { t } = storeToRefs(languageStore);

function handleToggleTheme() {
  toggleTheme();
}
</script>
