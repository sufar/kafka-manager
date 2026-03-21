<template>
  <div class="p-3 relative overflow-hidden">
    <!-- Animated background particles -->
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
              <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
            </svg>
            {{ t.clusters.title }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">{{ t.clusters.description }}</p>
        </div>
        <div class="flex items-center gap-2">
          <button class="btn btn-ghost btn-sm" @click="openManageGroupsModal">
            {{ t.clusters.addGroup }}
          </button>
          <button class="btn btn-primary btn-sm" @click="openCreateModal">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
            {{ t.clusters.addCluster }}
          </button>
        </div>
      </div>
      <!-- Group Selector -->
      <div v-if="clusterStore.groups.length > 0" class="flex items-center gap-1 overflow-x-auto scrollbar-hide py-2 mt-2 relative">
        <span class="text-sm font-medium text-base-content/60 mr-2 flex-shrink-0">{{ t.clusters.group }}:</span>
        <button
          class="btn btn-xs btn-ghost px-1 flex-shrink-0 hover:bg-base-200"
          @click="scrollGroups(-200)"
          :title="t.clusters.scrollLeft"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5" />
          </svg>
        </button>
        <div ref="groupSelectorRef" class="flex items-center gap-1 overflow-x-auto scrollbar-hide flex-1" @wheel="handleHorizontalScroll">
          <button
            class="btn btn-xs btn-ghost whitespace-nowrap flex-shrink-0"
            :class="{ 'btn-active': selectedGroupId === null }"
            @click="selectGroup(null)"
          >
            {{ t.common.all }}
          </button>
          <button
            v-for="group in clusterStore.groups"
            :key="group.id"
            class="btn btn-xs btn-ghost whitespace-nowrap flex-shrink-0"
            :class="{ 'btn-active': selectedGroupId === group.id }"
            @click="selectGroup(group.id)"
          >
            {{ group.name }}
          </button>
        </div>
        <button
          class="btn btn-xs btn-ghost px-1 flex-shrink-0 hover:bg-base-200"
          @click="scrollGroups(200)"
          :title="t.clusters.scrollRight"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3">
            <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex justify-center items-center py-8">
      <div class="flex flex-col items-center">
        <span class="loading loading-spinner loading-md text-primary"></span>
        <p class="mt-2 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
      </div>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="flex flex-col items-center justify-center py-8 text-center">
      <div class="text-base-content/40 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-1">{{ t.clusters.connectionError }}</h3>
      <p class="text-base-content/60 mb-3 max-w-md text-sm">{{ error }}</p>
      <div class="flex gap-2">
        <button class="btn btn-primary btn-sm" @click="refreshClusters">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
          </svg>
          {{ t.clusters.retry }}
        </button>
        <button class="btn btn-outline btn-sm" @click="openCreateModal">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          {{ t.clusters.addCluster }}
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div v-else-if="clusters.length === 0" class="flex flex-col items-center justify-center py-8 text-center">
      <div class="text-base-content/40 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-1">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-3 text-sm">{{ t.clusters.description }}</p>
      <button class="btn btn-primary btn-sm" @click="openCreateModal">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        {{ t.clusters.addCluster }}
      </button>
    </div>

    <!-- Clusters Grid -->
    <div v-else class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="cluster in filteredClusters"
        :key="cluster.id"
        class="card glass gradient-border hover:shadow-xl hover:-translate-y-0.5 transition-all duration-200"
      >
        <div class="flex items-center justify-between p-3 border-b border-base-content/10">
          <div class="flex items-center gap-2">
            <div class="flex items-center justify-center w-8 h-8 rounded-lg bg-primary/10 glow-primary text-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
              </svg>
            </div>
            <div>
              <h3 class="font-semibold text-sm">{{ cluster.name }}</h3>
              <p class="text-[10px] text-base-content/60">{{ t.clusters.createdDate }} {{ formatDate(cluster.created_at) }}</p>
            </div>
          </div>
          <div class="flex items-center gap-1">
            <button
              class="btn btn-xs btn-ghost h-auto p-1 min-h-0"
              @click="editCluster(cluster)"
              :title="t.clusters.editCluster"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" />
              </svg>
            </button>
            <button
              class="btn btn-xs btn-ghost h-auto p-1 min-h-0 text-error hover:bg-error/10"
              @click="confirmDelete(cluster)"
              :title="t.common.delete"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
              </svg>
            </button>
            <div
              class="badge gap-1 badge-xs ml-2"
              :class="{
                'badge-success': getConnectionStatus(cluster.name)?.status === 'connected',
                'badge-error': getConnectionStatus(cluster.name)?.status === 'error',
                'badge-ghost': getConnectionStatus(cluster.name)?.status === 'disconnected',
              }"
            >
              <div
                class="w-1.5 h-1.5 rounded-full"
                :class="{
                  'bg-success animate-pulse': getConnectionStatus(cluster.name)?.status === 'connected',
                  'bg-error': getConnectionStatus(cluster.name)?.status === 'error',
                }"
              ></div>
              {{ getConnectionStatus(cluster.name)?.status || t.clusters.unknown }}
            </div>
          </div>
        </div>

        <div class="card-body p-3">
          <div class="mb-2">
            <div class="text-[10px] uppercase tracking-wider text-base-content/60 mb-1">{{ t.clusters.brokersLabel }}</div>
            <div class="text-xs font-mono truncate">{{ cluster.brokers }}</div>
          </div>
          <div class="mb-2">
            <div class="text-[10px] uppercase tracking-wider text-base-content/60 mb-1">{{ t.clusters.timeoutsLabel }}</div>
            <div class="text-xs">
              <div>Request: <span class="font-mono">{{ cluster.request_timeout_ms }}ms</span></div>
              <div>Operation: <span class="font-mono">{{ cluster.operation_timeout_ms }}ms</span></div>
            </div>
          </div>

          <div v-if="getConnectionStatus(cluster.name)?.error_message" class="alert alert-error py-1.5 px-2 mt-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
            </svg>
            <span class="text-xs truncate">{{ getConnectionStatus(cluster.name)?.error_message }}</span>
          </div>
        </div>

        <div class="card-actions justify-start p-2 bg-base-200 gap-1">
          <button
            class="btn btn-xs btn-outline flex items-center gap-1.5"
            @click="testConnection(cluster.id)"
            :disabled="testing.has(cluster.id)"
          >
            <span v-if="testing.has(cluster.id)" class="loading loading-spinner loading-xs"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z" />
            </svg>
            {{ t.clusters.test }}
          </button>
          <button
            class="btn btn-xs btn-outline flex items-center gap-1.5"
            @click="reconnectCluster(cluster.name)"
            :disabled="reconnecting.has(cluster.name)"
          >
            <span v-if="reconnecting.has(cluster.name)" class="loading loading-spinner loading-xs"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            {{ t.clusters.reconnect }}
          </button>
          <button
            class="btn btn-xs btn-outline flex items-center gap-1.5"
            @click="viewClusterTopics(cluster.name)"
            :disabled="refreshingTopics.has(cluster.name)"
            :title="t.clusters.viewTopics"
          >
            <span v-if="refreshingTopics.has(cluster.name)" class="loading loading-spinner loading-xs"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
            </svg>
            {{ t.clusters.viewTopicsLink }}
          </button>
          <button
            class="btn btn-xs btn-outline flex items-center gap-1.5"
            @click="refreshClusterTopics(cluster.name)"
            :disabled="refreshingTopics.has(cluster.name)"
            :title="t.clusters.refreshTopics || '刷新 Topic'"
          >
            <span v-if="refreshingTopics.has(cluster.name)" class="loading loading-spinner loading-xs"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
            </svg>
            {{ t.clusters.refreshTopics || '刷新 Topic' }}
          </button>
          <button
            class="btn btn-xs btn-ghost"
            @click="disconnectCluster(cluster.name)"
            :disabled="disconnecting.has(cluster.name)"
          >
            {{ t.clusters.disconnect }}
          </button>
        </div>
      </div>
    </div>

    <!-- Create/Edit Modal using Teleport and DaisyUI modal -->
    <Teleport to="body">
      <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle">
        <div class="modal-box">
          <!-- close button -->
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="closeModal">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
          <h3 class="font-bold text-xl mt-2 mb-4">{{ editingCluster ? t.clusters.editCluster : t.clusters.createCluster }}</h3>
          <form @submit.prevent="handleSubmit" class="flex flex-col gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ t.clusters.clusterName }}</span>
              </label>
              <input
                v-model="formData.name"
                type="text"
                placeholder="my-cluster"
                class="input input-bordered w-full"
                required
              />
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ t.clusters.brokers }}</span>
              </label>
              <input
                v-model="formData.brokers"
                type="text"
                placeholder="localhost:9092,localhost:9093"
                class="input input-bordered w-full"
                required
              />
              <label class="label">
                <span class="label-text-alt text-base-content/60">{{ t.clusters.brokersHelp }}</span>
              </label>
            </div>
            <div class="flex flex-wrap gap-4">
              <div class="form-control w-auto">
                <label class="label">
                  <span class="label-text font-medium">{{ t.clusters.requestTimeout }}</span>
                </label>
                <input
                  v-model.number="formData.request_timeout_ms"
                  type="number"
                  class="input input-bordered w-40"
                  placeholder="30000"
                />
              </div>
              <div class="form-control w-auto">
                <label class="label">
                  <span class="label-text font-medium">{{ t.clusters.operationTimeout }}</span>
                </label>
                <input
                  v-model.number="formData.operation_timeout_ms"
                  type="number"
                  class="input input-bordered w-40"
                  placeholder="30000"
                />
              </div>
            </div>
            <!-- Group Selector -->
            <div v-if="clusterStore.groups.length > 0" class="form-control">
              <label class="label">
                <span class="label-text font-medium">{{ t.clusters.group }}</span>
              </label>
              <select v-model="formData.group_id" class="select select-bordered w-full">
                <option :value="undefined">{{ t.clusters.noGroup }}</option>
                <option v-for="group in clusterStore.groups" :key="group.id" :value="group.id">
                  {{ group.name }}
                </option>
              </select>
            </div>
            <!-- Test Connection Button -->
            <div class="flex items-center gap-2">
              <button
                type="button"
                class="btn btn-outline btn-sm flex items-center gap-2"
                :disabled="!formData.brokers || testingConnection"
                @click="testConnectionConfig"
              >
                <span v-if="testingConnection" class="loading loading-spinner loading-xs"></span>
                <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12c0 1.268-.63 2.39-1.593 3.068a3.745 3.745 0 0 1-1.043 3.296 3.745 3.745 0 0 1-3.296 1.043A3.745 3.745 0 0 1 12 21c-1.268 0-2.39-.63-3.068-1.593a3.746 3.746 0 0 1-3.296-1.043 3.745 3.745 0 0 1-1.043-3.296A3.745 3.745 0 0 1 3 12c0-1.268.63-2.39 1.593-3.068a3.745 3.745 0 0 1 1.043-3.296 3.746 3.746 0 0 1 3.296-1.043A3.746 3.746 0 0 1 12 3c1.268 0 2.39.63 3.068 1.593a3.746 3.746 0 0 1 3.296 1.043 3.746 3.746 0 0 1 1.043 3.296A3.745 3.745 0 0 1 21 12Z" />
                </svg>
                {{ testingConnection ? t.clusters.testingConnection : t.clusters.testConnection }}
              </button>
              <span v-if="connectionTestResult" class="text-xs" :class="connectionTestResult.success ? 'text-success' : 'text-error'">
                {{ connectionTestResult.success ? t.clusters.connectionSuccess : `${t.clusters.connectionFailed}：${connectionTestResult.error}` }}
              </span>
            </div>
            <div class="modal-action mt-4">
              <button type="button" class="btn btn-outline" @click="closeModal">{{ t.common.cancel }}</button>
              <button type="submit" class="btn btn-primary flex items-center gap-2" :disabled="submitting">
                <span v-if="submitting" class="loading loading-spinner loading-sm"></span>
                {{ editingCluster ? t.common.edit : t.common.create }}
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeModal">
          <button>close</button>
        </form>
      </dialog>

      <!-- Disconnect Confirm Modal -->
      <dialog ref="disconnectModalRef" class="modal modal-bottom sm:modal-middle">
        <div class="modal-box">
          <h3 class="font-bold text-xl mb-4">
            {{ t.clusters.disconnectConfirm }}
            <span class="text-primary">{{ clusterToDisconnect }}</span>?
          </h3>
          <div class="flex justify-end gap-2 mt-6">
            <button type="button" class="btn btn-ghost" @click="closeDisconnectModal">{{ t.common.cancel }}</button>
            <button type="button" class="btn btn-error" @click="confirmDisconnect" :disabled="disconnecting.has(clusterToDisconnect || '')">
              <span v-if="disconnecting.has(clusterToDisconnect || '')" class="loading loading-spinner loading-sm"></span>
              {{ t.common.confirm }}
            </button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeDisconnectModal">
          <button>close</button>
        </form>
      </dialog>

      <!-- Manage Groups Modal -->
      <dialog ref="manageGroupsModalRef" class="modal modal-bottom sm:modal-middle">
        <div class="modal-box p-4">
          <!-- Header -->
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44z" />
                </svg>
              </div>
              <h3 class="font-bold text-base">{{ t.clusters.manageGroups }}</h3>
            </div>
            <button class="btn btn-sm btn-circle btn-ghost" @click="closeManageGroupsModal">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Groups List -->
          <div class="modal-option-list mb-4">
            <div v-for="group in clusterStore.groups" :key="group.id" class="modal-option justify-between">
              <div>
                <div class="font-semibold">{{ group.name }}</div>
                <div class="text-xs text-base-content/60">{{ group.description || t.clusters.noDescription }}</div>
              </div>
              <div class="flex gap-1">
                <button class="btn btn-ghost btn-xs" @click="editGroup(group)" :title="t.clusters.editGroup">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" />
                  </svg>
                </button>
                <button class="btn btn-ghost btn-xs text-error" @click="confirmDeleteGroup(group)" :title="t.common.delete">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                  </svg>
                </button>
              </div>
            </div>
            <div v-if="clusterStore.groups.length === 0" class="modal-empty-state">
              <p class="text-base-content/60 text-sm">
                {{ t.clusters.noGroup }}
              </p>
            </div>
          </div>

          <div class="modal-actions">
            <button type="button" class="btn btn-outline" @click="closeManageGroupsModal">{{ t.common.cancel }}</button>
            <button type="button" class="btn btn-primary" @click="openAddGroupForm">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
              </svg>
              {{ t.clusters.addGroup }}
            </button>
          </div>

          <!-- Add/Edit Group Form -->
          <div v-if="showGroupForm" class="modal-form">
            <h4 class="font-semibold mb-3">{{ editingGroup ? t.clusters.editGroup : t.clusters.addGroup }}</h4>
            <div class="form-control mb-3">
              <label class="label">
                <span class="label-text font-medium">{{ t.clusters.groupName }}</span>
              </label>
              <input v-model="groupFormData.name" type="text" class="input input-bordered w-full" :placeholder="t.clusters.groupNamePlaceholder" />
            </div>
            <div class="form-control mb-3">
              <label class="label">
                <span class="label-text font-medium">{{ t.clusters.groupDescription }}</span>
              </label>
              <textarea v-model="groupFormData.description" class="textarea textarea-bordered w-full" :placeholder="t.clusters.groupDescPlaceholder"></textarea>
            </div>
            <div class="modal-actions">
              <button type="button" class="btn btn-ghost btn-sm" @click="cancelGroupForm">{{ t.common.cancel }}</button>
              <button type="button" class="btn btn-primary btn-sm" @click="submitGroupForm" :disabled="groupSubmitting">
                <span v-if="groupSubmitting" class="loading loading-spinner loading-xs"></span>
                {{ editingGroup ? t.common.save : t.common.create }}
              </button>
            </div>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeManageGroupsModal">
          <button>close</button>
        </form>
      </dialog>

      <!-- Delete Group Confirm Modal -->
      <dialog ref="deleteGroupModalRef" class="modal modal-bottom sm:modal-middle">
        <div class="modal-box p-4">
          <!-- Header -->
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-error/20 to-error/10 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-error">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z" />
                </svg>
              </div>
              <h3 class="font-bold text-base">{{ t.common.confirmDelete }}</h3>
            </div>
          </div>
          <p class="mb-2 text-base-content/80">{{ t.common.confirmDelete }} <span class="font-semibold text-primary">{{ groupToDelete?.name }}</span>？</p>
          <p class="text-sm text-base-content/60 mb-4">{{ t.clusters.confirmDeleteGroup }}</p>
          <div class="modal-actions">
            <button type="button" class="btn btn-ghost" @click="closeDeleteGroupModal">{{ t.common.cancel }}</button>
            <button type="button" class="btn btn-error" @click="confirmDeleteGroupAction" :disabled="groupDeleting">
              <span v-if="groupDeleting" class="loading loading-spinner loading-xs"></span>
              {{ t.common.delete }}
            </button>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop" @click="closeDeleteGroupModal">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { apiClient } from '@/api/client';
