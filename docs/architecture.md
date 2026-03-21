# Kafka Manager Architecture

## System Overview

Kafka Manager is a full-stack Kafka cluster management application built with Rust backend and Vue 3 frontend, packaged as a cross-platform desktop app using Tauri 2.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Kafka Manager Desktop App                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Vue 3 Frontend                         │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │   │
│  │  │   Views     │  │ Components  │  │   API Client    │   │   │
│  │  │             │  │             │  │  (SSE Support)  │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              │ HTTP / SSE                        │
└──────────────────────────────┼───────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Rust Backend (Axum)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  Middleware │  │   Routes    │  │   Service Layer         │ │
│  │  - Auth     │  │  - Cluster  │  │  - Kafka Admin          │ │
│  │  - Audit    │  │  - Topic    │  │  - Consumer/Producer    │ │
│  │  - Rate     │  │  - Message  │  │  - Throughput           │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│           │              │                    │                 │
│           ▼              ▼                    ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   SQLite    │  │   Cache     │  │   Kafka Cluster(s)      │ │
│  │  (SQLx)     │  │   (Moka)    │  │   - Brokers             │ │
│  │             │  │             │  │   - Topics              │ │
│  │             │  │             │  │   - Consumer Groups     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Backend Architecture

### Directory Structure

```
src/
├── main.rs                 # Application entry point, server setup
├── config.rs               # Configuration loading (config.toml, env vars)
├── error.rs                # Error types and conversions
├── lib.rs                  # API routing registration
├── cache/
│   └── mod.rs              # Moka cache configuration
├── db/                     # Database layer
│   ├── mod.rs              # DB initialization, table creation
│   ├── cluster.rs          # Cluster CRUD operations
│   ├── cluster_connection.rs  # Connection status management
│   ├── topic.rs            # Topic metadata sync and storage
│   ├── topic_template.rs   # Topic template management
│   ├── user.rs             # User and role management
│   ├── api_key.rs          # API Key storage and validation
│   ├── audit_log.rs        # Audit log recording and queries
│   ├── notification.rs     # Notification configuration
│   ├── tag.rs              # Resource tagging
│   └── settings.rs         # User settings
├── kafka/                  # Kafka client wrappers
│   ├── admin.rs            # Admin client (topic CRUD, cluster metadata)
│   ├── consumer.rs         # Consumer with streaming support
│   ├── producer.rs         # Producer for sending messages
│   ├── offset.rs           # Offset and watermark management
│   ├── throughput.rs       # Throughput statistics
│   ├── schema.rs           # Local schema management
│   ├── schema_registry.rs  # Schema Registry HTTP client
│   ├── transaction.rs      # Transaction support (Kafka 2.8+)
│   └── import_export.rs    # Data import/export utilities
├── pool/                   # Connection pooling
│   ├── mod.rs              # KafkaClients pool management
│   ├── kafka_consumer.rs   # Consumer connection pool
│   └── kafka_producer.rs   # Producer connection pool
├── middleware/             # HTTP middleware
│   ├── mod.rs              # Middleware registration
│   ├── auth.rs             # API Key auth, RBAC checks
│   ├── audit.rs            # Audit log recording
│   ├── performance.rs      # Request timing and metrics
│   └── rate_limit.rs       # Rate limiting with governor
├── routes/                 # API route handlers
│   ├── cluster.rs          # Cluster CRUD endpoints
│   ├── cluster_connection.rs   # Connection management endpoints
│   ├── cluster_stats.rs    # Cluster statistics
│   ├── cluster_monitor.rs  # Cluster monitoring (brokers, metrics)
│   ├── topic.rs            # Topic management endpoints
│   ├── consumer_group.rs   # Consumer group endpoints
│   ├── message.rs          # Message browsing and sending
│   ├── schema.rs           # Schema Registry endpoints
│   ├── notification.rs     # Notification configuration
│   ├── user.rs             # User and role endpoints
│   ├── auth.rs             # API Key management
│   ├── audit_log.rs        # Audit log queries
│   ├── tag.rs              # Resource tagging endpoints
│   ├── topic_template.rs   # Topic template endpoints
│   ├── settings.rs         # Settings endpoints
│   ├── health.rs           # Health check endpoint
│   └── unified.rs          # Unified API handler (legacy compatibility)
├── task/                   # Background tasks
│   ├── mod.rs              # Task management
│   └── health_check.rs     # Periodic health checks
└── models/                 # Data models
    └── mod.rs              # Request/Response models
```

