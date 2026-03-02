# Kafka Manager

一个使用 Rust 实现的 Kafka 管理工具后端，支持多集群管理和 SQLite 持久化存储。

## 功能

### 核心功能

| 功能模块 | 状态 | 说明 |
|---------|------|------|
| **集群管理** | ✅ | 支持多集群配置，SQLite 持久化存储 |
| **Topic 管理** | ✅ | 完整的 CRUD 操作，支持配置管理、批量操作、自动同步到数据库 |
| **Topic Offset 查询** | ✅ | 查看各分区的 earliest/latest offset |
| **消息管理** | ✅ | 发送消息、查看消息（支持分区/offset 过滤、排序、搜索、JSON/Hex 格式化） |
| **Consumer Group 管理** | ⚠️ | 支持删除和 offset 重置，列表/详情功能受 rdkafka 限制 |
| **Consumer Group Offset** | ✅ | 查看消费进度和 Lag，支持重置 offset |
| **集群监控** | ✅ | Broker 列表、详情、集群指标 |
| **Schema Registry** | ✅ | 完整的 Confluent Schema Registry HTTP API 客户端 |
| **API 认证** | ✅ | 基于 API Key 的认证中间件（可选启用） |
| **审计日志** | ✅ | 自动记录所有 API 操作日志 |
| **标签管理** | ✅ | 为 Topic/Consumer Group 添加自定义标签，支持按标签筛选 |
| **Topic 模板** | ✅ | 预定义 Topic 配置模板，支持一键创建标准化 Topic |
| **告警规则** | ✅ | 支持 Consumer Lag、Produce Rate、Consumer Rate 告警 |
| **通知管理** | ✅ | 支持钉钉、企业微信、Slack、Webhook 等通知渠道 |
| **RBAC 用户管理** | ✅ | 基于角色的权限控制，内置 admin/operator/viewer 角色 |
| **集群连接管理** | ✅ | 集群连接状态监控、断开/重连、健康检查 |
| **全局设置** | ✅ | 用户偏好设置存储 |
| **Task 任务管理** | ✅ | 异步任务状态追踪 |

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
| GET | /api/clusters/:cluster_id/topics/:name/throughput | 查看 Topic 生产速率 |
| GET | /api/clusters/:cluster_id/topics/:name/messages/export | 导出 Topic 消息 |
| POST | /api/clusters/:cluster_id/topics/refresh | 刷新 Topic 列表（同步到数据库） |
| GET | /api/clusters/:cluster_id/topics/:name/consumer-lag | 查看 Topic 所有 Consumer Group 积压情况 |
| GET | /api/clusters/:cluster_id/topics/:name/consumer-lag-history | 查看 Consumer Group 历史积压数据 |
| GET | /api/clusters/:cluster_id/topics/:name/tags | 获取 Topic 标签列表 |
| POST | /api/clusters/:cluster_id/topics/:name/tags | 为 Topic 添加标签 |

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
| GET | /api/clusters/:cluster_id/consumer-groups/:name/throughput | 查看 Consumer Group 消费速率 |
| GET | /api/clusters/:cluster_id/consumer-groups/:name/tags | 获取 Consumer Group 标签列表 |

### 集群监控

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/info | 获取集群信息（Brokers、Topic 统计） |
| GET | /api/clusters/:cluster_id/metrics | 获取集群监控指标 |
| GET | /api/clusters/:cluster_id/brokers | 列出所有 Broker |
| GET | /api/clusters/:cluster_id/brokers/:broker_id | 获取 Broker 详情 |
| GET | /api/clusters/:cluster_id/brokers/:broker_id/logdirs | 获取 Broker 日志目录 |
| GET | /api/clusters/:cluster_id/brokers/:broker_id/metrics | 获取 Broker 详细指标 |
| GET | /api/clusters/:cluster_id/stats | 获取集群统计 Dashboard |

### 健康检查

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/health | 健康检查 |
| GET | /api/ready | 就绪检查 |

### 集群连接管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/cluster-connections | 获取所有集群连接状态 |
| GET | /api/cluster-connections/:id/status | 获取指定集群连接状态 |
| POST | /api/cluster-connections/:id/disconnect | 断开集群连接 |
| POST | /api/cluster-connections/:id/reconnect | 重连集群 |
| POST | /api/cluster-connections/:id/health-check | 执行健康检查 |

### 认证管理

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/auth/keys | 创建 API Key |
| GET | /api/auth/keys | 获取 API Key 列表 |
| DELETE | /api/auth/keys/:id | 删除 API Key |