import { useClusterStore } from '@/stores/cluster';
import { useClusterConnectionStore } from '@/stores/clusterConnection';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import type { Cluster } from '@/types/api';

const route = useRoute();
const router = useRouter();
const clusterStore = useClusterStore();
const connectionStore = useClusterConnectionStore();
const languageStore = useLanguageStore();
const { showError, showSuccess } = useToast();

const clusters = computed(() => clusterStore.clusters);
const loading = computed(() => clusterStore.loading);
const error = computed(() => clusterStore.error);

// 翻译
const t = computed(() => languageStore.t);

// 选中的分组 ID
const selectedGroupId = ref<number | null>(null);

// Group selector ref for scrolling
const groupSelectorRef = ref<HTMLElement | null>(null);

// Scroll groups horizontally
function scrollGroups(distance: number) {
  if (groupSelectorRef.value) {
    groupSelectorRef.value.scrollBy({ left: distance, behavior: 'smooth' });
  }
}

// Handle mouse wheel to scroll horizontally
function handleHorizontalScroll(event: WheelEvent) {
  if (groupSelectorRef.value && event.deltaY !== 0) {
    event.preventDefault();
    groupSelectorRef.value.scrollBy({ left: event.deltaY, behavior: 'auto' });
  }
}

// 过滤后的集群列表
const filteredClusters = computed(() => {
  if (selectedGroupId.value === null) {
    return clusters.value;
  }
  return clusters.value.filter(c => (c.group_id ?? null) === selectedGroupId.value);
});

