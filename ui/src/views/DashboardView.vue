<template>
  <div class="p-6">
    <!-- Page Header -->
    <div class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold flex items-center gap-3">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-10 h-10">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
            </svg>
            {{ t.dashboard.title }}
          </h1>
          <p class="text-base-content/60 mt-2 text-lg">{{ t.dashboard.description }}</p>
        </div>
        <button class="btn btn-primary gap-2" @click="refreshAll">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5" :class="{ 'animate-spin': refreshing }">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
          </svg>
          {{ t.common.refresh }}
        </button>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex justify-center items-center py-20">
      <div class="text-center">
        <span class="loading loading-spinner loading-lg text-primary"></span>
        <p class="mt-4 text-base-content/60">{{ t.common.loading }}...</p>
      </div>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="alert alert-error gap-3">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" />
      </svg>
      <span>{{ error }}</span>
    </div>

    <!-- Dashboard Content -->
    <template v-else-if="clusters.length > 0">
      <!-- Stats Overview -->
      <div class="stats stats-vertical lg:stats-horizontal shadow w-full mb-8 glass-strong gradient-border">
        <div class="stat glass">
          <div class="stat-figure text-primary glow-primary">
            <div class="p-3 rounded-xl bg-primary/10">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
              </svg>
            </div>
          </div>
          <div class="stat-title">{{ t.dashboard.totalClusters }}</div>
          <div class="stat-value text-primary">{{ totalStats.totalClusters }}</div>
          <div class="stat-desc">
            <span class="text-success">{{ totalStats.healthyClusters }} {{ t.dashboard.healthyClusters }}</span>
            <span v-if="totalStats.totalClusters - totalStats.healthyClusters > 0" class="text-error ml-2">
              {{ totalStats.totalClusters - totalStats.healthyClusters }} {{ t.dashboard.unhealthyClusters }}
            </span>
          </div>
        </div>

        <div class="stat glass">
          <div class="stat-figure text-secondary glow-secondary">
            <div class="p-3 rounded-xl bg-secondary/10">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8">
                <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
              </svg>
            </div>
          </div>
          <div class="stat-title">{{ t.dashboard.totalTopics }}</div>
          <div class="stat-value text-secondary">{{ totalStats.totalTopics }}</div>
          <div class="stat-desc">{{ t.dashboard.partitions }}/{{ t.dashboard.topics }} avg</div>
        </div>

        <div class="stat glass">
          <div class="stat-figure text-accent">
            <div class="p-3 rounded-xl bg-accent/10">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 6h2.25A2.25 2.25 0 0110.5 8.25v9.75a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18V8.25A2.25 2.25 0 016 6zM6 6a1.125 1.125 0 00-1.125 1.125v9.75A1.125 1.125 0 006 18h2.25a1.125 1.125 0 001.125-1.125V8.25A1.125 1.125 0 008.25 6H6z" />
              </svg>
            </div>
          </div>
          <div class="stat-title">{{ t.dashboard.totalPartitions }}</div>
          <div class="stat-value text-accent">{{ formatNumber(totalStats.totalPartitions) }}</div>
          <div class="stat-desc">{{ t.dashboard.partitionsPerTopic }}</div>
        </div>

        <div class="stat glass">
          <div class="stat-figure text-warning">
            <div class="p-3 rounded-xl bg-warning/10">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8">
                <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198l.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.941-3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0z" />
              </svg>
            </div>
          </div>
          <div class="stat-title">{{ t.dashboard.consumerGroups }}</div>
          <div class="stat-value text-warning">{{ totalStats.totalConsumerGroups }}</div>
          <div class="stat-desc">{{ t.dashboard.totalLag }}: {{ formatLag(totalStats.totalLag) }}</div>
        </div>
      </div>

      <!-- Cluster Cards -->
      <div class="mb-8">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-xl font-semibold text-gradient">{{ t.dashboard.clusters }}</h2>
          <div class="flex gap-2">
            <button
              class="btn btn-sm btn-outline"
              @click="toggleAllSelection"
            >
              {{ selectedClusterIds.length === clusters.length ? t.dashboard.deselectAll : t.dashboard.selectAll }}
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div
            v-for="cluster in clusters"
            :key="cluster.id"
            class="card glass gradient-border transition-all duration-300 cursor-pointer"
            :class="{
              'ring-2 ring-primary ring-offset-2 ring-offset-base-100': selectedClusterIds.includes(cluster.name),
              'opacity-70': !getClusterHealth(cluster.name)?.healthy
            }"
            @click="toggleCluster(cluster.name)"
          >
            <div class="card-body p-4">
              <!-- Card Header -->
              <div class="flex items-center justify-between mb-2">
                <div class="flex items-center gap-3">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :checked="selectedClusterIds.includes(cluster.name)"
                    @click.stop="toggleCluster(cluster.name)"
                  />
                  <div class="flex items-center gap-2">
                    <div
                      class="relative w-3 h-3 status-dot"
                    >
                      <span
                        class="absolute inset-0 rounded-full"
                        :class="{
                          'bg-success glow-success': getClusterHealth(cluster.name)?.healthy,
                          'bg-error glow-error': getClusterHealth(cluster.name)?.healthy === false,
                          'bg-warning glow-warning': getClusterHealth(cluster.name)?.healthy === undefined
                        }"
                      ></span>
                    </div>
                    <span class="font-semibold text-base text-gradient">{{ cluster.name }}</span>
                  </div>
                </div>
                <div
                  class="badge gap-1 badge-primary"
                  :class="getClusterHealth(cluster.name)?.healthy ? 'badge-success' : 'badge-error'"
                >
                  {{ getClusterHealth(cluster.name)?.healthy ? t.common.connected : t.common.disconnected }}
                </div>
              </div>

              <!-- Card Body -->
              <div class="text-sm text-base-content/60 truncate mb-3">
                {{ cluster.brokers }}
              </div>

              <div v-if="getClusterHealth(cluster.name)?.stats" class="grid grid-cols-3 gap-3 mb-3">
                <div class="text-center">
                  <div class="text-xs uppercase tracking-wide text-base-content/50">{{ t.dashboard.topics }}</div>
                  <div class="text-lg font-semibold">{{ getClusterHealth(cluster.name)?.stats?.topic_count || 0 }}</div>
                </div>
                <div class="text-center">
                  <div class="text-xs uppercase tracking-wide text-base-content/50">{{ t.dashboard.partitions }}</div>
                  <div class="text-lg font-semibold">{{ getClusterHealth(cluster.name)?.stats?.partition_count || 0 }}</div>
                </div>
                <div class="text-center">
                  <div class="text-xs uppercase tracking-wide text-base-content/50">{{ t.dashboard.lag }}</div>
                  <div class="text-lg font-semibold text-warning">{{ formatLag(getClusterHealth(cluster.name)?.stats?.total_lag || 0) }}</div>
                </div>
              </div>

              <div v-if="getClusterHealth(cluster.name)?.lastChecked" class="text-xs text-base-content/50">
                {{ t.dashboard.lastChecked }}: {{ formatRelativeTime(getClusterHealth(cluster.name)?.lastChecked) }}
              </div>

              <!-- Card Actions -->
              <div class="card-actions justify-end mt-3 pt-3 border-t border-base-200">
                <router-link :to="`/topics?cluster=${cluster.name}`" class="btn btn-primary btn-sm gap-1">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                  </svg>
                  {{ t.dashboard.topics }}
                </router-link>
                <router-link :to="`/consumer-groups?cluster=${cluster.name}`" class="btn btn-ghost btn-sm gap-1">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198l.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.941-3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0z" />
                  </svg>
                  {{ t.consumerGroups.title }}
                </router-link>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Lag Chart Placeholder -->
      <div class="card bg-base-100 shadow-md mb-8">
        <div class="card-body">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-lg font-semibold">{{ t.dashboard.consumerLagTitle }}</h2>
            <select class="select select-bordered select-sm w-auto">
              <option value="24h">{{ t.dashboard.last24Hours }}</option>
              <option value="7d">{{ t.dashboard.last7Days }}</option>
              <option value="30d">{{ t.dashboard.last30Days }}</option>
            </select>
          </div>
          <div class="border-2 border-dashed border-base-300 rounded-box min-h-[240px] flex items-center justify-center">
            <div class="text-center">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16 text-base-content/30 mx-auto">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125z" />
              </svg>
              <p class="text-base-content/60 mt-4">{{ t.dashboard.chartComingSoon }}</p>
              <p class="text-base-content/40 text-sm mt-2">{{ t.dashboard.totalLag }}: {{ formatLag(totalStats.totalLag) }}</p>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Empty State -->
    <div v-else class="flex flex-col items-center justify-center py-16 text-center">
      <div class="text-base-content/30 mb-6">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-20 h-20">
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
        </svg>
      </div>
      <h3 class="text-xl font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 mb-6">{{ t.clusters.description }}</p>
      <router-link to="/clusters" class="btn btn-primary gap-2">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        {{ t.clusters.addCluster }}
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';

