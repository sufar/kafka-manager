# 统一 API 接口文档

## 概述

所有 API 请求都通过统一的 POST 端点 `/api` 发送，操作类型通过 `X-API-Method` HTTP Header 指定，请求参数放在 JSON Body 中。

## 请求格式

```http
POST /api HTTP/1.1
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

## API 方法列表

### Health

| Method | 描述 | 参数 |
|--------|------|------|
| `health` | 健康检查 | 无 |

### Cluster 集群管理

| Method | 描述 | 参数 |
|--------|------|------|
| `cluster.list` | 获取集群列表 | 无 |
| `cluster.get` | 获取集群详情 | `id: number` |
| `cluster.create` | 创建集群 | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.update` | 更新集群 | `id: number, name?: string, brokers?: string, ...` |
| `cluster.delete` | 删除集群 | `id: number` |
| `cluster.test` | 测试集群连接 | `id: number` |

### Topic 主题管理

| Method | 描述 | 参数 |
|--------|------|------|
| `topic.list` | 获取Topic列表 | `cluster: string` |
| `topic.get` | 获取Topic详情 | `cluster: string, topic: string` |
| `topic.create` | 创建Topic | `cluster: string, name: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `topic.delete` | 删除Topic | `cluster: string, topic: string` |
| `topic.batch_create` | 批量创建Topic | `cluster: string, topics: array, continue_on_error?: boolean` |
| `topic.batch_delete` | 批量删除Topic | `cluster: string, topics: string[], continue_on_error?: boolean` |
| `topic.offsets` | 获取Topic偏移量 | `cluster: string, topic: string` |
| `topic.config.get` | 获取Topic配置 | `cluster: string, topic: string` |
| `topic.config.alter` | 修改Topic配置 | `cluster: string, topic: string, config: object` |
| `topic.partitions.add` | 增加分区 | `cluster: string, topic: string, new_partitions: number` |
| `topic.throughput` | 获取Topic吞吐量 | `cluster: string, topic: string` |
| `topic.refresh` | 刷新Topic列表 | `cluster: string` |
| `topic.search` | 搜索所有集群的Topic | 无 |
| `topic.count` | 获取Topic数量 | `cluster: string` |

### Consumer Group 消费者组管理

| Method | 描述 | 参数 |
|--------|------|------|
| `consumer_group.list` | 获取消费者组列表 | `cluster: string` |
| `consumer_group.get` | 获取消费者组详情 | `cluster: string, group: string` |
| `consumer_group.delete` | 删除消费者组 | `cluster: string, group: string` |
| `consumer_group.offsets` | 获取消费者组偏移量 | `cluster: string, group: string, topic?: string` |
| `consumer_group.offsets.reset` | 重置消费者组偏移量 | `cluster: string, group: string, topic: string, offset_type: string, offset_value?: number, partition?: number` |
| `consumer_group.throughput` | 获取消费者组吞吐量 | `cluster: string, group: string, topic: string` |
| `consumer_group.batch_delete` | 批量删除消费者组 | `cluster: string, group_names: string[], continue_on_error?: boolean` |
| `consumer_group.consumer_offsets` | 获取所有消费者组的偏移量 | `cluster: string` |

### Message 消息管理

| Method | 描述 | 参数 |
|--------|------|------|
| `message.list` | 获取消息列表 | `cluster: string, topic: string, partition?: number, offset?: number, max_messages?: number, ...` |
| `message.send` | 发送消息 | `cluster: string, topic: string, partition: number, key?: string, value: string, headers?: object` |
| `message.export` | 导出消息 | (使用原始URL) |

### Cluster Connection 集群连接管理

| Method | 描述 | 参数 |
|--------|------|------|
| `cluster_connection.list` | 获取所有连接状态 | 无 |
| `cluster_connection.get` | 获取连接状态 | `cluster: string` |
| `cluster_connection.disconnect` | 断开连接 | `cluster: string` |
| `cluster_connection.reconnect` | 重新连接 | `cluster: string` |
| `cluster_connection.health_check` | 健康检查 | `cluster: string` |
| `cluster_connection.metrics` | 获取连接指标 | `cluster: string` |
| `cluster_connection.history` | 获取连接历史 | `cluster: string, limit?: number` |
| `cluster_connection.stats` | 获取连接统计 | `cluster: string` |
| `cluster_connection.batch_disconnect` | 批量断开连接 | `cluster_names: string[]` |
| `cluster_connection.batch_reconnect` | 批量重新连接 | `cluster_names: string[]` |

### User 用户管理

| Method | 描述 | 参数 |
|--------|------|------|
| `user.list` | 获取用户列表 | 无 |
| `user.get` | 获取用户详情 | `id: number` |
| `user.create` | 创建用户 | `username: string, password: string, email?: string, role_id?: number` |
| `user.update` | 更新用户 | `id: number, email?: string, role_id?: number, is_active?: boolean` |
| `user.password.update` | 更新密码 | `id: number, old_password: string, new_password: string` |

### Role 角色管理

| Method | 描述 | 参数 |
|--------|------|------|
| `role.list` | 获取角色列表 | 无 |
| `role.get` | 获取角色详情 | `id: number` |
| `role.create` | 创建角色 | `name: string, description?: string, permissions: string[]` |
| `role.update` | 更新角色 | `id: number, name?: string, description?: string, permissions?: string[]` |

### Notification 通知管理

| Method | 描述 | 参数 |
|--------|------|------|
| `notification.list` | 获取通知配置列表 | 无 |
| `notification.get` | 获取通知配置详情 | `id: number` |
| `notification.create` | 创建通知配置 | `name: string, type: string, config: object, enabled?: boolean` |
| `notification.delete` | 删除通知配置 | `id: number` |
| `notification.enable` | 启用通知 | `id: number` |
| `notification.disable` | 禁用通知 | `id: number` |
| `alert.history` | 获取告警历史 | `limit?: number, severity?: string, notified?: boolean` |

### Schema Registry

| Method | 描述 | 参数 |
|--------|------|------|
| `schema.subjects` | 获取Subject列表 | `cluster: string, schema_registry_url: string` |
| `schema.versions` | 获取Subject版本列表 | `cluster: string, subject: string, schema_registry_url: string` |
| `schema.get` | 获取Schema详情 | `cluster: string, subject: string, version: string, schema_registry_url: string` |
| `schema.register` | 注册Schema | `cluster: string, subject: string, schema: string, schema_type?: string` |
| `schema.delete` | 删除Schema | `cluster: string, subject: string, schema_registry_url: string` |
| `schema.version.delete` | 删除Schema版本 | `cluster: string, subject: string, version: string, schema_registry_url: string` |
| `schema.compatibility_level` | 获取兼容性级别 | `cluster: string, schema_registry_url: string` |

### Settings 设置

| Method | 描述 | 参数 |
|--------|------|------|
| `settings.get` | 获取设置 | `keys?: string[]` |
| `settings.update` | 更新设置 | `key: string, value: string` |

### Cluster Monitor 集群监控

| Method | 描述 | 参数 |
|--------|------|------|
| `cluster.stats` | 获取集群统计 | `cluster: string` |
| `cluster.info` | 获取集群信息 | `cluster: string` |
| `cluster.metrics` | 获取集群指标 | `cluster: string` |
| `cluster.brokers` | 获取Broker列表 | `cluster: string` |
| `cluster.broker.get` | 获取Broker详情 | `cluster: string, broker_id: number` |

## 示例

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

### 获取Topic列表

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.list" \
  -d '{
    "cluster": "test-cluster"
  }'
```

### 删除Topic

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.delete" \
  -d '{
    "cluster": "test-cluster",
    "topic": "test-topic"
  }'
```
