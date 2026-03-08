<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between p-2 mb-2 flex-shrink-0">
      <div class="flex items-center gap-2">
        <div class="w-8 h-8 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center glow-primary animate-float">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
            <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
          </svg>
        </div>
        <span class="text-xs font-bold text-base-content/60 uppercase tracking-wider">Clusters</span>
      </div>
      <div class="flex gap-0.5">
        <button class="btn btn-ghost btn-xs" @click="handleAddCluster" title="Add cluster">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-success">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-xs" @click="expandAll" title="Expand all">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-xs" @click="collapseAll" title="Collapse all">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Tree Content - Scrollable Area -->
    <div class="flex-1 overflow-y-auto p-2">
      <div v-for="cluster in clusters" :key="cluster.name" class="mb-1">
        <!-- Cluster Node -->
        <div
          class="flex items-center p-2 rounded-xl cursor-pointer transition-all duration-300 hover:bg-primary/5 hover:shadow-md sticky top-0 z-30 bg-base-100/95 backdrop-blur-sm"
          :class="{ 'bg-primary/10 shadow-inner': expandedClusters.has(cluster.name) }"
          @contextmenu.prevent="showClusterMenu($event, cluster.name)"
          @dblclick="toggleCluster(cluster.name)"
        >
          <div class="flex items-center gap-1.5 flex-1 min-w-0">
            <button class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0" @click.stop="toggleCluster(cluster.name)" tabindex="-1">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="2"
                stroke="currentColor"
                class="w-3 h-3 transition-transform duration-200"
                :class="{ 'rotate-90': expandedClusters.has(cluster.name) }"
              >
                <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
              </svg>
            </button>
            <div
              class="w-2 h-2 rounded-full transition-colors duration-300"
              :class="{
                'bg-success shadow-[0_0_4px_rgba(16,185,129,0.4)]': getClusterHealth(cluster.name)?.healthy === true && !refreshingClusters.has(cluster.name),
                'bg-error shadow-[0_0_4px_rgba(239,68,68,0.4)]': getClusterHealth(cluster.name)?.healthy === false && !refreshingClusters.has(cluster.name),
                'bg-warning shadow-[0_0_4px_rgba(245,158,11,0.4)]': getClusterHealth(cluster.name)?.healthy === undefined && !refreshingClusters.has(cluster.name),
                'bg-warning animate-pulse': refreshingClusters.has(cluster.name)
              }"
            ></div>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary flex-shrink-0">
              <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
            </svg>
            <span class="text-sm font-semibold truncate">{{ cluster.name }}</span>
          </div>
        </div>

        <!-- Cluster Children -->
        <div v-show="expandedClusters.has(cluster.name)" class="flex flex-col">
          <!-- Topics Folder -->
          <div class="mb-1 sticky top-0 z-20 bg-base-100/95 backdrop-blur-sm flex-shrink-0">
            <div
              class="flex items-center p-2 rounded-xl cursor-pointer transition-all duration-300 hover:bg-secondary/5 hover:shadow-md relative"
              :class="{ 'bg-secondary/10 shadow-inner': expandedTopicsFolders.has(cluster.name) }"
              @click.stop="handleTopicsFolderClickAndExpand(cluster.name)"
              @contextmenu.prevent="showTopicsFolderMenu($event, cluster.name)"
            >
              <div class="flex items-center gap-1.5 flex-1 min-w-0">
                <button class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0" @click.stop="handleTopicsFolderToggle(cluster.name)" tabindex="-1">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="2"
                    stroke="currentColor"
                    class="w-3 h-3 transition-transform duration-200"
                    :class="{ 'rotate-90': expandedTopicsFolders.has(cluster.name) }"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                  </svg>
                </button>
                <span class="text-sm truncate">{{ topicCounts[cluster.name] || 0 }}</span>
                <span class="text-sm truncate">Topics</span>
              </div>
              <!-- Topics Refresh Button -->
              <button
                class="btn btn-ghost btn-xs p-0 w-6 h-6 min-h-0 ml-1 opacity-0 hover:opacity-100 transition-opacity"
                :class="{ 'opacity-100': refreshingClusters.has(cluster.name) }"
                @click.stop="refreshClusterTopics(cluster.name)"
                :disabled="refreshingClusters.has(cluster.name)"
                title="Refresh Topics"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke-width="1.5"
                  stroke="currentColor"
                  class="w-3.5 h-3.5"
                  :class="{ 'animate-spin': refreshingClusters.has(cluster.name) }"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                </svg>
              </button>
            </div>

            <!-- Topics Scrollable Container (includes search box + topics list) -->
            <div v-show="expandedTopicsFolders.has(cluster.name)" class="pl-4 overflow-y-auto max-h-[500px]">
              <!-- Topic Search Box - sticky under Topics folder -->
              <div v-if="getTotalTopics(cluster.name) > 0" class="mb-2 sticky top-0 bg-base-100 z-10 py-1">
                <div class="join w-full">
                  <input
                    v-model="topicSearchQuery[cluster.name]"
                    type="text"
                    :placeholder="`Search ${getTotalTopics(cluster.name)} topics...`"
                    class="input input-bordered input-xs join-item w-full"
                    @click.stop
                  />
                  <button
                    v-if="topicSearchQuery[cluster.name]"
                    class="btn join-item btn-ghost btn-xs"
                    @click.stop="setTopicSearch(cluster.name, '')"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
                <p v-if="!topicSearchQuery[cluster.name]" class="text-xs text-base-content/50 mt-1">
                  Showing {{ getTotalTopics(cluster.name) }} topics
                </p>
                <p v-else class="text-xs text-primary mt-1">
                  Found {{ getClusterTopics(cluster.name).length }} matching topics
                </p>
              </div>

              <div
                v-for="topic in getClusterTopics(cluster.name)"
                :key="topic.name"
                class="mb-1"
                ref="el => setTopicElementRef(`${cluster.name}:${topic.name}`, el)"
              >
                <!-- Topic Node -->
                <div
                  class="flex items-center p-2 rounded-xl cursor-pointer transition-all duration-300 hover:bg-accent/5 hover:shadow-md"
                  :class="{ 'bg-accent/10 shadow-inner text-accent': selectedTopic?.name === topic.name && selectedTopic?.cluster === cluster.name }"
                  @contextmenu.prevent="showContextMenu($event, topic.name, cluster.name)"
                  @click="selectTopic(topic, cluster.name)"
                >
                  <div class="flex items-center gap-1.5 flex-1 min-w-0">
                    <button
                      class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0"
                      @click.stop="toggleTopic(topic.name, cluster.name)"
                      tabindex="-1"
                    >
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="2"
                        stroke="currentColor"
                        class="w-3 h-3 transition-transform duration-200"
                        :class="{ 'rotate-90': expandedTopics.has(`${cluster.name}:${topic.name}`) }"
                      >
                        <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                      </svg>
                    </button>
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-secondary flex-shrink-0">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                    </svg>
                    <span class="text-sm truncate">{{ topic.name }}</span>
                  </div>
                </div>

                <!-- Partitions List -->
                <div v-show="expandedTopics.has(`${cluster.name}:${topic.name}`)" class="pl-4">
                  <div
                    v-for="partition in topic.partitions || []"
                    :key="partition.id"
                    class="flex items-center p-2 rounded-xl cursor-pointer transition-all duration-300 hover:bg-base-200/50 hover:shadow-md"
                    :class="{ 'bg-accent/10 shadow-inner text-accent': selectedPartition?.topic === topic.name && selectedPartition?.partition === partition.id && selectedPartition?.cluster === cluster.name }"
                    @click="selectPartition(partition.id, topic, cluster.name)"
                    @contextmenu.prevent="showPartitionMenu($event, topic.name, cluster.name, partition.id)"
                  >
                    <div class="flex items-center gap-1.5 flex-1 min-w-0">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-base-content/40 flex-shrink-0">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
                      </svg>
                      <span class="text-xs truncate text-base-content/70">Partition {{ partition.id }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Consumer Groups Folder -->
          <div class="mb-1 flex-shrink-0">
            <div
              class="flex items-center p-2 rounded-xl cursor-pointer transition-all duration-300 hover:bg-success/5 hover:shadow-md relative"
              :class="{ 'bg-success/10 shadow-inner': expandedConsumerGroupsFolders.has(cluster.name) }"
              @click.stop="handleConsumerGroupsFolderClick(cluster.name)"
            >
              <div class="flex items-center gap-1.5 flex-1 min-w-0 pr-7">
                <button class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0" @click.stop="handleConsumerGroupsFolderToggle(cluster.name)" tabindex="-1">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="2"
                    stroke="currentColor"
                    class="w-3 h-3 transition-transform duration-200"
                    :class="{ 'rotate-90': expandedConsumerGroupsFolders.has(cluster.name) }"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                  </svg>
                </button>
                <span class="text-sm truncate">{{ consumerGroupCounts[cluster.name] || 0 }}</span>
                <span class="text-sm truncate">Consumer Groups</span>
              </div>
              <!-- Consumer Groups Refresh Button -->
              <button
                class="btn btn-ghost btn-xs p-0 w-6 h-6 min-h-0 ml-1 absolute right-2 opacity-0 hover:opacity-100 transition-opacity"
                :class="{ 'opacity-100': refreshingConsumerGroups.has(cluster.name) }"
                @click.stop="refreshClusterConsumerGroups(cluster.name)"
                :disabled="refreshingConsumerGroups.has(cluster.name)"
                title="Refresh Consumer Groups"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke-width="1.5"
                  stroke="currentColor"
                  class="w-3.5 h-3.5"
                  :class="{ 'animate-spin': refreshingConsumerGroups.has(cluster.name) }"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                </svg>
              </button>
            </div>

            <!-- Consumer Groups List -->
            <div v-show="expandedConsumerGroupsFolders.has(cluster.name)" class="pl-4 overflow-y-auto max-h-[150px]">
              <div
                v-for="group in getClusterConsumerGroups(cluster.name)"
                :key="group.groupId"
                class="mb-1"
              >
                <!-- Consumer Group Node -->
                <div
                  class="flex items-center p-2 rounded-xl cursor-pointer transition-all duration-300 hover:bg-success/5 hover:shadow-md"
                  :class="{ 'bg-success/10 shadow-inner text-success': selectedConsumerGroup?.groupId === group.groupId && selectedConsumerGroup?.cluster === cluster.name }"
                  @click="selectConsumerGroup(group, cluster.name)"
                >
                  <div class="flex items-center gap-1.5 flex-1 min-w-0">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-success flex-shrink-0">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.941-3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.625 2.625 0 11-4.5 0 2.625 2.625 0 014.5 0z" />
                    </svg>
                    <span class="text-sm truncate">{{ group.groupId }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Connection Error Dialog -->
    <dialog ref="errorDialogRef" class="modal modal-bottom sm:modal-middle">
      <form method="dialog" class="modal-box">
        <h3 class="font-bold text-lg mb-2">Connection Failed</h3>
        <p class="text-sm text-base-content/70 mb-1">Cluster: <span class="font-mono">{{ errorDialogClusterName }}</span></p>
        <p class="text-sm text-error mb-4">{{ errorDialogMessage }}</p>
        <div class="flex justify-end gap-2">
          <button class="btn btn-ghost btn-sm" @click="handleErrorDialogClose">Cancel</button>
          <button class="btn btn-error btn-sm" @click="handleErrorDialogRetry">Retry</button>
        </div>
      </form>
      <form method="dialog" class="modal-backdrop">
        <button @click="handleErrorDialogClose">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue';
import { useClusterStore } from '@/stores/cluster';
import { apiClient } from '@/api/client';

interface Topic {
  name: string;
  partitions?: Array<{ id: number }>;
}

interface ConsumerGroup {
  groupId: string;
}

const emit = defineEmits<{
  navigate: [{ path: string; query?: Record<string, string> }];
  refresh: [];
  'cluster-context-menu': [event: MouseEvent, clusterName: string];
  'topics-folder-context-menu': [event: MouseEvent, clusterName: string];
  'topic-context-menu': [event: MouseEvent, topicName: string, clusterName: string];
  'partition-context-menu': [event: MouseEvent, topicName: string, clusterName: string, partitionId: number];
  toast: [type: 'success' | 'error' | 'warning' | 'info', message: string];
}>();

const clusterStore = useClusterStore();
const clusters = computed(() => clusterStore.clusters);

const expandedClusters = ref(new Set<string>());
const expandedTopics = ref(new Set<string>());
const refreshingClusters = ref(new Set<string>()); // 正在刷新的集群
const refreshingConsumerGroups = ref(new Set<string>()); // 正在刷新 Consumer Groups 的集群
const loadingClusters = ref(new Set<string>()); // 正在加载 topics 的集群
const expandedTopicsFolders = ref(new Set<string>()); // Topics 文件夹展开状态
const expandedConsumerGroupsFolders = ref(new Set<string>()); // Consumer Groups 文件夹展开状态

// 选中状态
const selectedTopic = ref<{ name: string; cluster: string } | null>(null);
const selectedPartition = ref<{ topic: string; partition: number; cluster: string } | null>(null);
const selectedConsumerGroup = ref<{ groupId: string; cluster: string } | null>(null);

const topicCounts = reactive<Record<string, number>>({});
const consumerGroupCounts = reactive<Record<string, number>>({});
const clusterTopics = reactive<Record<string, Topic[]>>({});
const clusterConsumerGroups = reactive<Record<string, ConsumerGroup[]>>({});

// 错误对话框状态
const errorDialogRef = ref<HTMLDialogElement>();
const errorDialogClusterName = ref<string>('');
const errorDialogMessage = ref<string>('');
const pendingClusterName = ref<string | null>(null); // 等待展开的集群

function getClusterHealth(clusterId: string) {
  return clusterStore.getClusterHealth(clusterId);
}

function toggleCluster(clusterName: string) {
  if (expandedClusters.value.has(clusterName)) {
    expandedClusters.value.delete(clusterName);
    expandedClusters.value = new Set(expandedClusters.value);
  } else {
    // 展开时进行心跳检查
    checkClusterHealth(clusterName);
  }
}

// 检查集群健康状态
async function checkClusterHealth(clusterName: string) {
  try {
    const health = await apiClient.healthCheckCluster(clusterName);
    if (health.healthy) {
      // 连接成功，展开集群并获取 topic 数量
      clusterStore.clusterHealth[clusterName] = {
        clusterId: clusterName,
        healthy: true,
        lastChecked: Date.now(),
      };
      expandedClusters.value = new Set(expandedClusters.value.add(clusterName));
      // 获取 topic 数量（从数据库）
      const count = await apiClient.getTopicCount(clusterName);
      topicCounts[clusterName] = count;
    } else {
      // 连接失败，更新状态但不立即显示错误对话框
      clusterStore.clusterHealth[clusterName] = {
        clusterId: clusterName,
        healthy: false,
        lastChecked: Date.now(),
        error: health.error_message || 'Cluster unavailable',
      };
      // 仍然展开集群，让用户可以看到缓存的数据
      expandedClusters.value = new Set(expandedClusters.value.add(clusterName));
      // 显示错误（只要有错误状态就显示）
      showConnectionError(clusterName, health.error_message || 'Cluster unavailable');
    }
  } catch (e) {
    const errorMsg = (e as { message?: string }).message || 'Unknown error';
    // 连接失败，更新状态
    clusterStore.clusterHealth[clusterName] = {
      clusterId: clusterName,
      healthy: false,
      lastChecked: Date.now(),
      error: errorMsg,
    };
    // 仍然展开集群，让用户可以看到缓存的数据
    expandedClusters.value = new Set(expandedClusters.value.add(clusterName));
    // 显示错误对话框
    showConnectionError(clusterName, errorMsg);
  }
}

// 显示连接错误对话框
function showConnectionError(clusterName: string, errorMsg: string) {
  // 对于临时性网络故障，不显示错误对话框，只更新集群状态 + 显示 Toast
  // 这样用户可以继续查看缓存的数据
  const isTransientError =
    errorMsg.includes('BrokerTransportFailure') ||
    errorMsg.includes('timed out') ||
    errorMsg.includes('Transport') ||
    errorMsg.includes('Metadata fetch failed');

  if (isTransientError) {
    // 临时性错误，不显示对话框，但显示 Toast 提示并更新 UI 状态
    console.warn(`[ClusterTreeNavigator] Transient connection error for cluster '${clusterName}': ${errorMsg}`);
    emit('toast', 'error', `Cluster "${clusterName}" connection failed: ${errorMsg}`);
    return;
  }

  // 其他错误（如配置错误、认证失败等）显示错误对话框
  errorDialogClusterName.value = clusterName;
  errorDialogMessage.value = errorMsg;
  pendingClusterName.value = clusterName;
  errorDialogRef.value?.showModal();
}

// 处理对话框关闭（取消）
function handleErrorDialogClose() {
  errorDialogRef.value?.close();
  pendingClusterName.value = null;
}

// 处理对话框重试
function handleErrorDialogRetry() {
  errorDialogRef.value?.close();
  if (pendingClusterName.value) {
    checkClusterHealth(pendingClusterName.value);
  }
}

function toggleTopic(topicName: string, clusterName: string) {
  const key = `${clusterName}:${topicName}`;
  if (expandedTopics.value.has(key)) {
    expandedTopics.value.delete(key);
    expandedTopics.value = new Set(expandedTopics.value);
  } else {
    expandedTopics.value = new Set(expandedTopics.value.add(key));
  }
}

function toggleTopicsFolder(clusterName: string) {
  if (expandedTopicsFolders.value.has(clusterName)) {
    expandedTopicsFolders.value.delete(clusterName);
    expandedTopicsFolders.value = new Set(expandedTopicsFolders.value);
  } else {
    expandedTopicsFolders.value = new Set(expandedTopicsFolders.value.add(clusterName));
  }
}

function handleTopicsFolderToggle(clusterName: string) {
  toggleTopicsFolder(clusterName);
  loadClusterTopics(clusterName);
}

async function handleTopicsFolderClickAndExpand(clusterName: string) {
  // 点击 Topics 文件夹时，先展开文件夹并加载 topics
  if (!expandedTopicsFolders.value.has(clusterName)) {
    expandedTopicsFolders.value = new Set(expandedTopicsFolders.value.add(clusterName));
    await loadClusterTopics(clusterName);
  }
  // 然后导航到 Topics 页面
  emit('navigate', { path: '/topics', query: { cluster: clusterName } });
}

function toggleConsumerGroupsFolder(clusterName: string) {
  if (expandedConsumerGroupsFolders.value.has(clusterName)) {
    expandedConsumerGroupsFolders.value.delete(clusterName);
    expandedConsumerGroupsFolders.value = new Set(expandedConsumerGroupsFolders.value);
  } else {
    expandedConsumerGroupsFolders.value = new Set(expandedConsumerGroupsFolders.value.add(clusterName));
  }
}

function handleConsumerGroupsFolderToggle(clusterName: string) {
  toggleConsumerGroupsFolder(clusterName);
  loadClusterConsumerGroups(clusterName);
}

function handleConsumerGroupsFolderClick(clusterName: string) {
  // 点击 Consumer Groups 文件夹时，导航到 Consumer Groups 页面
  emit('navigate', { path: '/consumer-groups', query: { cluster: clusterName } });
}

// 处理添加集群
function handleAddCluster() {
  emit('navigate', { path: '/clusters', query: { action: 'create' } });
}

function expandAll() {
  expandedClusters.value = new Set(clusters.value.map(c => c.name));
  expandedTopicsFolders.value = new Set(clusters.value.map(c => c.name));
  expandedConsumerGroupsFolders.value = new Set(clusters.value.map(c => c.name));
}

function collapseAll() {
  expandedClusters.value = new Set();
  expandedTopics.value = new Set();
  expandedTopicsFolders.value = new Set();
  expandedConsumerGroupsFolders.value = new Set();
}

async function loadClusterTopics(clusterName: string) {
  // 如果正在加载中，直接返回 false
  if (loadingClusters.value.has(clusterName)) return false;

  loadingClusters.value.add(clusterName);
  let retryCount = 0;
  const maxRetries = 3;

  while (retryCount < maxRetries) {
    try {
      // 先从数据库获取已保存的 topics
      const topics = await apiClient.getSavedTopics(clusterName);

      // 如果数据库中没有 topics，从 Kafka 集群实时获取并保存到数据库
      if (!topics || topics.length === 0) {
        // 调用刷新接口，将集群 topics 同步到数据库
        await apiClient.refreshTopics(clusterName);
        // 刷新后重新获取完整的 topics 列表
        const refreshedTopics = await apiClient.getSavedTopics(clusterName);
        // 获取每个 topic 的分区信息
        clusterTopics[clusterName] = await Promise.all(
          (refreshedTopics || []).map(async (name: string) => {
            try {
              const detail = await apiClient.getTopicDetail(clusterName, name);
              return {
                name,
                partitions: detail.partitions.map(p => ({ id: p.id }))
              };
            } catch (e) {
              console.warn(`Failed to get partitions for topic ${name}:`, e);
              return { name, partitions: [] };
            }
          })
        );
        topicCounts[clusterName] = refreshedTopics?.length || 0;
      } else {
        // 获取每个 topic 的分区信息
        clusterTopics[clusterName] = await Promise.all(
          topics.map(async (name: string) => {
            try {
              const detail = await apiClient.getTopicDetail(clusterName, name);
              return {
                name,
                partitions: detail.partitions.map(p => ({ id: p.id }))
              };
            } catch (e) {
              console.warn(`Failed to get partitions for topic ${name}:`, e);
              return { name, partitions: [] };
            }
          })
        );
        topicCounts[clusterName] = topics.length;
      }
      // 成功加载，跳出循环
      break;
    } catch (error: any) {
      retryCount++;
      console.warn(`[ClusterTreeNavigator] Failed to load topics for ${clusterName} (attempt ${retryCount}/${maxRetries}):`, error?.message || error);

      // 如果是集群未找到错误，可能是后端还没准备好，等待后重试
      if (error?.message?.includes('not found') && retryCount < maxRetries) {
        await new Promise(resolve => setTimeout(resolve, retryCount * 500));
      } else {
        // 其他错误或已达到最大重试次数，显示错误但继续
        console.error('[ClusterTreeNavigator] Failed to load topics:', error);
        break;
      }
    }
  }

  loadingClusters.value.delete(clusterName);
}

async function loadClusterConsumerGroups(clusterName: string) {
  // 如果已经有 consumer groups 数据，直接返回
  if (clusterConsumerGroups[clusterName]) return;

  try {
    // Consumer Groups 始终从 Kafka 集群实时获取
    const groups = await apiClient.getConsumerGroups(clusterName);
    clusterConsumerGroups[clusterName] = groups.map((g: { name: string; state: string }) => ({
      groupId: g.name,
      state: g.state
    }));
    consumerGroupCounts[clusterName] = groups.length;
  } catch (error) {
    console.error('Failed to load consumer groups:', error);
  }
}

async function refreshClusterConsumerGroups(clusterName: string) {
  if (refreshingConsumerGroups.value.has(clusterName)) return;

  refreshingConsumerGroups.value = new Set(refreshingConsumerGroups.value.add(clusterName));
  try {
    // 调用后端 API 刷新 consumer groups
    const groups = await apiClient.getConsumerGroups(clusterName);
    clusterConsumerGroups[clusterName] = groups.map((g: { name: string; state: string }) => ({
      groupId: g.name,
      state: g.state
    }));
    consumerGroupCounts[clusterName] = groups.length;

    // 自动展开 Consumer Groups 文件夹（重新赋值触发响应式更新）
    expandedConsumerGroupsFolders.value = new Set(expandedConsumerGroupsFolders.value.add(clusterName));
  } catch (error) {
    console.error('Failed to refresh consumer groups:', error);
  } finally {
    refreshingConsumerGroups.value.delete(clusterName);
    refreshingConsumerGroups.value = new Set(refreshingConsumerGroups.value);
  }
}

async function refreshClusterTopics(clusterName: string) {
  if (refreshingClusters.value.has(clusterName)) return;

  refreshingClusters.value = new Set(refreshingClusters.value.add(clusterName));
  try {
    // 调用后端 API 刷新 topics（从 Kafka 集群同步到数据库）
    const refreshResult = await apiClient.refreshTopics(clusterName);

    // 刷新后重新获取完整的 topics 列表
    const savedTopics = await apiClient.getSavedTopics(clusterName);
    // 获取每个 topic 的分区信息
    clusterTopics[clusterName] = await Promise.all(
      (savedTopics || []).map(async (name: string) => {
        try {
          const detail = await apiClient.getTopicDetail(clusterName, name);
          return {
            name,
            partitions: detail.partitions.map(p => ({ id: p.id }))
          };
        } catch (e) {
          console.warn(`Failed to get partitions for topic ${name}:`, e);
          return { name, partitions: [] };
        }
      })
    );
    topicCounts[clusterName] = savedTopics?.length || refreshResult.total || 0;

    // 自动展开 Topics 文件夹（重新赋值触发响应式更新）
    expandedTopicsFolders.value = new Set(expandedTopicsFolders.value.add(clusterName));
  } catch (error) {
    console.error('Failed to refresh topics:', error);
  } finally {
    refreshingClusters.value.delete(clusterName);
    refreshingClusters.value = new Set(refreshingClusters.value);
  }
}

// 虚拟滚动相关
const VISIBLE_ITEMS = 100; // 最大渲染 100 个 topic，超过的需要搜索
const topicSearchQuery = reactive<Record<string, string>>({});

// 存储每个 topic 元素的 ref
const topicElementRefs = reactive<Record<string, HTMLDivElement | null>>({});

function setTopicElementRef(key: string, el: HTMLDivElement | null) {
  topicElementRefs[key] = el;
}

function getClusterTopics(clusterName: string): Topic[] {
  const topics = clusterTopics[clusterName] || [];
  const query = topicSearchQuery[clusterName];

  if (query && query.trim()) {
    // 搜索模式：过滤匹配的 topic
    const lowerQuery = query.toLowerCase();
    return topics.filter(t => t.name.toLowerCase().includes(lowerQuery));
  }

  // 默认只返回前 100 个
  return topics.slice(0, VISIBLE_ITEMS);
}

function setTopicSearch(clusterName: string, query: string) {
  topicSearchQuery[clusterName] = query;
}

function getTotalTopics(clusterName: string): number {
  return (clusterTopics[clusterName] || []).length;
}

function getClusterConsumerGroups(clusterName: string): ConsumerGroup[] {
  return clusterConsumerGroups[clusterName] || [];
}

async function selectTopic(topic: Topic, clusterName: string) {
  selectedTopic.value = { name: topic.name, cluster: clusterName };
  selectedPartition.value = null;
  selectedConsumerGroup.value = null;

  // 确保集群已连接
  await connectClusterIfNeeded(clusterName);

  emit('navigate', { path: '/messages', query: { cluster: clusterName, topic: topic.name } });
}

async function selectPartition(partitionId: number, topic: Topic, clusterName: string) {
  selectedTopic.value = { name: topic.name, cluster: clusterName };
  selectedPartition.value = { topic: topic.name, partition: partitionId, cluster: clusterName };
  selectedConsumerGroup.value = null;

  // 确保集群已连接
  await connectClusterIfNeeded(clusterName);

  emit('navigate', { path: '/messages', query: { cluster: clusterName, topic: topic.name, partition: String(partitionId) } });
}

async function connectClusterIfNeeded(clusterName: string) {
  const health = getClusterHealth(clusterName);
  if (!health || health.healthy !== true) {
    try {
      await apiClient.reconnectCluster(clusterName);
      // 静默刷新健康状态
      await clusterStore.refreshAllHealth();
    } catch (e) {
      console.error(`Failed to connect cluster ${clusterName}:`, e);
    }
  }
}

function selectConsumerGroup(group: ConsumerGroup, clusterName: string) {
  selectedTopic.value = null;
  selectedPartition.value = null;
  selectedConsumerGroup.value = { groupId: group.groupId, cluster: clusterName };
  emit('navigate', { path: '/consumer-groups', query: { cluster: clusterName, group: group.groupId } });
}

// 从外部调用：展开集群、Topics 文件夹并选中特定 Topic
async function highlightAndSelectTopic(topicName: string, clusterName: string) {
  // 先检查集群健康状态
  const health = getClusterHealth(clusterName);

  if (!health || health.healthy !== true) {
    // 如果集群状态不是绿色，先进行静默健康检查（不显示错误对话框）
    try {
      const checkResult = await apiClient.healthCheckCluster(clusterName);
      if (checkResult.healthy) {
        clusterStore.clusterHealth[clusterName] = {
          clusterId: clusterName,
          healthy: true,
          lastChecked: Date.now(),
        };
        // 获取 topic 数量（从数据库）
        const count = await apiClient.getTopicCount(clusterName);
        topicCounts[clusterName] = count;
      } else {
        clusterStore.clusterHealth[clusterName] = {
          clusterId: clusterName,
          healthy: false,
          lastChecked: Date.now(),
          error: checkResult.error_message || 'Cluster unavailable',
        };
      }
    } catch (e) {
      // 静默失败，更新状态但不显示错误对话框
      clusterStore.clusterHealth[clusterName] = {
        clusterId: clusterName,
        healthy: false,
        lastChecked: Date.now(),
        error: (e as { message: string }).message,
      };
    }
  } else {
    // 集群已经是健康状态，直接获取 topic 数量
    if (!topicCounts[clusterName]) {
      const count = await apiClient.getTopicCount(clusterName);
      topicCounts[clusterName] = count;
    }
  }

  // 无论集群是否可用，都展开集群和 Topics 文件夹
  // 展开集群
  expandedClusters.value = new Set(expandedClusters.value.add(clusterName));
  // 展开 Topics 文件夹
  expandedTopicsFolders.value = new Set(expandedTopicsFolders.value.add(clusterName));
  // 加载 Topics（从数据库加载缓存的 topic 列表）
  // 如果正在加载中，等待加载完成
  const isLoading = await loadClusterTopics(clusterName);
  if (isLoading) {
    // 等待加载完成（最多等待 5 秒）
    let waitCount = 0;
    while (loadingClusters.value.has(clusterName) && waitCount < 50) {
      await new Promise(resolve => setTimeout(resolve, 100));
      waitCount++;
    }
  }
  // 设置选中状态
  selectedTopic.value = { name: topicName, cluster: clusterName };
  selectedPartition.value = null;
  selectedConsumerGroup.value = null;

  // 滚动到选中的 topic（等待 DOM 更新后执行）
  await new Promise(resolve => setTimeout(resolve, 100));
  const topicKey = `${clusterName}:${topicName}`;
  const topicElement = topicElementRefs[topicKey];
  if (topicElement) {
    topicElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }

  // 导航到消息页面（如果集群不可用，消息页面会显示错误提示）
  emit('navigate', { path: '/messages', query: { cluster: clusterName, topic: topicName } });
}

// 暴露给父组件的方法
defineExpose({
  highlightAndSelectTopic
});

function showClusterMenu(event: MouseEvent, clusterName: string) {
  emit('cluster-context-menu', event, clusterName);
}

function showTopicsFolderMenu(event: MouseEvent, clusterName: string) {
  emit('topics-folder-context-menu', event, clusterName);
}

function showContextMenu(event: MouseEvent, topicName: string, clusterName: string) {
  emit('topic-context-menu', event, topicName, clusterName);
}

function showPartitionMenu(event: MouseEvent, topicName: string, clusterName: string, partitionId: number) {
  emit('partition-context-menu', event, topicName, clusterName, partitionId);
}

// 处理 Topics 刷新事件（从 ModernLayout 的 Refresh Topics 按钮触发）
async function handleTopicsRefreshed(event: Event) {
  const customEvent = event as CustomEvent<{ cluster: string }>;
  const { cluster } = customEvent.detail;
  if (cluster) {
    // 等待一小段时间让后端完成数据库更新
    await new Promise(resolve => setTimeout(resolve, 300));

    // 直接获取最新的 topics 并更新
    try {
      const topics = await apiClient.getSavedTopics(cluster);
      clusterTopics[cluster] = await Promise.all(
        (topics || []).map(async (name: string) => {
          try {
            const detail = await apiClient.getTopicDetail(cluster, name);
            return {
              name,
              partitions: detail.partitions.map(p => ({ id: p.id }))
            };
          } catch (e) {
            console.warn(`Failed to get partitions for topic ${name}:`, e);
            return { name, partitions: [] };
          }
        })
      );
      topicCounts[cluster] = topics?.length || 0;
    } catch (error) {
      console.error('[ClusterTreeNavigator] Failed to refresh topics:', error);
    }

    // 如果 Topics 文件夹还没有展开，先展开它
    expandedTopicsFolders.value = new Set(expandedTopicsFolders.value.add(cluster));
  }
}

onMounted(() => {
  // 监听刷新事件（从 ModernLayout 的 Refresh Topics 按钮触发）
  window.addEventListener('cluster-topics-refreshed', handleTopicsRefreshed);
});

onUnmounted(() => {
  // 清理事件监听
  window.removeEventListener('cluster-topics-refreshed', handleTopicsRefreshed);
});
</script>
