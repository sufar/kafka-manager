export type Language = 'zh' | 'en';

export interface Translation {
  nav: {
    dashboard: string;
    clusters: string;
    topics: string;
    messages: string;
    consumers: string;
    consumerGroups: string;
    consumerLag: string;
    schemaRegistry: string;
    users: string;
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
  };
  dashboard: {
    title: string;
    description: string;
    totalClusters: string;
    totalTopics: string;
    totalPartitions: string;
    consumerGroups: string;
    totalLag: string;
    healthyClusters: string;
    unhealthyClusters: string;
    partitionsPerTopic: string;
    clusters: string;
    selectAll: string;
    deselectAll: string;
    topics: string;
    partitions: string;
    lag: string;
    lastChecked: string;
    consumerLagTitle: string;
    last24Hours: string;
    last7Days: string;
    last30Days: string;
    chartComingSoon: string;
    byCluster: string;
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
    viewConsumerGroups: string;
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
  };
  topics: {
    title: string;
    description: string;
    createTopic: string;
    topicName: string;
    partitionCount: string;
    replicationFactor: string;
    cleanupPolicy: string;
    retentionMs: string;
    retentionBytes: string;
    segmentBytes: string;
    viewMessages: string;
    viewDetails: string;
    viewPartitions: string;
    viewConsumerLag: string;
    sendMessage: string;
    deleteTopic: string;
    exportData: string;
    allTopics: string;
    topicDetails: string;
    partitions: string;
    consumers: string;
    settings: string;
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
    fetchMode: string;
    oldest: string;
    newest: string;
    maxMessages: string;
    perPartition: string;
    startTime: string;
    endTime: string;
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
  };
  consumerGroups: {
    title: string;
    description: string;
    groupId: string;
    state: string;
    members: string;
    totalLag: string;
    coordinator: string;
    topics: string;
    resetOffset: string;
    deleteGroup: string;
    groupDetails: string;
    partitions: string;
    offsetInfo: string;
    startOffset: string;
    endOffset: string;
    currentOffset: string;
    lag: string;
    partition: string;
    stateLabel: string;
    selectCluster: string;
    noActiveMembers: string;
    noGroupsFound: string;
    noGroupsInCluster: string;
    groups: string;
    offsetType: string;
    earliest: string;
    latest: string;
    specificValue: string;
    value: string;
    allPartitionsPlaceholder: string;
  };
  settings: {
    title: string;
    description: string;
    language: string;
    theme: string;
    languageZh: string;
    languageEn: string;
    selectLanguage: string;
    messageViewMode: string;
    selectMessageViewMode: string;
    classicMode: string;
    simpleMode: string;
    classicModeDesc: string;
    simpleModeDesc: string;
    sidebarMode: string;
    selectSidebarMode: string;
    treeMode: string;
    flatMode: string;
    treeModeDesc: string;
    flatModeDesc: string;
  };
  notifications: {
    title: string;
    description: string;
    addNotification: string;
    editNotification: string;
    name: string;
    type: string;
    enabled: string;
    disabled: string;
    enable: string;
    disable: string;
    edit: string;
    delete: string;
    update: string;
    create: string;
    cancel: string;
    config: string;
    webhookUrl: string;
    channel: string;
    url: string;
    method: string;
    smtpServer: string;
    port: string;
    fromEmail: string;
    toEmails: string;
    commaSeparatedEmails: string;
    noNotifications: string;
    noNotificationsDesc: string;
    confirmDelete: string;
    updated: string;
    created: string;
    deleted: string;
    updatedDesc: string;
    createdDesc: string;
  };
  schemaRegistry: {
    title: string;
    description: string;
    registerSchema: string;
    deleteSchema: string;
    deleteVersion: string;
    view: string;
    copy: string;
    close: string;
    subjectName: string;
    schemaType: string;
    schemaDefinition: string;
    versions: string;
    type: string;
    schemaId: string;
    noSchemasFound: string;
    noSchemasFoundDesc: string;
    selectClusterDesc: string;
    confirmDeleteSubject: string;
    confirmDeleteVersion: string;
    schemaCopied: string;
  };
  users: {
    title: string;
    description: string;
    showUsers: string;
    showRoles: string;
    createUser: string;
    createRole: string;
    editUser: string;
    editRole: string;
    username: string;
    email: string;
    role: string;
    status: string;
    active: string;
    inactive: string;
    noRole: string;
    created: string;
    actions: string;
    edit: string;
    activate: string;
    deactivate: string;
    noUsers: string;
    noUsersDesc: string;
    noRoles: string;
    noRolesDesc: string;
    roleName: string;
    roleDescription: string;
    permissions: string;
    noDescription: string;
    cancel: string;
    update: string;
    create: string;
    updated: string;
    password: string;
    selectRole: string;
  };
  consumerLag: {
    title: string;
    cluster: string;
    refresh: string;
    totalLag: string;
    consumerGroups: string;
    groups: string;
    partitions: string;
    maxLagGroup: string;
    lagTrend: string;
    loadingData: string;
    noHistoricalData: string;
    consumerGroupDetails: string;
    partitionDetails: string;
    noConsumerGroups: string;
    noConsumerGroupsDesc: string;
    messages: string;
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
  };
  topicContextMenu: {
    viewMessages: string;
    viewDetails: string;
    viewPartitions: string;
    viewConsumerLag: string;
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
    consumerGroups: string;
    messages: string;
    schemaRegistry: string;
    acls: string;
  };
}

