<template>
  <div class="message-query-tool h-full flex flex-col">
    <!-- 简洁搜索栏 -->
    <div class="toolbar flex flex-wrap items-center gap-1.5 p-1.5 border-b border-base-300 bg-base-100">
      <!-- 分区选择 -->
      <select v-model="selectedPartition" class="select select-bordered select-sm w-28">
        <option value="all">{{ t.messages.allPartitions }}</option>
        <option v-for="p in partitions" :key="p" :value="p">{{ t.messages.partition }} {{ p }}</option>
      </select>

      <!-- 查询模式 -->
      <select v-model="fetchMode" class="select select-bordered select-sm w-24">
        <option value="newest">{{ t.messages.newest }}</option>
        <option value="oldest">{{ t.messages.oldest }}</option>
      </select>

      <!-- 数量 -->
      <input v-model.number="maxMessages" type="number" class="input input-bordered input-sm w-16" min="1" max="1000" :title="t.messages.maxMessages" />

      <!-- 搜索 -->
      <div class="flex-1 min-w-[120px] relative">
        <input v-model="searchKeyword" type="text" class="input input-bordered input-sm w-full pr-8" :placeholder="t.messages.valuePlaceholder" @keyup.enter="queryMessages" />
        <button v-if="searchKeyword" class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content" @click="searchKeyword = ''; queryMessages()">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- 查询按钮 -->
      <button class="btn btn-primary btn-sm" :class="{ 'loading': loading }" :disabled="!canQuery || loading" @click="queryMessages">
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
      <button class="btn btn-ghost btn-sm" @click="openSendModal" :title="t.messages.sendMessage">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
        </svg>
      </button>
    </div>

    <!-- 状态栏 -->
    <div class="status-bar flex items-center justify-between px-3 py-1 text-xs border-b border-base-300 bg-base-200/50">
      <div class="flex items-center gap-4">
        <span v-if="selectedTopic" class="text-base-content/70 flex items-center gap-1">
          Topic: <span class="font-mono font-bold text-primary">{{ selectedTopic }}</span>
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
          <span>接收中 {{ streamingProgress.received.toLocaleString() }}</span>
          <span v-if="streamingProgress.total > 0">/ {{ streamingProgress.total.toLocaleString() }}</span>
        </span>
        <span v-else-if="lastQueryTime > 0" class="text-base-content/70">
          {{ t.messages.elapsedTime }}: <span class="font-mono font-bold">{{ lastQueryTime }}ms</span>
        </span>
        <span v-if="messages.length > 0" class="text-base-content/70">
          {{ t.messages.totalMessages }} <span class="font-mono font-bold text-success">{{ messages.length.toLocaleString() }}</span> {{ t.messages.messages }}
          <button class="btn btn-ghost btn-xs ml-2" :disabled="messages.length === 0" @click="exportMessages" :title="t.messages.exportMessages">
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
    <div class="flex-1 overflow-hidden bg-base-100">
      <!-- Desktop Table with Virtual Scroll -->
      <div class="hidden md:flex md:flex-col h-full">
        <!-- Table Header -->
        <div class="flex bg-base-200 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide w-full">
          <div class="w-12 flex-shrink-0">{{ t.messages.partitionLabel2 }}</div>
          <div class="w-16 flex-shrink-0">{{ t.messages.offsetLabel }}</div>
          <div class="w-28 flex-shrink-0">{{ t.messages.timestampLabel }}</div>
          <div class="w-20 flex-shrink-0">{{ t.messages.key }}</div>
          <div class="flex-1">{{ t.messages.value }}</div>
          <div class="w-10 flex-shrink-0 text-center">{{ t.messages.actions }}</div>
        </div>
        <!-- Virtual Scroll List -->
        <RecycleScroller
          v-if="messages.length > 0"
          ref="scrollerRef"
          class="flex-1 overflow-auto w-full"
          :items="messages"
          :item-size="24"
          key-field="uid"
          :buffer="200"
          v-slot="{ item }"
        >
          <div
            class="flex items-center px-2 py-0.5 hover:bg-base-200/50 transition-colors border-b border-base-200/30 cursor-pointer w-full"
            :class="{ 'bg-primary/20': selectedMessage?.partition === getMsgPartition(item) && selectedMessage?.offset === getMsgOffset(item) }"
            style="height: 24px;"
            @click="selectedMessage = (item as any)"
          >
            <div class="w-12 flex-shrink-0 text-[10px]">
              <span class="badge badge-ghost badge-xs scale-90">{{ getMsgPartition(item) }}</span>
            </div>
            <div class="w-16 flex-shrink-0 text-[10px] font-mono">{{ getMsgOffset(item) }}</div>
            <div class="w-28 flex-shrink-0 text-[10px] text-base-content/60 whitespace-nowrap">{{ formatTime(getMsgTimestamp(item)) }}</div>
            <div class="w-20 flex-shrink-0 text-[10px] font-mono truncate">{{ getMsgKey(item) || '-' }}</div>
            <div class="flex-1 text-[10px] font-mono truncate pr-2" style="min-width: 0;">{{ getMsgValue(item) }}</div>
            <div class="w-10 flex-shrink-0 text-[10px] flex items-center justify-center">
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click.stop="copyMessageValue(item as any)" :title="t.messages.copyValue">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
          </div>
        </RecycleScroller>
        <div v-if="messages.length === 0 && !loading" class="flex-1 flex flex-col items-center justify-center text-base-content/50">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50 mb-2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
          </svg>
          <span>{{ t.messages.noMessages }}</span>
        </div>
      </div>

      <!-- Mobile Card View with Virtual Scroll -->
      <div class="md:hidden flex flex-col h-full">
        <RecycleScroller
          v-if="messages.length > 0"
          ref="scrollerRefMobile"
          class="flex-1 overflow-auto p-2 pb-20"
          :items="messages"
          :item-size="70"
          key-field="uid"
          :buffer="100"
          v-slot="{ item }"
        >
          <div
            class="card bg-base-100 border border-base-200 p-2 shadow-sm mb-2 cursor-pointer"
            :class="{ 'bg-primary/20 border-primary/50': selectedMessage?.partition === getMsgPartition(item) && selectedMessage?.offset === getMsgOffset(item) }"
            style="height: 62px;"
            @click="selectedMessage = (item as any)"
          >
            <div class="flex items-center justify-between mb-1">
              <div class="flex items-center gap-2">
                <span class="badge badge-ghost badge-xs">P{{ getMsgPartition(item) }}</span>
                <span class="text-xs font-mono text-base-content/70">#{{ getMsgOffset(item) }}</span>
              </div>
              <span class="text-xs text-base-content/50">{{ formatTime(getMsgTimestamp(item)) }}</span>
            </div>
            <div v-if="getMsgKey(item)" class="text-xs font-mono text-secondary mb-1 truncate">
              Key: {{ getMsgKey(item) }}
            </div>
            <div class="text-xs font-mono truncate text-base-content/80">
              {{ truncate(getMsgValue(item), 100) }}
            </div>
          </div>
        </RecycleScroller>
        <div v-else-if="messages.length === 0 && !loading" class="flex-1 flex flex-col items-center justify-center text-base-content/50 p-2">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-8 h-8 opacity-50 mb-2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694-4.125-8.25-4.125s-8.25-1.847-8.25-4.125" />
          </svg>
          <span>{{ t.messages.noMessages }}</span>
        </div>
      </div>
    </div>

    <!-- Detail Panel (Sticky at bottom with resize handle) -->
    <div v-if="selectedMessage" class="detail-panel border-t border-base-300 bg-base-200/30 flex flex-col" :style="{ height: panelHeight + 'px' }">
      <!-- Resize Handle -->
      <div class="resize-handle h-1 cursor-row-resize bg-base-300 hover:bg-primary/50 transition-colors flex items-center justify-center" @mousedown="startResize">
        <div class="w-8 h-0.5 bg-base-content/20 rounded-full"></div>
      </div>
      <div class="flex-1 overflow-hidden p-2">
        <div class="flex items-center justify-between mb-1.5 pb-1 border-b border-base-content/10">
          <div class="flex items-center gap-2 flex-wrap">
            <h4 class="text-xs font-bold">{{ t.messages.messageDetail }}</h4>
            <span class="text-[10px] text-base-content/50">{{ t.messages.partition }}: <span class="font-mono">{{ selectedMessage.partition }}</span></span>
            <span class="text-[10px] text-base-content/50">{{ t.messages.offset }}: <span class="font-mono">{{ selectedMessage.offset }}</span></span>
            <span class="text-[10px] text-base-content/50">{{ formatTime(selectedMessage.timestamp) }}</span>
          </div>
          <div class="flex items-center gap-1">
            <button class="btn btn-ghost btn-xs px-1" @click="selectedMessage = null">{{ t.messages.close }}</button>
          </div>
        </div>
        <div class="space-y-1.5 text-[10px] h-[calc(100%-32px)] overflow-auto flex flex-col">
          <div v-if="selectedMessage.key" class="mb-1">
            <div class="flex items-center justify-between mb-0.5">
              <div class="text-base-content/50 text-[10px] font-semibold">{{ t.messages.key }}:</div>
              <button class="btn btn-ghost btn-xs px-1 min-h-[18px] h-[18px]" @click="copyKey" :title="t.messages.copyKey">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
            </div>
            <div class="bg-base-100 p-1 rounded text-[10px] font-mono border border-base-content/5 truncate">{{ selectedMessage.key }}</div>
          </div>
          <div class="flex flex-col flex-1">
            <div class="flex items-center justify-between mb-0.5">
              <div class="text-base-content/50 text-[10px] font-semibold flex items-center gap-1">
                {{ t.messages.value }}:
                <select v-model="valueViewFormat" class="select select-bordered select-xs scale-90 origin-left">
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
              class="bg-base-100 p-1.5 rounded text-[10px] font-mono overflow-auto whitespace-pre-wrap border border-base-content/5 flex-1 json-highlight"
              v-html="highlightJson(formatValue(selectedMessage.value, valueViewFormat))"
            ></pre>
            <pre
              v-else
              class="bg-base-100 p-1.5 rounded text-[10px] font-mono overflow-auto whitespace-pre-wrap border border-base-content/5 flex-1"
            >{{ formatValue(selectedMessage.value, valueViewFormat) }}</pre>
          </div>
        </div>
      </div>
    </div>

    <!-- Send Message Modal -->
    <Teleport to="body">
      <dialog ref="sendModalRef" class="modal" @click.self="closeSendModal">
        <div class="modal-box w-full max-w-lg mx-2 md:mx-auto">
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="closeSendModal">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
          <h3 class="font-bold text-lg flex items-center gap-2 mb-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-info">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 12 3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" />
            </svg>
            {{ t.messages.sendMessage }} <span class="font-mono text-sm truncate max-w-[150px] md:max-w-xs">{{ selectedTopic }}</span>
          </h3>
          <form @submit.prevent="() => handleSendMessage(false)" class="flex flex-col gap-3">
            <!-- Partition Dropdown -->
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ t.messages.partition }}</span>
              </label>
              <select v-model.number="messageForm.partition" class="select select-bordered w-full sm:w-32" required :disabled="partitions.length === 0">
                <option v-for="p in partitions" :key="p" :value="p">{{ p }}</option>
              </select>
            </div>
            <!-- Key Input -->
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ t.messages.key }}</span>
                <span class="label-text-alt">{{ t.messages.keyOptional }}</span>
              </label>
              <input v-model="messageForm.key" type="text" class="input input-bordered w-full" :placeholder="t.messages.keyOptional" />
            </div>
            <!-- Value Textarea -->
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ t.messages.value }}</span>
                <span class="label-text-alt">{{ t.messages.valueRequired }}</span>
              </label>
              <textarea v-model="messageForm.value" class="textarea textarea-bordered h-24 sm:h-32 font-mono text-sm w-full" required placeholder='{"id": 1, "data": "example"}'></textarea>
            </div>
            <!-- Headers Toggle -->
            <div>
              <button type="button" class="btn btn-ghost btn-sm" @click="showHeaders = !showHeaders">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
                </svg>
                Headers
                <span v-if="messageForm.headers.length > 0" class="badge badge-primary badge-xs">{{ messageForm.headers.length }}</span>
              </button>
            </div>
            <!-- Headers Input -->
            <div v-if="showHeaders" class="border border-base-300 rounded-lg p-3 space-y-2">
              <div v-for="(header, index) in messageForm.headers" :key="index" class="flex gap-2">
                <input
                  v-model="header.key"
                  type="text"
                  class="input input-bordered input-sm flex-1"
                  placeholder="Header key"
                />
                <input
                  v-model="header.value"
                  type="text"
                  class="input input-bordered input-sm flex-1"
                  placeholder="Header value"
                />
                <button type="button" class="btn btn-ghost btn-sm" @click="messageForm.headers.splice(index, 1)">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <button type="button" class="btn btn-ghost btn-sm w-full" @click="messageForm.headers.push({ key: '', value: '' })">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                </svg>
                Add Header
              </button>
            </div>
            <!-- Success Alert -->
            <div v-if="sendSuccess" class="alert alert-success py-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
              </svg>
              <span class="text-sm">{{ t.messages.messageSent }}! {{ t.messages.offset }}: {{ lastOffset }}</span>
            </div>
            <!-- Actions -->
            <div class="modal-action flex-wrap">
              <button type="button" class="btn" @click="closeSendModal">{{ t.messages.cancel }}</button>
              <button type="button" class="btn btn-primary" @click="handleSendMessage(true)" :disabled="sending">
                {{ t.messages.continue }}
              </button>
              <button type="submit" class="btn btn-primary" :disabled="sending">
                <svg v-if="sending" class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {{ sending ? t.messages.sending : t.messages.send }}
              </button>
            </div>
          </form>
        </div>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, shallowRef } from 'vue';
