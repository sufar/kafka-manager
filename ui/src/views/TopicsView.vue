<template>
  <div class="p-3">
    <!-- Page Header -->
    <div class="mb-4">
      <div class="flex items-center justify-between">
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
        <div class="flex gap-2">
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
            {{ t.common.refresh }}
          </button>
          <button
            class="btn btn-xs btn-primary"
            @click="() => openCreateModal()"
            :disabled="selectedClusterIds.length === 0 && !clusterParam"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
            {{ t.topics.createTopic }}
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
      <button class="btn btn-primary btn-sm" @click="() => openCreateModal()">{{ t.topics.createTopic }}</button>
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
      <div class="overflow-x-auto">
        <!-- Search Bar -->
        <div class="p-3">
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
              <th>{{ t.topics.partitions }}</th>
              <th>{{ t.topics.replicationFactor }}</th>
              <th>{{ t.common.actions }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="topic in filteredClusterTopics" :key="topic.name" @dblclick="selectTopicInTree(clusterParam, topic)" class="hover cursor-pointer">
              <td>
                <div class="flex items-center gap-3">
                  <div class="grid h-6 w-6 place-items-center rounded bg-base-300 text-base-content/70">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m3.75 9v6m3-3H9m1.5-12H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9Z" />
                    </svg>
                  </div>
                  <span class="font-medium">{{ topic.name }}</span>
                </div>
              </td>
              <td>
                <div class="badge badge-neutral">{{ topic.partition_count || '-' }}</div>
              </td>
              <td>
                <span class="text-base-content/60">{{ topic.replication_factor || '-' }}</span>
              </td>
              <td>
                <div class="flex gap-1">
                  <button class="btn btn-ghost btn-xs" @click="viewTopicDetail(clusterParam, topic)">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.09.27.09.56 0 .83C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" />
                      <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                    {{ t.topics.viewDetails }}
                  </button>
                  <button class="btn btn-ghost btn-xs" @click="viewTopicMessages(clusterParam, topic)">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9.776c.112-.017.224-.026.336-.026h15.84c.112 0 .224.009.336.026m0-.026c.298.046.59.116.872.21l1.912.637a1.125 1.125 0 010 2.136l-1.912.637c-.282.094-.574.164-.872.21m-16.8.026c-.298.046-.59.116-.872.21l-1.912.637a1.125 1.125 0 010 2.136l1.912.637c.282.094.574.164-.872.21m12.078-6.053a3 3 0 00-2.974-2.723c-.624-.033-1.252.025-1.865.17-.64.151-1.247.382-1.808.683m6.647 1.873c.242.53.412 1.096.503 1.686m-12.078.026c.298-.046-.59-.116-.872-.21l1.912-.637a1.125 1.125 0 010-2.136l-1.912-.637c-.282-.094-.574-.164-.872-.21m16.8-.026c-.298-.046-.59-.116-.872-.21l-1.912-.637a1.125 1.125 0 010-2.136l1.912-.637c.282.094.574.164-.872.21" />
                    </svg>
                    {{ t.topics.viewMessages }}
                  </button>
                  <button class="btn btn-ghost btn-xs text-error hover:bg-error/10" @click="confirmDelete(clusterParam, topic.name)">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                    </svg>
                    {{ t.common.delete }}
                  </button>
                </div>
              </td>
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
              class="btn btn-primary btn-xs"
              @click="() => { targetCluster = clusterName; openCreateModal() }"
            >
              {{ t.topics.createTopic }}
            </button>
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
                  <th>{{ t.topics.partitions }}</th>
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
                        <span class="font-medium">{{ topic.name }}</span>
                      </div>
                    </td>
                    <td><div class="badge badge-neutral">{{ topic.partition_count || '-' }}</div></td>
                    <td>
                      <div class="flex gap-1">
                        <button class="btn btn-ghost btn-xs" @click="viewTopicDetail(clusterName, topic)">{{ t.topics.viewDetails }}</button>
                        <button class="btn btn-ghost btn-xs" @click="viewTopicMessages(clusterName, topic)">{{ t.topics.viewMessages }}</button>
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
                <th>{{ t.topics.partitions }}</th>
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
                <td><div class="badge badge-neutral">{{ item.partition_count || '-' }}</div></td>
                <td>
                  <div class="flex gap-1">
                    <button class="btn btn-ghost btn-xs" @click="viewTopicDetail(item.cluster, item)">{{ t.topics.viewDetails }}</button>
                    <button class="btn btn-ghost btn-xs" @click="viewTopicMessages(item.cluster, item)">{{ t.topics.viewMessages }}</button>
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

    <!-- Create Topic Modal using Teleport -->
    <Teleport to="body">
      <dialog ref="createModalRef" class="modal">
        <div class="modal-box">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg font-bold">{{ t.topics.createTopic }}</h3>
            <form method="dialog">
              <button class="btn btn-sm btn-circle btn-ghost">✕</button>
            </form>
          </div>
          <form @submit.prevent="handleCreate" class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.dashboard.clusters }}</span>
              </label>
              <input
                v-model="targetCluster"
                type="text"
                class="input input-bordered"
                readonly
              />
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.topics.topicName }}</span>
              </label>
              <input
                v-model="newTopic.name"
                type="text"
                placeholder="my-topic"
                class="input input-bordered"
                required
              />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t.topics.partitions }}</span>
                </label>
                <input
                  v-model.number="newTopic.num_partitions"
                  type="number"
                  min="1"
                  class="input input-bordered"
                />
              </div>
              <div class="form-control">
                <label class="label">
                  <span class="label-text">{{ t.topics.replicationFactor }}</span>
                </label>
                <input
                  v-model.number="newTopic.replication_factor"
                  type="number"
                  min="1"
                  class="input input-bordered"
                />
              </div>
            </div>
            <div class="modal-action mt-6">
              <button type="button" class="btn" @click="closeCreateModal">{{ t.common.cancel }}</button>
              <button type="submit" class="btn btn-primary" :disabled="creating">
                <span v-if="creating" class="loading loading-spinner loading-sm mr-2"></span>
                {{ t.topics.createTopic }}
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Topic Detail Modal using Teleport -->
    <Teleport to="body">
      <dialog ref="detailModalRef" class="modal">
        <div class="modal-box modal-box-lg p-0">
          <!-- Header -->
          <div class="p-4 border-b border-base-content/10 flex items-center justify-between bg-base-200">
            <div>
              <h3 class="text-lg font-bold flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                </svg>
                {{ selectedTopicDetail?.name }}
              </h3>
              <p class="text-xs text-base-content/60 mt-1">
                Cluster: {{ selectedTopicCluster }}
              </p>
            </div>
            <button class="btn btn-sm btn-circle btn-ghost" @click="closeDetailModal">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Tabs -->
          <div class="tabs tabs-boxed bg-base-200 p-2">
            <a :class="['tab tab-sm', activeTab === 'partitions' ? 'tab-active' : '']" @click="activeTab = 'partitions'">Partitions</a>
            <a :class="['tab tab-sm', activeTab === 'config' ? 'tab-active' : '']" @click="activeTab = 'config'">Config</a>
            <a :class="['tab tab-sm', activeTab === 'consumers' ? 'tab-active' : '']" @click="activeTab = 'consumers'">Consumers</a>
            <a :class="['tab tab-sm', activeTab === 'tags' ? 'tab-active' : '']" @click="activeTab = 'tags'">Tags</a>
          </div>

          <!-- Tab Content -->
          <div class="p-4 max-h-96 overflow-auto">
            <!-- Partitions Tab -->
            <div v-if="activeTab === 'partitions'">
              <div class="flex items-center justify-between mb-3">
                <div class="badge badge-neutral">{{ selectedTopicDetail?.partitions.length }} partitions</div>
              </div>
              <div class="overflow-x-auto">
                <table class="table table-zebra">
                  <thead>
                    <tr>
                      <th>Partition</th>
                      <th>Leader</th>
                      <th>ISR</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="partition in selectedTopicDetail?.partitions" :key="partition.id">
                      <td><span class="font-mono">{{ partition.id }}</span></td>
                      <td><span class="badge badge-ghost badge-xs">{{ partition.leader }}</span></td>
                      <td>
                        <span :class="['font-mono text-xs', partition.isr.length === partition.replicas.length ? 'text-success' : 'text-warning']">
                          {{ partition.isr.join(', ') }}
                        </span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Config Tab -->
            <div v-if="activeTab === 'config'">
              <div class="flex items-center justify-between mb-3">
                <h4 class="font-semibold text-sm">Topic Configuration</h4>
                <button class="btn btn-primary btn-xs" @click="editConfig" :disabled="!topicConfig">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.863 4.487zm0 0L10.5 10.5" />
                  </svg>
                  Edit Config
                </button>
              </div>
              <div v-if="!topicConfig" class="text-center py-8 text-base-content/60">
                <span class="loading loading-spinner loading-sm"></span>
                <p class="text-xs mt-2">Loading configuration...</p>
              </div>
              <div v-else class="space-y-2">
                <div v-for="(value, key) in topicConfig" :key="key" class="flex items-center justify-between p-2 bg-base-100 rounded">
                  <span class="text-xs font-mono text-base-content/60">{{ key }}</span>
                  <span class="text-xs font-mono">{{ value }}</span>
                </div>
              </div>
            </div>

            <!-- Consumers Tab -->
            <div v-if="activeTab === 'consumers'">
              <div class="flex items-center justify-between mb-3">
                <h4 class="font-semibold text-sm">Consumer Groups</h4>
                <button class="btn btn-ghost btn-xs" @click="fetchConsumers">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                  </svg>
                  Refresh
                </button>
              </div>
              <div v-if="consumersLoading" class="text-center py-8">
                <span class="loading loading-spinner loading-sm"></span>
              </div>
              <div v-else-if="!consumers || consumers.length === 0" class="text-center py-8 text-base-content/60">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 mx-auto mb-2 opacity-50">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198l.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.94 3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0z" />
                </svg>
                <p class="text-xs">No consumer groups found for this topic</p>
              </div>
              <div v-else class="space-y-2">
                <div v-for="consumer in consumers" :key="consumer.groupId" class="p-3 bg-base-100 rounded border border-base-content/10">
                  <div class="flex items-center justify-between mb-2">
                    <div class="flex items-center gap-2">
                      <span class="font-mono text-sm font-semibold">{{ consumer.groupId }}</span>
                      <span :class="['badge badge-xs', consumer.state === 'Stable' ? 'badge-success' : 'badge-warning']">
                        {{ consumer.state || 'Unknown' }}
                      </span>
                    </div>
                    <span class="text-xs text-base-content/60">{{ consumer.members?.length || 0 }} members</span>
                  </div>
                  <div class="grid grid-cols-3 gap-2 text-xs">
                    <div>
                      <span class="text-base-content/60">Lag:</span>
                      <span class="font-mono ml-1">{{ formatNumber(consumer.totalLag || 0) }}</span>
                    </div>
                    <div>
                      <span class="text-base-content/60">Coordinator:</span>
                      <span class="font-mono ml-1">{{ consumer.coordinator || '-' }}</span>
                    </div>
                    <div class="text-right">
                      <button class="btn btn-ghost btn-xs text-primary" @click="viewConsumerDetail(consumer.groupId)">View Details</button>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Tags Tab -->
            <div v-if="activeTab === 'tags'">
              <div class="flex items-center justify-between mb-3">
                <h4 class="font-semibold text-sm">Tags</h4>
                <button class="btn btn-primary btn-xs" @click="showAddTagModal">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                  </svg>
                  Add Tag
                </button>
              </div>
              <div v-if="!topicTags || topicTags.length === 0" class="text-center py-8 text-base-content/60">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 mx-auto mb-2 opacity-50">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.593l6.002-4.299a1.125 1.125 0 00.55-1.435l-4.299-6.002a2.25 2.25 0 00-.593-.952l-9.58-9.58z" />
                </svg>
                <p class="text-xs">No tags yet</p>
              </div>
              <div v-else class="flex flex-wrap gap-2">
                <div v-for="tag in topicTags" :key="tag.id" class="badge badge-lg gap-1 pr-1">
                  <span :class="['w-2 h-2 rounded-full', tagColorClass(tag.color)]"></span>
                  {{ tag.name }}
                  <button class="btn btn-ghost btn-xs btn-circle ml-1" @click="removeTag(tag.id)">×</button>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer Actions -->
          <div class="p-4 border-t border-base-content/10 bg-base-200 flex justify-between items-center">
            <div class="flex items-center gap-2 text-xs text-base-content/60">
              <span>Updated:</span>
              <span>{{ formatRelativeTime(topicDetailUpdatedAt) }}</span>
            </div>
            <div class="flex gap-2">
              <button class="btn btn-ghost btn-sm" @click="viewTopicMessagesFromDetail">View Messages</button>
              <button class="btn btn-error btn-sm btn-ghost" @click="confirmDeleteFromDetail">Delete Topic</button>
            </div>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeDetailModal">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Add Partitions Modal -->
    <Teleport to="body">
      <dialog ref="addPartitionsModalRef" class="modal">
        <div class="modal-box">
          <h3 class="font-bold text-lg mb-4">Add Partitions to {{ selectedTopicDetail?.name }}</h3>
          <form @submit.prevent="submitAddPartitions">
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text">Number of new partitions</span>
              </label>
              <input v-model.number="addPartitionsForm.count" type="number" min="1" max="100" class="input input-bordered" placeholder="1" required />
              <label class="label">
                <span class="label-text-alt">Current partitions: {{ selectedTopicDetail?.partitions.length || 0 }}</span>
                <span class="label-text-alt">After: {{ (selectedTopicDetail?.partitions.length || 0) + (addPartitionsForm.count || 0) }}</span>
              </label>
            </div>
            <div class="alert alert-warning p-2 mb-4">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
              </svg>
              <span class="text-xs">Note: Kafka does not support decreasing partitions or reassigning existing messages.</span>
            </div>
            <div class="modal-action">
              <button type="button" class="btn" @click="closeAddPartitionsModal">Cancel</button>
              <button type="submit" class="btn btn-primary" :disabled="addingPartitions">
                <span v-if="addingPartitions" class="loading loading-spinner loading-sm"></span>
                Add Partitions
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeAddPartitionsModal">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Edit Config Modal -->
    <Teleport to="body">
      <dialog ref="configModalRef" class="modal">
        <div class="modal-box">
          <h3 class="font-bold text-lg mb-4">Edit Topic Configuration</h3>
          <form @submit.prevent="submitConfigUpdate">
            <div class="max-h-80 overflow-auto space-y-2 mb-4">
              <div v-for="(_, key) in editConfigForm" :key="key" class="form-control">
                <label class="label">
                  <span class="label-text font-mono text-xs">{{ key }}</span>
                </label>
                <input v-model="editConfigForm[key]" type="text" class="input input-bordered input-sm" />
              </div>
            </div>
            <div class="modal-action">
              <button type="button" class="btn" @click="configModalRef?.close()">Cancel</button>
              <button type="submit" class="btn btn-primary" :disabled="updatingConfig">
                <span v-if="updatingConfig" class="loading loading-spinner loading-sm"></span>
                Save Changes
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop" @click="configModalRef?.close()">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Add Tag Modal -->
    <Teleport to="body">
      <dialog ref="addTagModalRef" class="modal">
        <div class="modal-box">
          <h3 class="font-bold text-lg mb-4">Add Tag to {{ selectedTopicDetail?.name }}</h3>
          <form @submit.prevent="submitAddTag">
            <div class="form-control mb-3">
              <label class="label">
                <span class="label-text">Tag Name</span>
              </label>
              <input v-model="addTagForm.name" type="text" class="input input-bordered" placeholder="e.g., production, critical" required />
            </div>
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text">Color</span>
              </label>
              <div class="flex gap-2">
                <button type="button" v-for="color in ['primary', 'secondary', 'accent', 'info', 'success', 'warning', 'error']" :key="color"
                  :class="['btn btn-xs', addTagForm.color === color ? 'btn-active' : 'btn-outline']"
                  @click="addTagForm.color = color">
                  <span :class="['badge badge-xs', `badge-${color}`]"></span>
                </button>
              </div>
            </div>
            <div class="modal-action">
              <button type="button" class="btn" @click="closeAddTagModal">Cancel</button>
              <button type="submit" class="btn btn-primary" :disabled="addingTag">
                <span v-if="addingTag" class="loading loading-spinner loading-sm"></span>
                Add Tag
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeAddTagModal">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- View Messages Modal using Teleport -->
    <Teleport to="body">
      <dialog ref="messagesModalRef" class="modal">
        <div class="modal-box modal-box-xl">
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="closeMessagesModal">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
          <h3 class="text-lg font-bold mb-4">
            {{ t.topics.viewMessages }}: {{ selectedMessageTopic }}
            <span v-if="selectedMessageCluster" class="text-sm font-normal text-base-content/60 ml-2">
              ({{ selectedMessageCluster }})
            </span>
          </h3>

          <!-- Filter Controls -->
          <div class="card bg-base-200 mb-4">
            <div class="card-body p-4">
              <div class="grid grid-cols-4 gap-3">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text-sm">{{ t.messages.partition }}</span>
                  </label>
                  <select v-model.number="messageFilters.partition" class="select select-bordered select-sm">
                    <option :value="undefined">{{ t.common.all }}</option>
                    <option v-for="p in availablePartitions" :key="p" :value="p">{{ p }}</option>
                  </select>
                </div>

                <div class="form-control">
                  <label class="label">
                    <span class="label-text-sm">{{ t.messages.maxMessages }}</span>
                  </label>
                  <input
                    v-model.number="messageFilters.limit"
                    type="number"
                    min="1"
                    max="1000"
                    class="input input-bordered input-sm"
                  />
                </div>

                <div class="form-control">
                  <label class="label">
                    <span class="label-text-sm">{{ t.messages.viewAs }}</span>
                  </label>
                  <select v-model="messageFilters.format" class="select select-bordered select-sm">
                    <option value="raw">{{ t.messages.raw }}</option>
                    <option value="json">{{ t.messages.json }}</option>
                    <option value="hex">{{ t.messages.hex }}</option>
                  </select>
                </div>

                <div class="form-control">
                  <label class="label">
                    <span class="label-text-sm">{{ t.common.search }}</span>
                  </label>
                  <input
                    v-model="messageFilters.search"
                    type="text"
                    :placeholder="t.messages.filter"
                    class="input input-bordered input-sm"
                    @keyup.enter="fetchMessages"
                  />
                </div>
              </div>

              <div class="grid grid-cols-4 gap-3 mt-3">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text-sm">{{ t.messages.startTime }}</span>
                  </label>
                  <input
                    v-model="messageFilters.startTime"
                    type="datetime-local"
                    class="input input-bordered input-sm"
                  />
                </div>

                <div class="form-control">
                  <label class="label">
                    <span class="label-text-sm">{{ t.messages.endTime }}</span>
                  </label>
                  <input
                    v-model="messageFilters.endTime"
                    type="datetime-local"
                    class="input input-bordered input-sm"
                  />
                </div>

                <div class="form-control col-span-2 flex items-end">
                  <button class="btn btn-primary" @click="fetchMessages" :disabled="loadingMessages">
                    <span v-if="loadingMessages" class="loading loading-spinner loading-sm mr-2"></span>
                    {{ t.messages.fetch }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Messages Table -->
          <div v-if="loadingMessages" class="flex justify-center py-12">
            <span class="loading loading-spinner loading-lg text-primary"></span>
          </div>

          <div v-else-if="messagesError" class="alert alert-error">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
            </svg>
            <span>{{ messagesError }}</span>
          </div>

          <div v-else-if="topicMessages.length === 0" class="flex flex-col items-center justify-center py-12 text-center">
            <div class="text-base-content/30 mb-4">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-12 h-12">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9.776c.112-.017.224-.026.336-.026h15.84c.112 0 .224.009.336.026m0-.026c.298.046.59.116.872.21l1.912.637a1.125 1.125 0 010 2.136l-1.912.637c-.282.094-.574.164-.872.21m-16.8.026c-.298.046-.59.116-.872.21l-1.912.637a1.125 1.125 0 010 2.136l1.912.637c.282.094.574.164-.872.21m12.078-6.053a3 3 0 00-2.974-2.723c-.624-.033-1.252.025-1.865.17-.64.151-1.247.382-1.808.683m6.647 1.873c.242.53.412 1.096.503 1.686m-12.078.026c.298-.046-.59-.116-.872-.21l1.912-.637a1.125 1.125 0 010-2.136l-1.912-.637c-.282-.094-.574-.164-.872-.21m16.8-.026c-.298-.046-.59-.116-.872-.21l-1.912-.637a1.125 1.125 0 010-2.136l1.912-.637c.282.094.574.164-.872.21" />
              </svg>
            </div>
            <p class="text-base-content/60">{{ t.messages.noMessages }}</p>
          </div>

          <div v-else class="overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th class="py-1 px-2">{{ t.messages.partition }}</th>
                  <th class="py-1 px-2">{{ t.messages.offset }}</th>
                  <th class="py-1 px-2">{{ t.messages.timestampLabel }}</th>
                  <th class="py-1 px-2">{{ t.messages.key }}</th>
                  <th class="py-1 px-2">{{ t.messages.value }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="msg in topicMessages" :key="`${msg.partition}-${msg.offset}`" class="hover">
                  <td class="py-1 px-2"><span class="font-mono text-xs">{{ msg.partition }}</span></td>
                  <td class="py-1 px-2"><span class="font-mono text-xs">{{ msg.offset }}</span></td>
                  <td class="py-1 px-2"><span class="text-base-content/60 text-xs">{{ formatTimestamp(msg.timestamp) }}</span></td>
                  <td class="max-w-xs truncate py-1 px-2" :title="msg.key || ''">
                    <span class="font-mono text-xs">{{ formatMessageContent(msg.key) }}</span>
                  </td>
                  <td class="max-w-md truncate py-1 px-2" :title="msg.value || ''">
                    <span class="font-mono text-xs">{{ formatMessageContent(msg.value) }}</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="modal-action mt-6">
            <button class="btn" @click="closeMessagesModal">{{ t.common.cancel }}</button>
            <button class="btn btn-primary" @click="exportMessages" :disabled="topicMessages.length === 0">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
              </svg>
              {{ t.topics.exportData }}
            </button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeMessagesModal">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
// 保持原有逻辑不变，只更新样式
import { ref, reactive, computed, onMounted, watch, type ComponentPublicInstance } from 'vue';
import { useRoute, onBeforeRouteUpdate, useRouter } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';
import type { TopicDetailResponse, MessageRecord } from '@/types/api';

// 定义本地类型
interface TopicItem {
  name: string;
  cluster: string;
  partition_count?: number;
  replication_factor?: number;
  status?: string;
}

const route = useRoute();
const router = useRouter();
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

const selectedClusterIds = computed(() => clusterStore.selectedClusterIds);

const clusterParam = computed(() => {
  const val = route.query.cluster;
  return Array.isArray(val) ? val[0] : (val || '');
});

const actionParam = computed(() => {
  const val = route.query.action;
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

const creating = ref(false);
const targetCluster = ref<string | undefined>(undefined);

const newTopic = reactive({
  name: '',
  num_partitions: 3,
  replication_factor: 1,
});

const selectedTopicDetail = ref<TopicDetailResponse | null>(null);
const selectedTopicCluster = ref<string>('');
const currentCluster = ref<string>('');

// 新增：Topic 详情增强功能相关变量
const activeTab = ref<'partitions' | 'config' | 'consumers' | 'tags'>('partitions');
const topicConfig = ref<Record<string, string> | null>(null);
const editConfigForm = ref<Record<string, string>>({});
const updatingConfig = ref(false);
const consumers = ref<Array<{ groupId: string; state?: string; members?: any[]; totalLag?: number; coordinator?: string }>>([]);
const consumersLoading = ref(false);
const topicTags = ref<Array<{ id: number; name: string; color: string }>>([]);
const addTagForm = reactive({ name: '', color: 'primary' });
const addingTag = ref(false);
const addPartitionsForm = reactive({ count: 1 });
const addingPartitions = ref(false);
const topicDetailUpdatedAt = ref<number>(Date.now());

const addPartitionsModalRef = ref<HTMLDialogElement>();
const configModalRef = ref<HTMLDialogElement>();
const addTagModalRef = ref<HTMLDialogElement>();

const selectedMessageTopic = ref<string>('');
const selectedMessageCluster = ref<string>('');
const topicMessages = ref<MessageRecord[]>([]);
const availablePartitions = ref<number[]>([]);
const loadingMessages = ref(false);
const messagesError = ref<string | null>(null);
const messageFilters = reactive({
  partition: undefined as number | undefined,
  limit: 100,
  format: 'raw' as 'raw' | 'json' | 'hex',
  search: '',
  startTime: '',
  endTime: '',
});

const createModalRef = ref<HTMLDialogElement>();
const detailModalRef = ref<HTMLDialogElement>();
const messagesModalRef = ref<HTMLDialogElement>();

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

function handleScroll(event: Event) {
  const target = event.target as HTMLElement;
  scrollTop.value = target.scrollTop;
  containerHeight.value = target.clientHeight;
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
  // 检查是否有 action=create 参数，打开创建模态框
  const action = Array.isArray(to.query.action) ? to.query.action[0] : (to.query.action || '');
  if (action === 'create') {
    openCreateModal(true); // 传入 true，避免重复设置 URL 参数
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
      error.value = (e as { message: string }).message;
    } finally {
      loading.value = false;
    }
    return;
  }

  if (selectedClusterIds.value.length === 0) {
    loading.value = false;
    return;
  }

  topicsByCluster.value = {};
  allTopicsList.value = [];

  try {
    const promises = selectedClusterIds.value.map(async (clusterId) => {
      try {
        const topicNames = await apiClient.getTopics(clusterId);
        const topics: TopicItem[] = topicNames.map((name) => ({
          name,
          cluster: clusterId,
          partition_count: undefined,
        }));
        topicsByCluster.value[clusterId] = topics;
      } catch (e) {
        topicsByCluster.value[clusterId] = [];
      }
    });

    await Promise.all(promises);
    updateAllTopicsList();
  } catch (e) {
    error.value = (e as { message: string }).message;
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

function openCreateModal(skipUrlUpdate = false) {
  // 设置 targetCluster 的值
  if (selectedClusterIds.value.length === 1) {
    // 只有一个集群被选中时，使用该集群
    targetCluster.value = selectedClusterIds.value[0];
  } else if (selectedClusterIds.value.length > 1) {
    // 多个集群被选中时，使用第一个
    targetCluster.value = selectedClusterIds.value[0];
  } else if (clusterParam.value) {
    // 从路由参数中获取集群
    targetCluster.value = clusterParam.value;
  }

  newTopic.name = '';
  newTopic.num_partitions = 3;
  newTopic.replication_factor = 1;
  // 设置 URL 参数（如果通过按钮点击打开）
  if (!skipUrlUpdate) {
    router.push({ query: { ...route.query, action: 'create', cluster: targetCluster.value || undefined } });
  }
  createModalRef.value?.showModal();
}

// 双击 Topic 时触发：通知父组件展开并选中左侧树中的对应 Topic
function selectTopicInTree(clusterName: string, topic: TopicItem) {
  // 触发自定义事件，由 ModernLayout 捕获并处理
  window.dispatchEvent(new CustomEvent('select-topic-in-tree', {
    detail: { topicName: topic.name, clusterName }
  }));
}

function closeCreateModal() {
  createModalRef.value?.close();
  // 清除 URL 中的 action 参数
  router.replace({ query: { ...route.query, action: undefined } });
}

async function handleCreate() {
  const cluster = targetCluster.value;
  if (!cluster) return;

  creating.value = true;
  try {
    await apiClient.createTopic(cluster, {
      name: newTopic.name,
      num_partitions: newTopic.num_partitions,
      replication_factor: newTopic.replication_factor,
    });
    showSuccess('Topic created successfully');
    closeCreateModal();
    fetchTopics();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    creating.value = false;
  }
}

async function viewTopicDetail(clusterId: string, topic: TopicItem) {
  try {
    selectedTopicCluster.value = clusterId;
    selectedTopicDetail.value = await apiClient.getTopicDetail(clusterId, topic.name);
    activeTab.value = 'partitions';
    topicDetailUpdatedAt.value = Date.now();

    // 并行加载配置和标签
    fetchTopicConfig(clusterId, topic.name);
    fetchTopicTags(clusterId, topic.name);

    detailModalRef.value?.showModal();
  } catch (e) {
    showError((e as { message: string }).message);
  }
}

function closeDetailModal() {
  detailModalRef.value?.close();
  selectedTopicDetail.value = null;
  selectedTopicCluster.value = '';
  topicConfig.value = null;
  consumers.value = [];
  topicTags.value = [];
}

// 获取 Topic 配置
async function fetchTopicConfig(clusterId: string, topicName: string) {
  try {
    topicConfig.value = await apiClient.getTopicConfig(clusterId, topicName);
  } catch (e) {
    console.error('Failed to fetch topic config:', e);
    topicConfig.value = {};
  }
}

// 获取消费者组列表
async function fetchConsumers() {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;

  consumersLoading.value = true;
  try {
    const groups = await apiClient.getConsumerGroups(selectedTopicCluster.value);
    // 过滤出消费该 topic 的 consumer groups
    const topicName = selectedTopicDetail.value.name;
    const filteredGroups = [];

    for (const group of groups) {
      try {
        const detail = await apiClient.getConsumerGroupDetail(selectedTopicCluster.value, group.name);
        // 检查该 consumer group 是否订阅了当前 topic
        const offsets = detail.offsets || [];
        const hasTopic = offsets.some((o: any) => o.topic === topicName);
        if (hasTopic) {
          const topicOffsets = offsets.filter((o: any) => o.topic === topicName);
          const totalLag = topicOffsets.reduce((sum: number, o: any) => sum + (o.lag || 0), 0);
          filteredGroups.push({
            groupId: group.name,
            state: detail.state,
            members: detail.members,
            totalLag,
          });
        }
      } catch (e) {
        // 忽略单个 consumer group 获取失败
      }
    }

    consumers.value = filteredGroups;
  } catch (e) {
    console.error('Failed to fetch consumers:', e);
  } finally {
    consumersLoading.value = false;
  }
}

// 查看 Consumer Group 详情
function viewConsumerDetail(groupId: string) {
  // 跳转到 Consumer Groups 页面
  const route = `/consumer-groups?cluster=${selectedTopicCluster.value}&group=${groupId}`;
  window.location.href = route;
}

// 获取 Topic 标签
async function fetchTopicTags(clusterId: string, topicName: string) {
  try {
    topicTags.value = await apiClient.getTopicTags(clusterId, topicName);
  } catch (e) {
    topicTags.value = [];
  }
}

// 添加标签
function showAddTagModal() {
  addTagForm.name = '';
  addTagForm.color = 'primary';
  addTagModalRef.value?.showModal();
}

function closeAddTagModal() {
  addTagModalRef.value?.close();
}

async function submitAddTag() {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;

  addingTag.value = true;
  try {
    const tag = await apiClient.createTopicTag(
      selectedTopicCluster.value,
      selectedTopicDetail.value.name,
      addTagForm.name,
      addTagForm.color
    );
    topicTags.value.push(tag);
    closeAddTagModal();
    showSuccess('Tag added successfully');
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    addingTag.value = false;
  }
}

// 删除标签
async function removeTag(tagId: number) {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;

  try {
    await apiClient.deleteTopicTag(selectedTopicCluster.value, selectedTopicDetail.value.name, tagId);
    topicTags.value = topicTags.value.filter(t => t.id !== tagId);
    showSuccess('Tag removed successfully');
  } catch (e) {
    showError((e as { message: string }).message);
  }
}

// 标签颜色 class
function tagColorClass(color: string): string {
  const colors: Record<string, string> = {
    primary: 'bg-primary',
    secondary: 'bg-secondary',
    accent: 'bg-accent',
    info: 'bg-info',
    success: 'bg-success',
    warning: 'bg-warning',
    error: 'bg-error',
  };
  return colors[color] || 'bg-primary';
}

function closeAddPartitionsModal() {
  addPartitionsModalRef.value?.close();
}

async function submitAddPartitions() {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;
  if (addPartitionsForm.count < 1) return;

  addingPartitions.value = true;
  try {
    await apiClient.addPartitions(
      selectedTopicCluster.value,
      selectedTopicDetail.value.name,
      addPartitionsForm.count
    );
    showSuccess('Partitions added successfully');
    closeAddPartitionsModal();
    // 重新加载详情
    viewTopicDetail(selectedTopicCluster.value, { name: selectedTopicDetail.value.name, cluster: selectedTopicCluster.value });
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    addingPartitions.value = false;
  }
}

// 更新配置
function editConfig() {
  if (!topicConfig.value) return;
  editConfigForm.value = { ...topicConfig.value };
  configModalRef.value?.showModal();
}

async function submitConfigUpdate() {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;

  updatingConfig.value = true;
  try {
    await apiClient.updateTopicConfig(
      selectedTopicCluster.value,
      selectedTopicDetail.value.name,
      editConfigForm.value
    );
    topicConfig.value = { ...editConfigForm.value };
    configModalRef.value?.close();
    showSuccess('Configuration updated successfully');
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    updatingConfig.value = false;
  }
}

// 从详情页查看消息
function viewTopicMessagesFromDetail() {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;
  viewTopicMessages(selectedTopicCluster.value, {
    name: selectedTopicDetail.value.name,
    cluster: selectedTopicCluster.value,
  });
}

// 从详情页删除
function confirmDeleteFromDetail() {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;
  confirmDelete(selectedTopicCluster.value, selectedTopicDetail.value.name);
}

// 格式化数字
function formatNumber(num: number): string {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1) + 'M';
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'K';
  }
  return num.toString();
}

// 格式化相对时间
function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 24) {
    return new Date(timestamp).toLocaleDateString();
  }
  if (hours > 0) {
    return `${hours}h ago`;
  }
  if (minutes > 0) {
    return `${minutes}m ago`;
  }
  return 'Just now';
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

async function viewTopicMessages(clusterId: string, topic: TopicItem) {
  selectedMessageCluster.value = clusterId;
  selectedMessageTopic.value = topic.name;
  topicMessages.value = [];
  messagesError.value = null;
  messageFilters.partition = undefined;
  messageFilters.search = '';
  messageFilters.startTime = '';
  messageFilters.endTime = '';

  try {
    const topicDetail = await apiClient.getTopicDetail(clusterId, topic.name);
    availablePartitions.value = topicDetail.partitions.map(p => p.id);
  } catch (e) {
    availablePartitions.value = [];
  }

  messagesModalRef.value?.showModal();
  fetchMessages();
}

async function fetchMessages() {
  if (!selectedMessageCluster.value || !selectedMessageTopic.value) return;

  loadingMessages.value = true;
  messagesError.value = null;
  topicMessages.value = [];

  try {
    const params: {
      partition?: number;
      max_messages?: number;
      limit?: number;
      search?: string;
      start_time?: number;
      end_time?: number;
    } = {
      limit: messageFilters.limit,
      max_messages: messageFilters.limit * 5,
    };

    if (messageFilters.partition !== undefined) {
      params.partition = messageFilters.partition;
    }

    if (messageFilters.search) {
      params.search = messageFilters.search;
    }

    if (messageFilters.startTime) {
      params.start_time = new Date(messageFilters.startTime).getTime();
    }

    if (messageFilters.endTime) {
      params.end_time = new Date(messageFilters.endTime).getTime();
    }

    const messages = await apiClient.getMessages(
      selectedMessageCluster.value,
      selectedMessageTopic.value,
      params
    );
    topicMessages.value = messages;
  } catch (e) {
    messagesError.value = (e as { message: string }).message;
  } finally {
    loadingMessages.value = false;
  }
}

function closeMessagesModal() {
  messagesModalRef.value?.close();
  selectedMessageTopic.value = '';
  selectedMessageCluster.value = '';
  topicMessages.value = [];
  messagesError.value = null;
}

function formatTimestamp(timestamp?: number): string {
  if (!timestamp) return '-';
  const date = new Date(timestamp);
  return date.toLocaleString();
}

function formatMessageContent(content: string | undefined): string {
  if (!content) return '-';

  if (messageFilters.format === 'json') {
    try {
      const parsed = JSON.parse(content);
      return JSON.stringify(parsed, null, 2).substring(0, 100);
    } catch {
      return content.substring(0, 100);
    }
  } else if (messageFilters.format === 'hex') {
    try {
      const bytes = new TextEncoder().encode(content);
      return Array.from(bytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ')
        .substring(0, 100);
    } catch {
      return content.substring(0, 100);
    }
  }

  return content.substring(0, 100);
}

async function exportMessages() {
  if (!selectedMessageCluster.value || !selectedMessageTopic.value) return;

  try {
    const params: {
      partition?: number;
      max_messages?: number;
      format?: 'json' | 'csv' | 'text';
      start_time?: number;
      end_time?: number;
    } = {
      format: 'json',
      max_messages: messageFilters.limit,
    };

    if (messageFilters.partition !== undefined) {
      params.partition = messageFilters.partition;
    }

    if (messageFilters.startTime) {
      params.start_time = new Date(messageFilters.startTime).getTime();
    }

    if (messageFilters.endTime) {
      params.end_time = new Date(messageFilters.endTime).getTime();
    }

    const result = await apiClient.exportMessages(
      selectedMessageCluster.value,
      selectedMessageTopic.value,
      params
    );

    // 确保返回的数据包含 messages 数组
    const messagesToExport = result?.messages || [];

    // 使用 Tauri API 保存文件
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { writeTextFile } = await import('@tauri-apps/plugin-fs');

    const filePath = await save({
      filters: [{
        name: 'JSON Files',
        extensions: ['json']
      }],
      defaultPath: `${selectedMessageTopic.value}_export_${Date.now()}.json`
    });

    if (filePath) {
      await writeTextFile(filePath, JSON.stringify(messagesToExport, null, 2));
      showSuccess('Export successful');
    }
  } catch (e) {
    showError(`Export failed: ${(e as { message: string }).message}`);
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
    await fetchTopics();
    showSuccess('Topics refreshed successfully');
  } catch (e) {
    showError(`Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshing.value = false;
  }
}

onMounted(() => {
  if (clusterParam.value) {
    fetchTopics();
  } else if (selectedClusterIds.value.length > 0) {
    fetchTopics();
  }
  // 检查是否有 action=create 参数，打开创建模态框
  if (actionParam.value === 'create') {
    openCreateModal(true); // 传入 true，避免重复设置 URL 参数
  }
});
</script>