const editingCluster = ref<Cluster | null>(null);
const testing = ref(new Set<number>());
const disconnecting = ref(new Set<string>());
const reconnecting = ref(new Set<string>());
const refreshingTopics = ref(new Set<string>());
const submitting = ref(false);
const testingConnection = ref(false);
const connectionTestResult = ref<{ success: boolean; error?: string } | null>(null);

const formData = reactive({
  name: '',
  brokers: '',
  request_timeout_ms: 30000,
  operation_timeout_ms: 30000,
  group_id: undefined as number | undefined,
});

const modalRef = ref<HTMLDialogElement>();
const disconnectModalRef = ref<HTMLDialogElement>();
const clusterToDisconnect = ref<string>('');

// 分组管理相关状态
const manageGroupsModalRef = ref<HTMLDialogElement>();
const deleteGroupModalRef = ref<HTMLDialogElement>();
const showGroupForm = ref(false);
const editingGroup = ref<{ id: number; name: string; description?: string | null } | null>(null);
const groupToDelete = ref<{ id: number; name: string } | null>(null);
const groupSubmitting = ref(false);
const groupDeleting = ref(false);

const groupFormData = reactive({
  name: '',
  description: '',
});

function openManageGroupsModal() {
  showGroupForm.value = false;
  editingGroup.value = null;
  groupFormData.name = '';
  groupFormData.description = '';
  manageGroupsModalRef.value?.showModal();
}

