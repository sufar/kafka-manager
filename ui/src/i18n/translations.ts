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
  };
  settings: {
    title: string;
    description: string;
    language: string;
    theme: string;
    languageZh: string;
    languageEn: string;
    selectLanguage: string;
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
    },
    settings: {
      title: '设置',
      description: '管理全局设置',
      language: '语言',
      theme: '主题',
      languageZh: '中文',
      languageEn: 'English',
      selectLanguage: '选择语言',
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
    },
    settings: {
      title: 'Settings',
      description: 'Manage global settings',
      language: 'Language',
      theme: 'Theme',
      languageZh: '中文',
      languageEn: 'English',
      selectLanguage: 'Select Language',
    },
  },
};
