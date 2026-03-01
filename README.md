# Kafka Manager

一个使用 Rust 实现的 Kafka 管理工具后端，支持多集群管理和 SQLite 持久化存储。

## 功能

### 已实现功能

| 功能模块 | 状态 | 说明 |
|---------|------|------|
| **集群管理** | ✅ | 支持多集群配置，SQLite 持久化存储 |
| **Topic 管理** | ✅ | 完整的 CRUD 操作，支持配置管理、批量操作、**自动同步到数据库** |
| **Topic Offset 查询** | ✅ | 查看各分区的 earliest/latest offset 和时间戳 |
| **消息管理** | ✅ | 发送消息、查看消息（支持分区/offset 过滤、排序、搜索、JSON/Hex 格式化） |
| **Consumer Group 管理** | ⚠️ | 支持删除和 offset 重置，列表/详情功能受 rdkafka 限制 |
| **Consumer Group Offset** | ✅ | 查看消费进度和 Lag，支持重置 offset |
| **集群监控** | ✅ | Broker 列表、详情、集群指标 |
| **Schema Registry** | ✅ | 完整的 Confluent Schema Registry 客户端 |
| **ACL 权限管理** | ❌ | rdkafka 0.39 不支持 |
| **API 认证** | ✅ | 基于 API Key 的认证中间件（可选启用） |
| **审计日志** | ✅ | 自动记录所有 API 操作日志 |
| **告警规则管理** | ✅ | 支持 Consumer Lag、Produce Rate、Consumer Rate 告警 |
| **标签管理** | ✅ | 为 Topic/Consumer Group 添加自定义标签，支持按标签筛选 |
| **Topic 元数据同步** | ✅ | 刷新 Topic 列表到数据库，集群变更时自动同步 |

### 新增功能（本次更新）

#### 1. API 认证中间件
- 基于 API Key 的认证
- 通过 `X-API-Key` Header 传递
- 支持通过环境变量启用：
  - `API_KEYS`: 逗号分隔的 API Key 列表
  - `AUTH_ENABLED`: 是否启用认证（true/false）

#### 2. 审计日志
- 自动记录所有 API 请求
- 记录内容包括：操作类型、路径、集群 ID、资源、耗时、状态码等
- 支持客户端 IP 追踪
- API Key 自动脱敏

#### 3. 批量操作 API
- **批量创建 Topic**: `POST /api/clusters/:cluster_id/topics/batch`
- **批量删除 Topic**: `DELETE /api/clusters/:cluster_id/topics/batch`
- 支持 `continue_on_error` 选项，失败后继续执行

#### 4. 增强消息预览
- 返回更丰富的消息信息
- 支持 JSON 格式化显示
- 支持 Hex 格式显示
- 自动推断内容类型
- 显示消息大小

#### 5. Topic 元数据同步（最新）
- **手动刷新**: `POST /api/clusters/:cluster_id/topics/refresh`
- **自动同步**: 创建/更新/删除集群时自动同步 Topic 列表
- **数据库持久化**: 新增 `topic_metadata` 表存储 Topic 元数据
- **差异对比**: 返回新增和删除的 Topic 列表

### 详细功能列表

#### 集群管理（SQLite 持久化）
- 列出所有 Kafka 集群
- 添加新集群
- 编辑集群配置
- 删除集群
- 测试集群连接

#### Topic 管理
- 列出所有 Topic
- 创建 Topic (可配置分区数、副本数、配置项)
- 查看 Topic 详情 (分区、副本分布)
- 删除 Topic
- 增加分区数
- 查看/修改 Topic 配置
- 查看 Topic 分区 Offset 信息 (earliest/latest offset)

#### 消息管理
- 发送消息到 Topic (支持指定分区)
- 查看 Topic 消息
  - 支持按分区、offset 过滤
  - 支持按时间戳/offset 排序 (升序/降序)
  - 支持关键词搜索 (key/value)
  - 支持限制返回数量

#### Consumer Group 管理
- 列出 Consumer Groups (受 rdkafka 限制，返回空列表)
- 查看 Consumer Group 详情 (简化数据)
- 删除 Consumer Group
- 查看 Consumer Group Offset 和 Lag
- **重置 Consumer Group Offset**
  - 重置到 earliest offset
  - 重置到 latest offset
  - 重置到指定 offset 值
  - 重置到指定时间戳对应的 offset

