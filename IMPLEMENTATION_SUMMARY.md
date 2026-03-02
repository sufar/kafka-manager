# Kafka Manager 实现总结

## 已完成功能

### 1. 核心架构
- **Web 框架**: Axum (Tokio ecosystem)
- **数据库**: SQLite with SQLx
- **Kafka 客户端**: rdkafka
- **连接池**: 自定义 Kafka Consumer/Producer 连接池
- **认证**: API Key + RBAC
- **中间件**: 认证、审计日志、性能监控

### 2. 数据库层 (`src/db/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 集群管理 | `cluster.rs` | 集群 CRUD 操作 |
| 集群连接 | `cluster_connection.rs` | 连接状态管理 |
| 用户管理 | `user.rs` | 用户和角色管理，bcrypt 密码哈希 |
| 通知 | `notification.rs` | 通知配置管理 |
| 审计日志 | `audit_log.rs` | API 请求审计 |
| 资源标签 | `tag.rs` | 资源标签管理 |
| Topic 管理 | `topic.rs` | Topic 元数据同步和存储 |
| Topic 模板 | `topic_template.rs` | Topic 创建模板 |
| API Key | `api_key.rs` | API Key 存储和验证 |
| 设置 | `settings.rs` | 全局用户设置 |

### 3. Kafka 模块 (`src/kafka/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| Admin | `admin.rs` | Kafka Admin 客户端封装 |
| Consumer | `consumer.rs` | Kafka Consumer 封装，支持流式查询 |
| Producer | `producer.rs` | Kafka Producer 封装 |
| Offset | `offset.rs` | Offset 管理 |
| Throughput | `throughput.rs` | 吞吐量统计 |
| Schema | `schema.rs` | Schema 管理 |
| Schema Registry | `schema_registry.rs` | Schema Registry HTTP 客户端 |
| Transaction | `transaction.rs` | 事务管理（Kafka 2.8+） |
| Import/Export | `import_export.rs` | 数据导入导出工具 |

### 4. 连接池层 (`src/pool/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 连接池管理 | `mod.rs` | KafkaClients 连接池管理 |
| Consumer 池 | `kafka_consumer.rs` | Consumer 连接池 |
| Producer 池 | `kafka_producer.rs` | Producer 连接池 |

### 5. 中间件 (`src/middleware/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 认证 | `auth.rs` | API Key 认证中间件 |
| 审计 | `audit.rs` | 自动审计日志记录 |
| 性能 | `performance.rs` | 性能监控中间件 |

### 6. 数据库表

| 表名 | 描述 |
|------|------|
| `kafka_clusters` | 集群配置 |
| `cluster_connection_history` | 连接历史 |
| `api_keys` | API Key 存储 |
| `audit_logs` | 审计日志 |
| `users`, `roles` | 用户和角色 |
| `notification_configs` | 通知配置 |
| `consumer_lag_history` | Consumer Lag 历史 |
| `resource_tags` | 资源标签 |
| `topic_templates` | Topic 模板 |
| `topic_metadata` | Topic 元数据 |
| `user_settings` | 用户全局设置 |

### 7. 路由层 (`src/routes/`)

| 模块 | 路径前缀 | 功能 |
|------|---------|------|
| `cluster.rs` | `/api/clusters` | 集群 CRUD |
| `cluster_connection.rs` | `/api/cluster-connections` | 集群连接管理 |
| `topic.rs` | `/api/clusters/:cluster_id/topics` | Topic 管理 |
| `consumer_group.rs` | `/api/clusters/:cluster_id/consumer-groups` | Consumer Group 管理 |
| `message.rs` | `/api/clusters/:cluster_id` | 消息浏览 |
| `cluster_stats.rs` | `/api/clusters/:cluster_id/stats` | 集群统计 |
| `cluster_monitor.rs` | `/api/clusters/:cluster_id` | 集群监控 |
| `schema.rs` | `/api/schema-registry` | Schema Registry 管理 |
| `notification.rs` | `/api/notifications` | 通知配置管理 |
| `user.rs` | `/api/users`, `/api/roles` | 用户和角色管理 |
| `auth.rs` | `/api/auth/keys` | API Key 认证 |
| `audit_log.rs` | `/api/audit-logs` | 审计日志查询 |
| `tag.rs` | `/api/clusters/:cluster_id` | 资源标签 |
| `topic_template.rs` | `/api` | Topic 模板管理 |
| `settings.rs` | `/api/settings` | 全局设置 |
| `health.rs` | `/api` | 健康检查 |

