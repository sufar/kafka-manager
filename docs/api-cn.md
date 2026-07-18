# Kafka Manager IPC API 参考

## 概述

Kafka Manager 是一个 **Tauri 2 桌面应用**。前端（Vue 3）不再通过 HTTP 访问后端——所有业务操作都通过 **Tauri IPC 命令** 调用 Rust 核心库（`kafka-manager-api`）中的统一分发器。

英文版见 [api.md](./api.md)。

业务操作共有三个 IPC 命令：

| 命令 | 用途 |
|------|------|
| `api_request` | 所有请求/响应式操作的统一入口 |
| `message_list_stream` | 流式消息查询（通过 Tauri `Channel` 推送事件） |
| `cancel_message_list` | 取消进行中的流式查询 |

此外还有一组**桌面 shell 命令**（自动更新、开机启动、系统托盘、日志等），见[桌面 Shell 命令](#桌面-shell-命令)。

## 请求 / 响应格式

### `api_request`

```ts
import { invoke } from '@tauri-apps/api/core';

const data = await invoke('api_request', {
  method: 'topic.list',          // 统一方法名
  params: { cluster_id: 'prod' } // 操作参数（JSON）
});
```

- **成功**：Promise 直接 resolve 为操作结果数据（无包装信封）。
- **失败**：Promise reject 为**错误消息字符串**（如 `"Cluster not found"`、`"Kafka error: ..."`）。

前端在 `ui/src/api/client.ts` 中做了封装，为每个操作提供类型化方法，失败时抛出 `{ message, status }` 对象，组件无需直接调用 `invoke`。

```ts
import { apiClient } from '@/api/client';

try {
  const topics = await apiClient.getTopics('prod');
} catch (e) {
  console.error((e as { message: string }).message);
}
```

### `message_list_stream`

流式消息查询通过 `tauri.Channel` 推送事件：

```ts
import { invoke, Channel } from '@tauri-apps/api/core';

interface StreamEvent { event: string; data: string } // data 为 JSON 字符串

const requestId = crypto.randomUUID();
const channel = new Channel<StreamEvent>();
channel.onmessage = (evt) => {
  const payload = JSON.parse(evt.data);
  switch (evt.event) {
    case 'start':    /* { partitions, total_target } */ break;
    case 'batch':    /* { messages, progress, total } */ break;
    case 'order':    /* { sort } */ break;
    case 'complete': /* { actual_total?, target_total? } */ break;
    case 'error':    /* { error } */ break;
  }
};

// 事件流结束时 Promise resolve
await invoke('message_list_stream', { requestId, params: { cluster_id: 'prod', topic: 'events' }, channel });

// 前端可随时取消：
await invoke('cancel_message_list', { requestId });
```

事件顺序：`start` → `batch`* → `order`? → `complete` | `error`。
后端强制 120 秒超时；取消操作会通过取消令牌（cancellation token）停止服务端 Kafka 消费者。

`ui/src/api/client.ts#getMessagesStream` 将以上封装为回调式 API（`onStart/onBatch/onOrder/onComplete/onError`），并返回带 `abort()` 的 `StreamHandle`。

---

## 统一 API 方法

以下方法均通过 `api_request` 以对应 `method` 名调用。参数命名使用 `snake_case`；时间戳为 Unix 毫秒。

### 健康检查

| 方法 | 说明 | 参数 |
|------|------|------|
| `health` | 健康检查 | 无 |

### 集群管理

| 方法 | 说明 | 参数 |
|------|------|------|
| `cluster.list` | 集群列表 | `group_id?: number, search?: string` |
| `cluster.get` | 集群详情 | `id: number` |
| `cluster.create` | 创建集群 | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.update` | 更新集群 | `id: number, name?: string, brokers?: string, ...` |
| `cluster.delete` | 删除集群 | `id: number` |
| `cluster.test` | 测试已保存集群的连接 | `id: number` |
| `cluster.test_config` | 测试连接（不保存） | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.stats` | 集群统计 | `cluster_id: string` |

### 集群分组管理

| 方法 | 说明 | 参数 |
|------|------|------|
| `cluster_group.list` | 分组列表 | 无 |
| `cluster_group.get` | 分组详情 | `id: number` |
| `cluster_group.create` | 创建分组 | `name: string, description?: string, sort_order?: number` |
| `cluster_group.update` | 更新分组 | `id: number, name?: string, description?: string, sort_order?: number` |
| `cluster_group.delete` | 删除分组 | `id: number` |
| `cluster_group.clusters` | 分组内集群列表 | `group_id: number` |
| `cluster_group.assign_cluster` | 将集群分配到分组 | `cluster_id: string, group_id: number` |

### Topic 管理

| 方法 | 说明 | 参数 |
|------|------|------|
| `topic.list` | Topic 列表 | `cluster_id?: string` |
| `topic.list_with_cluster` | 带集群信息的 Topic 列表（分页） | `cluster_id?: string, cluster_ids?: string[], offset?: number, limit?: number, search?: string` |
| `topic.get` | Topic 详情 | `cluster_id: string, name: string` |
| `topic.create` | 创建 Topic | `cluster_id: string, name: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `topic.delete` | 删除 Topic | `cluster_id: string, topic: string` |
| `topic.delete_all` | 删除集群下所有 Topic | `cluster_id: string` |
| `topic.batch_create` | 批量创建 | `cluster_id: string, topics: array, continue_on_error?: boolean` |
| `topic.batch_delete` | 批量删除 | `cluster_id: string, topics: string[], continue_on_error?: boolean` |
| `topic.offsets` | Topic 各分区偏移量 | `cluster_id: string, topic: string` |
| `topic.config_get` | 获取 Topic 配置 | `cluster_id: string, topic: string` |
| `topic.config_alter` | 修改 Topic 配置 | `cluster_id: string, topic: string, config: object` |
| `topic.partitions_add` | 增加分区 | `cluster_id: string, topic: string, new_partitions: number` |
| `topic.partition.watermarks` | 分区水位线 | `cluster_id: string, topic: string, partition: number` |
| `topic.throughput` | Topic 吞吐量 | `cluster_id: string, topic: string` |
| `topic.refresh` | 从集群刷新 Topic 列表 | `cluster_id?: string, topic_name?: string` |
| `topic.saved` | 获取已缓存的 Topic | `cluster_id: string` |
| `topic.search` | 跨集群搜索 Topic | `keyword?: string` |
| `topic.count` | Topic 数量 | `cluster_id: string` |
| `topic.cleanup_orphans` | 清理已删除 Topic 的元数据 | 无 |
| `refresh.status` | 查询正在刷新的集群 | 无 |

### 消费组管理

| 方法 | 说明 | 参数 |
|------|------|------|
| `consumer_group.list` | 消费组列表（分页） | `cluster_ids?: string[], offset?: number, limit?: number, search?: string` |
| `consumer_group.list_by_topic` | 消费某 Topic 的消费组（含 lag） | `cluster_id: string, topic: string` |
| `consumer_group.get` | 消费组详情 | `cluster_id: string, group_name: string` |
| `consumer_group.offsets` | 消费组各分区偏移量 | `cluster_id: string, group_name: string` |
| `consumer_group.refresh` | 从集群刷新消费组列表 | `cluster_id?: string, group_name?: string` |
| `consumer_group.saved` | 获取已缓存的消费组 | `cluster_id: string` |
| `consumer_group.reset_offset` | 重置偏移量 | `cluster_id: string, group_name: string, topic: string, partition: number, reset_to: 'earliest'\|'latest'\|'offset'\|'timestamp', offset?: number, timestamp?: number` |
| `consumer_group.delete` | 删除消费组 | `cluster_id: string, group: string` |

### 消息管理

| 方法 | 说明 | 参数 |
|------|------|------|
| `message.list` | 查询消息（非流式） | `cluster_id: string, topic: string, partition?: number, offset?: number, max_messages?: number, order_by?: 'timestamp'\|'offset', sort?: 'asc'\|'desc', search?: string, search_in?: 'key'\|'value'\|'all', start_time?: number, end_time?: number, fetchMode?: 'oldest'\|'newest'` |
| `message.send` | 发送消息 | `cluster_id: string, topic: string, value: string, key?: string, partition?: number, headers?: object` |
| `message.export` | 导出消息 | `cluster_id: string, topic: string, partition?: number, max_messages?: number, search?: string, fetch_mode?: string, start_time?: number, end_time?: number` |

> 大结果集建议使用 `message_list_stream` 命令的流式变体（参数相同）。

### 集群连接管理

| 方法 | 说明 | 参数 |
|------|------|------|
| `connection.list` | 所有连接状态 | 无 |
| `connection.get` | 单个连接状态 | `cluster_id: string` |
| `connection.disconnect` | 断开集群连接 | `cluster_id: string` |
| `connection.reconnect` | 重连集群 | `cluster_name: string` |
| `connection.health_check` | 集群健康检查 | `cluster_id: string` |
| `connection.metrics` | 连接指标 | `cluster_id: string` |
| `connection.batch_disconnect` | 批量断开 | `cluster_ids: string[]` |
| `connection.batch_reconnect` | 批量重连 | `cluster_names: string[]` |

### 设置与导入导出

| 方法 | 说明 | 参数 |
|------|------|------|
| `settings.get` | 获取设置 | `keys?: string[]` |
| `settings.update` | 更新设置 | `key: string, value: string` |
| `settings.export` | 导出全部数据（集群、分组、Topic、收藏、历史） | 无 |
| `settings.import` | 导入数据（后台执行） | `data: object, strategy: 'skip'\|'overwrite'` |

### 应用信息

| 方法 | 说明 | 参数 |
|------|------|------|
| `app.version` | 后端版本号 | 无 |
| `app.logs` | 最近的后端日志 | 无 |
| `app.logs.clear` | 清空后端日志 | 无 |

### Topic 模板

| 方法 | 说明 | 参数 |
|------|------|------|
| `template.list` | 模板列表 | 无 |
| `template.get` | 模板详情 | `id: number` |
| `template.create` | 创建模板 | `name: string, description?: string, num_partitions: number, replication_factor: number, config?: object` |
| `template.update` | 更新模板 | `id: number, ...` |
| `template.delete` | 删除模板 | `id: number` |
| `template.presets` | 预定义模板列表 | 无 |
| `template.create_topic` | 从模板创建 Topic | `cluster_id: string, topic_name: string, template_id?: number, template_name?: string, override_config?: object` |

### 收藏管理

| 方法 | 说明 | 参数 |
|------|------|------|
| `favorite.group.list` | 收藏分组列表（含条目数） | 无 |
| `favorite.group.create` | 创建收藏分组 | `name: string, description?: string, sort_order?: number` |
| `favorite.group.get` | 收藏分组详情 | `id: number` |
| `favorite.group.update` | 更新收藏分组 | `id: number, ...` |
| `favorite.group.delete` | 删除收藏分组 | `id: number` |
| `favorite.list` | 收藏列表（按分组含条目） | 无 |
| `favorite.create` | 添加收藏 | `group_id: number, cluster_id: string, topic_name: string, description?: string, sort_order?: number` |
| `favorite.get` | 收藏详情 | `id: number` |
| `favorite.update` | 更新收藏 | `id: number, group_id?: number, description?: string, sort_order?: number` |
| `favorite.delete` | 删除收藏 | `id: number` |
| `favorite.check` | 检查 Topic 是否已收藏 | `cluster_id: string, topic_name: string` |
| `favorite.delete_by_topic` | 按 Topic 删除收藏 | `cluster_id: string, topic_name: string` |

### Topic 浏览历史

| 方法 | 说明 | 参数 |
|------|------|------|
| `topic_history.list` | 最近浏览的 Topic | `limit?: number, offset?: number` |
| `topic_history.record` | 记录一次浏览 | `cluster_id: string, topic_name: string` |
| `topic_history.delete` | 删除一条历史 | `id: number` |
| `topic_history.delete_by_topic` | 删除某 Topic 的历史 | `cluster_id: string, topic_name: string` |
| `topic_history.clear` | 清空历史 | 无 |

### 发送消息历史

| 方法 | 说明 | 参数 |
|------|------|------|
| `sent_message.list` | 已发送消息列表 | `limit?: number, offset?: number, cluster_id?: string, topic_name?: string` |
| `sent_message.record` | 记录一条已发送消息 | `cluster_id: string, topic_name: string, partition: number, key?: string, value: string, headers?: object, offset?: number` |
| `sent_message.delete` | 删除一条记录 | `id: number` |
| `sent_message.clear` | 清空记录 | 无 |

### JSON 高亮模板

| 方法 | 说明 | 参数 |
|------|------|------|
| `json_highlight.list` | 高亮样式模板列表 | 无 |
| `json_highlight.get_current` | 当前使用的模板 | 无 |
| `json_highlight.set_current` | 设置当前模板 | `id: number` |
| `json_highlight.create` | 创建模板 | `name: string, description?: string, style_json: object` |
| `json_highlight.update` | 更新模板 | `id: number, ...` |
| `json_highlight.delete` | 删除模板 | `id: number` |

### Schema Registry

| 方法 | 说明 | 参数 |
|------|------|------|
| `schema_registry.config.get` | 获取集群的 Registry 配置 | `cluster_id: string` |
| `schema_registry.config.save` | 保存 Registry 配置 | `cluster_id: string, registry_url: string, username?: string, password?: string` |
| `schema_registry.config.delete` | 删除 Registry 配置 | `cluster_id: string` |
| `schema_registry.config.test` | 测试 Registry 连接 | `registry_url: string, username?: string, password?: string` |
| `schema_registry.subject.list` | Subject 列表 | `cluster_id: string` |
| `schema_registry.version.list` | Subject 的版本列表 | `cluster_id: string, subject: string` |
| `schema_registry.get` | 按 Subject + 版本获取 Schema | `cluster_id: string, subject: string, version: number` |
| `schema_registry.get_latest` | 获取最新 Schema | `cluster_id: string, subject: string` |
| `schema_registry.register` | 注册 Schema | `cluster_id: string, subject: string, schema_json: string, schema_type: string` |
| `schema_registry.compatibility.test` | 兼容性测试 | `cluster_id: string, subject: string, schema_json: string, version?: number` |
| `schema_registry.compatibility.get` | 获取兼容性级别 | `cluster_id: string, subject: string` |
| `schema_registry.compatibility.set` | 设置兼容性级别 | `cluster_id: string, subject: string, compatibility_level: string` |
| `schema_registry.list` | Schema 汇总列表 | `cluster_id: string` |
| `schema_registry.delete` | 删除 Subject | `cluster_id: string, subject: string` |

### 遥测与反馈

| 方法 | 说明 | 参数 |
|------|------|------|
| `telemetry.check_connection` | 检查遥测连通性 | 无 |
| `telemetry.report` | 触发遥测上报 | 无 |
| `telemetry.submit_feedback` | 提交用户反馈 | `feedback_content: string` |

---

## 桌面 Shell 命令

以下命令由 Tauri 壳（`src-tauri`）直接暴露，不经过统一分发器：

| 命令 | 说明 |
|------|------|
| `get_app_version` | 应用版本号（取自 Cargo.toml） |
| `check_for_updates` | 检查新版本 |
| `install_update` | 下载并安装更新 |
| `get_download_status` / `clear_download_status` / `set_auto_download_flag` | 更新下载状态管理 |
| `get_app_logs` / `clear_app_logs` | 应用日志读取/清空 |
| `set_auto_launch` / `get_auto_launch` | 开机自启（Windows 注册表） |
| `set_system_tray` | 启用/禁用系统托盘图标 |
| `is_windows` | 平台判断 |
| `share_current_version` | 分享版本信息 |
| `open_url` | 用系统浏览器打开链接 |

---

## 前端调用示例

```ts
import { apiClient } from '@/api/client';

// 创建集群
const cluster = await apiClient.createCluster({
  name: 'prod',
  brokers: 'broker1:9092,broker2:9092',
});

// Topic 列表
const topics = await apiClient.getTopics('prod');

// 发送消息
await apiClient.sendMessage('prod', 'events', {
  key: 'user-123',
  value: JSON.stringify({ event: 'user_login', userId: 123 }),
});

// 流式消息查询（可取消）
const handle = apiClient.getMessagesStream('prod', 'events',
  { max_messages: 1000, fetchMode: 'newest' },
  {
    onBatch: (messages, progress, total) => render(messages),
    onComplete: () => console.log('done'),
    onError: (err) => console.error(err),
  });
// 之后可随时：handle.abort();

// 重置消费组偏移量到最新
await apiClient.resetConsumerGroupOffset('prod', 'my-group', 'events', 0, 'latest');
```

## 错误处理

- `api_request` 失败时 reject 为纯错误消息字符串；`apiClient` 会重新包装为 `{ message, status }` 抛出。
- 未知方法名返回 `Unknown method: <name>`。
- 流式错误通过 `error` 事件（`{ error: string }`）传递，而不是 Promise reject。

## 注意事项

1. **无 HTTP 层**：没有 REST 服务、没有端口、没有 CORS——前端只通过 Tauri IPC 与 Rust 通信。
2. **参数命名**：`snake_case`；时间戳为 Unix 毫秒。
3. **流式超时**：流式消息查询在服务端 120 秒后自动取消。
4. **后台导入**：`settings.import` 立即返回并在后台执行；并发的导入/导出操作会被拒绝。
5. **权威来源**：方法分发表见 `src/api.rs`（`dispatch_request`）；流式事件见 `src/api.rs`（`start_message_list_stream`）；Shell 命令见 `src-tauri/src/lib.rs` 与 `src-tauri/src/api_commands.rs`。
