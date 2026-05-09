<template>
  <div class="flex flex-col h-full overflow-hidden">
    <div class="p-3 relative flex-1 flex flex-col min-h-0">
    <!-- Page Header -->
    <div class="mb-4 relative flex-shrink-0">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-xl font-bold text-gradient flex items-center gap-2">
            <button class="btn btn-ghost btn-xs p-1 mr-2" @click="router.back()" :title="t.common.back || 'Back'">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
              </svg>
            </button>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281Z" />
              <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
            </svg>
            {{ t.settings.title }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">{{ t.settings.description }}</p>
        </div>
      </div>
    </div>

    <!-- Content Area (scrollable) -->
    <div class="flex-1 overflow-auto min-h-0">
      <!-- Settings Cards -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 pb-4">
      <!-- Language Setting -->
      <div class="card glass gradient-border">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center glow-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="m10.5 21 5.25-13.5m0 0L10.5 21m0 0 5.25 5.25M15.75 7.5H2.25a.75.75 0 0 1-.75-.75v-1.5h15v1.5a.75.75 0 0 1-.75.75Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M15 19.5H2.25a.75.75 0 0 1-.75-.75v-1.5h15v1.5a.75.75 0 0 1-.75.75Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M21.75 10.5v3a.75.75 0 0 1-.75.75h-1.5a.75.75 0 0 1-.75-.75v-3a.75.75 0 0 1 .75-.75h1.5a.75.75 0 0 1 .75.75Z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient">{{ t.settings.language }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.selectLanguage }}</p>
            </div>
          </div>
          <LanguageSelector />
        </div>
      </div>

      <!-- Theme Setting -->
      <div class="card glass gradient-border">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center glow-secondary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-secondary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient-warm">{{ t.settings.theme }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.themeDesc }}</p>
            </div>
          </div>
          <div class="flex items-center justify-between p-3 rounded-xl bg-base-100/50">
            <span class="text-sm font-medium">{{ isDark ? t.settings.darkMode : t.settings.lightMode }}</span>
            <button class="btn btn-toggle relative overflow-hidden" @click="handleToggleTheme">
              <input type="checkbox" :checked="isDark" class="toggle" />
            </button>
          </div>
        </div>
      </div>

      <!-- Sidebar Mode Setting -->
      <div class="card glass gradient-border">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-accent/10 flex items-center justify-center glow-accent">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-accent">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient">{{ t.settings.sidebarMode }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.selectSidebarMode }}</p>
            </div>
          </div>
          <div class="flex flex-col gap-2 p-3 rounded-xl bg-base-100/50">
            <button
              class="btn btn-sm"
              :class="{ 'btn-active btn-primary': sidebarMode === 'tree', 'btn-ghost': sidebarMode !== 'tree' }"
              @click="setSidebarMode('tree')"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" />
              </svg>
              <span class="font-medium">{{ t.settings.treeMode }}</span>
            </button>
            <p class="text-xs text-base-content/50 px-1">{{ t.settings.treeModeDesc }}</p>
            <div class="divider my-1"></div>
            <button
              class="btn btn-sm"
              :class="{ 'btn-active btn-primary': sidebarMode === 'flat', 'btn-ghost': sidebarMode !== 'flat' }"
              @click="setSidebarMode('flat')"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 6.75h12M8.25 12h12m-12 5.25h12M3.75 6.75h.007v.008H3.75V6.75zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zM3.75 12h.007v.008H3.75V12zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm-.375 5.25h.007v.008H3.75v-.008zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
              </svg>
              <span class="font-medium">{{ t.settings.flatMode }}</span>
            </button>
            <p class="text-xs text-base-content/50 px-1">{{ t.settings.flatModeDesc }}</p>
          </div>
        </div>
      </div>

      <!-- Version Info -->
      <div class="card glass gradient-border hover:glow-primary transition-all duration-300">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center glow-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient">{{ t.settings.version }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.versionDesc }}</p>
            </div>
          </div>
          <div class="p-3 rounded-xl bg-base-100/50 flex items-center justify-between">
            <span class="text-sm font-medium">{{ t.settings.currentVersion }}</span>
            <div class="flex items-center gap-2">
              <span
                class="badge badge-primary font-mono cursor-pointer select-none"
                @click="handleVersionClick"
                title="点击查看版本历史"
              >
                {{ appVersion }}
              </span>
              <button
                v-if="updateAvailable"
                class="btn btn-xs btn-warning text-white"
                @click="showUpdateModal = true"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M11.42 15.17 17.25 20A2.653 2.653 0 0 0 21 17.25l-4.662-4.662a7.467 7.467 0 0 0-.75-5.933m-6.75 5.511L12 15m0 0V3.75m0 11.25-2.822-2.822m0 0a7.467 7.467 0 0 1 5.933-.75m-5.933.75L5.25 15m0 0V3.75m0 11.25a2.653 2.653 0 0 1-3.75-3.75L6.127 6.375" />
                </svg>
                {{ t.update.newVersion }} {{ updateInfo?.version }}
              </button>
            </div>
          </div>
          <div class="p-3 rounded-xl bg-base-100/50 flex items-center justify-between mt-2">
            <span class="text-sm font-medium">{{ t.settings.author }}</span>
            <span class="text-sm font-medium">朱占全</span>
          </div>
          <div class="p-3 rounded-xl bg-base-100/50 flex items-center justify-between mt-2">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium">{{ t.update.checkForUpdates }}</span>
              <span v-if="checking" class="loading loading-spinner loading-xs"></span>
              <span v-if="updateAvailable" class="text-warning text-xs">{{ t.update.newVersion }} {{ updateInfo?.version }}</span>
            </div>
            <div class="flex gap-2">
              <button v-if="showLogButton" class="btn btn-sm" @click="viewLogs">
                {{ t.settings.viewLogs }}
              </button>
              <button v-if="isTauriEnv" class="btn btn-sm" @click="checkForUpdates(true)">
                {{ checking ? t.update.checking : t.update.checkNow }}
              </button>
            </div>
          </div>
          <!-- 后台下载进度 -->
          <div v-if="downloading" class="p-3 rounded-xl bg-base-300 flex flex-col gap-2 mt-2 border border-primary/50 shadow-lg">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span class="loading loading-spinner loading-sm text-primary"></span>
                <span class="text-sm font-bold text-base-content">{{ t.update.downloading || '正在下载更新...' }}</span>
              </div>
              <button class="btn btn-xs btn-error text-white" @click="cancelDownload">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
                {{ t.common.cancel || '取消' }}
              </button>
            </div>
            <div class="text-xs text-base-content/70">{{ downloadDetailText }}</div>
            <progress class="progress progress-primary w-full" :value="downloadProgress" max="100"></progress>
          </div>
        </div>
      </div>

      <!-- Import/Export -->
      <div class="card glass gradient-border">
        <div class="card-body p-3">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center glow-secondary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-secondary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-gradient-warm">{{ t.settings.importExport }}</h2>
              <p class="text-xs text-base-content/60">{{ t.settings.importExportDesc }}</p>
            </div>
          </div>
          <div class="flex gap-2">
            <button class="btn btn-sm btn-primary flex-1" @click="handleExport" :disabled="exporting">
              <svg v-if="!exporting" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-1">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
              </svg>
              <span v-else class="loading loading-spinner loading-xs mr-1"></span>
              {{ exporting ? t.settings.exporting : t.settings.exportData }}
            </button>
            <label class="btn btn-sm btn-secondary flex-1 cursor-pointer" :class="{ 'opacity-50': importing }">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-1">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" />
              </svg>
              {{ importing ? t.settings.importing : t.settings.importData }}
              <input type="file" accept=".json" @change="handleImport" :disabled="importing" class="hidden" />
            </label>
          </div>
        </div>
      </div>

      <!-- Logs Modal -->
      <dialog ref="logsModalRef" class="modal modal-bottom sm:modal-middle" :class="{ 'modal-open': showLogsModal }">
        <div class="modal-box max-w-5xl">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-lg">{{ t.settings.appLogs }}</h3>
            <div class="flex gap-2">
              <button class="btn btn-xs" @click="scrollToBottom" :title="t.settings.scrollToBottom">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 5.25 7.5 7.5 7.5-7.5m-15 6 7.5 7.5 7.5-7.5" />
                </svg>
              </button>
              <button class="btn btn-xs" @click="refreshLogs" :title="t.settings.refreshLogs">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
                </svg>
              </button>
              <button class="btn btn-xs" @click="copyLogs" :title="t.settings.copyLogs">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
              </button>
              <button class="btn btn-xs btn-ghost" @click="clearLogs" :title="t.settings.clearLogs">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                </svg>
              </button>
            </div>
          </div>
          <div ref="logsContainerRef" class="bg-base-200 rounded-lg p-3 max-h-[60vh] overflow-auto">
            <pre class="text-xs font-mono whitespace-pre-wrap">{{ logs }}</pre>
          </div>
          <div class="modal-action">
            <form method="dialog">
              <button class="btn" @click="showLogsModal = false">关闭</button>
            </form>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button @click="showLogsModal = false">close</button>
        </form>
      </dialog>

      <!-- Update Modal -->
      <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle" :class="{ 'modal-open': showUpdateModal }">
        <div class="modal-box">
          <h3 class="font-bold text-lg mb-4">
            {{ t.update.available }}
          </h3>

          <div class="flex items-center gap-3 mb-4">
            <div class="w-12 h-12 rounded-lg bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center flex-shrink-0 glow-primary">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M15.59 14.37a6 6 0 0 1-5.84 7.38c-4.38 0-7.21-4.79-5.43-8.96a7.46 7.46 0 0 1 3.43-3.69l.12-.06a6 6 0 1 1 7.72 5.33Zm-5.84-5.12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm6.54 6.04a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z" />
                <path stroke-linecap="round" stroke-linejoin="round" d="m15 12-3.5-3.5m0 0L8 12m3.5-3.5v9" />
              </svg>
            </div>
            <div>
              <div class="text-sm text-base-content/60">{{ t.update.currentVersion }}</div>
              <div class="font-mono font-bold">{{ appVersion }}</div>
            </div>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-base-content/40">
              <path stroke-linecap="round" stroke-linejoin="round" d="M17 8.25 21 12m0 0-4 3.75M21 12H3" />
            </svg>
            <div>
              <div class="text-sm text-base-content/60">{{ t.update.newVersion }}</div>
              <div class="font-mono font-bold text-primary">{{ updateInfo?.version }}</div>
            </div>
          </div>

          <div v-if="updateInfo?.notes" class="mb-4 p-3 bg-base-200 rounded-lg">
            <h4 class="font-semibold text-sm mb-2">{{ t.update.releaseNotes }}</h4>
            <div class="text-sm whitespace-pre-wrap">{{ updateInfo.notes }}</div>
          </div>

          <!-- 下载进度 -->
          <div v-if="downloading" class="mb-4">
            <div class="flex justify-between text-xs mb-1">
              <span>{{ t.update.downloadingInBackground || '正在后台下载...' }}</span>
              <span>{{ downloadProgress }}%</span>
            </div>
            <progress class="progress progress-primary w-full" :value="downloadProgress" max="100"></progress>
            <p class="text-xs text-base-content/60 mt-2">{{ t.update.minimizeHint || '关闭弹窗后下载将在后台继续' }}</p>
          </div>

          <!-- 下载中断提示 -->
          <div v-if="downloadInterrupted" class="mb-4 p-3 bg-warning/10 border border-warning/30 rounded-lg">
            <div class="flex items-center gap-2 text-warning">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z" />
              </svg>
              <span class="font-medium">下载已中断</span>
            </div>
            <p class="text-xs text-base-content/70 mt-1">检测到下载未完成，您可以选择重新下载（支持断点续传）</p>
          </div>

          <div class="modal-action">
            <div class="flex gap-2 w-full">
              <button
                type="button"
                class="btn btn-outline flex-1 border-base-content/30 hover:border-base-content"
                :class="downloading ? 'text-base-content' : 'text-base-content'"
                @click="showUpdateModal = false"
              >
                <svg v-if="downloading" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4 mr-1">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 5.25l-7.5 7.5-7.5-7.5m15 6l-7.5 7.5-7.5-7.5" />
                </svg>
                <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4 mr-1">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
                <span class="font-medium">{{ downloading ? (t.update.minimizeModal || '隐藏窗口') : (t.common.cancel || '取消') }}</span>
              </button>
              <button
                v-if="downloadInterrupted"
                type="button"
                class="btn btn-warning flex-1 text-white font-medium shadow-lg shadow-warning/20"
                @click="resumeDownload"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4 mr-1">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
                </svg>
                <span>重新下载</span>
              </button>
              <button
                v-else
                type="button"
                class="btn btn-primary flex-1 text-white font-medium shadow-lg shadow-primary/20"
                :class="{'opacity-60 cursor-not-allowed': downloading}"
                :disabled="downloading"
                @click="installUpdate"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4 mr-1">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" />
                </svg>
                <span>{{ downloading ? (t.update.downloading || '下载中') : (t.update.updateAndRestart || '下载并安装') }}</span>
              </button>
            </div>
          </div>
        </div>

        <form method="dialog" class="modal-backdrop">
          <button @click="showUpdateModal = false">close</button>
        </form>
      </dialog>

      <!-- JSON Highlight Setting -->
      <div class="md:col-span-2">
        <JsonHighlightSelector />
      </div>
    </div>
    </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useRouter } from 'vue-router';