### Key Design Patterns

#### 1. Unified API Pattern

All API requests use a unified POST endpoint with method discrimination:

```rust
// POST /api with X-API-Method header
match method.as_str() {
    "cluster.list" => handle_cluster_list(state).await,
    "topic.create" => handle_topic_create(state, params).await,
    "message.list" => handle_message_list(state, params).await,
    // ...
}
```

**Benefits:**
- Single endpoint for all operations
- Consistent error handling
- Easy to extend with new methods

#### 2. Connection Pooling

Producer and Consumer connections are pooled using deadpool:

```rust
pub struct KafkaClients {
    producers: Arc<MokaPool<ProducerClient>>,
    consumers: Arc<MokaPool<ConsumerClient>>,
}
```

**Benefits:**
- Avoid connection creation overhead
- Automatic idle cleanup
- Per-cluster isolation

#### 3. SSE Streaming

Server-Sent Events for real-time message streaming:

```rust
async fn fetch_messages_streaming_sse(
    tx: mpsc::Sender<Result<Event, Infallible>>,
    // ...
) {
    // Parallel partition reads
    // Min-heap merge sort
    // Real-time batch sending via channel
}
```

**Benefits:**
- Progressive UI updates
- Lower perceived latency
- Better memory efficiency

#### 4. Middleware Chain

Axum middleware stack for cross-cutting concerns:

```rust
let app = Router::new()
    .merge(routes)
    .layer(middleware::from_fn(auth_middleware))
    .layer(middleware::from_fn(audit_middleware))
    .layer(middleware::from_fn(performance_middleware))
    .layer(ConcurrencyLimitLayer::new(100));
```

### Database Schema

#### Core Tables

```sql
-- Cluster configuration
CREATE TABLE kafka_clusters (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    brokers TEXT NOT NULL,
    request_timeout_ms INTEGER DEFAULT 30000,
    operation_timeout_ms INTEGER DEFAULT 30000,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Topic metadata cache
CREATE TABLE topic_metadata (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    partition_count INTEGER NOT NULL,
    replication_factor INTEGER NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    fetched_at TEXT NOT NULL,
    UNIQUE(cluster_id, topic_name)
);

-- User and roles
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role_id INTEGER REFERENCES roles(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE roles (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    permissions TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Audit logs
CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    cluster_id TEXT,
    resource TEXT,
    action TEXT,
    api_key TEXT,
    status INTEGER,
    duration_ms INTEGER,
    client_ip TEXT
);

-- Notification configurations
CREATE TABLE notification_configs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    config_type TEXT NOT NULL,
    webhook_url TEXT,
    email_recipients TEXT,
    dingtalk_webhook TEXT,
    slack_webhook TEXT,
    wechat_webhook TEXT,
    enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Alert rules
CREATE TABLE alert_rules (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    name TEXT NOT NULL,
    rule_type TEXT NOT NULL,
    topic TEXT,
    consumer_group TEXT,
    threshold REAL NOT NULL,
    comparison TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL,
    severity TEXT NOT NULL,
    notification_config TEXT,
    enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Resource tags
CREATE TABLE resource_tags (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_name TEXT NOT NULL,
    tag_key TEXT NOT NULL,
    tag_value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(cluster_id, resource_type, resource_name, tag_key)
);
```

## Frontend Architecture

### Directory Structure

```
ui/
├── src/
│   ├── api/
│   │   ├── client.ts       # API client with SSE support
│   │   └── types.ts        # TypeScript type definitions
│   ├── components/         # Reusable components
│   │   ├── MessageQueryTool.vue
│   │   ├── ClusterSelector.vue
│   │   └── ...
│   ├── views/              # Page components
│   │   ├── ClustersView.vue
│   │   ├── TopicsView.vue
│   │   ├── MessagesClassicView.vue
│   │   └── ...
│   ├── layouts/            # Layout components
│   │   └── MainLayout.vue
│   ├── stores/             # Pinia state management
│   │   ├── clusters.ts
│   │   ├── topics.ts
│   │   └── messages.ts
│   ├── i18n/               # Internationalization
│   │   └── translations.ts
│   ├── App.vue
│   └── main.ts
├── src-tauri/              # Tauri configuration
│   ├── src/
│   │   └── lib.rs
│   ├── icons/
│   ├── tauri.conf.json
│   └── Cargo.toml
└── package.json
```

