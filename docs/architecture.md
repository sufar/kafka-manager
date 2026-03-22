# Kafka Manager Architecture Design

## System Overview

Kafka Manager is a full-stack Kafka cluster management application with a Rust backend and Vue 3 frontend, packaged as a cross-platform desktop app using Tauri 2.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Kafka Manager Desktop App                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Vue 3 Frontend                         │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │   │
│  │  │   Views     │  │ Components  │  │  API Client     │   │   │
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
│  │ Middleware  │  │   Routes    │  │   Service Layer         │ │
│  │  - Audit    │  │  - Cluster  │  │  - Kafka Admin          │ │
│  │             │  │  - Topic    │  │  - Consumer/Producer    │ │
│  │             │  │  - Message  │  │  - Throughput Stats     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│           │              │                    │                 │
│           ▼              ▼                    ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   SQLite    │  │   Cache     │  │   Kafka Clusters        │ │
│  │  (SQLx)     │  │  (Moka)     │  │   - Brokers             │ │
│  │             │  │             │  │   - Topics              │ │
│  │             │  │             │  │   - Partition Metadata  │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Backend Architecture

### Directory Structure

```
src/
├── main.rs                 # Application entry point, server setup
├── config.rs               # Configuration loading (config.toml, env vars)
├── error.rs                # Error types and conversion
├── lib.rs                  # API route registration
├── cache/
│   └── mod.rs              # MetadataCache configuration
├── db/                     # Database layer
│   ├── mod.rs              # Database initialization, table creation
│   ├── cluster.rs          # Cluster CRUD operations
│   ├── cluster_group.rs    # Cluster group management
│   ├── cluster_connection.rs  # Connection status management
│   ├── topic.rs            # Topic metadata sync and storage
│   ├── topic_template.rs   # Topic template management
│   ├── settings.rs         # User settings
│   ├── favorite.rs         # Topic favorites
│   ├── favorite_group.rs   # Favorite group management
│   └── audit_log.rs        # Audit log recording and query
├── kafka/                  # Kafka client wrapper
│   ├── mod.rs              # Module exports
│   ├── admin.rs            # Admin client (Topic CRUD, cluster metadata)
│   ├── consumer.rs         # Consumer (streaming query support)
│   ├── producer.rs         # Producer sending messages
│   ├── offset.rs           # Offset and watermark management
│   ├── throughput.rs       # Throughput statistics
│   ├── transaction.rs      # Transaction support (Kafka 2.8+)
│   └── import_export.rs    # Data import/export tools
├── pool/                   # Connection pool
│   ├── mod.rs              # KafkaClients connection pool management
│   ├── kafka_consumer.rs   # Consumer connection pool
│   └── kafka_producer.rs   # Producer connection pool
├── middleware/             # HTTP middleware
│   ├── mod.rs              # Middleware registration
│   └── audit.rs            # Audit log recording
├── routes/                 # API route handlers
│   ├── mod.rs              # Route registration
│   ├── unified.rs          # Unified API handler (all API requests)
│   ├── cluster.rs          # Cluster CRUD endpoints (deprecated, kept for compatibility)
│   ├── cluster_group.rs    # Cluster group endpoints (deprecated, kept for compatibility)
│   ├── cluster_connection.rs   # Connection management endpoints (deprecated)
│   ├── topic.rs            # Topic management endpoints (deprecated)
│   ├── topic_template.rs   # Topic template endpoints (deprecated)
│   ├── message.rs          # Message endpoints (deprecated)
│   ├── settings.rs         # Settings endpoints (deprecated)
│   ├── favorite.rs         # Favorite management endpoints (deprecated)
│   ├── audit_log.rs        # Audit log query (deprecated)
│   ├── health.rs           # Health check endpoint (deprecated)
│   ├── user.rs             # User management (unused)
│   └── tag.rs              # Tag management (unused)
└── models/                 # Data models
    └── mod.rs              # Request/Response models
```

### Core Design Patterns

#### 1. Unified API Pattern

All API requests use a unified POST endpoint, with operations distinguished via the `X-API-Method` header:

```rust
// POST /api with X-API-Method header
match method.as_str() {
    "cluster.list" => handle_cluster_list(state).await,
    "topic.create" => handle_topic_create(state, params).await,
    "message.list" => handle_message_list(state, params).await,
    // ...
}
```