import { useThemeStore } from '@/stores/theme';
import { useLanguageStore } from '@/stores/language';
import { useUpdateStore } from '@/stores/update';
import { useToast } from '@/composables/useToast';
import { apiClient } from '@/api/client';
import LanguageSelector from '@/components/Settings/LanguageSelector.vue';
import JsonHighlightSelector from '@/components/Settings/JsonHighlightSelector.vue';

// 检测是否在 Tauri 环境下运行
function isTauri(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }
  const win = window as any;
  return !!(
    win.__TAURI__ ||
    win.__TAURI_INTERNALS__ ||
    win.__TAURI_IPC__
  );
}

// 安全的 invoke 调用（Tauri 环境）
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  // 在调用时检测 Tauri 环境，使用 @tauri-apps/api 的 invoke 函数
  const win = window as any;
  if (win.__TAURI__?.core?.invoke || win.__TAURI__?.invoke) {
    // Tauri 环境，使用 Tauri invoke
    if (win.__TAURI__.core?.invoke) {
      return win.__TAURI__.core.invoke(cmd, args);
    } else {
      return win.__TAURI__.invoke(cmd, args);
    }
  }
  // 非 Tauri 环境，通过 HTTP 调用后端 API
  const response = await fetch('http://localhost:9732/api', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-API-Method': cmd,
    },
    body: JSON.stringify(args || {}),
  });
  const data = await response.json();
  if (!data.success) {
    throw new Error(data.error || 'API call failed');
  }
  return data.data as T;
}