import { useRoute } from 'vue-router';
import { RecycleScroller } from 'vue-virtual-scroller';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';
import { useLanguageStore } from '@/stores/language';
import FavoriteButton from '@/components/FavoriteButton.vue';

const route = useRoute();
const { showSuccess } = useToast();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

interface Message {
  partition: number;
  offset: number;
  key: string | null;
  value: string | null;
  timestamp: number | null;
  uid: string;
}

// Props - 从父组件接收 cluster 和 topic
const props = defineProps<{
  cluster?: string;
  topic?: string;
}>();

// 状态 - 使用 shallowRef 避免深度响应式带来的性能问题
const partitions = ref<number[]>([]);
const messages = shallowRef<Message[]>([]);
const selectedMessage = ref<any>(null);
const valueViewFormat = ref<'json' | 'raw' | 'hex'>('json');
const panelHeight = ref(280); // 默认高度增加到 280px

// 查询参数
const selectedCluster = ref('');
const selectedTopic = ref('');
const selectedPartition = ref<string | number>('all');
const fetchMode = ref<'newest' | 'oldest'>('newest');
const maxMessages = ref(100);
const searchKeyword = ref('');

// UI 状态
const loading = ref(false);
const error = ref('');
const lastQueryTime = ref(0);

// SSE 流式状态
const streamingProgress = ref<{ received: number; total: number; isStreaming: boolean }>({ received: 0, total: 0, isStreaming: false });
let currentAbortController: AbortController | null = null;
let currentRequestId = 0;
let finalizedSort: 'desc' | undefined = undefined;

