<template>
  <div class="topic-navigator h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between p-1.5 flex-shrink-0 border-b border-base-200">
      <div class="flex items-center gap-1.5">
        <div class="w-6 h-6 rounded bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-primary">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
          </svg>
        </div>
        <span class="text-xs font-bold text-base-content/60 uppercase tracking-wider">Topics</span>
      </div>
      <div class="flex items-center gap-0.5">
        <button
          class="btn btn-ghost btn-xs"
          @click="goToFavorites"
          title="Topic Favorites"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.563 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
          </svg>
        </button>
        <button
          class="btn btn-ghost btn-xs"
          @click="goToClusters"
          title="Manage Clusters"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 6h9.75M10.5 6a1.5 1.5 0 11-3 0m3 0a1.5 1.5 0 10-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-9.75 0h9.75" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Search Box -->
    <div class="px-1.5 py-1 flex-shrink-0">
      <div class="relative">
        <input
          v-model="searchQuery"
          type="text"
          class="input input-bordered input-sm w-full pr-8"
          placeholder="搜索 Topic..."
          @input="onSearchInput"
        />
        <svg
          v-if="!searchQuery"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          stroke-width="1.5"
          stroke="currentColor"
          class="w-4 h-4 absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40"
        >
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
        </svg>
        <button
          v-else
          class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content"
          @click="clearSearch"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Topic List with Virtual Scroll -->
    <div class="flex-1 overflow-hidden px-2">
      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center py-8">
        <span class="loading loading-spinner loading-sm"></span>
      </div>

      <!-- Empty -->
      <div v-else-if="filteredTopics.length === 0" class="text-center py-8 text-base-content/50">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 mx-auto mb-2 opacity-50">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
        <p class="text-xs">{{ searchQuery ? '无匹配结果' : '暂无 Topics' }}</p>
      </div>

      <!-- Virtual Scroll Topic Items -->
      <RecycleScroller
        v-else
        class="h-full overflow-auto"
        :items="searchQuery ? filteredTopicsWithUid : displayedTopicsWithUid"
        :item-size="28"
        key-field="uid"
        v-slot="{ item, index }"
        @scroll="handleScroll"
      >
        <div
          class="group flex items-center gap-1.5 px-1.5 py-1 rounded cursor-pointer transition-all duration-200 hover:bg-base-200"
          :class="{ 'bg-primary/10': hoveredIndex === index }"
          @click="selectTopic((item as TopicItem).topic)"
          @mouseenter="hoveredIndex = index"
        >
          <!-- Cluster Health Indicator -->
          <div
            class="w-1.5 h-1.5 rounded-full flex-shrink-0"
            :class="{
              'bg-success': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === true,
              'bg-error': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === false,
              'bg-warning': getClusterHealth((item as TopicItem).topic.cluster)?.healthy === undefined
            }"
          ></div>

          <!-- Topic Name with Tooltip -->
          <div class="flex-1 min-w-0 relative">
            <span
              class="text-xs truncate block"
              :title="`${(item as TopicItem).topic.name} (${(item as TopicItem).topic.cluster})`"
            >
              {{ (item as TopicItem).topic.name }}
            </span>
          </div>

          <!-- Cluster Badge -->
          <span class="badge badge-ghost badge-xs flex-shrink-0 truncate max-w-14 text-[10px] px-1">
            {{ (item as TopicItem).topic.cluster }}
          </span>
        </div>
      </RecycleScroller>
    </div>

    <!-- Status Bar -->
    <div class="p-1.5 text-xs text-base-content/50 border-t border-base-200 flex-shrink-0">
      <div class="flex items-center justify-between gap-2">
        <span class="truncate max-w-[50%]">{{ allTopics.length }} / {{ total }} topics</span>
        <div class="flex items-center gap-1 flex-shrink-0">
          <!-- Load More Button -->
          <button
            v-if="hasMore && allTopics.length < 10000"
            class="btn btn-ghost btn-xs text-primary"
            :disabled="loadingMore"
            @click="loadMoreTopics"
          >
            <span v-if="loadingMore" class="loading loading-spinner loading-xs"></span>
            加载更多
          </button>
          <span class="text-xs">Cluster:</span>
          <!-- Advanced Cluster Selector -->
          <div class="relative">
            <button
              ref="clusterSelectorButtonRef"
              class="btn btn-ghost btn-xs gap-1"
              @click="toggleClusterSelector"
              :title="getClusterSelectorSummary()"
            >
              <span class="truncate max-w-[120px]">{{ getClusterSelectorSummary() }}</span>
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3" :class="{ 'rotate-180': showClusterSelector }">
                <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" />
              </svg>
            </button>
            <!-- Cluster Selector Dropdown - Desktop -->
            <div
              v-show="showClusterSelector && !isMobile"
              ref="clusterSelectorRef"
              class="absolute bottom-full right-0 mb-1 w-[280px] sm:w-[320px] max-h-[400px] overflow-hidden rounded-lg bg-base-100 border border-base-200 shadow-xl z-[100]"
            >
              <div class="flex flex-col sm:flex-row h-[300px]">
                <!-- Left: Groups List -->
                <div class="w-full sm:w-1/2 border-b sm:border-b-0 sm:border-r border-base-200 overflow-y-auto">
                  <div class="p-2 border-b border-base-200 bg-base-100/50 sticky top-0">
                    <span class="text-[10px] font-medium text-base-content/60 uppercase">{{ t.navigator.groups }}</span>
                  </div>
                  <!-- All Clusters Option -->
                  <label
                    class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer border-b border-base-100"
                    :class="{ 'bg-primary/10': !hasCustomSelection }"
                  >
                    <input
                      type="radio"
                      name="clusterMode"
                      class="radio radio-xs radio-primary flex-shrink-0"
                      :checked="!hasCustomSelection"
                      @change="setSelectionMode('all')"
                    />
                    <span class="text-xs font-medium flex-1">{{ t.navigator.allClusters }}</span>
                  </label>
                  <!-- Groups -->
                  <div
                    v-for="group in clusterStore.groups"
                    :key="group.id"
                    class="border-b border-base-100"
                  >
                    <div
                      class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer"
                      :class="{ 'bg-primary/10': isGroupFullySelected(group.id), 'bg-base-200': activeGroupId === group.id }"
                    >
                      <input
                        type="checkbox"
                        class="checkbox checkbox-xs checkbox-primary flex-shrink-0 cursor-pointer"
                        :checked="isGroupFullySelected(group.id)"
                        @click.stop="toggleGroupFull(group.id)"
                      />
                      <span
                        class="text-xs font-medium flex-1 truncate cursor-pointer"
                        @click="activeGroupId = group.id"
                      >
                        {{ group.name }}
                      </span>
                    </div>
                  </div>
                </div>
                <!-- Right: Clusters List -->
                <div class="w-full sm:w-1/2 overflow-y-auto">
                  <div class="p-2 border-b border-base-200 bg-base-100/50 sticky top-0 flex items-center justify-between">
                    <span class="text-[10px] font-medium text-base-content/60 uppercase">{{ t.navigator.clusters }}</span>
                    <button
                      v-if="hasSelectedClustersInCurrentView"
                      class="text-[10px] text-primary hover:underline"
                      @click="deselectAllInCurrentView"
                    >
                      {{ t.navigator.deselectAll }}
                    </button>
                  </div>
                  <!-- Clusters for selected group or all clusters -->
                  <template v-if="activeGroupId === null || activeGroupId === 0">
                    <!-- Show all clusters when no group selected -->
                    <label
                      v-for="cluster in clusterStore.clusters"
                      :key="cluster.name"
                      class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer border-b border-base-100"
                    >
                      <input
                        type="checkbox"
                        class="checkbox checkbox-xs flex-shrink-0"
                        :checked="selectedClusters.has(cluster.name)"
                        @change.stop="toggleCluster(cluster.name, cluster.group_id)"
                      />
                      <span class="text-xs truncate flex-1">{{ cluster.name }}</span>
                    </label>
                  </template>
                  <template v-else>
                    <!-- Show clusters for selected group -->
                    <label
                      v-for="cluster in getClustersByGroup(activeGroupId)"
                      :key="cluster.name"
                      class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer border-b border-base-100"
                    >
                      <input
                        type="checkbox"
                        class="checkbox checkbox-xs flex-shrink-0"
                        :checked="selectedClusters.has(cluster.name)"
                        @change.stop="toggleCluster(cluster.name, getGroupId(cluster.group_id))"
                      />
                      <span class="text-xs truncate flex-1">{{ cluster.name }}</span>
                    </label>
                  </template>
                </div>
              </div>
              <!-- Action Buttons -->
              <div class="p-2 border-t border-base-200 flex gap-2">
                <button
                  class="btn btn-ghost btn-xs flex-1"
                  @click="clearAllSelections"
                >
                  {{ t.common.clear }}
                </button>
                <button
                  class="btn btn-primary btn-xs flex-1"
                  @click="applyClusterSelection"
                >
                  {{ t.common.apply }}
                </button>
              </div>
            </div>
            <!-- Cluster Selector Modal - Mobile -->
            <div
              v-show="showClusterSelector && isMobile"
              class="fixed inset-0 z-[200] flex items-end sm:items-center justify-center bg-black/50"
              @click="toggleClusterSelector"
            >
              <div
                class="w-full max-w-md max-h-[80vh] bg-base-100 rounded-t-xl sm:rounded-xl overflow-hidden"
                @click.stop
              >
                <div class="flex flex-col h-[60vh] sm:h-[400px]">
                  <div class="p-3 border-b border-base-200 flex items-center justify-between">
                    <span class="text-sm font-semibold">{{ t.navigator.selectClusters }}</span>
                    <button class="btn btn-ghost btn-sm btn-circle" @click="toggleClusterSelector">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </div>
                  <div class="flex flex-1 overflow-hidden">
                    <!-- Left: Groups List -->
                    <div class="w-1/2 border-r border-base-200 overflow-y-auto">
                      <div class="p-2 border-b border-base-200 bg-base-100/50">
                        <span class="text-[10px] font-medium text-base-content/60 uppercase">{{ t.navigator.groups }}</span>
                      </div>
                      <!-- All Clusters Option -->
                      <label
                        class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer border-b border-base-100"
                        :class="{ 'bg-primary/10': !hasCustomSelection }"
                      >
                        <input
                          type="radio"
                          name="clusterModeMobile"
                          class="radio radio-sm radio-primary flex-shrink-0"
                          :checked="!hasCustomSelection"
                          @change="setSelectionMode('all')"
                        />
                        <span class="text-xs font-medium flex-1">{{ t.navigator.allClusters }}</span>
                      </label>
                      <!-- Groups -->
                      <div
                        v-for="group in clusterStore.groups"
                        :key="group.id"
                        class="border-b border-base-100"
                      >
                        <div
                          class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer"
                          :class="{ 'bg-primary/10': isGroupFullySelected(group.id) }"
                        >
                          <input
                            type="checkbox"
                            class="checkbox checkbox-sm checkbox-primary flex-shrink-0 cursor-pointer"
                            :checked="isGroupFullySelected(group.id)"
                            @click.stop="toggleGroupFull(group.id)"
                          />
                          <span
                            class="text-xs font-medium flex-1 truncate cursor-pointer"
                            @click="activeGroupId = group.id"
                          >
                            {{ group.name }}
                          </span>
                        </div>
                      </div>
                    </div>
                    <!-- Right: Clusters List -->
                    <div class="w-1/2 overflow-y-auto">
                      <div class="p-2 border-b border-base-200 bg-base-100/50">
                        <span class="text-[10px] font-medium text-base-content/60 uppercase">{{ t.navigator.clusters }}</span>
                      </div>
                      <template v-if="activeGroupId === null || activeGroupId === 0">
                        <label
                          v-for="cluster in clusterStore.clusters"
                          :key="cluster.name"
                          class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer border-b border-base-100"
                        >
                          <input
                            type="checkbox"
                            class="checkbox checkbox-sm flex-shrink-0"
                            :checked="selectedClusters.has(cluster.name)"
                            @change.stop="toggleCluster(cluster.name, cluster.group_id)"
                          />
                          <span class="text-xs truncate flex-1">{{ cluster.name }}</span>
                        </label>
                      </template>
                      <template v-else>
                        <label
                          v-for="cluster in getClustersByGroup(activeGroupId)"
                          :key="cluster.name"
                          class="flex items-center gap-2 p-2 hover:bg-base-200 cursor-pointer border-b border-base-100"
                        >
                          <input
                            type="checkbox"
                            class="checkbox checkbox-sm flex-shrink-0"
                            :checked="selectedClusters.has(cluster.name)"
                            @change.stop="toggleCluster(cluster.name, getGroupId(cluster.group_id))"
                          />
                          <span class="text-xs truncate flex-1">{{ cluster.name }}</span>
                        </label>
                      </template>
                    </div>
                  </div>
                  <!-- Action Buttons -->
                  <div class="p-3 border-t border-base-200 flex gap-2">
                    <button
                      class="btn btn-ghost btn-sm flex-1"
                      @click="clearAllSelections"
                    >
                      {{ t.common.clear }}
                    </button>
                    <button
                      class="btn btn-primary btn-sm flex-1"
                      @click="applyClusterSelection"
                    >
                      {{ t.common.apply }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <!-- Refresh Button -->
          <button
            class="btn btn-ghost btn-xs"
            :disabled="refreshing"
            @click="refreshTopics"
            title="刷新 Topics"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="w-3.5 h-3.5"
              :class="{ 'animate-spin': refreshing }"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted, onMounted } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import { useRoute } from 'vue-router';
