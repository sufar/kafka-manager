# Kafka Manager API 文档

## 概述

Kafka Manager 采用**统一的 POST API**设计，所有 API 请求都通过 `POST /api` 发送，操作类型通过 `X-API-Method` HTTP Header 指定，请求参数放在 JSON Body 中。

## 请求格式

```http
POST /api HTTP/1.1
Host: localhost:9732
Content-Type: application/json
X-API-Method: {method_name}

{
  "param1": "value1",
  "param2": "value2"
}
```

## 响应格式

### 成功响应
```json
{
  "success": true,
  "data": { ... }
}
```

### 错误响应
```json
{
  "success": false,
  "error": "错误信息"
}
```

### 错误类型
```json
{
  "success": false,
  "error": "Bad Request",
  "message": "具体错误描述"
}
```

---

## API Method 列表

### 健康检查

| Method | 描述 | 参数 |
|--------|------|------|
| `health` | 健康检查 | 无 |

---

### 集群管理

| Method | 描述 | 参数 |
|--------|------|------|
| `cluster.list` | 获取集群列表 | 无 |
| `cluster.get` | 获取集群详情 | `id: number` |
| `cluster.create` | 创建集群 | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.update` | 更新集群 | `id: number, name?: string, brokers?: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.delete` | 删除集群 | `id: number` |
| `cluster.test` | 测试集群连接 | `id: number` |

> **注意**: `cluster.delete` 会删除集群及其所有 Topic 的**数据库元数据**，不会删除 Kafka 集群中的实际数据。

---

### Topic 主题管理

| Method | 描述 | 参数 |
|--------|------|------|
| `topic.list` | 获取 Topic 列表 | `cluster_id: string` |
| `topic.get` | 获取 Topic 详情 | `cluster_id: string, name: string` |
| `topic.create` | 创建 Topic | `cluster_id: string, name: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `topic.delete` | 删除 Topic | `cluster_id: string, name: string` |
| `topic.delete_all` | 删除集群下所有 Topic | `cluster_id: string` |
| `topic.batch_create` | 批量创建 Topic | `cluster_id: string, topics: array, continue_on_error?: boolean` |
| `topic.batch_delete` | 批量删除 Topic | `cluster_id: string, topics: string[], continue_on_error?: boolean` |
| `topic.offsets` | 获取 Topic 偏移量 | `cluster_id: string, name: string` |
| `topic.config_get` | 获取 Topic 配置 | `cluster_id: string, name: string` |
| `topic.config_alter` | 修改 Topic 配置 | `cluster_id: string, name: string, config: object` |
| `topic.partitions.add` | 增加分区 | `cluster_id: string, name: string, new_partitions: number` |
| `topic.throughput` | 获取 Topic 吞吐量 | `cluster_id: string, name: string` |
| `topic.refresh` | 刷新 Topic 列表 | `cluster_id: string` |
| `topic.saved` | 获取已保存的 Topic | `cluster_id: string` |
| `topic.search` | 搜索所有集群的 Topic | `search?: string` |
| `topic.count` | 获取 Topic 数量 | `cluster_id: string` |

---

### Consumer Group 消费者组管理

| Method | 描述 | 参数 |
|--------|------|------|
| `consumer_group.list` | 获取消费者组列表 | `cluster_id: string` |
| `consumer_group.get` | 获取消费者组详情 | `cluster_id: string, name: string` |
| `consumer_group.delete` | 删除消费者组 | `cluster_id: string, name: string` |
| `consumer_group.offsets` | 获取消费者组偏移量 | `cluster_id: string, name: string, topic?: string` |
| `consumer_group.offsets_reset` | 重置消费者组偏移量 | `cluster_id: string, name: string, topic: string, offset: { type: string, value?: number }, partition?: number` |
| `consumer_group.throughput` | 获取消费者组吞吐量 | `cluster_id: string, name: string, topic: string` |
| `consumer_group.batch_delete` | 批量删除消费者组 | `cluster_id: string, group_names: string[], continue_on_error?: boolean` |
| `consumer_group.consumer_offsets` | 获取所有消费者组的偏移量 | `cluster_id: string` |
| `consumer_lag.get` | 获取 Topic 消费积压 | `cluster_id: string, topic: string` |
| `consumer_lag.history` | 获取消费积压历史 | `cluster_id: string, topic: string, start_time?: number, end_time?: number` |