// 重置状态
function resetMessageState() {
  messages.value = [];
  streamingProgress.value = { received: 0, total: 0, isStreaming: false };
  finalizedSort = undefined;
}

// 发送消息弹框状态
const sendModalRef = ref<HTMLDialogElement | null>(null);
const sending = ref(false);
const sendSuccess = ref(false);
const lastOffset = ref<number | null>(null);
const messageForm = ref({
  partition: 0,
  key: '',
  value: '',
  headers: [] as Array<{ key: string; value: string }>,
});
const showHeaders = ref(false);

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

async function queryMessages() {
  if (!canQuery.value) return;

  // 取消上一次的请求
  stopQuery();

  // 增加请求序列号
  currentRequestId++;
  const requestId = currentRequestId;

  loading.value = true;
  error.value = '';
  resetMessageState();
  const startTime = performance.now();

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

    // 使用 SSE 流式获取
    currentAbortController = apiClient.getMessagesStream(
      selectedCluster.value,
      selectedTopic.value,
      params,
      {
        onStart: (data) => {
          if (requestId !== currentRequestId) return;
          streamingProgress.value = { received: 0, total: data.total_target, isStreaming: true };
        },
        onBatch: (newMessages, progress, total) => {
          if (requestId !== currentRequestId) return;
          // 更新进度
          streamingProgress.value.received = progress;
          streamingProgress.value.total = total;
          lastQueryTime.value = Math.round(performance.now() - startTime);
          // 直接追加到消息数组，使用 shallowRef 需要直接修改数组
          const currentMessages = messages.value;
          for (const msg of newMessages) {
            currentMessages.push({
              partition: msg.partition,
              offset: msg.offset,
              key: msg.key || '',
              value: msg.value || '',
              timestamp: msg.timestamp || null,
              uid: `${msg.partition}-${msg.offset}`,
            });
          }
          // 触发响应式更新
          messages.value = currentMessages;
        },
        onOrder: (sort) => {
          if (requestId !== currentRequestId) return;
          finalizedSort = sort === 'desc' ? 'desc' : undefined;
        },
        onComplete: () => {
          if (requestId !== currentRequestId) return;
          // 如果是降序，反转数组
          if (finalizedSort === 'desc') {
            const reversed = [...messages.value].reverse();
            messages.value = reversed;
          }
          loading.value = false;
          streamingProgress.value.isStreaming = false;
          currentAbortController = null;
          lastQueryTime.value = Math.round(performance.now() - startTime);
          console.log(`[SSE] Complete: total=${messages.value.length}`);
        },
        onError: (err) => {
          if (requestId !== currentRequestId) return;
          error.value = err;
          loading.value = false;
          streamingProgress.value.isStreaming = false;
          currentAbortController = null;
        }
      }
    );
  } catch (e: any) {
    if (e.message === 'AbortError' || e.message?.includes('aborted')) {
      // 用户取消，不显示错误
    } else {
      console.error('Query failed:', e);
      error.value = e.message || t.value.messages.queryFailed;
    }
    loading.value = false;
    streamingProgress.value.isStreaming = false;
  }
}