import { apiClient } from '@/api/client';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';

interface TopicInfo {
  name: string;
  cluster: string;
}

interface TopicItem {
  topic: TopicInfo;
  uid: string;
}

const emit = defineEmits<{
  navigate: [{ path: string; query?: Record<string, string> }];
  update: [];
}>();

const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const route = useRoute();
const t = computed(() => languageStore.t);

// State
const searchQuery = ref('');
const allTopics = ref<TopicInfo[]>([]);
const loading = ref(false);
const refreshing = ref(false);
const loadingMore = ref(false);
const selectedTopic = ref<TopicInfo | null>(null);
const isUnmounted = ref(false);

// Advanced cluster selection state
const showClusterSelector = ref(false);
const clusterSelectorButtonRef = ref<HTMLElement | null>(null);
const clusterSelectorRef = ref<HTMLElement | null>(null);
const activeGroupId = ref<number | null>(null); // For tracking selected group in right panel

// Mobile detection
const isMobile = ref(window.innerWidth < 640);
function updateMobileState() {
  isMobile.value = window.innerWidth < 640;
}

// Selected clusters (empty = all clusters mode)
const selectedClusters = ref<Set<string>>(new Set());
const selectedGroups = ref<Set<number>>(new Set()); // Groups that are fully selected

// Load saved cluster selection from settings
let hasLoadedClusterSelection = false;
async function loadSavedClusterSelection() {
  if (hasLoadedClusterSelection) return;
  try {
    const settings = await apiClient.getSettings(['ui.selected_clusters']);
    const setting = settings.find((s: { key: string; value: string }) => s.key === 'ui.selected_clusters');
    if (setting && setting.value) {
      try {
        const saved = JSON.parse(setting.value);
        if (saved.clusters && Array.isArray(saved.clusters)) {
          selectedClusters.value = new Set(saved.clusters);
        }
        if (saved.groups && Array.isArray(saved.groups)) {
          selectedGroups.value = new Set(saved.groups);
        }
        hasLoadedClusterSelection = true;
        return;
      } catch (e) {
        console.warn('Failed to parse saved cluster selection:', e);
      }
    }
    hasLoadedClusterSelection = true;
  } catch (e) {
    console.error('Failed to load cluster selection:', e);
    hasLoadedClusterSelection = true;
  }
}

