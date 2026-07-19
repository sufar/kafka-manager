# Kafka Manager Architecture Design

For Chinese version, see [architecture-cn.md](./architecture-cn.md).

## System Overview

Kafka Manager is a pure-Rust **GPUI desktop application** for managing Kafka clusters. It consists of two crates:

- **`app/`** (crate `kafka-manager-app`) — the desktop application: UI built with [GPUI](https://gpui.rs/) 0.2 + [gpui-component](https://github.com/longbridge/gpui-component) 0.5, plus desktop integration (tray, auto-launch, updater, single instance, native dialogs)
- **`src/`** (crate `kafka-manager-api`) — the business core library: Kafka clients (rdkafka), SQLite persistence (sqlx), and a unified method dispatcher (~113 methods)

There is **no webview, no JavaScript, no HTTP server, no IPC boundary** — the UI calls the core library directly in-process.

## Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                  Kafka Manager (single process)                     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │              App Crate (app/, kafka-manager-app)              │  │
│  │                                                              │  │
│  │  Pages (pages/)          Workspace        Desktop            │  │
│  │  ├─ clusters.rs          sidebar nav      ├─ tray.rs         │  │
│  │  ├─ topics.rs            page switch      ├─ updater.rs      │  │
│  │  ├─ messages.rs ──┐      i18n (zh/en)     └─ main.rs         │  │
│  │  ├─ consumer_     │      theme (dark/        (single inst.,  │  │
│  │  │  groups.rs     │       light)              auto-launch)   │  │
│  │  ├─ schema_       │                                          │  │
│  │  │  registry.rs   │      service.rs ────────┐                │  │
│  │  ├─ favorites.rs  │      (runtime bridge)   │                │  │
│  │  └─ settings.rs   │                         │                │  │
│  └────────────────────┼─────────────────────────┼────────────────┘  │
│                       │ direct calls            │ tokio JoinHandle  │
│  ┌────────────────────▼─────────────────────────▼────────────────┐  │
│  │           Core Library (src/, kafka-manager-api)              │  │
│  │  api.rs              kafka/              db/                  │  │
│  │  unified dispatcher  rdkafka wrappers    sqlx / SQLite        │  │
│  │  (~113 methods)      admin/consumer/     clusters, topics,    │  │
│  │  + streaming         producer, offsets   favorites, settings  │  │
│  └──────────────────────────────┬────────────────────────────────┘  │
└─────────────────────────────────┼───────────────────────────────────┘
                                  ▼
                        Kafka Clusters (rdkafka)
```

## Process & Threading Model

The app is a single process with two async execution contexts:

1. **GPUI main thread** — runs the window, all page entities, and `cx.spawn` UI tasks. It never touches Kafka or SQLite directly.
2. **Backend thread + dedicated tokio runtime** — created at startup (`app/src/backend.rs`). It initializes the SQLite pool and `AppState`, then stays alive forever (`std::future::pending`). The DB pool's background tasks, rdkafka client threads, connection pools, and the telemetry loop all live on this runtime.

Startup sequence (`app/src/main.rs`):

```
main()
 ├─ single-instance check
 ├─ spawn backend thread ──► tokio runtime
 │                            ├─ load config.toml (optional)
 │                            ├─ open + init SQLite (OS data dir)
 │                            ├─ build AppState ──► send to main thread
 │                            ├─ spawn: Kafka client creation (background)
 │                            └─ spawn: telemetry loop (hourly)
 ├─ recv AppState (≤ 30 s) + tokio runtime Handle
 ├─ read tray setting from SQLite
 └─ gpui Application::run
      ├─ globals: Backend(AppState), TokioRuntime(Handle), I18n, Theme
      ├─ tray icon + menu-event poll task (if enabled)
      ├─ auto update check (3 s delay, notification on new version)
      └─ open window → Workspace (sidebar + pages)
```

If the database fails to open, the UI still starts; business calls return `"Backend not initialized"` instead of crashing.

### The Runtime Bridge (`app/src/service.rs`)

`dispatch_request` is async and internally uses tokio primitives (sqlx, rdkafka), so it must run **on the backend tokio runtime**, not on GPUI's executor. The bridge pattern:

```rust
// Runs the dispatch on the tokio runtime; the JoinHandle is a plain
// Future that GPUI tasks can await on their own executor.
let handle = tokio_handle.spawn(async move {
    api::dispatch_request(method, state, params).await.map_err(|e| e.to_message())
});
let result = handle.await?;  // awaited inside cx.spawn UI tasks
```

### Shared State (`AppState`, `src/lib.rs`)

| Field | Type | Purpose |
|-------|------|---------|
| `db` | `DbPool` (sqlx SQLite, WAL) | All persistence |
| `clients` | `Arc<ArcSwap<KafkaClients>>` | Lock-free swappable per-cluster Kafka client registry |
| `pools` | `ClusterPools` | deadpool-based consumer/producer pools per cluster |
| `config` | `Config` | Loaded from optional `config.toml` |
| `refresh_state` | `Arc<Mutex<RefreshState>>` | Dedup concurrent topic/group refreshes |
| `import_export_lock` | `Arc<Mutex<ImportExportLock>>` | One import/export at a time |

## UI Architecture (`app/`)

### Layout

```
app/
├── Cargo.toml            # gpui 0.2.2, gpui-component 0.5.1, kafka-manager-api, …
├── assets/icon.png       # tray icon
├── locales/              # zh.json / en.json (856 keys, converted from the old Vue i18n)
└── src/
    ├── main.rs           # entry: logging, single instance, backend bootstrap, tray, auto update check
    ├── backend.rs        # AppState bootstrap (DB, Kafka clients, telemetry)
    ├── service.rs        # runtime bridge to dispatch_request
    ├── state.rs          # globals: Backend / TokioRuntime / Page enum
    ├── i18n.rs           # t(cx, "nav.clusters") dotted-key lookup
    ├── workspace.rs      # sidebar navigation + page switching
    ├── tray.rs           # system tray (tray-icon crate), show/quit menu
    ├── updater.rs        # GitHub releases check, download, portable install (ported from the old Tauri shell)
    ├── components/       # shared widgets (StringOption select item, …)
    └── pages/            # one entity per page
        ├── clusters.rs       # cluster CRUD + groups + connection test
        ├── topics.rs         # topic list/create/delete/detail + favorites toggle
        ├── messages.rs       # streaming query + virtual table + detail panel + send + export + history
        ├── consumer_groups.rs# group list + offsets + reset + delete
        ├── schema_registry.rs# registry config + subjects + schema detail
        ├── favorites.rs      # favorite groups & items management
        └── settings.rs       # theme / language / tray / auto-launch / logs / import-export / updates
```

### Page Pattern

Each page is a GPUI entity (`Render` impl) holding its view state. Data flows one way:

```
user action → entity method → service::call (tokio runtime)
           → cx.spawn task → update entity fields → cx.notify() → re-render
```

Key GPUI patterns used throughout:

- **Async**: `cx.spawn(async move |this, cx| { … })` for background work; `this.update(cx, |this, cx| …)` to mutate state after awaiting.
- **Selects/inputs** are entities created with `cx.new(|cx| …)`; dynamic selects are (re)created via the stored `AnyWindowHandle` when async data arrives.
- **Virtual lists**: `uniform_list(id, count, |range, window, cx| …)` renders only visible rows (topics, messages, consumer groups — tens of thousands of rows are fine).
- **Dialogs**: `window.open_dialog(cx, |dialog, …| …)` with `on_ok` callbacks that capture the page entity handle.
- **Notifications**: pushed via `window.push_notification` (helper fans out to all windows from async contexts).

### i18n & Theme

- **i18n**: `app/locales/zh.json` / `en.json` (converted 1:1 from the old Vue `translations.ts`), embedded at compile time; `t(cx, "messages.title")` walks the dotted key. Language is persisted via `settings.update("ui.language", …)`.
- **Theme**: gpui-component `Theme` with light/dark modes, switched via `Theme::change(...)` and persisted via `settings.update("ui.theme", …)`.

## Streaming Message Query

```
MessagesPage                      backend tokio runtime
     │                                    │
     │── start_message_list_stream ──────►│── ensure cluster client
     │   (params, CancellationToken)      │── spawn producer task
     │◄── mpsc::Receiver<StreamEvent>     │   (per-partition consumers,
     │                                    │   min-heap merge, batch send)
     │── UI task: rx.recv() loop          │
     │    append batches → Rc<Vec>        │── cancel_token checked
     │    progress → progress bar         │   between batches
     │── stop button → token.cancel() ───►│
```

- Events: `start` → `batch`* → `order`? → `complete` | `error` (`StreamEvent { event, data }`, `data` is a JSON string).
- A 120 s timeout guard cancels the token automatically.
- Messages accumulate in an `Rc<Vec<Message>>`; the table renders through `uniform_list`, so only visible rows are built per frame.
- The stream ends when the producer task completes (senders drop → `recv()` returns `None`), which also ends the UI read loop.

## Database

SQLite (sqlx, WAL mode, 20 connections) at the OS data dir (`Kafka Manager/kafka_manager.db`). Schema is created idempotently at startup via inline `CREATE TABLE IF NOT EXISTS` — no migration framework.

Tables: `kafka_clusters`, `cluster_groups`, `topic_metadata`, `consumer_group_metadata`, `consumer_group_topics`, `consumer_group_offsets`, `topic_templates`, `user_settings`, `json_highlight_templates`, `favorite_groups`, `favorite_items`, `topic_history`, `sent_messages`, `schema_registry_configs`, `schemas`, `telemetry_records`, `resource_tags` (unused).

Topic and consumer-group lists are **cached in SQLite** (`topic_metadata`, `consumer_group_*`) and refreshed on demand (`topic.refresh`, `consumer_group.refresh`), so navigation is instant even with unreachable clusters.

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

### UI

| Optimization | Description |
|--------------|-------------|
| Virtual lists | `uniform_list` renders only visible rows |
| Rc message buffer | Incoming batches share one `Rc<Vec>` — clones are cheap |
| Native GPU rendering | GPUI draws everything on the GPU; no DOM/webview overhead |
| Streaming without serialization | In-process mpsc, no HTTP/SSE boundary |

## Desktop Integration

| Feature | Implementation |
|---------|----------------|
| System tray | `tray-icon` crate (`app/src/tray.rs`); show/quit menu, close-to-hide when enabled (`ui.system_tray` setting) |
| Auto-launch | `auto-launch` crate (Windows registry / macOS LaunchAgent / Linux .desktop) |
| Single instance | `single-instance` crate, checked before anything else in `main()` |
| Auto update | `app/src/updater.rs` — GitHub releases check (semver compare), background download, portable-zip install + restart (Windows: exe rename + PowerShell swap, ported from the old Tauri shell) |
| File dialogs | `rfd` crate (message export, data import/export) |
| Logging | tracing → `~/.cache/kafka-manager/kafka-manager.log` |

## Deployment

```bash
# Development
./start-dev.sh                  # or: cargo run -p kafka-manager-app

# Production build
cargo build --release -p kafka-manager-app
```

The binary is self-contained: the core library is statically linked, locales are embedded, and the database lives in the OS data directory. Optional `config.toml` next to the executable can pre-configure clusters and pool sizes.

## Telemetry & Updates

- **Telemetry**: optional hourly usage report to a MySQL endpoint (`src/telemetry/`); silently disabled when unreachable. User feedback goes through `telemetry.submit_feedback`.
- **Updates**: the app checks GitHub releases 3 s after startup; if a newer version exists it shows a notification, and the Settings page can download and install portable builds in the background.
