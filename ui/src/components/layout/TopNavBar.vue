<template>
  <header class="fixed top-0 left-0 right-0 h-10 bg-base-100/80 border-b border-base-200 z-50 px-2 flex items-center justify-between">
    <div class="flex items-center gap-2">
      <!-- Mobile Menu Button -->
      <button
        v-if="isMobile"
        class="btn btn-ghost btn-circle btn-xs h-7 w-7 min-h-0 md:hidden"
        @click="$emit('toggle-sidebar')"
        title="Open menu"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
        </svg>
      </button>

      <!-- Logo and Brand -->
      <div class="flex items-center gap-1.5">
        <div class="w-6 h-6 rounded bg-primary flex items-center justify-center shadow-lg flex-shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-primary-content">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
          </svg>
        </div>
        <h1 class="text-base font-bold whitespace-nowrap">Kafka Manager</h1>
      </div>

      <!-- Topic Search - Desktop -->
      <div class="relative ml-2 hidden md:block">
        <input
          ref="searchInputRef"
          v-model="searchQuery"
          type="text"
          class="input input-bordered input-xs w-56 h-7"
          :placeholder="t.layout.searchPlaceholder"
          @focus="showSearchDropdown = true"
          @keydown="handleSearchKeydown"
        />

        <!-- Search Results Dropdown -->
        <div
          v-if="showSearchDropdown && (searchQuery || searchResults.length > 0)"
          class="absolute top-full left-0 mt-1 w-72 bg-base-100 border border-base-200 rounded-lg shadow-xl z-[100] max-h-96 overflow-y-auto"
        >
          <div v-if="searchLoading" class="p-4 text-center">
            <span class="loading loading-spinner loading-sm"></span>
          </div>
          <div v-else-if="searchError" class="p-4 text-error text-sm">
            {{ searchError }}
          </div>
          <div v-else-if="searchResults.length === 0 && searchQuery" class="p-4 text-sm text-base-content/60">
            {{ t.layout.noTopicsFound }}
          </div>
          <div v-else>
            <div
              v-for="(result, index) in filteredSearchResults"
              :key="`${result.cluster}-${result.topic}`"
              class="flex items-center justify-between p-3 hover:bg-base-200 cursor-pointer border-b border-base-200 last:border-0"
              :class="{ 'bg-base-200': selectedIndex === index }"
              @click="selectSearchResult(result)"
              @mouseenter="selectedIndex = index"
            >
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-secondary flex-shrink-0">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                  </svg>
                  <span class="font-mono text-sm truncate" :title="result.topic" v-html="highlightMatch(result.topic)"></span>
                </div>
              </div>
              <div class="badge badge-ghost badge-xs ml-2">
                {{ result.cluster }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Mobile Search Button -->
      <button
        v-if="isMobile"
        class="btn btn-ghost btn-circle btn-xs h-7 w-7 min-h-0 md:hidden ml-1"
        @click="$emit('open-mobile-search')"
        title="Search"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
        </svg>
      </button>
    </div>

    <div class="flex items-center gap-0.5 flex-shrink-0">
      <!-- Mobile More Menu -->
      <div v-if="isMobile" class="dropdown dropdown-end md:hidden">
        <button tabindex="0" class="btn btn-ghost btn-circle btn-xs h-7 w-7 min-h-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 6.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 12.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 18.75a.75.75 0 110-1.5.75.75 0 010 1.5z" />
          </svg>
        </button>
        <ul tabindex="0" class="dropdown-content menu menu-sm bg-base-100 rounded-box shadow-xl z-50 w-32 mt-2">
          <li>
            <a @click="$emit('toggle-language')">
              <span class="text-xs">{{ currentLanguage === 'zh' ? 'English' : '中文' }}</span>
            </a>
          </li>
          <li>
            <a @click="$emit('toggle-theme')">
              <span class="text-xs">{{ isDark ? 'Light' : 'Dark' }}</span>
            </a>
          </li>
          <li>
            <router-link to="/settings">
              <span class="text-xs">{{ t.layout.settings }}</span>
            </router-link>
          </li>
        </ul>
      </div>

      <!-- Desktop Controls -->
      <div class="hidden md:flex items-center gap-0.5">
        <!-- Language Toggle -->
        <button
          class="btn btn-ghost btn-circle btn-xs h-6 w-6 min-h-0"
          @click="$emit('toggle-language')"
          :title="`Toggle language (Current: ${currentLanguage === 'zh' ? '中文' : 'EN'})`"
        >
          <span class="text-[10px] font-bold">{{ currentLanguage === 'zh' ? 'EN' : '中' }}</span>
        </button>

        <!-- Theme Toggle -->
        <button
          class="btn btn-ghost btn-circle btn-xs h-6 w-6 min-h-0"
          @click="$emit('toggle-theme')"
          title="Toggle theme"
        >
          <svg v-if="!isDark" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" />
          </svg>
          <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" />
          </svg>
        </button>

        <!-- Settings Button -->
        <router-link to="/settings" class="btn btn-ghost btn-circle btn-xs h-6 w-6 min-h-0" :title="t.layout.settings">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 0 1 0-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281Z" />
            <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
          </svg>
        </router-link>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useRouter } from 'vue-router';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';