// Watch for cluster list changes and restore selection
watch(() => clusterStore.clusters, (newClusters) => {
  if (!hasLoadedClusterSelection && newClusters.length > 0) {
    loadSavedClusterSelection();
  }
  // Validate selections
  const validClusterNames = newClusters.map(c => c.name);
  selectedClusters.value = new Set([...selectedClusters.value].filter(name => validClusterNames.includes(name)));

  // Update selectedGroups based on current cluster selection
  updateSelectedGroups();
}, { deep: true });

onMounted(() => {
  if (clusterStore.clusters.length > 0) {
    loadSavedClusterSelection();
  }
  // Load groups if not already loaded
  if (clusterStore.groups.length === 0) {
    clusterStore.fetchGroups();
  }
  document.addEventListener('click', handleOutsideClick);
  window.addEventListener('resize', updateMobileState);
  document.addEventListener('keydown', handleKeydown);
  updateMobileState();
});

onUnmounted(() => {
  isUnmounted.value = true;
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  apiClient.cancelRequest();
  document.removeEventListener('click', handleOutsideClick);
  document.removeEventListener('keydown', handleKeydown);
  window.removeEventListener('resize', updateMobileState);
});

function handleOutsideClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (showClusterSelector.value &&
      !clusterSelectorRef.value?.contains(target) &&
      !clusterSelectorButtonRef.value?.contains(target)) {
    showClusterSelector.value = false;
  }
}