const themeStore = useThemeStore();
const languageStore = useLanguageStore();
const updateStore = useUpdateStore();
const toast = useToast();
const router = useRouter();

const { isDark, toggleTheme } = themeStore;

// 翻译快捷访问
const t = computed(() => languageStore.t);

// Tauri 环境检测
const isTauriEnv = ref(isTauri());

// App version（初始值，onMounted 时会通过 getCurrentVersion() 从后端 Cargo.toml 读取覆盖）
const appVersion = ref('0.0.0');

// 日志相关
const logs = ref('');
const showLogsModal = ref(false);
const logsModalRef = ref<HTMLDialogElement>();
const logsContainerRef = ref<HTMLDivElement>();

// 更新相关状态
const updateAvailable = ref(false);
const updateInfo = ref<{ version: string; notes: string } | null>(null);
const showUpdateModal = ref(false);
const checking = ref(false);
const installing = ref(false);
const downloadInterrupted = ref(false);

// 使用 store 中的下载状态（持久化）
const { downloading, downloadProgress, downloadDownloaded, downloadTotal, downloadSpeed } = storeToRefs(updateStore);

// 格式化文件大小
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const KB = 1024;
  const MB = 1024 * KB;
  const GB = 1024 * MB;
  if (bytes >= GB) return `${(bytes / GB).toFixed(2)} GB`;
  if (bytes >= MB) return `${(bytes / MB).toFixed(2)} MB`;
  if (bytes >= KB) return `${(bytes / KB).toFixed(2)} KB`;
  return `${bytes} B`;
}

