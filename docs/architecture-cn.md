# Kafka Manager 架构设计

英文版见 [architecture.md](./architecture.md)。

## 系统概览

Kafka Manager 是一个纯 Rust 的 **GPUI 桌面应用**，用于管理 Kafka 集群，由两个 crate 组成：

- **`app/`**（crate `kafka-manager-app`）——桌面应用：基于 [GPUI](https://gpui.rs/) 0.2 + [gpui-component](https://github.com/longbridge/gpui-component) 0.5 的界面，以及桌面集成（托盘、开机自启、自动更新、单实例、原生对话框）
- **`src/`**（crate `kafka-manager-api`）——业务核心库：Kafka 客户端（rdkafka）、SQLite 持久化（sqlx）、统一方法分发器（约 113 个方法）

应用中**没有 webview、没有 JavaScript、没有 HTTP 服务器、没有 IPC 边界**——界面直接在进程内调用核心库。

## 架构图

```
┌────────────────────────────────────────────────────────────────────┐
│                  Kafka Manager（单进程）                            │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │              应用 Crate (app/, kafka-manager-app)             │  │
│  │                                                              │  │
│  │  页面 (pages/)           工作区           桌面集成            │  │
│  │  ├─ clusters.rs          侧边栏导航       ├─ tray.rs         │  │
│  │  ├─ topics.rs            页面切换         ├─ updater.rs      │  │
│  │  ├─ messages.rs ──┐      i18n (中英)      └─ main.rs         │  │
│  │  ├─ consumer_     │      主题 (明暗)          (单实例,        │  │
│  │  │  groups.rs     │                            开机自启)     │  │
│  │  ├─ schema_       │                                          │  │
│  │  │  registry.rs   │      service.rs ────────┐                │  │
│  │  ├─ favorites.rs  │      (运行时桥接)        │                │  │
│  │  └─ settings.rs   │                         │                │  │
│  └────────────────────┼─────────────────────────┼────────────────┘  │
│                       │ 直接调用                 │ tokio JoinHandle  │
│  ┌────────────────────▼─────────────────────────▼────────────────┐  │
│  │           核心库 (src/, kafka-manager-api)                    │  │
│  │  api.rs              kafka/              db/                  │  │
│  │  统一分发器          rdkafka 封装        sqlx / SQLite        │  │
│  │  (约 113 个方法)     admin/consumer/     集群、Topic、        │  │
│  │  + 流式查询          producer、offsets   收藏、设置           │  │
│  └──────────────────────────────┬────────────────────────────────┘  │
└─────────────────────────────────┼───────────────────────────────────┘
                                  ▼
                        Kafka 集群 (rdkafka)
```

## 进程与线程模型

整个应用是单进程，包含两个异步执行上下文：

1. **GPUI 主线程**——运行窗口、所有页面实体和 `cx.spawn` 界面任务。不直接触碰 Kafka 和 SQLite。
2. **后端线程 + 独立 tokio runtime**——启动时创建（`app/src/backend.rs`）。初始化 SQLite 连接池和 `AppState` 后永久驻留（`std::future::pending`）。DB 连接池的后台任务、rdkafka 客户端线程、连接池和遥测循环都运行在这个 runtime 上。

启动流程（`app/src/main.rs`）：

```
main()
 ├─ 单实例检查
 ├─ 启动后端线程 ──► tokio runtime
 │                    ├─ 加载 config.toml（可选）
 │                    ├─ 打开并初始化 SQLite（系统数据目录）
 │                    ├─ 构建 AppState ──► 发送给主线程
 │                    ├─ 后台任务：创建 Kafka 客户端
 │                    └─ 后台任务：遥测循环（每小时）
 ├─ 接收 AppState（≤ 30 秒）+ tokio runtime 句柄
 ├─ 从 SQLite 读取托盘设置
 └─ gpui Application::run
      ├─ 全局：Backend(AppState)、TokioRuntime(Handle)、I18n、Theme
      ├─ 托盘图标 + 菜单事件轮询任务（如启用）
      ├─ 自动检查更新（延迟 3 秒，有新版本时通知）
      └─ 打开窗口 → Workspace（侧边栏 + 页面）
```

如果数据库打开失败，UI 仍会启动；业务调用返回 `"Backend not initialized"` 而不是崩溃。

### 运行时桥接（`app/src/service.rs`）

`dispatch_request` 是 async 的，内部使用 tokio 原语（sqlx、rdkafka），因此必须**在后端 tokio runtime 上**执行，而不是 GPUI 的 executor。桥接模式：

```rust
// 在 tokio runtime 上执行分发；JoinHandle 是一个普通 Future，
// GPUI 任务可以在自己的 executor 上直接 await
let handle = tokio_handle.spawn(async move {
    api::dispatch_request(method, state, params).await.map_err(|e| e.to_message())
});
let result = handle.await?;  // 在 cx.spawn 的 UI 任务中 await
```

### 共享状态（`AppState`，`src/lib.rs`）

| 字段 | 类型 | 用途 |
|------|------|------|
| `db` | `DbPool`（sqlx SQLite，WAL） | 全部持久化 |
| `clients` | `Arc<ArcSwap<KafkaClients>>` | 无锁可换的按集群 Kafka 客户端注册表 |
| `pools` | `ClusterPools` | 基于 deadpool 的按集群消费者/生产者连接池 |
| `config` | `Config` | 从可选的 `config.toml` 加载 |
| `refresh_state` | `Arc<Mutex<RefreshState>>` | 防止 Topic/消费组并发重复刷新 |
| `import_export_lock` | `Arc<Mutex<ImportExportLock>>` | 同一时间只允许一个导入/导出 |

## 界面架构（`app/`）

### 目录结构

```
app/
├── Cargo.toml            # gpui 0.2.2、gpui-component 0.5.1、kafka-manager-api……
├── assets/icon.png       # 托盘图标
├── locales/              # zh.json / en.json（856 个键，从旧 Vue i18n 转换）
└── src/
    ├── main.rs           # 入口：日志、单实例、后端引导、托盘、自动检查更新
    ├── backend.rs        # AppState 引导（DB、Kafka 客户端、遥测）
    ├── service.rs        # 到 dispatch_request 的运行时桥接
    ├── state.rs          # 全局：Backend / TokioRuntime / Page 枚举
    ├── i18n.rs           # t(cx, "nav.clusters") 点键查询
    ├── workspace.rs      # 侧边栏导航 + 页面切换
    ├── tray.rs           # 系统托盘（tray-icon crate），显示/退出菜单
    ├── updater.rs        # GitHub releases 检查、下载、绿色版安装（从旧 Tauri 壳移植）
    ├── components/       # 共享组件（StringOption 下拉项……）
    └── pages/            # 每页一个实体
        ├── clusters.rs       # 集群 CRUD + 分组 + 连接测试
        ├── topics.rs         # Topic 列表/创建/删除/详情 + 收藏切换
        ├── messages.rs       # 流式查询 + 虚拟表格 + 详情面板 + 发送 + 导出 + 历史
        ├── consumer_groups.rs# 消费组列表 + 偏移量 + 重置 + 删除
        ├── schema_registry.rs# Registry 配置 + Subject 列表 + Schema 详情
        ├── favorites.rs      # 收藏分组与条目管理
        └── settings.rs       # 主题 / 语言 / 托盘 / 自启 / 日志 / 导入导出 / 更新
```

### 页面模式

每个页面是一个 GPUI 实体（`Render` 实现），持有自己的视图状态。数据单向流动：

```
用户操作 → 实体方法 → service::call（tokio runtime）
        → cx.spawn 任务 → 更新实体字段 → cx.notify() → 重新渲染
```

贯穿应用的关键 GPUI 模式：

- **异步**：`cx.spawn(async move |this, cx| { … })` 执行后台工作；await 后用 `this.update(cx, |this, cx| …)` 修改状态。
- **下拉/输入框**是通过 `cx.new(|cx| …)` 创建的实体；异步数据到达后，动态下拉通过保存的 `AnyWindowHandle` 重建。
- **虚拟列表**：`uniform_list(id, count, |range, window, cx| …)` 只渲染可见行（Topic、消息、消费组——数万行也流畅）。
- **对话框**：`window.open_dialog(cx, |dialog, …| …)`，`on_ok` 回调捕获页面实体句柄。
- **通知**：通过 `window.push_notification` 推送（辅助函数在异步上下文中向所有窗口广播）。

### i18n 与主题

- **i18n**：`app/locales/zh.json` / `en.json`（从旧 Vue `translations.ts` 1:1 转换），编译期内嵌；`t(cx, "messages.title")` 按点键查询。语言通过 `settings.update("ui.language", …)` 持久化。
- **主题**：gpui-component `Theme` 支持亮/暗模式，通过 `Theme::change(...)` 切换，`settings.update("ui.theme", …)` 持久化。

## 流式消息查询

```
MessagesPage                      后端 tokio runtime
     │                                    │
     │── start_message_list_stream ──────►│── 确保集群客户端就绪
     │   (params, CancellationToken)      │── 启动生产者任务
     │◄── mpsc::Receiver<StreamEvent>     │   (按分区消费,
     │                                    │   最小堆合并, 批量发送)
     │── UI 任务：rx.recv() 循环          │
     │    批次追加 → Rc<Vec>              │── 批次间检查 cancel_token
     │    进度 → 进度条                   │
     │── 停止按钮 → token.cancel() ──────►│
```

- 事件：`start` → `batch`* → `order`? → `complete` | `error`（`StreamEvent { event, data }`，`data` 为 JSON 字符串）。
- 服务端 120 秒超时自动取消令牌。
- 消息累积在 `Rc<Vec<Message>>`；表格通过 `uniform_list` 渲染，每帧只构建可见行。
- 生产者任务结束时发送端全部释放，`recv()` 返回 `None`，UI 读取循环随之结束。

## 数据库

SQLite（sqlx，WAL 模式，20 个连接），位于系统数据目录（`Kafka Manager/kafka_manager.db`）。表结构在启动时通过内联 `CREATE TABLE IF NOT EXISTS` 幂等创建——没有迁移框架。

数据表：`kafka_clusters`、`cluster_groups`、`topic_metadata`、`consumer_group_metadata`、`consumer_group_topics`、`consumer_group_offsets`、`topic_templates`、`user_settings`、`json_highlight_templates`、`favorite_groups`、`favorite_items`、`topic_history`、`sent_messages`、`schema_registry_configs`、`schemas`、`telemetry_records`、`resource_tags`（未使用）。

Topic 和消费组列表**缓存在 SQLite** 中（`topic_metadata`、`consumer_group_*`），按需刷新（`topic.refresh`、`consumer_group.refresh`），因此即使集群不可达，导航也能秒开。

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

### 界面

| 优化 | 说明 |
|------|------|
| 虚拟列表 | `uniform_list` 只渲染可见行 |
| Rc 消息缓冲 | 批次共享同一个 `Rc<Vec>`，克隆代价为零 |
| 原生 GPU 渲染 | GPUI 全部 GPU 绘制，无 DOM/webview 开销 |
| 无序列化流式 | 进程内 mpsc，无 HTTP/SSE 边界 |

## 桌面集成

| 功能 | 实现 |
|------|------|
| 系统托盘 | `tray-icon` crate（`app/src/tray.rs`）；显示/退出菜单，启用时关闭窗口改为隐藏（`ui.system_tray` 设置） |
| 开机自启 | `auto-launch` crate（Windows 注册表 / macOS LaunchAgent / Linux .desktop） |
| 单实例 | `single-instance` crate，`main()` 最先检查 |
| 自动更新 | `app/src/updater.rs`——GitHub releases 检查（semver 比较）、后台下载、绿色版 zip 安装 + 重启（Windows：exe 重命名 + PowerShell 替换，从旧 Tauri 壳移植） |
| 文件对话框 | `rfd` crate（消息导出、数据导入导出） |
| 日志 | tracing → `~/.cache/kafka-manager/kafka-manager.log` |

## 部署

```bash
# 开发模式
./start-dev.sh                  # 或：cargo run -p kafka-manager-app

# 生产构建
cargo build --release -p kafka-manager-app
```

二进制完全自包含：核心库静态链接、语言文件内嵌、数据库位于系统数据目录。可在可执行文件旁放置可选的 `config.toml` 预置集群与连接池参数。

## 遥测与更新

- **遥测**：可选的每小时 MySQL 使用上报（`src/telemetry/`），不可达时静默禁用；用户反馈走 `telemetry.submit_feedback`。
- **更新**：应用在启动 3 秒后检查 GitHub releases；有新版本时显示通知，设置页可后台下载并安装绿色版更新。
