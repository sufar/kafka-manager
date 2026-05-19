<template>
  <div class="message-query-tool flex-1 flex flex-col min-h-0">
    <!-- 简洁搜索栏 -->
    <div class="toolbar flex flex-wrap items-center gap-1.5 p-1.5 border-b border-base-300 bg-base-100">
      <!-- 返回按钮 -->
      <button class="btn btn-ghost btn-sm p-1" @click="goBack" title="返回" data-tour="messages-back">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
        </svg>
      </button>

      <!-- 分区选择 -->
      <select v-model="selectedPartition" class="select select-bordered select-sm w-28" data-tour="messages-partition">
        <option value="all">{{ t.messages.allPartitions }}</option>
        <option v-for="p in partitions" :key="p" :value="p">{{ t.messages.partition }} {{ p }}</option>
      </select>

      <!-- 查询模式 -->
      <select v-model="fetchMode" class="select select-bordered select-sm w-24" data-tour="messages-mode">
        <option value="newest">{{ t.messages.newest }}</option>
        <option value="oldest">{{ t.messages.oldest }}</option>
      </select>

      <!-- 数量 -->
      <input v-model.number="maxMessages" type="number" class="input input-bordered input-sm w-16" min="1" max="1000" :title="t.messages.maxMessages" data-tour="messages-count" />

      <!-- 高级筛选按钮 -->
      <button
        class="btn btn-sm btn-ghost gap-1"
        :class="{ 'btn-active text-primary': showTimeFilters }"
        @click="showTimeFilters = !showTimeFilters"
        :title="t.messages.timeRangeFilter"
        data-tour="messages-time-filter"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0Z" />
        </svg>
        <span class="hidden sm:inline">{{ t.messages.timeRange }}</span>
      </button>

      <!-- 搜索 -->
      <div class="flex-1 min-w-[120px] relative" data-tour="messages-search">
        <input v-model="searchKeyword" type="text" class="input input-bordered input-sm w-full pr-8" :placeholder="t.messages.valuePlaceholder" @keyup.enter="queryMessages" />
        <button v-if="searchKeyword" class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content" @click="searchKeyword = ''; queryMessages()">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- 查询按钮 -->
      <button class="btn btn-primary btn-sm" :class="{ 'loading': loading }" :disabled="!canQuery || loading" @click="queryMessages" data-tour="messages-query">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
        </svg>
      </button>

      <!-- 停止按钮 -->
      <button v-if="loading" class="btn btn-error btn-sm" @click="stopQuery">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      <!-- 发送消息 -->
      <button class="btn btn-ghost btn-sm" @click="openSendModal" :title="t.messages.sendMessage" data-tour="messages-send">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
        </svg>
      </button>

      <!-- 更多操作菜单 -->
      <div
        class="dropdown dropdown-end"
        :class="{ 'dropdown-open': showMoreMenu }"
        @focusout="handleMoreMenuFocusOut"
        data-tour="messages-more"
      >
        <button
          class="btn btn-ghost btn-sm"
          :disabled="!selectedCluster || !selectedTopic"
          title="更多操作"
          @click="handleMoreMenuClick"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 6.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 12.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 18.75a.75.75 0 110-1.5.75.75 0 010 1.5z" />
          </svg>
        </button>
        <ul tabindex="0" class="dropdown-content menu menu-sm bg-base-100 rounded-box z-20 w-44 p-1 shadow-lg border border-base-200">
          <li>
            <a @click="toggleHistory(); showMoreMenu = false">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0Z" />
              </svg>
              {{ t.sentMessageHistory?.title || '发送历史' }}
            </a>
          </li>
          <li>
            <a @click="viewTopicConsumerGroups(); showMoreMenu = false">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
              </svg>
              {{ t.topicConsumerGroups?.title || '消费者组' }}
            </a>
          </li>
          <li>
            <a class="text-error" @click="handleDeleteTopic(); showMoreMenu = false">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
              </svg>
              {{ t.messages?.deleteTopic || '删除主题' }}
            </a>
          </li>
        </ul>
      </div>
    </div>

    <!-- 时间范围筛选面板 -->
    <div v-if="showTimeFilters" class="time-filters flex flex-wrap items-center gap-2 px-3 py-2 border-b border-base-300 bg-base-200/50 animate-fadeIn" data-tour="messages-time-presets">
      <div class="flex items-center gap-1.5">
        <span class="text-xs font-medium text-base-content/70">{{ t.messages.startTime }}:</span>
        <input
          v-model="filters.startTime"
          type="text"
          class="input input-bordered input-sm w-48 font-mono text-xs"
          placeholder="YYYY-MM-DD HH:mm:ss"
          @blur="formatDateTime('startTime')"
        />
      </div>
      <div class="flex items-center gap-1.5">
        <span class="text-xs font-medium text-base-content/70">{{ t.messages.endTime }}:</span>
        <input
          v-model="filters.endTime"
          type="text"
          class="input input-bordered input-sm w-48 font-mono text-xs"
          placeholder="YYYY-MM-DD HH:mm:ss"
          @blur="formatDateTime('endTime')"
        />
      </div>
      <!-- 快捷按钮 -->
      <div class="flex items-center gap-0.5 ml-1">
        <button class="btn btn-ghost btn-xs" @click="setPresetTime(5)" :title="t.messages.recent5Minutes">5 {{ t.messages.minutes }}</button>
        <button class="btn btn-ghost btn-xs" @click="setPresetTime(15)" :title="t.messages.recent15Minutes">15 {{ t.messages.minutes }}</button>
        <button class="btn btn-ghost btn-xs" @click="setPresetTime(30)" :title="t.messages.recent30Minutes">30 {{ t.messages.minutes }}</button>
        <button class="btn btn-ghost btn-xs" @click="setPresetTime(60)" :title="t.messages.recent1Hour">1 {{ t.messages.hour }}</button>
        <button class="btn btn-ghost btn-xs" @click="setPresetTime(24 * 60)" :title="t.messages.recent1Day">1 {{ t.messages.day }}</button>
      </div>
      <button
        class="btn btn-ghost btn-xs gap-1"
        @click="clearTimeFilters"
        :disabled="!filters.startTime && !filters.endTime"
      >
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
        {{ t.messages.clear }}
      </button>
    </div>

    <!-- 状态栏 -->
    <div class="status-bar flex items-center justify-between px-3 py-1 text-xs border-b border-base-300 bg-base-200/50">
      <div class="flex items-center gap-4">
        <span v-if="selectedTopic" class="text-base-content/70 flex items-center gap-1">
          {{ t.messages.topicLabel }}:
          <span class="font-mono font-bold text-primary relative group cursor-help">
            {{ selectedTopic }}
            <span class="invisible group-hover:visible absolute top-full left-0 mt-1 px-2 py-1 bg-base-300 text-base-content text-xs rounded whitespace-nowrap z-50">
              {{ t.clusters.clusters }}: {{ props.cluster || selectedCluster }}
            </span>
          </span>
          <FavoriteButton
            v-if="props.cluster && selectedTopic"
            :cluster-id="props.cluster"
            :topic-name="selectedTopic"
            :t="t"
          />
        </span>
        <!-- 流式进度指示器 -->
        <span v-if="streamingProgress.isStreaming" class="flex items-center gap-1.5 text-info">
          <span class="loading loading-spinner loading-xs"></span>
          <span>{{ t.messages.receiving }} {{ streamingProgress.received.toLocaleString() }}</span>
          <span v-if="streamingProgress.total > 0">/ {{ streamingProgress.total.toLocaleString() }}</span>
        </span>
        <span v-else-if="lastQueryTime > 0" class="text-base-content/70">
          {{ t.messages.elapsedTime }}: <span class="font-mono font-bold">{{ lastQueryTime }}ms</span>
        </span>
        <span v-if="messages.length > 0" class="text-base-content/70">
          {{ t.messages.totalMessages }} <span class="font-mono font-bold text-success">{{ messages.length.toLocaleString() }}</span> {{ t.messages.messages }}
          <span class="tooltip tooltip-right ml-1" :data-tip="t.messages.exportTip">
            <button class="btn btn-ghost btn-xs btn-circle" @click="exportMessages" :disabled="messages.length === 0" :title="t.messages.exportMessages">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z" />
              </svg>
            </button>
          </span>
          <button class="btn btn-ghost btn-xs ml-1" :disabled="messages.length === 0" @click="exportMessages" :title="t.messages.exportMessages" data-tour="messages-export">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
            </svg>
          </button>
        </span>
        <span v-if="error" class="text-error">{{ error }}</span>
      </div>
      <!-- 流式进度条 -->
      <div v-if="streamingProgress.isStreaming && streamingProgress.total > 0" class="flex-1 mx-4 hidden md:block">
        <div class="w-full bg-base-300 rounded-full h-1.5 overflow-hidden">
          <div
            class="bg-info h-full rounded-full transition-all duration-300"
            :style="{ width: Math.min((streamingProgress.received / streamingProgress.total) * 100, 100) + '%' }"
          ></div>
        </div>
      </div>
    </div>

    <!-- 消息列表 -->
    <div class="flex-1 overflow-hidden bg-base-100 relative">
      <!-- 加载中状态 -->
      <div v-if="loading && messages.length === 0" class="absolute inset-0 flex items-center justify-center text-base-content/60 pointer-events-none z-10">
        <div class="text-center">
          <span class="loading loading-spinner loading-lg text-primary"></span>
          <p class="text-sm mt-2">{{ t.messages.loading }}</p>
        </div>
      </div>

      <!-- Desktop Table with Virtual Scroll -->
      <div class="hidden md:flex md:flex-col h-full">
        <!-- Table Header -->
        <div class="flex bg-base-200 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide w-full" data-tour="messages-table-header">
          <div class="flex-shrink-0" :style="{ width: columnWidths.partition + 'px' }">
            {{ t.messages.partitionLabel2 }}
          </div>
          <div
            class="resizer w-1 cursor-col-resize hover:bg-primary/40 transition-colors rounded-sm"
            @mousedown="startColumnResize('partition', $event)"
          ></div>
          <div class="flex-shrink-0" :style="{ width: columnWidths.offset + 'px' }">
            {{ t.messages.offsetLabel }}
          </div>
          <div
            class="resizer w-1 cursor-col-resize hover:bg-primary/40 transition-colors rounded-sm"
            @mousedown="startColumnResize('offset', $event)"
          ></div>
          <div class="flex-shrink-0 flex items-center gap-1 cursor-pointer hover:text-primary transition-colors"
            :style="{ width: columnWidths.timestamp + 'px' }"
            @click="toggleTimestampSort">
            {{ t.messages.timestampLabel }}
            <svg v-if="timestampSort === 'asc'" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
              <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 15.75l7.5-7.5 7.5 7.5" />
            </svg>
            <svg v-else-if="timestampSort === 'desc'" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
              <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3 opacity-30">
              <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 15L12 18.75 15.75 15m-7.5-6L12 5.25 15.75 9" />
            </svg>
          </div>
          <div
            class="resizer w-1 cursor-col-resize hover:bg-primary/40 transition-colors rounded-sm"
            @mousedown="startColumnResize('timestamp', $event)"
          ></div>
          <div class="flex-shrink-0" :style="{ width: columnWidths.key + 'px' }">{{ t.messages.key }}</div>
          <div class="resizer w-1 cursor-col-resize hover:bg-primary/40 transition-colors rounded-sm"
            @mousedown="startColumnResize('key', $event)"
          ></div>
          <div class="flex-1" :style="{ minWidth: columnWidths.value + 'px' }">{{ t.messages.value }}</div>
          <div class="flex-shrink-0 text-center" :style="{ width: columnWidths.actions + 'px' }">{{ t.messages.actions }}</div>
          <div
            class="resizer w-1 cursor-col-resize hover:bg-primary/40 transition-colors rounded-sm"
            @mousedown="startColumnResize('actions', $event)"
          ></div>
        </div>
        <!-- Virtual Scroll List -->
        <RecycleScroller
          v-if="sortedMessages.length > 0"
          ref="scrollerRef"
          class="flex-1 overflow-auto w-full"
          :items="sortedMessages"
          :item-size="24"
          key-field="uid"
          :buffer-size="5"
          v-slot="{ item }"
        >
          <div
            class="flex items-center px-2 py-0.5 border-b border-base-200/30 cursor-pointer w-full"
            :class="[
              selectedMessage?.p === getMsgPartition(item) && selectedMessage?.o === getMsgOffset(item)
                ? 'bg-primary/30 border-l-2 border-l-primary shadow-sm'
                : 'hover:bg-base-200/50'
            ]"
            style="height: 24px;"
            @click="selectedMessage = (item as any)"
            data-tour="messages-table-row"
          >
            <div class="flex-shrink-0 text-[10px]" :style="{ width: columnWidths.partition + 'px' }">
              <span class="badge badge-ghost badge-xs scale-90">{{ getMsgPartition(item) }}</span>
            </div>
            <div class="flex-shrink-0 text-[10px] font-mono" :style="{ width: columnWidths.offset + 'px' }">{{ getMsgOffset(item) }}</div>
            <div class="flex-shrink-0 text-[10px] whitespace-nowrap" :style="{ width: columnWidths.timestamp + 'px' }">{{ formatTime(getMsgTimestamp(item)) }}</div>
            <div class="flex-shrink-0 text-[10px] font-mono truncate" :style="{ width: columnWidths.key + 'px' }">{{ getMsgKey(item) || '-' }}</div>
            <div class="flex-1 text-[10px] font-mono truncate pr-2" style="min-width: 0;" :title="getMsgValue(item) || ''">{{ getMsgValue(item) }}</div>
            <div class="flex-shrink-0 text-[10px] flex items-center justify-center" :style="{ width: columnWidths.actions + 'px' }">
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click.stop="copyMessageValue(item as any)" :title="t.messages.copyValue">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
          </div>
        </RecycleScroller>
        <div v-if="sortedMessages.length === 0 && !loading" class="flex-1 flex flex-col items-center justify-center text-base-content/50">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50 mb-2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
          </svg>
          <span>{{ t.messages.noMessages }}</span>
        </div>
      </div>

      <!-- Mobile Card View with Virtual Scroll -->
      <div class="md:hidden flex flex-col h-full">
        <RecycleScroller
          v-if="sortedMessages.length > 0"
          ref="scrollerRefMobile"
          class="flex-1 overflow-auto p-2 pb-20"
          :items="sortedMessages"
          :item-size="76"
          key-field="uid"
          :buffer-size="5"
          v-slot="{ item }"
        >
          <div
            class="card bg-base-100 border border-base-200 p-2 shadow-sm mb-0 cursor-pointer"
            :class="{ 'border-l-2 border-l-primary shadow-primary/30 ring-1 ring-primary/20': selectedMessage?.p === getMsgPartition(item) && selectedMessage?.o === getMsgOffset(item) }"
            @click="selectedMessage = (item as any)"
          >
            <div class="flex items-center justify-between mb-1">
              <div class="flex items-center gap-2">
                <span class="badge badge-ghost badge-xs">P{{ getMsgPartition(item) }}</span>
                <span class="text-xs font-mono text-base-content/70">#{{ getMsgOffset(item) }}</span>
              </div>
              <span class="text-[10px] whitespace-nowrap">{{ formatTime(getMsgTimestamp(item)) }}</span>
            </div>
            <div class="h-4 mb-1 overflow-hidden">
              <div v-if="getMsgKey(item)" class="text-[10px] font-mono text-secondary truncate">
                <span class="opacity-60">{{ t.messages.key }}:</span> {{ getMsgKey(item) }}
              </div>
            </div>
            <div class="text-[10px] font-mono truncate text-base-content/80">
              {{ truncate(getMsgValue(item), 100) }}
            </div>
          </div>
        </RecycleScroller>
        <div v-if="sortedMessages.length === 0 && !loading" class="flex-1 flex flex-col items-center justify-center text-base-content/50 p-2">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50 mb-2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
          </svg>
          <span>{{ t.messages.noMessages }}</span>
        </div>
      </div>
    </div>

    <!-- Detail Panel (Sticky at bottom with resize handle) -->
    <div
      v-if="selectedMessage"
      class="detail-panel border-t border-base-300 bg-base-200/30 flex flex-col"
      :style="{ height: panelHeight + 'px' }"
      tabindex="-1"
      @keydown.ctrl.a.prevent="handleSelectAll"
      @keydown.meta.a.prevent="handleSelectAll"
      @keydown.ctrl.f.prevent="handleDetailSearch"
      @keydown.meta.f.prevent="handleDetailSearch"
      data-tour="messages-detail-panel"
    >
      <!-- Search Bar -->
      <div v-if="detailSearchActive" class="flex items-center gap-1 px-2 py-1 bg-base-100 border-b border-base-200">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 text-base-content/40 flex-shrink-0">
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
        </svg>
        <input
          ref="detailSearchInputRef"
          v-model="detailSearchQuery"
          type="text"
          class="input input-bordered input-xs flex-1 min-w-0"
          placeholder="搜索详情内容..."
          @input="updateDetailSearch"
          @keydown.escape="closeDetailSearch"
        />
        <span v-if="detailSearchQuery && detailMatchCount > 0" class="text-[10px] text-base-content/50 flex-shrink-0 whitespace-nowrap">
          {{ detailMatchIndex }}/{{ detailMatchCount }}
        </span>
        <span v-else-if="detailSearchQuery && detailMatchCount === 0" class="text-[10px] text-error flex-shrink-0">0 匹配</span>
        <button v-if="detailSearchQuery && detailMatchCount > 0" class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="detailSearchPrev" title="上一个">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 15.75l7.5-7.5 7.5 7.5" />
          </svg>
        </button>
        <button v-if="detailSearchQuery && detailMatchCount > 0" class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="detailSearchNext" title="下一个">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3 h-3">
            <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
          </svg>
        </button>
        <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="closeDetailSearch" title="关闭">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      <!-- Resize Handle -->
      <div class="resize-handle h-1 cursor-row-resize bg-base-300 hover:bg-primary/50 transition-colors flex items-center justify-center" @mousedown="startResize">
        <div class="w-8 h-0.5 bg-base-content/20 rounded-full"></div>
      </div>
      <div class="flex-1 overflow-hidden p-2">
        <div class="flex items-center justify-between mb-1.5 pb-1 border-b border-base-content/10">
          <div class="flex items-center gap-2 flex-wrap">
            <h4 class="text-xs font-bold">{{ t.messages.messageDetail }}</h4>
            <span class="text-[10px] text-base-content/50">{{ t.messages.partition }}: <span class="font-mono">{{ selectedMessage.p }}</span></span>
            <span class="text-[10px] text-base-content/50">{{ t.messages.offset }}: <span class="font-mono">{{ selectedMessage.o }}</span></span>
            <span class="text-[10px]">{{ formatTime(selectedMessage.ts) }}</span>
          </div>
          <div class="flex items-center gap-1">
            <button class="btn btn-ghost btn-xs px-1" @click="selectedMessage = null">{{ t.messages.close }}</button>
          </div>
        </div>
        <div ref="detailContentRef" class="space-y-1.5 text-[10px] h-[calc(100%-32px)] overflow-auto flex flex-col">
          <div v-if="selectedMessage.k" class="mb-1">
            <div class="flex items-center justify-between mb-0.5">
              <div class="text-base-content/50 text-[10px] font-semibold">{{ t.messages.key }}:</div>
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="copyKey" :title="t.messages.copyKey">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
            <div ref="keyPreRef" class="bg-base-100 p-1 rounded text-[10px] font-mono border border-base-content/5 truncate">{{ selectedMessage.k }}</div>
          </div>
          <div class="flex flex-col flex-1">
            <div class="flex items-center justify-between mb-0.5">
              <div class="text-base-content/50 text-[10px] font-semibold flex items-center gap-1">
                {{ t.messages.value }}:
                <select v-model="valueViewFormat" class="select select-bordered select-xs scale-90 origin-left" data-tour="messages-value-view-format">
                  <option value="json">{{ t.messages.json }}</option>
                  <option value="raw">{{ t.messages.raw }}</option>
                  <option value="hex">{{ t.messages.hex }}</option>
                </select>
              </div>
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="copyValue" :title="t.messages.copyValue">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
            <pre
              v-if="valueViewFormat === 'json'"
              ref="valuePreRef"
              class="bg-base-100 p-1.5 rounded text-xs font-mono overflow-auto whitespace-pre-wrap border border-base-content/5 flex-1 json-highlight"
              v-html="highlightJson(formatValue(selectedMessage.v, valueViewFormat))"
            ></pre>
            <pre
              v-else
              ref="valuePreRef"
              class="bg-base-100 p-1.5 rounded text-xs font-mono overflow-auto whitespace-pre-wrap border border-base-content/5 flex-1"
            >{{ formatValue(selectedMessage.v, valueViewFormat) }}</pre>
          </div>
        </div>
      </div>
    </div>

    <!-- Send Message Modal -->
    <SendMessageModal
      ref="sendModalRef"
      v-model="showSendModal"
      :topic-name="selectedTopic || ''"
      :cluster-name="clusterName || ''"
      :partitions="partitions"
      :initial-partition="messageFormPartition"
      :initial-key="messageFormKey"
      :initial-value="messageFormValue"
      @submit="handleSubmit"
    />

    <!-- Sent Message History Panel -->
    <div v-show="showHistory" class="absolute inset-0 top-[41px] overflow-y-auto bg-base-100 z-20">
      <SentMessageHistory :t="t" :cluster="selectedCluster" :topic="selectedTopic" @select="handleSelectHistory" @close="showHistory = false" />
    </div>

    <!-- Delete Topic Dialog -->
    <DeleteTopicDialog
      ref="deleteTopicDialogRef"
      :cluster="selectedCluster || ''"
      :topic="selectedTopic || ''"
      @deleted="handleTopicDeleted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, shallowRef, nextTick, reactive } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { RecycleScroller } from 'vue-virtual-scroller';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';
