# Kafka Manager 架构设计

## 系统概览

Kafka Manager 是一个全栈 Kafka 集群管理应用，采用 Rust 后端和 Vue 3 前端，使用 Tauri 2 打包为跨平台桌面应用。

## 架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                     Kafka Manager 桌面应用                        │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Vue 3 前端                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │   │
│  │  │   视图层     │  │   组件层     │  │   API 客户端      │   │   │
│  │  │   Views     │  │ Components  │  │  (SSE 支持)      │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              │ HTTP / SSE                        │
└──────────────────────────────┼───────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Rust 后端 (Axum)                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  中间件层    │  │  路由层      │  │   服务层               │ │
│  │  - 认证      │  │  - 集群      │  │  - Kafka Admin         │ │
│  │  - 审计      │  │  - Topic    │  │  - Consumer/Producer   │ │
│  │  - 限流      │  │  - 消息      │  │  - 吞吐量统计          │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│           │              │                    │                 │
│           ▼              ▼                    ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   SQLite    │  │   缓存层     │  │   Kafka 集群             │ │
│  │  (SQLx)     │  │   (Moka)    │  │   - Brokers             │ │
│  │             │  │             │  │   - Topics              │ │
│  │             │  │             │  │   - 分区元数据           │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## 后端架构

### 目录结构

```
src/
├── main.rs                 # 应用入口，服务器设置
├── config.rs               # 配置加载 (config.toml, 环境变量)
├── error.rs                # 错误类型和转换
├── lib.rs                  # API 路由注册
├── cache/
│   └── mod.rs              # MetadataCache 缓存配置
├── db/                     # 数据库层
│   ├── mod.rs              # 数据库初始化、建表
│   ├── cluster.rs          # 集群 CRUD 操作
│   ├── cluster_group.rs    # 集群分组管理
│   ├── cluster_connection.rs  # 连接状态管理
│   ├── topic.rs            # Topic 元数据同步和存储
│   ├── topic_template.rs   # Topic 模板管理
│   ├── settings.rs         # 用户设置
│   ├── favorite.rs         # Topic 收藏
│   ├── favorite_group.rs   # 收藏分组管理
│   └── audit_log.rs        # 审计日志记录和查询
├── kafka/                  # Kafka 客户端封装
│   ├── mod.rs              # 模块导出
│   ├── admin.rs            # Admin 客户端 (Topic CRUD, 集群元数据)
│   ├── consumer.rs         # Consumer (支持流式查询)
│   ├── producer.rs         # Producer 发送消息
│   ├── offset.rs           # Offset 和水位管理
│   ├── throughput.rs       # 吞吐量统计
│   ├── transaction.rs      # 事务支持 (Kafka 2.8+)
│   └── import_export.rs    # 数据导入导出工具
├── pool/                   # 连接池
│   ├── mod.rs              # KafkaClients 连接池管理
│   ├── kafka_consumer.rs   # Consumer 连接池
│   └── kafka_producer.rs   # Producer 连接池
├── middleware/             # HTTP 中间件
│   ├── mod.rs              # 中间件注册
│   ├── auth.rs             # API Key 认证
│   ├── audit.rs            # 审计日志记录
│   └── performance.rs      # 请求计时和指标
├── routes/                 # API 路由处理器
│   ├── mod.rs              # 路由注册
│   ├── unified.rs          # 统一 API 处理器（所有 API 请求）
│   ├── cluster.rs          # 集群 CRUD 端点（已废弃，保留兼容）
│   ├── cluster_group.rs    # 集群分组端点（已废弃，保留兼容）
│   ├── cluster_connection.rs   # 连接管理端点（已废弃，保留兼容）
│   ├── topic.rs            # Topic 管理端点（已废弃，保留兼容）
│   ├── topic_template.rs   # Topic 模板端点（已废弃，保留兼容）
│   ├── message.rs          # 消息端点（已废弃，保留兼容）
│   ├── settings.rs         # 设置端点（已废弃，保留兼容）
│   ├── favorite.rs         # 收藏管理端点（已废弃，保留兼容）
│   ├── audit_log.rs        # 审计日志查询（已废弃，保留兼容）
│   ├── health.rs           # 健康检查端点（已废弃，保留兼容）
│   ├── user.rs             # 用户管理（未使用）
│   └── tag.rs              # 标签管理（未使用）
└── models/                 # 数据模型
    └── mod.rs              # 请求/响应模型
```

