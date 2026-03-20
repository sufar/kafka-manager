<template>
  <div class="p-3">
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
            <span v-if="clusterParam">{{ t.dashboard.clusters }}: <span class="font-medium">{{ clusterParam }}</span></span>
            <span v-else>{{ t.topics.description }}</span>
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            v-if="!clusterParam && selectedClusterIds.length > 0"
            class="btn btn-xs"
            :class="viewMode === 'by-cluster' ? 'btn-primary' : 'btn-outline'"
            @click="viewMode = 'by-cluster'"
          >
            {{ t.dashboard.byCluster }}
          </button>
          <button
            v-if="!clusterParam && selectedClusterIds.length > 0"
            class="btn btn-xs"
            :class="viewMode === 'all-topics' ? 'btn-primary' : 'btn-outline'"
            @click="viewMode = 'all-topics'"
          >
            {{ t.topics.allTopics }}
          </button>
          <button
            class="btn btn-xs btn-outline"
            @click="refreshAllTopics"
            :disabled="refreshing || selectedClusterIds.length === 0"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            <span class="hidden md:inline ml-1">{{ t.common.refresh }}</span>
          </button>
          <button
            class="btn btn-xs btn-primary"
            @click="openCreateTopicDialog"
            :disabled="!clusterParam && selectedClusterIds.length !== 1"
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
    <div v-else-if="clusterParam && !filteredClusterTopics.length && !loading" class="flex flex-col items-center justify-center py-8 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-4 text-sm">{{ t.topics.description }}</p>
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
      <div ref="singleClusterContainerRef" class="overflow-x-auto overflow-y-auto" @scroll="handleSingleClusterScroll" style="max-height: calc(100vh - 250px);">
        <!-- Search Bar -->
        <div class="p-3 sticky top-0 bg-base-100 z-10">
          <div class="relative">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40">
              <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
            </svg>
            <input
              v-model="searchQuery"
              type="text"
              :placeholder="t.common.search"
              class="input input-bordered w-full max-w-md pl-10"
            />
          </div>
        </div>
        <table class="table">
          <thead>
            <tr>
              <th>
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                  </svg>
                  {{ t.topics.topicName }}
                </div>
              </th>
              <th>{{ t.common.actions }}</th>
            </tr>
          </thead>
          <tbody>
            <!-- 虚拟滚动：顶部占位 -->
            <tr v-if="singleClusterVirtualStartIndex > 0" :style="{ height: singleClusterVirtualStartIndex * ROW_HEIGHT + 'px' }">
              <td colspan="2" style="padding: 0; border: 0;"></td>
            </tr>
            <!-- 可见区域的行 -->
            <tr v-for="topic in singleClusterVisibleTopics" :key="topic.name" @dblclick="selectTopicInTree(clusterParam, topic)" class="hover cursor-pointer" :style="{ height: ROW_HEIGHT + 'px' }">
              <td>
                <div class="flex items-center gap-3">
                  <div class="grid h-6 w-6 place-items-center rounded bg-base-300 text-base-content/70">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m3.75 9v6m3-3H9m1.5-12H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9Z" />
                    </svg>
                  </div>
                  <FavoriteButton
                    :cluster-id="clusterParam"
                    :topic-name="topic.name"
                    :t="t"
                    @update="refreshFavorites"
                  />
                  <span class="font-medium">{{ topic.name }}</span>
                </div>
              </td>
              <td>
                <div class="flex gap-1">
                  <button class="btn btn-ghost btn-xs text-error hover:bg-error/10" @click="confirmDelete(clusterParam, topic.name)">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
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

    <!-- View by cluster mode -->
    <template v-else-if="viewMode === 'by-cluster'">
      <!-- Search Bar -->
      <div class="mb-4">
        <div class="relative">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
          </svg>
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Search topics..."
            class="input input-bordered w-full max-w-md pl-10"
          />
        </div>
      </div>
      <div v-for="(clusterTopics, clusterName) in filteredTopicsByCluster" :key="clusterName" class="mb-8">
        <div class="flex items-center justify-between mb-3">
          <div class="flex items-center gap-2">
            <div
              class="w-2 h-2 rounded-full"
              :class="[
                clusterHealthCache[clusterName]?.healthy ? 'bg-success animate-pulse' : 'bg-error'
              ]"
            ></div>
            <h3 class="text-lg font-semibold">{{ clusterName }}</h3>
            <span class="text-sm text-base-content/60">{{ clusterTopics.length }} topics</span>
          </div>
          <div class="flex gap-2">
            <button
              v-if="clusterTopics.length > 0"
              class="btn btn-outline btn-xs"
              @click="currentCluster = clusterName; viewAllTopics(clusterTopics)"
            >
              {{ t.common.refresh }}
            </button>
          </div>
        </div>

        <div v-if="clusterTopics.length === 0" class="card glass gradient-border">
          <p class="text-base-content/60 text-center p-8">{{ t.common.noData }}</p>
        </div>
        <div v-else class="card glass gradient-border">
          <div
            class="overflow-auto"
            :ref="(el: Element | ComponentPublicInstance | null) => setClusterContainerRef(el as HTMLElement | null, clusterName)"
            @scroll="(e: Event) => handleClusterScroll(e, clusterName)"
            style="max-height: calc(100vh - 300px);"
          >
            <table class="table">
              <thead class="sticky top-0 bg-base-100 z-10">
                <tr>
                  <th>{{ t.topics.topicName }}</th>
                  <th>{{ t.common.actions }}</th>
                </tr>
              </thead>
              <tbody>
                <template v-if="(visibleClusterTopicsMap[clusterName] || []).length > 0">
                  <tr v-for="topic in visibleClusterTopicsMap[clusterName]" :key="topic.name" @dblclick="selectTopicInTree(clusterName, topic)" class="hover cursor-pointer" :style="{ height: `${ROW_HEIGHT}px` }">
                    <td>
                      <div class="flex items-center gap-3">
                        <div class="grid h-6 w-6 place-items-center rounded bg-base-300 text-base-content/70">
                          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m3.75 9v6m3-3H9m1.5-12H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9Z" />
                          </svg>
                        </div>
                        <FavoriteButton
                          :cluster-id="clusterName"
                          :topic-name="topic.name"
                          :t="t"
                          @update="refreshFavorites"
                        />
                        <span class="font-medium">{{ topic.name }}</span>
                      </div>
                    </td>
                    <td>
                      <div class="flex gap-1">
                        <button class="btn btn-ghost btn-xs text-error" @click="confirmDelete(clusterName, topic.name)">{{ t.common.delete }}</button>
                      </div>
                    </td>
                  </tr>
                </template>
                <template v-else>
                  <tr style="height: 1px;"><td colspan="3"></td></tr>
                </template>
              </tbody>
            </table>
            <div :style="{ height: `${clusterBottomPaddingMap[clusterName] || 0}px` }"></div>
          </div>
        </div>
      </div>
    </template>

    <!-- All topics mode (consolidated) -->
    <template v-else>
      <!-- Search Bar -->
      <div class="mb-4">
        <div class="relative">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
          </svg>
          <input
            v-model="searchQuery"
            type="text"
            :placeholder="t.common.search"
            class="input input-bordered w-full max-w-md pl-10"
          />
        </div>
      </div>
      <div v-if="filteredAllTopicsList.length === 0" class="flex flex-col items-center justify-center py-12 text-center">
        <div class="text-base-content/30 mb-4">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
          </svg>
        </div>
        <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
        <p class="text-base-content/60 mb-4">{{ t.topics.description }}</p>
      </div>

      <div v-else class="card bg-base-200">
        <div ref="containerRef" class="overflow-auto" @scroll="handleScroll" style="max-height: calc(100vh - 250px);">
          <table class="table">
            <thead class="sticky top-0 bg-base-200 z-10">
              <tr>
                <th>{{ t.dashboard.clusters }}</th>
                <th>{{ t.topics.topicName }}</th>
                <th>{{ t.common.actions }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in visibleTopics" :key="`${item.cluster}-${item.name}`" @dblclick="selectTopicInTree(item.cluster, item)" class="hover cursor-pointer" :style="{ height: `${ROW_HEIGHT}px` }">
                <td>
                  <div class="flex items-center gap-2">
                    <div
                      class="w-2 h-2 rounded-full"
                      :class="[
                        getClusterHealth(item.cluster)?.healthy ? 'bg-success animate-pulse' : 'bg-error'
                      ]"
                    ></div>
                    <span class="font-medium">{{ item.cluster }}</span>
                  </div>
                </td>
                <td>
                  <div class="flex items-center gap-3">
                    <div class="grid h-6 w-6 place-items-center rounded bg-base-300 text-base-content/70">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m3.75 9v6m3-3H9m1.5-12H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9Z" />
                      </svg>
                    </div>
                    <span class="font-medium">{{ item.name }}</span>
                  </div>
                </td>
                <td>
                  <div class="flex gap-1">
                    <button class="btn btn-ghost btn-xs text-error" @click="confirmDelete(item.cluster, item.name)">{{ t.common.delete }}</button>
                  </div>
                </td>
              </tr>
              <!-- 占位行，保持滚动位置 -->
              <tr v-if="visibleTopics.length === 0" style="height: 1px;"></tr>
            </tbody>
          </table>
          <!-- 底部占位，用于虚拟滚动 -->
          <div :style="{ height: `${bottomPadding}px` }"></div>
        </div>
      </div>
    </template>

    <!-- Create Topic Dialog -->
    <dialog ref="createTopicDialogRef" class="modal modal-bottom sm:modal-middle">
      <form method="dialog" class="modal-box" @submit.prevent="handleCreateTopic">
        <h3 class="font-bold text-lg mb-4">{{ t.topics.createTopic }}</h3>

        <!-- Topic Name -->
        <div class="mb-4">
          <label class="label text-sm font-medium">{{ t.topics.topicName }}</label>
          <input
            v-model="newTopic.name"
            type="text"
            :placeholder="t.topics.topicNamePlaceholder"
            class="input input-bordered w-full"
            required
            pattern="^[a-zA-Z0-9._-]+$"
            :title="t.topics.topicNameValidation"
          />
        </div>

        <!-- Partitions -->
        <div class="mb-4">
          <label class="label text-sm font-medium">{{ t.topics.numPartitions }}</label>
          <input
            v-model.number="newTopic.numPartitions"
            type="number"
            min="1"
            max="100"
            class="input input-bordered w-full"
            required
          />
          <p class="text-xs text-base-content/60 mt-1">{{ t.topics.numPartitionsHelp }}</p>
        </div>

        <!-- Replication Factor -->
        <div class="mb-4">
          <label class="label text-sm font-medium">{{ t.topics.replicationFactor }}</label>
          <input
            v-model.number="newTopic.replicationFactor"
            type="number"
            min="1"
            max="10"
            class="input input-bordered w-full"
            required
          />
          <p class="text-xs text-base-content/60 mt-1">{{ t.topics.replicationFactorHelp }}</p>
        </div>

        <!-- Advanced Options -->
        <div class="mb-4">
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              v-model="showAdvanced"
              class="checkbox checkbox-sm"
            />
            <span class="text-sm">{{ t.topics.advancedOptions }}</span>
          </label>
        </div>

        <!-- Advanced Config -->
        <div v-if="showAdvanced" class="mb-4 space-y-3">
          <div>
            <label class="label text-sm font-medium">cleanup.policy</label>
            <select v-model="newTopic.config.cleanup_policy" class="select select-bordered w-full">
              <option value="delete">delete</option>
              <option value="compact">compact</option>
              <option value="delete,compact">delete,compact</option>
            </select>
          </div>
          <div>
            <label class="label text-sm font-medium">retention.ms</label>
            <input
              v-model="newTopic.config.retention_ms"
              type="text"
              placeholder="604800000 (7 days)"
              class="input input-bordered w-full"
            />
          </div>
          <div>
            <label class="label text-sm font-medium">retention.bytes</label>
            <input
              v-model="newTopic.config.retention_bytes"
              type="text"
              placeholder="-1 (unlimited)"
              class="input input-bordered w-full"
            />
          </div>
          <div>
            <label class="label text-sm font-medium">segment.bytes</label>
            <input
              v-model="newTopic.config.segment_bytes"
              type="text"
              placeholder="1073741824 (1GB)"
              class="input input-bordered w-full"
            />
          </div>
        </div>

        <div class="flex justify-end gap-2 mt-6">
          <button type="button" class="btn btn-ghost btn-sm" @click="closeCreateTopicDialog">
            {{ t.common.cancel }}
          </button>
          <button type="submit" class="btn btn-primary btn-sm" :disabled="creatingTopic">
            <span v-if="creatingTopic" class="loading loading-spinner loading-sm"></span>
            {{ t.common.create }}
          </button>
        </div>
      </form>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeCreateTopicDialog">{{ t.common.close }}</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch, type ComponentPublicInstance } from 'vue';
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

