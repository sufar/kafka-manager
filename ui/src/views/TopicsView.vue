<template>
  <div class="p-3 overflow-auto min-h-full">
    <!-- Page Header -->
    <div class="mb-4">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
            </svg>
            {{ t.topics.title }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">
            <span v-if="clusterParam">{{ t.clusters.clusters }}: <span class="font-medium">{{ clusterParam }}</span></span>
            <span v-else>{{ t.topics.description }}</span>
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            class="btn btn-xs btn-outline"
            @click="refreshAllTopics"
            :disabled="refreshing || !clusterParam"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            <span class="hidden md:inline ml-1">{{ t.common.refresh }}</span>
          </button>
          <button
            class="btn btn-xs btn-primary"
            @click="openCreateTopicDialog"
            :disabled="!clusterParam"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
            <span class="hidden md:inline ml-1">{{ t.common.create }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- No cluster selected -->
    <div v-if="!clusterParam && selectedClusterIds.length === 0" class="flex flex-col items-center justify-center py-8 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-4 text-sm">{{ t.topics.description }}</p>
    </div>

    <!-- Single cluster view (from URL param) -->
    <div v-else-if="clusterParam && filteredClusterTopics.length === 0 && !loading" class="card glass gradient-border shadow-xl">
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
      <!-- 搜索无结果或真正无数据 -->
      <div class="flex flex-col items-center justify-center py-12 text-center">
        <div class="text-base-content/30 mb-4">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
          </svg>
        </div>
        <h3 class="text-lg font-semibold mb-2">{{ searchQuery ? t.topics.noSearchResults : t.common.noData }}</h3>
        <p class="text-base-content/60 mb-4 text-sm">
          <span v-if="searchQuery">{{ t.common.search }}: "{{ searchQuery }}"</span>
          <span v-else>{{ t.topics.description }}</span>
        </p>
        <button v-if="searchQuery" class="btn btn-sm btn-outline" @click="searchQuery = ''">{{ t.topics.clearSearch }}</button>
      </div>
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

    <!-- Single cluster view (from URL param) -->
    <div v-else-if="clusterParam && filteredClusterTopics.length > 0" class="card glass gradient-border shadow-xl">
      <!-- Search Bar - 固定在容器外部 -->
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
      <!-- 表格容器 - 只有表格内容滚动 -->
      <!-- 表头 - 固定 -->
      <div class="bg-base-100 border-b border-base-200">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="p-2">
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                  </svg>
                  <span class="text-sm font-semibold">{{ t.topics.topicName }}</span>
                </div>
              </th>
              <th class="p-2 text-right pr-4">{{ t.common.actions }}</th>
            </tr>
          </thead>
        </table>
      </div>
      <!-- 表格内容 - 滚动 -->
      <div ref="singleClusterContainerRef" class="overflow-y-auto" @scroll="handleSingleClusterScroll" style="max-height: calc(100vh - 350px);">
        <table class="table w-full">
          <tbody>
            <!-- 虚拟滚动：顶部占位 -->
            <tr v-if="singleClusterVirtualStartIndex > 0" :style="{ height: singleClusterVirtualStartIndex * ROW_HEIGHT + 'px' }">
              <td colspan="2" style="padding: 0; border: 0;"></td>
            </tr>
            <!-- 可见区域的行 -->
            <tr v-for="topic in singleClusterVisibleTopics" :key="topic.name" @dblclick="selectTopicInTree(clusterParam, topic)" class="hover cursor-pointer" :style="{ height: `${ROW_HEIGHT}px`, minHeight: `${ROW_HEIGHT}px` }">
              <td>
                <div class="flex items-center gap-2">
                  <div class="grid h-5 w-5 place-items-center rounded bg-base-300 text-base-content/70">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m3.75 9v6m3-3H9m1.5-12H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9Z" />
                    </svg>
                  </div>
                  <FavoriteButton
                    :cluster-id="clusterParam"
                    :topic-name="topic.name"
                    :t="t"
                    :favorite-cache="favoriteCache"
                    @change="handleFavoriteChange(clusterParam, topic.name, $event)"
                  />
                  <span class="font-medium text-sm">{{ topic.name }}</span>
                </div>
              </td>
              <td class="p-2">
                <div class="flex justify-end gap-0.5">
                  <button class="btn btn-ghost btn-xs text-error hover:bg-error/10" @click="confirmDelete(clusterParam, topic.name)">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                    </svg>
                    {{ t.common.delete }}
                  </button>
                </div>
              </td>
            </tr>
            <!-- 虚拟滚动：底部占位 -->
            <tr v-if="singleClusterVirtualStartIndex + singleClusterVisibleTopics.length < filteredClusterTopics.length" :style="{ height: (filteredClusterTopics.length - singleClusterVirtualStartIndex - singleClusterVisibleTopics.length) * ROW_HEIGHT + 'px' }">
              <td colspan="2" style="padding: 0; border: 0;"></td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Loading state -->
    <div v-else-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-md text-primary"></span>
      <p class="ml-4 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
    </div>

    <!-- Create Topic Dialog -->
    <Teleport to="body">
      <dialog ref="createTopicDialogRef" class="modal modal-bottom sm:modal-middle" @click.self="closeCreateTopicDialog">
        <div class="modal-box w-full max-w-2xl mx-2 md:mx-auto p-5">
          <!-- Header -->
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                </svg>
              </div>
              <div>
                <h3 class="font-bold text-base">{{ t.topics.createTopic }}</h3>
                <span class="text-xs text-base-content/60 font-mono">{{ newTopic.name || t.topics.topicNamePlaceholder }}</span>
              </div>
            </div>
            <button class="btn btn-sm btn-circle btn-ghost" @click="closeCreateTopicDialog">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <form @submit.prevent="handleCreateTopic" class="space-y-4">
        <!-- Topic Name -->
        <div>
          <label class="block text-sm font-medium mb-1.5">
            <span class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
              </svg>
              {{ t.topics.topicName }} <span class="text-error text-xs ml-1">{{ t.messages.required }}</span>
            </span>
          </label>
          <input
            v-model="newTopic.name"
            type="text"
            :placeholder="t.topics.topicNamePlaceholder"
            class="input input-bordered input-sm w-full"
            required
            pattern="^[a-zA-Z0-9._-]+$"
            :title="t.topics.topicNameValidation"
          />
        </div>

        <!-- Partitions and Replication Factor Row -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- Partitions -->
          <div>
            <label class="block text-sm font-medium mb-1.5">
              <span class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6Z" />
                </svg>
                {{ t.topics.numPartitions }}
              </span>
            </label>
            <input
              v-model.number="newTopic.numPartitions"
              type="number"
              min="1"
              max="100"
              class="input input-bordered input-sm w-full"
              required
            />
            <p class="text-xs text-base-content/60 mt-1">{{ t.topics.numPartitionsHelp }}</p>
          </div>

          <!-- Replication Factor -->
          <div>
            <label class="block text-sm font-medium mb-1.5">
              <span class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                </svg>
                {{ t.topics.replicationFactor }}
              </span>
            </label>
            <input
              v-model.number="newTopic.replicationFactor"
              type="number"
              min="1"
              max="10"
              class="input input-bordered input-sm w-full"
              required
            />
            <p class="text-xs text-base-content/60 mt-1">{{ t.topics.replicationFactorHelp }}</p>
          </div>
        </div>

        <!-- Advanced Options -->
        <div class="border-t border-base-content/10 pt-4">
          <button type="button" class="btn btn-ghost btn-sm w-full justify-between group" @click="showAdvanced = !showAdvanced">
            <span class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 transition-transform group-hover:rotate-90" :class="{ 'rotate-90': showAdvanced }">
                <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
              </svg>
              {{ t.topics.advancedOptions }}
            </span>
            <span class="text-xs text-base-content/50">{{ showAdvanced ? t.messages.hide : t.messages.show }}</span>
          </button>

          <!-- Advanced Config -->
          <div v-if="showAdvanced" class="mt-3 space-y-3 animate-fadeIn">
            <div>
              <label class="block text-sm font-medium mb-1.5">cleanup.policy</label>
              <select v-model="newTopic.config.cleanup_policy" class="select select-bordered select-sm w-full">
                <option value="delete">delete</option>
                <option value="compact">compact</option>
                <option value="delete,compact">delete,compact</option>
              </select>
            </div>
            <div>
              <label class="block text-sm font-medium mb-1.5">retention.ms</label>
              <input
                v-model="newTopic.config.retention_ms"
                type="text"
                placeholder="604800000 (7 days)"
                class="input input-bordered input-sm w-full"
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-1.5">retention.bytes</label>
              <input
                v-model="newTopic.config.retention_bytes"
                type="text"
                placeholder="-1 (unlimited)"
                class="input input-bordered input-sm w-full"
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-1.5">segment.bytes</label>
              <input
                v-model="newTopic.config.segment_bytes"
                type="text"
                placeholder="1073741824 (1GB)"
                class="input input-bordered input-sm w-full"
              />
            </div>
          </div>
        </div>

            <div class="modal-action flex-wrap gap-2 pt-3">
              <button type="button" class="btn btn-ghost btn-sm" @click="closeCreateTopicDialog">{{ t.common.cancel }}</button>
              <button type="submit" class="btn btn-primary btn-sm flex items-center gap-2" :disabled="creatingTopic">
                <span v-if="creatingTopic" class="loading loading-spinner loading-sm"></span>
                {{ t.common.create }}
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Delete Topic Confirmation Dialog -->
    <Teleport to="body">
      <dialog ref="deleteTopicDialogRef" class="modal modal-bottom sm:modal-middle" @click.self="closeDeleteDialog">
        <div class="modal-box w-full max-w-md mx-2 md:mx-auto p-5">
          <!-- Header -->
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-error/20 to-error/10 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-error">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                </svg>
              </div>
              <div>
                <h3 class="font-bold text-base">{{ t.topics.confirmDeleteTitle }}</h3>
                <span class="text-xs text-base-content/60">{{ t.topics.confirmDeleteHint }}</span>
              </div>
            </div>
            <button class="btn btn-sm btn-circle btn-ghost" @click="closeDeleteDialog">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="space-y-4">
            <!-- Topic Info -->
            <div class="bg-base-200 rounded-lg p-3">
              <div class="mb-2">
                <label class="text-xs text-base-content/60 block mb-1">Cluster</label>
                <div class="flex items-center gap-2">
                  <div
                    class="w-2 h-2 rounded-full"
                    :class="[
                      getClusterHealth(deleteTarget.cluster)?.healthy ? 'bg-success' : 'bg-error'
                    ]"
                  ></div>
                  <span class="font-medium text-sm">{{ deleteTarget.cluster }}</span>
                </div>
              </div>
              <div>
                <label class="text-xs text-base-content/60 block mb-1">{{ t.topics.topicName }}</label>
                <div class="flex items-center gap-2">
                  <code class="text-sm bg-base-100 px-2 py-1 rounded flex-1 truncate block">{{ deleteTarget.topic }}</code>
                  <button
                    class="btn btn-ghost btn-xs"
                    @click="copyTopicName"
                    :title="t.common.copy"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                    </svg>
                  </button>
                </div>
              </div>
            </div>

            <!-- Confirmation Input -->
            <div>
              <label class="block text-sm font-medium mb-1.5">
                {{ t.topics.confirmDeleteInput }}
              </label>
              <input
                v-model="deleteConfirmInput"
                type="text"
                :placeholder="deleteTarget.topic"
                class="input input-bordered w-full"
                @input="checkMatch"
              />
              <p v-if="deleteMatchError" class="text-xs text-error mt-1">{{ deleteMatchError }}</p>
            </div>
          </div>

          <div class="modal-action flex-wrap gap-2 pt-4">
            <button type="button" class="btn btn-ghost btn-sm" @click="closeDeleteDialog">{{ t.common.cancel }}</button>
            <button
              type="button"
              class="btn btn-error btn-sm"
              :disabled="!deleteInputMatches || deletingTopic"
              @click="handleDeleteTopic"
            >
              <span v-if="deletingTopic" class="loading loading-spinner loading-sm"></span>
              {{ t.common.delete }}
            </button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useRoute, onBeforeRouteUpdate } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';
