<template>
  <div class="p-6">
    <!-- Page Header -->
    <div class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold flex items-center gap-3">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-10 h-10">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
            </svg>
            {{ t.topics.title }}
          </h1>
          <p class="text-base-content/60 mt-2 text-lg">
            <span v-if="clusterParam">{{ t.dashboard.clusters }}: <span class="font-medium">{{ clusterParam }}</span></span>
            <span v-else>{{ t.topics.description }}</span>
          </p>
        </div>
        <div class="flex gap-2">
          <button
            v-if="!clusterParam && selectedClusterIds.length > 0"
            class="btn btn-sm"
            :class="viewMode === 'by-cluster' ? 'btn-primary' : 'btn-outline'"
            @click="viewMode = 'by-cluster'"
          >
            {{ t.dashboard.byCluster }}
          </button>
          <button
            v-if="!clusterParam && selectedClusterIds.length > 0"
            class="btn btn-sm"
            :class="viewMode === 'all-topics' ? 'btn-primary' : 'btn-outline'"
            @click="viewMode = 'all-topics'"
          >
            {{ t.topics.allTopics }}
          </button>
          <button
            class="btn btn-sm btn-outline"
            @click="refreshAllTopics"
            :disabled="refreshing || selectedClusterIds.length === 0"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4" :class="{ 'animate-spin': refreshing }">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            {{ t.common.refresh }}
          </button>
          <button
            class="btn btn-sm btn-primary"
            @click="openCreateModal"
            :disabled="selectedClusterIds.length === 0 && !clusterParam"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
            {{ t.topics.createTopic }}
          </button>
        </div>
      </div>
    </div>

    <!-- No cluster selected -->
    <div v-if="!clusterParam && selectedClusterIds.length === 0" class="flex flex-col items-center justify-center py-12 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-4">{{ t.topics.description }}</p>
    </div>

    <!-- Single cluster view (from URL param) -->
    <div v-else-if="clusterParam && !filteredClusterTopics.length && !loading" class="flex flex-col items-center justify-center py-12 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-4">{{ t.topics.description }}</p>
      <button class="btn btn-primary" @click="openCreateModal">{{ t.topics.createTopic }}</button>
    </div>

    <!-- Loading state -->
    <div v-else-if="loading" class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg text-primary"></span>
      <p class="ml-4 text-base-content/60">{{ t.common.loading }}...</p>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
      </svg>
      <span>{{ error }}</span>
    </div>

    <!-- Single cluster view (from URL param) -->
    <div v-else-if="clusterParam && filteredClusterTopics.length > 0" class="card glass gradient-border shadow-xl">
      <div class="overflow-x-auto">
        <!-- Search Bar -->
        <div class="p-4">
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
                getClusterHealth(clusterName)?.healthy ? 'bg-success animate-pulse' : 'bg-error'
              ]"
            ></div>
            <h3 class="text-lg font-semibold">{{ clusterName }}</h3>
            <span class="text-sm text-base-content/60">{{ clusterTopics.length }} topics</span>
          </div>
          <div class="flex gap-2">
            <button
              class="btn btn-primary btn-xs"
              @click="createCluster = clusterName; openCreateModal()"
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
          <div class="overflow-x-auto">
            <table class="table">
              <thead>
                <tr>
                  <th>{{ t.topics.topicName }}</th>
                  <th>{{ t.topics.partitions }}</th>
                  <th>{{ t.common.actions }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="topic in clusterTopics" :key="topic.name" @dblclick="selectTopicInTree(clusterName, topic)" class="hover cursor-pointer">
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
              </tbody>
            </table>
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
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>{{ t.dashboard.clusters }}</th>
                <th>{{ t.topics.topicName }}</th>
                <th>{{ t.topics.partitions }}</th>
                <th>{{ t.common.actions }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in filteredAllTopicsList" :key="`${item.cluster}-${item.name}`" @dblclick="selectTopicInTree(item.cluster, item)" class="hover cursor-pointer">
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
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- Create Topic Modal using Teleport -->
    <Teleport to="body">
      <dialog ref="createModalRef" class="modal">
        <div class="modal-box">
          <form method="dialog">
            <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2">✕</button>
          </form>
          <h3 class="text-lg font-bold mb-4">{{ t.topics.createTopic }}</h3>
          <form @submit.prevent="handleCreate" class="space-y-4">
            <div v-if="!createCluster" class="form-control">
              <label class="label">
                <span class="label-text">{{ t.dashboard.clusters }}</span>
              </label>
              <select v-model="targetCluster" class="select select-bordered" required>
                <option v-for="cluster in selectedClusterIds" :key="cluster" :value="cluster">
                  {{ cluster }}
                </option>
              </select>
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
        <div class="modal-box modal-box-lg">
          <form method="dialog">
            <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2">✕</button>
          </form>
          <h3 class="text-lg font-bold mb-2">
            Topic: {{ selectedTopicDetail?.name }}
            <span v-if="selectedTopicCluster" class="text-sm font-normal text-base-content/60 ml-2">
              ({{ selectedTopicCluster }})
            </span>
          </h3>
          <div class="flex items-center gap-3 mb-4">
            <div class="badge badge-neutral">{{ selectedTopicDetail?.partitions.length }} partitions</div>
          </div>
          <div class="overflow-x-auto">
            <table class="table">
              <thead>
                <tr>
                  <th>Partition ID</th>
                  <th>Leader</th>
                  <th>Replicas</th>
                  <th>ISR</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="partition in selectedTopicDetail?.partitions" :key="partition.id">
                  <td><span class="font-mono">{{ partition.id }}</span></td>
                  <td><span class="font-mono">{{ partition.leader }}</span></td>
                  <td><span class="font-mono text-base-content/60">{{ partition.replicas.join(', ') }}</span></td>
                  <td><span class="font-mono text-base-content/60">{{ partition.isr.join(', ') }}</span></td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="modal-action mt-6">
            <button class="btn" @click="closeDetailModal">{{ t.common.cancel }}</button>
            <button class="btn btn-primary" @click="addPartitions">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
              </svg>
              {{ t.topics.partitions }} +
            </button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- View Messages Modal using Teleport -->
    <Teleport to="body">
      <dialog ref="messagesModalRef" class="modal">
        <div class="modal-box modal-box-xl">
          <form method="dialog">
            <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2">✕</button>
          </form>
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
            <table class="table">
              <thead>
                <tr>
                  <th>{{ t.messages.partition }}</th>
                  <th>{{ t.messages.offset }}</th>
                  <th>{{ t.messages.timestampLabel }}</th>
                  <th>{{ t.messages.key }}</th>
                  <th>{{ t.messages.value }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="msg in topicMessages" :key="`${msg.partition}-${msg.offset}`">
                  <td><span class="font-mono">{{ msg.partition }}</span></td>
                  <td><span class="font-mono">{{ msg.offset }}</span></td>
                  <td><span class="text-base-content/60">{{ formatTimestamp(msg.timestamp) }}</span></td>
                  <td class="max-w-xs truncate" :title="msg.key || ''">
                    <span class="font-mono text-sm">{{ formatMessageContent(msg.key) }}</span>
                  </td>
                  <td class="max-w-md truncate" :title="msg.value || ''">
                    <span class="font-mono text-sm">{{ formatMessageContent(msg.value) }}</span>
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
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
// 保持原有逻辑不变，只更新样式
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useRoute, onBeforeRouteUpdate } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
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
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const selectedClusterIds = computed(() => clusterStore.selectedClusterIds);

const clusterParam = computed(() => {
  const val = route.query.cluster;
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
const createCluster = ref<string | undefined>(undefined);
const targetCluster = ref<string | undefined>(undefined);

const newTopic = reactive({
  name: '',
  num_partitions: 3,
  replication_factor: 1,
});

const selectedTopicDetail = ref<TopicDetailResponse | null>(null);
const selectedTopicCluster = ref<string>('');
const currentCluster = ref<string>('');

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
  console.log('[TopicsView] fetchTopics called, clusterParam:', clusterParam.value);
  loading.value = true;
  error.value = null;

  if (clusterParam.value) {
    console.log('[TopicsView] Fetching from API:', `/api/clusters/${clusterParam.value}/topics`);
    try {
      const topicNames = await apiClient.getTopics(clusterParam.value);
      console.log('[TopicsView] Topics received:', topicNames);
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

function openCreateModal() {
  if (selectedClusterIds.value.length === 1) {
    targetCluster.value = selectedClusterIds.value[0];
  } else if (createCluster.value) {
    targetCluster.value = createCluster.value;
  }
  newTopic.name = '';
  newTopic.num_partitions = 3;
  newTopic.replication_factor = 1;
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
  createCluster.value = undefined;
}

async function handleCreate() {
  const cluster = targetCluster.value || createCluster.value;
  if (!cluster) return;

  creating.value = true;
  try {
    await apiClient.createTopic(cluster, {
      name: newTopic.name,
      num_partitions: newTopic.num_partitions,
      replication_factor: newTopic.replication_factor,
    });
    closeCreateModal();
    fetchTopics();
  } catch (e) {
    alert((e as { message: string }).message);
  } finally {
    creating.value = false;
  }
}

async function viewTopicDetail(clusterId: string, topic: TopicItem) {
  try {
    selectedTopicCluster.value = clusterId;
    selectedTopicDetail.value = await apiClient.getTopicDetail(clusterId, topic.name);
    detailModalRef.value?.showModal();
  } catch (e) {
    alert((e as { message: string }).message);
  }
}

function closeDetailModal() {
  detailModalRef.value?.close();
  selectedTopicDetail.value = null;
  selectedTopicCluster.value = '';
}

async function addPartitions() {
  if (!selectedTopicCluster.value || !selectedTopicDetail.value) return;

  const newPartitions = prompt('Enter number of new partitions to add:');
  if (!newPartitions) return;

  try {
    await apiClient.addPartitions(
      selectedTopicCluster.value,
      selectedTopicDetail.value.name,
      parseInt(newPartitions)
    );
    closeDetailModal();
    fetchTopics();
  } catch (e) {
    alert((e as { message: string }).message);
  }
}

async function confirmDelete(clusterId: string, topicName: string) {
  if (confirm(`Are you sure you want to delete topic "${topicName}" from cluster "${clusterId}"?`)) {
    try {
      await apiClient.deleteTopic(clusterId, topicName);
      fetchTopics();
    } catch (e) {
      alert((e as { message: string }).message);
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

    const blob = await apiClient.exportMessages(
      selectedMessageCluster.value,
      selectedMessageTopic.value,
      params
    );

    const url = window.URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${selectedMessageTopic.value}_export_${Date.now()}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
  } catch (e) {
    alert(`Export failed: ${(e as { message: string }).message}`);
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
  } catch (e) {
    alert(`Refresh failed: ${(e as { message: string }).message}`);
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
});
</script>