// 下载详细信息文本
const downloadDetailText = computed(() => {
  const downloaded = downloadDownloaded.value;
  const total = downloadTotal.value;
  const speed = downloadSpeed.value;

  if (total > 0) {
    const progress = (downloaded / total * 100).toFixed(1);
    let text = `${formatBytes(downloaded)}/${formatBytes(total)} (${progress}%)`;
    if (speed > 0) {
      text += `, speed: ${formatBytes(speed)}/s`;
    }
    return text;
  }

  // 无总大小时只显示已下载
  if (downloaded > 0) {
    let text = `${formatBytes(downloaded)} 已下载`;
    if (speed > 0) {
      text += `, speed: ${formatBytes(speed)}/s`;
    }
    return text;
  }

  return t.value.update.downloading || '正在下载更新...';
});

// 隐藏功能 - 查看日志按钮（2 秒内点击版本号 5 次显示）
// 默认隐藏，点击 5 次后显示
const showLogButton = ref(false);
const clickCount = ref(0);
const clickTimer = ref<ReturnType<typeof setTimeout> | null>(null);

// 导入导出相关
const exporting = ref(false);
const importing = ref(false);

// Modal ref
const modalRef = ref<HTMLDialogElement>();

// 监听 modal 打开/关闭
watch(showUpdateModal, (newVal) => {
  if (modalRef.value) {
    if (newVal) {
      modalRef.value.showModal();
    } else {
      modalRef.value.close();
    }
  }
});