import FavoriteButton from '@/components/FavoriteButton.vue';

// 定义本地类型
interface TopicItem {
  name: string;
  cluster: string;
  partition_count?: number;
  replication_factor?: number;
  status?: string;
}

const route = useRoute();
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

const selectedClusterIds = computed(() => clusterStore.selectedClusterIds);

const clusterParam = computed(() => {
  const val = route.query.cluster;
  return Array.isArray(val) ? val[0] : (val || '');
});


// 从URL读取搜索参数
const searchParam = computed(() => {
  const val = route.query.search;
  return Array.isArray(val) ? val[0] : (val || '');
});

const loading = ref(false);
const error = ref<string | null>(null);
const refreshing = ref(false);
const searchQuery = ref('');

const topicsByCluster = ref<Record<string, TopicItem[]>>({});
const clusterTopics = ref<TopicItem[]>([]);

// Favorite cache: Set of "clusterId-topicName" keys
const favoriteCache = ref<Set<string>>(new Set());

// Create Topic Dialog
const createTopicDialogRef = ref<HTMLDialogElement>();
const creatingTopic = ref(false);
const showAdvanced = ref(false);

// Delete Topic Dialog
const deleteTopicDialogRef = ref<HTMLDialogElement>();
const deletingTopic = ref(false);
const deleteTarget = reactive({ cluster: '', topic: '' });
const deleteConfirmInput = ref('');
const deleteInputMatches = ref(false);
const deleteMatchError = ref('');