const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const clusters = computed(() => clusterStore.clusters);
const selectedClusterIds = computed(() => clusterStore.selectedClusterIds);
const loading = computed(() => clusterStore.loading);
const error = computed(() => clusterStore.error);
const totalStats = computed(() => clusterStore.totalStats);
const refreshing = ref(false);

function formatNumber(num: number): string {
  if (num >= 1000000) {
    return `${(num / 1000000).toFixed(1)}M`;
  }
  if (num >= 1000) {
    return `${(num / 1000).toFixed(0)}K`;
  }
  return num.toString();
}

function formatLag(lag: number): string {
  if (lag >= 1000000) {
    return `${(lag / 1000000).toFixed(1)}M`;
  }
  if (lag >= 1000) {
    return `${(lag / 1000).toFixed(0)}K`;
  }
  return lag.toString();
}

function formatRelativeTime(timestamp: number | undefined): string {
  if (!timestamp) return 'Never';
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  if (diff < 60000) return 'Just now';
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  return date.toLocaleDateString();
}

function getClusterHealth(clusterId: string) {
  return clusterStore.getClusterHealth(clusterId);
}

function toggleCluster(clusterId: string) {
  clusterStore.toggleClusterSelection(clusterId);
}

function toggleAllSelection() {
  if (selectedClusterIds.value.length === clusters.value.length) {
    clusterStore.clearSelection();
  } else {
    clusterStore.selectAllClusters();
  }
}

async function refreshAll() {
  refreshing.value = true;
  try {
    await clusterStore.refreshAllHealth();
  } finally {
    refreshing.value = false;
  }
}

onMounted(() => {
  clusterStore.fetchClusters();
  clusterStore.refreshAllHealth();
});
</script>