const viewMode = ref<'by-cluster' | 'all-topics'>('by-cluster');
const loading = ref(false);
const error = ref<string | null>(null);
const refreshing = ref(false);
const searchQuery = ref('');

const topicsByCluster = ref<Record<string, TopicItem[]>>({});
const allTopicsList = ref<TopicItem[]>([]);
const clusterTopics = ref<TopicItem[]>([]);

const currentCluster = ref<string>('');

// Create Topic Dialog
const createTopicDialogRef = ref<HTMLDialogElement>();
const creatingTopic = ref(false);
const showAdvanced = ref(false);
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
const ROW_HEIGHT = 52; // 每行高度（像素）
const VISIBLE_OFFSET = 5; // 额外渲染的行数（减少以优化性能）
const containerRef = ref<HTMLElement | null>(null); // used in template
void containerRef; // prevent ts-unused warning
const clusterContainerRefs = ref<Record<string, HTMLElement | null>>({});
const scrollTop = ref(0);
const containerHeight = ref(0);
const clusterScrollTops = ref<Record<string, number>>({});
const clusterContainerHeights = ref<Record<string, number>>({});

// Single cluster 模式虚拟滚动
const singleClusterContainerRef = ref<HTMLElement | null>(null);
void singleClusterContainerRef; // prevent ts-unused warning
const singleClusterScrollTop = ref(0);
const singleClusterContainerHeight = ref(0);