function toggleClusterSelector() {
  showClusterSelector.value = !showClusterSelector.value;
  activeGroupId.value = null;
}

// Set selection mode to 'all' (clear all selections)
function setSelectionMode(mode: 'all') {
  if (mode === 'all') {
    selectedClusters.value.clear();
    selectedGroups.value.clear();
    activeGroupId.value = null;
  }
}

// Check if group is fully selected (all clusters in group are selected)
function isGroupFullySelected(groupId: number): boolean {
  const groupClusters = getClustersByGroup(groupId);
  if (groupClusters.length === 0) return false;
  return groupClusters.every(c => selectedClusters.value.has(c.name));
}

// Toggle all clusters in a group
function toggleGroupFull(groupId: number) {
  if (isGroupFullySelected(groupId)) {
    // Deselect all clusters in this group
    const groupClusters = getClustersByGroup(groupId);
    groupClusters.forEach(c => selectedClusters.value.delete(c.name));
    selectedGroups.value.delete(groupId);
  } else {
    // Select all clusters in this group
    const groupClusters = getClustersByGroup(groupId);
    groupClusters.forEach(c => selectedClusters.value.add(c.name));
    selectedGroups.value.add(groupId);
  }
}

// Toggle single cluster
function toggleCluster(clusterName: string, groupId: number | null | undefined) {
  if (selectedClusters.value.has(clusterName)) {
    selectedClusters.value.delete(clusterName);
    // If cluster is removed from a group, remove group from selectedGroups
    if (groupId) {
      selectedGroups.value.delete(groupId);
    }
  } else {
    selectedClusters.value.add(clusterName);
  }
  // Update group selection status
  updateSelectedGroups();
}