function closeManageGroupsModal() {
  manageGroupsModalRef.value?.close();
  showGroupForm.value = false;
  editingGroup.value = null;
  groupFormData.name = '';
  groupFormData.description = '';
}

function openAddGroupForm() {
  editingGroup.value = null;
  groupFormData.name = '';
  groupFormData.description = '';
  showGroupForm.value = true;
}

function editGroup(group: { id: number; name: string; description?: string | null }) {
  editingGroup.value = group;
  groupFormData.name = group.name;
  groupFormData.description = group.description || '';
  showGroupForm.value = true;
}

function cancelGroupForm() {
  showGroupForm.value = false;
  editingGroup.value = null;
  groupFormData.name = '';
  groupFormData.description = '';
}

async function submitGroupForm() {
  if (!groupFormData.name.trim()) {
    showError(t.value.clusters.groupNamePlaceholder);
    return;
  }

  groupSubmitting.value = true;
  try {
    if (editingGroup.value) {
      await clusterStore.updateGroup(editingGroup.value.id, {
        name: groupFormData.name.trim(),
        description: groupFormData.description.trim() || null,
      });
      showSuccess(t.value.clusters.groupUpdated);
    } else {
      await clusterStore.createGroup({
        name: groupFormData.name.trim(),
        description: groupFormData.description.trim() || null,
      });
      showSuccess(t.value.clusters.groupCreated);
    }
    // 刷新分组列表
    await clusterStore.fetchGroups();
    // 关闭表单
    cancelGroupForm();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    groupSubmitting.value = false;
  }
}

