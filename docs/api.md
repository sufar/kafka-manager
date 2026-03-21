# Kafka Manager API Reference

## Overview

Kafka Manager uses a **unified POST API** design. All API requests are made to `POST /api` with the operation specified via the `X-API-Method` HTTP header. Request parameters are passed in the JSON body.

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

---

### Topic Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `topic.list` | Get topic list | `cluster_id: string` |
| `topic.get` | Get topic details | `cluster_id: string, name: string` |
| `topic.create` | Create topic | `cluster_id: string, name: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `topic.delete` | Delete topic | `cluster_id: string, name: string` |
| `topic.delete_all` | Delete all topics in cluster | `cluster_id: string` |
| `topic.batch_create` | Batch create topics | `cluster_id: string, topics: array, continue_on_error?: boolean` |
| `topic.batch_delete` | Batch delete topics | `cluster_id: string, topics: string[], continue_on_error?: boolean` |
| `topic.offsets` | Get topic offsets | `cluster_id: string, name: string` |
| `topic.config_get` | Get topic config | `cluster_id: string, name: string` |
| `topic.config_alter` | Alter topic config | `cluster_id: string, name: string, config: object` |
| `topic.partitions.add` | Add partitions | `cluster_id: string, name: string, new_partitions: number` |
| `topic.throughput` | Get topic throughput | `cluster_id: string, name: string` |
| `topic.refresh` | Refresh topic list | `cluster_id: string` |
| `topic.saved` | Get saved topics | `cluster_id: string` |
| `topic.search` | Search topics across clusters | `search?: string` |
| `topic.count` | Get topic count | `cluster_id: string` |

---

### Consumer Group Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `consumer_group.list` | Get consumer group list | `cluster_id: string` |
| `consumer_group.get` | Get consumer group details | `cluster_id: string, name: string` |
| `consumer_group.delete` | Delete consumer group | `cluster_id: string, name: string` |
| `consumer_group.offsets` | Get consumer group offsets | `cluster_id: string, name: string, topic?: string` |
| `consumer_group.offsets_reset` | Reset consumer group offsets | `cluster_id: string, name: string, topic: string, offset: { type: string, value?: number }, partition?: number` |
| `consumer_group.throughput` | Get consumer group throughput | `cluster_id: string, name: string, topic: string` |
| `consumer_group.batch_delete` | Batch delete consumer groups | `cluster_id: string, group_names: string[], continue_on_error?: boolean` |
| `consumer_group.consumer_offsets` | Get all consumer group offsets | `cluster_id: string` |
| `consumer_lag.get` | Get topic consumer lag | `cluster_id: string, topic: string` |
| `consumer_lag.history` | Get consumer lag history | `cluster_id: string, topic: string, start_time?: number, end_time?: number` |

---

### Message Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `message.list` | Get message list | `cluster_id: string, topic: string, partition?: number, offset?: number, max_messages?: number, order_by?: string, sort?: string, limit?: number, search?: string, search_in?: string, format?: string, decode?: string` |
| `message.send` | Send message | `cluster_id: string, topic: string, value: string, key?: string, partition?: number, headers?: object` |

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

### Cluster Monitoring

| Method | Description | Parameters |
|--------|-------------|------------|
| `monitor.stats` | Get cluster stats | `cluster_id: string` |
| `monitor.info` | Get cluster info | `cluster_id: string` |
| `monitor.metrics` | Get cluster metrics | `cluster_id: string` |
| `monitor.brokers` | Get broker list | `cluster_id: string` |
| `monitor.broker_get` | Get broker details | `cluster_id: string, broker_id: number` |

---

### User Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `user.list` | Get user list | None |
| `user.get` | Get user details | `id: number` |
| `user.create` | Create user | `username: string, password: string, email?: string, role_id?: number` |
| `user.update` | Update user | `id: number, email?: string, role_id?: number, is_active?: boolean` |
| `user.password_update` | Update password | `id: number, old_password: string, new_password: string` |

---

### Role Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `role.list` | Get role list | None |
| `role.get` | Get role details | `id: number` |
| `role.create` | Create role | `name: string, description?: string, permissions: string[]` |
| `role.update` | Update role | `id: number, name?: string, description?: string, permissions?: string[]` |

---

### Notification Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `notification.list` | Get notification config list | None |
| `notification.get` | Get notification config details | `id: number` |
| `notification.create` | Create notification config | `name: string, config_type: string, webhook_url?: string, email_recipients?: string[], dingtalk_webhook?: string, slack_webhook?: string, wechat_webhook?: string, enabled?: boolean` |
| `notification.delete` | Delete notification config | `id: number` |
| `notification.enable` | Enable notification | `id: number` |
| `notification.disable` | Disable notification | `id: number` |
| `alert_history.list` | Get alert history | `cluster_id?: string, severity?: string, notified?: boolean, limit?: number, offset?: number` |

---

### Schema Registry

| Method | Description | Parameters |
|--------|-------------|------------|
| `schema.subjects` | Get subject list | `cluster_id: string, schema_registry_url: string` |
| `schema.versions` | Get subject versions | `cluster_id: string, subject: string, schema_registry_url: string` |
| `schema.get` | Get schema details | `cluster_id: string, subject: string, version?: string, schema_registry_url: string` |
| `schema.register` | Register schema | `cluster_id: string, subject: string, schema: object, schema_type?: string` |
| `schema.delete` | Delete schema | `cluster_id: string, subject: string, schema_registry_url: string` |
| `schema.version_delete` | Delete schema version | `cluster_id: string, subject: string, version: string, schema_registry_url: string` |
| `schema.compatibility_level` | Get compatibility level | `cluster_id: string, schema_registry_url: string` |

---

### Settings Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `settings.get` | Get settings | `keys?: string[]` |
| `settings.update` | Update settings | `key: string, value: string` |

---

### Topic Templates

| Method | Description | Parameters |
|--------|-------------|------------|
| `template.list` | Get template list | None |
| `template.get` | Get template details | `id: number` |
| `template.create` | Create template | `name: string, description?: string, num_partitions: number, replication_factor: number, config?: object` |
| `template.update` | Update template | `id: number, name?: string, description?: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `template.delete` | Delete template | `id: number` |
| `template.presets` | Get predefined templates | None |
| `template.create_topic` | Create topic from template | `cluster_id: string, topic_name: string, template_id?: number, template_name?: string, override_config?: object` |

