# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/tauri-2.x-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[中文文档](README-cn.md)** | **[English](README.md)**

A powerful Kafka cluster management tool with Web API and cross-platform desktop application. Easily manage multiple Kafka clusters, browse topics, explore messages, and manage favorites.

![Kafka Manager](img/about.png)

## Quick Start

### Option 1: Run Web Backend

```bash
# Clone the repository
git clone <repo-url>
cd kafka-manager

# Run the backend service
cargo run
```

The backend server will start on `http://localhost:9732`

Then start the frontend development server:

```bash
cd ui
npm install
npm run dev
```

The frontend will start on `http://localhost:9733`

### Option 2: Run Desktop Application

Prerequisites: Install [Tauri dependencies](https://tauri.app/start/prerequisites/) first.

```bash
# Install frontend dependencies
cd ui
npm install

# Run in development mode
npm run tauri dev

# Or build production version
cd ui
npm run build
cd ..
npm run tauri build
```

## Features

### Core Features

| Module | Description |
|--------|-------------|
| **Multi-Cluster Management** | Manage multiple Kafka clusters with group support, real-time connection status and health monitoring |
| **Topic Management** | Create, delete, configure topics with partition management, batch operations and template support |
| **Message Browser** | Browse, search, filter messages with time range queries, SSE streaming and export (JSON/CSV/TXT) |
| **Cluster Monitoring** | Broker information, partition distribution, watermark info, throughput statistics |
| **Favorite Management** | Favorite frequently used topics with group management, remarks and sorting |
| **Multi-Language Support** | Switch between Chinese and English interfaces |
| **Data Persistence** | SQLite database for cluster configs, user settings, favorites and audit logs |

### UI Features

- Modern responsive interface based on Vue 3 + Tailwind CSS 4 + DaisyUI 5
- Real-time cluster connection status display
- Left sidebar tree navigation with group support
- Cross-platform desktop app based on Tauri 2

## Tech Stack

### Backend

| Technology | Purpose |
|------------|---------|
| [Axum](https://github.com/tokio-rs/axum) | Web framework |
| [Tokio](https://tokio.rs/) | Async runtime |
| [SQLx](https://github.com/launchbadge/sqlx) | Async database (SQLite) |
| [rdkafka](https://github.com/fede1024/rust-rdkafka) | Kafka client |
| [deadpool](https://github.com/bikeshedder/deadpool) | Connection pool |
| [Moka](https://github.com/moka-rs/moka) | Cache |

### Frontend

| Technology | Purpose |
|------------|---------|
| [Vue 3](https://vuejs.org/) + TypeScript | Frontend framework |
| [Tailwind CSS 4](https://tailwindcss.com/) | CSS framework |
| [DaisyUI 5](https://daisyui.com/) | Component library |
| [Pinia](https://pinia.vuejs.org/) | State management |
| [Tauri 2](https://tauri.app/) | Desktop application framework |

## Configuration

Create `config.toml` for server configuration:

```toml
[server]
host = "0.0.0.0"
port = 9732

[database]
path = "kafka_manager.db"

[pool]
max_connections = 10
min_idle = 2
connection_timeout_sec = 30
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `HEALTH_CHECK_INTERVAL_SECS` | Health check interval | `30` |

## Database

SQLite database `kafka_manager.db` is created automatically on first run.

### Core Tables

| Table Name | Description |
|------------|-------------|
| `kafka_clusters` | Cluster configuration |
| `cluster_group` | Cluster groups |
| `cluster_connection_history` | Connection history |
| `topic_metadata` | Topic metadata cache |
| `user_settings` | User settings (language, sidebar mode, selected group, etc.) |
| `favorites` | Topic favorites |
| `favorite_groups` | Favorite groups |
| `audit_logs` | Audit logs |
| `topic_templates` | Topic templates |

## API Documentation

For detailed API documentation:
- [API Reference](./docs/api.md) - Complete API documentation (English)
- [API Reference](./docs/api-cn.md) - Complete API documentation (Chinese)
- [Architecture Design](./docs/architecture.md) - Technical architecture and design (English)
- [Architecture Design](./docs/architecture-cn.md) - Technical architecture and design (Chinese)

### Development

```bash
# Build frontend
cd ui && npm run build

# Build backend
cargo build --release
```

### Test

```bash
cargo test
```

### Code Check and Format

```bash
cargo clippy
cargo fmt
```

## License

MIT License