// Update selectedGroups based on current cluster selection
function updateSelectedGroups() {
  selectedGroups.value.clear();
  for (const group of clusterStore.groups) {
    if (isGroupFullySelected(group.id)) {
      selectedGroups.value.add(group.id);
    }
  }
}

function getClustersByGroup(groupId: number) {
  return clusterStore.clusters.filter(c => (c.group_id ?? 0) === groupId);
}

function getGroupId(groupId: number | null | undefined): number {
  return groupId ?? 0;
}

// Check if there's any custom selection
const hasCustomSelection = computed(() => selectedClusters.value.size > 0);

// Check if there are selected clusters in current view
const hasSelectedClustersInCurrentView = computed(() => {
  if (activeGroupId.value === null) {
    return selectedClusters.value.size > 0;
  }
  const groupClusters = getClustersByGroup(activeGroupId.value);
  return groupClusters.some(c => selectedClusters.value.has(c.name));
});

// Deselect all in current view
function deselectAllInCurrentView() {
  if (activeGroupId.value === null) {
    selectedClusters.value.clear();
  } else {
    const groupClusters = getClustersByGroup(activeGroupId.value);
    groupClusters.forEach(c => selectedClusters.value.delete(c.name));
    selectedGroups.value.delete(activeGroupId.value);
  }
  updateSelectedGroups();
}

function getAllSelectedClusterNames(): string[] {
  // Empty selection = all clusters
  if (selectedClusters.value.size === 0) {
    return clusterStore.clusters.map(c => c.name);
  }
  return [...selectedClusters.value];
}

function getClusterSelectorSummary(): string {
  if (selectedClusters.value.size === 0) {
    return t.value.navigator.allClusters;
  }
  const count = selectedClusters.value.size;
  if (count === 1) {
    const first = [...selectedClusters.value][0];
    return first || '';
  }
  return `${count} clusters`;
}

function clearAllSelections() {
  selectedClusters.value.clear();
  selectedGroups.value.clear();
  applyClusterSelection();
}

async function applyClusterSelection() {
  showClusterSelector.value = false;
  searchQuery.value = '';
  offset.value = 0;
  await saveClusterSelection();
  await loadAllTopics();
}