function confirmDeleteGroup(group: { id: number; name: string }) {
  groupToDelete.value = group;
  deleteGroupModalRef.value?.showModal();
}

function closeDeleteGroupModal() {
  deleteGroupModalRef.value?.close();
  groupToDelete.value = null;
}

async function confirmDeleteGroupAction() {
  if (!groupToDelete.value) return;

  groupDeleting.value = true;
  try {
    await clusterStore.deleteGroup(groupToDelete.value.id);
    showSuccess(t.value.clusters.groupDeleted);
    // 刷新分组列表和集群列表
    await clusterStore.fetchGroups();
    await clusterStore.fetchClusters();
    closeDeleteGroupModal();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    groupDeleting.value = false;
  }
}

function openCreateModal() {
  editingCluster.value = null;
  formData.name = '';
  formData.brokers = '';
  formData.request_timeout_ms = 30000;
  formData.operation_timeout_ms = 30000;
  formData.group_id = undefined;
  modalRef.value?.showModal();
}

function editCluster(cluster: Cluster) {
  editingCluster.value = cluster;
  formData.name = cluster.name;
  formData.brokers = cluster.brokers;
  formData.request_timeout_ms = cluster.request_timeout_ms;
  formData.operation_timeout_ms = cluster.operation_timeout_ms;
  // 保持集群当前的分组状态（null 表示无分组）
  formData.group_id = cluster.group_id ?? undefined;
  modalRef.value?.showModal();
}

