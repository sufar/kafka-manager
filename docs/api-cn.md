# Kafka Manager API 参考文档

## 概述

Kafka Manager 采用**统一的 POST API**设计。所有 API 请求都发送到 `POST /api`，操作通过 `X-API-Method` HTTP 头指定，请求参数通过 JSON body 传递。

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

### HTTP 错误响应
```json
{
  "success": false,
  "error": "错误类型",
  "message": "详细描述"
}
```

---

## API 方法

### 健康检查

| 方法 | 描述 | 参数 |
|------|------|------|
| `health` | 健康检查端点 | 无 |

---

### 集群管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `cluster.list` | 获取集群列表 | 无 |
| `cluster.get` | 获取集群详情 | `id: number` |
| `cluster.create` | 创建集群 | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.update` | 更新集群 | `id: number, name?: string, brokers?: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.delete` | 删除集群 | `id: number` |
| `cluster.test` | 测试集群连接 | `id: number` |
| `cluster.test_config` | 测试集群配置（不保存） | `brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.stats` | 获取集群统计信息 | `cluster_id: string` |

---

### 集群分组管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `cluster_group.list` | 获取集群分组列表 | 无 |
| `cluster_group.get` | 获取集群分组详情 | `id: number` |
| `cluster_group.create` | 创建集群分组 | `name: string, description?: string` |
| `cluster_group.update` | 更新集群分组 | `id: number, name?: string, description?: string` |
| `cluster_group.delete` | 删除集群分组 | `id: number` |
| `cluster_group.clusters` | 获取分组内的集群列表 | `group_id: number` |
| `cluster_group.assign_cluster` | 将集群分配到分组 | `group_id: number, cluster_id: string` |

---

### Topic 管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `topic.list` | 获取 Topic 列表 | `cluster_id: string` |
| `topic.list_with_cluster` | 获取带集群信息的 Topic 列表 | `cluster_id: string` |
| `topic.get` | 获取 Topic 详情 | `cluster_id: string, name: string` |
| `topic.create` | 创建 Topic | `cluster_id: string, name: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `topic.delete` | 删除 Topic | `cluster_id: string, name: string` |
| `topic.delete_all` | 删除集群中的所有 Topic | `cluster_id: string` |
| `topic.batch_create` | 批量创建 Topic | `cluster_id: string, topics: array, continue_on_error?: boolean` |
| `topic.batch_delete` | 批量删除 Topic | `cluster_id: string, topics: string[], continue_on_error?: boolean` |
| `topic.offsets` | 获取 Topic 的分区水位 | `cluster_id: string, name: string` |
| `topic.config_get` | 获取 Topic 配置 | `cluster_id: string, name: string` |
| `topic.config_alter` | 修改 Topic 配置 | `cluster_id: string, name: string, config: object` |
| `topic.partitions_add` | 增加 Topic 分区数 | `cluster_id: string, name: string, new_partitions: number` |
| `topic.partition.watermarks` | 获取分区水位信息 | `cluster_id: string, topic: string, partition: number` |
| `topic.throughput` | 获取 Topic 吞吐量统计 | `cluster_id: string, name: string` |
| `topic.refresh` | 刷新 Topic 列表 | `cluster_id: string` |
| `topic.saved` | 获取已收藏的 Topic | `cluster_id: string` |
| `topic.search` | 跨集群搜索 Topic | `search?: string` |
| `topic.count` | 获取 Topic 数量 | `cluster_id: string` |
| `topic.cleanup_orphans` | 清理孤儿 Topic 元数据 | `cluster_id: string` |

---

### 消费者组管理

> **注意**: 当前版本未实现消费者组操作。`cluster.stats` 响应中包含 `consumer_group_count` 字段，但仅提供计数（始终为 0）。

---

### 消息管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `message.list` | 获取消息列表 | `cluster_id: string, topic: string, partition?: number, offset?: number, max_messages?: number, order_by?: string, sort?: string, limit?: number, search?: string, search_in?: string, format?: string, decode?: string` |
| `message.send` | 发送消息 | `cluster_id: string, topic: string, value: string, key?: string, partition?: number, headers?: object` |
| `message.export` | 导出消息 | `cluster_id: string, topic: string, partition?: number, offset?: number, limit?: number, format?: string` |

> **注意**: 消息导出还有一个独立的 GET 端点：`GET /api/clusters/:cluster_id/topics/:topic/messages/export`