export const translations: Record<Language, Translation> = {
  zh: {
    nav: {
      dashboard: '仪表盘',
      clusters: '集群',
      topics: '主题',
      messages: '消息',
      consumers: '消费者',
      consumerGroups: '消费者组',
      consumerLag: '消费延迟',
      schemaRegistry: 'Schema 注册',
      users: '用户',
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
    },
    dashboard: {
      title: '仪表盘',
      description: 'Kafka 集群概览',
      totalClusters: '集群总数',
      totalTopics: '主题总数',
      totalPartitions: '分区总数',
      consumerGroups: '消费者组',
      totalLag: '总延迟',
      healthyClusters: '健康',
      unhealthyClusters: '不健康',
      partitionsPerTopic: '平均每主题分区',
      clusters: '集群',
      selectAll: '全选',
      deselectAll: '取消全选',
      topics: '主题',
      partitions: '分区',
      lag: '延迟',
      lastChecked: '最后检查',
      consumerLagTitle: '消费延迟概览',
      last24Hours: '过去 24 小时',
      last7Days: '过去 7 天',
      last30Days: '过去 30 天',
      chartComingSoon: '图表功能即将推出',
      byCluster: '按集群',
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
      viewConsumerGroups: '查看消费者组',
      createTopic: '创建 Topic',
      connectionError: '连接错误',
      retry: '重试',
      brokersLabel: 'Brokers',
      timeoutsLabel: '超时设置',
      confirmDelete: '确定要删除集群',
      disconnectConfirm: '确定要断开集群连接',
      reconnectFailed: '重连失败',
      fetchFailed: '获取失败',
      connectedLabel: '已连接',
      disconnectedLabel: '未连接',
      connectingLabel: '连接中',
      updated: '集群已更新',
      created: '集群已创建',
      connected: '连接成功',
    },
    topics: {
      title: '主题',
      description: '管理 Kafka 主题',
      createTopic: '创建主题',
      topicName: '主题名称',
      partitionCount: '分区数',
      replicationFactor: '副本因子',
      cleanupPolicy: '清理策略',
      retentionMs: '保留时间 (ms)',
      retentionBytes: '保留大小 (bytes)',
      segmentBytes: '段大小 (bytes)',
      viewMessages: '查看消息',
      viewDetails: '查看详情',
      viewPartitions: '查看分区',
      viewConsumerLag: '查看消费延迟',
      sendMessage: '发送消息',
      deleteTopic: '删除主题',
      exportData: '导出数据',
      allTopics: '所有主题',
      topicDetails: '主题详情',
      partitions: '分区',
      consumers: '消费者',
      settings: '设置',
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
      fetchMode: '获取模式',
      oldest: '最早',
      newest: '最新',
      maxMessages: '最大消息数',
      perPartition: '每分区',
      startTime: '开始时间',
      endTime: '结束时间',
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
    },
    consumerGroups: {
      title: '消费者组',
      description: '管理消费者组',
      groupId: '组 ID',
      state: '状态',
      members: '成员',
      totalLag: '总延迟',
      coordinator: '协调器',
      topics: '主题',
      resetOffset: '重置偏移量',
      deleteGroup: '删除组',
      groupDetails: '组详情',
      partitions: '分区',
      offsetInfo: '偏移量信息',
      startOffset: '起始偏移量',
      endOffset: '结束偏移量',
      currentOffset: '当前偏移量',
      lag: '延迟',
      partition: '分区',
      stateLabel: '状态',
      selectCluster: '请从侧边栏选择一个集群来查看消费者组',
      noActiveMembers: '没有活跃成员 - 该消费者组似乎为空',
      noGroupsFound: '消费者组将在消费者连接到集群时出现',
      noGroupsInCluster: '该集群中没有消费者组',
      groups: '组',
      offsetType: '偏移量类型',
      earliest: '最早',
      latest: '最新',
      specificValue: '指定值',
      value: '值',
      allPartitionsPlaceholder: '留空表示所有分区',
    },
    settings: {
      title: '设置',
      description: '管理全局设置',
      language: '语言',
      theme: '主题',
      languageZh: '中文',
      languageEn: 'English',
      selectLanguage: '选择语言',
      messageViewMode: '消息界面模式',
      selectMessageViewMode: '选择消息界面模式',
      classicMode: '经典模式',
      simpleMode: '简洁模式',
      classicModeDesc: '功能完整，适合复杂操作',
      simpleModeDesc: '轻量快速，适合日常查询',
      sidebarMode: '侧边栏模式',
      selectSidebarMode: '选择侧边栏显示模式',
      treeMode: '树形模式',
      flatMode: '列表模式',
      treeModeDesc: '按集群分组显示主题',
      flatModeDesc: '平铺显示所有主题',
    },
    notifications: {
      title: '通知设置',
      description: '配置告警通知渠道',
      addNotification: '添加通知',
      editNotification: '编辑通知',
      name: '名称',
      type: '类型',
      enabled: '已启用',
      disabled: '已禁用',
      enable: '启用',
      disable: '禁用',
      edit: '编辑',
      delete: '删除',
      update: '更新',
      create: '创建',
      cancel: '取消',
      config: '配置',
      webhookUrl: 'Webhook 地址',
      channel: '频道',
      url: '地址',
      method: '方法',
      smtpServer: 'SMTP 服务器',
      port: '端口',
      fromEmail: '发件人邮箱',
      toEmails: '收件人邮箱',
      commaSeparatedEmails: '逗号分隔的邮箱地址',
      noNotifications: '暂无通知渠道',
      noNotificationsDesc: '配置通知渠道以接收告警',
      confirmDelete: '确定要删除此通知渠道吗？',
      updated: '通知已更新',
      created: '通知已创建',
      deleted: '通知已删除',
      updatedDesc: '通知配置已成功更新',
      createdDesc: '通知配置已成功创建',
    },
    schemaRegistry: {
      title: 'Schema 注册',
      description: '管理 Avro、JSON 和 Protobuf Schema',
      registerSchema: '注册 Schema',
      deleteSchema: '删除 Schema',
      deleteVersion: '删除版本',
      view: '查看',
      copy: '复制',
      close: '关闭',
      subjectName: 'Subject 名称',
      schemaType: 'Schema 类型',
      schemaDefinition: 'Schema 定义',
      versions: '版本',
      type: '类型',
      schemaId: 'Schema ID',
      noSchemasFound: '暂无 Schema',
      noSchemasFoundDesc: '注册您的第一个 Schema 以开始使用',
      selectClusterDesc: '请从侧边栏选择集群以查看 Schema',
      confirmDeleteSubject: '确定要删除 subject "{subject}" 的所有版本吗？',
      confirmDeleteVersion: '确定要删除 subject "{subject}" 的版本 {version} 吗？',
      schemaCopied: 'Schema 已复制到剪贴板！',
    },
    users: {
      title: '用户管理',
      description: '管理用户和角色',
      showUsers: '显示用户',
      showRoles: '显示角色',
      createUser: '创建用户',
      createRole: '创建角色',
      editUser: '编辑用户',
      editRole: '编辑角色',
      username: '用户名',
      email: '邮箱',
      role: '角色',
      status: '状态',
      active: '活跃',
      inactive: '未激活',
      noRole: '无角色',
      created: '创建时间',
      actions: '操作',
      edit: '编辑',
      activate: '激活',
      deactivate: '停用',
      noUsers: '暂无用户',
      noUsersDesc: '创建您的第一个用户以开始使用',
      noRoles: '暂无角色',
      noRolesDesc: '创建角色以管理用户权限',
      roleName: '角色名称',
      roleDescription: '描述',
      permissions: '权限',
      noDescription: '暂无描述',
      cancel: '取消',
      update: '更新',
      create: '创建',
      updated: '用户已更新',
      password: '密码',
      selectRole: '选择角色',
    },
    consumerLag: {
      title: '消费延迟',
      cluster: '集群',
      refresh: '刷新',
      totalLag: '总延迟',
      consumerGroups: '消费者组',
      groups: '组',
      partitions: '分区',
      maxLagGroup: '最大延迟组',
      lagTrend: '延迟趋势',
      loadingData: '加载数据中...',
      noHistoricalData: '暂无历史数据',
      consumerGroupDetails: '消费者组详情',
      partitionDetails: '分区详情',
      noConsumerGroups: '暂无消费者组',
      noConsumerGroupsDesc: '此主题暂无消费者组',
      messages: '消息',
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
    },
    topicContextMenu: {
      viewMessages: '查看消息',
      viewDetails: '查看详情',
      viewPartitions: '查看分区',
      viewConsumerLag: '查看消费延迟',
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
      consumerGroups: '消费者组',
      messages: '消息',
      schemaRegistry: 'Schema 注册',
      acls: 'ACLs',
    },
  },
  en: {
    nav: {
      dashboard: 'Dashboard',
      clusters: 'Clusters',
      topics: 'Topics',
      messages: 'Messages',
      consumers: 'Consumers',
      consumerGroups: 'Consumer Groups',
      consumerLag: 'Consumer Lag',
      schemaRegistry: 'Schema Registry',
      users: 'Users',
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
    },
    dashboard: {
      title: 'Dashboard',
      description: 'Overview of your Kafka clusters',
      totalClusters: 'Total Clusters',
      totalTopics: 'Total Topics',
      totalPartitions: 'Total Partitions',
      consumerGroups: 'Consumer Groups',
      totalLag: 'Total Lag',
      healthyClusters: 'healthy',
      unhealthyClusters: 'unhealthy',
      partitionsPerTopic: 'partitions/topic avg',
      clusters: 'Clusters',
      selectAll: 'Select All',
      deselectAll: 'Deselect All',
      topics: 'Topics',
      partitions: 'Partitions',
      lag: 'Lag',
      lastChecked: 'Last checked',
      consumerLagTitle: 'Consumer Lag Overview',
      last24Hours: 'Last 24 hours',
      last7Days: 'Last 7 days',
      last30Days: 'Last 30 days',
      chartComingSoon: 'Chart integration coming soon',
      byCluster: 'By Cluster',
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
      viewConsumerGroups: 'View Consumer Groups',
      createTopic: 'Create Topic',
      connectionError: 'Connection Error',
      retry: 'Retry',
      brokersLabel: 'Brokers',
      timeoutsLabel: 'Timeouts',
      confirmDelete: 'Are you sure you want to delete cluster',
      disconnectConfirm: 'Are you sure you want to disconnect cluster',
      reconnectFailed: 'Reconnect failed',
      fetchFailed: 'Fetch failed',
      connectedLabel: 'Connected',
      disconnectedLabel: 'Disconnected',
      connectingLabel: 'Connecting',
      updated: 'Cluster updated',
      created: 'Cluster created',
      connected: 'Connection successful',
    },
    topics: {
      title: 'Topics',
      description: 'Manage Kafka topics',
      createTopic: 'Create Topic',
      topicName: 'Topic Name',
      partitionCount: 'Partition Count',
      replicationFactor: 'Replication Factor',
      cleanupPolicy: 'Cleanup Policy',
      retentionMs: 'Retention (ms)',
      retentionBytes: 'Retention (bytes)',
      segmentBytes: 'Segment Size (bytes)',
      viewMessages: 'View Messages',
      viewDetails: 'View Details',
      viewPartitions: 'View Partitions',
      viewConsumerLag: 'View Consumer Lag',
      sendMessage: 'Send Message',
      deleteTopic: 'Delete Topic',
      exportData: 'Export Data',
      allTopics: 'All Topics',
      topicDetails: 'Topic Details',
      partitions: 'Partitions',
      consumers: 'Consumers',
      settings: 'Settings',
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
      fetchMode: 'Fetch Mode',
      oldest: 'Oldest',
      newest: 'Newest',
      maxMessages: 'Max Messages',
      perPartition: 'Per Partition',
      startTime: 'Start Time',
      endTime: 'End Time',
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
    },
    consumerGroups: {
      title: 'Consumer Groups',
      description: 'Manage consumer groups',
      groupId: 'Group ID',
      state: 'State',
      members: 'Members',
      totalLag: 'Total Lag',
      coordinator: 'Coordinator',
      topics: 'Topics',
      resetOffset: 'Reset Offset',
      deleteGroup: 'Delete Group',
      groupDetails: 'Group Details',
      partitions: 'Partitions',
      offsetInfo: 'Offset Info',
      startOffset: 'Start Offset',
      endOffset: 'End Offset',
      currentOffset: 'Current Offset',
      lag: 'Lag',
      partition: 'Partition',
      stateLabel: 'State',
      selectCluster: 'Select a cluster from the sidebar to view consumer groups',
      noActiveMembers: 'No active members - this consumer group appears to be empty',
      noGroupsFound: 'Consumer groups will appear when consumers connect to the cluster',
      noGroupsInCluster: 'No consumer groups in this cluster',
      groups: 'groups',
      offsetType: 'Offset Type',
      earliest: 'Earliest',
      latest: 'Latest',
      specificValue: 'Specific Value',
      value: 'Value',
      allPartitionsPlaceholder: 'Leave empty for all partitions',
    },
    settings: {
      title: 'Settings',
      description: 'Manage global settings',
      language: 'Language',
      theme: 'Theme',
      languageZh: '中文',
      languageEn: 'English',
      selectLanguage: 'Select Language',
      messageViewMode: 'Message View Mode',
      selectMessageViewMode: 'Select Message View Mode',
      classicMode: 'Classic Mode',
      simpleMode: 'Simple Mode',
      classicModeDesc: 'Full featured, suitable for complex operations',
      simpleModeDesc: 'Lightweight and fast, suitable for daily queries',
      sidebarMode: 'Sidebar Mode',
      selectSidebarMode: 'Select Sidebar Display Mode',
      treeMode: 'Tree Mode',
      flatMode: 'List Mode',
      treeModeDesc: 'Display topics grouped by cluster',
      flatModeDesc: 'Display all topics in a flat list',
    },
    notifications: {
      title: 'Notification Settings',
      description: 'Configure alert notification channels',
      addNotification: 'Add Notification',
      editNotification: 'Edit Notification',
      name: 'Name',
      type: 'Type',
      enabled: 'Enabled',
      disabled: 'Disabled',
      enable: 'Enable',
      disable: 'Disable',
      edit: 'Edit',
      delete: 'Delete',
      update: 'Update',
      create: 'Create',
      cancel: 'Cancel',
      config: 'Config',
      webhookUrl: 'Webhook URL',
      channel: 'Channel',
      url: 'URL',
      method: 'Method',
      smtpServer: 'SMTP Server',
      port: 'Port',
      fromEmail: 'From Email',
      toEmails: 'To Emails',
      commaSeparatedEmails: 'Comma separated emails',
      noNotifications: 'No Notification Channels',
      noNotificationsDesc: 'Configure notification channels to receive alerts',
      confirmDelete: 'Are you sure you want to delete this notification channel?',
      updated: 'Notification updated',
      created: 'Notification created',
      deleted: 'Notification deleted',
      updatedDesc: 'Notification configuration has been updated successfully',
      createdDesc: 'Notification configuration has been created successfully',
    },
    schemaRegistry: {
      title: 'Schema Registry',
      description: 'Manage Avro, JSON, and Protobuf schemas',
      registerSchema: 'Register Schema',
      deleteSchema: 'Delete Schema',
      deleteVersion: 'Delete Version',
      view: 'View',
      copy: 'Copy',
      close: 'Close',
      subjectName: 'Subject Name',
      schemaType: 'Schema Type',
      schemaDefinition: 'Schema Definition',
      versions: 'versions',
      type: 'Type',
      schemaId: 'Schema ID',
      noSchemasFound: 'No Schemas Found',
      noSchemasFoundDesc: 'Register your first schema to get started',
      selectClusterDesc: 'Please select a cluster from the sidebar to view schemas',
      confirmDeleteSubject: 'Are you sure you want to delete all versions of subject "{subject}"?',
      confirmDeleteVersion: 'Are you sure you want to delete version {version} of subject "{subject}"?',
      schemaCopied: 'Schema copied to clipboard!',
    },
    users: {
      title: 'User Management',
      description: 'Manage users and roles',
      showUsers: 'Show Users',
      showRoles: 'Show Roles',
      createUser: 'Create User',
      createRole: 'Create Role',
      editUser: 'Edit User',
      editRole: 'Edit Role',
      username: 'Username',
      email: 'Email',
      role: 'Role',
      status: 'Status',
      active: 'Active',
      inactive: 'Inactive',
      noRole: 'No Role',
      created: 'Created',
      actions: 'Actions',
      edit: 'Edit',
      activate: 'Activate',
      deactivate: 'Deactivate',
      noUsers: 'No Users',
      noUsersDesc: 'Create your first user to get started',
      noRoles: 'No Roles',
      noRolesDesc: 'Create roles to manage user permissions',
      roleName: 'Role Name',
      roleDescription: 'Description',
      permissions: 'Permissions',
      noDescription: 'No description',
      cancel: 'Cancel',
      update: 'Update',
      create: 'Create',
      updated: 'User updated',
      password: 'Password',
      selectRole: 'Select Role',
    },
    consumerLag: {
      title: 'Consumer Lag',
      cluster: 'Cluster',
      refresh: 'Refresh',
      totalLag: 'Total Lag',
      consumerGroups: 'Consumer Groups',
      groups: 'groups',
      partitions: 'partitions',
      maxLagGroup: 'Max Lag Group',
      lagTrend: 'Consumer Lag Trend',
      loadingData: 'Loading data...',
      noHistoricalData: 'No historical data available',
      consumerGroupDetails: 'Consumer Group Details',
      partitionDetails: 'Partition Details',
      noConsumerGroups: 'No consumer groups',
      noConsumerGroupsDesc: 'No consumer groups found for this topic',
      messages: 'messages',
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
    },
    topicContextMenu: {
      viewMessages: 'View Messages',
      viewDetails: 'View Details',
      viewPartitions: 'View Partitions',
      viewConsumerLag: 'View Consumer Lag',
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
      consumerGroups: 'Consumer Groups',
      messages: 'Messages',
      schemaRegistry: 'Schema Registry',
      acls: 'ACLs',
    },
  },
};