### 核心设计模式

#### 1. 统一 API 模式

所有 API 请求使用统一的 POST 端点，通过 `X-API-Method` 头区分操作：

```rust
// POST /api with X-API-Method header
match method.as_str() {
    "cluster.list" => handle_cluster_list(state).await,
    "topic.create" => handle_topic_create(state, params).await,
    "message.list" => handle_message_list(state, params).await,
    // ...
}
```

**优势：**
- 单一端点处理所有操作
- 一致的错误处理
- 易于扩展新方法

#### 2. 连接池化

Producer 和 Consumer 连接使用自定义连接池进行池化：

```rust
pub struct ClusterPools {
    /// 集群 ID -> (Consumer Pool, Producer Pool)
    pools: Arc<tokio::sync::RwLock<HashMap<String, (KafkaConsumerPool, KafkaProducerPool)>>,
}
```

**优势：**
- 避免连接创建开销
- 自动清理空闲连接
- 每集群隔离
- 基于 deadpool 的泛型池实现

#### 3. SSE 流式传输

Server-Sent Events 用于实时消息流式传输：

```rust
async fn fetch_messages_streaming_sse(
    tx: mpsc::Sender<Result<Event, Infallible>>,
    // ...
) {
    // 并行分区读取
    // 小顶堆合并排序
    // 通过 channel 实时批量发送
}
```

**优势：**
- UI 渐进式更新
- 降低感知延迟
- 更好的内存效率

#### 4. 中间件链

Axum 中间件栈处理横切关注点：

```rust
let app = Router::new()
    .merge(routes)
    .layer(middleware::from_fn(auth_middleware))      // API Key 认证
    .layer(middleware::from_fn(audit_middleware))      // 审计日志记录
    .layer(middleware::from_fn(performance_middleware)) // 请求计时和指标
    .layer(TimeoutLayer::new(Duration::from_secs(300))) // 5 分钟超时
    .layer(CompressionLayer::new()                     // Gzip/Brotli 压缩
        .gzip(true)
        .br(true))
    .layer(CorsLayer::permissive())                    // CORS 支持
    .layer(TraceLayer::new_for_http());                // 请求追踪
```

**中间件说明：**
- `auth_middleware`: API Key 认证，从 `X-API-Key` 头或查询参数中验证
- `audit_middleware`: 记录所有 API 请求到审计日志
- `performance_middleware`: 追踪请求处理时间，记录慢查询

### 数据库表结构

#### 核心表

