<template>
  <div class="h-full flex flex-col overflow-auto bg-base-100">
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-base-300 bg-base-200">
      <div class="flex items-center gap-3">
        <button class="btn btn-square btn-ghost btn-sm" @click="goBack">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
          </svg>
        </button>
        <div>
          <h1 class="text-lg font-semibold text-base-content">{{ topicName }}</h1>
          <p class="text-xs text-base-content/60">{{ t.consumerLag.cluster }}: {{ clusterName }}</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button class="btn btn-ghost btn-sm" @click="refreshData" :disabled="loading">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4" :class="{ 'animate-spin': loading }">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
          </svg>
          {{ t.consumerLag.refresh }}
        </button>
      </div>
    </div>

    <!-- Summary Cards -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 p-4">
      <div class="stat bg-base-200 rounded-box p-4">
        <div class="stat-title text-base-content/60">{{ t.consumerLag.totalLag }}</div>
        <div class="stat-value text-lg" :class="{ 'text-error': totalLag > 10000, 'text-warning': totalLag > 1000 && totalLag <= 10000 }">
          {{ formatNumber(totalLag) }}
        </div>
        <div class="stat-desc text-base-content/60">{{ t.consumerLag.messages }}</div>
      </div>
      <div class="stat bg-base-200 rounded-box p-4">
        <div class="stat-title text-base-content/60">{{ t.consumerLag.consumerGroups }}</div>
        <div class="stat-value text-lg text-primary">{{ consumerGroups.length }}</div>
        <div class="stat-desc text-base-content/60">{{ t.consumerLag.groups }}</div>
      </div>
      <div class="stat bg-base-200 rounded-box p-4">
        <div class="stat-title text-base-content/60">{{ t.consumerLag.partitions }}</div>
        <div class="stat-value text-lg text-secondary">{{ partitionCount }}</div>
        <div class="stat-desc text-base-content/60">{{ t.consumerLag.partitions }}</div>
      </div>
      <div class="stat bg-base-200 rounded-box p-4">
        <div class="stat-title text-base-content/60">{{ t.consumerLag.maxLagGroup }}</div>
        <div class="stat-value text-lg truncate" :title="maxLagGroup">
          {{ maxLagGroup || '-' }}
        </div>
        <div class="stat-desc text-base-content/60">{{ t.consumerLag.groups }}</div>
      </div>
    </div>

    <!-- Lag Chart -->
    <div class="p-4">
      <div class="card bg-base-200 rounded-box border border-base-300">
        <div class="card-body p-4">
          <h3 class="card-title text-sm">{{ t.consumerLag.lagTrend }}</h3>
          <div v-if="loading" class="flex items-center justify-center h-64">
            <span class="loading loading-spinner loading-md"></span>
            <span class="ml-2 text-sm text-base-content/60">{{ t.consumerLag.loadingData }}</span>
          </div>
          <div v-else-if="chartData.length === 0" class="flex items-center justify-center h-64">
            <span class="text-sm text-base-content/60">{{ t.consumerLag.noHistoricalData }}</span>
          </div>
          <div v-else class="chart-wrapper" style="height: 300px;">
            <canvas ref="chartCanvas"></canvas>
          </div>
        </div>
      </div>
    </div>

    <!-- Consumer Group Details -->
    <div class="p-4">
      <div class="card bg-base-200 rounded-box border border-base-300">
        <div class="card-body p-4">
          <h3 class="card-title text-sm">{{ t.consumerLag.consumerGroupDetails }}</h3>
          <div class="overflow-x-auto">
            <table class="table table-zebra w-full text-sm">
              <thead>
                <tr>
                  <th class="font-medium text-base-content/60">{{ t.consumerLag.consumerGroups }}</th>
                  <th class="text-right font-medium text-base-content/60">{{ t.consumerLag.totalLag }}</th>
                  <th class="font-medium text-base-content/60">{{ t.consumerLag.partitionDetails }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="group in consumerGroups" :key="group.group_name">
                  <td class="font-medium">{{ group.group_name }}</td>
                  <td class="text-right">
                    <span class="badge" :class="{
                      'badge-error text-error': group.total_lag > 10000,
                      'badge-warning text-warning': group.total_lag > 1000 && group.total_lag <= 10000,
                      'badge-success text-success': group.total_lag <= 1000
                    }">
                      {{ formatNumber(group.total_lag) }}
                    </span>
                  </td>
                  <td>
                    <div class="flex flex-wrap gap-1">
                      <span
                        v-for="p in group.partitions"
                        :key="p.partition"
                        class="badge badge-sm"
                        :class="{
                          'badge-error text-error': p.lag > 1000,
                          'badge-warning text-warning': p.lag > 100 && p.lag <= 1000,
                          'badge-success text-success': p.lag <= 100
                        }"
                        :title="`Partition ${p.partition}: ${p.lag} lag`"
                      >
                        P{{ p.partition }}: {{ formatNumber(p.lag) }}
                      </span>
                    </div>
                  </td>
                </tr>
                <tr v-if="consumerGroups.length === 0">
                  <td colspan="3" class="py-8 text-center text-base-content/60">
                    {{ t.consumerLag.noConsumerGroupsDesc }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick, computed } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import Chart from 'chart.js/auto';

