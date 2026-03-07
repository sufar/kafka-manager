# Kafka Manager 实现总结

## 项目概述

Kafka Manager 是一个功能齐全的 Kafka 集群管理工具，采用 Rust + Vue 3 + Tauri 2 架构，提供 Web API 和跨平台桌面应用。

## 已完成功能

### 1. 核心架构

- **Web 框架**: Axum 0.7 (Tokio ecosystem)
- **数据库**: SQLite with SQLx 0.8
- **Kafka 客户端**: rdkafka 0.39
- **连接池**: 自定义 Kafka Consumer/Producer 连接池 (deadpool)
- **缓存**: Moka 缓存库
- **认证**: API Key + RBAC 权限系统
- **中间件**: 认证、审计日志、性能监控、速率限制
- **桌面应用**: Tauri 2 跨平台支持

### 2. 数据库层 (`src/db/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 集群管理 | `cluster.rs` | 集群 CRUD、批量操作 |
| 集群连接 | `cluster_connection.rs` | 连接状态管理、断开/重连、健康检查 |
| 用户管理 | `user.rs` | 用户和角色管理，bcrypt 密码哈希，权限控制 |
| 通知 | `notification.rs` | 通知配置管理，多渠道支持 |
| 审计日志 | `audit_log.rs` | API 请求审计，查询和清理 |
| 资源标签 | `tag.rs` | Topic 和 Consumer Group 标签管理 |
| Topic 管理 | `topic.rs` | Topic 元数据同步和存储 |
| Topic 模板 | `topic_template.rs` | Topic 创建模板，预定义配置 |
| API Key | `api_key.rs` | API Key 存储、验证和脱敏 |
| 设置 | `settings.rs` | 全局用户设置 |
| 主模块 | `mod.rs` | 数据库初始化、表创建、迁移 |

### 3. Kafka 模块 (`src/kafka/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| Admin | `admin.rs` | Kafka Admin 客户端封装，Topic CRUD |
| Consumer | `consumer.rs` | Kafka Consumer 封装，流式查询，水印获取 |
| Producer | `producer.rs` | Kafka Producer 封装，同步/异步发送 |
| Offset | `offset.rs` | Offset 管理，水印查询 |
| Throughput | `throughput.rs` | 吞吐量统计，消费速率计算 |
| Schema | `schema.rs` |本地 Schema 管理 |
| Schema Registry | `schema_registry.rs` | Schema Registry HTTP 客户端 |
| Transaction | `transaction.rs` | 事务管理（Kafka 2.8+） |
| Import/Export | `import_export.rs` | 数据导入导出工具 |

### 4. 连接池层 (`src/pool/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 连接池管理 | `mod.rs` | KafkaClients 连接池管理，多集群支持 |
| Consumer 池 | `kafka_consumer.rs` | Consumer 连接池，按需创建 |
| Producer 池 | `kafka_producer.rs` | Producer 连接池，批量优化 |

### 5. 中间件 (`src/middleware/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 认证 | `auth.rs` | API Key 认证，路径白名单，RBAC 权限检查 |
| 审计 | `audit.rs` | 自动审计日志记录，路径解析，资源识别 |
| 性能 | `performance.rs` | 请求性能监控，慢查询检测 |
| 速率限制 | `rate_limit.rs` | 基于 governor 的速率限制 |
| 主模块 | `mod.rs` | 中间件注册和配置 |

### 6. 路由层 (`src/routes/`)

| 模块 | 路径前缀 | 功能 |
|------|---------|------|
| `cluster.rs` | `/api/clusters` | 集群 CRUD、测试连接 |
| `cluster_connection.rs` | `/api/cluster-connections` | 集群连接管理（断开/重连/健康检查） |
| `cluster_stats.rs` | `/api/clusters/:cluster_id/stats` | 集群统计 Dashboard |
| `cluster_monitor.rs` | `/api/clusters/:cluster_id` | 集群监控（Broker、Metrics） |
| `topic.rs` | `/api/clusters/:cluster_id/topics` | Topic 管理（**支持刷新同步**） |
| `consumer_group.rs` | `/api/clusters/:cluster_id/consumer-groups` | Consumer Group 管理 |
| `message.rs` | `/api/clusters/:cluster_id/topics` | 消息浏览、搜索、导出、发送 |
| `schema.rs` | `/api/schema-registry` | Schema Registry 管理 |
| `notification.rs` | `/api/notifications` | 通知配置管理 |
| `user.rs` | `/api/users`, `/api/roles` | 用户和角色管理 |
| `auth.rs` | `/api/auth/keys` | API Key 认证管理 |
| `audit_log.rs` | `/api/audit-logs` | 审计日志查询和清理 |
| `tag.rs` | `/api/clusters/:cluster_id` | 资源标签管理 |
| `topic_template.rs` | `/api` | Topic 模板管理 |
| `settings.rs` | `/api/settings` | 全局设置 |
| `health.rs` | `/api` | 健康检查 |
| `unified.rs` | `/api` | 统一 API 路由（历史兼容） |

