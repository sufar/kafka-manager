# Kafka Manager GPUI Migration - 功能点规格文档

> 本文档记录所有前端功能点，防止迁移到 GPUI 时功能丢失。
> 
> 生成日期: 2026-05-24
> 项目版本: 1.1.47

---

## 1. 核心视图 (Views)

### 1.1 ClustersView - 集群管理页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| 集群列表展示 | 卡片式展示所有集群，包含名称、Brokers、超时配置、连接状态 | `cluster.list` | 分组筛选支持 |
| 创建集群 | Modal 弹窗表单，填写名称、Brokers、超时配置、所属分组 | `cluster.create` | 名称验证(15字符、字母数字中文连字符下划线) |
| 编辑集群 | Modal 弹窗修改集群配置 | `cluster.update` | 同创建表单 |
| 删除集群 | 确认弹窗后删除 | `cluster.delete` | 需要 confirm |
| 测试连接 | 测试集群连通性，显示成功/失败状态 | `cluster.test` / `cluster.test_config` | 创建前可测试 |
| 断开连接 | 断开集群连接 | `connection.disconnect` | 需要确认 |
| 重新连接 | 重连集群 | `connection.reconnect` | |
| 查看 Topics | 跳转到 Topics 页面，筛选该集群 | | 路由跳转 |
| 刷新 Topics | 后台刷新集群 Topic 元数据 | `topic.refresh` | Fire-and-forget |
| 创建 Topic | 调用 CreateTopicDialog | `topic.create` | Modal |
| 集群分组管理 | 分组的 CRUD（创建、编辑、删除） | `cluster_group.*` | Modal 内嵌表单 |
| 分组筛选 | 按分组筛选集群列表 | | 横向滚动分组标签 |
| 集群连接状态徽章 | 显示 connected/disconnected/error | `connection.list` | 实时状态 |
| 错误信息展示 | 连接失败时显示错误信息 | | Alert 样式 |

### 1.2 TopicsView - Topic 管理页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| Topic 列表 | 单集群 Topic 列表，虚拟滚动 | `topic.list` | URL query.cluster 参数 |
| 搜索筛选 | 按名称搜索，防抖 150ms | | 本地过滤 |
| 创建 Topic | CreateTopicDialog 弹窗 | `topic.create` | |
| 删除 Topic | DeleteTopicDialog 确认弹窗 | `topic.delete` | |
| 刷新 Topic | 刷新单个或全部 Topic 元数据 | `topic.refresh` | Fire-and-forget |
| 收藏按钮 | FavoriteButton 组件 | `favorite.*` | 状态缓存 |
| 双击导航 | 双击 Topic 跳转到消息页面 | | 触发 select-topic-in-tree 事件 |
| 虚拟滚动 | 大量 Topic 时仅渲染可见区域 | | 40px 行高 |

### 1.3 ConsumerGroupsView - 消费者组页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| 消费者组详情 | 显示 Group 状态、Topics | `consumer_group.get` | URL query.group 参数 |
| Offset 表格 | 显示 partition/start/end/committed/lag/last_commit | `consumer_group.offsets` | 可调整列宽 |
| 列宽调整 | 拖动调整表格列宽 | | 鼠标拖动 |
| 刷新 Offset | 手动刷新 offset 数据 | `consumer_group.refresh_offsets` | |
| 重置 Offset | 弹窗选择 topic/partition/resetTo(offset/earliest/latest/timestamp) | `consumer_group.reset_offset` | Modal |
| 删除消费者组 | 确认后删除 | `consumer_group.delete` | 需要 confirm |
| 状态徽章 | stable/empty(绿), rebalance(黄), dead/unknown(红) | | 颜色编码 |
| Lag 颜色 | 0(绿), <1000(黄), >1000(红) | | |
| 操作菜单 | Actions 下拉菜单(重置/删除) | | Teleport 到 body |