import { useLanguageStore } from '@/stores/language';
import { highlightJsonWithTemplate, clearTemplateCache } from '@/utils/json-highlight';
import { formatJson } from '@/utils/json';
import FavoriteButton from '@/components/FavoriteButton.vue';
import SendMessageModal from '@/components/SendMessageModal.vue';
import SentMessageHistory from '@/components/SentMessageHistory.vue';
import DeleteTopicDialog from '@/components/DeleteTopicDialog.vue';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';

// 键盘导航方向类型
type NavigationDirection = 'up' | 'down';

const route = useRoute();
const router = useRouter();

function goBack() {
  router.back();
}
const { showSuccess, showError } = useToast();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

// 消息类型 - 使用紧凑格式（短字段名减少内存占用）
interface Message {
  p: number;          // partition
  o: number;          // offset
  k: string | null;   // key
  v: string | null;   // value
  ts: number | null;  // timestamp
  uid: string;        // 唯一 ID，用于虚拟滚动
}

// Props - 从父组件接收 cluster 和 topic
const props = defineProps<{
  cluster?: string;
  topic?: string;
}>();

const emit = defineEmits<{
  navigate: [{ path: string; query?: Record<string, string> }];
}>();

// 状态 - 使用 shallowRef 避免深度响应式带来的性能问题
const partitions = ref<number[]>([]);
const messages = shallowRef<Message[]>([]);
const selectedMessage = ref<any>(null);
const valueViewFormat = ref<'json' | 'raw' | 'hex'>('json');
const panelHeight = ref(380); // 默认高度增加到 380px
// 用于强制刷新 JSON 高亮的响应式变量
const jsonHighlightRefresh = ref(0);
// 虚拟滚动 ref
const scrollerRef = ref<any>(null);
const scrollerRefMobile = ref<any>(null);
// 详情面板 ref
const keyPreRef = ref<HTMLElement | null>(null);
const valuePreRef = ref<HTMLElement | null>(null);