const newTopic = reactive({
  name: '',
  numPartitions: 3,
  replicationFactor: 1,
  config: {
    cleanup_policy: 'delete',
    retention_ms: '',
    retention_bytes: '',
    segment_bytes: '',
  } as Record<string, string>,
});

// 虚拟滚动相关
const ROW_HEIGHT = 40; // 每行高度（像素）
const VISIBLE_OFFSET = 5; // 额外渲染的行数（减少以优化性能）

// Single cluster 模式虚拟滚动
const singleClusterContainerRef = ref<HTMLElement | null>(null);
void singleClusterContainerRef; // prevent ts-unused warning
const singleClusterScrollTop = ref(0);
const singleClusterContainerHeight = ref(0);

function handleSingleClusterScroll(event: Event) {
  const target = event.target as HTMLElement;
  if (!target) return;
  singleClusterScrollTop.value = target.scrollTop;
  singleClusterContainerHeight.value = target.clientHeight;
}

// Single cluster 模式虚拟滚动
const singleClusterVirtualStartIndex = computed(() => {
  return Math.max(0, Math.floor(singleClusterScrollTop.value / ROW_HEIGHT) - VISIBLE_OFFSET);
});

const singleClusterVisibleTopics = computed(() => {
  const allTopics = filteredClusterTopics.value;
  if (!allTopics.length) return [];

  const startIndex = singleClusterVirtualStartIndex.value;
  // 使用容器实际高度或默认值（确保有可见行）
  const containerH = singleClusterContainerHeight.value > 0 ? singleClusterContainerHeight.value : 400;
  const visibleCount = Math.ceil(containerH / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
  const endIndex = Math.min(allTopics.length, startIndex + visibleCount);

  return allTopics.slice(startIndex, endIndex);
});

// Filtered topics based on search query (使用防抖优化)
const searchQueryDebounced = ref('');
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchQuery, (newVal) => {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  searchDebounceTimer = setTimeout(() => {
    searchQueryDebounced.value = newVal;
  }, 150); // 150ms 防抖
});