function stopQuery() {
  // 取消 SSE 请求
  if (currentAbortController) {
    currentAbortController.abort();
    currentAbortController = null;
  }
  // 增加请求序列号，使之前的请求回调失效
  currentRequestId++;
  loading.value = false;
  streamingProgress.value.isStreaming = false;
}

function exportMessages() {
  if (messages.value.length === 0) return;

  const data = JSON.stringify(messages.value, null, 2);
  const blob = new Blob([data], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${selectedTopic.value}_messages_${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

// 发送消息弹框控制
function openSendModal() {
  // 如果没有选中分区，默认选第一个
  if (partitions.value.length > 0 && messageForm.value.partition === 0) {
    messageForm.value.partition = partitions.value[0] ?? 0;
  }
  sendModalRef.value?.showModal();
}

function closeSendModal() {
  sendModalRef.value?.close();
  sendSuccess.value = false;
  lastOffset.value = null;
}

async function handleSendMessage(keepOpen: boolean) {
  if (!selectedCluster.value || !selectedTopic.value) return;
  if (!messageForm.value.value.trim()) return;

  sending.value = true;
  sendSuccess.value = false;

  try {
    // 转换 headers 数组为对象
    const headers: Record<string, string> = {};
    for (const h of messageForm.value.headers) {
      if (h.key.trim()) {
        headers[h.key.trim()] = h.value;
      }
    }

    const result = await apiClient.sendMessage(selectedCluster.value, selectedTopic.value, {
      partition: messageForm.value.partition,
      key: messageForm.value.key || undefined,
      value: messageForm.value.value,
      headers: Object.keys(headers).length > 0 ? headers : undefined,
    });

    lastOffset.value = result.offset;
    sendSuccess.value = true;
    showSuccess(t.value.messages.messageSent);

    if (!keepOpen) {
      // 清空表单并关闭弹框
      setTimeout(() => {
        messageForm.value.key = '';
        messageForm.value.value = '';
        messageForm.value.headers = [];
        showHeaders.value = false;
        closeSendModal();
      }, 500);
    }
    // keepOpen 为 true 时不清空任何内容，方便重复发送相同消息
  } catch (e) {
    console.error('Failed to send message:', e);
  } finally {
    sending.value = false;
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
function getMsgPartition(item: any): number { return item?.partition ?? 0; }
function getMsgOffset(item: any): number { return item?.offset ?? 0; }
function getMsgKey(item: any): string | null { return item?.key; }
function getMsgValue(item: any): string | null { return item?.value; }
function getMsgTimestamp(item: any): number | null { return item?.timestamp; }

// 格式化值为不同格式
function formatValue(value: string | null, format: 'json' | 'raw' | 'hex'): string {
  if (!value) return 'null';

  if (format === 'json') {
    try {
      const parsed = JSON.parse(value);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return value;
    }
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

// JSON 语法高亮 - 将 JSON 字符串转换为带样式的 HTML
function highlightJson(json: string): string {
  if (!json) return '';

  // 转义 HTML 特殊字符
  let html = json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  // 高亮字符串（包括键和字符串值）
  html = html.replace(
    /("(?:\\.|[^"\\])*")/g,
    '<span class="json-string">$1</span>'
  );

  // 高亮数字
  html = html.replace(
    /:\s*(-?\d+\.?\d*)/g,
    ': <span class="json-number">$1</span>'
  );

  // 高亮布尔值和 null
  html = html.replace(
    /:\s*(true|false|null)/g,
    ': <span class="json-boolean">$1</span>'
  );

  // 高亮键名（在 : 前面的字符串）
  html = html.replace(
    /(<span class="json-string">)([^<]+)(<\/span>)(\s*:)/g,
    '<span class="json-key">$2</span>$4'
  );

  // 高亮标点符号
  html = html.replace(/([{}[\],])/g, '<span class="json-punctuation">$1</span>');

  return html;
}

function copyKey() {
  if (!selectedMessage.value?.key) return;
  navigator.clipboard.writeText(selectedMessage.value.key).then(() => {
    showSuccess(`${t.value.messages.key} ${t.value.messages.copied}`, 2000);
  });
}

function copyValue() {
  if (!selectedMessage.value?.value) return;
  const text = formatValue(selectedMessage.value.value, valueViewFormat.value);
  navigator.clipboard.writeText(text).then(() => {
    showSuccess(`${t.value.messages.value} ${t.value.messages.copied}`, 2000);
  });
}

function copyMessageValue(msg: any) {
  if (!msg?.value) return;
  // 默认按 JSON 格式化复制
  const text = formatValue(msg.value, 'json');
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
    await queryMessages();
  }
});

// 监听 props 变化
watch(() => props.cluster, async (newCluster) => {
  if (newCluster && newCluster !== selectedCluster.value) {
    selectedCluster.value = newCluster;
    await loadPartitions();
    if (selectedCluster.value && selectedTopic.value) {
      await queryMessages();
    }
  }
});

watch(() => props.topic, async (newTopic) => {
  if (newTopic && newTopic !== selectedTopic.value) {
    selectedTopic.value = newTopic;
    selectedPartition.value = 'all';
    partitions.value = [];
    messages.value = [];
    await loadPartitions();
    if (selectedCluster.value && selectedTopic.value) {
      await queryMessages();
    }
  }
});

onUnmounted(() => {
  if (loading.value) {
    stopQuery();
  }
  stopResize();
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

pre {
  white-space: pre-wrap;
  word-break: break-all;
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
  will-change: transform;
}

/* JSON 语法高亮样式 - 使用 :deep 穿透 scoped 限制 */
:deep(.json-highlight .json-key) {
  color: #9cdcfe;
}

:deep(.json-highlight .json-string) {
  color: #ce9178;
}

:deep(.json-highlight .json-number) {
  color: #b5cea8;
}

:deep(.json-highlight .json-boolean) {
  color: #569cd6;
}

:deep(.json-highlight .json-punctuation) {
  color: #d4d4d4;
}

:deep([data-theme="light"] .json-highlight .json-key) {
  color: #2c7a7b;
}

:deep([data-theme="light"] .json-highlight .json-string) {
  color: #b07d58;
}

:deep([data-theme="light"] .json-highlight .json-number) {
  color: #2f855a;
}

:deep([data-theme="light"] .json-highlight .json-boolean) {
  color: #2b6cb0;
}

:deep([data-theme="light"] .json-highlight .json-punctuation) {
  color: #718096;
}
</style>

<style>
/* JSON 语法高亮样式 - 全局样式，用于 v-html 内容 */
.json-highlight .json-key {
  color: #9cdcfe;
}

.json-highlight .json-string {
  color: #ce9178;
}

.json-highlight .json-number {
  color: #b5cea8;
}

.json-highlight .json-boolean {
  color: #569cd6;
}

.json-highlight .json-punctuation {
  color: #d4d4d4;
}

[data-theme="light"] .json-highlight .json-key {
  color: #2c7a7b;
}

[data-theme="light"] .json-highlight .json-string {
  color: #b07d58;
}

[data-theme="light"] .json-highlight .json-number {
  color: #2f855a;
}

[data-theme="light"] .json-highlight .json-boolean {
  color: #2b6cb0;
}

[data-theme="light"] .json-highlight .json-punctuation {
  color: #718096;
}
</style>