### 审计日志

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/audit-logs | 获取审计日志列表 |
| DELETE | /api/audit-logs | 清理旧审计日志 |

### 用户与权限

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/users | 获取用户列表 |
| POST | /api/users | 创建用户 |
| GET | /api/users/:id | 获取用户详情 |
| PUT | /api/users/:id | 更新用户 |
| PUT | /api/users/:id/password | 更新密码 |
| GET | /api/roles | 获取角色列表 |
| POST | /api/roles | 创建角色 |
| GET | /api/roles/:id | 获取角色详情 |
| PUT | /api/roles/:id | 更新角色 |

### 通知管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/notifications | 获取通知配置列表 |
| POST | /api/notifications | 创建通知配置 |
| GET | /api/notifications/:id | 获取通知配置详情 |
| POST | /api/notifications/:id/enable | 启用通知 |
| POST | /api/notifications/:id/disable | 禁用通知 |
| GET | /api/alerts/history | 获取告警历史 |

### 告警规则

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/alert-rules | 获取告警规则列表 |
| POST | /api/alert-rules | 创建告警规则 |
| GET | /api/alert-rules/:id | 获取告警规则详情 |
| PUT | /api/alert-rules/:id | 更新告警规则 |
| DELETE | /api/alert-rules/:id | 删除告警规则 |

### Schema Registry

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/schema-registry/ | 获取 Subjects 列表 |
| GET | /api/schema-registry/:subject | 获取版本列表 |
| GET | /api/schema-registry/:subject/:version | 获取 Schema 详情 |
| POST | /api/schema-registry/register | 注册 Schema |
| DELETE | /api/schema-registry/:subject | 删除 Subject |
| DELETE | /api/schema-registry/:subject/:version | 删除 Schema 版本 |
| POST | /api/schema-registry/compatibility | 检查兼容性 |
| GET | /api/schema-registry/compatibility-level | 获取兼容性级别 |
| PUT | /api/schema-registry/compatibility-level | 更新兼容性级别 |

### 标签管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/tags/keys | 列出所有标签键 |
| GET | /api/clusters/:cluster_id/tags/:key/values | 列出标签键对应的所有值 |
| GET | /api/clusters/:cluster_id/tags/filter | 按标签过滤资源 |
| GET | /api/clusters/:cluster_id/topics/:name/tags | 获取 Topic 标签列表 |
| POST | /api/clusters/:cluster_id/topics/:name/tags | 为 Topic 添加标签 |
| PUT | /api/clusters/:cluster_id/topics/:name/tags/:key | 更新 Topic 标签 |
| DELETE | /api/clusters/:cluster_id/topics/:name/tags/:key | 删除 Topic 标签 |
| DELETE | /api/clusters/:cluster_id/topics/:name/tags/all | 删除 Topic 的所有标签 |

### Topic 模板

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/topic-templates/presets | 列出预定义模板 |
| GET | /api/topic-templates | 列出所有自定义模板 |
| POST | /api/topic-templates | 创建自定义模板 |
| GET | /api/topic-templates/:id | 获取模板详情 |
| PUT | /api/topic-templates/:id | 更新模板 |
| DELETE | /api/topic-templates/:id | 删除模板 |
| POST | /api/topics/from-template | 使用模板创建 Topic |

### 全局设置

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/settings | 获取所有设置 |
| GET | /api/settings/:key | 获取指定设置 |
| PUT | /api/settings/:key | 更新设置 |