const router = useRouter();
const route = useRoute();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const clusterName = ref(route.query.cluster as string || '');
const topicName = ref(route.query.topic as string || '');

const loading = ref(false);
const totalLag = ref(0);
const partitionCount = ref(0);
const maxLagGroup = ref('');
const consumerGroups = ref<Array<{
  group_name: string;
  total_lag: number;
  partitions: Array<{ partition: number; lag: number; current_offset: number; log_end_offset: number; state: string }>;
}>>([]);

const chartData = ref<Array<{ timestamp: number; groups: Array<{ group_id: string; total_lag: number }> }>>([]);
const chartCanvas = ref<HTMLCanvasElement>();
let chartInstance: Chart | null = null;

async function loadConsumerLag() {
  if (!clusterName.value || !topicName.value) return;

  loading.value = true;
  try {
    // 获取当前 consumer lag
    const lagData = await apiClient.getConsumerLag(clusterName.value, topicName.value);
    totalLag.value = lagData.total_lag || 0;
    consumerGroups.value = lagData.consumer_groups || [];
    partitionCount.value = consumerGroups.value[0]?.partitions?.length || 0;

    // 找到 lag 最大的 group
    if (consumerGroups.value.length > 0) {
      const maxGroup = consumerGroups.value.reduce((max, g) => g.total_lag > (max?.total_lag ?? 0) ? g : max, consumerGroups.value[0]);
      maxLagGroup.value = maxGroup ? maxGroup.group_name : '';
    }

    // 获取历史数据
    const historyData = await apiClient.getConsumerLagHistory(clusterName.value, topicName.value);
    // 转换历史数据格式
    chartData.value = historyData.timestamps.map((timestamp, idx) => ({
      timestamp,
      groups: historyData.consumer_groups.map(g => ({
        group_id: g.group_name,
        total_lag: g.lag_series[idx] || 0
      }))
    }));
    await nextTick();
    renderChart();
  } catch (error) {
    console.error('Failed to load consumer lag:', error);
  } finally {
    loading.value = false;
  }
}

function renderChart() {
  if (!chartCanvas.value || chartData.value.length === 0) return;

  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  // 销毁旧图表
  if (chartInstance) {
    chartInstance.destroy();
  }

  // 准备数据
  const timestamps = chartData.value.map(d => new Date(d.timestamp).toLocaleString());
  const groupIds = Array.from(new Set(chartData.value.flatMap(d => d.groups.map(g => g.group_id))));

  const datasets = groupIds.map((groupId, index) => ({
    label: groupId,
    data: chartData.value.map(d => {
      const group = d.groups.find(g => g.group_id === groupId);
      return group ? group.total_lag : 0;
    }),
    borderColor: `hsl(${index * 60}, 70%, 50%)`,
    backgroundColor: `hsl(${index * 60}, 70%, 50%, 0.1)`,
    tension: 0.3,
    fill: false,
  }));

  chartInstance = new Chart(ctx, {
    type: 'line',
    data: { labels: timestamps, datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: 'bottom',
          labels: { boxWidth: 12, font: { size: 10 } }
        }
      },
      scales: {
        y: {
          beginAtZero: true,
          ticks: {
            callback: (value: number | string) => formatNumber(Number(value))
          }
        },
        x: {
          display: false
        }
      }
    }
  });
}

function formatNumber(num: number): string {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
  if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
  return num.toString();
}

function refreshData() {
  loadConsumerLag();
}

function goBack() {
  router.back();
}

onMounted(() => {
  loadConsumerLag();
});
</script>