// 监听日志 modal 打开/关闭
watch(showLogsModal, (newVal) => {
  if (logsModalRef.value) {
    if (newVal) {
      logsModalRef.value.showModal();
    } else {
      logsModalRef.value.close();
    }
  }
});

// 获取当前版本
async function getCurrentVersion() {
  try {
    const win = window as any;
    // Tauri 环境使用 get_app_version（返回字符串），浏览器环境使用 app.version（返回{version}）
    const cmd = win.__TAURI__?.core?.invoke || win.__TAURI__?.invoke ? 'get_app_version' : 'app.version';
    const result = await tauriInvoke<any>(cmd);
    // Tauri 返回纯字符串，HTTP API 返回{version}对象
    appVersion.value = typeof result === 'string' ? result : (result.version || '0.0.0');
  } catch (e) {
    console.error('Failed to get current version:', e);
  }
}

// 处理版本号点击（隐藏功能：2 秒内点击 5 次显示查看日志按钮）
function handleVersionClick() {
  clickCount.value++;

  // 如果是第 5 次点击，显示按钮
  if (clickCount.value >= 5) {
    showLogButton.value = true;
    clickCount.value = 0;
    if (clickTimer.value) {
      clearTimeout(clickTimer.value);
      clickTimer.value = null;
    }
    toast.showSuccess('已开启开发者功能');
    return;
  }

  // 清除之前的定时器
  if (clickTimer.value) {
    clearTimeout(clickTimer.value);
  }

  // 设置新的定时器，2 秒后重置计数
  clickTimer.value = setTimeout(() => {
    clickCount.value = 0;
    clickTimer.value = null;
  }, 2000);
}

// 查看日志
async function viewLogs() {
  try {
    const win = window as any;
    // Tauri 环境使用 get_app_logs（返回字符串），浏览器环境使用 app.logs（返回{logs}）
    const cmd = win.__TAURI__?.core?.invoke || win.__TAURI__?.invoke ? 'get_app_logs' : 'app.logs';
    const result = await tauriInvoke<any>(cmd);
    // Tauri 返回纯字符串，HTTP API 返回{logs}对象
    logs.value = typeof result === 'string' ? result : (result.logs || t.value.settings.noLogs);
    showLogsModal.value = true;
  } catch (e) {
    console.error('Failed to get logs:', e);
    logs.value = t.value.settings.noLogs;
    showLogsModal.value = true;
  }
}

// 复制日志
async function copyLogs() {
  try {
    await navigator.clipboard.writeText(logs.value);
    toast.showSuccess(t.value.settings.logsCopied);
  } catch (e) {
    toast.showError(t.value.common.copyFailed);
  }
}

// 滚动到底部
function scrollToBottom() {
  if (logsContainerRef.value) {
    logsContainerRef.value.scrollTop = logsContainerRef.value.scrollHeight;
  }
}

// 刷新日志
async function refreshLogs() {
  try {
    const win = window as any;
    const cmd = win.__TAURI__?.core?.invoke || win.__TAURI__?.invoke ? 'get_app_logs' : 'app.logs';
    const result = await tauriInvoke<any>(cmd);
    logs.value = typeof result === 'string' ? result : (result.logs || t.value.settings.noLogs);
    toast.showSuccess(t.value.settings.logsRefreshed);
  } catch (e) {
    console.error('Failed to refresh logs:', e);
    toast.showError(t.value.settings.refreshLogs + t.value.common.failed);
  }
}

// 清除日志
async function clearLogs() {
  try {
    const win = window as any;
    // Tauri 环境使用 clear_app_logs，浏览器环境使用 app.logs.clear
    const cmd = win.__TAURI__?.core?.invoke || win.__TAURI__?.invoke ? 'clear_app_logs' : 'app.logs.clear';
    await tauriInvoke(cmd);
    logs.value = t.value.settings.noLogs;
    toast.showSuccess(t.value.settings.logsCleared);
  } catch (e) {
    toast.showError(t.value.settings.clearLogs + t.value.common.failed);
  }
}