// 按时间戳排序状态：'asc' | 'desc' | null（null 表示按查询顺序）
const timestampSort = ref<'asc' | 'desc' | null>('desc');

// 排序后的消息数组 - 维护单一引用避免重复创建副本
const sortedMessages = shallowRef<Message[]>([]);

// 重新计算排序结果
function recomputeSortedMessages() {
  const src = messages.value;
  if (!timestampSort.value) {
    sortedMessages.value = src;
    return;
  }
  // 复用同一数组引用，避免 shallowRef 每次都触发新订阅者
  const result = [...src];
  result.sort((a, b) => {
    const tsA = a.ts ?? 0;
    const tsB = b.ts ?? 0;
    return timestampSort.value === 'asc' ? tsA - tsB : tsB - tsA;
  });
  sortedMessages.value = result;
}

// 监听 messages 和 timestampSort 变化，按需重算
watch([messages, timestampSort], recomputeSortedMessages);

// 切换时间戳排序
function toggleTimestampSort() {
  if (timestampSort.value === null) {
    timestampSort.value = 'desc'; // 默认降序（最新的在前）
  } else if (timestampSort.value === 'desc') {
    timestampSort.value = 'asc';
  } else {
    timestampSort.value = null; // 回到原始顺序
  }
}

// 查询参数
const selectedCluster = ref('');
const clusterName = computed(() => selectedCluster.value);
const selectedTopic = ref('');
const selectedPartition = ref<string | number>('all');
const fetchMode = ref<'newest' | 'oldest'>('newest');
const maxMessages = ref(100);
const searchKeyword = ref('');