function getClusterHealth(clusterId: string) {
  return clusterStore.getClusterHealth(clusterId);
}

const filteredClusterTopics = computed(() => {
  if (!searchQueryDebounced.value) return clusterTopics.value;

  const query = searchQueryDebounced.value.toLowerCase();
  return clusterTopics.value.filter(topic =>
    topic.name.toLowerCase().includes(query)
  );
});

onBeforeRouteUpdate((to) => {
  // 处理 search 参数变化
  const searchVal = Array.isArray(to.query.search) ? to.query.search[0] : (to.query.search || '');
  if (searchVal) {
    searchQuery.value = searchVal;
  } else {
    searchQuery.value = '';
  }
});

// 使用 watch 替代 watchEffect，精确监听 clusterParam 变化
watch(clusterParam, (newClusterParam) => {
  if (newClusterParam) {
    fetchTopics();
  }
}, { immediate: true });

async function fetchTopics() {
  loading.value = true;
  error.value = null;

  if (!clusterParam.value) {
    loading.value = false;
    return;
  }

  try {
    const topicNames = await apiClient.getTopics(clusterParam.value);
    clusterTopics.value = topicNames.map((name) => ({
      name,
      cluster: clusterParam.value as string,
      partition_count: undefined,
    }));
    topicsByCluster.value = {};
    // Fetch favorites
    await fetchFavorites();
  } catch (e) {
    console.error('[TopicsView] Error fetching topics:', e);
    error.value = (e as { message: string }).message || 'Failed to load topics';
  } finally {
    loading.value = false;
  }
}