async function saveClusterSelection() {
  try {
    const selection = {
      clusters: [...selectedClusters.value],
      groups: [...selectedGroups.value],
    };
    await apiClient.updateSetting('ui.selected_clusters', JSON.stringify(selection));
  } catch (e) {
    console.error('Failed to save cluster selection:', e);
  }
}

// Pagination state
const offset = ref(0);
const limit = ref(10000);
const total = ref(0);
const hasMore = ref(false);

// Scroll state
let scrollLock = false; // Prevent multiple simultaneous loads

// Debounce timer
let searchTimer: number | null = null;

// Cleanup on unmount to prevent blocking navigation
onUnmounted(() => {
  isUnmounted.value = true;
  if (searchTimer) {
    clearTimeout(searchTimer);
  }
  // Cancel any pending API requests
  apiClient.cancelRequest();
});

// 判断是否使用后端搜索（当 topic 总数超过 10000 时使用后端搜索）
const useBackendSearch = computed(() => total.value > 10000);

// Filtered topics - search only topic name (no cluster search)
// 当使用后端搜索时，filteredTopics 直接使用 allTopics（因为后端已经过滤）
const filteredTopics = computed(() => {
  if (!searchQuery.value.trim()) {
    return allTopics.value;
  }
  // 如果使用后端搜索，后端已经过滤了，直接返回
  if (useBackendSearch.value) {
    return allTopics.value;
  }
  // 否则使用前端过滤
  const query = searchQuery.value.toLowerCase();
  return allTopics.value.filter(
    t => t.name.toLowerCase().includes(query)
  );
});

// Topics with uid for virtual scroll (use filtered for search results)
const filteredTopicsWithUid = computed((): TopicItem[] => {
  return filteredTopics.value.map(topic => ({
    topic,
    uid: `${topic.cluster}-${topic.name}`
  }));
});

// Displayed topics - for pagination display
const displayedTopics = computed(() => {
  return allTopics.value;
});

// Displayed topics with uid for virtual scroll
const displayedTopicsWithUid = computed((): TopicItem[] => {
  return displayedTopics.value.map(topic => ({
    topic,
    uid: `${topic.cluster}-${topic.name}`
  }));
});

// Get cluster health
function getClusterHealth(clusterName: string) {
  return clusterStore.clusterHealth[clusterName];
}

// Track pending highlight for after topics load
const pendingHighlight = ref<{ cluster: string; topic: string } | null>(null);

// Keyboard navigation
const hoveredIndex = ref<number>(-1);

// Get current visible topics list
const visibleTopics = computed(() => {
  return searchQuery.value.trim() ? filteredTopics.value : allTopics.value;
});

// Navigate to previous topic
function navigateUp() {
  if (visibleTopics.value.length === 0) return;
  if (hoveredIndex.value <= 0) {
    hoveredIndex.value = visibleTopics.value.length - 1;
  } else {
    hoveredIndex.value--;
  }
  updateSelectedFromHover();
}

// Navigate to next topic
function navigateDown() {
  if (visibleTopics.value.length === 0) return;
  if (hoveredIndex.value >= visibleTopics.value.length - 1) {
    hoveredIndex.value = 0;
  } else {
    hoveredIndex.value++;
  }
  updateSelectedFromHover();
}

// Update selected topic based on hover index
function updateSelectedFromHover() {
  if (hoveredIndex.value < 0 || hoveredIndex.value >= visibleTopics.value.length) return;
  const topic = visibleTopics.value[hoveredIndex.value];
  if (topic) {
    selectedTopic.value = topic;
  }
}

// Handle keyboard events
function handleKeydown(event: KeyboardEvent) {
  // Only handle arrow keys when search input is not focused
  const target = event.target as HTMLElement;
  if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;

  if (event.key === 'ArrowUp') {
    event.preventDefault();
    navigateUp();
  } else if (event.key === 'ArrowDown') {
    event.preventDefault();
    navigateDown();
  } else if (event.key === 'Enter' && hoveredIndex.value >= 0) {
    event.preventDefault();
    if (hoveredIndex.value < visibleTopics.value.length) {
      const topic = visibleTopics.value[hoveredIndex.value];
      if (topic) {
        selectTopic(topic);
      }
    }
  }
}