### State Management

Pinia stores for reactive state:

```typescript
// stores/clusters.ts
export const useClustersStore = defineStore('clusters', {
  state: () => ({
    clusters: [] as Cluster[],
    selectedClusters: [] as string[],
    connectionStatus: {} as Record<string, ConnectionStatus>,
  }),
  actions: {
    async fetchClusters() {
      const res = await apiClient.getClusterList();
      this.clusters = res.clusters;
    },
    async updateConnectionStatus(clusterId: string) {
      // ...
    },
  },
});
```

### SSE Client

```typescript
// api/client.ts
getMessagesStream(
  clusterId: string,
  topic: string,
  params: MessageQueryParams,
  callbacks: {
    onStart?: (data: StartData) => void;
    onBatch?: (messages: Message[], progress: number, total: number) => void;
    onComplete?: () => void;
    onError?: (error: string) => void;
  }
): AbortController {
  // Fetch + ReadableStream for SSE parsing
  // Buffer handling for cross-chunk data
  // AbortController for cancellation
}
```

## API Design

### Unified POST API

All operations use `POST /api` with `X-API-Method` header:

| Method | Description | Parameters |
|--------|-------------|------------|
| `cluster.list` | List clusters | - |
| `cluster.create` | Create cluster | name, brokers, timeouts |
| `topic.list` | List topics | cluster_id |
| `topic.create` | Create topic | cluster_id, name, partitions, replication |
| `message.list` | Get messages | cluster_id, topic, partition, max_messages |
| `message.send` | Send message | cluster_id, topic, key, value |
| `consumer_group.list` | List consumer groups | cluster_id |

### Response Format

```json
{
  "success": true,
  "data": { ... }
}
```

Error response:
```json
{
  "success": false,
  "error": "Error message"
}
```

## Message Query Design

See [message-query-design.md](./message-query-design.md) for detailed message query architecture including:

- Per-partition max_messages limit
- Serial vs parallel mode selection
- Timestamp-based offset lookup
- Watermark boundary handling
- SSE streaming with min-heap merge sort

## Security

### Authentication

- API Key validation via `X-API-Key` header
- Whitelist paths skip auth (`/api/health`, `/api/ready`)
- Configurable via `API_KEYS` env var

### Authorization

- RBAC with `resource:action` permissions
- Default roles: admin (`*`), operator, viewer
- Permission checks in auth middleware

### Audit

- All API requests logged to `audit_logs` table
- Includes method, path, cluster, resource, action, status, duration

## Performance Optimizations

### Backend

| Optimization | Description |
|--------------|-------------|
| Connection pooling | Producer/Consumer pools with deadpool |
| Query caching | Moka cache for hot data |
| Async I/O | Full async/await with Tokio |
| Bulk operations | Batch create/delete support |
| Unique group.id | Per-partition consumer group IDs |
| Explicit seek | Consumer seek after assign for stable offsets |
| Watermark checks | Boundary validation before clamp() |

### Frontend

| Optimization | Description |
|--------------|-------------|
| Virtual scrolling | vue-virtual-scroller for large lists |
| shallowRef | Shallow reactivity for message arrays |
| Throttled updates | Batch UI updates every 200ms |
| Non-reactive buffers | Pending messages in plain arrays |
| SSE streaming | Progressive rendering |

## Deployment

### Development

```bash
# Backend
cargo run

# Frontend (dev mode)
cd ui && npm run tauri dev
```

### Production

```bash
# Build backend
cargo build --release

# Build desktop app
cd ui && npm run tauri build
```

### Configuration

Server configured via `config.toml` and environment variables:

```toml
[server]
host = "0.0.0.0"
port = 9732

[database]
path = "kafka_manager.db"

[pool]
max_connections = 10
```

## Monitoring

### Health Checks

- Periodic cluster health checks (configurable interval)
- Connection history tracked in `cluster_connection_history` table
- Real-time status available via `connection.health_check` API

### Metrics

- Request duration tracked in audit logs
- Slow query detection (>5s requests logged as warnings)
- Connection pool metrics available via `connection.metrics` API