### 8. 认证和授权

| 模块 | 路径前缀 | 功能 |
|------|---------|------|
| `cluster.rs` | `/api/clusters` | 集群 CRUD（自动同步 Topic） |
| `cluster_connection.rs` | `/api/cluster-connections` | 集群连接管理（断开/重连/健康检查） |
| `topic.rs` | `/api/clusters/:cluster_id/topics` | Topic 管理（**支持刷新同步**） |
| `consumer_group.rs` | `/api/clusters/:cluster_id/consumer-groups` | Consumer Group 管理 |
| `message.rs` | `/api/clusters/:cluster_id` | 消息浏览 |
| `cluster_stats.rs` | `/api/clusters/:cluster_id/stats` | 集群统计 |
| `cluster_monitor.rs` | `/api/clusters/:cluster_id/monitor` | 集群监控 |
| `acl.rs` | `/api/acls` | ACL 管理 |
| `quota.rs` | `/api/quotas` | Quota 管理 |
| `schema.rs` | `/api/schema-registry` | Schema Registry 管理 |
| `notification.rs` | `/api/notifications` | 通知配置管理 |
| `alert_rule.rs` | `/api/alert-rules` | 告警规则管理 |
| `rebalance.rs` | `/api/rebalance` | Rebalance 监控 |
| `export.rs` | `/api/data` | 数据导入导出 |
| `user.rs` | `/api/users`, `/api/roles` | 用户和角色管理 |
| `auth.rs` | `/api/auth/keys` | API Key 认证 |
| `audit_log.rs` | `/api/audit-logs` | 审计日志查询 |
| `tag.rs` | `/api/clusters/:cluster_id/tags` | 资源标签 |
| `topic_template.rs` | `/api` | Topic 模板管理 |
| `health.rs` | `/api` | 健康检查 |

### 8. 认证和授权

**认证中间件** (`src/middleware/auth.rs`):
- API Key 验证（从 `X-API-Key` 头）
- 支持路径白名单跳过认证
- 可配置的环境变量：`API_KEYS`, `AUTH_ENABLED`

**RBAC 权限系统**:
- 基于角色的访问控制
- 权限格式：`resource:action` (如 `topic:read`, `topic:*`)
- 默认角色：
  - `admin`: 所有权限 (`*`)
  - `operator`: 运维权限
  - `viewer`: 只读权限

### 9. 告警通知系统

**支持的通知渠道**:
- Webhook (通用 HTTP 回调)
- 钉钉机器人 (支持 HMAC 签名)
- 企业微信机器人
- Slack Webhook
- 邮件 (SMTP)

**告警类型**:
- Consumer Lag (消费延迟)
- Produce Rate (生产速率)
- Consumer Rate (消费速率)

**告警级别**:
- `warning`: 警告级别
- `critical`: 严重级别

### 10. 集群连接管理

**功能**:
- 获取所有集群连接状态
- 获取单个集群连接状态
- 主动断开连接
- 主动重连
- 健康检查

**连接状态**:
- `Connected`: 已连接
- `Disconnected`: 已断开
- `Error(message)`: 错误状态

### 11. 任务管理

**功能**:
- 异步任务状态追踪
- 定期健康检查任务
- 任务取消支持

## 数据库表结构

### 核心表
- `kafka_clusters`: 集群配置
- `cluster_connection_history`: 连接历史
- `api_keys`: API Key 存储
- `audit_logs`: 审计日志

### 用户权限
- `users`: 用户账户
- `roles`: 角色定义

### 监控告警
- `notification_configs`: 通知配置
- `consumer_lag_history`: Consumer Lag 历史

### 资源管理
- `resource_tags`: 资源标签
- `topic_templates`: Topic 模板
- `topic_metadata`: Topic 元数据
- `user_settings`: 用户全局设置

## API 端点总览

### 集群相关
```
GET    /api/clusters                     # 获取集群列表
POST   /api/clusters                     # 创建集群
GET    /api/clusters/:id                 # 获取集群详情
PUT    /api/clusters/:id                 # 更新集群
DELETE /api/clusters/:id                 # 删除集群

GET    /api/cluster-connections          # 获取连接状态
GET    /api/cluster-connections/:id/status
POST   /api/cluster-connections/:id/disconnect
POST   /api/cluster-connections/:id/reconnect
POST   /api/cluster-connections/:id/health-check
```