// Load all topics from all clusters or selected clusters
async function loadAllTopics() {
  if (isUnmounted.value) return;
  loading.value = true;
  // Reset pagination when loading fresh topics
  offset.value = 0;
  allTopics.value = [];
  try {
    const topics: TopicInfo[] = [];

    // Get selected cluster names
    const selectedClustersList = getAllSelectedClusterNames();

    // 如果有搜索词，传递给后端进行过滤
    const searchQueryValue = searchQuery.value.trim();

    // Use multi-cluster API
    const result = await apiClient.getTopicsWithClusters(
      selectedClustersList,
      0,
      limit.value,
      searchQueryValue || undefined
    );
    if (isUnmounted.value) return;
    for (const topic of result.topics) {
      topics.push({
        name: topic.name,
        cluster: topic.cluster
      });
    }
    total.value = result.total;
    hasMore.value = result.has_more;

    // 后端搜索已经排序，不需要再次排序
    // 只有在没有搜索词时才按 cluster 和 name 排序
    if (!searchQueryValue) {
      topics.sort((a, b) => {
        if (a.cluster !== b.cluster) {
          return a.cluster.localeCompare(b.cluster);
        }
        return a.name.localeCompare(b.name);
      });
    }

    if (!isUnmounted.value) {
      allTopics.value = topics;

      // Initialize hovered index to match selected topic or pending highlight
      if (pendingHighlight.value) {
        const index = topics.findIndex(
          t => t.cluster === pendingHighlight.value!.cluster && t.name === pendingHighlight.value!.topic
        );
        hoveredIndex.value = index >= 0 ? index : -1;
      } else if (selectedTopic.value) {
        const index = topics.findIndex(
          t => t.cluster === selectedTopic.value!.cluster && t.name === selectedTopic.value!.name
        );
        hoveredIndex.value = index >= 0 ? index : -1;
      } else {
        hoveredIndex.value = -1;
      }

      // Check if there's a pending highlight after topics load
      if (pendingHighlight.value) {
        const { cluster, topic } = pendingHighlight.value;
        const targetTopic = allTopics.value.find(
          t => t.cluster === cluster && t.name === topic
        );
        if (targetTopic) {
          selectedTopic.value = targetTopic;
        }
        pendingHighlight.value = null;
      }
    }
  } catch (e) {
    if (!isUnmounted.value) {
      console.error('Failed to load topics:', e);
    }
  } finally {
    if (!isUnmounted.value) {
      loading.value = false;
    }
  }
}

// Load more topics (pagination)
async function loadMoreTopics() {
  if (loadingMore.value || isUnmounted.value || !hasMore.value) return;

  loadingMore.value = true;
  try {
    const nextOffset = offset.value + limit.value;
    const selectedClustersList = getAllSelectedClusterNames();

    // 传递搜索词给后端
    const searchQueryValue = searchQuery.value.trim();

    const result = await apiClient.getTopicsWithClusters(
      selectedClustersList,
      nextOffset,
      limit.value,
      searchQueryValue || undefined
    );
    if (isUnmounted.value) return;

    // Append new topics
    for (const topic of result.topics) {
      allTopics.value.push({
        name: topic.name,
        cluster: topic.cluster
      });
    }

    offset.value = nextOffset;
    hasMore.value = result.has_more;
  } catch (e) {
    if (!isUnmounted.value) {
      console.error('Failed to load more topics:', e);
    }
  } finally {
    if (!isUnmounted.value) {
      loadingMore.value = false;
    }
  }
}

// Refresh topics from Kafka to SQLite database
async function refreshTopics() {
  if (refreshing.value || isUnmounted.value) return;

  refreshing.value = true;
  try {
    const selectedClustersList = getAllSelectedClusterNames();

    // Refresh only selected clusters
    for (const clusterName of selectedClustersList) {
      if (isUnmounted.value) return;
      try {
        await apiClient.refreshTopics(clusterName);
      } catch (e) {
        if (isUnmounted.value) return;
        // Silent failure - wait 5 seconds then continue to next cluster
        console.warn(`Failed to refresh topics for cluster ${clusterName}, waiting 5s before continuing...`);
        await new Promise(resolve => setTimeout(resolve, 5000));
        if (isUnmounted.value) return;
      }
    }

    // Cleanup orphan topics
    try {
      const result = await apiClient.cleanupOrphanTopics();
      if (isUnmounted.value) return;
      if (result.count > 0) {
        console.log(`Cleaned up ${result.count} orphan topics:`, result.removed);
      }
    } catch (e) {
      if (!isUnmounted.value) {
        console.warn('Failed to cleanup orphan topics:', e);
      }
    }

    // Reload topics after refresh
    if (!isUnmounted.value) {
      await loadAllTopics();
    }
  } catch (e) {
    if (!isUnmounted.value) {
      console.error('Failed to refresh topics:', e);
    }
  } finally {
    if (!isUnmounted.value) {
      refreshing.value = false;
    }
  }
}