// 双击 Topic 时触发：通知父组件展开并选中左侧树中的对应 Topic
function selectTopicInTree(clusterName: string, topic: TopicItem) {
  // 触发自定义事件，由 ModernLayout 捕获并处理
  window.dispatchEvent(new CustomEvent('select-topic-in-tree', {
    detail: { topicName: topic.name, clusterName }
  }));
}

async function confirmDelete(clusterId: string, topicName: string) {
  // Open delete confirmation dialog
  deleteTarget.cluster = clusterId;
  deleteTarget.topic = topicName;
  deleteConfirmInput.value = '';
  deleteInputMatches.value = false;
  deleteMatchError.value = '';
  deleteTopicDialogRef.value?.showModal();
}

function closeDeleteDialog() {
  deleteTopicDialogRef.value?.close();
}

function checkMatch() {
  const input = deleteConfirmInput.value.trim();
  if (input === deleteTarget.topic) {
    deleteInputMatches.value = true;
    deleteMatchError.value = '';
  } else {
    deleteInputMatches.value = false;
    deleteMatchError.value = '';
  }
}

async function copyTopicName() {
  try {
    await navigator.clipboard.writeText(deleteTarget.topic);
    showSuccess(t.value.topics.copied || 'Copied');
  } catch (e) {
    showError(t.value.topics.copyFailed || 'Copy failed');
  }
}

async function handleDeleteTopic() {
  if (!deleteInputMatches.value) {
    deleteMatchError.value = t.value.topics.confirmDeleteMatchError || 'Topic name does not match';
    return;
  }

  deletingTopic.value = true;
  try {
    await apiClient.deleteTopic(deleteTarget.cluster, deleteTarget.topic);
    showSuccess(t.value.topics.deletedSuccess || 'Topic deleted successfully');
    closeDeleteDialog();
    fetchTopics();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    deletingTopic.value = false;
  }
}

