<template>
  <div class="p-3">
    <!-- Detail View -->
    <div v-if="detailView" class="space-y-4">
      <!-- Header with group name and actions -->
      <div class="card glass gradient-border shadow-xl">
        <div class="flex items-center justify-between p-4">
          <div class="flex items-center gap-3">
            <button class="btn btn-ghost btn-sm" @click="detailView = false">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5L3 12m0 0l7.5-7.5M3 12h18" />
              </svg>
            </button>
            <div>
              <h2 class="text-xl font-bold flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                </svg>
                {{ currentGroup }}
              </h2>
              <p class="text-base-content/60 text-sm mt-1">
                {{ t.clusters.clusters }}: <span class="font-medium">{{ clusterParam }}</span>
                <span v-if="groupState" class="ml-3 badge" :class="getStateBadgeClass(groupState)">{{ groupState }}</span>
              </p>
            </div>
          </div>
          <div class="flex gap-2">
            <button
              class="btn btn-sm btn-outline"
              @click="refreshOffsets"
              :disabled="refreshing"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4" :class="{ 'animate-spin': refreshing }">
                <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
              </svg>
              {{ t.common.refresh }}
            </button>
            <div class="dropdown dropdown-end">
              <label tabindex="0" class="btn btn-sm btn-primary">
                {{ t.common.actions }}
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 ml-1">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                </svg>
              </label>
              <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52">
                <li><a @click="openResetOffsetDialog">{{ t.consumerGroups.resetOffset }}</a></li>
                <li><a @click="deleteConsumerGroup" class="text-error">{{ t.consumerGroups.deleteGroup }}</a></li>
              </ul>
            </div>
          </div>
        </div>
      </div>

      <!-- Loading state -->
      <div v-if="loadingDetail" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-md text-primary"></span>
        <p class="ml-4 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
      </div>

      <!-- Error state -->
      <div v-else-if="detailError" class="alert alert-error">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
        </svg>
        <span class="text-sm">{{ detailError }}</span>
      </div>

      <!-- Offsets Table -->
      <div v-else-if="offsets.length > 0" class="card glass gradient-border shadow-xl">
        <div class="p-3 bg-base-100 border-b border-base-200 flex items-center justify-between">
          <h3 class="font-semibold">{{ t.consumerGroups.offsets }}</h3>
          <span class="text-xs text-base-content/60">{{ offsets.length }} {{ t.consumerGroups.partitions }}</span>
        </div>
        <div class="overflow-x-auto">
          <table class="table w-full">
            <thead>
              <tr>
                <th class="p-2">{{ t.consumerGroups.topic }}</th>
                <th class="p-2">{{ t.consumerGroups.partition }}</th>
                <th class="p-2 text-right">{{ t.consumerGroups.startOffset }}</th>
                <th class="p-2 text-right">{{ t.consumerGroups.endOffset }}</th>
                <th class="p-2 text-right">{{ t.consumerGroups.committedOffset }}</th>
                <th class="p-2 text-right">{{ t.consumerGroups.lag }}</th>
                <th class="p-2 text-right">{{ t.consumerGroups.lastCommit }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in offsets" :key="`${item.topic}-${item.partition}`" class="hover">
                <td class="font-medium">{{ item.topic }}</td>
                <td><span class="badge badge-ghost badge-sm">{{ item.partition }}</span></td>
                <td class="text-right font-mono text-sm">{{ formatNumber(item.start_offset) }}</td>
                <td class="text-right font-mono text-sm">{{ formatNumber(item.end_offset) }}</td>
                <td class="text-right font-mono text-sm">{{ formatNumber(item.committed_offset) }}</td>
                <td class="text-right">
                  <span :class="getLagClass(item.lag)" class="font-mono text-sm">{{ formatNumber(item.lag) }}</span>
                </td>
                <td class="text-right text-xs text-base-content/60">{{ formatTime(item.last_commit) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- No data state -->
      <div v-else class="card glass gradient-border shadow-xl">
        <div class="flex flex-col items-center justify-center py-12 text-center">
          <div class="text-base-content/30 mb-4">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
            </svg>
          </div>
          <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
          <p class="text-base-content/60 mb-4 text-sm">{{ t.consumerGroups.noOffsets }}</p>
          <button class="btn btn-sm btn-outline" @click="refreshOffsets">{{ t.common.refresh }}</button>
        </div>
      </div>

      <!-- Reset Offset Dialog -->
      <dialog ref="resetOffsetDialog" class="modal modal-bottom sm:modal-middle">
        <div class="modal-box">
          <h3 class="font-bold text-lg mb-4">{{ t.consumerGroups.resetOffset }}</h3>
          <div class="form-control w-full">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.selectTopic }}</span>
            </label>
            <select v-model="resetOffsetForm.topic" class="select select-bordered w-full">
              <option v-for="topic in uniqueTopics" :key="topic" :value="topic">{{ topic }}</option>
            </select>
          </div>
          <div class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.partition }}</span>
            </label>
            <select v-model="resetOffsetForm.partition" class="select select-bordered w-full">
              <option v-for="p in getPartitionsForTopic(resetOffsetForm.topic)" :key="p" :value="p">{{ p }}</option>
            </select>
          </div>
          <div class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.resetTo }}</span>
            </label>
            <select v-model="resetOffsetForm.resetTo" class="select select-bordered w-full">
              <option value="earliest">{{ t.consumerGroups.earliest }}</option>
              <option value="latest">{{ t.consumerGroups.latest }}</option>
              <option value="offset">{{ t.consumerGroups.specificOffset }}</option>
              <option value="timestamp">{{ t.consumerGroups.timestamp }}</option>
            </select>
          </div>
          <div v-if="resetOffsetForm.resetTo === 'offset'" class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.offsetValue }}</span>
            </label>
            <input v-model.number="resetOffsetForm.offsetValue" type="number" class="input input-bordered w-full" />
          </div>
          <div v-if="resetOffsetForm.resetTo === 'timestamp'" class="form-control w-full mt-4">
            <label class="label">
              <span class="label-text">{{ t.consumerGroups.timestampValue }}</span>
            </label>
            <input v-model="resetOffsetForm.timestampValue" type="datetime-local" class="input input-bordered w-full" />
          </div>
          <div class="modal-action">
            <button class="btn btn-ghost" @click="closeResetOffsetDialog">{{ t.common.cancel }}</button>
            <button class="btn btn-primary" @click="confirmResetOffset" :disabled="resettingOffset">{{ t.common.confirm }}</button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </div>

    <!-- List View -->
    <div v-else>
      <!-- Page Header -->
      <div class="mb-4">
        <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
          <div>
            <h1 class="text-xl font-bold flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
                <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
              </svg>
              {{ t.consumerGroups.title }}
            </h1>
            <p class="text-base-content/60 mt-1 text-sm">
              <span v-if="clusterParam">{{ t.clusters.clusters }}: <span class="font-medium">{{ clusterParam }}</span></span>
              <span v-else>{{ t.consumerGroups.description }}</span>
            </p>
          </div>
          <div class="flex flex-wrap gap-2">
            <button
              class="btn btn-xs btn-outline"
              @click="refreshConsumerGroups"
              :disabled="refreshing || !clusterParam"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }">
                <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
              </svg>
              <span class="hidden md:inline ml-1">{{ t.common.refresh }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- No cluster selected -->
      <div v-if="!clusterParam" class="flex flex-col items-center justify-center py-8 text-center">
        <div class="text-base-content/30 mb-4">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
            <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
          </svg>
        </div>
        <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
        <p class="text-base-content/60 mb-4 text-sm">{{ t.consumerGroups.description }}</p>
      </div>

      <!-- Loading state -->
      <div v-else-if="loading" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-md text-primary"></span>
        <p class="ml-4 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
      </div>

      <!-- Error state -->
      <div v-else-if="error" class="alert alert-error">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
        </svg>
        <span class="text-sm">{{ error }}</span>
      </div>

      <!-- Consumer Groups list -->
      <div v-else-if="clusterParam && filteredConsumerGroups.length > 0" class="card glass gradient-border shadow-xl">
        <!-- Search Bar -->
        <div class="p-3 bg-base-100">
          <div class="relative w-full">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40">
              <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
            </svg>
            <input
              v-model="searchQuery"
              type="text"
              :placeholder="t.common.search"
              class="input input-bordered w-full pl-10"
            />
          </div>
        </div>
        <!-- Table Header -->
        <div class="bg-base-100 border-b border-base-200">
          <table class="table w-full">
            <thead>
              <tr>
                <th class="p-2">
                  <div class="flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                    </svg>
                    <span class="text-sm font-semibold">{{ t.consumerGroups.groupName }}</span>
                  </div>
                </th>
                <th class="p-2">
                  <span class="text-sm font-semibold">{{ t.consumerGroups.topics }}</span>
                </th>
                <th class="p-2 text-right pr-4">{{ t.common.actions }}</th>
              </tr>
            </thead>
          </table>
        </div>
        <!-- Table Content -->
        <div ref="containerRef" class="overflow-y-auto" @scroll="handleScroll" style="max-height: calc(100vh - 350px);">
          <table class="table w-full">
            <tbody>
              <!-- Virtual scroll: top spacer -->
              <tr v-if="virtualStartIndex > 0" :style="{ height: virtualStartIndex * ROW_HEIGHT + 'px' }">
                <td colspan="3" style="padding: 0; border: 0;"></td>
              </tr>
              <!-- Visible rows -->
              <tr v-for="group in visibleConsumerGroups" :key="group.name" @dblclick="selectConsumerGroup(clusterParam, group)" class="hover cursor-pointer" :style="{ height: `${ROW_HEIGHT}px`, minHeight: `${ROW_HEIGHT}px` }">
                <td>
                  <div class="flex items-center gap-2">
                    <div class="grid h-5 w-5 place-items-center rounded bg-base-300 text-base-content/70">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                      </svg>
                    </div>
                    <span class="font-medium text-sm">{{ group.name }}</span>
                  </div>
                </td>
                <td>
                  <div class="flex flex-wrap gap-1 max-w-xs">
                    <span v-for="topic in group.topics.slice(0, 3)" :key="topic" class="badge badge-ghost badge-xs">
                      {{ topic }}
                    </span>
                    <span v-if="group.topics.length > 3" class="badge badge-ghost badge-xs">
                      +{{ group.topics.length - 3 }}
                    </span>
                  </div>
                </td>
                <td class="p-2">
                  <div class="flex justify-end gap-0.5">
                    <button class="btn btn-ghost btn-xs" @click="viewConsumerGroup(group.name)" title="View">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" />
                        <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
                      </svg>
                    </button>
                    <button class="btn btn-ghost btn-xs" @click="refreshOffsetsInList(group.name)" :title="t.common.refresh">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                      </svg>
                    </button>
                  </div>
                </td>
              </tr>
              <!-- Virtual scroll: bottom spacer -->
              <tr v-if="virtualStartIndex + visibleConsumerGroups.length < filteredConsumerGroups.length" :style="{ height: (filteredConsumerGroups.length - virtualStartIndex - visibleConsumerGroups.length) * ROW_HEIGHT + 'px' }">
                <td colspan="3" style="padding: 0; border: 0;"></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- No data state -->
      <div v-else-if="clusterParam && consumerGroups.length === 0" class="card glass gradient-border shadow-xl">
        <div class="flex flex-col items-center justify-center py-12 text-center">
          <div class="text-base-content/30 mb-4">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
              <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
            </svg>
          </div>
          <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
          <p class="text-base-content/60 mb-4 text-sm">{{ t.consumerGroups.emptyHelp }}</p>
          <button class="btn btn-sm btn-outline" @click="refreshConsumerGroups">{{ t.common.refresh }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

interface ConsumerGroupItem {
  name: string;
  cluster: string;
  topics: string[];
}

interface OffsetItem {
  topic: string;
  partition: number;
  start_offset: number;
  end_offset: number;
  committed_offset: number;
  lag: number;
  last_commit?: string;
}

const route = useRoute();
const router = useRouter();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

const clusterParam = computed(() => {
  const val = route.query.cluster;
  return Array.isArray(val) ? val[0] : (val || '');
});

const groupParam = computed(() => {
  const val = route.query.group;
  return Array.isArray(val) ? val[0] : (val || '');
});

// Detail view state
const detailView = ref(false);
const currentGroup = ref('');
const groupState = ref('');
const loadingDetail = ref(false);
const detailError = ref<string | null>(null);
const offsets = ref<OffsetItem[]>([]);

// Reset offset dialog
const resetOffsetDialog = ref<HTMLDialogElement | null>(null);
const resettingOffset = ref(false);
const resetOffsetForm = ref<{
  topic: string;
  partition: number;
  resetTo: 'earliest' | 'latest' | 'offset' | 'timestamp';
  offsetValue: number;
  timestampValue: string;
}>({
  topic: '',
  partition: -1,
  resetTo: 'earliest',
  offsetValue: 0,
  timestampValue: '',
});

const searchQuery = ref('');
const loading = ref(false);
const error = ref<string | null>(null);
const refreshing = ref(false);

const consumerGroups = ref<ConsumerGroupItem[]>([]);

// Virtual scroll
const ROW_HEIGHT = 40;
const VISIBLE_OFFSET = 5;
const containerRef = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const containerHeight = ref(0);

function handleScroll(event: Event) {
  const target = event.target as HTMLElement;
  if (!target) return;
  scrollTop.value = target.scrollTop;
  containerHeight.value = target.clientHeight;
}

const virtualStartIndex = computed(() => {
  return Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - VISIBLE_OFFSET);
});

const visibleConsumerGroups = computed(() => {
  const allGroups = filteredConsumerGroups.value;
  if (!allGroups.length) return [];

  const startIndex = virtualStartIndex.value;
  const containerH = containerHeight.value > 0 ? containerHeight.value : 400;
  const visibleCount = Math.ceil(containerH / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
  const endIndex = Math.min(allGroups.length, startIndex + visibleCount);

  return allGroups.slice(startIndex, endIndex);
});

const filteredConsumerGroups = computed(() => {
  if (!searchQuery.value) return consumerGroups.value;

  const query = searchQuery.value.toLowerCase();
  return consumerGroups.value.filter(group =>
    group.name.toLowerCase().includes(query) ||
    group.topics.some(topic => topic.toLowerCase().includes(query))
  );
});

// Unique topics from all consumer groups
const uniqueTopics = computed(() => {
  const topics = new Set<string>();
  consumerGroups.value.forEach(g => g.topics.forEach(t => topics.add(t)));
  return Array.from(topics).sort();
});

// Watch for group parameter to show detail view
watch([() => groupParam.value, () => clusterParam.value, () => route.path], ([newGroup, newCluster, newPath], [oldGroup, oldCluster, oldPath]) => {
  console.log('[ConsumerGroupsView] Watch triggered:', { newGroup, newCluster, newPath, oldGroup, oldCluster, oldPath, detailView: detailView.value });

  if (newGroup && newCluster && newPath === '/consumer-groups') {
    detailView.value = true;
    currentGroup.value = newGroup;
    loadConsumerGroupDetail();
  } else if (newPath === '/consumer-groups' && clusterParam.value && !groupParam.value) {
    detailView.value = false;
    console.log('[ConsumerGroupsView] Loading consumer groups list');
    fetchConsumerGroups();
  } else {
    detailView.value = false;
  }
}, { immediate: true });

async function loadConsumerGroupDetail() {
  if (!clusterParam.value || !currentGroup.value) return;

  loadingDetail.value = true;
  detailError.value = null;

  try {
    // Get group info
    const groupInfo = await apiClient.getConsumerGroupInfo(clusterParam.value, currentGroup.value);
    groupState.value = groupInfo.state || 'Unknown';

    // Get offsets
    await loadOffsets();
  } catch (e) {
    console.error('[ConsumerGroupsView] Error loading group detail:', e);
    detailError.value = (e as { message: string }).message || 'Failed to load consumer group details';
  } finally {
    loadingDetail.value = false;
  }
}

async function loadOffsets() {
  if (!clusterParam.value || !currentGroup.value) return;

  try {
    const offsetsData = await apiClient.getConsumerGroupOffsets(clusterParam.value, currentGroup.value);
    offsets.value = offsetsData.map(o => ({
      ...o,
      last_commit: new Date().toISOString(),
    }));
  } catch (e) {
    console.error('[ConsumerGroupsView] Error loading offsets:', e);
    throw e;
  }
}

async function fetchConsumerGroups() {
  console.log('[ConsumerGroupsView] fetchConsumerGroups called', { clusterParam: clusterParam.value });
  loading.value = true;
  error.value = null;

  if (!clusterParam.value) {
    loading.value = false;
    return;
  }

  try {
    // Use consumer_group.list API to get groups with topics information
    const result = await apiClient.getConsumerGroupsList([clusterParam.value]);
    console.log('[ConsumerGroupsView] fetchConsumerGroups result:', result);
    consumerGroups.value = result.groups.map((g) => ({
      name: g.group_name,
      cluster: g.cluster_id,
      topics: g.topics || [],
    }));
  } catch (e) {
    console.error('[ConsumerGroupsView] Error fetching consumer groups:', e);
    error.value = (e as { message: string }).message || 'Failed to load consumer groups';
  } finally {
    loading.value = false;
  }
}

function selectConsumerGroup(clusterName: string, group: ConsumerGroupItem) {
  window.dispatchEvent(new CustomEvent('select-consumer-group-in-tree', {
    detail: { groupName: group.name, clusterName }
  }));
}

function viewConsumerGroup(groupName: string) {
  router.push({
    path: '/consumer-groups',
    query: { cluster: clusterParam.value, group: groupName }
  });
}

function refreshOffsetsInList(groupName: string) {
  window.dispatchEvent(new CustomEvent('refresh-consumer-group-offsets', {
    detail: { groupName, clusterName: clusterParam.value }
  }));
}

async function refreshOffsets() {
  if (!clusterParam.value || !currentGroup.value) return;

  refreshing.value = true;
  try {
    await loadOffsets();
    showSuccess(t.value.consumerGroups.offsetsRefreshed);
  } catch (e) {
    console.error('[ConsumerGroupsView] Refresh offsets error:', e);
    showError(`Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value = false;
  }
}

async function refreshConsumerGroups() {
  if (!clusterParam.value) return;

  refreshing.value = true;
  try {
    // First, refresh data from Kafka to database
    const refreshResult = await apiClient.refreshConsumerGroups(clusterParam.value);
    console.log('[ConsumerGroupsView] Refresh result:', refreshResult);

    // Wait for sync to complete
    await new Promise(resolve => setTimeout(resolve, 500));

    // Then fetch the updated data from database
    await fetchConsumerGroups();

    // If in detail view, reload the detail
    if (detailView.value && currentGroup.value) {
      await loadConsumerGroupDetail();
    }

    if (refreshResult && refreshResult.success) {
      showSuccess(t.value.consumerGroups.refreshed);
    } else {
      showError('Refresh completed but no data was updated');
    }
  } catch (e) {
    console.error('[ConsumerGroupsView] Refresh error:', e);
    showError(`Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value = false;
  }
}

function openResetOffsetDialog() {
  resetOffsetForm.value = {
    topic: uniqueTopics.value[0] || '',
    partition: -1,
    resetTo: 'earliest',
    offsetValue: 0,
    timestampValue: '',
  };
  resetOffsetDialog.value?.showModal();
}

function closeResetOffsetDialog() {
  resetOffsetDialog.value?.close();
}

function getPartitionsForTopic(topic: string): number[] {
  const topicOffsets = offsets.value.filter(o => o.topic === topic);
  const partitions = topicOffsets.map(o => o.partition);
  // Remove duplicates and sort
  return [...new Set(partitions)].sort((a, b) => a - b);
}

async function confirmResetOffset() {
  if (!clusterParam.value || !currentGroup.value) return;

  const { topic, partition, resetTo, offsetValue, timestampValue } = resetOffsetForm.value;

  if (partition < 0) {
    showError('Please select a partition');
    return;
  }

  if (resetTo === 'offset' && offsetValue < 0) {
    showError('Please enter a valid offset value');
    return;
  }

  if (resetTo === 'timestamp' && !timestampValue) {
    showError('Please enter a timestamp');
    return;
  }

  resettingOffset.value = true;
  try {
    let resetToParam: 'earliest' | 'latest' | 'offset' | 'timestamp' = resetTo;
    let timestampParam: number | undefined = undefined;
    let offsetParam: number | undefined = undefined;

    if (resetTo === 'offset') {
      offsetParam = offsetValue;
    } else if (resetTo === 'timestamp') {
      timestampParam = new Date(timestampValue).getTime();
    }

    await apiClient.resetConsumerGroupOffset(
      clusterParam.value,
      currentGroup.value,
      topic,
      partition,
      resetToParam,
      timestampParam,
      offsetParam
    );

    showSuccess(t.value.consumerGroups.offsetResetSuccess);
    closeResetOffsetDialog();
    await loadOffsets();
  } catch (e) {
    console.error('[ConsumerGroupsView] Reset offset error:', e);
    showError(`Reset failed: ${(e as { message: string }).message}`);
  } finally {
    resettingOffset.value = false;
  }
}

async function deleteConsumerGroup() {
  if (!clusterParam.value || !currentGroup.value) return;

  if (!confirm(`Are you sure you want to delete consumer group "${currentGroup.value}"?`)) {
    return;
  }

  try {
    await apiClient.deleteConsumerGroup(clusterParam.value, currentGroup.value);
    showSuccess(t.value.consumerGroups.deleted);
    // Go back to list view
    router.push({ path: '/consumer-groups', query: { cluster: clusterParam.value } });
    detailView.value = false;
    await fetchConsumerGroups();
  } catch (e) {
    console.error('[ConsumerGroupsView] Delete error:', e);
    showError(`Delete failed: ${(e as { message: string }).message}`);
  }
}

// Utility functions
function getStateBadgeClass(state: string): string {
  const lowerState = state.toLowerCase();
  if (lowerState === 'stable' || lowerState === 'empty') return 'badge-success';
  if (lowerState === 'preparing_rebalance' || lowerState === 'completing_rebalance') return 'badge-warning';
  if (lowerState === 'dead' || lowerState === 'unknown') return 'badge-error';
  return 'badge-ghost';
}

function getLagClass(lag: number): string {
  if (lag === 0) return 'text-success';
  if (lag < 1000) return 'text-warning';
  return 'text-error';
}

function formatNumber(num: number): string {
  if (num < 0) return '-';
  if (num >= 1000000) return (num / 1000000).toFixed(2) + 'M';
  if (num >= 1000) return (num / 1000).toFixed(2) + 'K';
  return num.toString();
}

function formatTime(timeStr?: string): string {
  if (!timeStr) return '-';
  try {
    const date = new Date(timeStr);
    return date.toLocaleString();
  } catch {
    return timeStr;
  }
}

onMounted(() => {
  console.log('[ConsumerGroupsView] onMounted', { clusterParam: clusterParam.value, groupParam: groupParam.value, path: route.path });
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight || 400;
  }
  // Load consumer groups on mount if cluster is selected and not in detail view
  if (clusterParam.value && !groupParam.value && route.path === '/consumer-groups') {
    console.log('[ConsumerGroupsView] Fetching consumer groups on mount');
    fetchConsumerGroups();
  }
});
</script>

<style scoped>
.table :deep(tbody tr) {
  height: 40px;
}

.table :deep(td) {
  padding: 0.25rem 0.5rem;
  vertical-align: middle;
}

.table :deep(th) {
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  text-transform: none;
  letter-spacing: normal;
}
</style>