**Advantages:**
- Single endpoint for all operations
- Consistent error handling
- Easy to extend with new methods

#### 2. Connection Pooling

Producer and Consumer connections use custom connection pooling:

```rust
pub struct ClusterPools {
    /// Cluster ID -> (Consumer Pool, Producer Pool)
    pools: Arc<tokio::sync::RwLock<HashMap<String, (KafkaConsumerPool, KafkaProducerPool)>>,
}
```

**Advantages:**
- Avoids connection creation overhead
- Automatic cleanup of idle connections
- Per-cluster isolation
- Generic pool implementation based on deadpool

#### 3. SSE Streaming

Server-Sent Events for real-time message streaming:

```rust
async fn fetch_messages_streaming_sse(
    tx: mpsc::Sender<Result<Event, Infallible>>,
    // ...
) {
    // Parallel partition reading
    // Min-heap merge and sort
    // Real-time batch sending via channel
}
```

**Advantages:**
- Progressive UI updates
- Reduced perceived latency
- Better memory efficiency

#### 4. Middleware Chain

Axum middleware stack handles cross-cutting concerns:

```rust
let app = Router::new()
    .merge(routes)
    .layer(middleware::from_fn(audit_middleware))      // Audit log recording
    .layer(TimeoutLayer::new(Duration::from_secs(300))) // 5 minute timeout
    .layer(CompressionLayer::new()                     // Gzip/Brotli compression
        .gzip(true)
        .br(true))
    .layer(CorsLayer::permissive())                    // CORS support
    .layer(TraceLayer::new_for_http());                // Request tracing
```