function handleScroll(event: Event) {
  const target = event.target as HTMLElement;
  scrollTop.value = target.scrollTop;
  containerHeight.value = target.clientHeight;
}

function handleSingleClusterScroll(event: Event) {
  const target = event.target as HTMLElement;
  singleClusterScrollTop.value = target.scrollTop;
  singleClusterContainerHeight.value = target.clientHeight;
}

function handleClusterScroll(event: Event, clusterName: string) {
  const target = event.target as HTMLElement;
  clusterScrollTops.value[clusterName] = target.scrollTop;
  clusterContainerHeights.value[clusterName] = target.clientHeight;
}

function setClusterContainerRef(el: HTMLElement | null, clusterName: string) {
  if (clusterName) {
    clusterContainerRefs.value[clusterName] = el;
  }
}

// 计算可见的行（all-topics 模式）
const visibleTopics = computed(() => {
  const allTopics = filteredAllTopicsList.value;
  if (!allTopics.length) return [];

  const startIndex = Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - VISIBLE_OFFSET);
  const visibleCount = Math.ceil(containerHeight.value / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
  const endIndex = Math.min(allTopics.length, startIndex + visibleCount);

  return allTopics.slice(startIndex, endIndex);
});

// 计算底部占位高度（all-topics 模式）
const bottomPadding = computed(() => {
  const allTopics = filteredAllTopicsList.value;
  if (!allTopics.length) return 0;

  const visibleCount = visibleTopics.value.length;
  if (visibleCount === 0) return 0;

  const startIndex = Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - VISIBLE_OFFSET);
  const hiddenBottom = Math.max(0, allTopics.length - startIndex - visibleCount);
  return hiddenBottom * ROW_HEIGHT;
});