// 时间范围筛选
const showTimeFilters = ref(false);
const filters = reactive({
  startTime: '',
  endTime: '',
});

// UI 状态
const loading = ref(false);
const error = ref('');
const lastQueryTime = ref(0);

// SSE 流式状态
const streamingProgress = ref<{ received: number; total: number; isStreaming: boolean }>({ received: 0, total: 0, isStreaming: false });
let currentAbortController: AbortController | null = null;
let currentRequestId = 0;
let isAborted = false;  // 取消标志
let finalizedSort: 'desc' | undefined = undefined;
// 非响应式消息缓存，减少响应式更新频率
let pendingMessages: Message[] = [];
// 加载超时保护定时器
let loadingTimeoutId: ReturnType<typeof setTimeout> | null = null;

// 重置状态
function resetMessageState() {
  messages.value = [];
  pendingMessages = [];
  streamingProgress.value = { received: 0, total: 0, isStreaming: false };
  finalizedSort = undefined;
}

// 批量更新 UI，动态调整更新频率
// 虚拟滚动只会渲染可见区域，所以更新频率可以更高
let updateTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleUpdate() {
  if (updateTimer) return;

  // 动态计算更新间隔：消息越多，间隔越长，避免 UI 卡顿
  // 基础间隔 50ms，每 100 条消息增加 25ms，最大 300ms
  const pendingCount = pendingMessages.length;
  const updateInterval = Math.min(300, Math.max(50, 50 + Math.floor(pendingCount / 100) * 25));

  updateTimer = setTimeout(() => {
    updateTimer = null;
    const count = pendingMessages.length;
    if (count > 0) {
      // 合并到 messages，而不是覆盖
      messages.value = [...messages.value, ...pendingMessages];
      pendingMessages = [];
      // console.log(`[UI Update] +${count} messages, total: ${messages.value.length}`);
      // 强制刷新虚拟滚动
      nextTick(() => {
        scrollerRef.value?.refresh();
        scrollerRefMobile.value?.refresh();
      });
    }
  }, updateInterval);
}

// 发送消息弹框状态
const sendModalRef = ref<InstanceType<typeof SendMessageModal> | null>(null);
const showSendModal = ref(false);
const messageFormPartition = ref(0);
const messageFormKey = ref('');
const messageFormValue = ref('');

// 发送历史状态
const showHistory = ref(false);
const showMoreMenu = ref(false);
let moreMenuButtonClicked = false;

// Column widths for the message table (in px)
type ColumnKey = 'partition' | 'offset' | 'timestamp' | 'key' | 'value' | 'actions';
const columnWidths = ref<Record<ColumnKey, number>>({
  partition: 48,
  offset: 64,
  timestamp: 112,
  key: 80,
  value: 200,
  actions: 40,
});
const columnResizing = ref(false);
const resizeColumn = ref<ColumnKey | null>(null);
const resizeStartX = ref(0);
const resizeStartWidth = ref(0);

// Detail panel search
const detailSearchActive = ref(false);
const detailSearchQuery = ref('');
const detailSearchInputRef = ref<HTMLInputElement | null>(null);
const detailContentRef = ref<HTMLElement | null>(null);
const detailMatchIndex = ref(0);
const detailMatchCount = ref(0);