```sql
-- 集群配置
CREATE TABLE kafka_clusters (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    brokers TEXT NOT NULL,
    request_timeout_ms INTEGER DEFAULT 30000,
    operation_timeout_ms INTEGER DEFAULT 30000,
    group_id INTEGER REFERENCES cluster_groups(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 集群分组
CREATE TABLE cluster_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 连接历史
CREATE TABLE cluster_connection_history (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT,
    checked_at TEXT NOT NULL
);

-- Topic 元数据缓存
CREATE TABLE topic_metadata (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    partition_count INTEGER NOT NULL,
    replication_factor INTEGER NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    fetched_at TEXT NOT NULL,
    UNIQUE(cluster_id, topic_name)
);

-- 用户设置
CREATE TABLE user_settings (
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(key)
);

-- Topic 收藏分组
CREATE TABLE favorite_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Topic 收藏
CREATE TABLE favorites (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    group_id INTEGER REFERENCES favorite_groups(id),
    remark TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(cluster_id, topic_name)
);

-- 审计日志
CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    cluster_id TEXT,
    resource TEXT,
    action TEXT,
    api_key TEXT,
    status INTEGER,
    duration_ms INTEGER,
    client_ip TEXT
);

-- Topic 模板
CREATE TABLE topic_templates (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    num_partitions INTEGER NOT NULL,
    replication_factor INTEGER NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

## 前端架构

### 目录结构

```
ui/
├── src/
│   ├── api/
│   │   ├── client.ts       # API 客户端（SSE 支持）
│   │   └── types.ts        # TypeScript 类型定义
│   ├── components/         # 可复用组件
│   │   ├── MessageQueryTool.vue    # 消息查询工具
│   │   ├── ClusterTreeNavigator.vue # 集群树导航
│   │   ├── TopicNavigator.vue      # Topic 导航
│   │   ├── FavoriteButton.vue      # 收藏按钮
│   │   └── ...
│   ├── views/              # 页面组件
│   │   ├── ClustersView.vue        # 集群管理
│   │   ├── TopicsView.vue          # Topic 管理
│   │   ├── MessagesView.vue        # 消息浏览
│   │   ├── FavoritesView.vue       # 收藏夹
│   │   ├── SettingsView.vue        # 设置
│   │   └── ...
│   ├── layouts/            # 布局组件
│   │   └── MainLayout.vue  # 主布局
│   ├── stores/             # Pinia 状态管理
│   │   ├── cluster.ts      # 集群状态
│   │   ├── language.ts     # 多语言
│   │   └── ...
│   ├── i18n/               # 国际化
│   │   └── translations.ts # 中英文翻译
│   ├── App.vue
│   └── main.ts
├── src-tauri/              # Tauri 配置
│   ├── src/
│   │   └── lib.rs
│   ├── icons/
│   ├── tauri.conf.json
│   └── Cargo.toml
└── package.json
```

### 状态管理

使用 Pinia 进行响应式状态管理：

```typescript
// stores/cluster.ts
export const useClusterStore = defineStore('cluster', {
  state: () => ({
    clusters: [] as Cluster[],
    groups: [] as Group[],
    clusterHealth: {} as Record<string, HealthStatus>,
  }),
  actions: {
    async fetchClusters() {
      const res = await apiClient.getClusterList();
      this.clusters = res.clusters;
    },
    async fetchGroups() {
      const res = await apiClient.getGroupList();
      this.groups = res.groups;
    },
  },
});
```

### SSE 客户端

```typescript
// api/client.ts
getMessagesStream(
  clusterId: string,
  topic: string,
  params: MessageQueryParams,
  callbacks: {
    onStart?: (data: StartData) => void;
    onBatch?: (messages: Message[], progress: number, total: number) => void;
    onComplete?: () => void;
    onError?: (error: string) => void;
  }
): AbortController {
  // Fetch + ReadableStream 用于 SSE 解析
  // 跨 chunk 的 buffer 处理
  // AbortController 用于取消
}
```

### 国际化

```typescript
// i18n/translations.ts
export const translations: Record<Language, Translation> = {
  zh: {
    nav: {
      clusters: '集群',
      topics: '主题',
      messages: '消息',
    },
    // ...
  },
  en: {
    nav: {
      clusters: 'Clusters',
      topics: 'Topics',
      messages: 'Messages',
    },
    // ...
  },
};
```

## API 设计

### 统一 POST API

所有操作使用 `POST /api`，通过 `X-API-Method` 头区分：

| 类别 | 方法 |
|------|------|
| **健康检查** | `health` |
| **集群** | `cluster.list`, `cluster.get`, `cluster.create`, `cluster.update`, `cluster.delete`, `cluster.test`, `cluster.test_config`, `cluster.stats` |
| **集群分组** | `cluster_group.list`, `cluster_group.get`, `cluster_group.create`, `cluster_group.update`, `cluster_group.delete`, `cluster_group.clusters`, `cluster_group.assign_cluster` |
| **连接** | `connection.list`, `connection.get`, `connection.disconnect`, `connection.reconnect`, `connection.health_check`, `connection.metrics`, `connection.history`, `connection.stats`, `connection.batch_disconnect`, `connection.batch_reconnect` |
| **Topic** | `topic.list`, `topic.list_with_cluster`, `topic.get`, `topic.create`, `topic.delete`, `topic.delete_all`, `topic.batch_create`, `topic.batch_delete`, `topic.offsets`, `topic.config_get`, `topic.config_alter`, `topic.partitions_add`, `topic.partition.watermarks`, `topic.throughput`, `topic.refresh`, `topic.saved`, `topic.search`, `topic.count`, `topic.cleanup_orphans` |
| **消息** | `message.list`, `message.send`, `message.export` |
| **设置** | `settings.get`, `settings.update` |
| **Topic 模板** | `template.list`, `template.get`, `template.create`, `template.update`, `template.delete`, `template.presets`, `template.create_topic` |
| **收藏** | `favorite.group.list`, `favorite.group.create`, `favorite.group.get`, `favorite.group.update`, `favorite.group.delete`, `favorite.list`, `favorite.create`, `favorite.get`, `favorite.update`, `favorite.delete`, `favorite.check`, `favorite.delete_by_topic` |
| **审计日志** | `audit_log.list` |

### 响应格式

成功响应：
```json
{
  "success": true,
  "data": { ... }
}
```

错误响应：
```json
{
  "success": false,
  "error": "错误信息"
}
```

## 消息查询设计

消息查询功能支持以下特性：

- 每分区 max_messages 限制
- 串行/并行模式选择（基于本地/远程集群自动选择）
- 基于时间戳的 Offset 定位
- 水位边界处理
- SSE 流式传输与小顶堆合并排序
- 两阶段查询（先收集后过滤）
- 并发控制（最多 10 个并行分区）

## 性能优化

### 后端

| 优化 | 描述 |
|------|------|
| 连接池化 | 使用自定义连接池进行 Producer/Consumer 池化，每集群独立管理 |
| 查询缓存 | MetadataCache 缓存 Topic 列表等热点数据 |
| 异步 I/O | 完全基于 Tokio 的异步架构 |
| 批量操作 | 支持批量创建/删除 Topic |
| 分区并行 | 消息查询支持并行/串行模式选择 |
| 唯一 group.id | 每分区独立的消费者 group.id |
| 显式 seek | assign 后 seek 以保证 offset 稳定 |
| 水位检查 | clamp() 前的边界验证 |
| 超时控制 | 请求级别 5 分钟超时限制 |
| 压缩传输 | 支持 Gzip 和 Brotli 压缩 |

### 前端

| 优化 | 描述 |
|------|------|
| 虚拟滚动 | vue-virtual-scroller 处理大数据列表 |
| shallowRef | 消息数组使用浅响应式 |
| 节流更新 | 每 200ms 批量更新 UI |
| 非响应式缓冲 | pendingMessages 使用普通数组 |
| SSE 流式 | 渐进式渲染 |

## 部署

### 开发环境

```bash
# 后端
cargo run

# 前端（开发模式）
cd ui && npm run tauri dev
```

### 生产环境

```bash
# 构建后端
cargo build --release

# 构建桌面应用
cd ui && npm run tauri build
```

### 配置

通过 `config.toml` 和环境变量进行配置：

```toml
[server]
host = "0.0.0.0"
port = 9732

[database]
path = "kafka_manager.db"

[pool]
max_connections = 10
```

## 监控

### 健康检查

- 定期集群健康检查（可配置间隔）
- 连接历史追踪到 `cluster_connection_history` 表
- 通过 `connection.health_check` API 获取实时状态

### 指标

- 审计日志中追踪请求持续时间
- 慢查询检测（>5s 请求记录为警告）
- 通过 `connection.metrics` API 获取连接池指标

## 安全

### 认证

- 通过 `X-API-Key` 头进行 API Key 验证
- 白名单路径跳过认证（`/api/health`）
- 通过 `API_KEYS` 环境变量配置 API Keys
- 支持从数据库动态管理 API Keys

### 审计

- 所有 API 请求记录到 `audit_logs` 表
- 包括方法、路径、集群、资源、操作、状态、持续时间、客户端 IP
- 支持分页查询和历史追溯