// Search input handler with debounce
function onSearchInput() {
  // Reset hover index when search changes
  const index = visibleTopics.value.findIndex(
    t => selectedTopic.value && t.cluster === selectedTopic.value.cluster && t.name === selectedTopic.value.name
  );
  hoveredIndex.value = index >= 0 ? index : -1;

  if (searchTimer) {
    clearTimeout(searchTimer);
  }

  // 当 topic 总数超过 10000 时，使用后端搜索（防抖 300ms）
  if (useBackendSearch.value) {
    searchTimer = window.setTimeout(() => {
      // 重置 offset，重新加载（会传递搜索词给后端）
      offset.value = 0;
      loadAllTopics();
    }, 300);
  } else {
    // 前端搜索，不需要 API 调用
    searchTimer = window.setTimeout(() => {
      // Search is reactive via computed property
    }, 300);
  }
}

// Clear search
function clearSearch() {
  searchQuery.value = '';
  // Reset pagination
  offset.value = 0;
}

// Select topic
function selectTopic(topic: TopicInfo) {
  selectedTopic.value = topic;
  // Update hovered index to match selected topic
  const index = visibleTopics.value.findIndex(
    t => t.cluster === topic.cluster && t.name === topic.name
  );
  hoveredIndex.value = index;
  emit('navigate', {
    path: '/messages',
    query: {
      cluster: topic.cluster,
      topic: topic.name
    }
  });
}

// Go to clusters page
function goToClusters() {
  // Cancel any pending operations before navigation
  if (searchTimer) {
    clearTimeout(searchTimer);
    searchTimer = null;
  }
  apiClient.cancelRequest();

  // Emit navigation event immediately
  emit('navigate', {
    path: '/clusters'
  });
}

// Go to favorites page
function goToFavorites() {
  emit('navigate', {
    path: '/favorites'
  });
}

// Handle scroll to load more automatically
function handleScroll(event: Event) {
  if (scrollLock || !hasMore.value || loadingMore.value || isUnmounted.value) return;

  const target = event.target as HTMLElement;
  const { scrollTop, scrollHeight, clientHeight } = target;

  // Load more when scrolled to 80% of the list
  const scrollThreshold = scrollHeight * 0.8;
  if (scrollTop + clientHeight >= scrollThreshold && allTopics.value.length < 10000) {
    scrollLock = true;
    loadMoreTopics().then(() => {
      scrollLock = false;
    }).catch(() => {
      scrollLock = false;
    });
  }
}

watch([() => clusterStore.clusters.length, selectedClusters], () => {
  loadAllTopics();
}, { immediate: true });

// Watch for route changes to handle cluster and topic query params
watch(
  () => route.query,
  (newQuery) => {
    const cluster = newQuery.cluster as string;
    const topic = newQuery.topic as string;

    // 如果有 cluster 和 topic，处理高亮
    if (cluster && topic) {
      pendingHighlight.value = { cluster, topic };
      // 外部导航不改变 cluster 下拉框，只设置 pendingHighlight 用于高亮选中的 topic
    } else {
      // 清除 pending highlight
      pendingHighlight.value = null;
    }
  },
  { immediate: true }
);
</script>

<style scoped>
.topic-navigator {
  display: flex;
  flex-direction: column;
  height: 100%;
}

/* Custom scrollbar */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}

/* Vue virtual scroller styles */
.vue-recycle-scroller {
  position: relative;
}

.vue-recycle-scroller__item-wrapper {
  overflow: hidden;
}

.vue-recycle-scroller__item-view {
  position: absolute;
  top: 0;
  left: 0;
  will-change: transform;
}
</style>