### 1.4 MessagesView - 消息查询页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| Topic 消息查询 | SSE 流式获取消息 | `message.list` (SSE) | `/api/stream` |
| 分区选择 | 全部或指定分区 | | Select |
| 查询模式 | newest/oldest | | Select |
| 消息数量限制 | 1-1000，保存到 settings | `settings.get/update` | |
| 关键词搜索 | 搜索 value 内容 | | Input |
| 时间范围筛选 | start_time/end_time | | 日期时间输入 |
| 预设时间 | 5分钟/15分钟/30分钟/1小时/1天 | | 快捷按钮 |
| 停止查询 | 中断 SSE 流 | AbortController | |
| 消息列表 | 虚拟滚动表格，分区/offset/时间戳/key/value | | vue-virtual-scroller |
| 时间戳排序 | asc/desc/null 三种状态切换 | | 点击表头 |
| 列宽调整 | 拖动调整各列宽度 | | |
| 消息详情面板 | 底部可调整高度面板 | | 拖动 resize |
| 详情搜索 | Ctrl+F 搜索详情内容 | | mark 高亮 |
| Value 格式切换 | json/raw/hex 三种格式 | | Select |
| JSON 高亮 | 模板化的 JSON 语法高亮 | `json_highlight.*` | 可自定义模板 |
| 复制消息 | 复制 key 或 value | | 剪贴板 |
| 导出消息 | 导出为 JSON 文件 | `message.export` | Tauri 原生对话框 |
| 发送消息 | SendMessageModal 弹窗 | `message.send` | |
| 发送历史 | SentMessageHistory 面板 | `sent_message.list` | |
| 查看消费者组 | 跳转到 TopicConsumerGroups 页面 | | |
| 删除 Topic | DeleteTopicDialog | `topic.delete` | |
| 收藏按钮 | FavoriteButton | | |
| 流式进度 | 显示已接收数量/总数/进度条 | | 实时更新 |
| 键盘导航 | 方向键上下选择消息 | | |
| 执行时间 | 显示查询耗时(ms) | | |

### 1.5 SettingsView - 设置页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| 语言切换 | 中文/英文 | `LanguageSelector` | 本地 localStorage |
| 主题切换 | Dark/Light toggle | `toggleTheme` | Tailwind dark mode |
| 侧边栏模式 | tree/flat toggle | `settings.update(ui.sidebar_mode)` | 触发事件 |
| 系统托盘 | 开关系统托盘(关闭按钮行为) | `settings.update(ui.system_tray)` | Tauri only |
| 开机自启动 | Windows 注册表设置 | Tauri `set_auto_launch` | Windows only |
| 版本显示 | 显示当前版本号 | Tauri `get_app_version` | |
| 检查更新 | 检查 GitHub Release | Tauri `check_for_updates` | |
| 下载更新 | 后台下载，进度显示 | Tauri `install_update` + polling | 断点续传 |
| 更新弹窗 | 显示新版本、Release Notes | | Modal |
| 取消下载 | 中断下载 | Tauri `clear_download_status` | |
| 查看日志 | 隐藏功能(点击版本号5次) | Tauri `get_app_logs` | Modal |
| 复制日志 | 复制日志到剪贴板 | | |
| 刷新日志 | 刷新日志内容 | | |
| 清除日志 | 清空日志文件 | Tauri `clear_app_logs` | |
| 导出数据 | 导出 cluster_groups/clusters/topics/favorites/history | `settings.export` | JSON 文件下载 |
| 导入数据 | 导入 JSON 文件 | `settings.import` | File input |
| JSON 高亮模板选择 | 预设模板 + 自定义模板 | `json_highlight.*` | JsonHighlightSelector |