---

### Resource Tags

| Method | Description | Parameters |
|--------|-------------|------------|
| `tag.list` | Get tag list | `cluster_id: string, resource_type: string, resource_name: string` |
| `tag.create` | Create tag | `cluster_id: string, resource_type: string, resource_name: string, key: string, value: string` |
| `tag.delete` | Delete tag | `cluster_id: string, resource_type: string, resource_name: string, key: string` |
| `tag.topics` | Get topic tags | `cluster_id: string, resource_type?: string, resource_name?: string` |
| `tag.keys` | Get tag keys | `cluster_id: string, resource_type?: string` |
| `tag.values` | Get tag values | `cluster_id: string, key: string, resource_type?: string` |
| `tag.filter` | Filter by tag | `cluster_id: string, resource_type: string, tag_key: string, tag_value?: string` |
| `tag.batch_update` | Batch update tags | `cluster_id: string, resource_type: string, resource_name: string, tags: object` |

---

### Audit Log

| Method | Description | Parameters |
|--------|-------------|------------|
| `audit_log.list` | Get audit log list | `limit?: number, offset?: number, action?: string, cluster_id?: string, status?: number` |

---

### Authentication

| Method | Description | Parameters |
|--------|-------------|------------|
| `auth.api_keys` | Get API key list | None |
| `auth.api_key_create` | Create API key | `name?: string, expires_in_days?: number` |
| `auth.api_key_revoke` | Revoke API key | `id: number` |

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

## Authentication

If authentication is enabled, include the API Key in the request header:

```http
X-API-Key: your-api-key
```

Or configure via environment variables:

```bash
export API_KEYS="key1,key2,key3"
export AUTH_ENABLED=true
```

---

## HTTP Status Codes

| Status Code | Description |
|-------------|-------------|
| 200 OK | Request successful |
| 400 Bad Request | Invalid request parameters |
| 401 Unauthorized | Authentication failed |
| 403 Forbidden | Insufficient permissions |
| 404 Not Found | Resource not found |
| 500 Internal Server Error | Server error |

---

## Notes

1. **Unified Endpoint**: All API requests use `POST /api` with `X-API-Method` header to distinguish operations
2. **Parameter Naming**: Parameters use snake_case naming convention
3. **Timestamps**: All timestamps use millisecond precision (Unix timestamp in milliseconds)
4. **Error Handling**: Check the `success` field in the response to determine if the request succeeded
5. **Batch Operations**: Batch operations support `continue_on_error` parameter to continue execution even if some operations fail
