# Kafka Manager 架构设计

英文版见 [architecture.md](./architecture.md)。

## 系统概览

Kafka Manager 是一个用于管理 Kafka 集群的 **Tauri 2 桌面应用**，由三层组成：

- **Vue 3 前端**（`ui/`）——运行在 Tauri webview 中的用户界面
- **Tauri 壳**（`src-tauri/`，crate `kafka-manager`）——桌面集成：窗口、自动更新、系统托盘、开机自启，以及 IPC 命令入口
- **核心库**（`src/`，crate `kafka-manager-api`）——全部业务逻辑：Kafka 客户端、SQLite 持久化、统一 API 分发器

应用中**没有 HTTP 服务器**，前端完全通过 **Tauri IPC**（`invoke` / `Channel`）与 Rust 通信。

## 架构图

```
┌────────────────────────────────────────────────────────────────────┐
│                     Kafka Manager 桌面应用                          │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Vue 3 前端 (ui/)                            │  │
│  │  ┌───────────┐  ┌────────────┐  ┌─────────────────────────┐  │  │
│  │  │   Views   │  │ Components │  │  apiClient (client.ts)  │  │  │
│  │  │           │  │            │  │  invoke + Channel       │  │  │
│  │  └───────────┘  └────────────┘  └─────────────────────────┘  │  │
│  └──────────────────────────────┬───────────────────────────────┘  │
│                                 │ Tauri IPC                        │
│  ┌──────────────────────────────▼───────────────────────────────┐  │
│  │              Tauri 壳 (src-tauri/, crate kafka-manager)       │  │
│  │  api_commands.rs:  api_request                                │  │
│  │                    message_list_stream (Channel)              │  │
│  │                    cancel_message_list                        │  │
│  │  lib.rs:           更新、托盘、开机自启、日志、窗口            │  │
│  └──────────────────────────────┬───────────────────────────────┘  │
│                                 │ Rust 直接调用                     │
│  ┌──────────────────────────────▼───────────────────────────────┐  │
│  │           核心库 (src/, crate kafka-manager-api)              │  │
│  │  ┌────────────┐  ┌─────────────┐  ┌────────────────────────┐ │  │
│  │  │  分发器    │  │ Kafka 层    │  │  DB 层 (sqlx/SQLite)   │ │  │
│  │  │  api.rs    │  │  src/kafka/ │  │  src/db/               │ │  │
│  │  └────────────┘  └─────────────┘  └────────────────────────┘ │  │
│  └──────────────────────────────┬───────────────────────────────┘  │
└─────────────────────────────────┼───────────────────────────────────┘
                                  ▼
                        Kafka 集群 (rdkafka)
```

## 进程与线程模型

整个应用是单进程，包含两个异步执行上下文：

1. **Tauri 主运行时**——运行 webview 和所有 `#[tauri::command]` 处理器。命令只是薄封装：克隆共享的 `AppState` 并调用核心库。
2. **后端线程 + 独立 tokio runtime**——启动时创建（`src-tauri/src/lib.rs` 的 `run()`）。它初始化 SQLite 连接池和 `AppState`，通过 mpsc channel 把状态交回主线程，然后永久驻留（`std::future::pending`）。DB 连接池的后台任务、Kafka 客户端线程（rdkafka）、连接池和遥测循环都运行在这个 runtime 上。

启动流程：

```
run()
 ├─ 启动后端线程 ──► tokio runtime
 │                    ├─ 加载 config.toml（可选）
 │                    ├─ 打开并初始化 SQLite（系统数据目录）
 │                    ├─ 构建 AppState ──► 发送给主线程
 │                    ├─ 后台任务：创建 Kafka 客户端
 │                    └─ 后台任务：遥测循环（每小时）
 ├─ 接收 AppState（≤ 30 秒）
 ├─ app.manage(BackendState(state))、app.manage(StreamRegistry)
 └─ 进入 Tauri 事件循环
```

如果数据库打开失败，UI 仍会启动；业务命令返回 `"Backend not initialized"` 而不是崩溃。

### 共享状态（`AppState`，`src/lib.rs`）

| 字段 | 类型 | 用途 |
|------|------|------|
| `db` | `DbPool`（sqlx SQLite，WAL） | 全部持久化 |
| `clients` | `Arc<ArcSwap<KafkaClients>>` | 无锁可换的按集群 Kafka 客户端注册表 |
| `pools` | `ClusterPools` | 基于 deadpool 的按集群消费者/生产者连接池 |
| `config` | `Config` | 从可选的 `config.toml` 加载 |
| `refresh_state` | `Arc<Mutex<RefreshState>>` | 防止 Topic/消费组并发重复刷新 |
| `import_export_lock` | `Arc<Mutex<ImportExportLock>>` | 同一时间只允许一个导入/导出 |

