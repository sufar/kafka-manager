# Kafka Manager API 文档

## 概述

Kafka Manager 是一个功能完整的 Kafka 集群管理工具，提供以下核心功能：

- **集群管理** - 多集群配置管理，连接状态监控
- **Topic 管理** - 完整的 CRUD 操作，元数据同步
- **Consumer Group 管理** - 消费进度查看，Offset 重置
- **消息管理** - 消息发送、浏览、搜索、导出
- **集群监控** - Broker 监控、指标采集
- **Schema Registry** - Confluent Schema Registry 集成
- **API 认证** - 基于 API Key 的认证
- **审计日志** - 自动记录所有 API 操作
- **标签管理** - 资源分类和筛选
- **Topic 模板** - 预定义配置模板
- **告警规则** - Consumer Lag、Produce Rate 告警
- **通知管理** - 多渠道通知配置
- **RBAC 用户管理** - 基于角色的权限控制
- **全局设置** - 用户偏好配置

## API 端点

### 认证

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/auth/keys | 创建 API Key |
| GET | /api/auth/keys | 获取 API Key 列表 |
| DELETE | /api/auth/keys/:id | 删除 API Key |

### 集群管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters | 获取所有集群 |
| POST | /api/clusters | 创建集群 |
| GET | /api/clusters/:id | 获取集群详情 |
| PUT | /api/clusters/:id | 更新集群 |
| DELETE | /api/clusters/:id | 删除集群 |
| POST | /api/clusters/:id/test | 测试集群连接 |
| GET | /api/clusters/:id/topics | 获取 Topics 列表 |
| GET | /api/clusters/:id/consumer-groups | 获取 Consumer Groups |
| GET | /api/clusters/:id/brokers | 获取 Brokers 信息 |
| GET | /api/clusters/:id/stats | 获取集群统计 |

**注意**: 创建、更新或删除集群时，会自动从 Kafka 集群同步 Topic 列表到数据库。

### 集群连接管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/cluster-connections | 获取所有集群连接状态 |
| GET | /api/cluster-connections/:id/status | 获取指定集群连接状态 |
| POST | /api/cluster-connections/:id/disconnect | 断开集群连接 |
| POST | /api/cluster-connections/:id/reconnect | 重连集群 |
| POST | /api/cluster-connections/:id/health-check | 执行健康检查 |

### Topic 管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/topics | 获取 Topics 列表 |
| POST | /api/clusters/:cluster_id/topics | 创建 Topic |
| GET | /api/clusters/:cluster_id/topics/:name | 获取 Topic 详情 |
| PUT | /api/clusters/:cluster_id/topics/:name | 更新 Topic 配置 |
| DELETE | /api/clusters/:cluster_id/topics/:name | 删除 Topic |
| GET | /api/clusters/:cluster_id/topics/:name/messages | 浏览消息 |
| GET | /api/clusters/:cluster_id/topics/:name/offsets | 查看分区 Offset 信息 |
| GET | /api/clusters/:cluster_id/topics/:name/throughput | 查看生产速率 |
| GET | /api/clusters/:cluster_id/topics/:name/consumer-lag | 查看 Consumer Group 积压 |
| GET | /api/clusters/:cluster_id/topics/:name/consumer-lag-history | 查看消费历史数据 |
| GET | /api/clusters/:cluster_id/topics/:name/tags | 获取 Topic 标签 |
| POST | /api/clusters/:cluster_id/topics/:name/tags | 添加 Topic 标签 |
| POST | /api/clusters/:cluster_id/topics/refresh | 刷新 Topic 列表（同步到数据库） |
| POST | /api/clusters/:cluster_id/topics/batch | 批量创建 Topic |
| DELETE | /api/clusters/:cluster_id/topics/batch | 批量删除 Topic |
| GET | /api/clusters/:cluster_id/topics/:name/messages/export | 导出 Topic 消息 |

**刷新 Topic 列表响应示例:**
```json
{
  "success": true,
  "added": ["new-topic-1", "new-topic-2"],
  "removed": ["deleted-topic"],
  "total": 15
}
```

**字段说明:**
- `success`: 同步是否成功
- `added`: 新增的 Topic（Kafka 存在但数据库不存在）
- `removed`: 删除的 Topic（数据库存在但 Kafka 不存在）
- `total`: 当前数据库中的 Topic 总数

### Consumer Group 管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/consumer-groups | 获取 Consumer Groups |
| GET | /api/clusters/:cluster_id/consumer-groups/:name | 获取详情 |
| DELETE | /api/clusters/:cluster_id/consumer-groups/:name | 删除 Consumer Group |
| GET | /api/clusters/:cluster_id/consumer-groups/:name/offsets | 查看 Offset 和 Lag |
| POST | /api/clusters/:cluster_id/consumer-groups/:name/offsets/reset | 重置 Offset |
| GET | /api/clusters/:cluster_id/consumer-groups/:name/throughput | 查看消费速率 |
| GET | /api/clusters/:cluster_id/consumer-groups/:name/tags | 获取 Consumer Group 标签 |

