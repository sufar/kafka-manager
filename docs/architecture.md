# Kafka Manager Architecture Design

For Chinese version, see [architecture-cn.md](./architecture-cn.md).

## System Overview

Kafka Manager is a **Tauri 2 desktop application** for managing Kafka clusters. It consists of three layers:

- **Vue 3 frontend** (`ui/`) — the UI, running in the Tauri webview
- **Tauri shell** (`src-tauri/`, crate `kafka-manager`) — desktop integration: window, updater, tray, auto-launch, and the IPC command surface
- **Core library** (`src/`, crate `kafka-manager-api`) — all business logic: Kafka clients, SQLite persistence, and the unified API dispatcher

There is **no HTTP server**. The frontend talks to Rust exclusively over **Tauri IPC** (`invoke` / `Channel`).

## Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                     Kafka Manager Desktop App                       │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Vue 3 Frontend (ui/)                        │  │
│  │  ┌───────────┐  ┌────────────┐  ┌─────────────────────────┐  │  │
│  │  │   Views   │  │ Components │  │  apiClient (client.ts)  │  │  │
│  │  │           │  │            │  │  invoke + Channel       │  │  │
│  │  └───────────┘  └────────────┘  └─────────────────────────┘  │  │
│  └──────────────────────────────┬───────────────────────────────┘  │
│                                 │ Tauri IPC                        │
│  ┌──────────────────────────────▼───────────────────────────────┐  │
│  │              Tauri Shell (src-tauri/, crate kafka-manager)    │  │
│  │  api_commands.rs:  api_request                                │  │
│  │                    message_list_stream (Channel)              │  │
│  │                    cancel_message_list                        │  │
│  │  lib.rs:           updater, tray, auto-launch, logs, window   │  │
│  └──────────────────────────────┬───────────────────────────────┘  │
│                                 │ direct Rust calls                │
│  ┌──────────────────────────────▼───────────────────────────────┐  │
│  │           Core Library (src/, crate kafka-manager-api)        │  │
│  │  ┌────────────┐  ┌─────────────┐  ┌────────────────────────┐ │  │
│  │  │ Dispatcher │  │ Kafka layer │  │  DB layer (sqlx/SQLite)│ │  │
│  │  │  api.rs    │  │  src/kafka/ │  │  src/db/               │ │  │
│  │  └────────────┘  └─────────────┘  └────────────────────────┘ │  │
│  └──────────────────────────────┬───────────────────────────────┘  │
└─────────────────────────────────┼───────────────────────────────────┘
                                  ▼
                        Kafka Clusters (rdkafka)
```

## Process & Threading Model

The app is a single process with two async execution contexts:

1. **Tauri main runtime** — runs the webview and all `#[tauri::command]` handlers. Commands are thin wrappers; they clone the shared `AppState` and call into the core library.
2. **Backend thread + dedicated tokio runtime** — created at startup (`run()` in `src-tauri/src/lib.rs`). It initializes the SQLite pool and `AppState`, hands the state back to the main thread via an mpsc channel, then stays alive forever (`std::future::pending`). The DB pool's background tasks, Kafka client threads (rdkafka), connection pools, and the telemetry loop all live on this runtime.

Startup sequence:

```
run()
 ├─ spawn backend thread ──► tokio runtime
 │                            ├─ load config.toml (optional)
 │                            ├─ open + init SQLite (OS data dir)
 │                            ├─ build AppState ──► send to main thread
 │                            ├─ spawn: Kafka client creation (background)
 │                            └─ spawn: telemetry loop (hourly)
 ├─ recv AppState (≤ 30 s)
 ├─ app.manage(BackendState(state)), app.manage(StreamRegistry)
 └─ Tauri event loop
```

If the database fails to open, the UI still starts; business commands return `"Backend not initialized"` instead of crashing.

### Shared State (`AppState`, `src/lib.rs`)

| Field | Type | Purpose |
|-------|------|---------|
| `db` | `DbPool` (sqlx SQLite, WAL) | All persistence |
| `clients` | `Arc<ArcSwap<KafkaClients>>` | Lock-free swappable per-cluster Kafka client registry |
| `pools` | `ClusterPools` | deadpool-based consumer/producer pools per cluster |
| `config` | `Config` | Loaded from optional `config.toml` |
| `refresh_state` | `Arc<Mutex<RefreshState>>` | Dedup concurrent topic/group refreshes |
| `import_export_lock` | `Arc<Mutex<ImportExportLock>>` | One import/export at a time |