## IPC 设计

### 统一分发器

与原先的 HTTP API 一样，业务操作是 RPC 风格，全部经过**一个**命令：

```rust
// src-tauri/src/api_commands.rs
#[tauri::command]
async fn api_request(state: State<'_, BackendState>, method: String, params: Value)
    -> Result<Value, String>
{
    api::dispatch_request(&method, state.get()?, params)
        .await
        .map_err(|e| e.to_message())
}
```

`dispatch_request`（`src/api.rs`）把约 113 个方法名（`cluster.list`、`topic.create`、`message.send`……）匹配到形如 `(AppState, Value) -> Result<Value>` 的处理器。处理器用 `get_string_param` / `get_optional_*` 辅助函数提取类型化参数，因此新增一个方法只需一行 `match` 分支加一个处理器。

- **成功**：Promise 直接 resolve 为结果 `Value`（无包装信封）。
- **失败**：Promise reject 为错误消息字符串；`AppError::to_message()` 保证错误文案与旧 HTTP API 一致。

### 流式传输（消息查询）

Tauri command 无法返回流，因此 `message.list` 有专门的流式命令，使用 `tauri::ipc::Channel`：

```
前端                              src-tauri                      核心库
 │                                │                               │
 │── message_list_stream ────────►│                               │
 │   (request_id, params, Channel)│── start_message_list_stream ─►│
 │                                │   (返回 mpsc::Receiver)       │── 启动生产者
 │                                │                               │   (按分区消费,
 │◄── Channel: start/batch/… ─────│◄── StreamEvent ───────────────│   堆合并)
 │                                │   转发循环                     │
 │── cancel_message_list ────────►│── CancellationToken.cancel() ─►│
```

- 事件：`start` → `batch`* → `order`? → `complete` | `error`；每个事件是 `StreamEvent { event, data }`，其中 `data` 为 JSON 字符串。
- 命令 future 在事件流结束前不会返回，因此前端 `invoke()` 的 Promise resolve 即代表流结束。
- **取消**：`cancel_message_list(request_id)` 取消存储在 `StreamRegistry` 中的令牌；生产者在批次间检查该令牌。如果前端 channel 关闭（窗口关闭等），转发循环也会取消令牌。
- **超时**：服务端 120 秒超时保护会自动取消令牌。
- 前端 `apiClient.getMessagesStream()` 将以上封装为回调形式，并返回带 `abort()` 的 `StreamHandle`。

### 桌面 Shell 命令

与业务命令一起注册在 `src-tauri/src/lib.rs`：`get_app_version`、`check_for_updates`、`install_update`、下载状态命令、`get_app_logs` / `clear_app_logs`、`set_auto_launch` / `get_auto_launch`、`set_system_tray`、`is_windows`、`share_current_version`、`open_url`。更新器支持断点续传和绿色版 zip 自更新，并通过 `update-available` 事件通知前端。

## 核心库结构（`src/`）

```
src/
├── lib.rs                    # AppState、crate 根
├── api.rs                    # 统一分发器 + 全部业务处理器（约 113 个方法）
│                             # + 流式消息生产者（堆合并、可取消）
├── api_import_export.rs      # 设置/数据导入导出（后台导入）
├── api_schema_registry.rs    # Schema Registry 处理器
├── config.rs                 # config.toml 加载与默认值
├── error.rs                  # AppError（thiserror）+ to_message()
├── db/                       # SQLite 层（sqlx）
│   ├── mod.rs                # DbPool、建表（内联 CREATE TABLE）
│   ├── cluster.rs            # ClusterStore
│   ├── cluster_group.rs      # ClusterGroupStore
│   ├── topic.rs              # TopicStore（元数据缓存）
│   ├── consumer_group.rs     # 消费组元数据/偏移量
│   ├── favorite.rs           # 收藏分组与条目
│   ├── topic_history.rs      # Topic 浏览历史
│   ├── sent_message.rs       # 发送消息历史
│   ├── json_highlight.rs     # JSON 高亮模板
│   ├── topic_template.rs     # Topic 创建模板
│   ├── settings.rs           # 键值用户设置
│   └── schema_registry.rs    # Registry 配置与缓存的 Schema
├── kafka/                    # rdkafka 封装
│   ├── mod.rs                # KafkaClients 注册表、客户端配置、连通性测试
│   ├── admin.rs              # Topic CRUD、配置、分区
│   ├── consumer.rs           # 消息拉取（游标式 + 流式变体）
│   ├── consumer_group.rs     # 消费组列表、偏移量、重置
│   ├── producer.rs           # 消息发送
│   ├── offset.rs             # 偏移量/水位线管理
│   ├── throughput.rs         # 吞吐量估算
│   ├── avro.rs / protobuf.rs # 消息体编解码
│   ├── schema_registry_client.rs # 基于 reqwest 的 Registry REST 客户端
│   ├── transaction.rs        # 事务支持
│   └── import_export.rs      # 集群数据导入导出辅助
├── pool/                     # 基于 deadpool 的消费者/生产者连接池
├── models/                   # 共享数据模型
├── telemetry/                # 可选的 MySQL 使用上报（每小时）
└── utils.rs                  # 日志路径辅助
```