**Middleware Description:**
- `audit_middleware`: Records all API requests to audit log

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
    group_id INTEGER REFERENCES cluster_groups(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Cluster groups
CREATE TABLE cluster_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Connection history
CREATE TABLE cluster_connection_history (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT,
    checked_at TEXT NOT NULL
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

-- User settings
CREATE TABLE user_settings (
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(key)
);

-- Topic favorite groups
CREATE TABLE favorite_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Topic favorites
CREATE TABLE favorites (
    id INTEGER PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    topic_name TEXT NOT NULL,
    group_id INTEGER REFERENCES favorite_groups(id),
    remark TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(cluster_id, topic_name)
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

-- Topic templates
CREATE TABLE topic_templates (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    num_partitions INTEGER NOT NULL,
    replication_factor INTEGER NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

## Frontend Architecture

### Directory Structure

```
ui/
├── src/
│   ├── api/
│   │   ├── client.ts       # API client (SSE support)
│   │   └── types.ts        # TypeScript type definitions
│   ├── components/         # Reusable components
│   │   ├── MessageQueryTool.vue    # Message query tool
│   │   ├── ClusterTreeNavigator.vue # Cluster tree navigation
│   │   ├── TopicNavigator.vue      # Topic navigation
│   │   ├── FavoriteButton.vue      # Favorite button
│   │   └── ...
│   ├── views/              # Page components
│   │   ├── ClustersView.vue        # Cluster management
│   │   ├── TopicsView.vue          # Topic management
│   │   ├── MessagesView.vue        # Message browser
│   │   ├── FavoritesView.vue       # Favorites
│   │   ├── SettingsView.vue        # Settings
│   │   └── ...
│   ├── layouts/            # Layout components
│   │   └── MainLayout.vue  # Main layout
│   ├── stores/             # Pinia state management
│   │   ├── cluster.ts      # Cluster state
│   │   ├── language.ts     # Multi-language
│   │   └── ...
│   ├── i18n/               # Internationalization
│   │   └── translations.ts # Chinese/English translations
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

Uses Pinia for reactive state management:

```typescript
// stores/cluster.ts
export const useClusterStore = defineStore('cluster', {
  state: () => ({
    clusters: [] as Cluster[],
    groups: [] as Group[],
    clusterHealth: {} as Record<string, HealthStatus>,
  }),
  actions: {
    async fetchClusters() {
      const res = await apiClient.getClusterList();
      this.clusters = res.clusters;
    },
    async fetchGroups() {
      const res = await apiClient.getGroupList();
      this.groups = res.groups;
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
  // Cross-chunk buffer handling
  // AbortController for cancellation
}
```

### Internationalization

```typescript
// i18n/translations.ts
export const translations: Record<Language, Translation> = {
  zh: {
    nav: {
      clusters: '集群',
      topics: '主题',
      messages: '消息',
    },
    // ...
  },
  en: {
    nav: {
      clusters: 'Clusters',
      topics: 'Topics',
      messages: 'Messages',
    },
    // ...
  },
};
```

## API Design

### Unified POST API

All operations use `POST /api`, distinguished by `X-API-Method` header:

| Category | Methods |
|----------|---------|
| **Health** | `health` |
| **Cluster** | `cluster.list`, `cluster.get`, `cluster.create`, `cluster.update`, `cluster.delete`, `cluster.test`, `cluster.test_config`, `cluster.stats` |
| **Cluster Group** | `cluster_group.list`, `cluster_group.get`, `cluster_group.create`, `cluster_group.update`, `cluster_group.delete`, `cluster_group.clusters`, `cluster_group.assign_cluster` |
| **Connection** | `connection.list`, `connection.get`, `connection.disconnect`, `connection.reconnect`, `connection.health_check`, `connection.metrics`, `connection.history`, `connection.stats`, `connection.batch_disconnect`, `connection.batch_reconnect` |
| **Topic** | `topic.list`, `topic.list_with_cluster`, `topic.get`, `topic.create`, `topic.delete`, `topic.delete_all`, `topic.batch_create`, `topic.batch_delete`, `topic.offsets`, `topic.config_get`, `topic.config_alter`, `topic.partitions_add`, `topic.partition.watermarks`, `topic.throughput`, `topic.refresh`, `topic.saved`, `topic.search`, `topic.count`, `topic.cleanup_orphans` |
| **Message** | `message.list`, `message.send`, `message.export` |
| **Settings** | `settings.get`, `settings.update` |
| **Topic Template** | `template.list`, `template.get`, `template.create`, `template.update`, `template.delete`, `template.presets`, `template.create_topic` |
| **Favorite** | `favorite.group.list`, `favorite.group.create`, `favorite.group.get`, `favorite.group.update`, `favorite.group.delete`, `favorite.list`, `favorite.create`, `favorite.get`, `favorite.update`, `favorite.delete`, `favorite.check`, `favorite.delete_by_topic` |
| **Audit Log** | `audit_log.list` |

### Response Format

Success response:
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

Message query feature supports the following:

- Per-partition max_messages limit
- Serial/parallel mode selection (automatically chosen based on local/remote cluster)
- Timestamp-based offset positioning
- Watermark boundary handling
- SSE streaming with min-heap merge sort
- Two-phase query (collect then filter)
- Concurrency control (max 10 parallel partitions)

## Performance Optimization

### Backend

| Optimization | Description |
|--------------|-------------|
| Connection Pooling | Custom connection pool for Producer/Consumer, per-cluster management |
| Query Cache | MetadataCache for hot data like topic lists |
| Async I/O | Fully async architecture based on Tokio |
| Batch Operations | Support for batch create/delete topics |
| Partition Parallelism | Message query supports parallel/serial mode selection |
| Unique group.id | Independent consumer group.id per partition |
| Explicit seek | Seek after assign for offset stability |
| Watermark check | Boundary validation before clamp() |
| Timeout control | 5-minute timeout limit per request |
| Compressed transport | Gzip and Brotli compression support |

### Frontend

| Optimization | Description |
|--------------|-------------|
| Virtual Scrolling | vue-virtual-scroller for large data lists |
| shallowRef | Shallow reactivity for message arrays |
| Throttled updates | Batch UI updates every 200ms |
| Non-reactive buffer | pendingMessages uses plain array |
| SSE streaming | Progressive rendering |

## Deployment

### Development Environment

```bash
# Backend
cargo run

# Frontend (development mode)
cd ui && npm run tauri dev
```

### Production Environment

```bash
# Build backend
cargo build --release

# Build desktop app
cd ui && npm run tauri build
```

### Configuration

Configure via `config.toml` and environment variables:

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

### Health Check

- Scheduled cluster health checks (configurable interval)
- Connection history tracked in `cluster_connection_history` table
- Real-time status via `connection.health_check` API

### Metrics

- Request duration tracking in audit logs
- Slow query detection (>5s requests logged as warnings)
- Connection pool metrics via `connection.metrics` API

## Security

### Audit

- All API requests recorded to `audit_logs` table
- Includes method, path, cluster, resource, action, status, duration, client IP
- Pagination query and historical trace support
