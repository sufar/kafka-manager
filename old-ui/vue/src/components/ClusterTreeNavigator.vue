<template>
  <div class="flex-1 flex flex-col min-h-0">
    <!-- Header -->
    <div class="flex flex-col gap-2 p-2 mb-2 flex-shrink-0">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2" data-tour="tree-clusters-icon">
          <div class="w-8 h-8 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center glow-primary">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
              <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
            </svg>
          </div>
          <span class="text-xs font-bold text-base-content/60 uppercase tracking-wider">{{ showHistory ? (t.history?.title || 'Browsing History') : 'Clusters' }}</span>
        </div>
        <div class="flex gap-0.5">
          <button v-if="showHistory" class="btn btn-ghost btn-xs" @click="showHistory = false" :title="t.tree?.backToClusters || 'Back to clusters'">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 15L3 9m0 0l6-6M3 9h12a6 6 0 010 12h-3" />
            </svg>
          </button>
          <button v-if="!showHistory" class="btn btn-ghost btn-xs" @click="collapseAll" :title="t.tree?.collapseAll || 'Collapse all'" data-tour="tree-collapse-btn">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" />
            </svg>
          </button>
          <button v-if="!showHistory" class="btn btn-ghost btn-xs" @click="goToClusters" :title="t.tree?.clusters || 'Clusters'" data-tour="tree-clusters-btn">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary">
              <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
            </svg>
          </button>
          <button v-if="!showHistory" class="btn btn-ghost btn-xs" @click="goToFavorites" :title="t.tree?.topicFavorites || 'Topic Favorites'" data-tour="tree-favorites-btn">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.563 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
            </svg>
          </button>
          <button v-if="!showHistory" class="btn btn-ghost btn-xs" @click="goToSchemaRegistry" :title="t.tree?.schemaRegistry || 'Schema Registry'" data-tour="tree-schema-btn">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m.75 12 3 3m0 0 3-3m-3 3v-6m-1.5-9H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
            </svg>
          </button>
          <button v-if="!showHistory" class="btn btn-ghost btn-xs" @click="toggleHistory" :title="t.history?.title || 'Browsing History'" data-tour="tree-history-btn">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0Z" />
            </svg>
          </button>
        </div>
      </div>
      <!-- Group Selector -->
      <div v-if="groups.length > 0" class="flex items-center gap-1 overflow-x-auto scrollbar-hide py-1 relative" data-tour="tree-group-selector">
        <span class="text-xs font-bold text-base-content/60 uppercase tracking-wider mr-2 flex-shrink-0">{{ t.clusters.group }}:</span>
        <button
          class="btn btn-xs btn-ghost px-0.5 flex-shrink-0 hover:bg-base-200"
          @click="scrollGroups(-150)"
          :title="t.clusters?.scrollLeft || 'Scroll left'"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-2.5 h-2.5">
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
            v-for="group in groups"
            :key="group.id"
            class="btn btn-xs btn-ghost whitespace-nowrap flex-shrink-0"
            :class="{ 'btn-active': selectedGroupId === group.id }"
            @click="selectGroup(group.id)"
          >
            {{ group.name }}
          </button>
        </div>
        <button
          class="btn btn-xs btn-ghost px-0.5 flex-shrink-0 hover:bg-base-200"
          @click="scrollGroups(150)"
          :title="t.clusters?.scrollRight || 'Scroll right'"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-2.5 h-2.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Topic History Panel -->
    <div v-if="showHistory" class="flex-1 overflow-y-auto p-2 pb-2 relative">
      <TopicHistory :t="t" @close="showHistory = false" />
    </div>

    <!-- Tree Content - Scrollable Area -->
    <div v-else class="flex-1 overflow-y-auto p-2 relative">
      <div v-for="cluster in filteredClusters" :key="cluster.name" class="mb-1">
        <!-- Cluster Node -->
        <div
          class="relative"
        >
          <div
            class="flex items-center p-2 rounded-xl cursor-pointer hover:bg-primary/5 sticky top-0 z-30 bg-base-100/95 group overflow-visible"
            :class="{ 'bg-primary/10 shadow-inner': expandedClusters.has(cluster.name) }"
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
                  'bg-success shadow-[0_0_4px_rgba(16,185,129,0.4)]': getClusterHealth(cluster.name)?.healthy === true && !refreshingTopics.has(cluster.name) && !refreshingConsumerGroups.has(cluster.name),
                  'bg-error shadow-[0_0_4px_rgba(239,68,68,0.4)]': getClusterHealth(cluster.name)?.healthy === false && !refreshingTopics.has(cluster.name) && !refreshingConsumerGroups.has(cluster.name),
                  'bg-warning shadow-[0_0_4px_rgba(245,158,11,0.4)]': getClusterHealth(cluster.name)?.healthy === undefined && !refreshingTopics.has(cluster.name) && !refreshingConsumerGroups.has(cluster.name),
                  'bg-warning animate-pulse': refreshingTopics.has(cluster.name) || refreshingConsumerGroups.has(cluster.name)
                }"
                data-tour="tree-cluster-health-dot"
              ></div>
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary flex-shrink-0">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
              </svg>
              <span class="text-sm font-semibold truncate cursor-pointer" data-tour="tree-cluster-name" @dblclick.stop="goToClusterDetail(cluster.name)" :title="t.clusters?.viewTopics || 'View cluster details'">{{ cluster.name }}</span>
            </div>
          </div>
        </div>

        <!-- Cluster Children -->
        <div v-show="expandedClusters.has(cluster.name)" class="flex flex-col">
          <!-- Topics Folder -->
          <div class="mb-0.5 sticky top-0 z-20 bg-base-100/95 flex-shrink-0">
            <div
              class="flex items-center p-1.5 rounded-lg cursor-pointer hover:bg-secondary/5 relative"
              :class="{ 'bg-secondary/10': expandedTopicsFolders.has(cluster.name) }"
              @click.stop="handleTopicsFolderClickAndExpand(cluster.name)"
              data-tour="tree-topics-folder"
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
                class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0 ml-0.5 opacity-100 hover:opacity-80 transition-opacity"
                :class="{ 'opacity-100': refreshingTopics.has(cluster.name) }"
                @click.stop="refreshClusterTopics(cluster.name)"
                :disabled="refreshingTopics.has(cluster.name)"
                :title="t.clusters?.refreshTopics || 'Refresh Topics'"
                data-tour="tree-topics-refresh"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke-width="1.5"
                  stroke="currentColor"
                  class="w-3 h-3"
                  :class="{ 'animate-spin': refreshingTopics.has(cluster.name) }"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
                </svg>
              </button>
            </div>

            <!-- Topics Scrollable Container (includes search box + topics list) -->
            <div v-show="expandedTopicsFolders.has(cluster.name)" class="pl-3 flex flex-col max-h-[500px]">
              <!-- Topic Search Box -->
              <div v-if="getTotalTopics(cluster.name) > 0" class="mb-1 flex-shrink-0 py-1">
                <div class="join w-full">
                  <input
                    v-model="topicSearchQuery[cluster.name]"
                    type="text"
                    :placeholder="`Search ${getTotalTopics(cluster.name)}...`"
                    class="input input-bordered input-xs join-item w-full"
                    @click.stop"
                    data-tour="tree-topic-search"
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

              <RecycleScroller
                :ref="el => setTopicScroller(cluster.name, el)"
                :key="cluster.name"
                class="topic-scroller flex-1"
                :items="getClusterTopics(cluster.name)"
                :item-size="28"
                key-field="uid"
                v-slot="{ item }"
              >
                <div
                  class="flex items-center px-1.5 py-0.5 rounded-lg cursor-pointer hover:bg-accent/5"
                  :class="{ 'bg-accent/10 text-accent': selectedTopic?.name === (item as Topic).name && selectedTopic?.cluster === cluster.name }"
                  @click="selectTopic(item as Topic, cluster.name)"
                >
                  <div class="flex items-center gap-1 flex-1 min-w-0">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-secondary flex-shrink-0">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
                    </svg>
                    <span class="text-xs truncate flex-1 min-w-0" :title="(item as Topic).name" data-tour="tree-topic-name">{{ (item as Topic).name }}</span>
                  </div>
                </div>
              </RecycleScroller>
            </div>

            <!-- Consumer Groups Folder -->
            <div class="mb-0.5 sticky top-0 z-20 bg-base-100/95 flex-shrink-0">
              <div
                class="flex items-center p-1.5 rounded-lg cursor-pointer hover:bg-secondary/5 relative"
                :class="{ 'bg-secondary/10': expandedConsumerGroupsFolders.has(cluster.name) }"
                @click.stop="handleConsumerGroupsFolderClickAndExpand(cluster.name)"
                data-tour="tree-consumer-groups-folder"
              >
                <div class="flex items-center gap-1 flex-1 min-w-0">
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
                  class="btn btn-ghost btn-xs p-0 w-5 h-5 min-h-0 ml-0.5 opacity-100 hover:opacity-80 transition-opacity"
                  :class="{ 'opacity-100': refreshingConsumerGroups.has(cluster.name) }"
                  @click.stop="refreshClusterConsumerGroups(cluster.name)"
                  :disabled="refreshingConsumerGroups.has(cluster.name)"
                  :title="t.tree?.refreshConsumerGroups || 'Refresh Consumer Groups'"
                  data-tour="tree-consumer-groups-refresh"
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

              <!-- Consumer Groups Scrollable Container -->
              <div v-show="expandedConsumerGroupsFolders.has(cluster.name)" class="pl-3 overflow-y-auto max-h-[500px]">
                <!-- Consumer Group Search Box - sticky under Consumer Groups folder -->
                <div v-if="getTotalConsumerGroups(cluster.name) > 0" class="mb-1 sticky top-0 bg-base-100 z-10 py-1">
                  <div class="join w-full">
                    <input
                      v-model="consumerGroupSearchQuery[cluster.name]"
                      type="text"
                      :placeholder="`Search ${getTotalConsumerGroups(cluster.name)}...`"
                      class="input input-bordered input-xs join-item w-full"
                      @click.stop"
                      data-tour="tree-consumer-group-search"
                    />
                    <button
                      v-if="consumerGroupSearchQuery[cluster.name]"
                      class="btn join-item btn-ghost btn-xs"
                      @click.stop="setConsumerGroupSearch(cluster.name, '')"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </div>
                  <p v-if="!consumerGroupSearchQuery[cluster.name]" class="text-[10px] text-base-content/50 mt-0.5">
                    {{ getTotalConsumerGroups(cluster.name) }} consumer groups
                  </p>
                  <p v-else class="text-[10px] text-primary mt-0.5">
                    {{ getClusterConsumerGroups(cluster.name).length }} matching
                  </p>
                </div>

                <div
                  v-for="group in getClusterConsumerGroups(cluster.name)"
                  :key="group.name"
                  class="mb-0.5"
                  ref="el => setConsumerGroupElementRef(`${cluster.name}:${group.name}`, el)"
                >
                  <!-- Consumer Group Node -->
                  <div
                    class="flex items-center p-1.5 rounded-lg cursor-pointer hover:bg-accent/5"
                    :class="{ 'bg-accent/10 text-accent': selectedConsumerGroup?.name === group.name && selectedConsumerGroup?.cluster === cluster.name }"
                    @click="selectConsumerGroup(group, cluster.name)"
                  >
                    <div class="flex items-center gap-1 flex-1 min-w-0">
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-secondary flex-shrink-0">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                      </svg>
                      <span class="text-xs truncate flex-1 min-w-0" :title="group.name" data-tour="tree-consumer-group-name">{{ group.name }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Connection Error Dialog -->
    <Teleport to="body">
      <dialog ref="errorDialogRef" class="modal modal-bottom sm:modal-middle" @click.self="handleErrorDialogClose">
        <div class="modal-box w-full max-w-sm mx-2 md:mx-auto p-5">
          <!-- Header -->
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-error/20 to-error/10 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-error">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
                </svg>
              </div>
              <div>
                <h3 class="font-bold text-base">Connection Failed</h3>
              </div>
            </div>
            <button class="btn btn-sm btn-circle btn-ghost" @click="handleErrorDialogClose">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- Content -->
          <div class="space-y-3">
            <p class="text-sm">
              Cluster: <span class="font-mono">{{ errorDialogClusterName }}</span>
            </p>
            <p class="text-sm" v-if="errorDialogBroker">
              Broker: <span class="font-mono text-error">{{ errorDialogBroker }}</span>
            </p>
            <p class="text-sm text-error" v-if="errorDialogDetail">{{ errorDialogDetail }}</p>
            <p class="text-sm text-error" v-else>{{ errorDialogMessage }}</p>
          </div>

          <!-- Footer -->
          <div class="modal-action flex-wrap gap-2 pt-3">
            <button type="button" class="btn btn-ghost btn-sm" @click="handleErrorDialogClose">Cancel</button>
            <button type="button" class="btn btn-error btn-sm" @click="handleErrorDialogRetry">Retry</button>
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
import { ref, shallowReactive, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import { apiClient } from '@/api/client';
import TopicHistory from '@/components/TopicHistory.vue';

interface Topic {
  name: string;
  uid: string;
  partitions?: Array<{ id: number }>;
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
const languageStore = useLanguageStore();
const clusters = computed(() => clusterStore.clusters);
const groups = computed(() => clusterStore.groups);

// 翻译
const t = computed(() => languageStore.t);

// 当前选中的分组（0 表示所有分组）
const selectedGroupId = ref<number | null>(null);
const hasLoadedGroupSelection = ref(false);

// Show history panel
const showHistory = ref(false);

function toggleHistory() {
  showHistory.value = !showHistory.value;
}

// 从数据库加载选中的分组
async function loadGroupSelection() {
  if (hasLoadedGroupSelection.value) return;
  try {
    const settings = await apiClient.getSettings(['ui.selected_group_id']);
    const setting = settings.find(s => s.key === 'ui.selected_group_id');
    if (setting && setting.value) {
      const groupId = parseInt(setting.value, 10);
      if (!isNaN(groupId)) {
        selectedGroupId.value = groupId;
      }
    }
    hasLoadedGroupSelection.value = true;
  } catch (e) {
    console.warn('Failed to load group selection:', e);
    hasLoadedGroupSelection.value = true;
  }
}

// 保存选中的分组到数据库
async function saveGroupSelection() {
  try {
    const value = selectedGroupId.value === null ? 'null' : String(selectedGroupId.value);
    await apiClient.updateSetting('ui.selected_group_id', value);
  } catch (e) {
    console.error('Failed to save group selection:', e);
  }
}

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

const expandedClusters = ref(new Set<string>());
const refreshingTopics = ref(new Set<string>()); // 正在刷新 topics 的集群
const refreshingConsumerGroups = ref(new Set<string>()); // 正在刷新 consumer groups 的集群
const loadingClusters = ref(new Set<string>()); // 正在加载 topics 的集群
const expandedTopicsFolders = ref(new Set<string>()); // Topics 文件夹展开状态
const expandedConsumerGroupsFolders = ref(new Set<string>()); // Consumer Groups 文件夹展开状态

// 按分组过滤集群
const filteredClusters = computed(() => {
  if (selectedGroupId.value === null) {
    return clusters.value;
  }
  return clusters.value.filter(c => (c.group_id ?? 0) === selectedGroupId.value);
});

// 选中状态
const selectedTopic = ref<{ name: string; cluster: string } | null>(null);
const selectedPartition = ref<{ topic: string; partition: number; cluster: string } | null>(null);

const topicCounts = shallowReactive<Record<string, number>>({});
const clusterTopics = shallowReactive<Record<string, Topic[]>>({});

// Consumer Groups state
const consumerGroupCounts = shallowReactive<Record<string, number>>({});
const clusterConsumerGroups = shallowReactive<Record<string, { name: string }[]>>({});
const consumerGroupSearchQuery = shallowReactive<Record<string, string>>({});
const selectedConsumerGroup = ref<{ name: string; cluster: string } | null>(null);
const consumerGroupElementRefs = shallowReactive<Record<string, HTMLDivElement | null>>({});

// 错误对话框状态
const errorDialogRef = ref<HTMLDialogElement | null>(null);
const errorDialogClusterName = ref<string>('');
const errorDialogMessage = ref<string>('');
const errorDialogBroker = ref<string>('');
const errorDialogDetail = ref<string>('');
const pendingClusterName = ref<string | null>(null); // 等待展开的集群

function getClusterHealth(clusterId: string) {
  return clusterStore.getClusterHealth(clusterId);
}

function selectGroup(groupId: number | null) {
  selectedGroupId.value = groupId;
  saveGroupSelection();
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
    emit('toast', 'error', `${t.value.clusters.connectionFailed}: ${clusterName} - ${errorMsg}`);
    return;
  }

  // 其他错误（如配置错误、认证失败等）显示错误对话框
  errorDialogClusterName.value = clusterName;

  // 尝试解析 broker 信息 (格式: "xxx (broker: host:port): detail")
  const brokerMatch = errorMsg.match(/\(broker:\s*([^)]+)\)/);
  if (brokerMatch) {
    errorDialogBroker.value = brokerMatch[1] || '';
    errorDialogDetail.value = errorMsg.replace(/\(broker:\s*[^)]+\)\s*:?\s*/, '').trim();
  } else {
    errorDialogBroker.value = '';
    errorDialogDetail.value = '';
  }
  errorDialogMessage.value = errorMsg;

  pendingClusterName.value = clusterName;
  nextTick(() => {
    errorDialogRef.value?.showModal();
  });
}