### 7. 后台任务 (`src/task/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 任务管理 | `mod.rs` | 异步任务状态追踪，任务取消 |
| 健康检查 | `health_check.rs` | 定期集群健康检查 |

### 8. 缓存模块 (`src/cache/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 缓存管理 | `mod.rs` | Moka 缓存配置和管理 |

### 9. 数据模型 (`src/models/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 模型定义 | `mod.rs` | 请求/响应数据模型 |

### 10. 错误处理 (`src/`)

| 模块 | 文件 | 功能 |
|------|------|------|
| 错误定义 | `error.rs` | 统一错误类型和处理 |
| 测试模块 | `tests/` | 单元测试和模型测试 |

## 数据库表结构

### 核心表

| 表名 | 字段 | 描述 |
|------|------|------|
| `kafka_clusters` | id, name, brokers, request_timeout_ms, operation_timeout_ms, created_at, updated_at | 集群配置 |
| `cluster_connection_history` | id, cluster_name, status, checked_at | 连接历史 |
| `api_keys` | id, key_hash, key_prefix, name, created_at, expires_at, is_active | API Key 存储 |
| `audit_logs` | id, timestamp, method, path, cluster_id, resource, action, api_key, status, duration_ms, client_ip | 审计日志 |

### 用户权限

| 表名 | 字段 | 描述 |
|------|------|------|
| `users` | id, username, password_hash, role_id, created_at, updated_at | 用户账户 |
| `roles` | id, name, description, permissions, created_at, updated_at | 角色定义 |

### 监控告警

| 表名 | 字段 | 描述 |
|------|------|------|
| `notification_configs` | id, name, config_type, webhook_url, email_recipients, dingtalk_webhook, enabled, created_at, updated_at | 通知配置 |
| `alert_rules` | id, cluster_id, name, rule_type, topic, consumer_group, threshold, comparison, duration_seconds, severity, enabled, notification_config | 告警规则 |
| `alert_history` | id, rule_id, cluster_id, alert_type, alert_message, alert_value, threshold, severity, notified, notified_at | 告警历史 |
| `consumer_lag_history` | id, cluster_id, topic, consumer_group, total_lag, recorded_at | Consumer Lag 历史 |

### 资源管理

| 表名 | 字段 | 描述 |
|------|------|------|
| `resource_tags` | id, cluster_id, resource_type, resource_name, tag_key, tag_value, created_at, updated_at | 资源标签 |
| `topic_templates` | id, name, description, num_partitions, replication_factor, config, created_at, updated_at | Topic 模板 |
| `topic_metadata` | id, cluster_id, topic_name, partition_count, created_at, updated_at | Topic 元数据缓存 |
| `user_settings` | id, user_id, setting_key, setting_value, created_at, updated_at | 用户设置 |

## API 端点总览

### 统一 API 风格

本项目采用**统一的 POST API**设计：

- **请求 URL**: `POST /api`
- **Method Header**: `X-API-Method: <method_name>`
- **请求 Body**: JSON 格式参数

**请求示例**:
```bash
curl -X POST http://localhost:3000/api \
  -H "X-API-Method: cluster.list" \
  -H "Content-Type: application/json"
```

### API Method 列表

#### 集群管理
| Method | 描述 | 参数 |
|--------|------|------|
| `cluster.list` | 获取集群列表 | - |
| `cluster.get` | 获取集群详情 | `cluster_id` |
| `cluster.create` | 创建集群 | `name`, `brokers`, `request_timeout_ms`, `operation_timeout_ms` |
| `cluster.update` | 更新集群 | `cluster_id`, 其他可选 |
| `cluster.delete` | 删除集群 | `cluster_id` |
| `cluster.test` | 测试集群连接 | `cluster_id` |