// Single cluster 模式虚拟滚动
const singleClusterVirtualStartIndex = computed(() => {
  return Math.max(0, Math.floor(singleClusterScrollTop.value / ROW_HEIGHT) - VISIBLE_OFFSET);
});

const singleClusterVisibleTopics = computed(() => {
  const allTopics = filteredClusterTopics.value;
  if (!allTopics.length) return [];

  const startIndex = singleClusterVirtualStartIndex.value;
  const containerH = singleClusterContainerHeight.value || 600;
  const visibleCount = Math.ceil(containerH / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
  const endIndex = Math.min(allTopics.length, startIndex + visibleCount);

  return allTopics.slice(startIndex, endIndex);
});

// 集群健康状态缓存（避免频繁调用）
const clusterHealthCache = computed(() => {
  const result: Record<string, { healthy: boolean }> = {};
  for (const clusterName of Object.keys(topicsByCluster.value)) {
    const health = clusterStore.getClusterHealth(clusterName);
    result[clusterName] = { healthy: health?.healthy ?? false };
  }
  return result;
});

// 可见的集群主题列表（computed 缓存，按集群名称）
const visibleClusterTopicsMap = computed(() => {
  const result: Record<string, TopicItem[]> = {};
  for (const [clusterName, clusterTopics] of Object.entries(filteredTopicsByCluster.value)) {
    if (!clusterTopics.length) {
      result[clusterName] = [];
      continue;
    }
    const scrollY = clusterScrollTops.value[clusterName] || 0;
    const containerH = clusterContainerHeights.value[clusterName] || 600;
    const startIndex = Math.max(0, Math.floor(scrollY / ROW_HEIGHT) - VISIBLE_OFFSET);
    const visibleCount = Math.ceil(containerH / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
    const endIndex = Math.min(clusterTopics.length, startIndex + visibleCount);
    result[clusterName] = clusterTopics.slice(startIndex, endIndex);
  }
  return result;
});

// 集群底部占位高度（computed 缓存，按集群名称）
const clusterBottomPaddingMap = computed(() => {
  const result: Record<string, number> = {};
  for (const [clusterName, clusterTopics] of Object.entries(filteredTopicsByCluster.value)) {
    if (!clusterTopics.length) {
      result[clusterName] = 0;
      continue;
    }
    const scrollY = clusterScrollTops.value[clusterName] || 0;
    const containerH = clusterContainerHeights.value[clusterName] || 600;
    const startIndex = Math.max(0, Math.floor(scrollY / ROW_HEIGHT) - VISIBLE_OFFSET);
    const visibleCount = Math.ceil(containerH / ROW_HEIGHT) + VISIBLE_OFFSET * 2;
    const endIndex = Math.min(clusterTopics.length, startIndex + visibleCount);
    const visibleTopics = clusterTopics.slice(startIndex, endIndex);
    if (visibleTopics.length === 0) {
      result[clusterName] = 0;
    } else {
      const hiddenBottom = Math.max(0, clusterTopics.length - startIndex - visibleTopics.length);
      result[clusterName] = hiddenBottom * ROW_HEIGHT;
    }
  }
  return result;
});

function getClusterHealth(clusterId: string) {
  return clusterStore.getClusterHealth(clusterId);
}

// Filtered topics based on search query (使用防抖优化)
const searchQueryDebounced = ref('');
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchQuery, (newVal) => {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
  searchDebounceTimer = setTimeout(() => {
    searchQueryDebounced.value = newVal;
  }, 150); // 150ms 防抖
});

const filteredTopicsByCluster = computed(() => {
  if (!searchQueryDebounced.value) return topicsByCluster.value;

  const query = searchQueryDebounced.value.toLowerCase();
  const filtered: Record<string, TopicItem[]> = {};

  for (const [clusterName, topics] of Object.entries(topicsByCluster.value)) {
    // 使用更高效的搜索方式
    const filteredTopics = [];
    for (const topic of topics) {
      if (topic.name.toLowerCase().includes(query)) {
        filteredTopics.push(topic);
      }
      // 限制每个集群的搜索结果数量，避免渲染过多
      if (filteredTopics.length >= 100) break;
    }
    if (filteredTopics.length > 0) {
      filtered[clusterName] = filteredTopics;
    }
  }

  return filtered;
});

const filteredAllTopicsList = computed(() => {
  if (!searchQueryDebounced.value) return allTopicsList.value;

  const query = searchQueryDebounced.value.toLowerCase();
  const result = [];
  for (const topic of allTopicsList.value) {
    if (topic.name.toLowerCase().includes(query)) {
      result.push(topic);
    }
    // 限制搜索结果数量
    if (result.length >= 200) break;
  }
  return result;
});

const filteredClusterTopics = computed(() => {
  if (!searchQuery.value) return clusterTopics.value;

  const query = searchQuery.value.toLowerCase();
  return clusterTopics.value.filter(topic =>
    topic.name.toLowerCase().includes(query)
  );
});

onBeforeRouteUpdate((to) => {
  if (to.query.cluster) {
    fetchTopics();
  }
  // 处理 search 参数变化
  const searchVal = Array.isArray(to.query.search) ? to.query.search[0] : (to.query.search || '');
  if (searchVal) {
    searchQuery.value = searchVal;
  } else {
    searchQuery.value = '';
  }
});

// 使用 watch 替代 watchEffect，精确监听特定依赖
watch([clusterParam, selectedClusterIds], ([newClusterParam, newSelectedClusterIds]) => {
  if (newClusterParam) {
    fetchTopics();
  } else if (newSelectedClusterIds.length > 0) {
    fetchTopics();
  }
}, { immediate: true });

async function fetchTopics() {
  loading.value = true;
  error.value = null;

  if (clusterParam.value) {
    try {
      const topicNames = await apiClient.getTopics(clusterParam.value);
      clusterTopics.value = topicNames.map((name) => ({
        name,
        cluster: clusterParam.value as string,
        partition_count: undefined,
      }));
      topicsByCluster.value = {};
      allTopicsList.value = [];
    } catch (e) {
      console.error('[TopicsView] Error fetching topics:', e);
      error.value = (e as { message: string }).message || 'Failed to load topics';
    } finally {
      loading.value = false;
    }
    return;
  }

  if (selectedClusterIds.value.length === 0) {
    loading.value = false;
    return;
  }

  // When multiple clusters are selected (no specific clusterParam), call API without cluster_id to get all topics
  if (selectedClusterIds.value.length > 1) {
    try {
      const topicNames = await apiClient.getTopics();
      // For all topics view without cluster filter, we use empty string as cluster placeholder
      allTopicsList.value = topicNames.map((name) => ({
        name,
        cluster: '',
        partition_count: undefined,
      }));
      topicsByCluster.value = {};
      clusterTopics.value = [];
    } catch (e) {
      console.error('[TopicsView] Error fetching all topics:', e);
      error.value = (e as { message: string }).message || 'Failed to load topics';
    } finally {
      loading.value = false;
    }
    return;
  }

  // Only one cluster selected - fetch from that cluster
  topicsByCluster.value = {};
  allTopicsList.value = [];

  try {
    const clusterId = selectedClusterIds.value[0];
    const topicNames = await apiClient.getTopics(clusterId);
    const topics: TopicItem[] = topicNames.map((name) => ({
      name,
      cluster: clusterId || '',
      partition_count: undefined,
    }));
    topicsByCluster.value[clusterId || ''] = topics;
    updateAllTopicsList();
  } catch (e) {
    error.value = (e as { message: string }).message || 'Failed to load topics';
  } finally {
    loading.value = false;
  }
}

function updateAllTopicsList() {
  allTopicsList.value = Object.entries(topicsByCluster.value).flatMap(([cluster, topics]) =>
    topics.map((t) => ({ ...t, cluster }))
  );
}

function viewAllTopics(topics: TopicItem[]) {
  currentCluster.value = topics[0]?.cluster || '';
  viewMode.value = 'all-topics';
}

// 双击 Topic 时触发：通知父组件展开并选中左侧树中的对应 Topic
function selectTopicInTree(clusterName: string, topic: TopicItem) {
  // 触发自定义事件，由 ModernLayout 捕获并处理
  window.dispatchEvent(new CustomEvent('select-topic-in-tree', {
    detail: { topicName: topic.name, clusterName }
  }));
}

async function confirmDelete(clusterId: string, topicName: string) {
  if (confirm(t.value.layout.confirmDeleteTopic.replace('{topic}', topicName))) {
    try {
      await apiClient.deleteTopic(clusterId, topicName);
      showSuccess('Topic deleted successfully');
      fetchTopics();
    } catch (e) {
      showError((e as { message: string }).message);
    }
  }
}

async function refreshAllTopics() {
  if (selectedClusterIds.value.length === 0) return;

  refreshing.value = true;
  try {
    const promises = selectedClusterIds.value.map((clusterId) =>
      apiClient.refreshTopics(clusterId).catch(() => {})
    );
    await Promise.all(promises);

    // 等待后台同步完成（异步刷新现在立即返回，需要等待一下）
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
  if (!clusterParam.value && selectedClusterIds.value.length !== 1) {
    showError('Please select a cluster first');
    return;
  }

  const clusterId = clusterParam.value || selectedClusterIds.value[0];
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

// Placeholder function for refreshing favorites (called when favorites are updated)
async function refreshFavorites() {
  // Favorites are refreshed automatically in the FavoriteButton component
  // This function is called via emit to notify parent if needed
}

onMounted(() => {
  // 从URL读取搜索参数
  if (searchParam.value) {
    searchQuery.value = searchParam.value;
  }
});
</script>
