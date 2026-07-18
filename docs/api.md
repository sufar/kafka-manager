# Kafka Manager IPC API Reference

## Overview

Kafka Manager is a **Tauri 2 desktop application**. The frontend (Vue 3) does not use HTTP — all business operations are invoked through **Tauri IPC commands** backed by a unified dispatcher in the Rust core (`kafka-manager-api`).

For Chinese version, see [api-cn.md](./api-cn.md).

There are three IPC commands for business operations:

| Command | Purpose |
|---------|---------|
| `api_request` | Unified entry for all request/response operations |
| `message_list_stream` | Streaming message query (events pushed over a Tauri `Channel`) |
| `cancel_message_list` | Cancel an in-flight streaming query |

Additionally, a set of **desktop shell commands** (updater, auto-launch, system tray, logs, etc.) is exposed directly — see [Desktop Shell Commands](#desktop-shell-commands).

## Request / Response Format

### `api_request`

```ts
import { invoke } from '@tauri-apps/api/core';

const data = await invoke('api_request', {
  method: 'topic.list',          // unified method name
  params: { cluster_id: 'prod' } // operation parameters (JSON)
});
```

- **Success**: the Promise resolves with the operation's result data directly (no envelope).
- **Failure**: the Promise rejects with an **error message string** (e.g. `"Cluster not found"`, `"Kafka error: ..."`).

The frontend wraps this in `ui/src/api/client.ts`, which exposes one typed method per operation and throws `{ message, status }` objects on failure, so components never call `invoke` directly.

```ts
import { apiClient } from '@/api/client';

try {
  const topics = await apiClient.getTopics('prod');
} catch (e) {
  console.error((e as { message: string }).message);
}
```

### `message_list_stream`

Streaming message queries push events over a `tauri.Channel`:

```ts
import { invoke, Channel } from '@tauri-apps/api/core';

interface StreamEvent { event: string; data: string } // data is a JSON string

const requestId = crypto.randomUUID();
const channel = new Channel<StreamEvent>();
channel.onmessage = (evt) => {
  const payload = JSON.parse(evt.data);
  switch (evt.event) {
    case 'start':    /* { partitions, total_target } */ break;
    case 'batch':    /* { messages, progress, total } */ break;
    case 'order':    /* { sort } */ break;
    case 'complete': /* { actual_total?, target_total? } */ break;
    case 'error':    /* { error } */ break;
  }
};

// The Promise resolves when the event stream ends
await invoke('message_list_stream', { requestId, params: { cluster_id: 'prod', topic: 'events' }, channel });

// Cancel from the frontend at any time:
await invoke('cancel_message_list', { requestId });
```

Event sequence: `start` → `batch`* → `order`? → `complete` | `error`.
The backend enforces a 120 s timeout; cancellation stops the Kafka consumers server-side via a cancellation token.

`ui/src/api/client.ts#getMessagesStream` wraps all of this behind a callback API (`onStart/onBatch/onOrder/onComplete/onError`) and returns a `StreamHandle` with `abort()`.

---

## Unified API Methods

All methods below are invoked via `api_request` with the given `method` name. Parameter naming uses `snake_case`; timestamps are Unix milliseconds.

### Health

| Method | Description | Parameters |
|--------|-------------|------------|
| `health` | Health check | None |

### Cluster Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `cluster.list` | List clusters | `group_id?: number, search?: string` |
| `cluster.get` | Get cluster details | `id: number` |
| `cluster.create` | Create cluster | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.update` | Update cluster | `id: number, name?: string, brokers?: string, ...` |
| `cluster.delete` | Delete cluster | `id: number` |
| `cluster.test` | Test saved cluster connection | `id: number` |
| `cluster.test_config` | Test connection without saving | `name: string, brokers: string, request_timeout_ms?: number, operation_timeout_ms?: number` |
| `cluster.stats` | Get cluster stats | `cluster_id: string` |

### Cluster Group Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `cluster_group.list` | List cluster groups | None |
| `cluster_group.get` | Get group details | `id: number` |
| `cluster_group.create` | Create group | `name: string, description?: string, sort_order?: number` |
| `cluster_group.update` | Update group | `id: number, name?: string, description?: string, sort_order?: number` |
| `cluster_group.delete` | Delete group | `id: number` |
| `cluster_group.clusters` | List clusters in group | `group_id: number` |
| `cluster_group.assign_cluster` | Assign cluster to group | `cluster_id: string, group_id: number` |

### Topic Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `topic.list` | List topics | `cluster_id?: string` |
| `topic.list_with_cluster` | List topics with cluster info (paged) | `cluster_id?: string, cluster_ids?: string[], offset?: number, limit?: number, search?: string` |
| `topic.get` | Get topic details | `cluster_id: string, name: string` |
| `topic.create` | Create topic | `cluster_id: string, name: string, num_partitions?: number, replication_factor?: number, config?: object` |
| `topic.delete` | Delete topic | `cluster_id: string, topic: string` |
| `topic.delete_all` | Delete all topics in cluster | `cluster_id: string` |
| `topic.batch_create` | Batch create topics | `cluster_id: string, topics: array, continue_on_error?: boolean` |
| `topic.batch_delete` | Batch delete topics | `cluster_id: string, topics: string[], continue_on_error?: boolean` |
| `topic.offsets` | Get topic partition offsets | `cluster_id: string, topic: string` |
| `topic.config_get` | Get topic config | `cluster_id: string, topic: string` |
| `topic.config_alter` | Alter topic config | `cluster_id: string, topic: string, config: object` |
| `topic.partitions_add` | Add partitions | `cluster_id: string, topic: string, new_partitions: number` |
| `topic.partition.watermarks` | Get partition watermarks | `cluster_id: string, topic: string, partition: number` |
| `topic.throughput` | Get topic throughput | `cluster_id: string, topic: string` |
| `topic.refresh` | Refresh topic list from cluster | `cluster_id?: string, topic_name?: string` |
| `topic.saved` | Get saved (cached) topics | `cluster_id: string` |
| `topic.search` | Search topics across clusters | `keyword?: string` |
| `topic.count` | Get topic count | `cluster_id: string` |
| `topic.cleanup_orphans` | Remove metadata of deleted topics | None |
| `refresh.status` | Get clusters currently refreshing | None |

### Consumer Group Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `consumer_group.list` | List consumer groups (paged) | `cluster_ids?: string[], offset?: number, limit?: number, search?: string` |
| `consumer_group.list_by_topic` | Groups consuming a topic (with lag) | `cluster_id: string, topic: string` |
| `consumer_group.get` | Get group info | `cluster_id: string, group_name: string` |
| `consumer_group.offsets` | Get group offsets per partition | `cluster_id: string, group_name: string` |
| `consumer_group.refresh` | Refresh group list from cluster | `cluster_id?: string, group_name?: string` |
| `consumer_group.saved` | Get saved (cached) groups | `cluster_id: string` |
| `consumer_group.reset_offset` | Reset offset | `cluster_id: string, group_name: string, topic: string, partition: number, reset_to: 'earliest'\|'latest'\|'offset'\|'timestamp', offset?: number, timestamp?: number` |
| `consumer_group.delete` | Delete consumer group | `cluster_id: string, group: string` |

### Message Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `message.list` | Query messages (non-streaming) | `cluster_id: string, topic: string, partition?: number, offset?: number, max_messages?: number, order_by?: 'timestamp'\|'offset', sort?: 'asc'\|'desc', search?: string, search_in?: 'key'\|'value'\|'all', start_time?: number, end_time?: number, fetchMode?: 'oldest'\|'newest'` |
| `message.send` | Send message | `cluster_id: string, topic: string, value: string, key?: string, partition?: number, headers?: object` |
| `message.export` | Export messages | `cluster_id: string, topic: string, partition?: number, max_messages?: number, search?: string, fetch_mode?: string, start_time?: number, end_time?: number` |

> For large result sets prefer the streaming variant of `message.list` via the `message_list_stream` command (same parameters).

### Cluster Connection Management

| Method | Description | Parameters |
|--------|-------------|------------|
| `connection.list` | List all connection statuses | None |
| `connection.get` | Get connection status | `cluster_id: string` |
| `connection.disconnect` | Disconnect a cluster | `cluster_id: string` |
| `connection.reconnect` | Reconnect a cluster | `cluster_name: string` |
| `connection.health_check` | Health check a cluster | `cluster_id: string` |
| `connection.metrics` | Get connection metrics | `cluster_id: string` |
| `connection.batch_disconnect` | Disconnect multiple clusters | `cluster_ids: string[]` |
| `connection.batch_reconnect` | Reconnect multiple clusters | `cluster_names: string[]` |

### Settings & Import/Export

| Method | Description | Parameters |
|--------|-------------|------------|
| `settings.get` | Get settings | `keys?: string[]` |
| `settings.update` | Update a setting | `key: string, value: string` |
| `settings.export` | Export all data (clusters, groups, topics, favorites, history) | None |
| `settings.import` | Import data (runs in background) | `data: object, strategy: 'skip'\|'overwrite'` |

### App Info

| Method | Description | Parameters |
|--------|-------------|------------|
| `app.version` | Get backend version | None |
| `app.logs` | Get recent backend logs | None |
| `app.logs.clear` | Clear backend logs | None |

### Topic Templates

| Method | Description | Parameters |
|--------|-------------|------------|
| `template.list` | List templates | None |
| `template.get` | Get template | `id: number` |
| `template.create` | Create template | `name: string, description?: string, num_partitions: number, replication_factor: number, config?: object` |
| `template.update` | Update template | `id: number, ...` |
| `template.delete` | Delete template | `id: number` |
| `template.presets` | List predefined templates | None |
| `template.create_topic` | Create topic from template | `cluster_id: string, topic_name: string, template_id?: number, template_name?: string, override_config?: object` |

### Favorites

| Method | Description | Parameters |
|--------|-------------|------------|
| `favorite.group.list` | List favorite groups (with item counts) | None |
| `favorite.group.create` | Create favorite group | `name: string, description?: string, sort_order?: number` |
| `favorite.group.get` | Get favorite group | `id: number` |
| `favorite.group.update` | Update favorite group | `id: number, ...` |
| `favorite.group.delete` | Delete favorite group | `id: number` |
| `favorite.list` | List favorites (grouped, with items) | None |
| `favorite.create` | Add favorite | `group_id: number, cluster_id: string, topic_name: string, description?: string, sort_order?: number` |
| `favorite.get` | Get favorite | `id: number` |
| `favorite.update` | Update favorite | `id: number, group_id?: number, description?: string, sort_order?: number` |
| `favorite.delete` | Delete favorite | `id: number` |
| `favorite.check` | Check if topic is favorited | `cluster_id: string, topic_name: string` |
| `favorite.delete_by_topic` | Remove favorite by topic | `cluster_id: string, topic_name: string` |

### Topic History

| Method | Description | Parameters |
|--------|-------------|------------|
| `topic_history.list` | List recently viewed topics | `limit?: number, offset?: number` |
| `topic_history.record` | Record a topic visit | `cluster_id: string, topic_name: string` |
| `topic_history.delete` | Delete a history entry | `id: number` |
| `topic_history.delete_by_topic` | Delete history for a topic | `cluster_id: string, topic_name: string` |
| `topic_history.clear` | Clear all history | None |

### Sent Message History

| Method | Description | Parameters |
|--------|-------------|------------|
| `sent_message.list` | List sent messages | `limit?: number, offset?: number, cluster_id?: string, topic_name?: string` |
| `sent_message.record` | Record a sent message | `cluster_id: string, topic_name: string, partition: number, key?: string, value: string, headers?: object, offset?: number` |
| `sent_message.delete` | Delete a sent-message entry | `id: number` |
| `sent_message.clear` | Clear sent-message history | None |

### JSON Highlight Templates

| Method | Description | Parameters |
|--------|-------------|------------|
| `json_highlight.list` | List highlight style templates | None |
| `json_highlight.get_current` | Get active template | None |
| `json_highlight.set_current` | Set active template | `id: number` |
| `json_highlight.create` | Create template | `name: string, description?: string, style_json: object` |
| `json_highlight.update` | Update template | `id: number, ...` |
| `json_highlight.delete` | Delete template | `id: number` |

### Schema Registry

| Method | Description | Parameters |
|--------|-------------|------------|
| `schema_registry.config.get` | Get registry config for cluster | `cluster_id: string` |
| `schema_registry.config.save` | Save registry config | `cluster_id: string, registry_url: string, username?: string, password?: string` |
| `schema_registry.config.delete` | Delete registry config | `cluster_id: string` |
| `schema_registry.config.test` | Test registry connection | `registry_url: string, username?: string, password?: string` |
| `schema_registry.subject.list` | List subjects | `cluster_id: string` |
| `schema_registry.version.list` | List versions of a subject | `cluster_id: string, subject: string` |
| `schema_registry.get` | Get schema by subject + version | `cluster_id: string, subject: string, version: number` |
| `schema_registry.get_latest` | Get latest schema | `cluster_id: string, subject: string` |
| `schema_registry.register` | Register schema | `cluster_id: string, subject: string, schema_json: string, schema_type: string` |
| `schema_registry.compatibility.test` | Test compatibility | `cluster_id: string, subject: string, schema_json: string, version?: number` |
| `schema_registry.compatibility.get` | Get compatibility level | `cluster_id: string, subject: string` |
| `schema_registry.compatibility.set` | Set compatibility level | `cluster_id: string, subject: string, compatibility_level: string` |
| `schema_registry.list` | List all schemas (summary) | `cluster_id: string` |
| `schema_registry.delete` | Delete subject | `cluster_id: string, subject: string` |

### Telemetry & Feedback

| Method | Description | Parameters |
|--------|-------------|------------|
| `telemetry.check_connection` | Check telemetry connectivity | None |
| `telemetry.report` | Trigger telemetry report | None |
| `telemetry.submit_feedback` | Submit user feedback | `feedback_content: string` |

---

## Desktop Shell Commands

These commands are exposed by the Tauri shell (`src-tauri`) directly, independent of the unified dispatcher:

| Command | Description |
|---------|-------------|
| `get_app_version` | Application version (from Cargo.toml) |
| `check_for_updates` | Check for new releases |
| `install_update` | Download and install an update |
| `get_download_status` / `clear_download_status` / `set_auto_download_flag` | Update download state |
| `get_app_logs` / `clear_app_logs` | Application log access |
| `set_auto_launch` / `get_auto_launch` | Launch at login (Windows registry) |
| `set_system_tray` | Enable/disable system tray icon |
| `is_windows` | Platform check |
| `share_current_version` | Share version info |
| `open_url` | Open URL in system browser |

---

## Usage Examples (Frontend)

```ts
import { apiClient } from '@/api/client';

// Create a cluster
const cluster = await apiClient.createCluster({
  name: 'prod',
  brokers: 'broker1:9092,broker2:9092',
});

// List topics
const topics = await apiClient.getTopics('prod');

// Send a message
await apiClient.sendMessage('prod', 'events', {
  key: 'user-123',
  value: JSON.stringify({ event: 'user_login', userId: 123 }),
});

// Streaming message query with cancellation
const handle = apiClient.getMessagesStream('prod', 'events',
  { max_messages: 1000, fetchMode: 'newest' },
  {
    onBatch: (messages, progress, total) => render(messages),
    onComplete: () => console.log('done'),
    onError: (err) => console.error(err),
  });
// ... later: handle.abort();

// Reset a consumer group offset to latest
await apiClient.resetConsumerGroupOffset('prod', 'my-group', 'events', 0, 'latest');
```

## Error Handling

- `api_request` rejects with a plain error message string; `apiClient` rethrows it as `{ message, status }`.
- Unknown method names return `Unknown method: <name>`.
- Streaming errors are delivered as an `error` event (`{ error: string }`), not as a rejection.

## Notes

1. **No HTTP layer**: there is no REST server, no port, no CORS — the frontend talks to Rust exclusively over Tauri IPC.
2. **Parameter naming**: `snake_case`; timestamps are Unix milliseconds.
3. **Streaming timeout**: streaming message queries are cancelled server-side after 120 s.
4. **Background import**: `settings.import` returns immediately and runs in the background; concurrent import/export operations are rejected.
5. **Source of truth**: the method dispatch table lives in `src/api.rs` (`dispatch_request`); stream events in `src/api.rs` (`start_message_list_stream`); shell commands in `src-tauri/src/lib.rs` and `src-tauri/src/api_commands.rs`.
