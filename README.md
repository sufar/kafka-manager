# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![GPUI](https://img.shields.io/badge/gpui-0.2-blue.svg)](https://gpui.rs/)
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

## Quick Start

Prerequisites: Install [Tauri dependencies](https://tauri.app/start/prerequisites/)

```bash
# Clone the repository
git clone <repo-url>
cd kafka-manager

# Development mode
cargo run -p kafka-manager-app
# or: ./start-dev.sh

# Production build
cargo build --release -p kafka-manager-app
```

The release binary will be at `target/release/kafka-manager-app`.

## Configuration

Clusters are configured and managed directly from the UI at runtime. An optional `config.toml` next to the executable can pre-configure clusters and connection-pool sizes.

## Architecture

Kafka Manager is a pure-Rust desktop application built with [GPUI](https://gpui.rs/) (the GPU-accelerated UI framework from Zed) and [gpui-component](https://github.com/longbridge/gpui-component) — no webview, no JavaScript, no HTTP server:

- **UI**: native GPUI widgets (`app/`), 60+ components from gpui-component (virtualized tables, dialogs, notifications, themes)
- **Core library** (`src/`, crate `kafka-manager-api`): all business logic — Kafka clients (rdkafka), SQLite persistence, and a unified dispatcher (~113 methods) shared by every page
- **Streaming**: message queries run per-partition consumers on a dedicated tokio runtime and stream batches into the UI with cooperative cancellation
- **Persistence**: SQLite (sqlx, WAL) in the OS data directory; topic/consumer-group metadata is cached locally for instant navigation

See the docs for details:

- [Architecture Design](docs/architecture.md) ([中文](docs/architecture-cn.md))
- [API Reference](docs/api.md) ([中文](docs/api-cn.md))

## Tech Stack

| Technology | Purpose |
|------------|---------|
| [Rust](https://www.rust-lang.org/) | Core language |
| [GPUI](https://gpui.rs/) 0.2 | GPU-accelerated native UI framework (from Zed) |
| [gpui-component](https://github.com/longbridge/gpui-component) 0.5 | UI component library |
| [Tokio](https://tokio.rs/) | Async runtime |
| [SQLx](https://github.com/launchbadge/sqlx) 0.8 | Async SQLite for local persistence |
| [rdkafka](https://github.com/fede1024/rust-rdkafka) 0.39 | Kafka client |
| [deadpool](https://github.com/bikeshedder/deadpool) 0.12 | Connection pooling |
| [apache-avro](https://github.com/apache/avro) 0.17 | Avro encoding / decoding |
| [prost](https://github.com/tokio-rs/prost) 0.12 | Protobuf encoding / decoding |
| [tray-icon](https://github.com/tauri-apps/tray-icon) | System tray |
| [rfd](https://github.com/PolyMeilex/rfd) | Native file dialogs |

## Development

```bash
# Build workspace (core library + GPUI app)
cargo build --release

# Run tests
cargo test

# Lint and format
cargo clippy
cargo fmt
```

## License

MIT License