---

### 集群连接管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `connection.list` | 获取所有连接状态 | 无 |
| `connection.get` | 获取连接状态 | `cluster_id: string` |
| `connection.disconnect` | 断开连接 | `cluster_id: string` |
| `connection.reconnect` | 重新连接 | `cluster_id: string` |
| `connection.health_check` | 健康检查 | `cluster_id: string` |
| `connection.metrics` | 获取连接池指标 | `cluster_id: string` |
| `connection.history` | 获取连接历史 | `cluster_id: string, limit?: number, offset?: number` |
| `connection.stats` | 获取连接统计 | `cluster_id: string` |
| `connection.batch_disconnect` | 批量断开连接 | `cluster_names: string[]` |
| `connection.batch_reconnect` | 批量重新连接 | `cluster_names: string[]` |

---

### 设置管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `settings.get` | 获取设置 | `keys?: string[]` |
| `settings.update` | 更新设置 | `key: string, value: string` |

---

### Topic 模板管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `template.list` | 获取模板列表 | 无 |
| `template.get` | 获取模板详情 | `id: number` |
| `template.create` | 创建模板 | `name: string, description?: string, num_partitions: number, replication_factor: number, config?: object` |
| `template.update` | 更新模板 | `id: number, name?: string, description?: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `template.delete` | 删除模板 | `id: number` |
| `template.presets` | 获取预定义模板 | 无 |
| `template.create_topic` | 从模板创建 Topic | `cluster_id: string, topic_name: string, template_id?: number, template_name?: string, override_config?: object` |

---

### 收藏管理

| 方法 | 描述 | 参数 |
|------|------|------|
| `favorite.group.list` | 获取收藏分组列表 | 无 |
| `favorite.group.create` | 创建收藏分组 | `name: string, description?: string, sort_order?: number` |
| `favorite.group.get` | 获取收藏分组详情 | `id: number` |
| `favorite.group.update` | 更新收藏分组 | `id: number, name?: string, description?: string, sort_order?: number` |
| `favorite.group.delete` | 删除收藏分组 | `id: number` |
| `favorite.list` | 获取收藏列表 | `cluster_id?: string, group_id?: number` |
| `favorite.create` | 创建收藏 | `cluster_id: string, topic_name: string, group_id?: number, remark?: string, sort_order?: number` |
| `favorite.get` | 获取收藏详情 | `id: number` |
| `favorite.update` | 更新收藏 | `id: number, group_id?: number, remark?: string, sort_order?: number` |
| `favorite.delete` | 删除收藏 | `id: number` |
| `favorite.check` | 检查 Topic 是否已收藏 | `cluster_id: string, topic_name: string` |
| `favorite.delete_by_topic` | 按 Topic 删除收藏 | `cluster_id: string, topic_name: string` |

---

### 审计日志

| 方法 | 描述 | 参数 |
|------|------|------|
| `audit_log.list` | 获取审计日志列表 | `limit?: number, offset?: number, action?: string, cluster_id?: string, status?: number` |

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
    "name": "my-topic"
  }'
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

# 批量删除 Topic
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.batch_delete" \
  -d '{
    "cluster_id": "test-cluster",
    "topics": ["topic-1", "topic-2"],
    "continue_on_error": true
  }'
```

---

## 认证

如果启用了认证，需要在请求头中包含 API Key：

```http
X-API-Key: your-api-key
```

或通过环境变量配置：

```bash
export API_KEYS="key1,key2,key3"
export AUTH_ENABLED=true
```

---

## HTTP 状态码

| 状态码 | 描述 |
|--------|------|
| 200 OK | 请求成功 |
| 400 Bad Request | 无效的请求参数 |
| 404 Not Found | 资源不存在 |
| 500 Internal Server Error | 服务器错误 |

---

## 注意事项

1. **统一端点**: 所有 API 请求都使用 `POST /api`，通过 `X-API-Method` 头区分操作
2. **参数命名**: 参数使用 snake_case 命名规范
3. **时间戳**: 所有时间戳使用毫秒精度（Unix 时间戳，单位为毫秒）
4. **错误处理**: 检查响应中的 `success` 字段判断请求是否成功
5. **批量操作**: 批量操作支持 `continue_on_error` 参数，即使部分操作失败也能继续执行
6. **流式传输**: 消息列表查询支持 SSE 流式传输，通过 `POST /api/stream` 端点