async function refreshAllTopics() {
  if (!clusterParam.value) return;

  refreshing.value = true;
  try {
    await apiClient.refreshTopics(clusterParam.value).catch(() => {});

    // 等待后台同步完成
    await new Promise(resolve => setTimeout(resolve, 500));

    await fetchTopics();
    showSuccess(t.value.topics.refreshed);
  } catch (e) {
    showError(`Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value = false;
  }
}

// Create Topic Dialog methods
function openCreateTopicDialog() {
  // Reset form
  newTopic.name = '';
  newTopic.numPartitions = 3;
  newTopic.replicationFactor = 1;
  newTopic.config = {
    cleanup_policy: 'delete',
    retention_ms: '',
    retention_bytes: '',
    segment_bytes: '',
  };
  showAdvanced.value = false;
  createTopicDialogRef.value?.showModal();
}

function closeCreateTopicDialog() {
  createTopicDialogRef.value?.close();
}

async function handleCreateTopic() {
  if (!clusterParam.value) {
    showError('Please select a cluster first');
    return;
  }

  const clusterId = clusterParam.value;
  if (!clusterId) {
    showError('Cluster ID is required');
    return;
  }

  // Validate topic name - Kafka topic names cannot contain spaces or special characters
  const trimmedName = newTopic.name.trim();
  if (!trimmedName) {
    showError('Topic name is required');
    return;
  }
  if (trimmedName.length > 256) {
    showError('Topic name cannot exceed 256 characters');
    return;
  }
  // Topic 名称不能包含空格、引号、逗号
  if (trimmedName.includes(' ') || trimmedName.includes('"') || trimmedName.includes("'") || trimmedName.includes(',')) {
    showError('Topic name cannot contain spaces, quotes, or commas');
    return;
  }

  // Kafka topic naming rules: only letters, numbers, dots, underscores, and hyphens
  const topicNameRegex = /^[a-zA-Z0-9._-]+$/;
  if (!topicNameRegex.test(trimmedName)) {
    showError('Topic name can only contain letters, numbers, dots, underscores, and hyphens');
    return;
  }

  creatingTopic.value = true;
  try {
    // Build config object with correct Kafka config key names (using dots, not underscores)
    // Only include config if advanced options are enabled
    const config: Record<string, string> = {};
    if (showAdvanced.value) {
      if (newTopic.config.cleanup_policy && newTopic.config.cleanup_policy.trim()) {
        config['cleanup.policy'] = newTopic.config.cleanup_policy.trim();
      }
      if (newTopic.config.retention_ms && newTopic.config.retention_ms.trim()) {
        // Validate it's a number
        const retentionMs = parseInt(newTopic.config.retention_ms.trim(), 10);
        if (isNaN(retentionMs) || retentionMs < 0) {
          showError('retention.ms must be a positive number');
          creatingTopic.value = false;
          return;
        }
        config['retention.ms'] = retentionMs.toString();
      }
      if (newTopic.config.retention_bytes && newTopic.config.retention_bytes.trim()) {
        // Validate it's a number
        const retentionBytes = parseInt(newTopic.config.retention_bytes.trim(), 10);
        if (isNaN(retentionBytes)) {
          showError('retention.bytes must be a number (use -1 for unlimited)');
          creatingTopic.value = false;
          return;
        }
        config['retention.bytes'] = retentionBytes.toString();
      }
      if (newTopic.config.segment_bytes && newTopic.config.segment_bytes.trim()) {
        // Validate it's a number
        const segmentBytes = parseInt(newTopic.config.segment_bytes.trim(), 10);
        if (isNaN(segmentBytes) || segmentBytes < 0) {
          showError('segment.bytes must be a positive number');
          creatingTopic.value = false;
          return;
        }
        config['segment.bytes'] = segmentBytes.toString();
      }
    }

    await apiClient.createTopic(clusterId, {
      name: trimmedName,
      num_partitions: newTopic.numPartitions,
      replication_factor: newTopic.replicationFactor,
      config: Object.keys(config).length > 0 ? config : undefined,
    });

    showSuccess(`Topic "${trimmedName}" created successfully`);
    closeCreateTopicDialog();
    await fetchTopics();
  } catch (e) {
    showError(`Failed to create topic: ${(e as { message: string }).message}`);
  } finally {
    creatingTopic.value = false;
  }
}

// Fetch favorites in batch and populate the cache
const favoritesCacheTime = 5 * 60 * 1000; // 5 分钟缓存
let lastFavoritesFetchTime = 0;
let favoritesFetchPromise: Promise<void> | null = null;

async function fetchFavorites() {
  // 如果正在请求中，等待完成
  if (favoritesFetchPromise) {
    return favoritesFetchPromise;
  }

  // 如果缓存未过期，使用缓存
  const now = Date.now();
  if (now - lastFavoritesFetchTime < favoritesCacheTime) {
    return;
  }

  favoritesFetchPromise = (async () => {
    try {
      const groups = await apiClient.getFavorites();

      // Always fetch all favorites and cache them
      const newCache = new Set<string>();

      for (const group of groups) {
        for (const item of group.items) {
          newCache.add(`${item.cluster_id}-${item.topic_name}`);
        }
      }

      favoriteCache.value = newCache;
      lastFavoritesFetchTime = Date.now();
    } catch (e) {
      console.error('[TopicsView] Error fetching favorites:', e);
      // 超时等错误不阻断主流程
    } finally {
      favoritesFetchPromise = null;
    }
  })();

  return favoritesFetchPromise;
}

// Handle favorite change event to update cache
function handleFavoriteChange(clusterId: string, topicName: string, isFavorite: boolean) {
  const key = `${clusterId}-${topicName}`;
  if (isFavorite) {
    favoriteCache.value.add(key);
  } else {
    favoriteCache.value.delete(key);
  }
}

onMounted(() => {
  // 从URL读取搜索参数
  if (searchParam.value) {
    searchQuery.value = searchParam.value;
  }
  // 初始化容器高度
  if (singleClusterContainerRef.value) {
    singleClusterContainerHeight.value = singleClusterContainerRef.value.clientHeight || 400;
  }
});
</script>

<style scoped>
/* 紧凑表格样式 */
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

/* 可展开区域的动画 */
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fadeIn {
  animation: fadeIn 0.2s ease-out;
}

/* 移除背景模糊效果 */
:global(.modal:has(.modal-box)::backdrop) {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:global(dialog[open]::backdrop) {
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
</style>