## 数据库

SQLite（sqlx，WAL 模式，20 个连接），位于系统数据目录（`Kafka Manager/kafka_manager.db`）。表结构在启动时通过内联 `CREATE TABLE IF NOT EXISTS` 幂等创建——没有迁移框架。

数据表：`kafka_clusters`、`cluster_groups`、`topic_metadata`、`consumer_group_metadata`、`consumer_group_topics`、`consumer_group_offsets`、`topic_templates`、`user_settings`、`json_highlight_templates`、`favorite_groups`、`favorite_items`、`topic_history`、`sent_messages`、`schema_registry_configs`、`schemas`、`telemetry_records`、`resource_tags`（未使用）。

Topic 和消费组列表**缓存在 SQLite** 中（`topic_metadata`、`consumer_group_*`），按需刷新（`topic.refresh`、`consumer_group.refresh`），因此即使集群不可达，导航也能秒开。

## 前端结构（`ui/`）

Vue 3 + TypeScript + Vite + Tailwind 4/DaisyUI 5 + Pinia + vue-router + vue-virtual-scroller。

```
ui/src/
├── api/client.ts           # apiClient：invoke/Channel 的类型化封装（唯一 IPC 使用方）
├── views/                  # 8 个页面（集群、Topic、消息、消费组……）
├── components/             # MessageQueryTool、TopicNavigator、ClusterTreeNavigator……
├── layouts/                # ModernLayout（主布局）
├── stores/                 # cluster、clusterConnection、theme、language、update
├── i18n/translations.ts    # 中 / 英
└── tour/                   # 新手引导浮层
```

关键模式：

- **无 HTTP**：`apiClient` 是唯一调用 `invoke` 的模块，组件从不直接调用。
- **虚拟滚动**：所有大列表（Topic、消息、消费组）。
- **流式渲染**：消息批次先进入非响应式的 `pendingMessages` 缓冲区，再按定时器批量刷入响应式数组。
- **i18n 与主题**：通过 `settings.*` 方法持久化，`localStorage` 兜底。

## 消息查询设计

- 每个分区独立的 `max_messages` 上限，每个分区使用独立的消费者 `group.id`
- 串行/并行模式（按集群延迟自动选择），信号量限制最多 10 个分区并行
- 基于时间戳的偏移量定位；钳位前做水位线边界检查
- 跨分区最小堆合并，保证流式输出有序
- `fetchMode: oldest|newest`，键/值搜索（`search_in: key|value|all`）
- 通过 `CancellationToken` 协作式取消，批次间检查

## 性能优化

### 后端

| 优化 | 说明 |
|------|------|
| 连接池 | 按集群的 deadpool 消费者/生产者池 |
| SQLite 缓存 | Topic/消费组元数据缓存，按需刷新 |
| 无锁客户端注册表 | `ArcSwap`，读取不加锁 |
| 分区并行拉取 | 信号量限制的并行分区消费 |
| 懒连接 | Kafka 客户端在启动时后台建立 |

### 前端

| 优化 | 说明 |
|------|------|
| 虚拟滚动 | 支撑数万行列表 |
| 非响应式缓冲 | 消息批次在刷入前绕过 Vue 响应式系统 |
| 批量 UI 刷新 | 按定时器刷新消息列表，而非每个事件刷新 |
| Channel 流式 | 无 HTTP/SSE 的序列化开销 |

## 部署

```bash
# 开发模式（前端 dev server + Tauri 热重载）
./start-tauri-dev.sh        # 或：cd ui && npm run tauri dev

# 生产构建
cd ui && npm run tauri build
```

生产二进制完全自包含：核心库静态链接进 Tauri 应用，数据库位于系统数据目录。可在可执行文件旁放置可选的 `config.toml` 预置集群与连接池参数。

## 遥测与更新

- **遥测**：可选的每小时 MySQL 使用上报（`src/telemetry/`），不可达时静默禁用；用户反馈走 `telemetry.submit_feedback`。
- **更新**：Tauri 壳在启动时和每小时检查 GitHub releases，后台断点续传下载，通过 `update-available` 事件通知前端，确认后自更新。