#### 集群连接管理
| Method | 描述 | 参数 |
|--------|------|------|
| `connection.list` | 获取所有连接状态 | - |
| `connection.get` | 获取单个连接状态 | `cluster_id` |
| `connection.disconnect` | 断开连接 | `cluster_id` |
| `connection.reconnect` | 重连 | `cluster_id` |
| `connection.health_check` | 健康检查 | `cluster_id` |
| `connection.metrics` | 获取连接指标 | `cluster_id` |
| `connection.history` | 获取连接历史 | `cluster_id`, `limit?`, `offset?` |
| `connection.stats` | 获取连接统计 | `cluster_id` |

#### Topic 管理
| Method | 描述 | 参数 |
|--------|------|------|
| `topic.list` | 获取 Topic 列表 | `cluster_id` |
| `topic.get` | 获取 Topic 详情 | `cluster_id`, `name` |
| `topic.create` | 创建 Topic | `cluster_id`, `name`, `num_partitions`, `replication_factor` |
| `topic.delete` | 删除 Topic | `cluster_id`, `name` |
| `topic.batch_create` | 批量创建 Topic | `cluster_id`, `topics[]` |
| `topic.batch_delete` | 批量删除 Topic | `cluster_id`, `topics[]` |
| `topic.offsets` | 获取 Topic Offset | `cluster_id`, `name` |
| `topic.config_get` | 获取 Topic 配置 | `cluster_id`, `name` |
| `topic.config_alter` | 修改 Topic 配置 | `cluster_id`, `name`, `config` |
| `topic.partitions_add` | 增加分区 | `cluster_id`, `name`, `new_partitions` |
| `topic.throughput` | 获取吞吐量 | `cluster_id`, `name` |
| `topic.refresh` | 刷新 Topic 列表 | `cluster_id` |
| `topic.saved` | 获取已保存 Topic | `cluster_id` |
| `topic.search` | 搜索 Topic | `cluster_id`, `search` |
| `topic.count` | 获取 Topic 数量 | `cluster_id` |

#### Consumer Group 管理
| Method | 描述 | 参数 |
|--------|------|------|
| `consumer_group.list` | 获取 Consumer Group 列表 | `cluster_id` |
| `consumer_group.get` | 获取 Consumer Group 详情 | `cluster_id`, `name` |
| `consumer_group.delete` | 删除 Consumer Group | `cluster_id`, `name` |
| `consumer_group.offsets` | 获取 Offset | `cluster_id`, `name` |
| `consumer_group.offsets_reset` | 重置 Offset | `cluster_id`, `name`, `topic`, `offset` |
| `consumer_group.throughput` | 获取吞吐量 | `cluster_id`, `name`, `topic` |
| `consumer_group.batch_delete` | 批量删除 | `cluster_id`, `group_names[]` |
| `consumer_group.consumer_offsets` | 获取消费进度 | `cluster_id`, `group_name` |
| `consumer_lag.get` | 获取消费积压 | `cluster_id`, `topic` |
| `consumer_lag.history` | 获取积压历史 | `cluster_id`, `topic` |

#### 消息管理
| Method | 描述 | 参数 |
|--------|------|------|
| `message.list` | 获取消息列表 | `cluster_id`, `topic`, `partition?`, `max_messages` |
| `message.send` | 发送消息 | `cluster_id`, `topic`, `value`, `key?`, `partition?` |
| `message.export` | 导出消息 | `cluster_id`, `topic`, `format` |

#### 监控和统计
| Method | 描述 | 参数 |
|--------|------|------|
| `monitor.stats` | 获取集群统计 | `cluster_id` |
| `monitor.info` | 获取集群信息 | `cluster_id` |
| `monitor.metrics` | 获取集群指标 | `cluster_id` |
| `monitor.brokers` | 获取 Broker 列表 | `cluster_id` |
| `monitor.broker_get` | 获取 Broker 详情 | `cluster_id`, `broker_id` |

#### 用户管理
| Method | 描述 | 参数 |
|--------|------|------|
| `user.list` | 用户列表 | - |
| `user.get` | 用户详情 | `id` |
| `user.create` | 创建用户 | `username`, `password`, `role_id` |
| `user.update` | 更新用户 | `id`, 其他可选 |
| `user.password_update` | 修改密码 | `id`, `password` |