---

### Message 消息管理

| Method | 描述 | 参数 |
|--------|------|------|
| `message.list` | 获取消息列表 | `cluster_id: string, topic: string, partition?: number, offset?: number, max_messages?: number, order_by?: string, sort?: string, limit?: number, search?: string, search_in?: string, format?: string, decode?: string` |
| `message.send` | 发送消息 | `cluster_id: string, topic: string, value: string, key?: string, partition?: number, headers?: object` |

> **注意**: 消息导出使用独立端点 `GET /api/clusters/:cluster_id/topics/:topic/messages/export`

---

### 集群连接管理

| Method | 描述 | 参数 |
|--------|------|------|
| `connection.list` | 获取所有连接状态 | 无 |
| `connection.get` | 获取连接状态 | `cluster_id: string` |
| `connection.disconnect` | 断开连接 | `cluster_id: string` |
| `connection.reconnect` | 重新连接 | `cluster_id: string` |
| `connection.health_check` | 健康检查 | `cluster_id: string` |
| `connection.metrics` | 获取连接指标 | `cluster_id: string` |
| `connection.history` | 获取连接历史 | `cluster_id: string, limit?: number, offset?: number` |
| `connection.stats` | 获取连接统计 | `cluster_id: string` |
| `connection.batch_disconnect` | 批量断开连接 | `cluster_names: string[]` |
| `connection.batch_reconnect` | 批量重新连接 | `cluster_names: string[]` |

---

### 集群监控

| Method | 描述 | 参数 |
|--------|------|------|
| `monitor.stats` | 获取集群统计 | `cluster_id: string` |
| `monitor.info` | 获取集群信息 | `cluster_id: string` |
| `monitor.metrics` | 获取集群指标 | `cluster_id: string` |
| `monitor.brokers` | 获取 Broker 列表 | `cluster_id: string` |
| `monitor.broker_get` | 获取 Broker 详情 | `cluster_id: string, broker_id: number` |

---

### 用户管理

| Method | 描述 | 参数 |
|--------|------|------|
| `user.list` | 获取用户列表 | 无 |
| `user.get` | 获取用户详情 | `id: number` |
| `user.create` | 创建用户 | `username: string, password: string, email?: string, role_id?: number` |
| `user.update` | 更新用户 | `id: number, email?: string, role_id?: number, is_active?: boolean` |
| `user.password_update` | 更新密码 | `id: number, old_password: string, new_password: string` |

---

### 角色管理

| Method | 描述 | 参数 |
|--------|------|------|
| `role.list` | 获取角色列表 | 无 |
| `role.get` | 获取角色详情 | `id: number` |
| `role.create` | 创建角色 | `name: string, description?: string, permissions: string[]` |
| `role.update` | 更新角色 | `id: number, name?: string, description?: string, permissions?: string[]` |

---

### 通知管理

| Method | 描述 | 参数 |
|--------|------|------|
| `notification.list` | 获取通知配置列表 | 无 |
| `notification.get` | 获取通知配置详情 | `id: number` |
| `notification.create` | 创建通知配置 | `name: string, config_type: string, webhook_url?: string, email_recipients?: string[], dingtalk_webhook?: string, slack_webhook?: string, wechat_webhook?: string, enabled?: boolean` |
| `notification.delete` | 删除通知配置 | `id: number` |
| `notification.enable` | 启用通知 | `id: number` |
| `notification.disable` | 禁用通知 | `id: number` |
| `alert_history.list` | 获取告警历史 | `cluster_id?: string, severity?: string, notified?: boolean, limit?: number, offset?: number` |