## IPC Design

### Unified Dispatcher

Just like the former HTTP API, business operations are RPC-style and go through **one** command:

```rust
// src-tauri/src/api_commands.rs
#[tauri::command]
async fn api_request(state: State<'_, BackendState>, method: String, params: Value)
    -> Result<Value, String>
{
    api::dispatch_request(&method, state.get()?, params)
        .await
        .map_err(|e| e.to_message())
}
```

`dispatch_request` (`src/api.rs`) matches ~113 method names (`cluster.list`, `topic.create`, `message.send`, …) to handlers of shape `(AppState, Value) -> Result<Value>`. Handlers extract typed parameters with `get_string_param` / `get_optional_*` helpers, so adding a method is a one-line `match` arm plus a handler.

- **Success**: Promise resolves with the result `Value` directly (no envelope).
- **Failure**: Promise rejects with an error message string; `AppError::to_message()` keeps messages identical to the old HTTP API.

### Streaming (Message Query)

Tauri commands can't return streams, so `message.list` has a dedicated streaming command using `tauri::ipc::Channel`:

```
frontend                         src-tauri                      core library
   │                                │                               │
   │── message_list_stream ────────►│                               │
   │   (request_id, params, Channel)│── start_message_list_stream ─►│
   │                                │   (returns mpsc::Receiver)    │── spawn producer
   │                                │                               │   (per-partition
   │◄── Channel: start/batch/… ─────│◄── StreamEvent ───────────────│   consumers,
   │                                │   forward loop                │   heap merge)
   │── cancel_message_list ────────►│── CancellationToken.cancel() ─►│
```

- Events: `start` → `batch`* → `order`? → `complete` | `error`; each is a `StreamEvent { event, data }` where `data` is a JSON string.
- The command future stays alive until the stream ends, so the frontend's `invoke()` Promise resolution signals end-of-stream.
- **Cancellation**: `cancel_message_list(request_id)` cancels a token stored in a `StreamRegistry`; producers check it between batches. If the frontend channel closes (window closed), the forward loop cancels the token too.
- **Timeout**: a 120 s guard cancels the token server-side.
- On the frontend, `apiClient.getMessagesStream()` wraps this in callbacks and returns a `StreamHandle` with `abort()`.

### Desktop Shell Commands

Registered alongside the business commands in `src-tauri/src/lib.rs`: `get_app_version`, `check_for_updates`, `install_update`, download-state commands, `get_app_logs` / `clear_app_logs`, `set_auto_launch` / `get_auto_launch`, `set_system_tray`, `is_windows`, `share_current_version`, `open_url`. The updater supports resumable downloads and portable-zip self-update, and emits `update-available` events to the frontend.

## Core Library Layout (`src/`)

```
src/
├── lib.rs                    # AppState, crate root
├── api.rs                    # Unified dispatcher + all business handlers (~113 methods)
│                             # + streaming message producer (heap-merge, cancellation)
├── api_import_export.rs      # Settings/data import & export (background import)
├── api_schema_registry.rs    # Schema Registry handlers
├── config.rs                 # config.toml loading, defaults
├── error.rs                  # AppError (thiserror) + to_message()
├── db/                       # SQLite layer (sqlx)
│   ├── mod.rs                # DbPool, schema creation (inline CREATE TABLE)
│   ├── cluster.rs            # ClusterStore
│   ├── cluster_group.rs      # ClusterGroupStore
│   ├── topic.rs              # TopicStore (metadata cache)
│   ├── consumer_group.rs     # Consumer group metadata/offsets
│   ├── favorite.rs           # Favorite groups + items
│   ├── topic_history.rs      # Recently viewed topics
│   ├── sent_message.rs       # Sent message history
│   ├── json_highlight.rs     # JSON highlight templates
│   ├── topic_template.rs     # Topic creation templates
│   ├── settings.rs           # Key-value user settings
│   └── schema_registry.rs    # Registry configs + cached schemas
├── kafka/                    # rdkafka wrappers
│   ├── mod.rs                # KafkaClients registry, client config, connectivity test
│   ├── admin.rs              # Topic CRUD, configs, partitions
│   ├── consumer.rs           # Message fetching (cursor + streaming variants)
│   ├── consumer_group.rs     # Group listing, offsets, reset
│   ├── producer.rs           # Message sending
│   ├── offset.rs             # Offset/watermark management
│   ├── throughput.rs         # Throughput estimation
│   ├── avro.rs / protobuf.rs # Payload codecs
│   ├── schema_registry_client.rs # reqwest-based Registry REST client
│   ├── transaction.rs        # Transaction support
│   └── import_export.rs      # Cluster data import/export helpers
├── pool/                     # deadpool-based consumer/producer pools
├── models/                   # Shared data models
├── telemetry/                # Optional MySQL usage reporting (hourly)
└── utils.rs                  # Log path helpers
```

