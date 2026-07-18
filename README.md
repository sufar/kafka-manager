# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/tauri-2.x-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[中文文档](README-cn.md)** | **[English](README.md)**

A cross-platform desktop application for querying and managing Kafka clusters. Designed for developers and data engineers who need fast, intuitive access to Kafka messages, consumer groups, and cluster internals — without the command line.

![Kafka Manager](img/about.png)

## Features

### Message Query & Browsing

- Query messages across any partition with newest-first or oldest-first ordering
- Keyword search through message values with real-time filtering
- Time range filtering — pinpoint messages by start/end timestamp or use quick presets (5 min, 15 min, 1 hour, 1 day)
- Real-time streaming message query with progressive rendering and stop control (pushed over Tauri IPC Channel)
- Multi-format message viewer: **JSON** (with syntax highlighting and customizable themes), **raw text**, **hex dump**
- Adjustable column widths and timestamp sorting in the message list
- Export query results to **JSON**, **CSV**, or **TXT**
- Send messages to any topic with custom partition and key selection
- Sent message history with replay capability

### Cluster Management

- Multi-cluster support with group organization and horizontal scrollable group selector
- Tree-style navigator: clusters → topics → partitions in a collapsible hierarchy
- Real-time connection health monitoring with status indicators
- One-click reconnect / disconnect per cluster
- Broker information display
- Browsing history — quickly revisit recently viewed topics

### Topic Management

- Create topics with configurable partitions, replication factor, and retention policies
- Delete topics with confirmation
- Partition detail view with leader and replica distribution
- Topic templates for one-click recurring creation
- Topic favorites with named groups
- Topic change history tracking
- Topic-level consumer group overview

### Consumer Groups

- Browse all consumer groups with state and member counts
- Drill into per-partition details: start offset, end offset, committed offset, and lag
- Last commit time tracking per partition
- Reset consumer group offsets to **earliest**, **latest**, or a **specific timestamp**
- Delete consumer groups
- View consumer groups scoped to a specific topic

### Schema Registry

- Connect to Schema Registry per cluster
- Browse and inspect Avro and Protobuf schemas
- View schema versions and details

### Desktop Experience

- Automatic updates with resume support and progress display
- System tray with background running — stays out of your way
- Single-instance enforcement
- Application log viewer
- Data import/export for settings migration between machines
- Dark / Light theme toggle
- Chinese / English bilingual interface (中英文双语)
- Guided tour for first-time users

## Quick Start

Prerequisites: Install [Tauri dependencies](https://tauri.app/start/prerequisites/)

```bash
# Clone the repository
git clone <repo-url>
cd kafka-manager

# Install frontend dependencies
cd ui && npm install

# Development mode (hot-reload)
npm run tauri dev
# or from the repo root: ./start-tauri-dev.sh

# Production build
npm run build
npm run tauri build
```

Built installers will be in `src-tauri/target/release/bundle/`.

## Configuration

Clusters are configured and managed directly from the UI at runtime. An optional `config.toml` next to the executable can pre-configure clusters and connection-pool sizes.

## Architecture

The app is a single self-contained Tauri 2 desktop application — there is **no HTTP server**. The Vue frontend talks to the Rust core exclusively over **Tauri IPC**:

- **Unified dispatcher**: all business operations go through one `api_request` command (~113 methods: clusters, topics, consumer groups, messages, schema registry, favorites, settings, …)
- **Streaming**: message queries push events over a Tauri `Channel` with cooperative cancellation (`cancel_message_list`)
- **Persistence**: SQLite (sqlx, WAL) in the OS data directory; topic/consumer-group metadata is cached locally for instant navigation

See the docs for details:

- [Architecture Design](docs/architecture.md) ([中文](docs/architecture-cn.md))
- [IPC API Reference](docs/api.md) ([中文](docs/api-cn.md))

## Tech Stack

### Backend

| Technology | Purpose |
|------------|---------|
| [Rust](https://www.rust-lang.org/) | Core language |
| [Tauri 2](https://tauri.app/) | Desktop shell & IPC bridge |
| [Tokio](https://tokio.rs/) | Async runtime |
| [SQLx](https://github.com/launchbadge/sqlx) 0.8 | Async SQLite for local persistence |
| [rdkafka](https://github.com/fede1024/rust-rdkafka) 0.39 | Kafka client |
| [deadpool](https://github.com/bikeshedder/deadpool) 0.12 | Connection pooling |
| [apache-avro](https://github.com/apache/avro) 0.17 | Avro encoding / decoding |
| [prost](https://github.com/tokio-rs/prost) 0.12 | Protobuf encoding / decoding |

### Frontend

| Technology | Purpose |
|------------|---------|
| [Vue 3](https://vuejs.org/) + TypeScript | UI framework |
| [Tailwind CSS 4](https://tailwindcss.com/) | Utility-first styling |
| [DaisyUI 5](https://daisyui.com/) | Component library |
| [Pinia](https://pinia.vuejs.org/) | State management |
| [vue-virtual-scroller](https://github.com/Akryum/vue-virtual-scroller) | Large-list rendering |
| [Vite](https://vitejs.dev/) 7 | Build tool |

## Development

```bash
# Build frontend
cd ui && npm run build

# Build workspace (core library + Tauri shell)
cargo build --release

# Run tests
cargo test

# Lint and format
cargo clippy
cargo fmt
```

## License

MIT License