#### 角色管理
| Method | 描述 | 参数 |
|--------|------|------|
| `role.list` | 角色列表 | - |
| `role.get` | 角色详情 | `id` |
| `role.create` | 创建角色 | `name`, `description`, `permissions` |
| `role.update` | 更新角色 | `id`, 其他可选 |

#### 通知管理
| Method | 描述 | 参数 |
|--------|------|------|
| `notification.list` | 通知配置列表 | - |
| `notification.get` | 通知配置详情 | `id` |
| `notification.create` | 创建通知配置 | `name`, `config_type`, `webhook_url?` |
| `notification.delete` | 删除通知配置 | `id` |
| `notification.enable` | 启用通知配置 | `id` |
| `notification.disable` | 禁用通知配置 | `id` |
| `alert_history.list` | 告警历史 | `cluster_id?`, `severity?` |

#### Schema Registry
| Method | 描述 | 参数 |
|--------|------|------|
| `schema.subjects` | 获取主题列表 | - |
| `schema.versions` | 获取版本列表 | `subject` |
| `schema.get` | 获取 Schema | `subject`, `version?` |
| `schema.register` | 注册 Schema | `subject`, `schema` |
| `schema.delete` | 删除主题 | `subject` |
| `schema.version_delete` | 删除版本 | `subject`, `version` |
| `schema.compatibility_level` | 兼容性级别 | `subject?` |

#### 设置管理
| Method | 描述 | 参数 |
|--------|------|------|
| `settings.get` | 获取设置 | `key?` |
| `settings.update` | 更新设置 | `key`, `value` |

#### Topic 模板
| Method | 描述 | 参数 |
|--------|------|------|
| `template.list` | 模板列表 | - |
| `template.get` | 模板详情 | `id` |
| `template.create` | 创建模板 | `name`, `description`, `num_partitions`, `replication_factor` |
| `template.update` | 更新模板 | `id`, 其他可选 |
| `template.delete` | 删除模板 | `id` |
| `template.presets` | 预定义模板 | - |
| `template.create_topic` | 使用模板创建 Topic | `cluster_id`, `topic_name`, `template_id/template_name` |

#### 资源标签
| Method | 描述 | 参数 |
|--------|------|------|
| `tag.list` | 标签列表 | `cluster_id`, `resource_type`, `resource_name` |
| `tag.create` | 创建标签 | `cluster_id`, `resource_type`, `resource_name`, `key`, `value` |
| `tag.delete` | 删除标签 | `cluster_id`, `resource_type`, `resource_name`, `key` |
| `tag.topics` | 获取 Topic 标签 | `cluster_id`, `resource_type?`, `resource_name?` |
| `tag.keys` | 标签键列表 | `cluster_id`, `resource_type?` |
| `tag.values` | 标签值列表 | `cluster_id`, `key`, `resource_type?` |
| `tag.filter` | 按标签过滤 | `cluster_id`, `resource_type`, `tag_key`, `tag_value?` |
| `tag.batch_update` | 批量更新标签 | `cluster_id`, `resource_type`, `resource_name`, `tags` |

#### 审计日志
| Method | 描述 | 参数 |
|--------|------|------|
| `audit_log.list` | 审计日志列表 | `limit?`, `offset?`, `action?`, `cluster_id?` |

#### 认证管理
| Method | 描述 | 参数 |
|--------|------|------|
| `auth.api_keys` | API Key 列表 | - |
| `auth.api_key_create` | 创建 API Key | `name?`, `expires_in_days?` |
| `auth.api_key_revoke` | 撤销 API Key | `id` |

#### 健康检查
| Method | 描述 | 参数 |
|--------|------|------|
| `health` | 健康检查 | - |

## 认证和授权

### 认证中间件 (`src/middleware/auth.rs`)

- **API Key 验证**: 从 `X-API-Key` 请求头验证
- **路径白名单**: `/api/health`, `/api/ready` 跳过认证
- **环境变量**: `API_KEYS`, `AUTH_ENABLED`

### RBAC 权限系统

- **权限格式**: `resource:action` (如 `topic:read`, `topic:*`)
- **默认角色**:
  - `admin`: 所有权限 (`*`)
  - `operator`: 运维权限
  - `viewer`: 只读权限