### 任务管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/tasks | 获取任务列表 |
| GET | /api/tasks/:id | 获取任务状态 |
| DELETE | /api/tasks/:id | 删除任务 |

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
│   ├── main.rs                 # 程序入口
│   ├── lib.rs                  # 测试库入口
│   ├── config.rs               # 配置加载
│   ├── pool/                   # 连接池管理
│   │   ├── mod.rs              # KafkaClients 连接池
│   │   ├── kafka_consumer.rs   # Consumer 连接池
│   │   └── kafka_producer.rs   # Producer 连接池
│   ├── db/                     # 数据库模块
│   │   ├── mod.rs              # DbPool 连接池
│   │   ├── cluster.rs          # 集群 CRUD 操作
│   │   ├── topic.rs            # Topic 元数据管理
│   │   ├── topic_template.rs   # Topic 模板管理
│   │   ├── consumer_group.rs   # Consumer Group 相关
│   │   ├── user.rs             # 用户和角色管理
│   │   ├── api_key.rs          # API Key 管理
│   │   ├── audit_log.rs        # 审计日志
│   │   ├── notification.rs     # 通知配置
│   │   ├── tag.rs              # 资源标签
│   │   ├── settings.rs         # 全局设置
│   │   └── cluster_connection.rs # 集群连接管理
│   ├── kafka/                  # Kafka 客户端封装
│   │   ├── mod.rs              # Kafka 客户端模块
│   │   ├── admin.rs            # Admin API
│   │   ├── consumer.rs         # Consumer 客户端
│   │   ├── producer.rs         # Producer 客户端
│   │   ├── offset.rs           # Offset 管理
│   │   ├── throughput.rs       # 吞吐量统计
│   │   ├── schema.rs           # Schema 管理
│   │   ├── schema_registry.rs  # Schema Registry 客户端
│   │   ├── transaction.rs      # 事务管理
│   │   └── import_export.rs    # 数据导入导出
│   ├── routes/                 # API 路由
│   │   ├── mod.rs              # 路由注册
│   │   ├── cluster.rs          # 集群管理 API
│   │   ├── topic.rs            # Topic 管理 API
│   │   ├── message.rs          # 消息 API
│   │   ├── consumer_group.rs   # Consumer Group API
│   │   ├── cluster_monitor.rs  # 集群监控 API
│   │   ├── cluster_stats.rs    # 集群统计 API
│   │   ├── health.rs           # 健康检查
│   │   ├── auth.rs             # 认证管理 API
│   │   ├── audit_log.rs        # 审计日志 API
│   │   ├── user.rs             # 用户管理 API
│   │   ├── notification.rs     # 通知管理 API
│   │   ├── schema.rs           # Schema Registry API
│   │   ├── tag.rs              # 标签管理 API
│   │   ├── topic_template.rs   # Topic 模板 API
│   │   ├── settings.rs         # 全局设置 API
│   │   └── cluster_connection.rs # 集群连接管理 API
│   ├── middleware/             # 中间件
│   │   ├── mod.rs
│   │   ├── auth.rs             # API Key 认证中间件
│   │   └── audit.rs            # 审计日志中间件
│   ├── task/                   # 异步任务
│   │   ├── mod.rs
│   │   └── health_check.rs     # 定期健康检查任务
│   ├── models/                 # 数据模型
│   │   └── mod.rs
│   ├── error.rs                # 错误处理
│   └── tests/                  # 测试模块
│       ├── mod.rs
│       ├── models.rs
│       └── error.rs
├── migrations/                 # 数据库迁移
│   ├── 001_create_kafka_clusters.sql
│   └── 001_add_indexes.sql
├── tests/                      # 集成测试
│   └── api_test.rs
├── Cargo.toml
└── config.toml
```

## 数据库

使用 SQLite 存储配置和元数据：

- **数据库文件**: `kafka_manager.db`
- **表结构**:

| 表名 | 描述 |
|------|------|
| `kafka_clusters` | 集群配置 |
| `topic_metadata` | Topic 元数据（分区数、副本数、配置） |
| `topic_templates` | Topic 配置模板 |
| `cluster_connection_history` | 集群连接历史记录 |
| `api_keys` | API Key 存储（哈希存储，前缀索引） |
| `audit_logs` | 审计日志（记录所有 API 请求） |
| `users` | 用户账户 |
| `roles` | 角色定义（权限配置） |
| `resource_tags` | 资源标签（Topic/Consumer Group） |
| `notification_configs` | 通知配置（钉钉/企业微信/Slack 等） |
| `consumer_lag_history` | Consumer Lag 历史数据 |
| `user_settings` | 用户全局设置 |

### 核心表结构

**kafka_clusters** - 集群配置
```sql
- id: 主键
- name: 集群名称（唯一）
- brokers: Kafka broker 地址
- request_timeout_ms: 请求超时
- operation_timeout_ms: 操作超时
- created_at: 创建时间
- updated_at: 更新时间
```

**topic_metadata** - Topic 元数据
```sql
- id: 主键
- cluster_id: 集群 ID
- topic_name: Topic 名称
- partition_count: 分区数
- replication_factor: 副本数
- config_json: 配置项（JSON 格式）
- fetched_at: 获取时间
```

**audit_logs** - 审计日志
```sql
- id: 主键
- timestamp: 时间戳
- method: HTTP 方法
- path: 请求路径
- cluster_id: 集群 ID
- resource: 资源名称
- action: 操作类型
- api_key: API Key（脱敏）
- status: 响应状态码
- duration_ms: 请求耗时
- client_ip: 客户端 IP
```

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