### 1.6 SchemaRegistryView - Schema Registry 页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| 集群选择 | 选择配置 Schema Registry 的集群 | | 路由 query.cluster |
| 配置管理 | Registry URL + 用户名密码 | `schema_registry.config.*` | Modal |
| 测试连接 | 测试 Registry 连通性 | `schema_registry.config.test` | |
| Subjects 列表 | 显示所有 Schema Subject | `schema_registry.list` | 表格 |
| 查看 Schema | 查看 Schema JSON 内容 | `schema_registry.get_latest` | Modal |
| 注册 Schema | 新 Subject/版本注册 | `schema_registry.register` | Modal |
| 测试兼容性 | 测试新 Schema 与现有版本的兼容性 | `schema_registry.compatibility.test` | |
| 删除 Schema | 删除 Subject 所有版本 | `schema_registry.delete` | 需要 confirm |
| Schema 类型 | AVRO/PROTOBUF/JSON | | Select |
| 连接状态指示 | 绿/红圆点显示连接状态 | | |

### 1.7 FavoritesView - 收藏页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| 收藏分组列表 | 显示所有收藏分组 | `favorite.list` | 可折叠 |
| 分组 CRUD | 创建/编辑/删除收藏分组 | `favorite.group.*` | |
| 收藏项 CRUD | 添加/删除/移动收藏项 | `favorite.create/delete/update` | |
| 导航到 Topic | 点击收藏项跳转到消息页面 | | 触发事件 |
| 分组内排序 | 调整收藏项顺序 | | sort_order |

### 1.8 TopicConsumerGroupsView - Topic 消费者组页面

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| Topic 的消费者组列表 | 显示订阅该 Topic 的所有消费者组 | `consumer_group.list_by_topic` | 表格 |
| Lag 详情 | 每个分区的 lag 信息 | | |

---

## 2. 布局组件 (Layout Components)

### 2.1 ModernLayout - 主布局

| 功能点 | 描述 | 备注 |
|--------|------|------|
| 顶部导航栏 | TopNavBar | |
| 左侧边栏 | LeftSidebar (tree/flat 模式) | 可调整宽度 |
| 右侧主内容区 | router-view | |
| 上下文菜单 | ContextMenus (右键菜单) | |
| Toast 提示 | ToastAndConfirm | |
| Tour 新手引导 | TourOverlay | |
| 移动端搜索抽屉 | MobileSearchDrawer | |
| 移动端响应式 | <768px 隐藏侧边栏，显示汉堡菜单 | |
| 状态持久化 | localStorage 保存最后路由状态 | |
| 分享版本 | 复制安装包到下载目录 | Tauri only |

### 2.2 LeftSidebar - 左侧边栏

| 功能点 | 描述 | 备注 |
|--------|------|------|
| 宽度调整 | 拖动 resizer 调整宽度 | 133px - 80vw |
| Tree 模式 | ClusterTreeNavigator | |
| Flat 模式 | TopicNavigator | |
| 移动端抽屉 | 固定定位抽屉式侧边栏 | |

### 2.3 TopNavBar - 顶部导航

| 功能点 | 描述 | 备注 |
|--------|------|------|
| 搜索框 | 全局搜索 Topics/ConsumerGroups | `topic.search` / `consumer_group.search` |
| 语言切换按钮 | 中/英切换 | |
| 主题切换按钮 | Dark/Light | |
| 设置按钮 | 跳转 Settings | |
| 分享按钮 | 分享版本 | Tauri only |
| 新手引导按钮 | 启动 Tour | |
| 汉堡菜单 | 移动端显示 | |

### 2.4 ClusterTreeNavigator - 树形导航

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| 集群树结构 | 集群 > Topics 文件夹 > Topic > Partition | | 可折叠 |
| 集群健康状态 | 绿/黄/红圆点 | `connection.health_check` | |
| Topic 搜索 | 搜索 Topic 名称 | `topic.search` | |
| 集群右键菜单 | ClusterContextMenu | | |
| Topics 文件夹右键菜单 | TopicsFolderContextMenu | | |
| Topic 右键菜单 | TopicContextMenu | | |
| Partition 右键菜单 | PartitionContextMenu | | |
| 展开/折叠控制 | 单独或批量展开折叠 | | |
| 刷新按钮 | 刷新集群/Topic 元数据 | | |
| 收藏图标 | FavoriteButton | | |
| 高亮选中 | 高亮当前选中的 Topic | | |