---

### Schema Registry

| Method | 描述 | 参数 |
|--------|------|------|
| `schema.subjects` | 获取 Subject 列表 | `cluster_id: string, schema_registry_url: string` |
| `schema.versions` | 获取 Subject 版本列表 | `cluster_id: string, subject: string, schema_registry_url: string` |
| `schema.get` | 获取 Schema 详情 | `cluster_id: string, subject: string, version?: string, schema_registry_url: string` |
| `schema.register` | 注册 Schema | `cluster_id: string, subject: string, schema: object, schema_type?: string` |
| `schema.delete` | 删除 Schema | `cluster_id: string, subject: string, schema_registry_url: string` |
| `schema.version_delete` | 删除 Schema 版本 | `cluster_id: string, subject: string, version: string, schema_registry_url: string` |
| `schema.compatibility_level` | 获取兼容性级别 | `cluster_id: string, schema_registry_url: string` |

---

### 设置管理

| Method | 描述 | 参数 |
|--------|------|------|
| `settings.get` | 获取设置 | `keys?: string[]` |
| `settings.update` | 更新设置 | `key: string, value: string` |

---

### Topic 模板

| Method | 描述 | 参数 |
|--------|------|------|
| `template.list` | 模板列表 | 无 |
| `template.get` | 模板详情 | `id: number` |
| `template.create` | 创建模板 | `name: string, description?: string, num_partitions: number, replication_factor: number, config?: object` |
| `template.update` | 更新模板 | `id: number, name?: string, description?: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `template.delete` | 删除模板 | `id: number` |
| `template.presets` | 预定义模板 | 无 |
| `template.create_topic` | 使用模板创建 Topic | `cluster_id: string, topic_name: string, template_id?: number, template_name?: string, override_config?: object` |

---

### 资源标签

| Method | 描述 | 参数 |
|--------|------|------|
| `tag.list` | 标签列表 | `cluster_id: string, resource_type: string, resource_name: string` |
| `tag.create` | 创建标签 | `cluster_id: string, resource_type: string, resource_name: string, key: string, value: string` |
| `tag.delete` | 删除标签 | `cluster_id: string, resource_type: string, resource_name: string, key: string` |
| `tag.topics` | 获取 Topic 标签 | `cluster_id: string, resource_type?: string, resource_name?: string` |
| `tag.keys` | 标签键列表 | `cluster_id: string, resource_type?: string` |
| `tag.values` | 标签值列表 | `cluster_id: string, key: string, resource_type?: string` |
| `tag.filter` | 按标签过滤 | `cluster_id: string, resource_type: string, tag_key: string, tag_value?: string` |
| `tag.batch_update` | 批量更新标签 | `cluster_id: string, resource_type: string, resource_name: string, tags: object` |

---

### 审计日志

| Method | 描述 | 参数 |
|--------|------|------|
| `audit_log.list` | 审计日志列表 | `limit?: number, offset?: number, action?: string, cluster_id?: string, status?: number` |

---

### 认证管理

| Method | 描述 | 参数 |
|--------|------|------|
| `auth.api_keys` | API Key 列表 | 无 |
| `auth.api_key_create` | 创建 API Key | `name?: string, expires_in_days?: number` |
| `auth.api_key_revoke` | 撤销 API Key | `id: number` |

---

## 使用示例

### 创建集群

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: cluster.create" \
  -d '{
    "name": "test-cluster",
    "brokers": "localhost:9092",
    "request_timeout_ms": 30000,
    "operation_timeout_ms": 30000
  }'
```

**响应:**
```json
{
  "id": 1,
  "name": "test-cluster",
  "brokers": "localhost:9092",
  "request_timeout_ms": 30000,
  "operation_timeout_ms": 30000,
  "created_at": "2026-03-07T10:00:00Z",
  "updated_at": "2026-03-07T10:00:00Z"
}
```

