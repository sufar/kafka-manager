# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/tauri-2.x-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[中文文档](README-cn.md)** | **[English](README.md)**

A cross-platform desktop application and RESTful API for managing Kafka clusters. Browse topics, explore messages, manage consumer groups, and monitor cluster health with an intuitive interface.

![Kafka Manager](img/about.png)

## Features

### Cluster Management

- Multi-cluster support with group organization
- Real-time connection health monitoring
- Cluster reconnection and disconnection
- Broker information display

### Topics

- Create, delete, and configure topics
- Partition details and distribution view
- Topic templates for quick creation
- Topic tagging and favorites with group management
- Topic change history tracking

### Messages

- Browse and search messages across partitions
- Time range filtering (recent 5min to 1 day)
- SSE real-time streaming message fetch
- Multiple view formats: JSON (with syntax highlighting), raw, hex
- Message export (JSON/CSV/TXT)
- Send messages to topics with custom keys and partitions

### Consumer Groups

- View consumer group state and member details
- Inspect committed offsets and lag per partition
- Reset offsets to earliest, latest, or specific timestamp
- Topic-level consumer group view

### Schema Registry

- Connect and configure Schema Registry
- Browse and manage Avro/Protobuf schemas

### Desktop App Features

- Automatic update with resume support and progress display
- System tray with background running
- Single-instance enforcement
- Application log viewer
- Data import/export for settings migration
- Dark/Light theme toggle
- Chinese/English bilingual interface

## Quick Start

### Option 1: Desktop Application (Recommended)

Prerequisites: [Tauri dependencies](https://tauri.app/start/prerequisites/)

```bash
# Clone the repository
git clone <repo-url>
cd kafka-manager

# Install frontend dependencies
cd ui && npm install

# Development mode
cd ui && npm run tauri dev

# Production build
cd ui && npm run build
npm run tauri build
```

Built installers will be in `src-tauri/target/release/bundle/`.

### Option 2: Web API + Frontend Dev Server

```bash
# Start the backend (port 9732)
cargo run

# In another terminal, start the frontend (port 9733)
cd ui && npm install && npm run dev
```

Open `http://localhost:9733` in your browser.

## Configuration

Create `config.toml` for server settings and multi-cluster configuration:

```toml
[server]
host = "127.0.0.1"
port = 9732

[pool]
max_size = 50
min_size = 5
acquire_timeout_secs = 15
idle_timeout_secs = 300

[kafka.clusters.development]
brokers = "localhost:9092"
request_timeout_ms = 5000
operation_timeout_ms = 5000

[kafka.clusters.production]
brokers = "prod-kafka-1:9092,prod-kafka-2:9092,prod-kafka-3:9092"
request_timeout_ms = 10000
operation_timeout_ms = 10000
```

### Environment Variables

Prefix: `KAFKA_MANAGER__` (double underscore for nested keys)

| Variable | Description | Default |
|----------|-------------|---------|
| `KAFKA_MANAGER__SERVER__PORT` | Server port | `9732` |
| `HEALTH_CHECK_INTERVAL_SECS` | Health check interval | `30` |

## Database

SQLite is used for local data persistence. The database is created automatically on first run.

| Table | Purpose |
|-------|---------|
| `kafka_clusters` | Cluster configurations |
| `cluster_groups` | Cluster group organization |
| `topic_metadata` | Cached topic metadata |
| `consumer_group_metadata` | Cached consumer group info |
| `consumer_group_offsets` | Cached consumer group offsets |
| `favorites` / `favorite_groups` | Topic favorites with grouping |
| `topic_templates` | Reusable topic creation templates |
| `json_highlight_templates` | JSON syntax highlighting themes |
| `resource_tags` | Resource tagging |
| `topic_history` | Topic change history |
| `sent_message_history` | Sent message records |
| `user_settings` | User preferences (language, theme, etc.) |
| `schema_registry_configs` | Schema Registry connection configs |

## Tech Stack

### Backend

| Technology | Purpose |
|------------|---------|
| [Axum](https://github.com/tokio-rs/axum) 0.7 | Web framework |
| [Tokio](https://tokio.rs/) | Async runtime |
| [SQLx](https://github.com/launchbadge/sqlx) 0.8 | Async SQLite |
| [rdkafka](https://github.com/fede1024/rust-rdkafka) 0.39 | Kafka client |
| [deadpool](https://github.com/bikeshedder/deadpool) 0.12 | Connection pooling |
| [Moka](https://github.com/moka-rs/moka) 0.12 | In-memory cache |
| [apache-avro](https://github.com/apache/avro) 0.17 | Avro encoding/decoding |
| [prost](https://github.com/tokio-rs/prost) 0.12 | Protobuf encoding/decoding |
| [tracing](https://github.com/tokio-rs/tracing) | Structured logging |
| [arc-swap](https://github.com/vorner/arc-swap) | Lock-free state updates |

### Frontend

| Technology | Purpose |
|------------|---------|
| [Vue 3](https://vuejs.org/) + TypeScript | UI framework |
| [Tailwind CSS 4](https://tailwindcss.com/) | Styling |
| [DaisyUI 5](https://daisyui.com/) | Component library |
| [Pinia](https://pinia.vuejs.org/) | State management |
| [vue-router](https://router.vuejs.org/) 5 | Client-side routing |
| [Chart.js](https://www.chartjs.org/) 4 | Data visualization |
| [Tauri 2](https://tauri.app/) | Desktop shell |
| [Vite](https://vitejs.dev/) 7 | Build tool |

## Development

```bash
# Build frontend
cd ui && npm run build

# Build backend
cargo build --release

# Run tests
cargo test

# Lint and format
cargo clippy
cargo fmt
```

## License

MIT License