function closeModal() {
  modalRef.value?.close();
  editingCluster.value = null;
  formData.name = '';
  formData.brokers = '';
  formData.request_timeout_ms = 30000;
  formData.operation_timeout_ms = 30000;
  formData.group_id = undefined;
  // 清除路由参数
  router.replace({ path: '/clusters', query: {} });
  // 清除测试结果
  connectionTestResult.value = null;
}

// Test cluster connection with current form configuration
async function testConnectionConfig() {
  testingConnection.value = true;
  connectionTestResult.value = null;
  try {
    const result = await apiClient.testClusterConfig({
      name: formData.name,
      brokers: formData.brokers,
      request_timeout_ms: formData.request_timeout_ms,
      operation_timeout_ms: formData.operation_timeout_ms,
    });
    connectionTestResult.value = result;
    if (result.success) {
      showSuccess(t.value.clusters.connected);
    } else {
      showError(result.error || t.value.clusters.connectionError);
    }
  } catch (e) {
    connectionTestResult.value = { success: false, error: (e as { message: string }).message };
    showError(`${t.value.clusters.connectionError}: ${(e as { message: string }).message}`);
  } finally {
    testingConnection.value = false;
  }
}

async function handleSubmit() {
  submitting.value = true;
  try {
    if (editingCluster.value) {
      await clusterStore.updateCluster(editingCluster.value.id, {
        name: formData.name,
        brokers: formData.brokers,
        request_timeout_ms: formData.request_timeout_ms,
        operation_timeout_ms: formData.operation_timeout_ms,
        group_id: formData.group_id,
      });
      // 更新后刷新集群列表，确保数据同步
      await clusterStore.fetchClusters();
    } else {
      await clusterStore.createCluster({
        name: formData.name,
        brokers: formData.brokers,
        request_timeout_ms: formData.request_timeout_ms,
        operation_timeout_ms: formData.operation_timeout_ms,
        group_id: formData.group_id,
      });
      // 创建后刷新集群列表和分组列表
      await clusterStore.fetchClusters();
    }
    showSuccess(editingCluster.value ? t.value.clusters.updated : t.value.clusters.created);
    closeModal();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    submitting.value = false;
  }
}