#### 集群监控
- 查看集群信息（Brokers、Topic 统计）
- 查看集群监控指标
- 查看 Broker 详情
- 列出所有 Broker

#### Schema Registry 集成
- 完整的 Confluent Schema Registry HTTP API 客户端
- 支持列出 subject、获取 schema、注册 schema 等
- 支持兼容性检查

### 功能限制

由于 rdkafka 0.39 的限制，以下功能暂不支持：

- **ACL 权限管理** - rdkafka 0.39 不支持 ACL API，建议使用 Kafka REST Proxy 或 Kafka CLI 工具
- **Consumer Group 列表** - rdkafka 0.39 未暴露 ListGroups API，返回空列表
- **Consumer Group 详情** - 无法获取实际状态和成员信息，返回简化数据
- **Controller ID/Cluster ID** - 暂不支持获取

## 快速开始

### 运行

```bash
cargo run
```

首次运行时会自动创建 SQLite 数据库文件 `kafka_manager.db`。

### 配置

服务器配置通过 `config.toml` 或环境变量：

```toml
[server]
host = "127.0.0.1"
port = 3000
```

Kafka 集群配置通过 API 动态管理，存储在 SQLite 数据库中。

## API 端点

### 集群管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters | 列出所有集群 |
| POST | /api/clusters | 创建集群 |
| GET | /api/clusters/:id | 获取集群详情 |
| PUT | /api/clusters/:id | 更新集群 |
| DELETE | /api/clusters/:id | 删除集群 |
| POST | /api/clusters/:id/test | 测试集群连接 |

### Topic 管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/topics | 列出所有 Topic |
| POST | /api/clusters/:cluster_id/topics | 创建 Topic |
| GET | /api/clusters/:cluster_id/topics/:name | 查看 Topic 详情 |
| DELETE | /api/clusters/:cluster_id/topics/:name | 删除 Topic |
| POST | /api/clusters/:cluster_id/topics/:name/partitions | 增加分区数 |
| GET | /api/clusters/:cluster_id/topics/:name/config | 查看 Topic 配置 |
| POST | /api/clusters/:cluster_id/topics/:name/config | 修改 Topic 配置 |
| GET | /api/clusters/:cluster_id/topics/:name/offsets | 查看 Topic 各分区 Offset 信息 |
| POST | /api/clusters/:cluster_id/topics/refresh | **刷新 Topic 列表（同步到数据库）** |

### 消息管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/:topic/messages | 查看消息 |
| POST | /api/clusters/:cluster_id/:topic/messages | 发送消息 |

### Consumer Group

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/consumer-groups | 列出 Consumer Groups |
| GET | /api/clusters/:cluster_id/consumer-groups/:name | Consumer Group 详情 |
| DELETE | /api/clusters/:cluster_id/consumer-groups/:name | 删除 Consumer Group |
| GET | /api/clusters/:cluster_id/consumer-groups/:name/offsets | 查看 Consumer Group Offset 和 Lag |
| POST | /api/clusters/:cluster_id/consumer-groups/:name/offsets/reset | 重置 Consumer Group Offset |

### 集群监控

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/info | 获取集群信息（Brokers、Topic 统计） |
| GET | /api/clusters/:cluster_id/metrics | 获取集群监控指标 |
| GET | /api/clusters/:cluster_id/brokers | 列出所有 Broker |
| GET | /api/clusters/:cluster_id/brokers/:broker_id | 获取 Broker 详情 |

### 健康检查

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/health | 健康检查 |

## API 示例

完整 API 示例文档请查看 [docs/API_EXAMPLES.md](docs/API_EXAMPLES.md)

### 快速示例

#### 创建 Kafka 集群

```bash
curl -X POST http://localhost:3000/api/clusters \
  -H "Content-Type: application/json" \
  -d '{
    "name": "development",
    "brokers": "localhost:9092",
    "request_timeout_ms": 5000,
    "operation_timeout_ms": 5000
  }'
```

#### 发送消息