### 2.5 TopicNavigator - 平铺导航

| 功能点 | 描述 | API 方法 | 备注 |
|--------|------|----------|------|
| 集群选择器 | Select 下拉选择集群 | | |
| 功能按钮组 | Clusters/Favorites/Schema/History 按钮 | | |
| 搜索框 | 搜索 Topic | `topic.search` | |
| Topic 列表 | 简单列表 | | |
| Consumer Group 列表 | 显示消费者组 | | |
| 刷新按钮 | 刷新 Topic 元数据 | | |

---

## 3. 通用组件 (Shared Components)

### 3.1 Modal 弹窗系统

| 组件 | 用途 | 特性 |
|------|------|------|
| ModernModal | 统一 Modal 样式 | DaisyUI modal-bottom sm:modal-middle |
| CreateTopicDialog | 创建 Topic | 名称、分区数、副本数、配置项 |
| DeleteTopicDialog | 删除 Topic 确认 | 确认弹窗 |
| SendMessageModal | 发送消息 | Partition、Key、Value、Headers |
| TourOverlay | 新手引导遮罩 | 高亮目标元素、步骤导航 |

### 3.2 上下文菜单 (Context Menus)

| 菜单 | 操作项 |
|------|--------|
| ClusterContextMenu | 查看Topics、刷新Topics、刷新连接、创建Topic、测试连接、断开、重连、编辑、删除 |
| TopicsFolderContextMenu | 刷新Topics、创建Topic、查看全部Topics |
| TopicContextMenu | 查看消息、查看详情、查看分区、查看消费者Lag、发送消息、删除 |
| PartitionContextMenu | 查看消息、发送消息 |

### 3.3 其他组件

| 组件 | 功能 |
|------|------|
| FavoriteButton | 收藏/取消收藏按钮 |
| JsonEditor | JSON 编辑器(未使用?) |
| NavigatorSearch | 导航搜索 |
| NavigatorStatusBar | 导航状态栏 |
| SentMessageHistory | 发送历史面板 |
| TopicHistory | Topic 浏览历史 |
| TopicListView | Topic 列表视图 |
| TopicFavorites | Topic 收藏视图 |
| LanguageSelector | 语言选择组件 |
| JsonHighlightSelector | JSON 高亮模板选择组件 |

---

## 4. 状态管理 (Pinia Stores)

| Store | 状态 | 用途 |
|-------|------|------|
| clusterStore | clusters, groups, loading, error, clusterHealth | 集群数据、分组、健康状态 |
| clusterConnectionStore | connections | 连接状态映射 |
| themeStore | isDark | 主题状态 |
| languageStore | currentLanguage, t | 语言、翻译函数 |
| updateStore | downloading, downloadProgress, etc. | 更新下载状态 |
| importExportStore | | 导入导出状态 |

---

## 5. API 客户端 (ApiClient)

### 5.1 请求特性

| 特性 | 描述 |
|------|------|
| 统一请求方法 | POST `/api` + `X-API-Method` header |
| 请求超时 | 默认 30s，消息查询 60s |
| 重试机制 | Tauri 环境重试 3 次 |
| 后端就绪检查 | Tauri 环境等待后端启动(最多 60 次) |
| 请求取消 | AbortController |
| SSE 流式请求 | `/api/stream` POST + SSE 解析 |

### 5.2 所有 API 方法清单

