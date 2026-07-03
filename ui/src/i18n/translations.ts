export type Language = 'zh' | 'en';

export interface Translation {
  nav: {
    clusters: string;
    favorites: string;
    topics: string;
    messages: string;
    settings: string;
  };
  tree: {
    backToClusters: string;
    collapseAll: string;
    clusters: string;
    topicFavorites: string;
    schemaRegistry: string;
    refreshConsumerGroups: string;
  };
  common: {
    loading: string;
    error: string;
    success: string;
    save: string;
    cancel: string;
    delete: string;
    edit: string;
    create: string;
    refresh: string;
    search: string;
    noData: string;
    actions: string;
    name: string;
    status: string;
    connected: string;
    disconnected: string;
    all: string;
    filter: string;
    back: string;
    ready: string;
    optional: string;
    confirm: string;
    close: string;
    confirmDelete: string;
    apply: string;
    clear: string;
    cannotBeGreaterThan: string;
    unknown: string;
    copy: string;
    copyFailed: string;
    viewDetails: string;
    view: string;
    failed: string;
    hide: string;
  };
  clusters: {
    title: string;
    description: string;
    addCluster: string;
    editCluster: string;
    createCluster: string;
    clusterName: string;
    brokers: string;
    requestTimeout: string;
    operationTimeout: string;
    removeCluster: string;
    viewBrokers: string;
    viewTopics: string;
    createTopic: string;
    connectionError: string;
    retry: string;
    brokersLabel: string;
    timeoutsLabel: string;
    confirmDelete: string;
    disconnectConfirm: string;
    reconnectFailed: string;
    fetchFailed: string;
    connectedLabel: string;
    disconnectedLabel: string;
    connectingLabel: string;
    updated: string;
    created: string;
    connected: string;
    refreshed: string;
    topicsRefreshed: string;
    refreshingBg: string;
    reconnected: string;
    topics: string;
    partitions: string;
    viewTopicsLink: string;
    group: string;
    noGroup: string;
    addGroup: string;
    testConnection: string;
    testingConnection: string;
    connectionSuccess: string;
    connectionFailed: string;
    noDescription: string;
    manageGroups: string;
    editGroup: string;
    groupName: string;
    groupNamePlaceholder: string;
    groupDescription: string;
    groupDescPlaceholder: string;
    confirmDeleteGroup: string;
    deleteGroupTitle: string;
    test: string;
    reconnect: string;
    disconnect: string;
    disconnectedSuccess: string;
    reconnectSuccess: string;
    groupUpdated: string;
    groupCreated: string;
    groupDeleted: string;
    clusterDeleted: string;
    scrollLeft: string;
    scrollRight: string;
    editClusterTitle: string;
    createClusterTitle: string;
    brokersHelp: string;
    createdDate: string;
    unknown: string;
    refreshTopics: string;
    refreshFailed: string;
    validationNameRequired: string;
    validationNameTooLong: string;
    validationNameInvalid: string;
    validationBrokersRequired: string;
    validationBrokersInvalid: string;
    newCluster: string;
    connectionTestSuccess: string;
    connectionTestFailed: string;
    clusterStatusRefreshed: string;
    clusterConnectionIssue: string;
    reconnectSuccessToast: string;
    clusterDeletedToast: string;
    clusters: string;
  };
  topics: {
    title: string;
    description: string;
    createTopic: string;
    topicName: string;
    topicNamePlaceholder: string;
    topicNameValidation: string;
    partitionCount: string;
    numPartitions: string;
    numPartitionsHelp: string;
    replicationFactor: string;
    replicationFactorHelp: string;
    advancedOptions: string;
    cleanupPolicy: string;
    retentionMs: string;
    retentionBytes: string;
    segmentBytes: string;
    retentionMsPlaceholder: string;
    retentionBytesPlaceholder: string;
    segmentBytesPlaceholder: string;
    viewMessages: string;
    viewDetails: string;
    viewPartitions: string;
    sendMessage: string;
    deleteTopic: string;
    exportData: string;
    topicDetails: string;
    partitions: string;
    settings: string;
    refreshed: string;
    refreshingBg: string;
    confirmDeleteTitle: string;
    confirmDeleteHint: string;
    confirmDeleteInput: string;
    confirmDeleteMatchError: string;
    copied: string;
    copyFailed: string;
    deletedSuccess: string;
    createdSuccess: string;
    createFailed: string;
    validationSelectCluster: string;
    validationClusterIdRequired: string;
    validationTopicNameRequired: string;
    validationTopicNameTooLong: string;
    validationTopicNameInvalidChars: string;
    validationTopicNameFormat: string;
    validationRetentionMs: string;
    validationRetentionBytes: string;
    validationSegmentBytes: string;
    noSearchResults: string;
    clearSearch: string;
  };
  messages: {
    title: string;
    description: string;
    sendMessage: string;
    partition: string;
    key: string;
    value: string;
    targetPartition: string;
    optional: string;
    required: string;
    messageSent: string;
    offset: string;
    send: string;
    sendAndNew: string;
    sending: string;
    allPartitions: string;
    fetch: string;
    filter: string;
    formatJson: string;
    fetchMode: string;
    oldest: string;
    newest: string;
    maxMessages: string;
    perPartition: string;
    startTime: string;
    endTime: string;
    timeRange: string;
    timeRangeFilter: string;
    clear: string;
    selectTopic: string;
    selectMessage: string;
    noTopicSelected: string;
    noMessages: string;
    offsetLabel: string;
    partitionLabel: string;
    timestampLabel: string;
    sizeLabel: string;
    viewAs: string;
    json: string;
    raw: string;
    hex: string;
    copied: string;
    messages: string;
    time: string;
    selectedOffset: string;
    ready: string;
    query: string;
    stop: string;
    exportMessages: string;
    messageDetail: string;
    copyValue: string;
    copyKey: string;
    valuePlaceholder: string;
    searchPlaceholder: string;
    elapsedTime: string;
    totalMessages: string;
    actions: string;
    close: string;
    cancel: string;
    continue: string;
    keyOptional: string;
    valueRequired: string;
    partitionLabel2: string;
    queryFailed: string;
    fetchFailed: string;
    loading: string;
    clearSort: string;
    hide: string;
    show: string;
    valid: string;
    exportSuccess: string;
    exportFailed: string;
    receiving: string;
    recent5Minutes: string;
    recent15Minutes: string;
    recent30Minutes: string;
    recent1Hour: string;
    recent1Day: string;
    topicLabel: string;
    cluster: string;
    minutes: string;
    hour: string;
    day: string;
    queryTimeout: string;
    exportTip: string;
    confirmDeleteTopic: string;
    deleteTopic: string;
    topicDeleted: string;
  };
  consumerGroups: {
    title: string;
    description: string;
    groupName: string;
    topics: string;
    state: string;
    partitions: string;
    offset: string;
    lag: string;
    start: string;
    end: string;
    resetOffset: string;
    resetOffsetToEarliest: string;
    resetOffsetToLatest: string;
    resetOffsetToTimestamp: string;
    timestamp: string;
    refreshOffsets: string;
    offsetsRefreshed: string;
    offsetResetSuccess: string;
    confirmResetOffset: string;
    confirmResetOffsetToEarliest: string;
    confirmResetOffsetToLatest: string;
    confirmResetOffsetToTimestamp: string;
    noData: string;
    emptyHelp: string;
    refreshed: string;
    refreshingBg: string;
    deleteGroup: string;
    deleted: string;
    offsets: string;
    topic: string;
    partition: string;
    startOffset: string;
    endOffset: string;
    committedOffset: string;
    lastCommit: string;
    selectTopic: string;
    resetTo: string;
    earliest: string;
    latest: string;
    specificOffset: string;
    offsetValue: string;
    timestampValue: string;
    noOffsets: string;
    selectFromNav: string;
    groupNamePrefix: string;
  };
  topicConsumerGroups: {
    title: string;
    description: string;
    noData: string;
    groupName: string;
    state: string;
    partitions: string;
    lag: string;
    viewDetails: string;
    refresh: string;
    refreshAllConsumerGroup: string;
    refreshed: string;
    dataNotice: string;
    dataNoticeTitle: string;
    topicNamePrefix: string;
  };
  settings: {
    title: string;
    description: string;
    language: string;
    theme: string;
    languageZh: string;
    languageEn: string;
    selectLanguage: string;
    systemSettings: string;
    sidebarMode: string;
    treeMode: string;
    flatMode: string;
    treeModeDesc: string;
    flatModeDesc: string;
    systemTray: string;
    systemTrayDesc: string;
    systemTrayEnabled: string;
    systemTrayDisabled: string;
    autoLaunch: string;
    autoLaunchDesc: string;
    autoLaunchEnabled: string;
    autoLaunchDisabled: string;
    autoLaunchFailed: string;
    version: string;
    versionDesc: string;
    currentVersion: string;
    author: string;
    help: string;
    themeDesc: string;
    lightMode: string;
    darkMode: string;
    jsonHighlight: string;
    jsonHighlightDesc: string;
    selectTemplate: string;
    preview: string;
    customTemplates: string;
    addCustomTemplate: string;
    templateName: string;
    templateDescription: string;
    templateStyle: string;
    saveTemplate: string;
    deleteTemplate: string;
    confirmDeleteTemplate: string;
    templateFormat: string;
    builtInTemplates: string;
    importExport: string;
    importExportDesc: string;
    exportData: string;
    exporting: string;
    exportSuccess: string;
    importData: string;
    importing: string;
    importSuccess: string;
    operationInProgress: string;
    importStarted: string;
    viewLogs: string;
    appLogs: string;
    refreshLogs: string;
    copyLogs: string;
    clearLogs: string;
    scrollToBottom: string;
    noLogs: string;
    logsCopied: string;
    logsCleared: string;
    logsRefreshed: string;
    feedback: string;
    feedbackDesc: string;
    feedbackPlaceholder: string;
    feedbackSubmit: string;
    feedbackSubmitting: string;
    feedbackSuccess: string;
    feedbackFailed: string;
    feedbackMaxLength: string;
    feedbackConnectionFailed: string;
    feedbackSystemInfo: string;
    feedbackDocLink: string;
  };
  layout: {
    searchPlaceholder: string;
    noTopicsFound: string;
    settings: string;
    help: string;
    checkForUpdates: string;
    confirmDeleteCluster: string;
    confirmDeleteTopic: string;
    clusterNotFound: string;
    topicNotFound: string;
    refreshFailed: string;
    refreshCancelled: string;
    shareVersion: string;
    shareCopied: string;
  };
  update: {
    available: string;
    currentVersion: string;
    newVersion: string;
    releaseNotes: string;
    downloading: string;
    installing: string;
    installed: string;
    updateAndRestart: string;
    checkCompleteNoUpdate: string;
    checkFailed: string;
    installFailed: string;
    networkError: string;
    downloadComplete: string;
    browserNotSupported: string;
    checkForUpdates: string;
    checking: string;
    checkNow: string;
    downloadingInBackground: string;
    minimizeHint: string;
    minimizeModal: string;
    rateLimitExceeded: string;
    downloadInProgress: string;
    downloadFailed: string;
  };
  toast: {
    error: string;
    success: string;
    warning: string;
    info: string;
    copySuccess: string;
    copyFailed: string;
    operationFailed: string;
    clusterNotFound: string;
    networkError: string;
    invalidFormat: string;
    skipped: string;
  };
  topicContextMenu: {
    viewMessages: string;
    viewDetails: string;
    viewPartitions: string;
    sendMessage: string;
    exportData: string;
    deleteTopic: string;
  };
  partitionContextMenu: {
    viewMessages: string;
    sendMessage: string;
  };
  mainLayout: {
    clustersSelected: string;
    refreshHealth: string;
    footerText: string;
    multiCluster: string;
    clustersLabel: string;
    selectAll: string;
    clearSelection: string;
    manageClusters: string;
    topics: string;
    messages: string;
    dragToResize: string;
  };
  navigator: {
    allClusters: string;
    byGroup: string;
    selectGroupsAndClusters: string;
    selected: string;
    selectCluster: string;
    groups: string;
    clusters: string;
    deselectAll: string;
    selectClusters: string;
    refreshing: string;
    favorites: string;
    schemaRegistry: string;
    history: string;
    topicsLabel: string;
    consumerGroupsLabel: string;
    viewClusterTopics: string;
  };
  favorites: {
    title: string;
    description: string;
    add: string;
    remove: string;
    added: string;
    removed: string;
    selectGroup: string;
    noGroups: string;
    createGroup: string;
    addGroup: string;
    editGroup: string;
    groupName: string;
    groupNamePlaceholder: string;
    groupDescription: string;
    groupDescPlaceholder: string;
    sortOrder: string;
    sortOrderPlaceholder: string;
    empty: string;
    emptyHint: string;
    noItems: string;
    noSearchResults: string;
    searchPlaceholder: string;
    editFavorite: string;
    favoriteDescription: string;
    favoriteDescPlaceholder: string;
    confirmDeleteGroup: string;
    confirmDeleteFavorite: string;
    createGroupHint: string;
    groupCreated: string;
    remark: string;
    remarkPlaceholder: string;
    group: string;
  };
  history: {
    title: string;
    description: string;
    empty: string;
    emptyHint: string;
    noSearchResults: string;
    searchPlaceholder: string;
    delete: string;
    clearAll: string;
    confirmClear: string;
    justNow: string;
    minutesAgo: string;
    hoursAgo: string;
    daysAgo: string;
  };
  sentMessageHistory: {
    title: string;
    description: string;
    empty: string;
    emptyHint: string;
    noSearchResults: string;
    searchPlaceholder: string;
    delete: string;
    clearAll: string;
    confirmClear: string;
    justNow: string;
    minutesAgo: string;
    hoursAgo: string;
    daysAgo: string;
  };
  schemaRegistry: {
    title: string;
    description: string;
    configTitle: string;
    registryUrl: string;
    registryUrlPlaceholder: string;
    authentication: string;
    username: string;
    password: string;
    testConnection: string;
    save: string;
    delete: string;
    subjects: string;
    noSubjects: string;
    versions: string;
    schemaType: string;
    compatibilityLevel: string;
    backward: string;
    forward: string;
    full: string;
    none: string;
    testCompatibility: string;
    compatible: string;
    incompatible: string;
    registerSchema: string;
    schemaContent: string;
    schemaContentPlaceholder: string;
    registerSuccess: string;
    compatibilityTestSuccess: string;
    compatibilityTestFailed: string;
    configNotSet: string;
    fetchFailed: string;
    deleteConfirm: string;
    version: string;
    latestVersion: string;
    viewSchema: string;
    deleteSchema: string;
    connectionSuccess: string;
    connectionFailed: string;
    selectCluster: string;
    selectClusterDesc: string;
  };
  tour: {
    prevStep: string;
    nextStep: string;
    finishTour: string;
    globalSidebar: { title: string; desc: string };
    globalSearch: { title: string; desc: string };
    globalShare: { title: string; desc: string };
    globalLang: { title: string; desc: string };
    globalTheme: { title: string; desc: string };
    globalSettings: { title: string; desc: string };
    clustersAdd: { title: string; desc: string };
    clustersGroup: { title: string; desc: string };
    clustersCard: { title: string; desc: string };
    topicsCreate: { title: string; desc: string };
    topicsList: { title: string; desc: string };
    topicsActions: { title: string; desc: string };
    messagesSearch: { title: string; desc: string };
    messagesBack: { title: string; desc: string };
    messagesPartition: { title: string; desc: string };
    messagesMode: { title: string; desc: string };
    messagesTimeFilter: { title: string; desc: string };
    messagesTimePresets: { title: string; desc: string };
    messagesCount: { title: string; desc: string };
    messagesQuery: { title: string; desc: string };
    messagesSend: { title: string; desc: string };
    messagesExport: { title: string; desc: string };
    messagesMore: { title: string; desc: string };
    messagesTableHeader: { title: string; desc: string };
    messagesTableRow: { title: string; desc: string };
    messagesDetailPanel: { title: string; desc: string };
    messagesValueViewFormat: { title: string; desc: string };
    consumerGroupsList: { title: string; desc: string };
    consumerGroupsRefresh: { title: string; desc: string };
    consumerGroupsDetail: { title: string; desc: string };
    favoritesList: { title: string; desc: string };
    favoritesAdd: { title: string; desc: string };
    settingsNav: { title: string; desc: string };
    settingsSections: { title: string; desc: string };
    schemaRegistryConfig: { title: string; desc: string };
    schemaRegistryActions: { title: string; desc: string };
    schemaRegistryTitle: { title: string; desc: string };
    schemaRegistryNoCluster: { title: string; desc: string };
    schemaRegistryNotConfig: { title: string; desc: string };
    schemaRegistryConfigCard: { title: string; desc: string };
    schemaRegistryConfigActions: { title: string; desc: string };
    schemaRegistrySubjectsCard: { title: string; desc: string };
    schemaRegistrySubjectsTable: { title: string; desc: string };
    schemaRegistryClusterDialog: { title: string; desc: string };
    schemaRegistryConfigDialog: { title: string; desc: string };
    schemaRegistrySchemaDialog: { title: string; desc: string };
    schemaRegistryRegisterDialog: { title: string; desc: string };
    sidebarViewSwitcher: { title: string; desc: string };
    sidebarClustersBtn: { title: string; desc: string };
    sidebarFavoritesBtn: { title: string; desc: string };
    sidebarSchemaBtn: { title: string; desc: string };
    sidebarHistoryBtn: { title: string; desc: string };
    sidebarClusterSelector: { title: string; desc: string };
    sidebarRefresh: { title: string; desc: string };
    sidebarSearch: { title: string; desc: string };
    sidebarTopicList: { title: string; desc: string };
    sidebarConsumerGroupList: { title: string; desc: string };
    sidebarTopicName: { title: string; desc: string };
    sidebarConsumerGroupName: { title: string; desc: string };
    sidebarHealthDot: { title: string; desc: string };
    sidebarClusterBadge: { title: string; desc: string };
    treeCollapseBtn: { title: string; desc: string };
    treeClustersBtn: { title: string; desc: string };
    treeFavoritesBtn: { title: string; desc: string };
    treeSchemaBtn: { title: string; desc: string };
    treeHistoryBtn: { title: string; desc: string };
    treeGroupSelector: { title: string; desc: string };
    treeClusterNode: { title: string; desc: string };
    treeClusterName: { title: string; desc: string };
    treeClusterHealthDot: { title: string; desc: string };
    treeTopicsFolder: { title: string; desc: string };
    treeTopicSearch: { title: string; desc: string };
    treeTopicName: { title: string; desc: string };
    treeTopicsRefresh: { title: string; desc: string };
    treeConsumerGroupsFolder: { title: string; desc: string };
    treeConsumerGroupSearch: { title: string; desc: string };
    treeConsumerGroupName: { title: string; desc: string };
    treeConsumerGroupsRefresh: { title: string; desc: string };
    tcgTitle: { title: string; desc: string };
    tcgDataNotice: { title: string; desc: string };
    tcgRefresh: { title: string; desc: string };
    tcgTable: { title: string; desc: string };
    cgBack: { title: string; desc: string };
    cgGroupName: { title: string; desc: string };
    cgState: { title: string; desc: string };
    cgRefresh: { title: string; desc: string };
    cgActions: { title: string; desc: string };
    cgList: { title: string; desc: string };
    cgTableTopic: { title: string; desc: string };
    cgTablePartition: { title: string; desc: string };
    cgTableStartOffset: { title: string; desc: string };
    cgTableEndOffset: { title: string; desc: string };
    cgTableCommittedOffset: { title: string; desc: string };
    cgTableLag: { title: string; desc: string };
    cgTableLastCommit: { title: string; desc: string };
    cgTableRow: { title: string; desc: string };
    cgEmptyState: { title: string; desc: string };
    cgResetDialog: { title: string; desc: string };
    cgNoGroup: { title: string; desc: string };
    clustersTitle: { title: string; desc: string };
    clustersManageGroups: { title: string; desc: string };
    clustersStatus: { title: string; desc: string };
    clustersEditBtn: { title: string; desc: string };
    clustersDeleteBtn: { title: string; desc: string };
    clustersTestBtn: { title: string; desc: string };
    clustersReconnectBtn: { title: string; desc: string };
    clustersTopicsBtn: { title: string; desc: string };
    clustersRefreshTopicsBtn: { title: string; desc: string };
    clustersAddTopicBtn: { title: string; desc: string };
    clustersEmpty: { title: string; desc: string };
    clustersError: { title: string; desc: string };
    clustersModal: { title: string; desc: string };
    clustersModalName: { title: string; desc: string };
    clustersModalBrokers: { title: string; desc: string };
    clustersModalTest: { title: string; desc: string };
    clustersModalSubmit: { title: string; desc: string };
    settingsTitle: { title: string; desc: string };
    settingsLanguage: { title: string; desc: string };
    settingsTheme: { title: string; desc: string };
    settingsSidebarMode: { title: string; desc: string };
    settingsVersion: { title: string; desc: string };
    settingsVersionBadge: { title: string; desc: string };
    settingsCheckUpdate: { title: string; desc: string };
    settingsImportExport: { title: string; desc: string };
    settingsJsonHighlight: { title: string; desc: string };
    favTitle: { title: string; desc: string };
    favGroupCard: { title: string; desc: string };
    favEditGroup: { title: string; desc: string };
    favDeleteGroup: { title: string; desc: string };
    favGroupSearch: { title: string; desc: string };
    favItemRow: { title: string; desc: string };
    favEditItem: { title: string; desc: string };
    favDeleteItem: { title: string; desc: string };
    favEmptyState: { title: string; desc: string };
    favGroupModal: { title: string; desc: string };
    favFavoriteModal: { title: string; desc: string };
  };
}