async function testConnection(id: number) {
  testing.value.add(id);
  try {
    const result = await clusterStore.testCluster(id);
    // 获取集群名称
    const cluster = clusters.value.find((c) => c.id === id);
    if (cluster) {
      // 立即更新 connectionStore 中的状态，让 UI 立即反映
      if (result.success) {
        connectionStore.updateConnectionStatus(cluster.name, 'connected');
      } else {
        connectionStore.updateConnectionStatus(cluster.name, 'error', result.error);
      }
    }
    // 然后刷新所有连接状态
    await connectionStore.fetchAllConnections();
    if (result.success) {
      showSuccess(t.value.clusters.connected);
    } else {
      showError(t.value.clusters.connectionError);
    }
  } catch (e) {
    const cluster = clusters.value.find((c) => c.id === id);
    if (cluster) {
      connectionStore.updateConnectionStatus(cluster.name, 'error', (e as { message: string }).message);
    }
    // 测试失败后也要刷新连接状态
    await connectionStore.fetchAllConnections();
    showError(`${t.value.clusters.connectionError}: ${(e as { message: string }).message}`);
  } finally {
    testing.value.delete(id);
  }
}

function confirmDelete(cluster: Cluster) {
  if (confirm(`${t.value.clusters.confirmDelete} "${cluster.name}"?`)) {
    clusterStore.deleteCluster(cluster.id);
  }
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

function selectGroup(groupId: number | null) {
  selectedGroupId.value = groupId;
}

function getConnectionStatus(clusterName: string) {
  return connectionStore.getConnectionStatus(clusterName);
}

async function disconnectCluster(clusterName: string) {
  clusterToDisconnect.value = clusterName;
  disconnectModalRef.value?.showModal();
}

function closeDisconnectModal() {
  disconnectModalRef.value?.close();
}

function confirmDisconnect() {
  const clusterName = clusterToDisconnect.value;
  if (!clusterName) return;

  disconnecting.value.add(clusterName);
  connectionStore.disconnectCluster(clusterName)
    .then(() => {
      connectionStore.fetchAllConnections();
      showSuccess(t.value.clusters.disconnectedSuccess);
    })
    .catch((e) => {
      showError(`${t.value.clusters.disconnect}: ${e.message}`);
    })
    .finally(() => {
      disconnecting.value.delete(clusterName);
      closeDisconnectModal();
    });
}

async function reconnectCluster(clusterName: string) {
  reconnecting.value.add(clusterName);
  try {
    await connectionStore.reconnectCluster(clusterName);
    await connectionStore.fetchAllConnections();
    await clusterStore.fetchClusters();
    const status = connectionStore.getConnectionStatus(clusterName);
    const statusText = status?.status === 'connected' ? t.value.clusters.connected :
                       status?.status === 'error' ? t.value.clusters.connectionError :
                       status?.status || t.value.clusters.unknown;
    showSuccess(`${t.value.clusters.reconnectSuccess}: ${statusText}`);
  } catch (e) {
    showError(`${t.value.clusters.reconnectFailed}: ${(e as { message: string }).message}`);
  } finally {
    reconnecting.value.delete(clusterName);
  }
}

// View cluster topics in topics page
function viewClusterTopics(clusterName: string) {
  router.push({ path: '/topics', query: { cluster: clusterName } });
}

// Refresh cluster topics metadata
async function refreshClusterTopics(clusterName: string) {
  refreshingTopics.value.add(clusterName);
  try {
    const result = await apiClient.refreshTopics(clusterName);
    if (result.success) {
      const addedCount = result.added?.length || 0;
      const removedCount = result.removed?.length || 0;
      let message = `${t.value.clusters.topicsRefreshed}: ${result.total || 0} ${t.value.topics.partitions}`;
      if (addedCount > 0 || removedCount > 0) {
        message += ` (+${addedCount} -${removedCount})`;
      }
      showSuccess(message);
    }
  } catch (e) {
    showError(`${t.value.clusters.refreshFailed}: ${(e as { message: string }).message}`);
  } finally {
    refreshingTopics.value.delete(clusterName);
  }
}

async function refreshClusters() {
  await clusterStore.fetchClusters();
  await connectionStore.fetchAllConnections();
}

onMounted(() => {
  // 只加载集群列表和连接状态（都是轻量级查询，不涉及 Kafka 连接）
  clusterStore.fetchClusters();
  clusterStore.fetchGroups();
  connectionStore.fetchAllConnections();

  // 检查路由参数，如果 action=create 则打开创建模态框
  if (route.query.action === 'create') {
    setTimeout(() => {
      openCreateModal();
    }, 100);
  }
  // 检查路由参数，如果 action=edit 则打开编辑模态框
  if (route.query.action === 'edit' && route.query.cluster) {
    const clusterToEdit = clusters.value.find(c => c.name === route.query.cluster);
    if (clusterToEdit) {
      setTimeout(() => {
        editCluster(clusterToEdit);
      }, 100);
    }
  }
});

// 监听路由参数变化
watch(() => route.fullPath, (newPath, oldPath) => {
  if (newPath !== oldPath && route.query.action === 'create') {
    // 确保 modal 完全关闭后再打开
    setTimeout(() => {
      openCreateModal();
    }, 50);
  }
  // 监听编辑集群
  if (newPath !== oldPath && route.query.action === 'edit' && route.query.cluster) {
    const clusterToEdit = clusters.value.find(c => c.name === route.query.cluster);
    if (clusterToEdit) {
      setTimeout(() => {
        editCluster(clusterToEdit);
      }, 50);
    }
  }
});
</script>

<style scoped>
.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
</style>
