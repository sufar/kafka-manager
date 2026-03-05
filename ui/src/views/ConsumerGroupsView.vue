<template>
  <div class="p-6">
    <div class="flex justify-between items-center mb-8">
      <div>
        <h2 class="text-3xl font-bold flex items-center gap-3">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-10 h-10">
            <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198l.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.941-3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0z" />
          </svg>
          {{ t.consumerGroups.title }}
        </h2>
        <p class="text-base-content/60 mt-2 text-lg">
          <span v-if="clusterParam || selectedGroup">
            <span v-if="clusterParam">{{ t.dashboard.clusters }}: {{ clusterParam }}</span>
            <span v-if="selectedGroup"> • {{ t.consumerGroups.groupId }}: {{ selectedGroup }}</span>
          </span>
          <span v-else>{{ t.consumerGroups.description }}</span>
        </p>
      </div>
      <div class="flex gap-2" v-if="!clusterParam && !selectedGroup">
        <button
          v-if="selectedClusterIds.length > 0"
          class="btn btn-sm"
          :class="viewMode === 'by-cluster' ? 'btn-active btn-primary' : 'btn-outline'"
          @click="viewMode = 'by-cluster'"
        >
          {{ t.dashboard.byCluster }}
        </button>
        <button
          class="btn btn-sm"
          :class="viewMode === 'all-groups' ? 'btn-active btn-primary' : 'btn-outline'"
          @click="viewMode = 'all-groups'"
        >
          {{ t.consumerGroups.title }}
        </button>
      </div>
      <button v-if="selectedGroup" class="btn btn-sm btn-ghost" @click="clearSelectedGroup">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
        </svg>
        {{ t.common.back }}
      </button>
    </div>

    <!-- No cluster selected -->
    <div v-if="!clusterParam && selectedClusterIds.length === 0" class="alert alert-info">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-6 h-6">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{{ t.consumerGroups.selectCluster }}</span>
    </div>

    <!-- Single cluster view (from URL param) - Must come FIRST before other checks -->
    <div v-else-if="clusterParam && clusterGroups.length > 0" class="card glass gradient-border overflow-x-auto">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t.common.name }}</th>
            <th>{{ t.consumerGroups.state }}</th>
            <th>{{ t.consumerGroups.members }}</th>
            <th>{{ t.consumerGroups.topics }}</th>
            <th>{{ t.consumerGroups.totalLag }}</th>
            <th>{{ t.common.actions }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="group in clusterGroups" :key="group.name" class="hover cursor-pointer" @click="selectGroup(group.name)">
            <td>
              <div class="font-semibold text-primary">{{ group.name }}</div>
            </td>
            <td>
              <div :class="`badge ${getStateBadge(group.state)}`">{{ group.state }}</div>
            </td>
            <td>
              <span :class="group.memberCount === 0 ? 'text-base-content/50' : 'text-success font-semibold'">
                {{ group.memberCount || 0 }}
              </span>
            </td>
            <td>
              <div class="flex flex-wrap gap-1 max-w-xs">
                <template v-if="group.topics && group.topics.length > 0">
                  <span v-for="topic in group.topics.slice(0, 3)" :key="topic" class="badge badge-ghost badge-sm">
                    {{ topic }}
                  </span>
                  <span v-if="group.topics.length > 3" class="badge badge-ghost badge-sm">+{{ group.topics.length - 3 }}</span>
                </template>
                <span v-else class="text-base-content/50 text-sm">-</span>
              </div>
            </td>
            <td>
              <span :class="(group.totalLag ?? 0) > 0 ? 'text-warning font-bold' : 'text-success'">
                {{ formatLag(group.totalLag ?? 0) }}
              </span>
            </td>
            <td>
              <div class="flex gap-2" @click.stop>
                <button class="btn btn-sm btn-ghost" @click="selectGroup(group.name)">{{ t.consumerGroups.groupDetails }}</button>
                <button class="btn btn-sm btn-ghost" @click.stop="openResetOffsetModal(clusterParam, group.name)">{{ t.consumerGroups.resetOffset }}</button>
                <button class="btn btn-sm btn-ghost text-error" @click.stop="confirmDelete(clusterParam, group.name)">{{ t.common.delete }}</button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Consumer Group Detail View -->
    <div v-else-if="selectedGroup" class="space-y-6">
      <!-- Loading detail -->
      <div v-if="loadingDetail" class="flex justify-center py-12">
        <span class="loading loading-spinner loading-lg text-primary"></span>
      </div>
      <!-- Detail content (shown when not loading) -->
      <template v-else>
        <!-- Header -->
        <div class="bg-base-100 rounded-box shadow p-6">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="text-2xl font-bold">
                {{ selectedGroup }}
              </h3>
              <p class="text-base-content/60 mt-1">
                Cluster: {{ clusterParam }}
              </p>
            </div>
            <div class="flex items-center gap-3">
              <div class="badge badge-lg" :class="getStateBadge(selectedGroupDetail?.state || '')">
                {{ selectedGroupDetail?.state }}
              </div>
              <button class="btn btn-primary" @click="openResetOffsetModal(clusterParam || '', selectedGroup)">
                {{ t.consumerGroups.resetOffset }}
              </button>
            </div>
          </div>

          <!-- Group Info Cards -->
          <div class="grid grid-cols-3 gap-4 mt-6">
            <div class="bg-base-200 rounded-lg p-4">
              <div class="text-sm text-base-content/60 mb-1">{{ t.consumerGroups.state }}</div>
              <div class="font-semibold text-lg">{{ selectedGroupDetail?.state || 'N/A' }}</div>
            </div>
            <div class="bg-base-200 rounded-lg p-4">
              <div class="text-sm text-base-content/60 mb-1">{{ t.consumerGroups.coordinator }}</div>
              <div class="font-semibold text-lg">{{ selectedGroupDetail?.protocol || 'N/A' }}</div>
            </div>
            <div class="bg-base-200 rounded-lg p-4">
              <div class="text-sm text-base-content/60 mb-1">{{ t.consumerGroups.members }}</div>
              <div class="font-semibold text-lg">{{ selectedGroupDetail?.members?.length || 0 }}</div>
            </div>
          </div>

          <!-- Topics -->
          <div v-if="offsetDetail && offsetDetail.partitions.length > 0" class="mt-6">
            <h4 class="font-semibold mb-3 flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
              </svg>
              {{ t.consumerGroups.topics }}
            </h4>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="topic in uniqueTopics"
                :key="topic"
                class="badge badge-primary badge-lg gap-2"
              >
                {{ topic }}
              </span>
            </div>
          </div>
        </div>

        <!-- Members -->
      <div v-if="selectedGroupDetail?.members && selectedGroupDetail.members.length > 0" class="bg-base-100 rounded-box shadow p-6">
        <h4 class="font-semibold mb-4 flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 0 0 2.625.372 9.337 9.337 0 0 0 4.121-.952 4.125 4.125 0 0 0-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 0 1 8.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0 1 11.964-3.07M12 6.375a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0Zm8.25 2.25a2.625 2.625 0 1 1-5.25 0 2.625 2.625 0 0 1 5.25 0Z" />
          </svg>
          {{ t.consumerGroups.members }} ({{ selectedGroupDetail.members.length }})
        </h4>
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>#</th>
                <th>{{ t.consumerGroups.coordinator }}</th>
                <th>Host</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(member, idx) in selectedGroupDetail.members" :key="member.client_id">
                <td class="text-base-content/50">{{ idx + 1 }}</td>
                <td class="font-mono">{{ member.client_id }}</td>
                <td class="font-mono">{{ member.host }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- No members warning -->
      <div v-if="(!selectedGroupDetail?.members || selectedGroupDetail.members.length === 0)" class="alert alert-warning">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-6 h-6">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <span>{{ t.consumerGroups.noActiveMembers }}</span>
      </div>

      <!-- Offsets by Topic -->
      <div v-if="offsetDetail && offsetDetail.partitions.length > 0" class="bg-base-100 rounded-box shadow p-6">
        <h4 class="font-semibold mb-4 flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 3v11.25A2.25 2.25 0 0 0 6 16.5h2.25M3.75 3h-1.5m1.5 0h16.5m0 0h1.5m-1.5 0v11.25A2.25 2.25 0 0 1 18 16.5h-2.25m-7.5 0h7.5m-7.5 0-1 3m8.5-3 1 3m0 0 .5 1.5m-.5-1.5h-9.5m0 0-.5 1.5M9 11.25v1.5M12 9v3.75m3-6v6" />
          </svg>
          {{ t.consumerGroups.offsetInfo }}
        </h4>

        <!-- Group partitions by topic -->
        <div v-for="(partitions, topic) in partitionsByTopic" :key="topic" class="mb-6">
          <h5 class="font-semibold text-primary mb-3">{{ topic }}</h5>
          <div class="overflow-x-auto">
            <table class="table">
              <thead>
                <tr>
                  <th>{{ t.consumerGroups.partition }}</th>
                  <th>{{ t.consumerGroups.startOffset }}</th>
                  <th>{{ t.consumerGroups.endOffset }}</th>
                  <th>{{ t.consumerGroups.currentOffset }}</th>
                  <th>{{ t.consumerGroups.lag }}</th>
                  <th>{{ t.consumerGroups.stateLabel }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="p in partitions" :key="p.partition">
                  <td class="font-mono">{{ p.partition }}</td>
                  <td class="font-mono text-base-content/70">{{ p.start_offset ?? '-' }}</td>
                  <td class="font-mono">{{ p.log_end_offset }}</td>
                  <td class="font-mono">{{ p.current_offset }}</td>
                  <td>
                    <span :class="p.lag > 0 ? 'text-warning font-bold' : 'text-success'">
                      {{ p.lag }}
                    </span>
                  </td>
                  <td>
                    <span class="text-sm text-base-content/60">{{ p.state || '-' }}</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <!-- Topic totals -->
          <div class="mt-3 p-3 bg-base-200 rounded-lg">
            <div class="flex gap-4 text-sm">
              <span><strong>Partitions:</strong> {{ partitions.length }}</span>
              <span><strong>Total Lag:</strong> <span :class="getTopicTotalLag(partitions) > 0 ? 'text-warning font-bold' : 'text-success'">{{ getTopicTotalLag(partitions) }}</span></span>
            </div>
          </div>
        </div>

        <!-- Grand Total -->
        <div class="bg-base-200 rounded-lg p-4 mt-4">
          <div class="flex justify-between items-center">
            <span class="font-semibold">{{ t.consumerGroups.totalLag }}</span>
            <span :class="offsetDetail.total_lag > 0 ? 'text-warning font-bold text-xl' : 'text-success text-xl'">
              {{ formatLag(offsetDetail.total_lag) }}
            </span>
          </div>
        </div>
      </div>
    </template>
  </div>

  <!-- Single cluster empty state -->
    <div v-else-if="clusterParam && !clusterGroups.length && !loading && !error" class="text-center py-12">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-24 h-24 mx-auto text-base-content/30 mb-4">
        <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 0 0 3.741-.479 3 3 0 0 0-4.682-2.72m.94 3.198.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0 1 12 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 0 1 6 18.719m12 0a5.971 5.971 0 0 0-.941-3.197m0 0A5.995 5.995 0 0 0 12 12.75a5.995 5.995 0 0 0-5.058 2.772m0 0a3 3 0 0 0-4.681 2.72 8.986 8.986 0 0 0 3.74.477m.94-3.197a5.971 5.971 0 0 0-.944-3.197M6 15.12a9.038 9.038 0 0 1-3.741.479 3 3 0 0 1 4.682-2.72m.941-3.197a5.971 5.971 0 0 1 .944-3.197M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm0 0h.008v.008H15V12Zm-3-3h.008v.008H12V9Z" />
      </svg>
      <h3 class="text-xl font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60">{{ t.consumerGroups.noGroupsFound }}</p>
    </div>

    <!-- Loading state -->
    <div v-else-if="loading" class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg text-primary"></span>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{{ error }}</span>
    </div>

    <!-- View by cluster mode -->
    <template v-else-if="viewMode === 'by-cluster'">
      <div v-for="(clusterGroups, clusterName) in groupsByCluster" :key="clusterName" class="mb-8">
        <div class="flex items-center justify-between mb-3">
          <div class="flex items-center gap-2">
            <div
              class="w-3 h-3 rounded-full"
              :class="{
                'bg-success': getClusterHealth(clusterName)?.healthy,
                'bg-error': getClusterHealth(clusterName)?.healthy === false
              }"
            ></div>
            <h3 class="text-xl font-semibold">{{ clusterName }}</h3>
            <span class="badge badge-ghost">{{ clusterGroups.length }} {{ t.consumerGroups.groups }}</span>
          </div>
        </div>

        <div v-if="clusterGroups.length === 0" class="card bg-base-100 shadow-sm">
          <div class="card-body py-8">
            <p class="text-center text-base-content/60">{{ t.consumerGroups.noGroupsInCluster }}</p>
          </div>
        </div>
        <div v-else class="overflow-x-auto bg-base-100 rounded-box shadow">
          <table class="table">
            <thead>
              <tr>
                <th>{{ t.common.name }}</th>
                <th>{{ t.consumerGroups.state }}</th>
                <th>{{ t.consumerGroups.members }}</th>
                <th>{{ t.consumerGroups.topics }}</th>
                <th>{{ t.consumerGroups.totalLag }}</th>
                <th>{{ t.common.actions }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="group in clusterGroups" :key="group.name" class="hover">
                <td>
                  <div class="font-semibold">{{ group.name }}</div>
                </td>
                <td>
                  <div :class="`badge ${getStateBadge(group.state)}`">{{ group.state }}</div>
                </td>
                <td>
                  <span :class="group.memberCount === 0 ? 'text-base-content/50' : 'text-success font-semibold'">
                    {{ group.memberCount || 0 }}
                  </span>
                </td>
                <td>
                  <div class="flex flex-wrap gap-1 max-w-xs">
                    <template v-if="group.topics && group.topics.length > 0">
                      <span v-for="topic in group.topics.slice(0, 3)" :key="topic" class="badge badge-ghost badge-sm">
                        {{ topic }}
                      </span>
                      <span v-if="group.topics.length > 3" class="badge badge-ghost badge-sm">+{{ group.topics.length - 3 }}</span>
                    </template>
                    <span v-else class="text-base-content/50 text-sm">-</span>
                  </div>
                </td>
                <td>
                  <span :class="(group.totalLag ?? 0) > 0 ? 'text-warning font-bold' : 'text-success'">
                    {{ formatLag(group.totalLag ?? 0) }}
                  </span>
                </td>
                <td>
                  <div class="flex gap-2">
                    <button class="btn btn-sm btn-ghost" @click="selectGroup(group.name)">{{ t.consumerGroups.groupDetails }}</button>
                    <button class="btn btn-sm btn-ghost" @click="openResetOffsetModal(clusterName, group.name)">{{ t.consumerGroups.resetOffset }}</button>
                    <button class="btn btn-sm btn-ghost text-error" @click="confirmDelete(clusterName, group.name)">{{ t.common.delete }}</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- All groups mode (consolidated) -->
    <template v-else>
      <div v-if="allGroupsList.length === 0" class="text-center py-12">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-24 h-24 mx-auto text-base-content/30 mb-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 0 0 3.741-.479 3 3 0 0 0-4.682-2.72m.94 3.198.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0 1 12 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 0 1 6 18.719m12 0a5.971 5.971 0 0 0-.941-3.197m0 0A5.995 5.995 0 0 0 12 12.75a5.995 5.995 0 0 0-5.058 2.772m0 0a3 3 0 0 0-4.681 2.72 8.986 8.986 0 0 0 3.74.477m.94-3.197a5.971 5.971 0 0 0-.944-3.197M6 15.12a9.038 9.038 0 0 1-3.741.479 3 3 0 0 1 4.682-2.72m.941-3.197a5.971 5.971 0 0 1 .944-3.197M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm0 0h.008v.008H15V12Zm-3-3h.008v.008H12V9Z" />
        </svg>
        <h3 class="text-xl font-semibold mb-2">{{ t.common.noData }}</h3>
        <p class="text-base-content/60">{{ t.consumerGroups.noGroupsFound }}</p>
      </div>

      <div v-else class="overflow-x-auto bg-base-100 rounded-box shadow">
        <table class="table">
          <thead>
            <tr>
              <th>{{ t.dashboard.clusters }}</th>
              <th>{{ t.common.name }}</th>
              <th>{{ t.consumerGroups.state }}</th>
              <th>{{ t.consumerGroups.members }}</th>
              <th>{{ t.consumerGroups.topics }}</th>
              <th>{{ t.consumerGroups.totalLag }}</th>
              <th>{{ t.common.actions }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in allGroupsList" :key="`${item.cluster}-${item.name}`" class="hover">
              <td>
                <div class="flex items-center gap-2">
                  <div
                    class="w-2 h-2 rounded-full"
                    :class="{
                      'bg-success': getClusterHealth(item.cluster)?.healthy,
                      'bg-error': getClusterHealth(item.cluster)?.healthy === false
                    }"
                  ></div>
                  <span class="font-medium">{{ item.cluster }}</span>
                </div>
              </td>
              <td>
                <div class="font-semibold">{{ item.name }}</div>
              </td>
              <td>
                <div :class="`badge ${getStateBadge(item.state)}`">{{ item.state }}</div>
              </td>
              <td>
                <span :class="item.memberCount === 0 ? 'text-base-content/50' : 'text-success font-semibold'">
                  {{ item.memberCount || 0 }}
                </span>
              </td>
              <td>
                <div class="flex flex-wrap gap-1 max-w-xs">
                  <template v-if="item.topics && item.topics.length > 0">
                    <span v-for="topic in item.topics.slice(0, 3)" :key="topic" class="badge badge-ghost badge-sm">
                      {{ topic }}
                    </span>
                    <span v-if="item.topics.length > 3" class="badge badge-ghost badge-sm">+{{ item.topics.length - 3 }}</span>
                  </template>
                  <span v-else class="text-base-content/50 text-sm">-</span>
                </div>
              </td>
              <td>
                <span :class="(item.totalLag ?? 0) > 0 ? 'text-warning font-bold' : 'text-success'">
                  {{ formatLag(item.totalLag ?? 0) }}
                </span>
              </td>
              <td>
                <div class="flex gap-2">
                  <button class="btn btn-sm btn-ghost" @click="selectGroup(item.name)">{{ t.consumerGroups.groupDetails }}</button>
                  <button class="btn btn-sm btn-ghost" @click="openResetOffsetModal(item.cluster, item.name)">{{ t.consumerGroups.resetOffset }}</button>
                  <button class="btn btn-sm btn-ghost text-error" @click="confirmDelete(item.cluster, item.name)">{{ t.common.delete }}</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>

    <!-- Reset Offset Modal using Teleport and DaisyUI modal -->
    <Teleport to="body">
      <dialog open v-if="resetOffsetModalOpen" class="modal modal-open">
        <div class="modal-box">
          <form method="dialog">
            <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2">✕</button>
          </form>
          <h3 class="font-bold text-lg mb-2">{{ t.consumerGroups.resetOffset }} - {{ resettingGroup }}</h3>
          <p class="text-sm text-base-content/60 mb-4">{{ t.dashboard.clusters }}: {{ resettingCluster }}</p>
          <form @submit.prevent="handleResetOffset">
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text font-semibold">{{ t.topics.topicName }}</span>
              </label>
              <input
                v-model="resetOffsetForm.topic"
                type="text"
                class="input input-bordered w-full"
                required
              />
            </div>
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text font-semibold">{{ t.consumerGroups.offsetType }}</span>
              </label>
              <select v-model="resetOffsetForm.type" class="select select-bordered w-full">
                <option value="earliest">{{ t.consumerGroups.earliest }}</option>
                <option value="latest">{{ t.consumerGroups.latest }}</option>
                <option value="value">{{ t.consumerGroups.specificValue }}</option>
                <option value="timestamp">{{ t.messages.timestampLabel }}</option>
              </select>
            </div>
            <div v-if="resetOffsetForm.type === 'value' || resetOffsetForm.type === 'timestamp'" class="form-control mb-4">
              <label class="label">
                <span class="label-text font-semibold">{{ t.consumerGroups.value }}</span>
              </label>
              <input
                v-model.number="resetOffsetForm.offsetValue"
                type="number"
                class="input input-bordered w-full"
                :required="resetOffsetForm.type === 'value' || resetOffsetForm.type === 'timestamp'"
              />
            </div>
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text font-semibold">{{ t.consumerGroups.partition }} ({{ t.common.optional }})</span>
              </label>
              <input
                v-model.number="resetOffsetForm.partition"
                type="number"
                class="input input-bordered w-full"
                :placeholder="t.consumerGroups.allPartitionsPlaceholder"
              />
            </div>
            <div class="modal-action">
              <button type="button" class="btn" @click="closeResetOffsetModal">{{ t.common.cancel }}</button>
              <button type="submit" class="btn btn-primary" :disabled="resetting">
                <span v-if="resetting" class="loading loading-spinner loading-sm"></span>
                {{ t.consumerGroups.resetOffset }}
              </button>
            </div>
          </form>
        </div>
        <div class="modal-backdrop" @click="closeResetOffsetModal"></div>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watchEffect } from 'vue';
import { useRoute, onBeforeRouteUpdate } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import type {
  ConsumerGroupDetailResponse,
  ConsumerGroupOffsetDetailResponse,
  ConsumerGroupOffsetsSummary,
  ConsumerGroupPartitionDetail,
} from '@/types/api';

// 定义本地类型
interface ConsumerGroupItem {
  name: string;
  cluster: string;
  state: string;
  memberCount: number;
  totalLag: number;
  topics: string[];
}

const route = useRoute();
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const selectedClusterIds = computed(() => clusterStore.selectedClusterIds);

// 从 URL 参数获取集群 ID - 直接使用响应式值
const clusterParam = computed(() => {
  const val = route.query.cluster;
  return Array.isArray(val) ? val[0] : (val || '');
});

// 从 URL 参数获取 group 名称
const groupParam = computed(() => {
  const val = route.query.group;
  return Array.isArray(val) ? val[0] : (val || '');
});

const viewMode = ref<'by-cluster' | 'all-groups'>('all-groups');
const loading = ref(false);
const error = ref<string | null>(null);

// Consumer groups data
const groupsByCluster = ref<Record<string, ConsumerGroupItem[]>>({});
const allGroupsList = ref<ConsumerGroupItem[]>([]);
const clusterGroups = ref<ConsumerGroupItem[]>([]);

// Selected group for detail view
const selectedGroup = ref<string | null>(null);
const loadingDetail = ref(false);

// 监听路由参数变化 - 使用路由守卫
onBeforeRouteUpdate((to) => {
  if (to.query.cluster) {
    fetchGroups();
  }
});

// 监听路由参数变化 - 使用 watchEffect
watchEffect(() => {
  if (clusterParam.value) {
    fetchGroups();
  } else if (selectedClusterIds.value.length > 0) {
    fetchGroups();
  }
});

const selectedGroupDetail = ref<ConsumerGroupDetailResponse | null>(null);
const offsetDetail = ref<ConsumerGroupOffsetDetailResponse | null>(null);

// Computed: Group partitions by topic
const partitionsByTopic = computed(() => {
  if (!offsetDetail.value?.partitions) return {};

  return offsetDetail.value.partitions.reduce((acc, p) => {
    const topic = p.topic || 'Unknown';
    if (!acc[topic]) {
      acc[topic] = [];
    }
    acc[topic].push(p);
    return acc;
  }, {} as Record<string, typeof offsetDetail.value.partitions>);
});

// Computed: Get unique topics list
const uniqueTopics = computed(() => {
  if (!offsetDetail.value?.partitions) return [];
  return Array.from(new Set(offsetDetail.value.partitions.map(p => p.topic || '').filter(Boolean)));
});

const resetOffsetModalOpen = ref(false);
const resettingGroup = ref<string>('');
const resettingCluster = ref<string>('');
const resetting = ref(false);

const resetOffsetForm = reactive({
  topic: '',
  type: 'latest' as 'earliest' | 'latest' | 'value' | 'timestamp',
  offsetValue: 0,
  partition: undefined as number | undefined,
});

function getStateBadge(state: string): string {
  switch (state.toLowerCase()) {
    case 'stable':
      return 'badge-success';
    case 'empty':
      return 'badge-ghost';
    case 'dead':
      return 'badge-error';
    case 'preparing_rebalance':
    case 'completing_rebalance':
      return 'badge-warning';
    default:
      return 'badge-ghost';
  }
}

function getTopicTotalLag(partitions: any[] | undefined | null): number {
  if (!partitions) return 0;
  return partitions.reduce((sum, p) => sum + (p.lag || 0), 0);
}

function getClusterHealth(clusterId: string) {
  return clusterStore.getClusterHealth(clusterId);
}

function formatLag(lag: number): string {
  if (lag >= 1000000) {
    return `${(lag / 1000000).toFixed(1)}M`;
  }
  if (lag >= 1000) {
    return `${(lag / 1000).toFixed(1)}K`;
  }
  return lag.toString();
}

async function fetchGroups() {
  loading.value = true;
  error.value = null;

  // 单集群模式（从 URL 参数）
  if (clusterParam.value) {
    try {
      // 使用新的 Consumer Offsets API 获取所有 Consumer Group 的 offsets 信息
      const offsetsData = await apiClient.getAllConsumerOffsets(clusterParam.value);

      // 转换为 ConsumerGroupItem 格式
      clusterGroups.value = offsetsData.consumer_groups.map((group: ConsumerGroupOffsetsSummary) => ({
        name: group.group_name,
        state: group.state,
        cluster: clusterParam.value as string,
        memberCount: 0, // Consumer Offsets API 不提供成员信息
        totalLag: group.total_lag,
        topics: group.topics.map(t => t.topic),
      }));

      // 存储完整的 offsets 数据用于详情展示
      groupsByCluster.value = {};
      allGroupsList.value = [];
    } catch (e) {
      error.value = (e as { message: string }).message;
    } finally {
      loading.value = false;
    }
    return;
  }

  // 多集群模式
  if (selectedClusterIds.value.length === 0) {
    loading.value = false;
    return;
  }

  groupsByCluster.value = {};
  allGroupsList.value = [];

  try {
    const promises = selectedClusterIds.value.map(async (clusterId) => {
      try {
        // 使用新的 Consumer Offsets API
        const offsetsData = await apiClient.getAllConsumerOffsets(clusterId);

        groupsByCluster.value[clusterId] = offsetsData.consumer_groups.map((group: ConsumerGroupOffsetsSummary) => ({
          name: group.group_name,
          state: group.state,
          cluster: clusterId,
          memberCount: 0,
          totalLag: group.total_lag,
          topics: group.topics.map(t => t.topic),
        }));
      } catch (e) {
        groupsByCluster.value[clusterId] = [];
      }
    });

    await Promise.all(promises);
    updateAllGroupsList();
  } catch (e) {
    error.value = (e as { message: string }).message;
  } finally {
    loading.value = false;
  }
}

function updateAllGroupsList() {
  allGroupsList.value = Object.entries(groupsByCluster.value).flatMap(([cluster, groups]) =>
    groups.map((g) => ({ ...g, cluster }))
  );
}

function openResetOffsetModal(clusterId: string, groupName: string) {
  resettingCluster.value = clusterId;
  resettingGroup.value = groupName;
  resetOffsetForm.topic = '';
  resetOffsetForm.type = 'latest';
  resetOffsetForm.offsetValue = 0;
  resetOffsetForm.partition = undefined;
  resetOffsetModalOpen.value = true;
}

function closeResetOffsetModal() {
  resetOffsetModalOpen.value = false;
  resettingGroup.value = '';
  resettingCluster.value = '';
}

async function handleResetOffset() {
  if (!resettingCluster.value) return;

  resetting.value = true;
  try {
    const offsetType = {
      type: resetOffsetForm.type,
      value: resetOffsetForm.type === 'value' || resetOffsetForm.type === 'timestamp'
        ? resetOffsetForm.offsetValue
        : undefined,
    };

    await apiClient.resetConsumerGroupOffset(
      resettingCluster.value,
      resettingGroup.value,
      {
        topic: resetOffsetForm.topic,
        partition: resetOffsetForm.partition,
        offset: offsetType as any,
      }
    );

    closeResetOffsetModal();
    alert('Offset reset successful!');
    fetchGroups();
  } catch (e) {
    alert((e as { message: string }).message);
  } finally {
    resetting.value = false;
  }
}

async function confirmDelete(clusterId: string, groupName: string) {
  if (confirm(`Are you sure you want to delete consumer group "${groupName}" from cluster "${clusterId}"?`)) {
    try {
      await apiClient.deleteConsumerGroup(clusterId, groupName);
      fetchGroups();
    } catch (e) {
      alert((e as { message: string }).message);
    }
  }
}

function selectGroup(groupName: string) {
  selectedGroup.value = groupName;
  loadGroupDetail(groupName);
}

function clearSelectedGroup() {
  selectedGroup.value = null;
  selectedGroupDetail.value = null;
  offsetDetail.value = null;
}

async function loadGroupDetail(groupName: string) {
  if (!clusterParam.value) return;

  loadingDetail.value = true;
  try {
    // 获取 Consumer Group 详情（成员信息）
    selectedGroupDetail.value = await apiClient.getConsumerGroupDetail(clusterParam.value, groupName);

    // 获取 Consumer Group offsets 详情
    const offsetsData = await apiClient.getAllConsumerOffsets(clusterParam.value);
    const groupOffsets = offsetsData.consumer_groups.find(g => g.group_name === groupName);

    if (groupOffsets) {
      // 将新的 offset 格式转换为 ConsumerGroupOffsetDetailResponse 格式
      const allPartitions: ConsumerGroupPartitionDetail[] = [];

      for (const topicOffset of groupOffsets.topics) {
        for (const p of topicOffset.partitions) {
          allPartitions.push({
            partition: p.partition,
            current_offset: p.current_offset,
            log_end_offset: p.end_offset,
            lag: p.lag,
            state: p.state,
            last_commit_time: undefined,
            topic: topicOffset.topic,
          });
        }
      }

      offsetDetail.value = {
        group_name: groupName,
        topic: groupOffsets.topics[0]?.topic || '',
        partitions: allPartitions,
        total_lag: groupOffsets.total_lag,
      };
    }
  } catch (e) {
    alert((e as { message: string }).message);
  } finally {
    loadingDetail.value = false;
  }
}

onMounted(() => {
  // 首次加载时获取 groups
  if (clusterParam.value) {
    fetchGroups();
  } else if (selectedClusterIds.value.length > 0) {
    fetchGroups();
  }

  // 如果有 group 参数，加载详情
  if (groupParam.value && clusterParam.value) {
    selectGroup(groupParam.value);
  }
});

// 监听 clusterParam 变化，自动刷新数据
watchEffect(() => {
  if (clusterParam.value) {
    fetchGroups();
  }
});
</script>