### Schema Registry 管理

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

**注册 Schema 请求示例:**
```json
{
  "cluster_id": "cluster-1",
  "subject": "test-topic-value",
  "schema": "{\"type\":\"record\",\"name\":\"Test\",\"fields\":[{\"name\":\"field1\",\"type\":\"string\"}]}",
  "schema_type": "AVRO"
}
```

### 通知管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/notifications | 获取通知配置列表 |
| POST | /api/notifications | 创建通知配置 |
| GET | /api/notifications/:id | 获取通知配置详情 |
| POST | /api/notifications/:id/enable | 启用通知 |
| POST | /api/notifications/:id/disable | 禁用通知 |
| GET | /api/alerts/history | 获取告警历史 |

### 告警规则管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/alert-rules | 获取告警规则列表 |
| POST | /api/alert-rules | 创建告警规则 |
| GET | /api/alert-rules/:id | 获取告警规则详情 |
| PUT | /api/alert-rules/:id | 更新告警规则 |
| DELETE | /api/alert-rules/:id | 删除告警规则 |

**告警规则类型**:
- `consumer_lag` - Consumer Group 积压告警
- `produce_rate` - Topic 生产速率告警
- `consumer_rate` - Consumer Group 消费速率告警

**比较操作符**:
- `gt` - 大于
- `lt` - 小于
- `gte` - 大于等于
- `lte` - 小于等于
- `eq` - 等于

**告警级别**:
- `warning` - 警告级别
- `critical` - 严重级别

**创建通知配置请求示例:**
```json
{
  "name": "生产环境告警",
  "config_type": "DINGTALK",
  "dingtalk_webhook": "https://oapi.dingtalk.com/robot/send?access_token=xxx",
  "dingtalk_secret": "xxx"
}
```

### 用户与权限管理

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

**默认角色**:
| 角色 | 描述 | 权限 |
|------|------|------|
| admin | 系统管理员 | 所有权限 (*) |
| operator | 运维人员 | cluster:*, topic:*, consumer_group:*, message:* |
| viewer | 只读用户 | cluster:read, topic:read, consumer_group:read, message:read |

**创建用户请求示例:**
```json
{
  "username": "newuser",
  "password": "SecurePassword123",
  "email": "user@example.com",
  "role_id": 1
}
```

**创建告警规则请求示例:**
```json
{
  "cluster_id": "cluster-1",
  "name": "Consumer Lag 告警",
  "rule_type": "consumer_lag",
  "topic": "test-topic",
  "consumer_group": "test-group",
  "threshold": 1000,
  "comparison": "gt",
  "duration_seconds": 300,
  "severity": "critical"
}
```

### 审计日志

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/audit-logs | 获取审计日志列表 |
| DELETE | /api/audit-logs | 清理旧审计日志 |

**查询参数**:
- `limit` - 每页数量
- `offset` - 偏移量
- `action` - 按操作类型过滤
- `cluster_id` - 按集群 ID 过滤
- `status` - 按状态码过滤

### 标签管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/clusters/:cluster_id/tags/keys | 列出所有标签键 |
| GET | /api/clusters/:cluster_id/tags/:key/values | 列出标签值 |
| GET | /api/clusters/:cluster_id/tags/filter | 按标签过滤资源 |
| POST | /api/clusters/:cluster_id/topics/:name/tags | 为 Topic 添加标签 |
| PUT | /api/clusters/:cluster_id/topics/:name/tags/:key | 更新 Topic 标签 |
| DELETE | /api/clusters/:cluster_id/topics/:name/tags/:key | 删除 Topic 标签 |
| DELETE | /api/clusters/:cluster_id/topics/:name/tags/all | 删除所有标签 |

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

### 健康检查

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/health | 健康检查 |
| GET | /api/ready | 就绪检查 |

## 错误响应

所有 API 错误响应格式统一为：

```json
{
  "error": "错误消息"
}
```

常见 HTTP 状态码:
- `200 OK` - 请求成功
- `400 Bad Request` - 请求参数错误
- `401 Unauthorized` - 认证失败
- `404 Not Found` - 资源不存在
- `500 Internal Server Error` - 服务器内部错误

## 安全认证

使用 API Key 进行认证，在请求头中携带：

```
X-API-Key: your-api-key-here
```

## 默认角色

系统预置三个角色：

| 角色 | 描述 | 权限 |
|------|------|------|
| admin | 系统管理员 | 所有权限 (*) |
| operator | 运维人员 | cluster:*, topic:*, consumer_group:*, message:* |
| viewer | 只读用户 | cluster:read, topic:read, consumer_group:read, message:read |