function startColumnResize(col: ColumnKey, e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
  columnResizing.value = true;
  resizeColumn.value = col;
  resizeStartX.value = e.clientX;
  resizeStartWidth.value = columnWidths.value[col];
  document.addEventListener('mousemove', onColumnResize);
  document.addEventListener('mouseup', stopColumnResize);
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function onColumnResize(e: MouseEvent) {
  if (!resizeColumn.value) return;
  const delta = e.clientX - resizeStartX.value;
  const newWidth = Math.max(30, resizeStartWidth.value + delta);
  columnWidths.value[resizeColumn.value] = newWidth;
}

function stopColumnResize() {
  columnResizing.value = false;
  resizeColumn.value = null;
  document.removeEventListener('mousemove', onColumnResize);
  document.removeEventListener('mouseup', stopColumnResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

function toggleHistory() {
  showHistory.value = !showHistory.value;
}

function handleMoreMenuFocusOut(event: FocusEvent) {
  if (moreMenuButtonClicked) return;
  const target = event.relatedTarget as Node | null;
  const dropdown = (event.target as HTMLElement).closest('.dropdown');
  if (dropdown && !dropdown?.contains(target)) {
    showMoreMenu.value = false;
  }
}

function handleMoreMenuClick() {
  moreMenuButtonClicked = true;
  showMoreMenu.value = !showMoreMenu.value;
  setTimeout(() => { moreMenuButtonClicked = false; }, 100);
}

// 查看 Topic 的消费者组
function viewTopicConsumerGroups() {
  if (!selectedCluster.value || !selectedTopic.value) return;
  // 跳转到 topic-consumer-groups 页面
  emit('navigate', {
    path: '/topic-consumer-groups',
    query: {
      cluster: selectedCluster.value,
      topic: selectedTopic.value,
    },
  });
}

// 删除当前 Topic
const deleteTopicDialogRef = ref<InstanceType<typeof DeleteTopicDialog> | null>(null);

function handleDeleteTopic() {
  if (!selectedCluster.value || !selectedTopic.value) return;
  deleteTopicDialogRef.value?.open();
}

function handleTopicDeleted(cluster: string, topic: string) {
  window.dispatchEvent(new CustomEvent('topic-deleted', { detail: { cluster, topic } }));
  // 如果删除的是当前正在查看的 topic，清空右侧界面
  if (cluster === selectedCluster.value && topic === selectedTopic.value) {
    selectedTopic.value = '';
    messages.value = [];
    selectedMessage.value = null;
  }
}

// 处理选择历史消息
function handleSelectHistory(message: { cluster_id: string; topic_name: string; partition: number; message_key: string | null; message_value: string; headers?: any }) {
  // 填充表单
  messageFormPartition.value = message.partition;
  messageFormKey.value = message.message_key || '';
  messageFormValue.value = message.message_value;
  // 关闭历史面板
  showHistory.value = false;
  // 打开弹窗
  showSendModal.value = true;
}

// 计算属性
const canQuery = computed(() => {
  return !!selectedCluster.value && !!selectedTopic.value && !loading.value;
});

// 方法
async function loadPartitions() {
  if (!selectedCluster.value || !selectedTopic.value) return;

  try {
    const detail = await apiClient.getTopicDetail(selectedCluster.value, selectedTopic.value);
    partitions.value = detail.partitions?.map((p: { id: number }) => p.id) || [];
  } catch (e) {
    console.error('Failed to fetch partitions:', e);
  }
}

// 记录浏览历史
async function recordHistory() {
  if (!selectedCluster.value || !selectedTopic.value) return;
  try {
    await apiClient.recordTopicHistory(selectedCluster.value, selectedTopic.value);
  } catch (e) {
    console.error('[MessageQueryTool] Failed to record history:', e);
  }
}

async function queryMessages() {
  console.log('[MessageQueryTool] queryMessages called, canQuery:', canQuery.value, 'isAborted:', isAborted);
  if (!canQuery.value) return;

  // 验证时间格式
  if (filters.startTime) {
    const startDate = parseDateTime(filters.startTime);
    if (!startDate) {
      showError(`${t.value.messages.startTime} ${t.value.toast.invalidFormat}`);
      return;
    }
  }
  if (filters.endTime) {
    const endDate = parseDateTime(filters.endTime);
    if (!endDate) {
      showError(`${t.value.messages.endTime} ${t.value.toast.invalidFormat}`);
      return;
    }
  }

  // 验证时间范围
  if (filters.startTime && filters.endTime) {
    const startDate = parseDateTime(filters.startTime);
    const endDate = parseDateTime(filters.endTime);
    if (startDate && endDate && startDate > endDate) {
      showError(`${t.value.messages.startTime} ${t.value.common.cannotBeGreaterThan} ${t.value.messages.endTime}`);
      return;
    }
  }

  // 取消上一次的请求
  stopQuery();
  // 重置取消标志，允许新的查询
  isAborted = false;

  // 增加请求序列号
  currentRequestId++;
  const requestId = currentRequestId;

  loading.value = true;
  error.value = '';
  // 保留 timestampSort 排序状态，不重置
  resetMessageState();
  const startTime = performance.now();

  // 设置加载超时保护：90 秒后自动停止加载
  if (loadingTimeoutId) {
    clearTimeout(loadingTimeoutId);
  }
  loadingTimeoutId = setTimeout(() => {
    console.warn('[MessageQueryTool] Loading timeout, forcing stop');
    if (loading.value) {
      stopQuery();
      error.value = t.value.messages.queryTimeout || '查询超时，请减少查询数量或缩短时间范围';
    }
  }, 90000);

  try {
    const params: any = {
      max_messages: maxMessages.value,
      fetchMode: fetchMode.value,
      sort: fetchMode.value === 'newest' ? 'desc' : 'asc',
    };

    if (selectedPartition.value !== 'all') {
      params.partition = selectedPartition.value;
    }

    if (searchKeyword.value.trim()) {
      params.search = searchKeyword.value.trim();
      params.search_in = 'value';
    }

    // 时间范围筛选
    if (filters.startTime) {
      const startDate = parseDateTime(filters.startTime);
      if (startDate) {
        params.start_time = startDate.getTime();
      }
    }
    if (filters.endTime) {
      const endDate = parseDateTime(filters.endTime);
      if (endDate) {
        params.end_time = endDate.getTime();
      }
    }

    // 使用 SSE 流式获取
    currentAbortController = apiClient.getMessagesStream(
      selectedCluster.value,
      selectedTopic.value,
      params,
      {
        onStart: (data) => {
          console.log('[MessageQueryTool] onStart called, isAborted:', isAborted, 'requestId:', requestId, 'currentRequestId:', currentRequestId, 'isStreaming:', streamingProgress.value.isStreaming);
          if (requestId !== currentRequestId || isAborted) {
            console.log('[MessageQueryTool] onStart skipped due to abort');
            return;
          }
          // 防止重复设置 isStreaming = true
          if (streamingProgress.value.isStreaming) {
            console.log('[MessageQueryTool] onStart skipped, already streaming');
            return;
          }
          streamingProgress.value = { received: 0, total: data.total_target, isStreaming: true };
          console.log('[MessageQueryTool] onStart set isStreaming = true');
        },
        onBatch: (newMessages, progress, total) => {
          if (requestId !== currentRequestId || isAborted) return;
          // 更新进度
          streamingProgress.value.received = progress;
          streamingProgress.value.total = total;
          lastQueryTime.value = Math.round(performance.now() - startTime);
          // 追加到非响应式缓存数组 - 使用紧凑格式减少内存
          for (const msg of newMessages) {
            pendingMessages.push({
              p: msg.partition,
              o: msg.offset,
              k: msg.key || '',
              v: msg.value || '',
              ts: msg.timestamp || null,
              uid: `${msg.partition}-${msg.offset}`,
            });
          }
          // 批量调度 UI 更新
          scheduleUpdate();
        },
        onOrder: (sort) => {
          if (requestId !== currentRequestId || isAborted) return;
          finalizedSort = sort === 'desc' ? 'desc' : undefined;
        },
        onComplete: (data) => {
          if (requestId !== currentRequestId || isAborted) {
            console.log('[MessageQueryTool] onComplete skipped, requestId:', requestId, 'currentRequestId:', currentRequestId, 'isAborted:', isAborted);
            return;
          }
          // 处理剩余未渲染的消息
          if (updateTimer) {
            clearTimeout(updateTimer);
            updateTimer = null;
          }
          if (pendingMessages.length > 0) {
            // 合并剩余消息，不是覆盖
            messages.value = [...messages.value, ...pendingMessages];
            pendingMessages = [];
          }
          // 如果是降序，反转数组
          if (finalizedSort === 'desc') {
            messages.value = [...messages.value].reverse();
          }
          // 再次检查取消状态，防止在执行过程中被取消
          if (isAborted) {
            console.log('[MessageQueryTool] onComplete skipped after processing, isAborted:', isAborted);
            return;
          }
          loading.value = false;
          streamingProgress.value.isStreaming = false;
          // 更新实际总数（如果与 target 不同）
          if (data?.actual_total && data.actual_total !== streamingProgress.value.total) {
            streamingProgress.value.total = data.actual_total;
          }
          // 清除 currentAbortController
          currentAbortController = null;
          lastQueryTime.value = Math.round(performance.now() - startTime);
          // 清除加载超时定时器
          if (loadingTimeoutId) {
            clearTimeout(loadingTimeoutId);
            loadingTimeoutId = null;
          }
          console.log(`[SSE] Complete: total=${messages.value.length}, actual_total from backend=${data?.actual_total}`);
        },
        onError: (err) => {
          if (requestId !== currentRequestId || isAborted) return;
          error.value = err;
          // 再次检查取消状态
          if (isAborted) return;
          loading.value = false;
          streamingProgress.value.isStreaming = false;
          // 不清除 currentAbortController，让 stopQuery 处理
          // 清除加载超时定时器
          if (loadingTimeoutId) {
            clearTimeout(loadingTimeoutId);
            loadingTimeoutId = null;
          }
        }
      }
    );
  } catch (e: any) {
    // 只有在未被取消的情况下才更新状态（stopQuery 已经更新过状态）
    if (!isAborted) {
      console.error('Query failed:', e);
      error.value = e.message || t.value.messages.queryFailed;
      loading.value = false;
      streamingProgress.value.isStreaming = false;
    }
    // 用户取消时，stopQuery 已经更新了状态，不需要重复更新
  }
}

function stopQuery() {
  console.log('[MessageQueryTool] stopQuery called, isAborted:', isAborted, 'currentAbortController:', currentAbortController);
  // 取消 SSE 请求
  if (currentAbortController) {
    currentAbortController.abort();
    currentAbortController = null;
  }
  // 增加请求序列号，使之后的回调失效
  currentRequestId++;
  loading.value = false;
  streamingProgress.value.isStreaming = false;
  console.log('[MessageQueryTool] stopQuery set isStreaming = false, currentRequestId:', currentRequestId);
  // 清除加载超时定时器
  if (loadingTimeoutId) {
    clearTimeout(loadingTimeoutId);
    loadingTimeoutId = null;
  }
  // 清除待处理的消息缓存
  pendingMessages = [];
  // 清除定时器
  if (updateTimer) {
    clearTimeout(updateTimer);
    updateTimer = null;
  }
  // 重置排序状态
  finalizedSort = undefined;
  // 重置流式进度
  streamingProgress.value = { received: 0, total: 0, isStreaming: false };
  // 设置取消标志（放在最后，阻止后续回调）
  isAborted = true;
}

// 检测是否在 Tauri 环境下运行
function isTauri(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }
  const win = window as any;
  return !!(
    win.__TAURI__ ||
    win.__TAURI_INTERNALS__ ||
    win.__TAURI_IPC__ ||
    win._TAURI_VERSION_ ||
    win.navigator?.userAgent?.includes('Tauri')
  );
}

function exportMessages() {
  if (messages.value.length === 0) return;

  // 将缩写字段名映射为完整字段名
  const exportData = messages.value.map(m => ({
    partition: m.p,
    offset: m.o,
    key: m.k,
    value: m.v,
    timestamp: m.ts,
    uid: m.uid,
  }));

  const data = JSON.stringify(exportData, null, 2);
  const filename = `${selectedTopic.value}_messages_${Date.now()}.json`;

  if (isTauri()) {
    // Tauri 环境下使用原生文件保存对话框
    save({
      defaultPath: filename,
      filters: [{
        name: 'JSON Files',
        extensions: ['json']
      }]
    }).then(async (filePath) => {
      if (filePath) {
        const encoder = new TextEncoder();
        const bytes = encoder.encode(data);
        await writeFile(filePath, bytes);
        showSuccess(t.value.messages.exportSuccess);
      }
    }).catch((err) => {
      console.error('Failed to save file:', err);
      showError(t.value.messages.exportFailed);
    });
  } else {
    // 浏览器环境下使用传统下载方式
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }
}

// 发送消息弹框控制
function openSendModal() {
  // 如果有选中的消息，自动填充 partition、key、value
  if (selectedMessage.value) {
    messageFormPartition.value = selectedMessage.value.p ?? 0;
    messageFormKey.value = selectedMessage.value.k ?? '';
    messageFormValue.value = selectedMessage.value.v ?? '';
  } else {
    // 如果没有选中分区，默认选第一个
    if (partitions.value.length > 0 && messageFormPartition.value === 0) {
      messageFormPartition.value = partitions.value[0] ?? 0;
    }
  }
  showSendModal.value = true;
}

// 处理发送消息提交
async function handleSubmit(data: { partition: number; key: string | undefined; value: string; headers: { key: string; value: string }[] }, keepOpen: boolean) {
  if (!selectedCluster.value || !selectedTopic.value) return;

  // 转换 headers 从数组到对象
  const headersObj: Record<string, string> = {};
  data.headers.forEach(h => {
    if (h.key && h.value) {
      headersObj[h.key] = h.value;
    }
  });

  try {
    const result = await apiClient.sendMessage(selectedCluster.value, selectedTopic.value, {
      partition: data.partition,
      key: data.key,
      value: data.value,
      headers: Object.keys(headersObj).length > 0 ? headersObj : undefined,
    });

    // 设置 offset 到弹窗
    sendModalRef.value?.setLastOffset(result.offset);

    showSuccess(`${t.value.messages.messageSent}! Offset: ${result.offset}`);

    // 只在不需要保持打开时关闭弹框
    if (!keepOpen) {
      showSendModal.value = false;
    }
  } catch (e) {
    console.error('Failed to send message:', e);
    showError(t.value.toast.operationFailed || 'Failed to send message');
  }
}

function formatTime(ts: number | null): string {
  if (!ts) return '-';
  const date = new Date(ts);
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function truncate(str: string | null, len: number): string {
  if (!str) return '';
  return str.length > len ? str.slice(0, len) + '...' : str;
}

// 辅助函数：获取消息属性
function getMsgPartition(item: any): number { return item?.p ?? 0; }
function getMsgOffset(item: any): number { return item?.o ?? 0; }
function getMsgKey(item: any): string | null { return item?.k; }
function getMsgValue(item: any): string | null { return item?.v; }
function getMsgTimestamp(item: any): number | null { return item?.ts; }

// 键盘导航：处理方向键
function handleKeyNavigation(e: KeyboardEvent) {
  // 只在有消息列表且详情面板未打开时响应
  if (sortedMessages.value.length === 0) return;

  // 如果详情面板打开且焦点在面板内，不处理方向键（让用户可以编辑/选择文本）
  const activeElement = document.activeElement;
  const isDetailPanelFocused = selectedMessage.value && (
    keyPreRef.value?.contains(activeElement) ||
    valuePreRef.value?.contains(activeElement)
  );
  if (isDetailPanelFocused) return;

  if (e.key === 'ArrowDown') {
    e.preventDefault();
    navigateMessage('down');
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    navigateMessage('up');
  }
}

// 导航到上一条或下一条消息
function navigateMessage(direction: NavigationDirection) {
  const currentIndex = selectedMessage.value
    ? sortedMessages.value.findIndex((msg) => {
        return msg.p === selectedMessage.value?.p && msg.o === selectedMessage.value?.o;
      })
    : -1;

  let newIndex: number;
  if (direction === 'down') {
    newIndex = currentIndex < sortedMessages.value.length - 1 ? currentIndex + 1 : 0; // 循环到第一条
  } else {
    newIndex = currentIndex > 0 ? currentIndex - 1 : sortedMessages.value.length - 1; // 循环到最后一条
  }

  const targetMessage = sortedMessages.value[newIndex];
  if (targetMessage) {
    selectedMessage.value = targetMessage;
    scrollToSelectedMessage();
  }
}

// 滚动虚拟列表使选中消息可见
function scrollToSelectedMessage() {
  if (!selectedMessage.value) return;

  const scroller = scrollerRef.value;
  if (!scroller) return;

  const uid = `${selectedMessage.value.p}-${selectedMessage.value.o}`;
  const index = sortedMessages.value.findIndex((m) => m.uid === uid);
  if (index >= 0) {
    // 使用虚拟滚动的 scrollToItem 方法
    scroller.scrollToItem(index);
  }
}

// 清除时间筛选
function clearTimeFilters() {
  filters.startTime = '';
  filters.endTime = '';
}

// 格式化日期时间输入（确保秒级精度）
// 支持多种输入格式：YYYY-MM-DD HH:mm:ss, YYYY/MM/DD HH:mm:ss, YYYY-MM-DDTHH:mm:ss 等
function formatDateTime(field: 'startTime' | 'endTime') {
  const input = filters[field];
  if (!input) return;

  const date = parseDateTime(input);
  if (date) {
    const pad = (n: number) => String(n).padStart(2, '0');
    filters[field] = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  }
}

// 解析日期时间字符串，支持多种格式
function parseDateTime(input: string): Date | null {
  // 尝试直接解析
  const date = new Date(input);
  if (!isNaN(date.getTime())) {
    return date;
  }

  // 尝试替换分隔符后解析 (YYYY-MM-DD HH:mm:ss -> YYYY-MM-DDTHH:mm:ss)
  const normalized = input.replace(' ', 'T').replace('/', '-g');
  const date2 = new Date(normalized);
  if (!isNaN(date2.getTime())) {
    return date2;
  }

  return null;
}

// 设置预设时间范围
function setPresetTime(minutes: number) {
  const now = new Date();
  const start = new Date(now.getTime() - minutes * 60 * 1000);

  // 格式化为文本输入框格式 (YYYY-MM-DD HH:mm:ss)
  const toLocalString = (date: Date) => {
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  };

  filters.endTime = toLocalString(now);
  filters.startTime = toLocalString(start);
  showTimeFilters.value = true;
}

// 处理键盘 Ctrl+A 事件，选中 Key 或 Value 内容
function handleSelectAll() {
  if (!selectedMessage.value) return;

  // 根据当前焦点位置决定选中 Key 还是 Value
  let preElement: HTMLElement | null | undefined = null;

  // 检查当前焦点是否在 Key 或 Value 区域
  const activeElement = document.activeElement;
  const isKeyFocused = keyPreRef.value && (activeElement === keyPreRef.value || keyPreRef.value.contains(activeElement));
  const isValueFocused = valuePreRef.value && (activeElement === valuePreRef.value || valuePreRef.value.contains(activeElement));

  // 如果 Key 区域有焦点且有 Key 内容，选中 Key
  if (isKeyFocused && selectedMessage.value.key) {
    preElement = keyPreRef.value;
  }
  // 如果 Value 区域有焦点，选中 Value
  else if (isValueFocused) {
    preElement = valuePreRef.value;
  }
  // 没有焦点时，默认选中 Value 内容
  else if (valuePreRef.value) {
    preElement = valuePreRef.value;
  }
  // 降级方案：如果没有 Value 但有 Key，选中 Key
  else if (keyPreRef.value && selectedMessage.value.key) {
    preElement = keyPreRef.value;
  }

  if (preElement) {
    const range = document.createRange();
    range.selectNodeContents(preElement);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
  }
}

// 格式化值为不同格式
function formatValue(value: string | null, format: 'json' | 'raw' | 'hex'): string {
  if (!value) return 'null';

  if (format === 'json') {
    return formatJson(value);
  } else if (format === 'hex') {
    try {
      const bytes = new TextEncoder().encode(value);
      return Array.from(bytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ');
    } catch {
      return value;
    }
  }
  return value;
}

// JSON 语法高亮 - 使用模板高亮
function highlightJson(json: string): string {
  // 读取 jsonHighlightRefresh 以建立依赖关系，当模板变化时强制重新计算
  void jsonHighlightRefresh.value;
  // 不传递 isDark，让函数直接从 DOM 获取当前主题
  return highlightJsonWithTemplate(json);
}

// Detail panel Ctrl+F search
function handleDetailSearch() {
  detailSearchActive.value = true;
  detailSearchQuery.value = '';
  detailMatchIndex.value = 0;
  detailMatchCount.value = 0;
  // Wait for DOM update then focus input
  setTimeout(() => {
    detailSearchInputRef.value?.focus();
  }, 50);
}

function closeDetailSearch() {
  detailSearchActive.value = false;
  detailSearchQuery.value = '';
  // Remove all highlights
  if (detailContentRef.value) {
    removeSearchHighlights(detailContentRef.value);
  }
}

function removeSearchHighlights(container: HTMLElement) {
  // Restore original text by unwrapping all <mark> elements
  const marks = container.querySelectorAll('mark.search-highlight');
  marks.forEach((mark) => {
    const text = document.createTextNode(mark.textContent || '');
    mark.parentNode?.replaceChild(text, mark);
  });
  // Also remove any selection class
  const selected = container.querySelectorAll('.search-match-active');
  selected.forEach((el) => el.classList.remove('search-match-active'));
}

function highlightTextInElement(container: HTMLElement, query: string) {
  removeSearchHighlights(container);
  if (!query) {
    detailMatchCount.value = 0;
    detailMatchIndex.value = 0;
    return;
  }

  const lowerQuery = query.toLowerCase();
  // Only search within leaf text nodes that are not inside <button>, <input>, <select>
  const textNodes: Text[] = [];
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => {
      const parent = node.parentElement;
      if (!parent) return NodeFilter.FILTER_REJECT;
      // Skip buttons, inputs, and already-highlighted content
      if (parent.closest('button, input, select, textarea')) return NodeFilter.FILTER_REJECT;
      if (parent.classList.contains('search-highlight-processed')) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    }
  });

  while (walker.nextNode()) {
    textNodes.push(walker.currentNode as Text);
  }

  let matchIndex = 0;
  let totalMatches = 0;
  const ranges: Range[] = [];

  textNodes.forEach((node) => {
    const text = node.textContent || '';
    const lowerText = text.toLowerCase();
    let startIndex = 0;
    while (true) {
      const pos = lowerText.indexOf(lowerQuery, startIndex);
      if (pos === -1) break;
      const range = document.createRange();
      range.setStart(node, pos);
      range.setEnd(node, pos + query.length);
      ranges.push(range);
      totalMatches++;
      startIndex = pos + 1;
    }
  });

  // Wrap matches in <mark> elements (reverse order to not break positions)
  for (let i = ranges.length - 1; i >= 0; i--) {
    const r = ranges[i];
    if (!r) continue;
    const mark = document.createElement('mark');
    mark.className = 'search-highlight search-highlight-processed';
    mark.style.cssText = 'background:#fde68a;color:inherit;border-radius:1px;padding:0;';
    try {
      r.surroundContents(mark);
    } catch {
      // If range spans multiple nodes, skip
    }
  }

  detailMatchCount.value = totalMatches;
  if (totalMatches > 0) {
    detailMatchIndex.value = 1;
    highlightActiveMatch(container, 0);
  }
}

function highlightActiveMatch(container: HTMLElement, index: number) {
  // Remove previous active highlight
  const prev = container.querySelector('.search-match-active');
  prev?.classList.remove('search-match-active');

  const marks = container.querySelectorAll('mark.search-highlight');
  if (marks.length === 0) return;

  const mark = marks[index % marks.length] as HTMLElement;
  mark.style.background = '#f59e0b';
  mark.style.color = '#fff';
  mark.classList.add('search-match-active');
  mark.scrollIntoView({ block: 'nearest', inline: 'nearest' });
}

function updateDetailSearch() {
  if (!detailContentRef.value) return;
  if (!detailSearchQuery.value) {
    removeSearchHighlights(detailContentRef.value);
    detailMatchCount.value = 0;
    detailMatchIndex.value = 0;
    return;
  }
  highlightTextInElement(detailContentRef.value, detailSearchQuery.value);
}

function detailSearchNext() {
  if (!detailContentRef.value || detailMatchCount.value === 0) return;
  detailMatchIndex.value = (detailMatchIndex.value % detailMatchCount.value) + 1;
  highlightActiveMatch(detailContentRef.value, detailMatchIndex.value - 1);
}

function detailSearchPrev() {
  if (!detailContentRef.value || detailMatchCount.value === 0) return;
  detailMatchIndex.value = detailMatchIndex.value <= 1 ? detailMatchCount.value : detailMatchIndex.value - 1;
  highlightActiveMatch(detailContentRef.value, detailMatchIndex.value - 1);
}

// 监听模板变化事件
function handleTemplateChange(_event: CustomEvent) {
  clearTemplateCache();
  // 增加触发器值，强制 computed 重新计算
  jsonHighlightRefresh.value++;
}

function copyKey() {
  if (!selectedMessage.value?.k) return;
  navigator.clipboard.writeText(selectedMessage.value.k).then(() => {
    showSuccess(`${t.value.messages.key} ${t.value.messages.copied}`, 2000);
  });
}

function copyValue() {
  if (!selectedMessage.value?.v) return;
  const text = formatValue(selectedMessage.value.v, valueViewFormat.value);
  navigator.clipboard.writeText(text).then(() => {
    showSuccess(`${t.value.messages.value} ${t.value.messages.copied}`, 2000);
  });
}

function copyMessageValue(msg: any) {
  if (!msg?.v) return;
  // 默认按 JSON 格式化复制
  const text = formatValue(msg.v, 'json');
  navigator.clipboard.writeText(text).then(() => {
    showSuccess(`${t.value.messages.value} ${t.value.messages.copied}`, 2000);
  });
}

onMounted(async () => {
  // 加载设置（包括 max_messages）
  await loadSettings();

  // 优先使用 props 传入的 cluster 和 topic
  if (props.cluster) {
    selectedCluster.value = props.cluster;
  }
  if (props.topic) {
    selectedTopic.value = props.topic;
  }

  // 如果 props 没有，从 URL 参数获取
  if (!selectedCluster.value || !selectedTopic.value) {
    const { cluster, topic, partition } = route.query;
    if (cluster && typeof cluster === 'string') {
      selectedCluster.value = cluster;
    }
    if (topic && typeof topic === 'string') {
      selectedTopic.value = topic;
    }
    if (partition && typeof partition === 'string') {
      const partitionNum = parseInt(partition, 10);
      if (!isNaN(partitionNum)) {
        selectedPartition.value = partitionNum;
      }
    }
  }

  // 加载分区信息并自动查询
  if (selectedCluster.value && selectedTopic.value) {
    await loadPartitions();
    // 记录浏览历史
    recordHistory();
    await queryMessages();
  }

  // 添加键盘事件监听
  document.addEventListener('keydown', handleKeyNavigation);

  // 监听模板变化事件
  window.addEventListener('json-highlight-changed', handleTemplateChange as EventListener);
});

// 监听 props 变化
watch(() => props.cluster, async (newCluster, oldCluster) => {
  if (newCluster && newCluster !== selectedCluster.value) {
    // 如果当前处于取消状态，不执行查询
    if (isAborted) {
      console.log('[Watch] cluster: skipped due to isAborted');
      return;
    }
    stopQuery();
    selectedCluster.value = newCluster;
    await loadPartitions();
    if (selectedCluster.value && selectedTopic.value) {
      // 记录浏览历史
      recordHistory();
      await queryMessages();
    }
  }
});

watch(() => props.topic, async (newTopic, oldTopic) => {
  if (newTopic && newTopic !== selectedTopic.value) {
    // 如果当前处于取消状态，不执行查询
    if (isAborted) {
      console.log('[Watch] topic: skipped due to isAborted');
      return;
    }
    stopQuery();
    selectedTopic.value = newTopic;
    selectedPartition.value = 'all';
    partitions.value = [];
    messages.value = [];
    await loadPartitions();
    if (selectedCluster.value && selectedTopic.value) {
      // 记录浏览历史
      recordHistory();
      await queryMessages();
    }
  }
});

// 监听路由参数变化（从左侧菜单树点击分区）
watch(() => route.query.partition, async (partition, oldPartition) => {
  // 如果当前处于取消状态，不处理分区变化
  if (isAborted) {
    console.log('[Watch] partition: skipped due to isAborted');
    return;
  }
  if (partition && typeof partition === 'string') {
    const partitionNum = parseInt(partition, 10);
    if (!isNaN(partitionNum)) {
      selectedPartition.value = partitionNum;
      // 重新查询消息
      if (selectedCluster.value && selectedTopic.value) {
        await queryMessages();
      }
    }
  } else {
    // 没有分区参数，重置为全部
    selectedPartition.value = 'all';
  }
});

onUnmounted(() => {
  if (loading.value) {
    stopQuery();
  }
  stopResize();
  stopColumnResize();
  closeDetailSearch();
  // 移除键盘事件监听
  document.removeEventListener('keydown', handleKeyNavigation);
  // 移除模板变化事件监听
  window.removeEventListener('json-highlight-changed', handleTemplateChange as EventListener);
});

// Close detail search when selected message changes
watch(selectedMessage, () => {
  closeDetailSearch();
});

// 加载设置（从数据库）
async function loadSettings() {
  try {
    const settings = await apiClient.getSettings(['messages.max_messages']);
    for (const setting of settings) {
      if (setting.key === 'messages.max_messages') {
        const savedMax = parseInt(setting.value, 10);
        if (!isNaN(savedMax) && savedMax >= 1 && savedMax <= 10000) {
          maxMessages.value = savedMax;
        }
      }
    }
  } catch (e) {
    // 静默失败 - 使用默认值
    console.debug('Settings load failed (using defaults):', (e as { message?: string }).message);
  }
}

// 保存 max_messages 设置到数据库
async function saveMaxMessagesSetting() {
  try {
    await apiClient.updateSetting('messages.max_messages', maxMessages.value.toString());
  } catch (e) {
    console.error('Failed to save max_messages setting:', e);
  }
}

// 监听 max_messages 变化，自动保存
watch(() => maxMessages.value, () => {
  saveMaxMessagesSetting();
});

// 拖动调整高度
let isResizing = false;
let startY = 0;
let startHeight = 0;

function startResize(e: MouseEvent) {
  isResizing = true;
  startY = e.clientY;
  startHeight = panelHeight.value;
  document.addEventListener('mousemove', onResize);
  document.addEventListener('mouseup', stopResize);
  e.preventDefault();
}

function onResize(e: MouseEvent) {
  if (!isResizing) return;
  const delta = startY - e.clientY; // 向上拖动增加高度
  const newHeight = startHeight + delta;
  panelHeight.value = Math.max(150, Math.min(600, newHeight)); // 限制高度范围 150-600px
}

function stopResize() {
  isResizing = false;
  document.removeEventListener('mousemove', onResize);
  document.removeEventListener('mouseup', stopResize);
}
</script>

<style scoped>
.message-query-tool {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  flex-shrink: 0;
}

.status-bar {
  flex-shrink: 0;
}

.detail-panel {
  flex-shrink: 0;
}

.resize-handle {
  user-select: none;
}

.resizer {
  position: relative;
  flex-shrink: 0;
  align-self: stretch;
}

.resizer::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 4px;
  transform: translateX(-50%);
}

pre {
  white-space: pre-wrap;
  word-break: break-all;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fadeIn {
  animation: fadeIn 0.2s ease-out;
}
.vue-recycle-scroller {
  position: relative;
}

.vue-recycle-scroller__item-wrapper {
  flex: 1;
}

.vue-recycle-scroller__item-view {
  position: absolute;
  top: 0;
  left: 0;
}
</style>

<style>
/* JSON 语法高亮样式已移至全局样式文件 */
</style>