const router = useRouter();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const props = defineProps<{
  isMobile: boolean;
  isDark: boolean;
  sidebarMode: 'tree' | 'flat';
}>();

const emit = defineEmits<{
  'toggle-sidebar': [];
  'toggle-language': [];
  'toggle-theme': [];
  'open-mobile-search': [];
  'select-topic': [cluster: string, topic: string];
}>();

// ==================== Topic Search ====================
const searchQuery = ref('');
const searchResults = ref<{ cluster: string; topic: string }[]>([]);
const searchLoading = ref(false);
const searchError = ref<string | null>(null);
const showSearchDropdown = ref(false);
const selectedIndex = ref(-1);
const searchInputRef = ref<HTMLInputElement>();

// Debounced search
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchQuery, (newQuery) => {
  const trimmed = newQuery.trim();
  if (trimmed) {
    showSearchDropdown.value = true;
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
      performSearch();
    }, 300);
  } else {
    searchResults.value = [];
  }
});

async function performSearch() {
  const query = searchQuery.value.trim();
  if (!query) return;

  searchLoading.value = true;
  searchError.value = null;
  try {
    searchResults.value = await apiClient.searchTopics(query);
  } catch (e) {
    console.error('[search] error:', e);
    searchError.value = (e as { message: string }).message;
  } finally {
    searchLoading.value = false;
  }
}

const filteredSearchResults = computed(() => {
  return searchResults.value;
});

function highlightMatch(topic: string) {
  if (!searchQuery.value.trim()) return topic;
  const query = searchQuery.value.toLowerCase();
  const topicLower = topic.toLowerCase();
  const index = topicLower.indexOf(query);
  if (index === -1) return topic;
  const before = topic.slice(0, index);
  const match = topic.slice(index, index + query.length);
  const after = topic.slice(index + query.length);
  return `${before}<mark class="bg-warning text-warning-content rounded px-0.5">${match}</mark>${after}`;
}

function handleSearchKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, filteredSearchResults.value.length - 1);
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
  } else if (e.key === 'Enter' && selectedIndex.value >= 0) {
    e.preventDefault();
    const result = filteredSearchResults.value[selectedIndex.value];
    if (result) selectSearchResult(result);
  } else if (e.key === 'Escape') {
    showSearchDropdown.value = false;
    searchInputRef.value?.blur();
  }
}

async function selectSearchResult(result: { cluster: string; topic: string }) {
  showSearchDropdown.value = false;
  searchQuery.value = '';

  // 统一跳转到 messages 页面
  router.push({ path: '/messages', query: { cluster: result.cluster, topic: result.topic } });

  // 通知父组件高亮树形节点（如果是树形模式）
  if (props.sidebarMode === 'tree') {
    emit('select-topic', result.cluster, result.topic);
  }
}

// Ctrl+K 快捷键聚焦搜索框
function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
    e.preventDefault();
    searchInputRef.value?.focus();
    showSearchDropdown.value = true;
  }
}

// 暴露方法给父组件
defineExpose({
  handleGlobalKeydown
});

const currentLanguage = computed(() => languageStore.currentLanguage);
</script>