### 获取 Topic 列表

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.list" \
  -d '{
    "cluster_id": "test-cluster"
  }'
```

**响应:**
```json
{
  "topics": [
    {"name": "topic-1", "partition_count": 3},
    {"name": "topic-2", "partition_count": 6}
  ]
}
```

### 创建 Topic

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.create" \
  -d '{
    "cluster_id": "test-cluster",
    "name": "my-topic",
    "num_partitions": 3,
    "replication_factor": 1
  }'
```

### 删除 Topic

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.delete" \
  -d '{
    "cluster_id": "test-cluster",
    "topic": "my-topic"
  }'
```

### 删除集群下所有 Topic

> **注意**: 此接口仅删除数据库中的 Topic 元数据，不会删除 Kafka 集群中的实际 Topic。

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.delete_all" \
  -d '{
    "cluster_id": "test-cluster"
  }'
```

**响应:**
```json
{
  "success": true,
  "deleted": ["topic-1", "topic-2", "topic-3"],
  "failed": [],
  "total_deleted": 3,
  "total_failed": 0
}
```

### 发送消息

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: message.send" \
  -d '{
    "cluster_id": "test-cluster",
    "topic": "my-topic",
    "key": "user-123",
    "value": "{\"event\": \"user_login\", \"userId\": 123}"
  }'
```

### 获取消息列表

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: message.list" \
  -d '{
    "cluster_id": "test-cluster",
    "topic": "my-topic",
    "partition": 0,
    "max_messages": 100,
    "order_by": "timestamp",
    "sort": "desc",
    "limit": 50
  }'
```

### 重置 Consumer Group Offset

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: consumer_group.offsets_reset" \
  -d '{
    "cluster_id": "test-cluster",
    "group_name": "my-consumer-group",
    "topic": "my-topic",
    "offset": {
      "type": "latest"
    }
  }'
```

### 使用模板创建 Topic

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: template.create_topic" \
  -d '{
    "cluster_id": "test-cluster",
    "topic_name": "high-throughput-topic",
    "template_name": "high-throughput"
  }'
```

### 批量操作

```bash
# 批量创建 Topic
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.batch_create" \
  -d '{
    "cluster_id": "test-cluster",
    "topics": [
      {"name": "topic-1", "num_partitions": 3, "replication_factor": 1},
      {"name": "topic-2", "num_partitions": 6, "replication_factor": 1}
    ],
    "continue_on_error": true
  }'

# 批量删除 Consumer Group
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: consumer_group.batch_delete" \
  -d '{
    "cluster_id": "test-cluster",
    "group_names": ["group-1", "group-2"],
    "continue_on_error": true
  }'
```

---

## 认证

如果启用了认证，需要在请求头中添加 API Key：

```http
X-API-Key: your-api-key
```

或者使用环境变量配置：
```bash
export API_KEYS="key1,key2,key3"
export AUTH_ENABLED=true
```

---

## 错误码

| HTTP 状态码 | 说明 |
|-------------|------|
| 200 OK | 请求成功 |
| 400 Bad Request | 请求参数错误 |
| 401 Unauthorized | 认证失败 |
| 403 Forbidden | 权限不足 |
| 404 Not Found | 资源不存在 |
| 500 Internal Server Error | 服务器内部错误 |

---

## 注意事项

1. **统一端点**: 所有 API 请求都使用 `POST /api`，通过 `X-API-Method` 区分操作
2. **参数命名**: 参数使用 snake_case 命名风格
3. **时间戳**: 所有时间戳使用毫秒精度 (Unix timestamp in milliseconds)
4. **错误处理**: 检查响应的 `success` 字段判断是否成功
5. **批量操作**: 批量操作支持 `continue_on_error` 参数，设置为 `true` 时即使部分失败也会继续执行
