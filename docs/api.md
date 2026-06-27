# Kafka Manager API Reference

## Overview

Kafka Manager uses a **unified POST API** design. All API requests are made to `POST /api` with the operation specified via the `X-API-Method` HTTP header. Request parameters are passed in the JSON body.

For Chinese version, see [api-cn.md](./api-cn.md).

## Request Format

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

## Response Format

### Success Response
```json
{
  "success": true,
  "data": { ... }
}
```

### Error Response
```json
{
  "success": false,
  "error": "Error message"
}
```

### HTTP Error Response
```json
{
  "success": false,
  "error": "Bad Request",
  "message": "Detailed error description"
}
```

---

## API Methods

### Health Check

| Method | Description | Parameters |
|--------|-------------|------------|
| `health` | Health check endpoint | None |

---

### Cluster Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `cluster.list` | Get cluster list | None |
| `cluster.get` | Get cluster details | `id: number` |
| `cluster.create` | Create cluster | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.update` | Update cluster | `id: number, name?: string, brokers?: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.delete` | Delete cluster | `id: number` |
| `cluster.test` | Test cluster connection | `id: number` |
| `cluster.test_config` | Test cluster config (without saving) | `brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.stats` | Get cluster stats | `cluster_id: string` |

---

### Cluster Group Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `cluster_group.list` | Get cluster group list | None |
| `cluster_group.get` | Get cluster group details | `id: number` |
| `cluster_group.create` | Create cluster group | `name: string, description?: string` |
| `cluster_group.update` | Update cluster group | `id: number, name?: string, description?: string` |
| `cluster_group.delete` | Delete cluster group | `id: number` |
| `cluster_group.clusters` | Get clusters in group | `group_id: number` |
| `cluster_group.assign_cluster` | Assign cluster to group | `group_id: number, cluster_id: string` |

---

### Topic Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `topic.list_with_cluster` | Get topic list with cluster info | `cluster_id: string` |
| `topic.get` | Get topic details | `cluster_id: string, name: string` |
| `topic.create` | Create topic | `cluster_id: string, name: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `topic.delete` | Delete topic | `cluster_id: string, name: string` |
| `topic.batch_create` | Batch create topics | `cluster_id: string, topics: array, continue_on_error?: boolean` |
| `topic.batch_delete` | Batch delete topics | `cluster_id: string, topics: string[], continue_on_error?: boolean` |
| `topic.offsets` | Get topic offsets | `cluster_id: string, name: string` |
| `topic.config_get` | Get topic config | `cluster_id: string, name: string` |
| `topic.config_alter` | Alter topic config | `cluster_id: string, name: string, config: object` |
| `topic.partitions_add` | Add partitions | `cluster_id: string, name: string, new_partitions: number` |
| `topic.partition.watermarks` | Get partition watermarks | `cluster_id: string, topic: string, partition: number` |
| `topic.throughput` | Get topic throughput | `cluster_id: string, name: string` |
| `topic.refresh` | Refresh topic list | `cluster_id: string` |
| `topic.saved` | Get saved topics | `cluster_id: string` |
| `topic.search` | Search topics across clusters | `search?: string` |
| `topic.count` | Get topic count | `cluster_id: string` |
| `topic.cleanup_orphans` | Cleanup orphan topic metadata | `cluster_id: string` |

---

### Consumer Group Management

> **Note**: Consumer group operations are not currently implemented in this version. The `consumer_group_count` field is available in `cluster.stats` response but provides only a count (always 0).

---

### Message Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `message.list` | Get message list | `cluster_id: string, topic: string, partition?: number, offset?: number, max_messages?: number, order_by?: string, sort?: string, limit?: number, search?: string, search_in?: string, format?: string, decode?: string` |
| `message.send` | Send message | `cluster_id: string, topic: string, value: string, key?: string, partition?: number, headers?: object` |
| `message.export` | Export messages | `cluster_id: string, topic: string, partition?: number, offset?: number, limit?: number, format?: string` |

> **Note**: Message export uses a separate endpoint: `GET /api/clusters/:cluster_id/topics/:topic/messages/export`

---

### Cluster Connection Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `connection.list` | Get all connection statuses | None |
| `connection.get` | Get connection status | `cluster_id: string` |
| `connection.disconnect` | Disconnect connection | `cluster_id: string` |
| `connection.reconnect` | Reconnect | `cluster_id: string` |
| `connection.health_check` | Health check | `cluster_id: string` |
| `connection.metrics` | Get connection metrics | `cluster_id: string` |
| `connection.history` | Get connection history | `cluster_id: string, limit?: number, offset?: number` |
| `connection.stats` | Get connection stats | `cluster_id: string` |
| `connection.batch_disconnect` | Batch disconnect | `cluster_names: string[]` |
| `connection.batch_reconnect` | Batch reconnect | `cluster_names: string[]` |

---

### Settings Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `settings.get` | Get settings | `keys?: string[]` |
| `settings.update` | Update settings | `key: string, value: string` |

---

### Favorite Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `favorite.group.list` | Get favorite group list | None |
| `favorite.group.create` | Create favorite group | `name: string, description?: string, sort_order?: number` |
| `favorite.group.update` | Update favorite group | `id: number, name?: string, description?: string, sort_order?: number` |
| `favorite.group.delete` | Delete favorite group | `id: number` |
| `favorite.list` | Get favorite list | `cluster_id?: string, group_id?: number` |
| `favorite.create` | Create favorite | `cluster_id: string, topic_name: string, group_id?: number, remark?: string, sort_order?: number` |
| `favorite.update` | Update favorite | `id: number, group_id?: number, remark?: string, sort_order?: number` |
| `favorite.delete` | Delete favorite | `id: number` |
| `favorite.check` | Check if topic is favorited | `cluster_id: string, topic_name: string` |
| `favorite.delete_by_topic` | Delete favorite by topic | `cluster_id: string, topic_name: string` |

---

### Audit Log

| Method | Description | Parameters |
|--------|-------------|------------|
| `audit_log.list` | Get audit log list | `limit?: number, offset?: number, action?: string, cluster_id?: string, status?: number` |

---

## Usage Examples

### Create Cluster

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

**Response:**
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

### Get Topic List

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.list" \
  -d '{
    "cluster_id": "test-cluster"
  }'
```

**Response:**
```json
{
  "topics": [
    {"name": "topic-1", "partition_count": 3},
    {"name": "topic-2", "partition_count": 6}
  ]
}
```

### Create Topic

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

### Delete Topic

```bash
curl -X POST http://localhost:9732/api \
  -H "Content-Type: application/json" \
  -H "X-API-Method: topic.delete" \
  -d '{
    "cluster_id": "test-cluster",
    "name": "my-topic"
  }'
```

### Send Message

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

### Get Message List

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

### Reset Consumer Group Offset

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

### Batch Operations

```bash
# Batch create topics
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

# Batch delete consumer groups
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

## HTTP Status Codes

| Status Code | Description |
|-------------|-------------|
| 200 OK | Request successful |
| 400 Bad Request | Invalid request parameters |
| 404 Not Found | Resource not found |
| 500 Internal Server Error | Server error |

---

## Notes

1. **Unified Endpoint**: All API requests use `POST /api` with `X-API-Method` header to distinguish operations
2. **Parameter Naming**: Parameters use snake_case naming convention
3. **Timestamps**: All timestamps use millisecond precision (Unix timestamp in milliseconds)
4. **Error Handling**: Check the `success` field in the response to determine if the request succeeded
5. **Batch Operations**: Batch operations support `continue_on_error` parameter to continue execution even if some operations fail