// 处理对话框关闭（取消）
function handleErrorDialogClose() {
  errorDialogRef.value?.close();
  pendingClusterName.value = null;
  errorDialogBroker.value = '';
  errorDialogDetail.value = '';
}

// 处理对话框重试
function handleErrorDialogRetry() {
  errorDialogRef.value?.close();
  if (pendingClusterName.value) {
    checkClusterHealth(pendingClusterName.value);
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

function goToClusters() {
  emit('navigate', {
    path: '/clusters'
  });
}

function goToClusterDetail(clusterName: string) {
  emit('navigate', { path: '/topics', query: { cluster: clusterName } });
}

function goToFavorites() {
  emit('navigate', {
    path: '/favorites'
  });
}

function goToSchemaRegistry() {
  // 优先使用选中的 topic 所在的集群
  if (selectedTopic.value) {
    emit('navigate', {
      path: '/schema-registry',
      query: { cluster: selectedTopic.value.cluster }
    });
    return;
  }
  // 否则使用第一个可用的集群
  if (clusters.value && clusters.value.length > 0) {
    emit('navigate', {
      path: '/schema-registry',
      query: { cluster: clusters.value[0]?.name! }
    });
  } else {
    emit('navigate', {
      path: '/schema-registry'
    });
  }
}

function collapseAll() {
  expandedClusters.value = new Set();
  expandedTopicsFolders.value = new Set();
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
          uid: `${clusterName}:${name}`,
          partitions: []
        }));
        topicCounts[clusterName] = refreshedTopics?.length || 0;
      } else {
        clusterTopics[clusterName] = topics.map((name: string) => ({
          name,
          uid: `${clusterName}:${name}`,
          partitions: []
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

async function refreshClusterTopics(clusterName: string) {
  if (refreshingTopics.value.has(clusterName)) {
    emit('toast', 'info', t.value.clusters.refreshingBg);
    return;
  }

  refreshingTopics.value = new Set(refreshingTopics.value.add(clusterName));

  // 如果搜索框有内容，只刷新该单个 topic
  const searchQuery = topicSearchQuery[clusterName];
  const isSingleTopic = searchQuery && searchQuery.trim() !== '';

  if (isSingleTopic) {
    emit('toast', 'success', `Refreshing topic "${searchQuery.trim()}"...`);
  } else {
    emit('toast', 'success', t.value.clusters.refreshingBg);
  }

  // Fire-and-forget: 立即返回，不等待后端响应
  apiClient.refreshTopics(clusterName, isSingleTopic ? searchQuery.trim() : undefined).catch(() => {});

  // 等待一小段时间后重新加载 topics
  setTimeout(async () => {
    const savedTopics = await apiClient.getSavedTopics(clusterName).catch(() => []);
    clusterTopics[clusterName] = (savedTopics || []).map((name: string) => ({
      name,
      uid: `${clusterName}:${name}`,
      partitions: []
    }));
    topicCounts[clusterName] = savedTopics?.length || 0;
    expandedTopicsFolders.value = new Set(expandedTopicsFolders.value.add(clusterName));
    refreshingTopics.value.delete(clusterName);
    refreshingTopics.value = new Set(refreshingTopics.value);
  }, 500);
}

// 虚拟滚动相关
const topicSearchQuery = shallowReactive<Record<string, string>>({});
const topicScrollers = shallowReactive<Record<string, any>>({});

function setTopicScroller(clusterName: string, el: any) {
  topicScrollers[clusterName] = el;
}

function getClusterTopics(clusterName: string): Topic[] {
  const topics = clusterTopics[clusterName] || [];
  const query = topicSearchQuery[clusterName];

  const filtered = query && query.trim()
    ? topics.filter(t => t.name.toLowerCase().includes(query.toLowerCase()))
    : topics;

  // 为 RecycleScroller 添加 uid
  return filtered.map(t =>
    t.uid ? t : { ...t, uid: `${clusterName}:${t.name}` }
  );
}

function setTopicSearch(clusterName: string, query: string) {
  topicSearchQuery[clusterName] = query;
}

function getTotalTopics(clusterName: string): number {
  return (clusterTopics[clusterName] || []).length;
}

async function selectTopic(topic: Topic, clusterName: string) {
  selectedTopic.value = { name: topic.name, cluster: clusterName };
  selectedPartition.value = null;

  // 确保集群已连接
  await connectClusterIfNeeded(clusterName);

  emit('navigate', { path: '/messages', query: { cluster: clusterName, topic: topic.name } });
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

// ==================== Consumer Groups Functions ====================

// @ts-ignore - used in template
function setConsumerGroupElementRef(key: string, el: HTMLDivElement | null) {
  consumerGroupElementRefs[key] = el;
}

async function handleConsumerGroupsFolderClickAndExpand(clusterName: string) {
  // 点击 Consumer Groups 文件夹时，先展开文件夹并加载 consumer groups
  if (!expandedConsumerGroupsFolders.value.has(clusterName)) {
    expandedConsumerGroupsFolders.value = new Set(expandedConsumerGroupsFolders.value.add(clusterName));
    await loadClusterConsumerGroups(clusterName);
  }
  // 然后导航到 Consumer Groups 页面
  emit('navigate', { path: '/consumer-groups', query: { cluster: clusterName } });
}

function handleConsumerGroupsFolderToggle(clusterName: string) {
  if (expandedConsumerGroupsFolders.value.has(clusterName)) {
    expandedConsumerGroupsFolders.value.delete(clusterName);
    expandedConsumerGroupsFolders.value = new Set(expandedConsumerGroupsFolders.value);
  } else {
    expandedConsumerGroupsFolders.value = new Set(expandedConsumerGroupsFolders.value.add(clusterName));
    loadClusterConsumerGroups(clusterName);
  }
}

async function loadClusterConsumerGroups(clusterName: string) {
  try {
    // 先从数据库获取已保存的 consumer groups
    const groups = await apiClient.getSavedConsumerGroups(clusterName);

    // 如果数据库中没有 consumer groups，从 Kafka 集群实时获取并保存到数据库
    if (!groups || groups.length === 0) {
      // 调用刷新接口，将集群 consumer groups 同步到数据库
      await apiClient.refreshConsumerGroups(clusterName);
      // 刷新后重新获取完整的 consumer groups 列表
      const refreshedGroups = await apiClient.getSavedConsumerGroups(clusterName);
      clusterConsumerGroups[clusterName] = (refreshedGroups || []).map((name: string) => ({
        name
      }));
      consumerGroupCounts[clusterName] = refreshedGroups?.length || 0;
    } else {
      clusterConsumerGroups[clusterName] = groups.map((name: string) => ({
        name
      }));
      consumerGroupCounts[clusterName] = groups.length;
    }
  } catch (error) {
    console.error('[ClusterTreeNavigator] Failed to load consumer groups:', error);
  }
}

async function refreshClusterConsumerGroups(clusterName: string) {
  if (refreshingConsumerGroups.value.has(clusterName)) return;

  // 如果搜索框有内容，只刷新该单个 consumer group
  const searchQuery = consumerGroupSearchQuery[clusterName];
  const isSingleGroup = searchQuery && searchQuery.trim() !== '';

  refreshingConsumerGroups.value = new Set(refreshingConsumerGroups.value.add(clusterName));
  try {
    // 调用后端 API 刷新 consumer groups（从 Kafka 集群同步到数据库）
    apiClient.refreshConsumerGroups(clusterName, isSingleGroup ? searchQuery.trim() : undefined).catch(() => {});

    // 等待后台同步完成
    await new Promise(resolve => setTimeout(resolve, 500));

    // 刷新后重新获取完整的 consumer groups 列表
    const savedGroups = await apiClient.getSavedConsumerGroups(clusterName);
    clusterConsumerGroups[clusterName] = (savedGroups || []).map((name: string) => ({
      name
    }));
    consumerGroupCounts[clusterName] = savedGroups?.length || 0;
  } catch (error: any) {
    // 检查是否是"正在刷新中"错误
    if (error.message?.includes('already being refreshed')) {
      console.warn('Consumer group refresh is already in progress, please wait');
    } else {
      console.error('Failed to refresh consumer groups:', error);
    }
  } finally {
    refreshingConsumerGroups.value.delete(clusterName);
    refreshingConsumerGroups.value = new Set(refreshingConsumerGroups.value);
  }
}

function getClusterConsumerGroups(clusterName: string): { name: string }[] {
  const groups = clusterConsumerGroups[clusterName] || [];
  const query = consumerGroupSearchQuery[clusterName];

  if (query && query.trim()) {
    // 搜索模式：过滤匹配的 group
    const lowerQuery = query.toLowerCase();
    return groups.filter(g => g.name.toLowerCase().includes(lowerQuery));
  }

  return groups;
}

function setConsumerGroupSearch(clusterName: string, query: string) {
  consumerGroupSearchQuery[clusterName] = query;
}

function getTotalConsumerGroups(clusterName: string): number {
  return (clusterConsumerGroups[clusterName] || []).length;
}

async function selectConsumerGroup(group: { name: string }, clusterName: string) {
  selectedConsumerGroup.value = { name: group.name, cluster: clusterName };

  // 确保集群已连接
  await connectClusterIfNeeded(clusterName);

  emit('navigate', { path: '/consumer-groups', query: { cluster: clusterName, group: group.name } });
}

// ==================== End Consumer Groups Functions ====================

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

  // 滚动到选中的 topic（使用虚拟滚动 scroller）
  await nextTick();
  const scroller = topicScrollers[clusterName];
  if (scroller) {
    const topics = getClusterTopics(clusterName);
    const index = topics.findIndex(t => t.name === topicName);
    if (index >= 0) {
      scroller.scrollToItem(index);
    }
  }

  // 导航到消息页面（如果集群不可用，消息页面会显示错误提示）
  emit('navigate', { path: '/messages', query: { cluster: clusterName, topic: topicName } });
}

// 暴露给父组件的方法
defineExpose({
  highlightAndSelectTopic
});

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
      clusterTopics[cluster] = (topics || []).map((name: string) => ({
        name,
        uid: `${cluster}:${name}`,
        partitions: []
      }));
      topicCounts[cluster] = topics?.length || 0;
    } catch (error) {
      console.error('[ClusterTreeNavigator] Failed to refresh topics:', error);
    }

    // 如果 Topics 文件夹还没有展开，先展开它
    expandedTopicsFolders.value = new Set(expandedTopicsFolders.value.add(cluster));
  }
}

// 监听 groups 变化，验证选中的分组是否仍然有效
watch(() => clusterStore.groups, (newGroups) => {
  if (hasLoadedGroupSelection.value && selectedGroupId.value !== null) {
    const groupExists = newGroups.some(g => g.id === selectedGroupId.value);
    if (!groupExists) {
      // 选中的分组不存在了，重置为 null
      selectedGroupId.value = null;
      saveGroupSelection();
    }
  }
}, { deep: false });

onMounted(() => {
  // 加载分组
  clusterStore.fetchGroups();
  // 加载选中的分组
  loadGroupSelection();
  // 监听刷新事件（从 ModernLayout 的 Refresh Topics 按钮触发）
  window.addEventListener('cluster-topics-refreshed', handleTopicsRefreshed);
  // 监听编辑集群事件
  window.addEventListener('edit-cluster-from-menu', handleEditClusterFromMenu);
  // 监听历史导航事件
  window.addEventListener('navigate-to-topic-from-history', handleNavigateFromHistory);
});

onUnmounted(() => {
  // 清理事件监听
  window.removeEventListener('cluster-topics-refreshed', handleTopicsRefreshed);
  window.removeEventListener('edit-cluster-from-menu', handleEditClusterFromMenu);
  window.removeEventListener('navigate-to-topic-from-history', handleNavigateFromHistory);
});

function handleEditClusterFromMenu(event: Event) {
  const customEvent = event as CustomEvent<{ clusterName: string }>;
  const { clusterName } = customEvent.detail;
  // 触发打开创建集群编辑弹窗
  window.dispatchEvent(new CustomEvent('open-create-cluster-modal', {
    detail: { editClusterName: clusterName }
  }));
}

async function handleNavigateFromHistory(event: Event) {
  const customEvent = event as CustomEvent<{ clusterId: string; topicName: string }>;
  const { clusterId, topicName } = customEvent.detail;
  if (clusterId && topicName) {
    showHistory.value = false;
    await highlightAndSelectTopic(topicName, clusterId);
  }
}
</script>

<style scoped>
/* Cluster actions dropdown positioning fix */
.cluster-actions-dropdown {
  position: fixed;
  z-index: 1000;
}

/* Hide scrollbar but allow scrolling for group selector */
.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

/* Connection error dialog backdrop */
:global(.modal:has(.modal-box)::backdrop) {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:global(dialog[open]::backdrop) {
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

/* Topic virtual scroller */
.topic-scroller {
  overflow-y: auto;
}

.topic-scroller::-webkit-scrollbar {
  width: 4px;
}

.topic-scroller::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.3);
  border-radius: 2px;
}
</style>
