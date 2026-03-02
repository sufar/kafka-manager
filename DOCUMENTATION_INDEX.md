# Kafka Manager 文档索引

## 核心文档

| 文档 | 说明 |
|------|------|
| [README.md](./README.md) | 项目概述、快速开始、功能列表 |
| [API.md](./API.md) | 完整的 API 端点文档 |
| [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) | 技术实现总结 |
| [docs/API_EXAMPLES.md](./docs/API_EXAMPLES.md) | API 使用示例 |

## 功能文档

| 文档 | 说明 |
|------|------|
| [TOPIC_REFRESH_FEATURE.md](./TOPIC_REFRESH_FEATURE.md) | Topic 刷新同步功能详解 |
| [docs/OPTIMIZATIONS.md](./docs/OPTIMIZATIONS.md) | 性能优化文档 |

## 快速导航

### API 端点

#### 集群管理
- `GET /api/clusters` - 列出所有集群
- `POST /api/clusters` - 创建集群
- `PUT /api/clusters/:id` - 更新集群
- `DELETE /api/clusters/:id` - 删除集群

#### Topic 管理
- `GET /api/clusters/:cluster_id/topics` - 列出 Topic
- `POST /api/clusters/:cluster_id/topics` - 创建 Topic
- `POST /api/clusters/:cluster_id/topics/refresh` - 刷新 Topic 列表
- `GET /api/clusters/:cluster_id/topics/:name` - Topic 详情
- `DELETE /api/clusters/:cluster_id/topics/:name` - 删除 Topic
- `GET /api/clusters/:cluster_id/topics/:name/throughput` - 生产速率
- `GET /api/clusters/:cluster_id/topics/:name/consumer-lag` - Consumer 积压

#### Consumer Group
- `GET /api/clusters/:cluster_id/consumer-groups` - 列出 Consumer Groups
- `GET /api/clusters/:cluster_id/consumer-groups/:name/offsets` - 消费进度和 Lag
- `POST /api/clusters/:cluster_id/consumer-groups/:name/offsets/reset` - 重置 Offset

#### 集群监控
- `GET /api/clusters/:cluster_id/info` - 集群信息
- `GET /api/clusters/:cluster_id/metrics` - 集群指标
- `GET /api/clusters/:cluster_id/brokers/:broker_id` - Broker 详情

#### 认证与权限
- `POST /api/auth/keys` - 创建 API Key
- `GET /api/auth/keys` - API Key 列表
- `GET /api/users` - 用户列表
- `GET /api/roles` - 角色列表

#### 告警与通知
- `GET /api/alert-rules` - 告警规则列表
- `POST /api/alert-rules` - 创建告警规则
- `GET /api/notifications` - 通知配置列表
- `GET /api/alerts/history` - 告警历史

### 数据库表

| 表名 | 描述 |
|------|------|
| `kafka_clusters` | 集群配置 |
| `topic_metadata` | Topic 元数据 |
| `topic_templates` | Topic 配置模板 |
| `cluster_connection_history` | 集群连接历史 |
| `api_keys` | API Key 存储 |
| `audit_logs` | 审计日志 |
| `users`, `roles` | 用户和角色 |
| `resource_tags` | 资源标签 |
| `notification_configs` | 通知配置 |
| `consumer_lag_history` | Consumer Lag 历史 |
| `user_settings` | 用户全局设置 |

### 配置文件

```toml
[server]
host = "127.0.0.1"
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

## 更新日志

### 2026-02-26
- 新增 Topic 元数据同步功能
- 新增 `topic_metadata` 数据库表
- 新增 `POST /api/clusters/:cluster_id/topics/refresh` API
- 集群变更时自动同步 Topic 列表

### 之前版本
- 完整的 Kafka 集群管理功能
- 多集群支持
- Schema Registry 集成
- RBAC 用户管理
- 告警通知系统
- 审计日志
- 标签管理
- Topic 模板