## Database

SQLite (sqlx, WAL mode, 20 connections) at the OS data dir (`Kafka Manager/kafka_manager.db`). Schema is created idempotently at startup via inline `CREATE TABLE IF NOT EXISTS` — no migration framework.

Tables: `kafka_clusters`, `cluster_groups`, `topic_metadata`, `consumer_group_metadata`, `consumer_group_topics`, `consumer_group_offsets`, `topic_templates`, `user_settings`, `json_highlight_templates`, `favorite_groups`, `favorite_items`, `topic_history`, `sent_messages`, `schema_registry_configs`, `schemas`, `telemetry_records`, `resource_tags` (unused).

Topic and consumer-group lists are **cached in SQLite** (`topic_metadata`, `consumer_group_*`) and refreshed on demand (`topic.refresh`, `consumer_group.refresh`), so navigation is instant even with unreachable clusters.

## Frontend Layout (`ui/`)

Vue 3 + TypeScript + Vite + Tailwind 4/DaisyUI 5 + Pinia + vue-router + vue-virtual-scroller.

```
ui/src/
├── api/client.ts           # apiClient: typed wrapper over invoke/Channel (sole IPC user)
├── views/                  # 8 pages (clusters, topics, messages, consumer groups, …)
├── components/             # MessageQueryTool, TopicNavigator, ClusterTreeNavigator, …
├── layouts/                # ModernLayout (main)
├── stores/                 # cluster, clusterConnection, theme, language, update
├── i18n/translations.ts    # zh / en
└── tour/                   # Guided tour overlay
```

Key patterns:

- **No HTTP**: `apiClient` is the only module calling `invoke`; components never do.
- **Virtual scrolling** for all large lists (topics, messages, consumer groups).
- **Streaming rendering**: message batches go into a non-reactive `pendingMessages` buffer and are flushed to the reactive array on a timer (batched UI updates).
- **i18n & theming** persisted via `settings.*` methods with `localStorage` fallback.

## Message Query Design

- Per-partition `max_messages` limit with independent consumer `group.id` per partition
- Serial/parallel mode (auto-selected by cluster latency), bounded by a semaphore (max 10 parallel partitions)
- Timestamp-based offset positioning; watermark boundary checks before clamping
- Min-heap merge across partitions for ordered streaming
- `fetchMode: oldest|newest`, key/value search (`search_in: key|value|all`)
- Cooperative cancellation via `CancellationToken` checked between batches

## Performance

### Backend

| Optimization | Description |
|--------------|-------------|
| Connection pooling | deadpool consumer/producer pools per cluster |
| SQLite caching | Topic/group metadata cached; refresh on demand |
| Lock-free clients | `ArcSwap` for the client registry (no read locking) |
| Parallel partitions | Semaphore-bounded parallel partition fetching |
| Lazy Kafka connect | Clients connect in background at startup |

### Frontend

| Optimization | Description |
|--------------|-------------|
| Virtual scrolling | Handles tens of thousands of rows |
| Non-reactive buffer | Incoming message batches bypass Vue reactivity until flush |
| Batched UI flush | Message list updates on a timer, not per event |
| Channel streaming | No serialization overhead of HTTP/SSE |

## Deployment

```bash
# Development (frontend dev server + Tauri with hot reload)
./start-tauri-dev.sh        # or: cd ui && npm run tauri dev

# Production build
cd ui && npm run tauri build
```

The production binary is self-contained: the core library is statically linked into the Tauri app, and the database lives in the OS data directory. Optional `config.toml` next to the executable can pre-configure clusters and pool sizes.

## Telemetry & Updates

- **Telemetry**: optional hourly usage report to a MySQL endpoint (`src/telemetry/`); silently disabled when unreachable. User feedback goes through `telemetry.submit_feedback`.
- **Updates**: the shell checks GitHub releases at startup and hourly, downloads in the background (resumable), emits `update-available`, and self-installs on confirm.