```bash
curl -X POST http://localhost:3000/api/clusters/development/topics/test-topic/messages \
  -H "Content-Type: application/json" \
  -d '{
    "key": "user-123",
    "value": "Hello, Kafka!"
  }'
```

#### 查看 Consumer Group Lag

```bash
curl "http://localhost:3000/api/clusters/development/consumer-groups/my-group/offsets?topic=test-topic"
```

---

## Schema Registry 集成

Schema Registry 功能通过 HTTP API 实现，支持 Confluent Schema Registry。

**使用方法**:

```rust
use kafka_manager::kafka::schema_registry::{SchemaRegistryClient, SchemaResponse};

let client = SchemaRegistryClient::new("http://localhost:8081", 5000);

// 获取所有 subject
let subjects = client.list_subjects().await?;

// 获取最新 schema
let schema: SchemaResponse = client.get_latest_schema("my-topic-value").await?;

// 注册新 schema
let response = client.register_schema("my-topic-value", &schema_json, "AVRO").await?;
```

**支持的 API**:
- `list_subjects()` - 获取所有 subject
- `list_versions(subject)` - 获取 subject 的所有版本
- `get_schema(subject, version)` - 获取指定版本的 schema
- `get_latest_schema(subject)` - 获取最新版本的 schema
- `register_schema(subject, schema, schema_type)` - 注册新 schema
- `delete_subject(subject)` - 删除 subject
- `delete_version(subject, version)` - 删除指定版本
- `check_compatibility(subject, version)` - 检查兼容性
- `get_compatibility_level()` - 获取全局兼容性级别
- `set_compatibility_level(subject, level)` - 设置兼容性级别

## 项目结构

```
kafka-manager/
├── src/
│   ├── main.rs           # 程序入口
│   ├── lib.rs            # 测试库入口
│   ├── config.rs         # 配置加载
│   ├── db/               # 数据库模块
│   │   ├── mod.rs        # DbPool 连接池
│   │   └── cluster.rs    # 集群 CRUD 操作
│   ├── error.rs          # 错误处理
│   ├── kafka/            # Kafka 客户端封装
│   │   ├── mod.rs        # KafkaClients 管理多集群连接
│   │   ├── admin.rs      # Admin API (Topic 管理、Consumer Group 管理、集群信息)
│   │   ├── consumer.rs   # Consumer 客户端
│   │   ├── producer.rs   # Producer 客户端
│   │   ├── offset.rs     # Offset 管理和 Consumer Group 查询
│   │   └── schema_registry.rs # Schema Registry 客户端
│   ├── models/           # 数据模型
│   │   └── mod.rs
│   └── routes/           # API 路由
│       ├── mod.rs
│       ├── cluster.rs    # 集群管理 API
│       ├── topic.rs      # Topic 管理 API
│       ├── message.rs    # 消息 API
│       ├── consumer_group.rs # Consumer Group API
│       └── health.rs     # 健康检查
├── migrations/           # 数据库迁移
│   └── 001_create_kafka_clusters.sql
├── tests/                # 集成测试
│   └── api_test.rs
├── Cargo.toml
└── config.toml
```

## 数据库

使用 SQLite 存储 Kafka 集群配置：

- **数据库文件**: `kafka_manager.db`
- **表结构**:
  - `kafka_clusters`: 存储集群配置
    - `id`: 主键
    - `name`: 集群名称（唯一）
    - `brokers`: Kafka broker 地址
    - `request_timeout_ms`: 请求超时
    - `operation_timeout_ms`: 操作超时
    - `created_at`: 创建时间
    - `updated_at`: 更新时间

## 测试

运行所有测试:

```bash
cargo test
```

运行单元测试:

```bash
cargo test --lib
```

运行集成测试:

```bash
cargo test --test api_test
```

## 依赖

- **axum 0.7**: Web 框架
- **tokio**: 异步运行时
- **rdkafka 0.39**: Kafka 客户端
- **sqlx 0.8**: SQLite 数据库
- **serde**: 序列化/反序列化
- **chrono**: 时间处理
- **tower-http**: 中间件（CORS、Trace）
- **reqwest 0.11**: HTTP 客户端（Schema Registry）