```typescript
// Health
health()

// Clusters
getClusters()
getCluster(id)
createCluster(cluster)
updateCluster(id, cluster)
deleteCluster(id)
testCluster(id)
testClusterConfig(config)

// Cluster Groups
getClusterGroups()
getClusterGroup(id)
createClusterGroup(group)
updateClusterGroup(id, group)
deleteClusterGroup(id)
getClustersInGroup(groupId)
assignClusterToGroup(clusterId, groupId)

// Topics
getTopics(clusterId)
getTopicsWithCluster(clusterId, offset, limit)
getTopicsWithClusters(clusterIds, offset, limit, search)
getTopicDetail(clusterId, topicName)
createTopic(clusterId, topic)
batchCreateTopics(clusterId, topics)
deleteTopic(clusterId, topicName)
batchDeleteTopics(clusterId, topics)
getTopicOffsets(clusterId, topicName)
addPartitions(clusterId, topicName, newPartitions)
refreshTopics(clusterId, topicName)
getRefreshStatus()
cleanupOrphanTopics()
getSavedTopics(clusterId)
searchTopics(keyword)
getTopicCount(clusterId)
getTopicConfig(clusterId, topicName)
alterTopicConfig(clusterId, topicName, config)
updateTopicConfig(clusterId, topicName, config)
getPartitionWatermarks(clusterId, topicName, partition)

// Topic Tags
getTopicTags(clusterId, topicName)
createTopicTag(clusterId, topicName, tagName, color)
deleteTopicTag(clusterId, topicName, tagId)

// Consumer Groups
getConsumerGroupsList(clusterIds, offset, limit, search)
getConsumerGroups(clusterId) // deprecated
getSavedConsumerGroups(clusterId)
refreshConsumerGroups(clusterId, groupName)
getConsumerGroupsByTopic(clusterId, topic)
getConsumerGroupInfo(clusterId, groupName)
getConsumerGroupOffsets(clusterId, groupName)
refreshConsumerGroupOffsets(clusterId, groupName)
resetConsumerGroupOffset(...)
resetConsumerGroupOffsetToEarliest(...)
resetConsumerGroupOffsetToLatest(...)
resetConsumerGroupOffsetToTimestamp(...)
deleteConsumerGroup(clusterId, groupName)
searchConsumerGroups(keyword)

// Messages
getMessages(clusterId, topic, params)
getMessagesStream(clusterId, topic, params, callbacks) // SSE
cancelGetMessages()
sendMessage(clusterId, topic, message)
exportMessages(clusterId, topic, params)

// Connections
getConnections()
disconnectCluster(clusterId)
reconnectCluster(clusterId)
healthCheckCluster(clusterId)

// Settings
getSettings(keys)
updateSetting(key, value)
exportData()
importData(data, strategy)

// Favorites
getFavoriteGroups()
createFavoriteGroup(params)
updateFavoriteGroup(id, params)
deleteFavoriteGroup(id)
getFavorites()
createFavorite(params)
updateFavorite(id, params)
deleteFavorite(id)
checkFavorite(clusterId, topicName)
deleteFavoriteByTopic(clusterId, topicName)

// JSON Highlight
getJsonHighlightTemplates()
getCurrentJsonHighlightTemplate()
setCurrentJsonHighlightTemplate(name)
createJsonHighlightTemplate(params)
updateJsonHighlightTemplate(id, params)
deleteJsonHighlightTemplate(id)

// Topic History
getTopicHistory(limit, offset)
recordTopicHistory(clusterId, topicName)
deleteTopicHistory(id)
deleteTopicHistoryByTopic(clusterId, topicName)
clearTopicHistory()

// Sent Message History
getSentMessageHistory(limit, offset, clusterId, topicName)
recordSentMessageHistory(...)
deleteSentMessageHistory(id)
clearSentMessageHistory()

// Schema Registry
getSchemaRegistryConfig(clusterId)
saveSchemaRegistryConfig(clusterId, registryUrl, username, password)
deleteSchemaRegistryConfig(clusterId)
testSchemaRegistryConnection(registryUrl, username, password)
getSchemaSubjects(clusterId)
getSchemaVersions(clusterId, subject)
getSchema(clusterId, subject, version)
getLatestSchema(clusterId, subject)
registerSchema(clusterId, subject, schemaJson, schemaType)
testSchemaCompatibility(clusterId, subject, schemaJson, version)
getCompatibilityLevel(clusterId, subject)
setCompatibilityLevel(clusterId, subject, level)
getSchemasList(clusterId)
deleteSchema(clusterId, subject)
```