### Topic 管理
```
GET    /api/clusters/:cluster_id/topics
POST   /api/clusters/:cluster_id/topics
GET    /api/clusters/:cluster_id/topics/:name
PUT    /api/clusters/:cluster_id/topics/:name
DELETE /api/clusters/:cluster_id/topics/:name
GET    /api/clusters/:cluster_id/topics/:name/messages
POST   /api/clusters/:cluster_id/topics/refresh    # 刷新 Topic 列表
```

### Consumer Group
```
GET    /api/clusters/:cluster_id/consumer-groups
GET    /api/clusters/:cluster_id/consumer-groups/:name
DELETE /api/clusters/:cluster_id/consumer-groups/:name
GET    /api/clusters/:cluster_id/consumer-groups/:name/offsets
POST   /api/clusters/:cluster_id/consumer-groups/:name/offsets/reset
```

### Schema Registry
```
GET    /api/schema-registry/
GET    /api/schema-registry/:subject
GET    /api/schema-registry/:subject/:version
POST   /api/schema-registry/register
DELETE /api/schema-registry/:subject
DELETE /api/schema-registry/:subject/:version
POST   /api/schema-registry/compatibility
GET    /api/schema-registry/compatibility-level
PUT    /api/schema-registry/compatibility-level
```

### 通知管理
```
GET    /api/notifications
POST   /api/notifications
GET    /api/notifications/:id
POST   /api/notifications/:id/enable
POST   /api/notifications/:id/disable
GET    /api/alerts/history
```

### 用户管理
```
GET    /api/users
POST   /api/users
GET    /api/users/:id
PUT    /api/users/:id
PUT    /api/users/:id/password
GET    /api/roles
POST   /api/roles
GET    /api/roles/:id
PUT    /api/roles/:id
```

### 其他
```
GET    /api/alert-rules
POST   /api/alert-rules
PUT    /api/alert-rules/:id
DELETE /api/alert-rules/:id

GET    /api/audit-logs
GET    /api/health
GET    /api/ready
```

## 编译和测试

### 编译
```bash
cargo build
```

### 测试
```bash
cargo test
```

### 运行
```bash
cargo run
```

## 依赖项

主要依赖:
- `axum`: Web 框架
- `tokio`: 异步运行时
- `sqlx`: 异步数据库驱动 (SQLite)
- `rdkafka`: Kafka 客户端
- `serde`/`serde_json`: JSON 序列化
- `bcrypt`: 密码哈希
- `reqwest`: HTTP 客户端
- `thiserror`: 错误处理
- `chrono`: 时间处理
- `tower-http`: 中间件工具

## 配置

配置文件 `config.toml`:
```toml
[server]
host = "0.0.0.0"
port = 3000

[database]
path = "kafka_manager.db"
```

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `API_KEYS` | API Key 列表（逗号分隔） | - |
| `AUTH_ENABLED` | 是否启用认证 | false |
| `HEALTH_CHECK_INTERVAL_SECS` | 健康检查间隔 | 30 |

## 待实现功能

以下功能需要前端 UI 来完整使用：

1. **Kafka Connect 管理** - 连接器 CRUD 和状态监控
2. **MirrorMaker 管理** - 数据复制配置
3. **Broker 磁盘管理** - Broker 磁盘空间监控
4. **可视化 Dashboard** - 集群指标图表

## 总结

已实现的后端功能涵盖了 Kafka 集群管理的核心需求：

- ✅ 多集群管理
- ✅ Topic 全生命周期管理（支持自动同步到数据库）
- ✅ Consumer Group 管理和 Lag 监控
- ✅ Schema Registry 集成
- ✅ 多渠道告警通知
- ✅ RBAC 用户管理
- ✅ 审计日志
- ✅ 集群连接管理（断开/重连/健康检查）
- ✅ Topic 元数据管理（刷新同步、自动对齐）
- ✅ 标签管理
- ✅ Topic 模板
- ✅ 全局设置

所有 API 端点均已注册并可通过 HTTP 访问，代码编译通过，基础测试通过。

## 最近更新

### Topic 同步功能 (2026-02-26)

**新增 API**:
- `POST /api/clusters/:cluster_id/topics/refresh` - 刷新 Topic 列表

**自动同步**:
- 创建/更新/删除集群时自动同步 Topic 列表

**数据库**:
- 新增 `topic_metadata` 表存储 Topic 元数据

**详见**: [TOPIC_REFRESH_FEATURE.md](./TOPIC_REFRESH_FEATURE.md)
