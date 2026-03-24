export type Language = 'zh' | 'en';

export interface Translation {
  nav: {
    clusters: string;
    favorites: string;
    topics: string;
    messages: string;
    settings: string;
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
    confirmDeleteTitle: string;
    confirmDeleteHint: string;
    confirmDeleteInput: string;
    confirmDeleteMatchError: string;
    copied: string;
    copyFailed: string;
    deletedSuccess: string;
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
  };
  settings: {
    title: string;
    description: string;
    language: string;
    theme: string;
    languageZh: string;
    languageEn: string;
    selectLanguage: string;
    sidebarMode: string;
    selectSidebarMode: string;
    treeMode: string;
    flatMode: string;
    treeModeDesc: string;
    flatModeDesc: string;
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
  };
  layout: {
    searchPlaceholder: string;
    noTopicsFound: string;
    settings: string;
    confirmDeleteCluster: string;
    confirmDeleteTopic: string;
    clusterNotFound: string;
    topicNotFound: string;
    refreshFailed: string;
    refreshCancelled: string;
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
      confirmDeleteGroup: '删除后，该分组下的所有集群将变为无分组状态。',
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
      confirmDeleteTitle: '删除主题',
      confirmDeleteHint: '删除后无法恢复',
      confirmDeleteInput: '输入主题名称以确认',
      confirmDeleteMatchError: '输入的主题名称不匹配',
      copied: '已复制',
      copyFailed: '复制失败',
      deletedSuccess: '主题已删除',
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
      searchPlaceholder: '可选',
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
      recent5Minutes: '最近 5 分钟',
      recent15Minutes: '最近 15 分钟',
      recent30Minutes: '最近 30 分钟',
      recent1Hour: '最近 1 小时',
      recent1Day: '最近 1 天',
      topicLabel: '主题',
      cluster: '集群',
      minutes: '分',
      hour: '时',
      day: '天',
      queryTimeout: '查询超时，请重试',
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
    },
    settings: {
      title: '设置',
      description: '管理全局设置',
      language: '语言',
      theme: '主题',
      languageZh: '中文',
      languageEn: 'English',
      selectLanguage: '选择语言',
      sidebarMode: '侧边栏模式',
      selectSidebarMode: '选择侧边栏显示模式',
      treeMode: '树形模式',
      flatMode: '列表模式',
      treeModeDesc: '按集群分组显示主题',
      flatModeDesc: '平铺显示所有主题',
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
    },
    layout: {
      searchPlaceholder: '搜索主题... (Ctrl+K)',
      noTopicsFound: '未找到主题',
      settings: '设置',
      confirmDeleteCluster: '确定要删除集群 "{cluster}" 吗？',
      confirmDeleteTopic: '确定要删除主题 "{topic}" 吗？',
      clusterNotFound: '集群不存在',
      topicNotFound: '主题不存在',
      refreshFailed: '刷新失败',
      refreshCancelled: '刷新已取消',
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
      confirmDeleteGroup: 'After deletion, all clusters in this group will become ungrouped.',
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
      confirmDeleteTitle: 'Delete Topic',
      confirmDeleteHint: 'This action cannot be undone',
      confirmDeleteInput: 'Type the topic name to confirm',
      confirmDeleteMatchError: 'Topic name does not match',
      copied: 'Copied',
      copyFailed: 'Copy failed',
      deletedSuccess: 'Topic deleted successfully',
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
      searchPlaceholder: 'Optional',
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
      recent5Minutes: 'Last 5 minutes',
      recent15Minutes: 'Last 15 minutes',
      recent30Minutes: 'Last 30 minutes',
      recent1Hour: 'Last 1 hour',
      recent1Day: 'Last 1 day',
      topicLabel: 'Topic',
      cluster: 'Cluster',
      minutes: 'min',
      hour: 'hr',
      day: 'day',
      queryTimeout: 'Query timeout, please try again',
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
    },
    settings: {
      title: 'Settings',
      description: 'Manage global settings',
      language: 'Language',
      theme: 'Theme',
      languageZh: '中文',
      languageEn: 'English',
      selectLanguage: 'Select Language',
      sidebarMode: 'Sidebar Mode',
      selectSidebarMode: 'Select Sidebar Display Mode',
      treeMode: 'Tree Mode',
      flatMode: 'List Mode',
      treeModeDesc: 'Display topics grouped by cluster',
      flatModeDesc: 'Display all topics in a flat list',
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
    },
    layout: {
      searchPlaceholder: 'Search topics... (Ctrl+K)',
      noTopicsFound: 'No topics found',
      settings: 'Settings',
      confirmDeleteCluster: 'Are you sure you want to remove cluster "{cluster}"?',
      confirmDeleteTopic: 'Are you sure you want to delete topic "{topic}"?',
      clusterNotFound: 'Cluster not found',
      topicNotFound: 'Topic not found',
      refreshFailed: 'Refresh failed',
      refreshCancelled: 'Refresh cancelled',
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
    },
  },
};