---

## 6. 国际化 (i18n)

| 语言 | 文件 |
|------|------|
| 中文 | translations.ts (zh 部分) |
| 英文 | translations.ts (en 部分) |

支持动态切换，实时更新 UI 文本。

---

## 7. 主题系统

| 特性 | 描述 |
|------|------|
| DaisyUI 主题 | 基于 Tailwind + DaisyUI |
| Dark/Light 模式 | Tailwind dark: 前缀 |
| Glass morphism | 半透明玻璃效果 |
| Gradient border | 渐变边框 |
| Glow effects | 发光效果 |

---

## 8. Tour 新手引导

| 特性 | 描述 |
|------|------|
| 步骤定义 | tour/definitions.ts |
| 多页面支持 | 不同路由不同步骤 |
| 高亮遮罩 | TourOverlay 组件 |
| 目标元素定位 | data-tour 属性 |
| 上一步/下一步/关闭 | 导航按钮 |

---

## 9. 移动端适配

| 特性 | 描述 |
|------|------|
| 响应式断点 | 768px |
| 汉堡菜单 | 顶部导航按钮 |
| 侧边栏抽屉 | 固定定位抽屉 |
| 搜索抽屉 | MobileSearchDrawer |
| 消息卡片视图 | 移动端用 Card 代替 Table |
| 虚拟滚动 | 移动端也支持 |

---

## 10. Tauri 特有功能

| 功能 | 描述 |
|------|------|
| 系统托盘 | 关闭按钮行为(隐藏/退出) |
| 开机自启动 | Windows 注册表 |
| 原生文件对话框 | save/open |
| 原生文件写入 | writeFile |
| 检查更新 | GitHub Release API |
| 后台下载更新 | 断点续传 |
| 分享版本 | 复制安装包 |
| 日志查看 | Tauri 命令获取日志 |
| invoke 调用 | Tauri core.invoke |

---

## 11. 路由结构

| 路径 | 页面 | Query 参数 |
|------|------|-----------|
| `/clusters` | ClustersView | action=create/edit, cluster=name |
| `/topics` | TopicsView | cluster=name, search=keyword |
| `/consumer-groups` | ConsumerGroupsView | cluster=name, group=name |
| `/topic-consumer-groups` | TopicConsumerGroupsView | cluster=name, topic=name |
| `/messages` | MessagesView | cluster=name, topic=name, partition=num |
| `/settings` | SettingsView | |
| `/favorites` | FavoritesView | |
| `/schema-registry` | SchemaRegistryView | cluster=name |

---

## 12. 后端 API 方法完整列表 (Rust)

参见 `/src/routes/` 目录下的路由定义，涵盖：
- cluster.*
- cluster_group.*
- topic.*
- topic_history.*
- consumer_group.*
- message.*
- sent_message.*
- connection.*
- settings.*
- favorite.*
- favorite.group.*
- tag.*
- json_highlight.*
- schema_registry.*
- app.version/logs/logs.clear
- check_for_updates
- install_update
- get_download_status
- clear_download_status
- share_current_version
- open_url
- is_windows
- get_auto_launch
- set_auto_launch

---

## 附录：迁移优先级建议

### P0 - 必须实现
- 集群管理 CRUD + 连接测试
- Topic 管理 CRUD + 刷新
- 消息查询 (SSE 流式 + 虚拟滚动)
- 消息发送
- Consumer Group 查看 + Offset 重置
- 基础布局 (侧边栏 + 顶部导航)

### P1 - 重要功能
- Schema Registry 管理
- 收藏功能
- 设置页面 (语言/主题/侧边栏模式)
- 导入导出
- Tour 新手引导

### P2 - 增强功能
- 系统托盘
- 开机自启动
- 更新检查 + 后台下载
- 日志查看
- JSON 高亮模板
- 移动端适配

### P3 - 可暂缓
- Topic Tags
- 主题详情页面增强
- 高级时间筛选