## 告警通知系统

### 支持的通知渠道

- **Webhook**: 通用 HTTP 回调
- **钉钉机器人**: 支持 HMAC 签名验证
- **企业微信机器人**
- **Slack Webhook**
- **邮件**: SMTP 发送

### 告警类型

- **Consumer Lag**: 消费延迟告警
- **Produce Rate**: 生产速率告警
- **Consumer Rate**: 消费速率告警

### 告警级别

- `warning`: 警告级别，需要关注
- `critical`: 严重级别，需要立即处理

## Topic 同步功能

### 自动同步

- 创建/更新/删除集群时自动同步 Topic 列表到数据库
- 支持手动刷新：`POST /api/clusters/:cluster_id/topics/refresh`

### 数据库存储

- `topic_metadata` 表存储 Topic 元数据
- 包含分区数量、创建时间等信息

## 编译和测试

### 编译
```bash
cargo build --release
```

### 测试
```bash
cargo test
```

### 代码检查
```bash
cargo clippy
cargo fmt
```

### 运行
```bash
cargo run
```

## 依赖项

### 主要依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `axum` | 0.7 | Web 框架 |
| `tokio` | 1 | 异步运行时 |
| `sqlx` | 0.8 | 异步数据库驱动 (SQLite) |
| `rdkafka` | 0.39 | Kafka 客户端 |
| `serde`/`serde_json` | 1 | JSON 序列化 |
| `bcrypt` | 0.15 | 密码哈希 |
| `reqwest` | 0.11 | HTTP 客户端 |
| `thiserror` | 1 | 错误处理 |
| `chrono` | 0.4 | 时间处理 |
| `tower-http` | 0.5 | 中间件工具 |
| `deadpool` | 0.12 | 连接池 |
| `moka` | 0.12 | 缓存 |
| `governor` | 0.6 | 速率限制 |
| `tracing` | 0.1 | 日志 |

### 前端依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `vue` | 3 | 前端框架 |
| `typescript` | - | 类型系统 |
| `tailwindcss` | 4 | CSS 框架 |
| `daisyui` | 5 | 组件库 |
| `pinia` | - | 状态管理 |
| `vue-router` | - | 路由 |
| `tauri` | 2 | 桌面应用 |

## 配置

### 配置文件 `config.toml`

```toml
[server]
host = "0.0.0.0"
port = 3000

[database]
path = "kafka_manager.db"

[pool]
max_connections = 10
min_idle = 2
connection_timeout_sec = 30
```

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `API_KEYS` | API Key 列表（逗号分隔） | - |
| `AUTH_ENABLED` | 是否启用认证 | `false` |
| `HEALTH_CHECK_INTERVAL_SECS` | 健康检查间隔 | `30` |

## 最近更新

### 代码质量改进 (2026-03-07)

- 移除所有不必要的 `console.log` 调试语句
- 修复 Rust 代码中的 `unwrap()` 调用，改用适当的错误处理
- 将 `eprintln!` 改为 `tracing::info!` 日志

### Topic 同步功能 (2026-02-26)

- 新增 API: `POST /api/clusters/:cluster_id/topics/refresh` - 刷新 Topic 列表
- 自动同步：创建/更新/删除集群时自动同步 Topic 列表
- 新增 `topic_metadata` 表存储 Topic 元数据

### Tauri 2 迁移

- 从纯 Web 应用迁移到 Tauri 2 跨平台桌面应用
- 支持 Windows、macOS、Linux
- 保持 Web 后端独立运行

## 总结

已实现的完整功能：

- ✅ 多集群管理（CRUD + 连接管理）
- ✅ Topic 全生命周期管理（支持自动同步到数据库）
- ✅ Consumer Group 管理和 Lag 监控
- ✅ 消息浏览、搜索、导出、发送
- ✅ Schema Registry 集成
- ✅ 多渠道告警通知
- ✅ RBAC 用户管理
- ✅ 审计日志
- ✅ 集群连接管理（断开/重连/健康检查）
- ✅ Topic 元数据管理（刷新同步、自动对齐）
- ✅ 标签管理
- ✅ Topic 模板
- ✅ 全局设置
- ✅ 跨平台桌面应用（Tauri 2）

所有 API 端点均已注册并可通过 HTTP 访问，代码编译通过，基础测试通过。
