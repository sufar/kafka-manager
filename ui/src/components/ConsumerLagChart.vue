<template>
  <div class="consumer-lag-chart card glass gradient-border shadow-xl">
    <!-- 图表头部 -->
    <div class="p-3 bg-base-100 border-b border-base-200 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 3v11.25A2.25 2.25 0 006 16.5h2.25M6 16.5v2.25m2.25-2.25v2.25m4.5 0c0-2.25 1.5-4.5 3-6.75m-3 6.75v-2.25m2.25 2.25V16.5m2.25 2.25v-4.5c0-1.5.75-3 1.5-4.5m-9 9.75h9.75m-9.75 0A2.25 2.25 0 013 18.75V3m18 0a2.25 2.25 0 012.25 2.25v13.5A2.25 2.25 0 0121 21H6" />
        </svg>
        <h3 class="font-semibold">{{ t.consumerGroups.lagTrend }}</h3>
      </div>
      <div class="flex items-center gap-2">
        <!-- 刷新间隔选择 -->
        <select v-model="refreshInterval" class="select select-bordered select-sm" @change="restartAutoRefresh">
          <option :value="3000">{{ t.common.seconds }}: 3</option>
          <option :value="5000">{{ t.common.seconds }}: 5</option>
          <option :value="10000">{{ t.common.seconds }}: 10</option>
          <option :value="30000">{{ t.common.minutes }}: 0.5</option>
        </select>
        <!-- 手动刷新按钮 -->
        <button class="btn btn-ghost btn-sm" @click="fetchLagData" :disabled="isFetching" :title="t.common.refresh">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4" :class="{ 'animate-spin': isFetching }">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
          </svg>
        </button>
        <!-- 暂停/继续按钮 -->
        <button
          class="btn btn-ghost btn-sm"
          @click="togglePause"
          :title="isPaused ? t.common.resume : t.common.pause"
        >
          <svg v-if="isPaused" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M5.25 5.653c0-.856.917-1.398 1.667-.986l11.54 6.347a1.125 1.125 0 010 1.972l-11.54 6.347a1.125 1.125 0 01-1.667-.986V5.653z" />
          </svg>
          <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 5.25v13.5m-7.5-13.5v13.5" />
          </svg>
        </button>
      </div>
    </div>

    <!-- 图表容器 -->
    <div class="p-4 relative" style="height: 300px;">
      <!-- 空数据状态 -->
      <div v-if="!isFetching && (!chartData.labels || chartData.labels.length === 0)" class="absolute inset-0 flex items-center justify-center text-base-content/60">
        <div class="text-center">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-12 h-12 mx-auto mb-2 text-base-content/30">
            <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 3v11.25A2.25 2.25 0 006 16.5h2.25M6 16.5v2.25m2.25-2.25v2.25m4.5 0c0-2.25 1.5-4.5 3-6.75m-3 6.75v-2.25m2.25 2.25V16.5m2.25 2.25v-4.5c0-1.5.75-3 1.5-4.5m-9 9.75h9.75m-9.75 0A2.25 2.25 0 013 18.75V3m18 0a2.25 2.25 0 012.25 2.25v13.5A2.25 2.25 0 0121 21H6" />
          </svg>
          <p class="text-sm">{{ t.consumerGroups.noLagData }}</p>
          <p class="text-xs text-base-content/60 mt-1">{{ t.consumerGroups.waitingForData }}</p>
        </div>
      </div>

      <!-- Canvas -->
      <canvas ref="chartCanvas"></canvas>
    </div>

    <!-- 图表底部状态栏 -->
    <div class="p-2 bg-base-200 border-t border-base-300 flex items-center justify-between text-xs">
      <div class="flex items-center gap-4">
        <span class="flex items-center gap-1">
          <span class="w-3 h-3 rounded" style="background-color: rgb(255, 99, 132);"></span>
          <span>{{ t.consumerGroups.totalLag }}</span>
        </span>
        <span v-if="latestLag !== null" class="font-mono font-bold" :class="getLagClass(latestLag)">
          {{ formatLag(latestLag) }}
        </span>
      </div>
      <div class="text-base-content/60">
        <span v-if="lastUpdateTime">{{ t.consumerGroups.lastUpdate }}: {{ formatTime(lastUpdateTime) }}</span>
        <span v-else>{{ t.consumerGroups.noDataYet }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { Chart, registerables } from 'chart.js';
import type { ChartData, ChartOptions } from 'chart.js';
import 'chartjs-adapter-date-fns';
import { apiClient } from '@/api/client';
import { useLanguageStore } from '@/stores/language';
import type { LagHistoryDataPoint } from '@/types/api';

// 注册 Chart.js 组件
Chart.register(...registerables);

const props = defineProps<{
  clusterId: string;
  groupName: string;
}>();

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

const chartCanvas = ref<HTMLCanvasElement | null>(null);
const chart = ref<Chart | null>(null);
const refreshInterval = ref<number>(5000);
const isFetching = ref(false);
const isPaused = ref(false);
const lastUpdateTime = ref<number | null>(null);
const latestLag = ref<number | null>(null);

// 图表数据
const chartData = ref<ChartData<'line'>>({
  labels: [],
  datasets: [
    {
      label: 'Total Lag',
      data: [] as number[],
      borderColor: 'rgb(255, 99, 132)',
      backgroundColor: 'rgba(255, 99, 132, 0.1)',
      tension: 0.3,
      fill: true,
      pointRadius: 2,
      pointHoverRadius: 4,
    },
  ],
});

// 图表选项
const chartOptions: ChartOptions<'line'> = {
  responsive: true,
  maintainAspectRatio: false,
  animation: false,
  interaction: {
    intersect: false,
    mode: 'index',
  },
  plugins: {
    legend: {
      display: false,
    },
    tooltip: {
      callbacks: {
        label: (context) => {
          const y = context.parsed.y;
          return `Lag: ${y != null ? y.toLocaleString() : '0'}`;
        },
        title: (items) => {
          if (items.length > 0) {
            const timestamp = items[0]?.parsed.x;
            return new Date(timestamp ?? 0).toLocaleString();
          }
          return '';
        },
      },
    },
  },
  scales: {
    x: {
      type: 'linear',
      ticks: {
        callback: (value) => {
          return new Date(Number(value)).toLocaleTimeString();
        },
        maxRotation: 45,
        minRotation: 45,
      },
    },
    y: {
      beginAtZero: true,
      ticks: {
        callback: (value) => {
          if (Number(value) >= 1000000) {
            return (Number(value) / 1000000).toFixed(1) + 'M';
          }
          if (Number(value) >= 1000) {
            return (Number(value) / 1000).toFixed(1) + 'K';
          }
          return value.toString();
        },
      },
    },
  },
};

let timer: ReturnType<typeof setInterval> | null = null;

// 初始化图表
function initChart() {
  if (!chartCanvas.value) return;

  chart.value = new Chart(chartCanvas.value, {
    type: 'line',
    data: chartData.value,
    options: chartOptions,
  });
}

// 获取 Lag 数据
async function fetchLagData() {
  if (!props.clusterId || !props.groupName) return;

  isFetching.value = true;

  try {
    // 获取最近 1 小时的数据
    const endTime = Date.now();
    const startTime = endTime - 60 * 60 * 1000;

    const history = await apiClient.getConsumerGroupLagHistory(
      props.clusterId,
      props.groupName,
      startTime,
      endTime
    );

    updateChart(history);
    lastUpdateTime.value = Date.now();

    if (history.length > 0) {
      const lastItem = history[history.length - 1];
      latestLag.value = lastItem?.total_lag ?? 0;
    }
  } catch (e) {
    console.error('[ConsumerLagChart] Failed to fetch lag data:', e);
    // 静默失败，不影响自动刷新
  } finally {
    isFetching.value = false;
  }
}

// 更新图表数据
function updateChart(history: LagHistoryDataPoint[]) {
  if (!chart.value || history.length === 0) return;

  // 转换为图表数据
  const labels = history.map((item) => item.timestamp);
  const data = history.map((item) => item.total_lag);

  if (chart.value.data.labels) {
    chart.value.data.labels = labels;
  }
  if (chart.value.data.datasets && chart.value.data.datasets[0]) {
    chart.value.data.datasets[0].data = data;
  }
  chart.value.update();
}

// 自动刷新
function startAutoRefresh() {
  stopAutoRefresh();
  if (!isPaused.value) {
    timer = setInterval(() => {
      fetchLagData();
    }, refreshInterval.value);
  }
}

function stopAutoRefresh() {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
}

function restartAutoRefresh() {
  stopAutoRefresh();
  startAutoRefresh();
}

function togglePause() {
  isPaused.value = !isPaused.value;
  if (isPaused.value) {
    stopAutoRefresh();
  } else {
    startAutoRefresh();
  }
}

// 格式化 Lag 值
function formatLag(lag: number): string {
  if (lag >= 1000000) {
    return (lag / 1000000).toFixed(2) + 'M';
  }
  if (lag >= 1000) {
    return (lag / 1000).toFixed(2) + 'K';
  }
  return lag.toString();
}

// 根据 Lag 值获取样式类
function getLagClass(lag: number): string {
  if (lag === 0) return 'text-success';
  if (lag < 1000) return 'text-warning';
  return 'text-error';
}

// 格式化时间
function formatTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString();
}

// 监听 clusterId 和 groupName 变化
watch(
  [() => props.clusterId, () => props.groupName],
  () => {
    fetchLagData();
  },
  { immediate: true }
);

onMounted(() => {
  initChart();
  fetchLagData();
  startAutoRefresh();
});

onUnmounted(() => {
  stopAutoRefresh();
  if (chart.value) {
    chart.value.destroy();
    chart.value = null;
  }
});
</script>

<style scoped>
.consumer-lag-chart {
  display: flex;
  flex-direction: column;
}

.consumer-lag-chart :deep(canvas) {
  max-height: 100%;
}
</style>