export const translations: Record<Language, Translation> = {
  zh: {
    nav: {
      clusters: '集群',
      favorites: '收藏',
      topics: '主题',
      messages: '消息',
      settings: '设置',
    },
    tree: {
      backToClusters: '返回集群列表',
      collapseAll: '全部收起',
      clusters: '集群',
      topicFavorites: '主题收藏',
      schemaRegistry: 'Schema Registry',
      refreshConsumerGroups: '刷新 Consumer Groups',
    },
    common: {
      loading: '加载中...',
      error: '错误',
      success: '成功',
      save: '保存',
      cancel: '取消',
      delete: '删除',
      edit: '编辑',
      create: '创建',
      refresh: '刷新',
      search: '搜索',
      noData: '暂无数据',
      actions: '操作',
      name: '名称',
      status: '状态',
      connected: '已连接',
      disconnected: '未连接',
      all: '全部',
      filter: '过滤...',
      back: '返回',
      ready: '就绪',
      optional: '可选',
      confirm: '确认',
      close: '关闭',
      confirmDelete: '确定要删除',
      apply: '应用',
      clear: '清除',
      cannotBeGreaterThan: '不能大于',
      unknown: '未知',
      copy: '复制',
      copyFailed: '复制失败',
      failed: '失败',
      hide: '隐藏',
      viewDetails: '查看详情',
      view: '查看',
    },
    clusters: {
      title: '集群',
      description: '管理 Kafka 集群连接',
      addCluster: '添加集群',
      editCluster: '编辑集群',
      createCluster: '创建集群',
      clusterName: '集群名称',
      brokers: 'Broker 地址',
      requestTimeout: '请求超时 (ms)',
      operationTimeout: '操作超时 (ms)',
      removeCluster: '移除集群',
      viewBrokers: '查看 Brokers',
      viewTopics: '查看 Topics',
      createTopic: '创建 Topic',
      connectionError: '连接错误',
      retry: '重试',
      brokersLabel: 'Brokers',
      timeoutsLabel: '超时设置',
      confirmDelete: '确定要删除集群 "{name}"',
      disconnectConfirm: '确定要断开集群 "{name}" 的连接',
      reconnectFailed: '重连失败',
      fetchFailed: '获取失败',
      connectedLabel: '已连接',
      disconnectedLabel: '未连接',
      connectingLabel: '连接中',
      updated: '集群已更新',
      created: '集群已创建',
      connected: '连接成功',
      refreshed: 'Topic 刷新成功',
      topicsRefreshed: '已刷新集群 Topic',
      refreshingBg: '已在后台同步 Topic',
      reconnected: '重连成功',
      topics: '主题',
      partitions: '分区',
      viewTopicsLink: '查看主题',
      group: '分组',
      noGroup: '无分组',
      addGroup: '添加分组',
      testConnection: '测试连接',
      testingConnection: '测试中...',
      connectionSuccess: '连接成功',
      connectionFailed: '连接失败',
      noDescription: '无描述',
      manageGroups: '管理分组',
      editGroup: '编辑分组',
      groupName: '分组名称',
      groupNamePlaceholder: '请输入分组名称',
      groupDescription: '分组描述',
      groupDescPlaceholder: '请输入分组描述（可选）',
      confirmDeleteGroup: '确定要删除分组 "{name}" 吗？删除后，该分组下的所有集群将变为无分组状态。',
      deleteGroupTitle: '删除分组',
      test: '测试',
      reconnect: '重连',
      disconnect: '断开连接',
      disconnectedSuccess: '集群已断开连接',
      reconnectSuccess: '重连成功',
      groupUpdated: '分组已更新',
      groupCreated: '分组已创建',
      groupDeleted: '分组已删除',
      clusterDeleted: '集群已删除',
      scrollLeft: '向左滚动',
      scrollRight: '向右滚动',
      editClusterTitle: '编辑集群',
      createClusterTitle: '创建集群',
      newCluster: '新集群',
      brokersHelp: '逗号分隔的 broker 地址列表',
      createdDate: '创建时间',
      unknown: '未知',
      refreshTopics: '刷新 Topic',
      refreshFailed: '刷新失败',
      validationNameRequired: '集群名称不能为空',
      validationBrokersRequired: 'Broker 地址不能为空',
      validationNameInvalid: '集群名称只能包含字母、数字、中文、连字符和下划线',
      validationNameTooLong: '集群名称不能超过 15 个字符',
      validationBrokersInvalid: 'Broker 地址不能包含空格、引号或逗号',
      connectionTestSuccess: '连接测试成功',
      connectionTestFailed: '连接测试失败',
      clusterStatusRefreshed: '集群状态已刷新',
      clusterConnectionIssue: '集群连接问题',
      reconnectSuccessToast: '重连成功',
      clusterDeletedToast: '集群已删除',
      clusters: '集群',
    },
    topics: {
      title: '主题',
      description: '管理 Kafka 主题',
      createTopic: '创建主题',
      topicName: '主题名称',
      topicNamePlaceholder: '请输入主题名称',
      topicNameValidation: '只能包含字母、数字、点号、下划线和短横线',
      partitionCount: '分区数',
      numPartitions: '分区数量',
      numPartitionsHelp: '分区数量范围：1-100',
      replicationFactor: '副本因子',
      replicationFactorHelp: '副本因子范围：1-10',
      advancedOptions: '高级选项',
      cleanupPolicy: '清理策略',
      retentionMs: '保留时间 (ms)',
      retentionBytes: '保留大小 (bytes)',
      segmentBytes: '段大小 (bytes)',
      retentionMsPlaceholder: '604800000 (7 天)',
      retentionBytesPlaceholder: '-1 (无限制)',
      segmentBytesPlaceholder: '1073741824 (1GB)',
      viewMessages: '查看消息',
      viewDetails: '查看详情',
      viewPartitions: '查看分区',
      sendMessage: '发送消息',
      deleteTopic: '删除主题',
      exportData: '导出数据',
      topicDetails: '主题详情',
      partitions: '分区',
      settings: '设置',
      refreshed: 'Topic 已刷新',
      refreshingBg: '已在后台同步 Topic',
      confirmDeleteTitle: '删除主题',
      confirmDeleteHint: '删除后无法恢复',
      confirmDeleteInput: '输入主题名称以确认',
      confirmDeleteMatchError: '输入的主题名称不匹配',
      copied: '已复制',
      copyFailed: '复制失败',
      deletedSuccess: '主题已删除',
      createdSuccess: '主题 "${name}" 创建成功',
      createFailed: '创建主题失败',
      validationSelectCluster: '请先选择一个集群',
      validationClusterIdRequired: '集群 ID 不能为空',
      validationTopicNameRequired: '主题名称不能为空',
      validationTopicNameTooLong: '主题名称不能超过 256 个字符',
      validationTopicNameInvalidChars: '主题名称不能包含空格、引号或逗号',
      validationTopicNameFormat: '主题名称只能包含字母、数字、点号、下划线和短横线',
      validationRetentionMs: 'retention.ms 必须是正数',
      validationRetentionBytes: 'retention.bytes 必须是数字（使用 -1 表示无限制）',
      validationSegmentBytes: 'segment.bytes 必须是正数',
      noSearchResults: '未找到匹配的 Topic',
      clearSearch: '清除搜索',
    },
    messages: {
      title: '消息',
      description: '查看和发送 Kafka 消息',
      sendMessage: '发送消息',
      partition: '分区',
      key: '键',
      value: '值',
      targetPartition: '目标分区',
      optional: '可选',
      required: '必填',
      messageSent: '消息已发送',
      offset: '偏移量',
      send: '发送',
      sendAndNew: '发送并新建',
      sending: '发送中...',
      allPartitions: '全部分区',
      fetch: '获取',
      filter: '过滤',
      formatJson: '格式化 JSON',
      fetchMode: '获取模式',
      oldest: '最早',
      newest: '最新',
      maxMessages: '最大消息数',
      perPartition: '每分区',
      startTime: '开始时间',
      endTime: '结束时间',
      timeRange: '时间范围',
      timeRangeFilter: '高级筛选',
      clear: '清除',
      selectTopic: '选择主题',
      selectMessage: '选择消息',
      noTopicSelected: '未选择主题',
      noMessages: '暂无消息',
      offsetLabel: '偏移量',
      partitionLabel: '分区',
      timestampLabel: '时间戳',
      sizeLabel: '大小',
      viewAs: '查看方式',
      json: 'JSON',
      raw: '原始',
      hex: '十六进制',
      copied: '已复制',
      messages: '消息',
      time: '耗时',
      selectedOffset: '选中偏移量',
      ready: '就绪',
      query: '查询',
      stop: '停止',
      exportMessages: '导出消息',
      messageDetail: '消息详情',
      copyValue: '复制 Value',
      copyKey: '复制 Key',
      valuePlaceholder: '搜索消息内容...',
      searchPlaceholder: '搜索键或值...',
      elapsedTime: '耗时',
      totalMessages: '共',
      actions: '操作',
      close: '关闭',
      cancel: '取消',
      continue: '发送并继续',
      keyOptional: '可选',
      valueRequired: '必填',
      partitionLabel2: '分区',
      queryFailed: '查询失败',
      fetchFailed: '获取失败',
      loading: '加载中...',
      clearSort: '清除排序',
      hide: '收起',
      show: '展开',
      valid: '有效',
      exportSuccess: '消息已导出',
      exportFailed: '导出失败',
      receiving: '接收中',
      recent5Minutes: '5~10 分钟前',
      recent15Minutes: '15~30 分钟前',
      recent30Minutes: '30~60 分钟前',
      recent1Hour: '1~2 小时前',
      recent1Day: '1~2 天前',
      topicLabel: '主题',
      cluster: '集群',
      minutes: '分',
      hour: '时',
      day: '天',
      queryTimeout: '查询超时，请重试',
      exportTip: '查询数量为每分区的消息数量，条件过滤在查询数据后进行',
      confirmDeleteTopic: '确定要删除主题 "{topic}" 吗？此操作不可撤销',
      deleteTopic: '删除主题',
      topicDeleted: '主题已删除',
    },
    consumerGroups: {
      title: '消费者组',
      description: '管理 Kafka 消费者组',
      groupName: '组名称',
      topics: '主题',
      state: '状态',
      partitions: '分区',
      offset: '偏移量',
      lag: '延迟',
      start: '起始',
      end: '结束',
      resetOffset: '重置偏移量',
      resetOffsetToEarliest: '重置到最早',
      resetOffsetToLatest: '重置到最新',
      resetOffsetToTimestamp: '重置到指定时间',
      timestamp: '时间戳',
      refreshOffsets: '刷新偏移量',
      offsetsRefreshed: '偏移量已刷新',
      offsetResetSuccess: '偏移量已重置',
      confirmResetOffset: '确认重置偏移量？',
      confirmResetOffsetToEarliest: '确认重置偏移量到最早？',
      confirmResetOffsetToLatest: '确认重置偏移量到最新？',
      confirmResetOffsetToTimestamp: '确认重置偏移量到指定时间？',
      noData: '暂无消费者组',
      emptyHelp: '点击上方刷新按钮从 Kafka 集群同步消费者组',
      refreshed: '消费者组已刷新',
      refreshingBg: '已在后台同步消费者组',
      deleteGroup: '删除消费者组',
      deleted: '消费者组已删除',
      offsets: '偏移量详情',
      topic: '主题',
      partition: '分区',
      startOffset: '起始偏移',
      endOffset: '结束偏移',
      committedOffset: '已提交偏移',
      lastCommit: '最后提交时间',
      selectTopic: '选择主题',
      resetTo: '重置到',
      earliest: '最早 (earliest)',
      latest: '最新 (latest)',
      specificOffset: '指定偏移 (offset)',
      offsetValue: '偏移值',
      timestampValue: '时间戳',
      noOffsets: '暂无偏移量数据',
      selectFromNav: '请从左侧导航栏选择一个消费者组查看详情',
      groupNamePrefix: '消费者组：',
    },
    topicConsumerGroups: {
      title: 'Topic 消费者组',
      description: '查看消费此 Topic 的所有 Consumer Groups',
      noData: '暂无消费此 Topic 的 Consumer Groups',
      groupName: 'Consumer Group 名称',
      state: '状态',
      partitions: '分区数',
      lag: '延迟',
      viewDetails: '查看详情',
      refresh: '刷新',
      refreshAllConsumerGroup: '刷新集群 Consumer Group',
      refreshed: 'Consumer Groups 已刷新',
      dataNoticeTitle: '数据来源说明',
      dataNotice: '此页面显示的 Consumer Groups 来自数据库历史记录，offset 数据实时从 Kafka 获取。',
      topicNamePrefix: 'Topic：',
    },
    settings: {
      title: '设置',
      description: '管理全局设置',
      language: '语言',
      theme: '主题',
      languageZh: '中文',
      languageEn: 'English',
      selectLanguage: '选择语言',
      systemSettings: '系统设置',
      sidebarMode: '侧边栏模式',
      treeMode: '树形模式',
      flatMode: '列表模式',
      treeModeDesc: '按集群分组显示主题',
      flatModeDesc: '平铺显示所有主题',
      systemTray: '系统托盘',
      systemTrayDesc: '开启后点击关闭按钮最小化到托盘，关闭后直接退出应用',
      systemTrayEnabled: '已开启系统托盘',
      systemTrayDisabled: '已关闭系统托盘',
      autoLaunch: '开机自启动',
      autoLaunchDesc: '开机自动启动 Kafka Manager',
      autoLaunchEnabled: '已开启开机自启动',
      autoLaunchDisabled: '已关闭开机自启动',
      autoLaunchFailed: '设置开机自启动失败',
      version: '版本信息',
      versionDesc: 'Kafka Manager 当前版本',
      currentVersion: '当前版本',
      author: '作者',
      help: '帮助',
      themeDesc: '切换浅色或深色模式',
      lightMode: '浅色模式',
      darkMode: '深色模式',
      jsonHighlight: 'JSON 高亮',
      jsonHighlightDesc: '配置消息详情和发送消息弹框中的 JSON 高亮样式',
      selectTemplate: '选择模板',
      preview: '预览效果',
      customTemplates: '自定义模板',
      addCustomTemplate: '添加自定义模板',
      templateName: '模板名称',
      templateDescription: '模板描述',
      templateStyle: '样式配置',
      saveTemplate: '保存模板',
      deleteTemplate: '删除模板',
      confirmDeleteTemplate: '确定要删除这个自定义模板吗？',
      templateFormat: '模板格式',
      builtInTemplates: '内置模板',
      importExport: '数据导入导出',
      importExportDesc: '导出或导入集群、收藏和历史记录',
      exportData: '导出数据',
      exporting: '导出中...',
      exportSuccess: '导出成功',
      importData: '导入数据',
      importing: '导入中...',
      importSuccess: '导入完成',
      operationInProgress: '已有其他导入/导出操作正在进行中，请等待完成后再试',
      importStarted: '导入已在后台启动，完成后会自动释放',
      viewLogs: '查看日志',
      appLogs: '应用日志',
      refreshLogs: '刷新日志',
      copyLogs: '复制日志',
      clearLogs: '清除日志',
      scrollToBottom: '滚动到底部',
      noLogs: '暂无日志',
      logsCopied: '已复制到剪贴板',
      logsCleared: '日志已清除',
      logsRefreshed: '刷新成功',
      feedback: '意见反馈',
      feedbackDesc: '向我们提供您的宝贵意见',
      feedbackPlaceholder: '请输入您的意见或建议（最多 2000 字）',
      feedbackSubmit: '提交反馈',
      feedbackSubmitting: '提交中...',
      feedbackSuccess: '反馈提交成功，感谢您的意见！',
      feedbackFailed: '反馈提交失败',
      feedbackMaxLength: '反馈内容不能超过 2000 字',
      feedbackConnectionFailed: '无法连接服务器，反馈功能暂不可用',
      feedbackSystemInfo: '系统信息',
      feedbackDocLink: 'Kafka Manager 使用说明文档',
    },
    layout: {
      searchPlaceholder: '搜索主题... (Ctrl+K)',
      noTopicsFound: '未找到主题',
      settings: '设置',
      help: '帮助',
      checkForUpdates: '检查更新',
      confirmDeleteCluster: '确定要删除集群 "{cluster}" 吗？',
      confirmDeleteTopic: '确定要删除主题 "{topic}" 吗？',
      clusterNotFound: '集群不存在',
      topicNotFound: '主题不存在',
      refreshFailed: '刷新失败',
      refreshCancelled: '刷新已取消',
      shareVersion: '分享安装包',
    shareCopied: '安装包已复制到您电脑的下载目录，请打开文件管理器查看',
    },
    update: {
      available: '发现新版本',
      currentVersion: '当前版本',
      newVersion: '新版本',
      releaseNotes: '更新说明',
      downloading: '正在下载...',
      installing: '正在安装...',
      installed: '已安装',
      installFailed: '安装失败',
      networkError: '网络请求失败，请检查网络连接后重试',
      updateAndRestart: '下载并安装',
      checkCompleteNoUpdate: '已是最新版本',
      checkFailed: '检查更新失败',
      downloadComplete: '下载完成，正在打开安装包...',
      browserNotSupported: '浏览器环境不支持检查更新',
      checkForUpdates: '检查更新',
      checking: '检查中...',
      checkNow: '立即检查',
      downloadingInBackground: '正在后台下载更新...',
      minimizeHint: '关闭弹窗后下载将在后台继续',
      minimizeModal: '隐藏窗口',
      rateLimitExceeded: '访问受限，请稍后重试',
      downloadInProgress: '下载正在进行中，请稍候...',
      downloadFailed: '下载失败，请检查网络连接后重试',
    },
    toast: {
      error: '错误',
      success: '成功',
      warning: '警告',
      info: '提示',
      copySuccess: '复制成功',
      copyFailed: '复制失败',
      operationFailed: '操作失败',
      clusterNotFound: '集群不存在',
      networkError: '网络错误',
      invalidFormat: '格式无效',
      skipped: '跳过',
    },
    topicContextMenu: {
      viewMessages: '查看消息',
      viewDetails: '查看详情',
      viewPartitions: '查看分区',
      sendMessage: '发送消息',
      exportData: '导出数据',
      deleteTopic: '删除主题',
    },
    partitionContextMenu: {
      viewMessages: '查看消息',
      sendMessage: '发送消息',
    },
    mainLayout: {
      clustersSelected: '个集群已选中',
      refreshHealth: '刷新集群健康状态',
      footerText: 'Kafka Manager v0.1.0 - 基于 Vue 3 + Tailwind CSS + DaisyUI',
      multiCluster: '多集群',
      clustersLabel: '集群',
      selectAll: '全选',
      clearSelection: '清除',
      manageClusters: '管理集群',
      topics: '主题',
      messages: '消息',
      dragToResize: '拖动以调整侧边栏宽度',
    },
    navigator: {
      allClusters: '所有集群',
      byGroup: '按分组',
      selectGroupsAndClusters: '选择分组/集群',
      selected: '已选择',
      selectCluster: '选择集群',
      groups: '分组',
      clusters: '集群',
      deselectAll: '取消全选',
      selectClusters: '选择集群',
      refreshing: '正在刷新...',
      favorites: 'Topic 收藏',
      schemaRegistry: 'Schema Registry',
      history: '浏览历史',
      topicsLabel: '主题',
      consumerGroupsLabel: '消费者组',
      viewClusterTopics: '查看此集群的所有主题',
    },
    favorites: {
      title: 'Topic 收藏',
      description: '管理您收藏的 Topic，支持分组管理',
      add: '收藏',
      remove: '取消收藏',
      added: '已添加到收藏',
      removed: '已取消收藏',
      selectGroup: '选择收藏分组',
      noGroups: '暂无分组',
      createGroup: '创建分组',
      addGroup: '新建分组',
      editGroup: '编辑分组',
      groupName: '分组名称',
      groupNamePlaceholder: '请输入分组名称',
      groupDescription: '分组描述',
      groupDescPlaceholder: '请输入分组描述（可选）',
      sortOrder: '排序',
      sortOrderPlaceholder: '数字越小越靠前',
      empty: '暂无收藏分组',
      emptyHint: '点击右上角创建分组',
      noItems: '该分组暂无收藏',
      noSearchResults: '无匹配的收藏',
      searchPlaceholder: '搜索 Topic 名、备注...',
      editFavorite: '编辑收藏',
      favoriteDescription: '描述',
      favoriteDescPlaceholder: '请输入描述（可选）',
      confirmDeleteGroup: '确定要删除这个分组吗？分组内的收藏也会被删除。',
      confirmDeleteFavorite: '确定要删除这个收藏吗？',
      createGroupHint: '请先在收藏管理中创建分组',
      groupCreated: '分组创建成功',
      remark: '备注',
      remarkPlaceholder: '添加备注（可选）',
      group: '个分组',
    },
    history: {
      title: '浏览历史',
      description: '自动记录您浏览过的 Topic',
      empty: '暂无浏览历史',
      emptyHint: '浏览 Topic 时会自动记录到这里',
      noSearchResults: '无匹配的历史记录',
      searchPlaceholder: '搜索 Topic...',
      delete: '删除记录',
      clearAll: '清空历史',
      confirmClear: '确定要清空所有历史记录吗？',
      justNow: '刚刚',
      minutesAgo: '分钟前',
      hoursAgo: '小时前',
      daysAgo: '天前',
    },
    sentMessageHistory: {
      title: '发送历史',
      description: '自动记录您发送过的消息',
      empty: '暂无发送历史',
      emptyHint: '发送消息时会自动记录到这里',
      noSearchResults: '无匹配的历史记录',
      searchPlaceholder: '搜索 Topic...',
      delete: '删除记录',
      clearAll: '清空历史',
      confirmClear: '确定要清空所有发送历史吗？',
      justNow: '刚刚',
      minutesAgo: '分钟前',
      hoursAgo: '小时前',
      daysAgo: '天前',
    },
    schemaRegistry: {
      title: 'Schema Registry',
      description: '管理 Schema Registry 配置和 Schema',
      configTitle: 'Schema Registry 配置',
      registryUrl: 'Registry URL',
      registryUrlPlaceholder: 'http://localhost:8081',
      authentication: '认证信息',
      username: '用户名',
      password: '密码',
      testConnection: '测试连接',
      save: '保存配置',
      delete: '删除配置',
      subjects: 'Subjects',
      noSubjects: '暂无 Subjects',
      versions: '版本',
      schemaType: 'Schema 类型',
      compatibilityLevel: '兼容性级别',
      backward: '向后兼容',
      forward: '向前兼容',
      full: '完全兼容',
      none: '无兼容',
      testCompatibility: '测试兼容性',
      compatible: '兼容',
      incompatible: '不兼容',
      registerSchema: '注册 Schema',
      schemaContent: 'Schema 内容',
      schemaContentPlaceholder: '粘贴 Schema JSON 内容...',
      registerSuccess: 'Schema 注册成功',
      compatibilityTestSuccess: '兼容性测试成功',
      compatibilityTestFailed: '兼容性测试失败',
      configNotSet: '未配置 Schema Registry',
      fetchFailed: '获取失败',
      deleteConfirm: '确定要删除此 Subject 及其所有版本？',
      version: '版本',
      latestVersion: '最新版本',
      viewSchema: '查看 Schema',
      deleteSchema: '删除 Subject',
      connectionSuccess: '连接成功',
      connectionFailed: '连接失败',
      selectCluster: '选择集群',
      selectClusterDesc: '选择要管理 Schema Registry 的 Kafka 集群',
    },
    tour: {
      prevStep: '上一步',
      nextStep: '下一步',
      finishTour: '完成',
      globalSidebar: { title: '导航栏', desc: '左侧导航栏可快速切换集群、主题、收藏和消费者组，点击项目即可查看详情。' },
      globalSearch: { title: '主题搜索', desc: '在顶部搜索框快速搜索所有集群中的主题名称。' },
      globalShare: { title: '分享安装包', desc: '点击此按钮可将安装包复制到下载目录，或跳转至 GitHub 发布页。' },
      globalLang: { title: '切换语言', desc: '在中文和英文之间切换界面语言。' },
      globalTheme: { title: '切换主题', desc: '在浅色和深色主题之间切换。' },
      globalSettings: { title: '设置', desc: '进入设置页面，管理语言、主题、侧边栏模式等全局配置。' },
      clustersAdd: { title: '添加集群', desc: '点击此按钮创建新的 Kafka 集群连接，填入名称和 Broker 地址即可。' },
      clustersGroup: { title: '分组筛选', desc: '按分组快速筛选要查看的集群。' },
      clustersCard: { title: '集群卡片', desc: '每张卡片显示集群连接状态，右键或点击操作按钮可刷新、编辑、删除或断开连接。' },
      topicsCreate: { title: '创建主题', desc: '点击此按钮创建新的 Kafka 主题，设置分区数和副本因子等参数。' },
      topicsList: { title: '主题列表', desc: '列出当前集群中的所有主题，支持搜索和排序。' },
      topicsActions: { title: '主题操作', desc: '查看消息、查看详情、发送消息或删除主题等操作入口。' },
      messagesSearch: { title: '消息搜索', desc: '在消息内容中搜索关键词。' },
      messagesBack: { title: '返回', desc: '返回上一页。' },
      messagesPartition: { title: '分区选择', desc: '选择要查询的特定分区或查询所有分区。' },
      messagesMode: { title: '查询模式', desc: '选择从最新消息或最早消息开始查询。' },
      messagesTimeFilter: { title: '高级筛选', desc: '按时间范围过滤消息，支持手动输入时间或选择快捷时间范围。' },
      messagesTimePresets: { title: '快捷时间范围', desc: '快速选择最近 5 分钟、15 分钟、30 分钟、1 小时或 1 天的消息范围。' },
      messagesCount: { title: '查询数量', desc: '设置每次查询每个分区返回的最大消息数量。' },
      messagesQuery: { title: '执行查询', desc: '从 Kafka 查询消息，每分区数据返回后，再按查询条件过滤数据并展示结果。' },
      messagesSend: { title: '发送消息', desc: '打开消息发送弹窗，可向当前主题发送新消息。' },
      messagesExport: { title: '导出消息', desc: '将查询到的消息导出为 JSON 文件保存到本地。' },
      messagesMore: { title: '更多操作', desc: '查看发送历史、消费者组和删除主题等更多操作。' },
      messagesTableHeader: { title: '消息列表表头', desc: '显示分区、Offset、时间戳、Key、Value 等列名，可点击时间戳列排序，拖动列间分隔线调整列宽。' },
      messagesTableRow: { title: '消息行', desc: '每条消息的预览信息，点击可查看详情，支持键盘上下键切换选中。' },
      messagesDetailPanel: { title: '消息详情', desc: '显示选中消息的完整内容，支持 Ctrl+F 搜索、Ctrl+A 全选、JSON 高亮，拖动顶部手柄可调整面板高度。' },
      messagesValueViewFormat: { title: '查看格式', desc: '切换 Value 的显示格式：JSON（自动格式化）、Raw（原始文本）、Hex（十六进制）。' },
      consumerGroupsList: { title: '消费者组列表', desc: '显示所有消费者组及其状态、延迟等信息。' },
      consumerGroupsRefresh: { title: '刷新', desc: '从 Kafka 集群同步最新的消费者组状态。' },
      consumerGroupsDetail: { title: '消费者组详情', desc: '点击消费者组名称可查看详情，包括分区分配和偏移量。' },
      favoritesList: { title: '收藏列表', desc: '显示收藏的主题列表，点击可快速跳转到消息页面。' },
      favoritesAdd: { title: '添加收藏', desc: '在主题列表或消息页面可将主题添加到收藏。' },
      settingsNav: { title: '设置分类', desc: '按类别浏览和修改全局设置。' },
      settingsSections: { title: '设置项', desc: '管理语言、主题、侧边栏模式、系统托盘、开机自启等配置。' },
      schemaRegistryConfig: { title: '配置区域', desc: '配置 Schema Registry 的连接地址和认证信息。' },
      schemaRegistryActions: { title: '操作按钮', desc: '测试连接、保存或删除 Schema Registry 配置。' },
      schemaRegistryTitle: { title: '页面标题', desc: 'Schema Registry 管理入口，配置和管理 Schema 注册表。' },
      schemaRegistryNoCluster: { title: '未选择集群', desc: '请先选择一个已连接的 Kafka 集群来配置 Schema Registry。' },
      schemaRegistryNotConfig: { title: '未配置', desc: '当前集群尚未配置 Schema Registry，点击按钮添加配置。' },
      schemaRegistryConfigCard: { title: '配置信息卡片', desc: '显示当前 Schema Registry 连接信息，包括 URL、用户名和连接状态。' },
      schemaRegistryConfigActions: { title: '配置操作', desc: '编辑配置或删除 Schema Registry 连接。' },
      schemaRegistrySubjectsCard: { title: 'Subjects 列表', desc: '管理所有已注册的 Schema Subject，支持查看、注册和删除。' },
      schemaRegistrySubjectsTable: { title: 'Subject 表格', desc: '显示每个 Subject 的版本、类型、兼容性等信息。' },
      schemaRegistryClusterDialog: { title: '集群选择对话框', desc: '选择要管理 Schema Registry 的 Kafka 集群。' },
      schemaRegistryConfigDialog: { title: '配置对话框', desc: '设置 Schema Registry 的连接地址和认证信息。' },
      schemaRegistrySchemaDialog: { title: 'Schema 详情对话框', desc: '查看 Schema 的完整内容和元数据信息。' },
      schemaRegistryRegisterDialog: { title: '注册 Schema 对话框', desc: '注册新的 Schema 到注册表，支持 AVRO、Protobuf、JSON 格式。' },
      sidebarViewSwitcher: { title: '视图切换', desc: '在主题（Topics）和消费者组（Consumer Groups）之间切换左侧导航栏的内容。' },
      sidebarClustersBtn: { title: '集群管理', desc: '跳转到集群管理页面，添加、编辑或删除集群连接。' },
      sidebarFavoritesBtn: { title: '收藏管理', desc: '进入收藏页面，管理已收藏的主题，支持分组管理。' },
      sidebarSchemaBtn: { title: 'Schema Registry', desc: '进入 Schema Registry 页面，配置和管理 Schema。' },
      sidebarHistoryBtn: { title: '浏览历史', desc: '查看最近浏览过的主题和消费者组记录。' },
      sidebarClusterSelector: { title: '集群筛选', desc: '点击打开集群选择面板，可以选择或取消选择要显示的集群。未选择任何集群时显示全部集群。' },
      sidebarRefresh: { title: '刷新数据', desc: '从 Kafka 集群同步最新的主题或消费者组数据。' },
      sidebarSearch: { title: '搜索', desc: '在当前集群的主题或消费者组中搜索名称，输入关键词后自动过滤。' },
      sidebarTopicList: { title: '主题列表', desc: '显示所有主题及其所属集群。点击任意主题即可跳转到消息页面查看详情。滚动到底部会自动加载更多。' },
      sidebarTopicName: { title: '主题名称', desc: '点击主题名称即可跳转到该主题的消息页面，查看最新消息详情。' },
      sidebarConsumerGroupList: { title: '消费者组列表', desc: '显示所有消费者组及其所属集群。点击任意消费者组即可跳转到详情页面查看分区分配和偏移量。' },
      sidebarConsumerGroupName: { title: '消费者组名称', desc: '点击消费者组名称即可跳转到该消费者组的详情页面，查看分区分配和偏移量信息。' },
      sidebarHealthDot: { title: '健康指示器', desc: '每个项目左侧的圆点表示集群连接状态：绿色表示已连接，红色表示连接失败，黄色表示状态未知。' },
      sidebarClusterBadge: { title: '集群标签', desc: '每个项目右侧的标签显示其所属的集群名称，方便区分多集群中的同名主题或消费者组。' },
      treeCollapseBtn: { title: '全部收起', desc: '点击收起所有展开的集群和文件夹。' },
      treeClustersBtn: { title: '集群管理', desc: '跳转到集群管理页面，添加、编辑或删除集群连接。' },
      treeFavoritesBtn: { title: '收藏管理', desc: '进入收藏页面，管理已收藏的主题。' },
      treeSchemaBtn: { title: 'Schema Registry', desc: '进入 Schema Registry 页面，配置和管理 Schema。' },
      treeHistoryBtn: { title: '浏览历史', desc: '查看最近浏览过的主题记录。' },
      treeGroupSelector: { title: '分组筛选', desc: '按分组快速筛选要查看的集群。' },
      treeClusterNode: { title: '集群节点', desc: '集群名称左侧的圆点表示连接状态。双击集群节点可展开查看其下的 Topics 和 Consumer Groups。' },
      treeClusterName: { title: '集群名称', desc: '集群的名称显示在节点标题位置，标识当前 Kafka 集群。' },
      treeClusterHealthDot: { title: '集群健康指示器', desc: '集群名称左侧的圆点表示连接状态：绿色表示已连接，红色表示连接失败，黄色闪烁表示正在刷新。' },
      treeTopicsFolder: { title: 'Topics 文件夹', desc: '点击展开查看该集群下所有 Topic 列表，右侧的刷新按钮可从 Kafka 同步最新数据。' },
      treeTopicSearch: { title: 'Topic 搜索', desc: '在当前集群的 Topic 列表中搜索名称，输入后自动过滤不匹配的结果。' },
      treeTopicName: { title: 'Topic 名称', desc: '点击 Topic 名称可跳转到消息页面，查看该 Topic 的最新消息。' },
      treeTopicsRefresh: { title: '刷新 Topics', desc: '从 Kafka 集群同步该集群下的最新 Topic 列表。' },
      treeConsumerGroupsFolder: { title: 'Consumer Groups 文件夹', desc: '点击展开查看该集群下所有消费者组列表，右侧的刷新按钮可从 Kafka 同步最新数据。' },
      treeConsumerGroupSearch: { title: 'Consumer Group 搜索', desc: '在当前集群的消费者组中搜索名称，输入后自动过滤不匹配的结果。' },
      treeConsumerGroupName: { title: 'Consumer Group 名称', desc: '点击消费者组名称可跳转到详情页面，查看分区分配和偏移量信息。' },
      treeConsumerGroupsRefresh: { title: '刷新 Consumer Groups', desc: '从 Kafka 集群同步该集群下的最新消费者组列表。' },
      tcgTitle: { title: 'Topic 消费者组', desc: '此页面显示消费指定 Topic 的所有消费者组的偏移量信息。' },
      tcgDataNotice: { title: '数据来源提示', desc: '消费者组来自数据库历史记录，偏移量数据实时从 Kafka 获取。' },
      tcgRefresh: { title: '刷新数据', desc: '重新获取最新的消费者组偏移量信息。' },
      tcgTable: { title: '偏移量表格', desc: '显示每个消费者组在各分区的起始偏移量、已提交偏移量和积压量。' },
      cgBack: { title: '返回', desc: '返回上一页。' },
      cgGroupName: { title: '消费者组名称', desc: '当前查看的消费者组名称。' },
      cgState: { title: '状态徽章', desc: '显示消费者组当前状态：Stable（稳定）、PreparingRebalance（准备重平衡）、Empty（空）等。' },
      cgRefresh: { title: '刷新', desc: '从 Kafka 集群同步最新的消费者组偏移量数据。' },
      cgActions: { title: '更多操作', desc: '重置偏移量或删除消费者组等高级操作入口。' },
      cgList: { title: '偏移量列表', desc: '显示该消费者组在各个 Topic 和分区上的偏移量信息。' },
      cgTableTopic: { title: 'Topic 列', desc: '显示该分区所属的 Topic 名称。' },
      cgTablePartition: { title: '分区列', desc: 'Kafka 分区编号，可拖动列间分隔线调整列宽。' },
      cgTableLag: { title: '延迟列', desc: '消费者滞后量（end_offset - committed_offset），绿色表示无滞后，红色表示严重滞后。' },
      cgTableStartOffset: { title: '起始偏移量', desc: '分区最早的偏移量，表示该分区当前可消费的最小偏移。' },
      cgTableEndOffset: { title: '结束偏移量', desc: '分区最新的偏移量，表示该分区当前已写入的最大偏移。' },
      cgTableCommittedOffset: { title: '已提交偏移量', desc: '消费者组已提交并记录的偏移量，下次消费将从此位置继续。' },
      cgTableLastCommit: { title: '最后提交时间', desc: '消费者组最后一次提交偏移量的时间戳，帮助判断消费者是否活跃。' },
      cgTableRow: { title: '数据行', desc: '每个分区的完整偏移量信息，包括起始、结束、已提交偏移量和最后提交时间。' },
      cgEmptyState: { title: '无数据', desc: '当前消费者组没有任何偏移量记录。' },
      cgResetDialog: { title: '重置偏移量', desc: '通过操作菜单打开的重置偏移量对话框，可选择 Topic、分区、重置目标（最早/最新/指定偏移量/指定时间戳）。' },
      cgNoGroup: { title: '未选择消费者组', desc: '请先在左侧导航栏选择一个消费者组查看详情。' },
      clustersTitle: { title: '集群管理页面', desc: '在此管理所有 Kafka 集群连接，添加、编辑、测试或断开连接。' },
      clustersManageGroups: { title: '分组管理', desc: '创建和编辑集群分组，方便按项目或环境组织集群。' },
      clustersStatus: { title: '连接状态', desc: '绿色表示已连接，红色表示连接失败，灰色表示未连接。' },
      clustersEditBtn: { title: '编辑集群', desc: '修改集群名称、Broker 地址或超时设置。' },
      clustersDeleteBtn: { title: '删除集群', desc: '删除集群连接配置，不影响 Kafka 集群本身。' },
      clustersTestBtn: { title: '测试连接', desc: '验证当前保存的集群配置是否可以成功连接。' },
      clustersReconnectBtn: { title: '重新连接', desc: '立即尝试重新建立与 Kafka 集群的连接。' },
      clustersTopicsBtn: { title: '查看 Topic', desc: '跳转到主题页面，查看该集群下的所有 Topic。' },
      clustersRefreshTopicsBtn: { title: '刷新 Topic', desc: '触发后台刷新该集群的 Topic 元数据。' },
      clustersAddTopicBtn: { title: '新增 Topic', desc: '在当前集群上创建新的 Topic。' },
      clustersEmpty: { title: '暂无集群', desc: '还没有添加任何集群，点击"添加集群"按钮开始配置。' },
      clustersError: { title: '连接错误', desc: '集群连接出现问题，请检查配置后重试。' },
      clustersModal: { title: '添加/编辑集群', desc: '填写集群名称和 Broker 地址，配置完成后点击保存。' },
      clustersModalName: { title: '集群名称', desc: '为集群起一个简短的名称，用于在界面中标识此集群。' },
      clustersModalBrokers: { title: 'Broker 地址', desc: 'Kafka Broker 的连接地址，多个用逗号分隔，如 localhost:9092,localhost:9093。' },
      clustersModalTest: { title: '测试连接', desc: '在保存之前验证 Broker 地址和配置是否可以成功连接。' },
      clustersModalSubmit: { title: '保存集群', desc: '保存集群配置并添加到集群列表中。' },
      settingsTitle: { title: '设置页面', desc: '管理语言、主题、侧边栏模式等全局配置。' },
      settingsLanguage: { title: '语言设置', desc: '切换中文或英文界面。' },
      settingsTheme: { title: '主题设置', desc: '在浅色和深色主题之间切换。' },
      settingsSidebarMode: { title: '侧边栏模式', desc: '在列表模式和树形模式之间切换侧边栏布局，还支持系统托盘和开机自启动设置。' },
      settingsVersion: { title: '版本信息', desc: '查看当前版本、作者信息，检查更新或查看应用日志。' },
      settingsVersionBadge: { title: '版本号', desc: '2 秒内快速点击 5 次可开启开发者功能（查看日志）。' },
      settingsCheckUpdate: { title: '检查更新', desc: '手动检查是否有新版本可用。' },
      settingsImportExport: { title: '导入/导出', desc: '导出所有配置为 JSON 文件备份，或从文件导入恢复数据。' },
      settingsJsonHighlight: { title: 'JSON 高亮设置', desc: '配置消息 Value 的 JSON 高亮显示方式。' },
      favTitle: { title: '收藏页面', desc: '管理您收藏的 Topic，支持分组管理。' },
      favGroupCard: { title: '收藏分组卡片', desc: '点击展开/折叠分组查看收藏的 Topic。' },
      favEditGroup: { title: '编辑分组', desc: '修改分组名称、描述和排序。' },
      favDeleteGroup: { title: '删除分组', desc: '删除分组及分组内的所有收藏。' },
      favGroupSearch: { title: '分组内搜索', desc: '在当前分组中搜索 Topic 名称或备注。' },
      favItemRow: { title: '收藏项', desc: '双击可跳转到对应 Topic 的消息页面。' },
      favEditItem: { title: '编辑收藏', desc: '修改收藏的分组、描述和排序。' },
      favDeleteItem: { title: '删除收藏', desc: '从收藏列表中移除该 Topic。' },
      favEmptyState: { title: '暂无收藏', desc: '还没有添加任何收藏分组，点击"新建分组"开始管理。' },
      favGroupModal: { title: '分组编辑弹窗', desc: '设置分组名称、描述和排序优先级。' },
      favFavoriteModal: { title: '收藏编辑弹窗', desc: '将收藏移到其他分组或添加描述。' },
    },
  },
  en: {
    nav: {
      clusters: 'Clusters',
      favorites: 'Favorites',
      topics: 'Topics',
      messages: 'Messages',
      settings: 'Settings',
    },
    tree: {
      backToClusters: 'Back to Clusters',
      collapseAll: 'Collapse All',
      clusters: 'Clusters',
      topicFavorites: 'Topic Favorites',
      schemaRegistry: 'Schema Registry',
      refreshConsumerGroups: 'Refresh Consumer Groups',
    },
    common: {
      loading: 'Loading...',
      error: 'Error',
      success: 'Success',
      save: 'Save',
      cancel: 'Cancel',
      delete: 'Delete',
      edit: 'Edit',
      create: 'Create',
      refresh: 'Refresh',
      search: 'Search',
      noData: 'No data',
      actions: 'Actions',
      name: 'Name',
      status: 'Status',
      connected: 'Connected',
      disconnected: 'Disconnected',
      all: 'All',
      filter: 'Filter...',
      back: 'Back',
      ready: 'Ready',
      optional: 'Optional',
      confirm: 'Confirm',
      close: 'Close',
      confirmDelete: 'Are you sure you want to delete',
      apply: 'Apply',
      clear: 'Clear',
      cannotBeGreaterThan: 'cannot be greater than',
      unknown: 'Unknown',
      copy: 'Copy',
      copyFailed: 'Copy failed',
      failed: 'Failed',
      hide: 'Hide',
      viewDetails: 'View Details',
      view: 'View',
    },
    clusters: {
      title: 'Clusters',
      description: 'Manage your Kafka cluster connections',
      addCluster: 'Add Cluster',
      editCluster: 'Edit Cluster',
      createCluster: 'Create Cluster',
      clusterName: 'Cluster Name',
      brokers: 'Brokers',
      requestTimeout: 'Request Timeout (ms)',
      operationTimeout: 'Operation Timeout (ms)',
      removeCluster: 'Remove Cluster',
      viewBrokers: 'View Brokers',
      viewTopics: 'View Topics',
      createTopic: 'Create Topic',
      connectionError: 'Connection Error',
      retry: 'Retry',
      brokersLabel: 'Brokers',
      timeoutsLabel: 'Timeouts',
      confirmDelete: 'Are you sure you want to delete cluster "{name}"',
      disconnectConfirm: 'Are you sure you want to disconnect from cluster "{name}"',
      reconnectFailed: 'Reconnect failed',
      fetchFailed: 'Fetch failed',
      connectedLabel: 'Connected',
      disconnectedLabel: 'Disconnected',
      connectingLabel: 'Connecting',
      updated: 'Cluster updated',
      created: 'Cluster created',
      connected: 'Connection successful',
      refreshed: 'Topics refreshed',
      topicsRefreshed: 'Topics refreshed for cluster',
      refreshingBg: 'Topic synced in background',
      reconnected: 'Reconnected successfully',
      topics: 'Topics',
      partitions: 'Partitions',
      viewTopicsLink: 'View Topics',
      group: 'Group',
      noGroup: 'No Group',
      addGroup: 'Add Group',
      testConnection: 'Test Connection',
      testingConnection: 'Testing...',
      connectionSuccess: 'Connection successful',
      connectionFailed: 'Connection failed',
      noDescription: 'No description',
      manageGroups: 'Manage Groups',
      editGroup: 'Edit Group',
      groupName: 'Group Name',
      groupNamePlaceholder: 'Enter group name',
      groupDescription: 'Group Description',
      groupDescPlaceholder: 'Enter description (optional)',
      confirmDeleteGroup: 'Are you sure you want to delete group "{name}"? After deletion, all clusters in this group will become ungrouped.',
      deleteGroupTitle: 'Delete Group',
      test: 'Test',
      reconnect: 'Reconnect',
      disconnect: 'Disconnect',
      disconnectedSuccess: 'Cluster disconnected',
      reconnectSuccess: 'Reconnected successfully',
      groupUpdated: 'Group updated',
      groupCreated: 'Group created',
      groupDeleted: 'Group deleted',
      clusterDeleted: 'Cluster deleted',
      scrollLeft: 'Scroll left',
      scrollRight: 'Scroll right',
      editClusterTitle: 'Edit Cluster',
      createClusterTitle: 'Create Cluster',
      newCluster: 'New Cluster',
      brokersHelp: 'Comma-separated list of broker addresses',
      createdDate: 'Created',
      unknown: 'unknown',
      refreshTopics: 'Refresh Topics',
      refreshFailed: 'Refresh failed',
      validationNameRequired: 'Cluster name is required',
      validationBrokersRequired: 'Broker address is required',
      validationNameInvalid: 'Cluster name can contain letters, numbers, Chinese characters, hyphens, and underscores',
      validationNameTooLong: 'Cluster name cannot exceed 15 characters',
      validationBrokersInvalid: 'Broker address cannot contain spaces, quotes, or commas',
      connectionTestSuccess: 'Connection test successful',
      connectionTestFailed: 'Connection test failed',
      clusterStatusRefreshed: 'Cluster status refreshed',
      clusterConnectionIssue: 'Cluster connection issue',
      reconnectSuccessToast: 'Reconnected successfully',
      clusterDeletedToast: 'Cluster deleted',
      clusters: 'Clusters',
    },
    topics: {
      title: 'Topics',
      description: 'Manage Kafka topics',
      createTopic: 'Create Topic',
      topicName: 'Topic Name',
      topicNamePlaceholder: 'Enter topic name',
      topicNameValidation: 'Only letters, numbers, dots, underscores, and hyphens are allowed',
      partitionCount: 'Partition Count',
      numPartitions: 'Number of Partitions',
      numPartitionsHelp: 'Partition count range: 1-100',
      replicationFactor: 'Replication Factor',
      replicationFactorHelp: 'Replication factor range: 1-10',
      advancedOptions: 'Advanced Options',
      cleanupPolicy: 'Cleanup Policy',
      retentionMs: 'Retention (ms)',
      retentionBytes: 'Retention (bytes)',
      segmentBytes: 'Segment Size (bytes)',
      retentionMsPlaceholder: '604800000 (7 days)',
      retentionBytesPlaceholder: '-1 (unlimited)',
      segmentBytesPlaceholder: '1073741824 (1GB)',
      viewMessages: 'View Messages',
      viewDetails: 'View Details',
      viewPartitions: 'View Partitions',
      sendMessage: 'Send Message',
      deleteTopic: 'Delete Topic',
      exportData: 'Export Data',
      topicDetails: 'Topic Details',
      partitions: 'Partitions',
      settings: 'Settings',
      refreshed: 'Topics refreshed',
      refreshingBg: 'Topic synced in background',
      confirmDeleteTitle: 'Delete Topic',
      confirmDeleteHint: 'This action cannot be undone',
      confirmDeleteInput: 'Type the topic name to confirm',
      confirmDeleteMatchError: 'Topic name does not match',
      copied: 'Copied',
      copyFailed: 'Copy failed',
      deletedSuccess: 'Topic deleted successfully',
      createdSuccess: 'Topic "${name}" created successfully',
      createFailed: 'Failed to create topic',
      validationSelectCluster: 'Please select a cluster first',
      validationClusterIdRequired: 'Cluster ID is required',
      validationTopicNameRequired: 'Topic name is required',
      validationTopicNameTooLong: 'Topic name cannot exceed 256 characters',
      validationTopicNameInvalidChars: 'Topic name cannot contain spaces, quotes, or commas',
      validationTopicNameFormat: 'Topic name can only contain letters, numbers, dots, underscores, and hyphens',
      validationRetentionMs: 'retention.ms must be a positive number',
      validationRetentionBytes: 'retention.bytes must be a number (use -1 for unlimited)',
      validationSegmentBytes: 'segment.bytes must be a positive number',
      noSearchResults: 'No matching topics found',
      clearSearch: 'Clear Search',
    },
    messages: {
      title: 'Messages',
      description: 'View and send Kafka messages',
      sendMessage: 'Send Message',
      partition: 'Partition',
      key: 'Key',
      value: 'Value',
      targetPartition: 'Target partition',
      optional: 'Optional',
      required: 'Required',
      messageSent: 'Message sent',
      offset: 'Offset',
      send: 'Send',
      sendAndNew: 'Send & New',
      sending: 'Sending...',
      allPartitions: 'All Partitions',
      fetch: 'Fetch',
      filter: 'Filter',
      formatJson: 'Format JSON',
      fetchMode: 'Fetch Mode',
      oldest: 'Oldest',
      newest: 'Newest',
      maxMessages: 'Max Messages',
      perPartition: 'Per Partition',
      startTime: 'Start Time',
      endTime: 'End Time',
      timeRange: 'Time Range',
      timeRangeFilter: 'Advanced Filter',
      clear: 'Clear',
      selectTopic: 'Select Topic',
      selectMessage: 'Select Message',
      noTopicSelected: 'No topic selected',
      noMessages: 'No messages',
      offsetLabel: 'Offset',
      partitionLabel: 'Partition',
      timestampLabel: 'Timestamp',
      sizeLabel: 'Size',
      viewAs: 'View As',
      json: 'JSON',
      raw: 'Raw',
      hex: 'Hex',
      copied: 'Copied',
      messages: 'Messages',
      time: 'Time',
      selectedOffset: 'Selected Offset',
      ready: 'Ready',
      query: 'Query',
      stop: 'Stop',
      exportMessages: 'Export Messages',
      messageDetail: 'Message Detail',
      copyValue: 'Copy Value',
      copyKey: 'Copy Key',
      valuePlaceholder: 'Search message content...',
      searchPlaceholder: 'Search key or value...',
      elapsedTime: 'Elapsed',
      totalMessages: 'Total',
      actions: 'Actions',
      close: 'Close',
      cancel: 'Cancel',
      continue: 'Send & Continue',
      keyOptional: 'Optional',
      valueRequired: 'Required',
      partitionLabel2: 'Partition',
      queryFailed: 'Query failed',
      fetchFailed: 'Fetch failed',
      loading: 'Loading...',
      clearSort: 'Clear sort',
      hide: 'Hide',
      show: 'Show',
      valid: 'Valid',
      exportSuccess: 'Messages exported',
      exportFailed: 'Export failed',
      receiving: 'Receiving',
      recent5Minutes: '5-10 min ago',
      recent15Minutes: '15-30 min ago',
      recent30Minutes: '30-60 min ago',
      recent1Hour: '1-2 hours ago',
      recent1Day: '1-2 days ago',
      topicLabel: 'Topic',
      cluster: 'Cluster',
      minutes: 'min',
      hour: 'hr',
      day: 'day',
      queryTimeout: 'Query timeout, please try again',
      exportTip: 'Query count is per partition, filtering is applied after fetching',
      confirmDeleteTopic: 'Are you sure you want to delete topic "{topic}"? This action cannot be undone',
      deleteTopic: 'Delete Topic',
      topicDeleted: 'Topic deleted successfully',
    },
    consumerGroups: {
      title: 'Consumer Groups',
      description: 'Manage Kafka consumer groups',
      groupName: 'Group Name',
      topics: 'Topics',
      state: 'State',
      partitions: 'Partitions',
      offset: 'Offset',
      lag: 'Lag',
      start: 'Start',
      end: 'End',
      resetOffset: 'Reset Offset',
      resetOffsetToEarliest: 'Reset to Earliest',
      resetOffsetToLatest: 'Reset to Latest',
      resetOffsetToTimestamp: 'Reset to Timestamp',
      timestamp: 'Timestamp',
      refreshOffsets: 'Refresh Offsets',
      offsetsRefreshed: 'Offsets refreshed',
      offsetResetSuccess: 'Offset reset successfully',
      confirmResetOffset: 'Confirm reset offset?',
      confirmResetOffsetToEarliest: 'Confirm reset offset to earliest?',
      confirmResetOffsetToLatest: 'Confirm reset offset to latest?',
      confirmResetOffsetToTimestamp: 'Confirm reset offset to timestamp?',
      noData: 'No consumer groups',
      emptyHelp: 'Click the refresh button above to sync consumer groups from the Kafka cluster',
      refreshed: 'Consumer groups refreshed',
      refreshingBg: 'Consumer group synced in background',
      deleteGroup: 'Delete Group',
      deleted: 'Consumer group deleted',
      offsets: 'Offset Details',
      topic: 'Topic',
      partition: 'Partition',
      startOffset: 'Start Offset',
      endOffset: 'End Offset',
      committedOffset: 'Committed Offset',
      lastCommit: 'Last Commit',
      selectTopic: 'Select Topic',
      resetTo: 'Reset To',
      earliest: 'Earliest',
      latest: 'Latest',
      specificOffset: 'Specific Offset',
      offsetValue: 'Offset Value',
      timestampValue: 'Timestamp',
      noOffsets: 'No offset data available',
      selectFromNav: 'Please select a consumer group from the left navigation to view details',
      groupNamePrefix: 'Consumer Group: ',
    },
    topicConsumerGroups: {
      title: 'Topic Consumer Groups',
      description: 'View all Consumer Groups consuming this Topic',
      noData: 'No Consumer Groups consuming this Topic',
      groupName: 'Consumer Group Name',
      state: 'State',
      partitions: 'Partitions',
      lag: 'Lag',
      viewDetails: 'View Details',
      refresh: 'Refresh',
      refreshAllConsumerGroup: 'Refresh Cluster Consumer Groups',
      refreshed: 'Consumer Groups refreshed',
      dataNoticeTitle: 'Data Source Notice',
      dataNotice: 'Consumer Groups shown on this page are from database history, offset data is fetched from Kafka in real-time.',
      topicNamePrefix: 'Topic: ',
    },
    settings: {
      title: 'Settings',
      description: 'Manage global settings',
      language: 'Language',
      theme: 'Theme',
      languageZh: 'Chinese',
      languageEn: 'English',
      selectLanguage: 'Select Language',
      systemSettings: 'System Settings',
      sidebarMode: 'Sidebar Mode',
      treeMode: 'Tree Mode',
      flatMode: 'List Mode',
      treeModeDesc: 'Display topics grouped by cluster',
      flatModeDesc: 'Display all topics in a flat list',
      systemTray: 'System Tray',
      systemTrayDesc: 'When enabled, close button minimizes to tray; when disabled, close button exits the app',
      systemTrayEnabled: 'System tray enabled',
      systemTrayDisabled: 'System tray disabled',
      autoLaunch: 'Auto Launch',
      autoLaunchDesc: 'Automatically start Kafka Manager on boot',
      autoLaunchEnabled: 'Auto launch enabled',
      autoLaunchDisabled: 'Auto launch disabled',
      autoLaunchFailed: 'Failed to set auto launch',
      version: 'Version',
      versionDesc: 'Kafka Manager Current Version',
      currentVersion: 'Current Version',
      author: 'Author',
      help: 'Help',
      themeDesc: 'Toggle light or dark mode',
      lightMode: 'Light Mode',
      darkMode: 'Dark Mode',
      jsonHighlight: 'JSON Highlight',
      jsonHighlightDesc: 'Configure JSON highlight style for message details and send message modal',
      selectTemplate: 'Select Template',
      preview: 'Preview',
      customTemplates: 'Custom Templates',
      addCustomTemplate: 'Add Custom Template',
      templateName: 'Template Name',
      templateDescription: 'Description',
      templateStyle: 'Style Config',
      saveTemplate: 'Save Template',
      deleteTemplate: 'Delete Template',
      confirmDeleteTemplate: 'Are you sure you want to delete this custom template?',
      templateFormat: 'Template Format',
      builtInTemplates: 'Built-in Templates',
      importExport: 'Import/Export Data',
      importExportDesc: 'Export or import clusters, favorites and history',
      exportData: 'Export Data',
      exporting: 'Exporting...',
      exportSuccess: 'Export successful',
      importData: 'Import Data',
      importing: 'Importing...',
      importSuccess: 'Import completed',
      operationInProgress: 'Another import/export operation is already in progress, please wait and try again',
      importStarted: 'Import started in background, will complete automatically',
      viewLogs: 'View Logs',
      appLogs: 'Application Logs',
      refreshLogs: 'Refresh Logs',
      copyLogs: 'Copy Logs',
      clearLogs: 'Clear Logs',
      scrollToBottom: 'Scroll to Bottom',
      noLogs: 'No logs',
      logsCopied: 'Copied to clipboard',
      logsCleared: 'Logs cleared',
      logsRefreshed: 'Refreshed',
      feedback: 'Feedback',
      feedbackDesc: 'Share your valuable feedback with us',
      feedbackPlaceholder: 'Enter your feedback or suggestions (max 2000 characters)',
      feedbackSubmit: 'Submit Feedback',
      feedbackSubmitting: 'Submitting...',
      feedbackSuccess: 'Feedback submitted successfully, thank you!',
      feedbackFailed: 'Feedback submission failed',
      feedbackMaxLength: 'Feedback cannot exceed 2000 characters',
      feedbackConnectionFailed: 'Cannot connect to server, feedback is temporarily unavailable',
      feedbackSystemInfo: 'System Info',
      feedbackDocLink: 'Kafka Manager Documentation',
    },
    layout: {
      searchPlaceholder: 'Search topics... (Ctrl+K)',
      noTopicsFound: 'No topics found',
      settings: 'Settings',
      help: 'Help',
      checkForUpdates: 'Check for Updates',
      confirmDeleteCluster: 'Are you sure you want to remove cluster "{cluster}"?',
      confirmDeleteTopic: 'Are you sure you want to delete topic "{topic}"?',
      clusterNotFound: 'Cluster not found',
      topicNotFound: 'Topic not found',
      refreshFailed: 'Refresh failed',
      refreshCancelled: 'Refresh cancelled',
      shareVersion: 'Share Installer',
    shareCopied: 'Installer copied to your downloads folder, please open file manager to view',
    },
    update: {
      available: 'New Version Available',
      currentVersion: 'Current Version',
      newVersion: 'New Version',
      releaseNotes: 'Release Notes',
      downloading: 'Downloading...',
      installing: 'Installing...',
      installed: 'Installed',
      installFailed: 'Install Failed',
      networkError: 'Network request failed, please check your connection and try again',
      updateAndRestart: 'Download & Install',
      checkCompleteNoUpdate: 'Already up to date',
      checkFailed: 'Failed to check for updates',
      downloadComplete: 'Download complete, opening installer...',
      browserNotSupported: 'Check for updates is not supported in browser environment',
      checkForUpdates: 'Check for Updates',
      checking: 'Checking...',
      checkNow: 'Check Now',
      downloadingInBackground: 'Downloading update in background...',
      minimizeHint: 'Download will continue in background after closing this dialog',
      minimizeModal: 'Hide Window',
      rateLimitExceeded: 'Access denied, please try again later',
      downloadInProgress: 'Download in progress, please wait...',
      downloadFailed: 'Download failed, please check your network connection and try again',
    },
    toast: {
      error: 'Error',
      success: 'Success',
      warning: 'Warning',
      info: 'Info',
      copySuccess: 'Copied',
      copyFailed: 'Copy failed',
      operationFailed: 'Operation failed',
      clusterNotFound: 'Cluster not found',
      networkError: 'Network error',
      invalidFormat: 'Invalid format',
      skipped: 'Skipped',
    },
    topicContextMenu: {
      viewMessages: 'View Messages',
      viewDetails: 'View Details',
      viewPartitions: 'View Partitions',
      sendMessage: 'Send Message',
      exportData: 'Export Data',
      deleteTopic: 'Delete Topic',
    },
    partitionContextMenu: {
      viewMessages: 'View Messages',
      sendMessage: 'Send Message',
    },
    mainLayout: {
      clustersSelected: 'cluster(s) selected',
      refreshHealth: 'Refresh cluster health',
      footerText: 'Kafka Manager v0.1.0 - Built with Vue 3 + Tailwind CSS + DaisyUI',
      multiCluster: 'Multi-Cluster',
      clustersLabel: 'Clusters',
      selectAll: 'All',
      clearSelection: 'None',
      manageClusters: 'Manage Clusters',
      topics: 'Topics',
      messages: 'Messages',
      dragToResize: 'Drag to resize sidebar',
    },
    navigator: {
      allClusters: 'All Clusters',
      byGroup: 'By Group',
      selectGroupsAndClusters: 'Select Groups/Clusters',
      selected: 'Selected',
      selectCluster: 'Select cluster',
      groups: 'Groups',
      clusters: 'Clusters',
      deselectAll: 'Deselect All',
      selectClusters: 'Select Clusters',
      refreshing: 'Refreshing...',
      favorites: 'Topic Favorites',
      schemaRegistry: 'Schema Registry',
      history: 'Browsing History',
      topicsLabel: 'Topics',
      consumerGroupsLabel: 'Consumer Groups',
      viewClusterTopics: 'View all topics in this cluster',
    },
    favorites: {
      title: 'Topic Favorites',
      description: 'Manage your favorite Topics with group support',
      add: 'Add to Favorites',
      remove: 'Remove from Favorites',
      added: 'Added to favorites',
      removed: 'Removed from favorites',
      selectGroup: 'Select Group',
      noGroups: 'No groups yet',
      createGroup: 'Create Group',
      addGroup: 'New Group',
      editGroup: 'Edit Group',
      groupName: 'Group Name',
      groupNamePlaceholder: 'Enter group name',
      groupDescription: 'Description',
      groupDescPlaceholder: 'Enter description (optional)',
      sortOrder: 'Sort Order',
      sortOrderPlaceholder: 'Smaller numbers appear first',
      empty: 'No favorite groups yet',
      emptyHint: 'Click the button above to create a group',
      noItems: 'No items in this group',
      noSearchResults: 'No matching favorites',
      searchPlaceholder: 'Search topic name, remark...',
      editFavorite: 'Edit Favorite',
      favoriteDescription: 'Description',
      favoriteDescPlaceholder: 'Enter description (optional)',
      confirmDeleteGroup: 'Are you sure you want to delete this group? Favorites in this group will also be deleted.',
      confirmDeleteFavorite: 'Are you sure you want to delete this favorite?',
      createGroupHint: 'Please create a group in favorite management first',
      groupCreated: 'Group created successfully',
      remark: 'Remark',
      remarkPlaceholder: 'Add remark (optional)',
      group: 'group(s)',
    },
    history: {
      title: 'Browsing History',
      description: 'Automatically record Topics you have viewed',
      empty: 'No browsing history yet',
      emptyHint: 'Topics will be automatically recorded here when viewed',
      noSearchResults: 'No matching history records',
      searchPlaceholder: 'Search Topic...',
      delete: 'Delete record',
      clearAll: 'Clear all history',
      confirmClear: 'Are you sure you want to clear all history records?',
      justNow: 'Just now',
      minutesAgo: 'm ago',
      hoursAgo: 'h ago',
      daysAgo: 'd ago',
    },
    sentMessageHistory: {
      title: 'Sent Messages',
      description: 'Automatically record messages you have sent',
      empty: 'No sent message history yet',
      emptyHint: 'Messages will be automatically recorded here when sent',
      noSearchResults: 'No matching history records',
      searchPlaceholder: 'Search Topic...',
      delete: 'Delete record',
      clearAll: 'Clear all history',
      confirmClear: 'Are you sure you want to clear all sent message history?',
      justNow: 'Just now',
      minutesAgo: 'm ago',
      hoursAgo: 'h ago',
      daysAgo: 'd ago',
    },
    schemaRegistry: {
      title: 'Schema Registry',
      description: 'Manage Schema Registry configuration and schemas',
      configTitle: 'Schema Registry Configuration',
      registryUrl: 'Registry URL',
      registryUrlPlaceholder: 'http://localhost:8081',
      authentication: 'Authentication',
      username: 'Username',
      password: 'Password',
      testConnection: 'Test Connection',
      save: 'Save Configuration',
      delete: 'Delete Configuration',
      subjects: 'Subjects',
      noSubjects: 'No subjects yet',
      versions: 'Versions',
      schemaType: 'Schema Type',
      compatibilityLevel: 'Compatibility Level',
      backward: 'Backward',
      forward: 'Forward',
      full: 'Full',
      none: 'None',
      testCompatibility: 'Test Compatibility',
      compatible: 'Compatible',
      incompatible: 'Incompatible',
      registerSchema: 'Register Schema',
      schemaContent: 'Schema Content',
      schemaContentPlaceholder: 'Paste schema JSON content...',
      registerSuccess: 'Schema registered successfully',
      compatibilityTestSuccess: 'Compatibility test successful',
      compatibilityTestFailed: 'Compatibility test failed',
      configNotSet: 'Schema Registry not configured',
      fetchFailed: 'Fetch failed',
      deleteConfirm: 'Are you sure you want to delete this subject and all its versions?',
      version: 'Version',
      latestVersion: 'Latest Version',
      viewSchema: 'View Schema',
      deleteSchema: 'Delete Subject',
      connectionSuccess: 'Connection successful',
      connectionFailed: 'Connection failed',
      selectCluster: 'Select Cluster',
      selectClusterDesc: 'Choose a Kafka cluster to manage Schema Registry',
    },
    tour: {
      prevStep: 'Previous',
      nextStep: 'Next',
      finishTour: 'Finish',
      globalSidebar: { title: 'Sidebar Navigation', desc: 'The left sidebar allows you to quickly switch between clusters, topics, favorites, and consumer groups. Click any item to view details.' },
      globalSearch: { title: 'Topic Search', desc: 'Search for topic names across all clusters in the top search bar.' },
      globalShare: { title: 'Share Installer', desc: 'Click to copy the installer to your downloads folder, or navigate to the GitHub releases page.' },
      globalLang: { title: 'Switch Language', desc: 'Toggle between Chinese and English interface.' },
      globalTheme: { title: 'Toggle Theme', desc: 'Switch between light and dark themes.' },
      globalSettings: { title: 'Settings', desc: 'Open the settings page to manage language, theme, sidebar mode, and other global configurations.' },
      clustersAdd: { title: 'Add Cluster', desc: 'Click to create a new Kafka cluster connection. Enter a name and broker addresses to get started.' },
      clustersGroup: { title: 'Group Filter', desc: 'Quickly filter clusters by group.' },
      clustersCard: { title: 'Cluster Cards', desc: 'Each card shows the cluster connection status. Right-click or use the action buttons to refresh, edit, delete, or disconnect.' },
      topicsCreate: { title: 'Create Topic', desc: 'Click to create a new Kafka topic with partition count, replication factor, and other parameters.' },
      topicsList: { title: 'Topic List', desc: 'Lists all topics in the current cluster with search and sort support.' },
      topicsActions: { title: 'Topic Actions', desc: 'View messages, view details, send messages, or delete topics.' },
      messagesSearch: { title: 'Message Search', desc: 'Search for keywords within message content.' },
      messagesBack: { title: 'Back', desc: 'Return to the previous page.' },
      messagesPartition: { title: 'Partition Selector', desc: 'Select a specific partition to query, or query all partitions.' },
      messagesMode: { title: 'Query Mode', desc: 'Choose whether to start from the newest or oldest messages.' },
      messagesTimeFilter: { title: 'Advanced Filter', desc: 'Filter messages by time range. Supports manual time input or quick time range presets.' },
      messagesTimePresets: { title: 'Quick Time Range', desc: 'Quickly select recent time ranges: 5 minutes, 15 minutes, 30 minutes, 1 hour, or 1 day.' },
      messagesCount: { title: 'Message Count', desc: 'Set the maximum number of messages to return per partition per query.' },
      messagesQuery: { title: 'Execute Query', desc: 'Fetch messages from Kafka. After per-partition data returns, filter results by query conditions and display.' },
      messagesSend: { title: 'Send Message', desc: 'Open the message send dialog to publish new messages to the current topic.' },
      messagesExport: { title: 'Export Messages', desc: 'Export queried messages as a JSON file saved locally.' },
      messagesMore: { title: 'More Actions', desc: 'Access more operations like sent history, consumer groups, and topic deletion.' },
      messagesTableHeader: { title: 'Message Table Header', desc: 'Column headers for partition, offset, timestamp, key, and value. Click the timestamp to sort, drag column dividers to resize.' },
      messagesTableRow: { title: 'Message Row', desc: 'Preview of each message. Click to view details. Use arrow keys to navigate between rows.' },
      messagesDetailPanel: { title: 'Message Detail', desc: 'Full message content with Ctrl+F search, Ctrl+A select-all, JSON highlighting. Drag the top handle to resize the panel.' },
      messagesValueViewFormat: { title: 'View Format', desc: 'Switch value display format: JSON (auto-formatted), Raw (plain text), or Hex (hexadecimal).' },
      consumerGroupsList: { title: 'Consumer Groups List', desc: 'Shows all consumer groups with their state, lag, and other details.' },
      consumerGroupsRefresh: { title: 'Refresh', desc: 'Sync the latest consumer group status from the Kafka cluster.' },
      consumerGroupsDetail: { title: 'Consumer Group Details', desc: 'Click a consumer group name to view details, including partition assignments and offsets.' },
      favoritesList: { title: 'Favorites List', desc: 'Shows your saved topics for quick access. Click to jump to the messages page.' },
      favoritesAdd: { title: 'Add to Favorites', desc: 'Add topics to favorites from the topic list or messages page for quick access later.' },
      settingsNav: { title: 'Settings Categories', desc: 'Browse and modify global settings by category.' },
      settingsSections: { title: 'Settings Items', desc: 'Manage language, theme, sidebar mode, system tray, auto-launch, and more.' },
      schemaRegistryConfig: { title: 'Configuration Area', desc: 'Configure the Schema Registry connection URL and authentication credentials.' },
      schemaRegistryActions: { title: 'Action Buttons', desc: 'Test connection, save, or delete your Schema Registry configuration.' },
      schemaRegistryTitle: { title: 'Page Title', desc: 'Schema Registry management entry point, configure and manage Schema Registry.' },
      schemaRegistryNoCluster: { title: 'No Cluster Selected', desc: 'Select a connected Kafka cluster first to configure Schema Registry.' },
      schemaRegistryNotConfig: { title: 'Not Configured', desc: 'Schema Registry is not yet configured for this cluster. Click the button to add configuration.' },
      schemaRegistryConfigCard: { title: 'Configuration Info Card', desc: 'Shows current Schema Registry connection info including URL, username, and connection status.' },
      schemaRegistryConfigActions: { title: 'Config Actions', desc: 'Edit or delete the Schema Registry connection configuration.' },
      schemaRegistrySubjectsCard: { title: 'Subjects List', desc: 'Manage all registered Schema Subjects, with support for viewing, registering, and deleting.' },
      schemaRegistrySubjectsTable: { title: 'Subject Table', desc: 'Displays each Subject version, type, compatibility, and other information.' },
      schemaRegistryClusterDialog: { title: 'Cluster Selector Dialog', desc: 'Select which Kafka cluster to manage Schema Registry for.' },
      schemaRegistryConfigDialog: { title: 'Configuration Dialog', desc: 'Set the Schema Registry connection URL and authentication info.' },
      schemaRegistrySchemaDialog: { title: 'Schema Details Dialog', desc: 'View the full Schema content and metadata.' },
      schemaRegistryRegisterDialog: { title: 'Register Schema Dialog', desc: 'Register a new Schema to the registry, supporting AVRO, Protobuf, and JSON formats.' },
      sidebarViewSwitcher: { title: 'View Switcher', desc: 'Toggle between Topics and Consumer Groups in the left sidebar navigation.' },
      sidebarClustersBtn: { title: 'Cluster Management', desc: 'Navigate to the cluster management page to add, edit, or remove cluster connections.' },
      sidebarFavoritesBtn: { title: 'Favorites', desc: 'Open the favorites page to manage your saved topics with group support.' },
      sidebarSchemaBtn: { title: 'Schema Registry', desc: 'Open the Schema Registry page to configure and manage schemas.' },
      sidebarHistoryBtn: { title: 'Browsing History', desc: 'View recently browsed topics and consumer groups.' },
      sidebarClusterSelector: { title: 'Cluster Filter', desc: 'Click to open the cluster selection panel. Select or deselect clusters to filter the displayed data. When no clusters are selected, all clusters are shown.' },
      sidebarRefresh: { title: 'Refresh Data', desc: 'Synchronize the latest topics or consumer groups from the Kafka cluster.' },
      sidebarSearch: { title: 'Search', desc: 'Search for topics or consumer groups by name within the current clusters. Results filter automatically as you type.' },
      sidebarTopicList: { title: 'Topic List', desc: 'Displays all topics with their cluster. Click any topic to navigate to the messages page. Scrolling to the bottom automatically loads more items.' },
      sidebarTopicName: { title: 'Topic Name', desc: 'Click the topic name to jump to the messages page and view the latest message details.' },
      sidebarConsumerGroupList: { title: 'Consumer Group List', desc: 'Displays all consumer groups with their cluster. Click any consumer group to navigate to the details page showing partition assignments and offsets.' },
      sidebarConsumerGroupName: { title: 'Consumer Group Name', desc: 'Click the consumer group name to jump to the details page showing partition assignments and offset information.' },
      sidebarHealthDot: { title: 'Health Indicator', desc: 'The dot on the left of each item shows the cluster connection status: green = connected, red = disconnected, yellow = unknown.' },
      sidebarClusterBadge: { title: 'Cluster Badge', desc: 'The badge on the right shows which cluster each item belongs to, helping you distinguish topics or consumer groups with the same name across multiple clusters.' },
      treeCollapseBtn: { title: 'Collapse All', desc: 'Click to collapse all expanded clusters and folders.' },
      treeClustersBtn: { title: 'Cluster Management', desc: 'Navigate to the cluster management page to add, edit, or remove cluster connections.' },
      treeFavoritesBtn: { title: 'Favorites', desc: 'Open the favorites page to manage your saved topics.' },
      treeSchemaBtn: { title: 'Schema Registry', desc: 'Open the Schema Registry page to configure and manage schemas.' },
      treeHistoryBtn: { title: 'Browsing History', desc: 'View recently browsed topics.' },
      treeGroupSelector: { title: 'Group Filter', desc: 'Quickly filter clusters by group membership.' },
      treeClusterNode: { title: 'Cluster Node', desc: 'The dot on the left shows the connection status. Double-click the cluster node to expand and view its Topics and Consumer Groups.' },
      treeClusterName: { title: 'Cluster Name', desc: 'The name of the Kafka cluster, displayed in the node header.' },
      treeClusterHealthDot: { title: 'Cluster Health Indicator', desc: 'The dot on the left of the cluster name shows the connection status: green = connected, red = disconnected, yellow pulsing = refreshing.' },
      treeTopicsFolder: { title: 'Topics Folder', desc: 'Click to expand and view all topics under this cluster. The refresh button syncs the latest data from Kafka.' },
      treeTopicSearch: { title: 'Topic Search', desc: 'Search for topics by name within the current cluster. Results filter automatically as you type.' },
      treeTopicName: { title: 'Topic Name', desc: 'Click a topic name to jump to the messages page and view the latest messages.' },
      treeTopicsRefresh: { title: 'Refresh Topics', desc: 'Synchronize the latest topic list from the Kafka cluster.' },
      treeConsumerGroupsFolder: { title: 'Consumer Groups Folder', desc: 'Click to expand and view all consumer groups under this cluster. The refresh button syncs the latest data from Kafka.' },
      treeConsumerGroupSearch: { title: 'Consumer Group Search', desc: 'Search for consumer groups by name within the current cluster. Results filter automatically as you type.' },
      treeConsumerGroupName: { title: 'Consumer Group Name', desc: 'Click a consumer group name to jump to the details page showing partition assignments and offsets.' },
      treeConsumerGroupsRefresh: { title: 'Refresh Consumer Groups', desc: 'Synchronize the latest consumer group list from the Kafka cluster.' },
      tcgTitle: { title: 'Topic Consumer Groups', desc: 'This page shows the offset information for all consumer groups consuming the specified topic.' },
      tcgDataNotice: { title: 'Data Source Notice', desc: 'Consumer groups are from database history, offset data is fetched from Kafka in real-time.' },
      tcgRefresh: { title: 'Refresh Data', desc: 'Fetch the latest consumer group offset information.' },
      tcgTable: { title: 'Offsets Table', desc: 'Displays start offset, end offset, committed offset, and lag for each consumer group across partitions.' },
      cgBack: { title: 'Back', desc: 'Return to the previous page.' },
      cgGroupName: { title: 'Consumer Group Name', desc: 'The name of the consumer group you are currently viewing.' },
      cgState: { title: 'State Badge', desc: 'Shows the current consumer group state: Stable, PreparingRebalance, Empty, etc.' },
      cgRefresh: { title: 'Refresh', desc: 'Synchronize the latest consumer group offset data from the Kafka cluster.' },
      cgActions: { title: 'More Actions', desc: 'Advanced operations like resetting offsets or deleting the consumer group.' },
      cgList: { title: 'Offsets List', desc: 'Shows offset information for this consumer group across all topics and partitions.' },
      cgTableTopic: { title: 'Topic Column', desc: 'Displays the topic name that this partition belongs to.' },
      cgTablePartition: { title: 'Partition Column', desc: 'Kafka partition number. Drag column dividers to resize.' },
      cgTableLag: { title: 'Lag Column', desc: 'Consumer lag (end_offset - committed_offset). Green = no lag, red = significant lag.' },
      cgTableStartOffset: { title: 'Start Offset', desc: 'The earliest offset in the partition, representing the minimum consumable offset.' },
      cgTableEndOffset: { title: 'End Offset', desc: 'The latest offset in the partition, representing the maximum written offset.' },
      cgTableCommittedOffset: { title: 'Committed Offset', desc: 'The offset committed by the consumer group, indicating where consumption will resume next.' },
      cgTableLastCommit: { title: 'Last Commit Time', desc: 'The timestamp of the last offset commit by the consumer group, helping determine consumer activity.' },
      cgTableRow: { title: 'Data Row', desc: 'Complete offset information for each partition including start, end, committed offsets, and last commit time.' },
      cgEmptyState: { title: 'No Data', desc: 'This consumer group has no offset records.' },
      cgResetDialog: { title: 'Reset Offsets', desc: 'Open the reset offset dialog from the actions menu. You can select a topic, partition, and reset target (earliest/latest/specific offset/timestamp).' },
      cgNoGroup: { title: 'No Group Selected', desc: 'Select a consumer group from the left sidebar to view details.' },
      clustersTitle: { title: 'Cluster Management', desc: 'Manage all Kafka cluster connections here: add, edit, test, or disconnect.' },
      clustersManageGroups: { title: 'Manage Groups', desc: 'Create and edit cluster groups to organize clusters by project or environment.' },
      clustersStatus: { title: 'Connection Status', desc: 'Green = connected, red = connection failed, gray = disconnected.' },
      clustersEditBtn: { title: 'Edit Cluster', desc: 'Modify cluster name, broker addresses, or timeout settings.' },
      clustersDeleteBtn: { title: 'Delete Cluster', desc: 'Remove the cluster connection configuration. This does not affect the Kafka cluster itself.' },
      clustersTestBtn: { title: 'Test Connection', desc: 'Verify the current cluster configuration can connect successfully.' },
      clustersReconnectBtn: { title: 'Reconnect', desc: 'Immediately attempt to re-establish the connection to the Kafka cluster.' },
      clustersTopicsBtn: { title: 'View Topics', desc: 'Jump to the Topics page showing all topics under this cluster.' },
      clustersRefreshTopicsBtn: { title: 'Refresh Topics', desc: 'Trigger a background refresh of topic metadata for this cluster.' },
      clustersAddTopicBtn: { title: 'Add Topic', desc: 'Create a new Topic on this cluster.' },
      clustersEmpty: { title: 'No Clusters', desc: "You haven't added any clusters yet. Click 'Add Cluster' to get started." },
      clustersError: { title: 'Connection Error', desc: 'There is an issue with the cluster connection. Please check the configuration and try again.' },
      clustersModal: { title: 'Add/Edit Cluster', desc: 'Fill in the cluster name and broker addresses, then click save.' },
      clustersModalName: { title: 'Cluster Name', desc: 'A short name to identify this cluster in the UI.' },
      clustersModalBrokers: { title: 'Broker Addresses', desc: 'Kafka broker connection addresses, comma-separated (e.g., localhost:9092,localhost:9093).' },
      clustersModalTest: { title: 'Test Connection', desc: 'Verify broker addresses and settings before saving.' },
      clustersModalSubmit: { title: 'Save Cluster', desc: 'Save the cluster configuration and add it to the cluster list.' },
      settingsTitle: { title: 'Settings Page', desc: 'Manage global settings like language, theme, sidebar mode, and more.' },
      settingsLanguage: { title: 'Language', desc: 'Switch between Chinese and English interface.' },
      settingsTheme: { title: 'Theme', desc: 'Toggle between light and dark themes.' },
      settingsSidebarMode: { title: 'Sidebar Mode', desc: 'Switch between list and tree sidebar layouts. Also includes system tray and auto-launch settings.' },
      settingsVersion: { title: 'Version Info', desc: 'View current version, author info, check for updates, or view app logs.' },
      settingsVersionBadge: { title: 'Version Number', desc: 'Click 5 times quickly to unlock developer features (view logs).' },
      settingsCheckUpdate: { title: 'Check for Updates', desc: 'Manually check for new versions.' },
      settingsImportExport: { title: 'Import/Export', desc: 'Export all configuration as a JSON backup file, or import from a file to restore data.' },
      settingsJsonHighlight: { title: 'JSON Highlight Settings', desc: 'Configure how JSON values are highlighted in messages.' },
      favTitle: { title: 'Favorites Page', desc: 'Manage your saved topics with group support.' },
      favGroupCard: { title: 'Group Card', desc: 'Click to expand or collapse a group to view saved topics.' },
      favEditGroup: { title: 'Edit Group', desc: 'Modify the group name, description, and sort order.' },
      favDeleteGroup: { title: 'Delete Group', desc: 'Remove the group and all favorites inside it.' },
      favGroupSearch: { title: 'Group Search', desc: 'Search for topics or notes within the current group.' },
      favItemRow: { title: 'Favorite Item', desc: 'Double-click to jump to the topic messages page.' },
      favEditItem: { title: 'Edit Favorite', desc: 'Change the favorite topic group, description, or sort order.' },
      favDeleteItem: { title: 'Delete Favorite', desc: 'Remove this topic from your favorites.' },
      favEmptyState: { title: 'No Favorites', desc: "You haven't created any favorite groups yet. Click 'Create Group' to get started." },
      favGroupModal: { title: 'Group Edit Dialog', desc: 'Set the group name, description, and sort priority.' },
      favFavoriteModal: { title: 'Favorite Edit Dialog', desc: 'Move the favorite to another group or add a description.' },
    },
  },
};