// 导出数据
async function handleExport() {
  if (exporting.value) return;

  exporting.value = true;
  try {
    const data = await apiClient.exportData();
    const exportData = {
      version: data.version,
      exported_at: data.exported_at,
      cluster_groups: data.cluster_groups,
      clusters: data.clusters,
      topics: data.topics,
      favorites: data.favorites,
      history: data.history,
    };

    // 创建下载
    const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `kafka-manager-export-${new Date().toISOString().split('T')[0]}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    const lang = languageStore.currentLanguage;
    if (lang === 'zh') {
      toast.showSuccess(`导出成功：${data.cluster_groups.length} 个分组，${data.clusters.length} 个集群，${data.topics.length} 个 Topic，${data.favorites.length} 个收藏分组，${data.history.length} 条历史。文件已保存到下载目录`);
    } else {
      toast.showSuccess(`Export successful: ${data.cluster_groups.length} groups, ${data.clusters.length} clusters, ${data.topics.length} topics, ${data.favorites.length} favorites, ${data.history.length} history records. File saved to downloads folder`);
    }
  } catch (e) {
    console.error('Export failed:', e);
    const msg = (e as Error).message || '';
    if (msg.includes('操作正在进行中') || msg.toLowerCase().includes('operation') && msg.toLowerCase().includes('progress')) {
      toast.showError(t.value.settings.operationInProgress);
    } else {
      toast.showError(msg);
    }
  } finally {
    exporting.value = false;
  }
}

// 导入数据
async function handleImport(event: Event) {
  if (importing.value) return;

  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];
  if (!file) return;

  // 重置 input，允许重复选择同一文件
  target.value = '';

  importing.value = true;
  try {
    const text = await file.text();
    const data = JSON.parse(text);

    // 验证数据格式
    if (!data.clusters && !data.favorites && !data.history && !data.cluster_groups && !data.topics) {
      throw new Error('Invalid export file format');
    }

    const result = await apiClient.importData({
      cluster_groups: data.cluster_groups || [],
      clusters: data.clusters || [],
      topics: data.topics || [],
      favorites: data.favorites || [],
      history: data.history || [],
    }, 'skip');

    // 异步导入：后端立即返回 "Import started in background"
    const lang = languageStore.currentLanguage;
    if (lang === 'zh') {
      toast.showSuccess(t.value.settings.importStarted);
    } else {
      toast.showSuccess(t.value.settings.importStarted);
    }

    // 短暂锁定按钮，避免用户重复点击
    await new Promise(resolve => setTimeout(resolve, 2000));
  } catch (e) {
    console.error('Import failed:', e);
    const msg = (e as Error).message || '';
    if (msg.includes('操作正在进行中') || msg.toLowerCase().includes('operation') && msg.toLowerCase().includes('progress')) {
      toast.showError(t.value.settings.operationInProgress);
    } else {
      toast.showError(msg);
    }
  } finally {
    importing.value = false;
  }
}

// 检查更新（仅 Tauri 环境）
async function checkForUpdates(manual = false) {
  if (checking.value) return;

  // 非 Tauri 环境不支持检查更新
  if (!isTauri()) {
    if (manual) {
      toast.showInfo(t.value.update.browserNotSupported);
    }
    return;
  }

  checking.value = true;
  try {
    const result = await tauriInvoke<{
      available: boolean;
      version: string;
      notes: string;
      date: string;
    }>('check_for_updates');

    if (result.available) {
      updateAvailable.value = true;
      updateInfo.value = {
        version: result.version,
        notes: result.notes || '',
      };
      if (manual) {
        showUpdateModal.value = true;
      }
    } else {
      // 无更新时清除状态
      updateAvailable.value = false;
      updateInfo.value = null;
      if (manual) {
        // 使用 Toast 显示成功提示，替代系统 alert
        toast.showInfo(t.value.update.checkCompleteNoUpdate || '已是最新版本', 3000);
      }
    }
  } catch (e) {
    console.error('Failed to check for updates:', e);
    // 检查失败时清除状态，避免显示过时的更新提示
    updateAvailable.value = false;
    updateInfo.value = null;

    // 处理 403 Forbidden 错误（GitHub API 限流）
    const errorMsg = typeof e === 'string' ? e : (e as Error)?.message || '';
    if (errorMsg.includes('403')) {
      toast.showError(t.value.update.rateLimitExceeded || '访问受限，请稍后重试');
    } else if (manual) {
      toast.showError(t.value.update.checkFailed || '检查更新失败');
    }
  } finally {
    checking.value = false;
  }
}

// 安装更新（仅 Tauri 环境）
async function installUpdate() {
  // 先同步后端状态，避免状态不一致
  try {
    const backendStatus = await tauriInvoke<any>('get_download_status');
    if (backendStatus.is_downloading) {
      // 检查是否是"假死"状态：后端显示在下载，但前端显示下载已中断
      // 这种情况下需要清除后端状态重新开始
      if (downloadInterrupted.value) {
        console.log('Download state inconsistent, clearing and retrying...');
        await tauriInvoke('clear_download_status');
      } else {
        // 后端确实在下载，更新前端状态并显示进度
        updateStore.setDownloading(true, backendStatus.total > 0 ? Math.round((backendStatus.downloaded / backendStatus.total) * 100) : 0, backendStatus.downloaded, backendStatus.total);
        toast.showInfo(t.value.update.downloadInProgress || '下载正在进行中，请稍候...');
        return;
      }
    }
    // 后端没有在下载，但前端显示在下载（状态不同步），清除前端状态
    if (downloading.value && !backendStatus.is_downloading) {
      updateStore.clearState();
      downloadInterrupted.value = false;
    }
  } catch (e) {
    console.error('Failed to get download status:', e);
  }

  try {
    // 先获取当前下载状态（包括缓存文件）
    const status = await tauriInvoke<any>('get_download_status');
    const initialProgress = status.total > 0
      ? Math.round((status.downloaded / status.total) * 100)
      : (status.downloaded > 0 ? 1 : 0);

    updateStore.setDownloading(true, initialProgress, status.downloaded, status.total, 0);

    // 开始轮询下载状态
    startPollingDownloadStatus();

    // 调用后台下载接口（不等待完成）
    tauriInvoke('install_update').catch((e) => {
      console.error('Background download error:', e);
      updateStore.clearState();
      stopPolling();
      const errorMsg = typeof e === 'string' ? e : (e as Error)?.message || t.value.common.unknown;
      if (errorMsg === 'network_error') {
        toast.showError(t.value.update.networkError);
      } else {
        toast.showError(t.value.update.installFailed + ': ' + errorMsg);
      }
    });

    // 注意：下载在后台进行，不由这里处理完成逻辑
    // 完成逻辑由轮询函数处理
  } catch (e) {
    updateStore.clearState();
    stopPolling();
    console.error('Failed to install update:', e);
    const errorMsg = typeof e === 'string' ? e : (e as Error)?.message || t.value.common.unknown;
    if (errorMsg === 'network_error') {
      toast.showError(t.value.update.networkError);
    } else {
      toast.showError(t.value.update.installFailed + ': ' + errorMsg);
    }
  }
}

// 轮询下载状态（使用自调度的 setTimeout，避免 setInterval + async 重叠问题）
let pollTimeout: ReturnType<typeof setTimeout> | null = null;
let downloadEverStarted = false;
let lastDownloadedBytes = 0; // 用于计算速度

function startPollingDownloadStatus() {
  // 清除之前的轮询
  stopPolling();
  downloadEverStarted = false;
  lastDownloadedBytes = 0;

  // 延迟 200ms 后开始首次轮询
  pollTimeout = setTimeout(() => {
    pollTimeout = null;
    pollDownloadStatus();
  }, 200);
}

async function pollDownloadStatus() {
  try {
    const status = await tauriInvoke<any>('get_download_status');
    if (status.is_downloading) {
      const progress = status.total > 0 ? Math.round((status.downloaded / status.total) * 100) : 0;
      const speed = status.downloaded - lastDownloadedBytes;
      lastDownloadedBytes = status.downloaded;
      updateStore.updateProgress(progress, status.downloaded, status.total, speed * 2);
      downloadInterrupted.value = false;
      downloadEverStarted = true;
      // 500ms 后继续下一次轮询
      pollTimeout = setTimeout(pollDownloadStatus, 500);
    } else {
      // 下载完成或停止
      if (status.downloaded > 0 && status.total > 0 && status.downloaded >= status.total) {
        updateStore.clearState();
        downloadInterrupted.value = false;
        toast.showSuccess(t.value.update.downloadComplete || '下载完成，安装包已打开');
      } else if (status.downloaded > 0 && status.total === 0) {
        updateStore.clearState();
        downloadInterrupted.value = false;
        toast.showInfo(t.value.update.downloadComplete || '下载完成，安装包已打开');
      } else if (status.downloaded > 0 || status.total > 0) {
        updateStore.setDownloading(false, 0);
        downloadInterrupted.value = true;
      } else if (status.downloaded === 0 && status.total === 0) {
        if (downloadEverStarted) {
          updateStore.setDownloading(false, 0);
          downloadInterrupted.value = true;
        } else if (downloading.value) {
          updateStore.clearState();
          downloadInterrupted.value = false;
        }
      }
      stopPolling();
    }
  } catch (e) {
    console.error('Failed to get download status:', e);
    // 出错后 500ms 继续轮询
    pollTimeout = setTimeout(pollDownloadStatus, 500);
  }
}

function stopPolling() {
  if (pollTimeout) {
    clearTimeout(pollTimeout);
    pollTimeout = null;
  }
}

// 取消下载
async function cancelDownload() {
  stopPolling();
  try {
    await tauriInvoke('clear_download_status');
  } catch (e) {
    console.error('Failed to cancel download:', e);
  }
  updateStore.clearState();
  downloadInterrupted.value = false;
  toast.showInfo('已取消下载');
}

// 重新下载（断点续传）
async function resumeDownload() {
  downloadInterrupted.value = false;
  downloadEverStarted = false;
  // 先清除后端下载状态，避免"下载已在进行中"的错误
  try {
    await tauriInvoke('clear_download_status');
  } catch (e) {
    console.error('Failed to clear download status:', e);
  }
  await installUpdate();
}

// Sidebar mode state
const sidebarMode = ref<'tree' | 'flat'>('flat');

// Load sidebar mode setting
async function loadSidebarModeSetting() {
  try {
    const settings = await apiClient.getSettings(['ui.sidebar_mode']);
    const mode = settings.find((s: { key: string; value: string }) => s.key === 'ui.sidebar_mode')?.value;
    if (mode === 'tree' || mode === 'flat') {
      sidebarMode.value = mode;
    }
  } catch (e) {
    console.error('Failed to load sidebar mode setting:', e);
  }
}

// Set sidebar mode
async function setSidebarMode(mode: 'tree' | 'flat') {
  sidebarMode.value = mode;
  try {
    await apiClient.updateSetting('ui.sidebar_mode', mode);
    // 触发设置变化事件通知其他组件
    window.dispatchEvent(new CustomEvent('settings-changed', {
      detail: { key: 'ui.sidebar_mode', value: mode }
    }));
  } catch (e) {
    console.error('Failed to save sidebar mode setting:', e);
  }
}

function handleToggleTheme() {
  toggleTheme();
}

onMounted(() => {
  getCurrentVersion();
  loadSidebarModeSetting();
  // 监听后端下载错误事件
  setupDownloadErrorListener();
  // 检查后端是否有下载状态或缓存文件
  tauriInvoke<any>('get_download_status').then((status) => {
    if (status.is_downloading) {
      // 后端确实在下载，开始轮询
      updateStore.setDownloading(true, 0);
      startPollingDownloadStatus();
    } else if (status.downloaded > 0 && status.total > 0 && downloading.value) {
      // 只有当前端已经在显示下载状态时，才检查是否中断
      if (status.downloaded >= status.total) {
        // 下载完成
        updateStore.clearState();
        toast.showSuccess(t.value.update.downloadComplete || '下载完成，安装包已打开');
      } else {
        // 下载中断，显示重新下载按钮
        downloadInterrupted.value = true;
        // 更新前端的下载进度显示
        updateStore.updateProgress(Math.round((status.downloaded / status.total) * 100));
        console.log('Download interrupted, showing resume button');
      }
    }
  }).catch((e) => {
    console.error('Failed to get download status:', e);
  });
});

// 监听下载状态变化，自动开始/停止轮询
watch(downloading, (newVal) => {
  if (newVal) {
    startPollingDownloadStatus();
  } else {
    stopPolling();
  }
});

// 组件卸载时清理定时器
onUnmounted(() => {
  if (clickTimer.value) {
    clearTimeout(clickTimer.value);
  }
  stopPolling();
  if (unlistenDownloadError) {
    unlistenDownloadError();
  }
});

// 监听后端下载错误事件
let unlistenDownloadError: (() => void) | null = null;
function setupDownloadErrorListener() {
  const win = window as any;
  // Tauri v2 global event API check
  if (win.__TAURI__ && win.__TAURI__.event && typeof win.__TAURI__.event.listen === 'function') {
    try {
      win.__TAURI__.event.listen('download_error', (event: any) => {
        console.error('Download error from backend:', event.payload);
        stopPolling();
        updateStore.clearState();
        const msg = typeof event.payload === 'string' ? event.payload : '未知错误';
        toast.showError(`下载失败: ${msg}`);
      }).then((unlisten: () => void) => {
        unlistenDownloadError = unlisten;
      }).catch((e: Error) => {
        console.error('Failed to setup download error listener:', e);
      });
    } catch (e) {
      console.error('Error setting up download error listener:', e);
    }
  }
}
</script>

