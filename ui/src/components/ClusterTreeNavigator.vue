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
    <div class="flex-1 overflow-y-auto p-2 relative">
      <div v-for="cluster in clusters" :key="cluster.name" class="mb-1">
        <!-- Cluster Node -->
        <div
          class="relative"
        >
          <div
            class="flex items-center p-2 rounded-xl cursor-pointer transition-all duration-300 hover:bg-primary/5 hover:shadow-md sticky top-0 z-30 bg-base-100/95 backdrop-blur-sm group overflow-visible"
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
            <!-- Three dots menu button -->
            <button
              :ref="(el) => { if (el) { const buttons = clusterActionButtons.value; clusterActionButtons.value = { ...buttons, [cluster.name]: el } } }"
              class="btn btn-ghost btn-xs p-0 w-6 h-6 min-h-0 ml-auto opacity-0 group-hover:opacity-100 transition-opacity"
              @click.stop="toggleClusterActionsDropdown(cluster.name, $event)"
              title="Cluster Actions"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 6.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 12.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 18.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5Z" />
              </svg>
            </button>
          </div>
        </div>

        <!-- Cluster Children -->
        <div v-show="expandedClusters.has(cluster.name)" class="flex flex-col">
          <!-- Topics Folder -->
          <div class="mb-0.5 sticky top-0 z-20 bg-base-100/95 backdrop-blur-sm flex-shrink-0">
            <div
              class="flex items-center p-1.5 rounded-lg cursor-pointer transition-all duration-300 hover:bg-secondary/5 relative"
              :class="{ 'bg-secondary/10': expandedTopicsFolders.has(cluster.name) }"
              @click.stop="handleTopicsFolderClickAndExpand(cluster.name)"
              @contextmenu.prevent="showTopicsFolderMenu($event, cluster.name)"
            >
              <div class="flex items-center gap-1 flex-1 min-w-0">
                <button class="btn btn-ghost btn-xs p-0 w-4 h-4 min-h-0" @click.stop="handleTopicsFolderToggle(cluster.name)" tabindex="-1">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="2"
                    stroke="currentColor"
                    class="w-2.5 h-2.5 transition-transform duration-200"
                    :class="{ 'rotate-90': expandedTopicsFolders.has(cluster.name) }"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                  </svg>
                </button>
                <span class="text-xs truncate">{{ topicCounts[cluster.name] || 0 }}</span>
                <span class="text-xs truncate">Topics</span>
              </div>
              <!-- Topics Refresh Button -->
              <button
                class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0 ml-0.5 opacity-0 hover:opacity-100 transition-opacity"
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
                  class="w-3 h-3"
                  :class="{ 'animate-spin': refreshingClusters.has(cluster.name) }"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                </svg>
              </button>
            </div>

            <!-- Topics Scrollable Container (includes search box + topics list) -->
            <div v-show="expandedTopicsFolders.has(cluster.name)" class="pl-3 overflow-y-auto max-h-[500px]">
              <!-- Topic Search Box - sticky under Topics folder -->
              <div v-if="getTotalTopics(cluster.name) > 0" class="mb-1 sticky top-0 bg-base-100 z-10 py-1">
                <div class="join w-full">
                  <input
                    v-model="topicSearchQuery[cluster.name]"
                    type="text"
                    :placeholder="`Search ${getTotalTopics(cluster.name)}...`"
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
                <p v-if="!topicSearchQuery[cluster.name]" class="text-[10px] text-base-content/50 mt-0.5">
                  {{ getTotalTopics(cluster.name) }} topics
                </p>
                <p v-else class="text-[10px] text-primary mt-0.5">
                  {{ getClusterTopics(cluster.name).length }} matching
                </p>
              </div>

              <div
                v-for="topic in getClusterTopics(cluster.name)"
                :key="topic.name"
                class="mb-0.5"
                ref="el => setTopicElementRef(`${cluster.name}:${topic.name}`, el)"
              >
                <!-- Topic Node -->
                <div
                  class="flex items-center p-1.5 rounded-lg cursor-pointer transition-all duration-300 hover:bg-accent/5"
                  :class="{ 'bg-accent/10 text-accent': selectedTopic?.name === topic.name && selectedTopic?.cluster === cluster.name }"
                  @contextmenu.prevent="showContextMenu($event, topic.name, cluster.name)"
                  @click="selectTopic(topic, cluster.name)"
                >
                  <div class="flex items-center gap-1 flex-1 min-w-0">
                    <button
                      class="btn btn-ghost btn-xs p-0 w-4 h-4 min-h-0"
                      @click.stop="toggleTopic(topic.name, cluster.name)"
                      tabindex="-1"
                    >
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="2"
                        stroke="currentColor"
                        class="w-2.5 h-2.5 transition-transform duration-200"
                        :class="{ 'rotate-90': expandedTopics.has(`${cluster.name}:${topic.name}`) }"
                      >
                        <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                      </svg>
                    </button>
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-secondary flex-shrink-0">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                    </svg>
                    <span class="text-xs truncate flex-1 min-w-0" :title="topic.name">{{ topic.name }}</span>
                  </div>
                </div>

                <!-- Partitions List -->
                <div v-show="expandedTopics.has(`${cluster.name}:${topic.name}`)" class="pl-3">
                  <!-- Loading State -->
                  <div v-if="loadingTopicPartitions.has(`${cluster.name}:${topic.name}`)" class="flex items-center py-2">
                    <span class="loading loading-spinner loading-xs text-primary"></span>
                    <span class="text-[10px] text-base-content/50 ml-2">Loading partitions...</span>
                  </div>
                  <!-- Partitions -->
                  <div
                    v-else
                    v-for="partition in topic.partitions || []"
                    :key="partition.id"
                    class="flex items-center p-1.5 rounded-lg cursor-pointer transition-all duration-300 hover:bg-base-200/50"
                    :class="{ 'bg-accent/10 text-accent': selectedPartition?.topic === topic.name && selectedPartition?.partition === partition.id && selectedPartition?.cluster === cluster.name }"
                    @click="selectPartition(partition.id, topic, cluster.name)"
                    @contextmenu.prevent="showPartitionMenu($event, topic.name, cluster.name, partition.id)"
                  >
                    <div class="flex items-center gap-1 flex-1 min-w-0">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-base-content/40 flex-shrink-0">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
                      </svg>
                      <span class="text-[10px] truncate text-base-content/70">#{{ partition.id }}</span>
                    </div>
                  </div>
                  <!-- No Partitions -->
                  <div v-if="!loadingTopicPartitions.has(`${cluster.name}:${topic.name}`) && (!topic.partitions || topic.partitions.length === 0)" class="text-[10px] text-base-content/50 py-2 pl-2">
                    No partitions available
                  </div>
                </div>
              </div>

              <!-- Load More Button -->
              <div v-if="!topicSearchQuery[cluster.name] && hasMoreTopics(cluster.name)" class="py-1">
                <button
                  class="btn btn-xs btn-ghost w-full text-primary"
                  @click.stop="loadMoreTopics(cluster.name)"
                >
                  Load more ({{ getTotalTopics(cluster.name) - getClusterTopics(cluster.name).length }} remaining)
                </button>
              </div>
            </div>
          </div>

          <!-- Consumer Groups Folder -->
          <div class="mb-0.5 flex-shrink-0">
            <div
              class="flex items-center p-1.5 rounded-lg cursor-pointer transition-all duration-300 hover:bg-success/5 hover:shadow-md relative"
              :class="{ 'bg-success/10 shadow-inner': expandedConsumerGroupsFolders.has(cluster.name) }"
              @click.stop="handleConsumerGroupsFolderClick(cluster.name)"
            >
              <div class="flex items-center gap-1 flex-1 min-w-0 pr-6">
                <button class="btn btn-ghost btn-xs p-0 w-4 h-4 min-h-0" @click.stop="handleConsumerGroupsFolderToggle(cluster.name)" tabindex="-1">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="2"
                    stroke="currentColor"
                    class="w-2.5 h-2.5 transition-transform duration-200"
                    :class="{ 'rotate-90': expandedConsumerGroupsFolders.has(cluster.name) }"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                  </svg>
                </button>
                <span class="text-xs truncate">{{ consumerGroupCounts[cluster.name] || 0 }}</span>
                <span class="text-xs truncate">Consumer Groups</span>
              </div>
              <!-- Consumer Groups Refresh Button -->
              <button
                class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0 ml-1 absolute right-1 opacity-0 hover:opacity-100 transition-opacity"
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
                  class="w-3 h-3"
                  :class="{ 'animate-spin': refreshingConsumerGroups.has(cluster.name) }"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                </svg>
              </button>
            </div>

            <!-- Consumer Groups List -->
            <div v-show="expandedConsumerGroupsFolders.has(cluster.name)" class="pl-3 overflow-y-auto max-h-[120px]">
              <div
                v-for="group in getClusterConsumerGroups(cluster.name)"
                :key="group.groupId"
                class="mb-0.5"
              >
                <!-- Consumer Group Node -->
                <div
                  class="flex items-center p-1.5 rounded-lg cursor-pointer transition-all duration-300 hover:bg-success/5 hover:shadow-md"
                  :class="{ 'bg-success/10 shadow-inner text-success': selectedConsumerGroup?.groupId === group.groupId && selectedConsumerGroup?.cluster === cluster.name }"
                  @click="selectConsumerGroup(group, cluster.name)"
                >
                  <div class="flex items-center gap-1.5 flex-1 min-w-0">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-success flex-shrink-0">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 003.741-.479 3 3 0 00-4.682-2.72m.94 3.198.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0112 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 016 18.719m12 0a5.971 5.971 0 00-.941-3.197m0 0A5.995 5.995 0 0012 12.75a5.995 5.995 0 00-5.058 2.772m0 0a3 3 0 00-4.681 2.72 8.986 8.986 0 003.74.477m.94-3.197a5.971 5.971 0 00-.941-3.197M15 6.75a3 3 0 11-6 0 3 3 0 016 0zm6 3a2.25 2.25 0 11-4.5 0 2.25 2.25 0 014.5 0zm-13.5 0a2.625 2.625 0 11-4.5 0 2.625 2.625 0 014.5 0z" />
                    </svg>
                    <span class="text-xs truncate">{{ group.groupId }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Cluster Actions Dropdown - rendered outside scroll container to avoid z-index issues -->
    <div
      v-for="cluster in clusters"
      :key="`dropdown-${cluster.name}`"
      v-show="openActionsDropdown === cluster.name"
      class="fixed z-[9999] min-w-[180px] rounded-lg bg-base-100 border border-base-200 shadow-xl p-1 pointer-events-none"
      :style="{ top: dropdownPositions[cluster.name]?.top + 'px', left: dropdownPositions[cluster.name]?.left + 'px' }"
    >
      <div class="pointer-events-auto">
        <!-- Menu Title -->
        <div class="px-3 py-2 text-sm font-semibold text-base-content border-b border-base-200 mb-1">
          <span>{{ cluster.name }}</span>
        </div>
        <!-- Connection Actions -->
        <ul class="menu menu-sm bg-base-100 w-full">
          <li>
            <a @click="handleClusterAction('testConnection', cluster.name)" :class="{ 'opacity-50': testing.has(cluster.name) }">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z" />
              </svg>
              Test Connection
            </a>
          </li>
          <li>
            <a @click="handleClusterAction('refreshConnection', cluster.name)">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4" :class="{ 'animate-spin': refreshingClusters.has(cluster.name) }">
                <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
              </svg>
              Refresh Status
            </a>
          </li>
          <li>
            <a @click="handleClusterAction('disconnect', cluster.name)" :class="{ 'opacity-50': disconnecting.has(cluster.name) }">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M18.364 18.364A9 9 0 0 0 5.636 5.636m12.728 12.728A9 9 0 0 1 5.636 5.636m12.728 12.728L5.636 5.636" />
              </svg>
              Disconnect
            </a>
          </li>
          <li>
            <a @click="handleClusterAction('reconnect', cluster.name)" :class="{ 'opacity-50': reconnecting.has(cluster.name) }">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
              </svg>
              Reconnect
            </a>
          </li>
        </ul>
        <hr class="h-px bg-base-200 my-1" />
        <!-- Management Actions -->
        <ul class="menu menu-sm bg-base-100 w-full">
          <li>
            <a @click="handleClusterAction('viewBrokers', cluster.name)">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
              </svg>
              View Brokers
            </a>
          </li>
          <li>
            <a @click="handleClusterAction('viewTopics', cluster.name)">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
              </svg>
              View Topics
            </a>
          </li>
          <li>
            <a @click="handleClusterAction('viewConsumers', cluster.name)">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.72a9.094 9.094 0 0 0 3.741-.479 3 3 0 0 0-4.682-2.72m.94 3.198.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0 1 12 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 0 1 6 18.719m12 0a5.971 5.971 0 0 0-.941-3.197m0 0A5.995 5.995 0 0 0 12 12.75a5.995 5.995 0 0 0-5.058 2.772m0 0a3 3 0 0 0-4.681 2.72 8.986 8.986 0 0 0 3.74.477m.94-3.197a5.971 5.971 0 0 0-.941-3.197M15 6.75a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm6 3a2.25 2.25 0 1 1-4.5 0 2.25 2.25 0 0 1 4.5 0Zm-13.5 0a2.25 2.25 0 1 1-4.5 0 2.25 2.25 0 0 1 4.5 0Z" />
              </svg>
              View Consumer Groups
            </a>
          </li>
        </ul>
        <hr class="h-px bg-base-200 my-1" />
        <!-- Edit/Delete Actions -->
        <ul class="menu menu-sm bg-base-100 w-full">
          <li>
            <a @click="handleClusterAction('edit', cluster.name)">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" />
              </svg>
              Edit Cluster
            </a>
          </li>
          <li>
            <a class="text-error hover:bg-error/10" @click="handleClusterAction('delete', cluster.name)">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
              </svg>
              Delete Cluster
            </a>
          </li>
        </ul>
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
import { ref, reactive, computed, onMounted, onUnmounted, nextTick, shallowRef } from 'vue';
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

// Cluster actions dropdown state
const openActionsDropdown = ref<string | null>(null);
const testing = ref(new Set<string>());
const disconnecting = ref(new Set<string>());
const reconnecting = ref(new Set<string>());
const clusterActionButtons = shallowRef<Record<string, any>>({});
const dropdownPositions = ref<Record<string, { top: number; left: number }>>({});

function toggleClusterActionsDropdown(clusterName: string, event: MouseEvent) {
  if (openActionsDropdown.value === clusterName) {
    openActionsDropdown.value = null;
  } else {
    openActionsDropdown.value = clusterName;
    // Calculate dropdown position - 菜单出现在按钮下方，右对齐按钮，并检测屏幕边界
    nextTick(() => {
      const rect = (event.target as HTMLElement).getBoundingClientRect();
      const menuHeight = 400; // 预估菜单高度
      const viewportHeight = window.innerHeight;

      // 检查菜单是否会超出屏幕底部
      const fitsBelow = rect.bottom + menuHeight <= viewportHeight;

      dropdownPositions.value[clusterName] = {
        top: fitsBelow ? rect.bottom - 8 : rect.top - menuHeight, // 如果下方空间不够，在按钮上方显示
        left: rect.right - 180 // 菜单宽度 180px，右对齐按钮右侧
      };
    });
  }
}

function closeActionsDropdown() {
  openActionsDropdown.value = null;
  // Clear position data
  dropdownPositions.value = {};
}

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

// 存储正在加载分区的 topic
const loadingTopicPartitions = reactive<Set<string>>(new Set());

function toggleTopic(topicName: string, clusterName: string) {
  const key = `${clusterName}:${topicName}`;
  if (expandedTopics.value.has(key)) {
    expandedTopics.value.delete(key);
    expandedTopics.value = new Set(expandedTopics.value);
  } else {
    // 展开时懒加载分区信息
    loadTopicPartitions(clusterName, topicName);
    expandedTopics.value = new Set(expandedTopics.value.add(key));
  }
}

async function loadTopicPartitions(clusterName: string, topicName: string) {
  const key = `${clusterName}:${topicName}`;

  // 如果正在加载，直接返回
  if (loadingTopicPartitions.has(key)) return;

  // 检查是否已经加载过分区
  const topic = clusterTopics[clusterName]?.find(t => t.name === topicName);
  if (topic && topic.partitions && topic.partitions.length > 0) return;

  loadingTopicPartitions.add(key);

  try {
    const detail = await apiClient.getTopicDetail(clusterName, topicName);
    const topicData = clusterTopics[clusterName]?.find(t => t.name === topicName);
    if (topicData) {
      topicData.partitions = detail.partitions.map(p => ({ id: p.id }));
    }
  } catch (e) {
    console.warn(`Failed to get partitions for topic ${topicName}:`, e);
  } finally {
    loadingTopicPartitions.delete(key);
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

// Handle cluster actions from three-dots menu
async function handleClusterAction(action: string, clusterName: string) {
  closeActionsDropdown();

  switch (action) {
    case 'testConnection':
      await testConnection(clusterName);
      break;
    case 'refreshConnection':
      await refreshConnectionStatus(clusterName);
      break;
    case 'disconnect':
      await disconnectCluster(clusterName);
      break;
    case 'reconnect':
      await reconnectCluster(clusterName);
      break;
    case 'viewBrokers':
      emit('navigate', { path: '/dashboard', query: { cluster: clusterName } });
      break;
    case 'viewTopics':
      emit('navigate', { path: '/topics', query: { cluster: clusterName } });
      break;
    case 'viewConsumers':
      emit('navigate', { path: '/consumer-groups', query: { cluster: clusterName } });
      break;
    case 'editCluster':
      // 导航到 clusters 页面并触发编辑
      window.dispatchEvent(new CustomEvent('edit-cluster-from-menu', { detail: { clusterName } }));
      break;
    case 'deleteCluster':
      if (confirm(`Are you sure you want to delete cluster "${clusterName}"?`)) {
        await deleteCluster(clusterName);
      }
      break;
  }
}

async function testConnection(clusterName: string) {
  const cluster = clusters.value.find(c => c.name === clusterName);
  if (!cluster) return;

  testing.value.add(clusterName);
  try {
    const result = await clusterStore.testCluster(cluster.id);
    if (result.success) {
      emit('toast', 'success', 'Connection test successful');
    } else {
      emit('toast', 'error', 'Connection test failed');
    }
  } catch (e) {
    emit('toast', 'error', `Connection test failed: ${(e as { message: string }).message}`);
  } finally {
    testing.value.delete(clusterName);
  }
}

async function refreshConnectionStatus(clusterName: string) {
  refreshingClusters.value.add(clusterName);
  try {
    const health = await apiClient.healthCheckCluster(clusterName);
    clusterStore.clusterHealth[clusterName] = {
      clusterId: clusterName,
      healthy: health.healthy,
      lastChecked: Date.now(),
      error: health.error_message,
    };
    if (health.healthy) {
      emit('toast', 'success', 'Cluster status refreshed');
    } else {
      emit('toast', 'error', `Cluster connection issue: ${health.error_message || 'Unknown error'}`);
    }
  } catch (e) {
    emit('toast', 'error', `Refresh failed: ${(e as { message: string }).message}`);
  } finally {
    refreshingClusters.value.delete(clusterName);
    refreshingClusters.value = new Set(refreshingClusters.value);
  }
}

async function disconnectCluster(clusterName: string) {
  if (confirm(`Are you sure you want to disconnect from cluster "${clusterName}"?`)) {
    disconnecting.value.add(clusterName);
    try {
      await apiClient.disconnectCluster(clusterName);
      // 更新集群健康状态为断开
      clusterStore.clusterHealth[clusterName] = {
        clusterId: clusterName,
        healthy: false,
        lastChecked: Date.now(),
        error: 'Disconnected',
      };
      emit('toast', 'success', 'Cluster disconnected');
    } catch (e) {
      emit('toast', 'error', `Disconnect failed: ${(e as { message: string }).message}`);
    } finally {
      disconnecting.value.delete(clusterName);
    }
  }
}

async function reconnectCluster(clusterName: string) {
  reconnecting.value.add(clusterName);
  try {
    await apiClient.reconnectCluster(clusterName);
    // 重新检查集群健康状态
    const health = await apiClient.healthCheckCluster(clusterName);
    clusterStore.clusterHealth[clusterName] = {
      clusterId: clusterName,
      healthy: health.healthy,
      lastChecked: Date.now(),
      error: health.error_message,
    };
    if (health.healthy) {
      emit('toast', 'success', 'Cluster reconnected');
    } else {
      emit('toast', 'error', `Reconnect failed: ${health.error_message || 'Unknown error'}`);
    }
  } catch (e) {
    emit('toast', 'error', `Reconnect failed: ${(e as { message: string }).message}`);
  } finally {
    reconnecting.value.delete(clusterName);
  }
}

async function deleteCluster(clusterName: string) {
  const cluster = clusters.value.find(c => c.name === clusterName);
  if (cluster) {
    await clusterStore.deleteCluster(cluster.id);
    emit('toast', 'success', 'Cluster deleted');
  }
}

// 处理添加集群
function handleAddCluster() {
  // 直接触发打开创建弹窗的事件，无论当前路由是什么
  window.dispatchEvent(new CustomEvent('open-create-cluster-modal'));
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
        // 只存储 topic 名称，分区信息在展开时懒加载
        clusterTopics[clusterName] = (refreshedTopics || []).map((name: string) => ({
          name,
          partitions: [] // 初始为空，展开时再加载
        }));
        topicCounts[clusterName] = refreshedTopics?.length || 0;
      } else {
        // 只存储 topic 名称，分区信息在展开时懒加载
        clusterTopics[clusterName] = topics.map((name: string) => ({
          name,
          partitions: [] // 初始为空，展开时再加载
        }));
        topicCounts[clusterName] = topics.length;
      }
      // 成功加载，跳出循环
      break;
    } catch (error: any) {
      retryCount++;
      console.warn(`[ClusterTreeNavigator] Failed to load topics for ${clusterName} (attempt ${retryCount}/${maxRetries}):`, error?.message || error);

      // 如果是集群未连接错误，可能是后端还没准备好，等待后重试
      if ((error?.message?.includes('not connected') || error?.message?.includes('not found')) && retryCount < maxRetries) {
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
    // 只存储 topic 名称，分区信息在展开时懒加载
    clusterTopics[clusterName] = (savedTopics || []).map((name: string) => ({
      name,
      partitions: [] // 初始为空，展开时再加载
    }));
    topicCounts[clusterName] = savedTopics?.length || refreshResult.total || 0;

    // 重置分页限制
    topicDisplayLimits[clusterName] = VISIBLE_ITEMS;

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
const VISIBLE_ITEMS = 1000; // 默认显示 1000 个 topic，超过的需要搜索
const topicSearchQuery = reactive<Record<string, string>>({});

// 存储每个 cluster 的 topic 显示数量（用于分页加载）
const topicDisplayLimits = reactive<Record<string, number>>({});

// 存储每个 topic 元素的 ref
const topicElementRefs = reactive<Record<string, HTMLDivElement | null>>({});

// @ts-ignore - used in template
function setTopicElementRef(key: string, el: HTMLDivElement | null) {
  topicElementRefs[key] = el;
}

function getClusterTopics(clusterName: string): Topic[] {
  const topics = clusterTopics[clusterName] || [];
  const query = topicSearchQuery[clusterName];

  if (query && query.trim()) {
    // 搜索模式：过滤匹配的 topic（搜索结果全部显示）
    const lowerQuery = query.toLowerCase();
    return topics.filter(t => t.name.toLowerCase().includes(lowerQuery));
  }

  // 默认返回前 N 个（支持加载更多）
  const limit = topicDisplayLimits[clusterName] || VISIBLE_ITEMS;
  return topics.slice(0, limit);
}

function loadMoreTopics(clusterName: string) {
  const currentLimit = topicDisplayLimits[clusterName] || VISIBLE_ITEMS;
  const totalTopics = (clusterTopics[clusterName] || []).length;

  if (currentLimit < totalTopics) {
    // 每次多加载 500 个
    topicDisplayLimits[clusterName] = Math.min(currentLimit + 500, totalTopics);
  }
}

function hasMoreTopics(clusterName: string): boolean {
  const currentLimit = topicDisplayLimits[clusterName] || VISIBLE_ITEMS;
  const totalTopics = (clusterTopics[clusterName] || []).length;
  return currentLimit < totalTopics;
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

  // 自动在搜索框中填入 topic 名称，方便用户定位（尤其是 topic 数量很多时）
  topicSearchQuery[clusterName] = topicName;

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
      // 只存储 topic 名称，分区信息在展开时懒加载
      clusterTopics[cluster] = (topics || []).map((name: string) => ({
        name,
        partitions: [] // 初始为空，展开时再加载
      }));
      topicCounts[cluster] = topics?.length || 0;

      // 重置分页限制
      topicDisplayLimits[cluster] = VISIBLE_ITEMS;
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
  // 监听点击外部关闭下拉菜单
  document.addEventListener('click', handleOutsideClick);
  // 监听编辑集群事件
  window.addEventListener('edit-cluster-from-menu', handleEditClusterFromMenu);
  // 监听滚动和窗口大小改变，关闭下拉菜单并更新位置
  window.addEventListener('scroll', handleScroll, true);
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  // 清理事件监听
  window.removeEventListener('cluster-topics-refreshed', handleTopicsRefreshed);
  document.removeEventListener('click', handleOutsideClick);
  window.removeEventListener('edit-cluster-from-menu', handleEditClusterFromMenu);
  window.removeEventListener('scroll', handleScroll, true);
  window.removeEventListener('resize', handleResize);
});

function handleOutsideClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  // 如果点击的不是三个点按钮或其子菜单，关闭下拉菜单
  if (!target.closest('[title="Cluster Actions"]') && openActionsDropdown.value) {
    closeActionsDropdown();
  }
}

function handleScroll() {
  // 滚动时关闭下拉菜单并清除位置
  if (openActionsDropdown.value) {
    closeActionsDropdown();
  }
}

function handleResize() {
  // 窗口大小改变时关闭下拉菜单
  if (openActionsDropdown.value) {
    closeActionsDropdown();
  }
}

function handleEditClusterFromMenu(event: Event) {
  const customEvent = event as CustomEvent<{ clusterName: string }>;
  const { clusterName } = customEvent.detail;
  // 触发打开创建集群编辑弹窗
  window.dispatchEvent(new CustomEvent('open-create-cluster-modal', {
    detail: { editClusterName: clusterName }
  }));
}
</script>

<style scoped>
/* Cluster actions dropdown positioning fix */
.cluster-actions-dropdown {
  position: fixed;
  z-index: 1000;
}
</style>
