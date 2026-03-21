# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/tauri-2.x-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A powerful Kafka cluster management tool with Web API and cross-platform desktop application. Manage multiple Kafka clusters, browse topics, monitor consumer groups, and explore messages with ease.

![Kafka Manager Dashboard](https://via.placeholder.com/800x400?text=Kafka+Manager+Dashboard)

## Features

### Core Capabilities

| Module | Description |
|--------|-------------|
| **Multi-Cluster Management** | Manage multiple Kafka clusters with real-time connection status and health monitoring |
| **Topic Management** | Create, delete, configure topics with partition management and bulk operations |
| **Message Browser** | Browse, search, filter messages with time-range queries and export (JSON/CSV/TXT) |
| **Consumer Group Monitoring** | Track consumer lag, reset offsets, monitor consumption rates |
| **Cluster Monitoring** | Broker info, partition distribution, under-replicated partitions, throughput stats |
| **Schema Registry** | Confluent Schema Registry integration with CRUD operations |
| **RBAC & Authentication** | User management, role-based access control, API Key authentication |
| **Alerting & Notifications** | Consumer lag alerts, rate alerts via Webhook/DingTalk/Slack/WeChat/Email |
| **Audit Logging** | Automatic API request auditing with query and cleanup support |
| **Resource Tagging** | Custom tags for topics and consumer groups |
| **Topic Templates** | Predefined templates for standardized topic creation |

### UI Highlights

- Modern, responsive interface built with Vue 3 + Tailwind CSS 4 + DaisyUI 5
- Real-time cluster connection status
- Multi-cluster sidebar with checkbox selection
- Cross-platform desktop app powered by Tauri 2

## Quick Start

### Option 1: Run Web Backend

```bash
# Clone repository
git clone <repo-url>
cd kafka-manager

# Run backend server
cargo run
```

Backend will start at `http://localhost:9732`

### Option 2: Run Desktop Application

```bash
# Install frontend dependencies
cd ui
npm install

# Run in development mode
npm run tauri dev

# Or build production release
npm run tauri build
```

### First Run

On first run, SQLite database `kafka_manager.db` is automatically created.

## Configuration

Create `config.toml` for server configuration:

```toml
[server]
host = "0.0.0.0"
port = 3000

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
| `API_KEYS` | Comma-separated API Keys | - |
| `AUTH_ENABLED` | Enable authentication | `false` |
| `HEALTH_CHECK_INTERVAL_SECS` | Health check interval | `30` |

## Architecture

```
kafka-manager/
├── src/                      # Rust backend source
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error handling
│   ├── cache/               # Caching layer (Moka)
│   ├── db/                  # Database layer (SQLx + SQLite)
│   ├── kafka/               # Kafka client wrappers (rdkafka)
│   ├── middleware/          # Auth, audit, rate limiting
│   ├── pool/                # Connection pooling (deadpool)
│   ├── routes/              # API route handlers
│   └── task/                # Background tasks
├── ui/                       # Vue 3 frontend
│   ├── src/
│   │   ├── api/             # API client (SSE support)
│   │   ├── views/           # Page components
│   │   ├── components/      # Reusable components
│   │   └── stores/          # Pinia state management
│   └── src-tauri/           # Tauri configuration
├── config.toml              # Server configuration
├── kafka_manager.db         # SQLite database
└── docs/                    # Documentation
```

## API Endpoints

### Unified API Style

This project uses a **unified POST API** design:

- **URL**: `POST /api`
- **Method Header**: `X-API-Method: <method_name>`
- **Body**: JSON parameters

**Example**:
```bash
# List clusters
curl -X POST http://localhost:9732/api \
  -H "X-API-Method: cluster.list" \
  -H "Content-Type: application/json"

# Create topic
curl -X POST http://localhost:9732/api \
  -H "X-API-Method: topic.create" \
  -H "Content-Type: application/json" \
  -d '{
    "cluster_id": "development",
    "name": "test-topic",
    "num_partitions": 3,
    "replication_factor": 1
  }'

# Get messages with streaming
curl -X POST http://localhost:9732/api \
  -H "X-API-Method: message.list" \
  -H "Content-Type: application/json" \
  -d '{
    "cluster_id": "development",
    "topic": "test-topic",
    "partition": 0,
    "max_messages": 100
  }'
```

### API Method Reference

For complete API documentation, see [docs/api.md](./docs/api.md)

| Category | Methods |
|----------|---------|
| **Cluster** | `cluster.list`, `cluster.get`, `cluster.create`, `cluster.update`, `cluster.delete`, `cluster.test` |
| **Connection** | `connection.list`, `connection.get`, `connection.disconnect`, `connection.reconnect`, `connection.health_check` |
| **Topic** | `topic.list`, `topic.get`, `topic.create`, `topic.delete`, `topic.batch_create`, `topic.batch_delete`, `topic.refresh` |
| **Consumer Group** | `consumer_group.list`, `consumer_group.get`, `consumer_group.delete`, `consumer_group.offsets_reset` |
| **Message** | `message.list`, `message.send`, `message.export` |
| **Monitoring** | `monitor.stats`, `monitor.info`, `monitor.metrics`, `monitor.brokers` |
| **User & Role** | `user.list`, `user.create`, `user.update`, `role.list`, `role.create` |
| **Schema** | `schema.subjects`, `schema.versions`, `schema.get`, `schema.register` |
| **Alert** | `notification.list`, `notification.create`, `alert_history.list` |

## Tech Stack

### Backend

| Technology | Purpose |
|------------|---------|
| [Axum](https://github.com/tokio-rs/axum) | Web framework |
| [Tokio](https://tokio.rs/) | Async runtime |
| [SQLx](https://github.com/launchbadge/sqlx) | Async database (SQLite) |
| [rdkafka](https://github.com/fede1024/rust-rdkafka) | Kafka client |
| [deadpool](https://github.com/bikeshedder/deadpool) | Connection pooling |
| [Moka](https://github.com/moka-rs/moka) | Caching |
| [bcrypt](https://crates.io/crates/bcrypt) | Password hashing |
| [tower-http](https://github.com/tower-rs/tower-http) | Middleware |

### Frontend

| Technology | Purpose |
|------------|---------|
| [Vue 3](https://vuejs.org/) + TypeScript | Frontend framework |
| [Tailwind CSS 4](https://tailwindcss.com/) | CSS framework |
| [DaisyUI 5](https://daisyui.com/) | Component library |
| [Pinia](https://pinia.vuejs.org/) | State management |
| [Tauri 2](https://tauri.app/) | Desktop application |

## Database Tables

| Table | Description |
|-------|-------------|
| `kafka_clusters` | Cluster configuration |
| `cluster_connection_history` | Connection history |
| `topic_metadata` | Topic metadata cache |
| `users`, `roles` | User accounts and roles |
| `api_keys` | API Key storage |
| `audit_logs` | API audit logs |
| `notification_configs` | Notification channels |
| `alert_rules`, `alert_history` | Alert rules and history |
| `consumer_lag_history` | Consumer lag tracking |
| `resource_tags` | Resource tags |
| `topic_templates` | Topic templates |
| `user_settings` | User preferences |

## Security

- **API Key Authentication**: Validate via `X-API-Key` header
- **RBAC**: Role-based access control with `resource:action` permissions
- **Password Hashing**: bcrypt for secure password storage
- **Audit Logging**: All API requests automatically logged

## Performance

- **Connection Pooling**: Producer/Consumer pools avoid connection overhead
- **Query Caching**: Moka cache for hot data
- **Async I/O**: Fully async architecture for high concurrency
- **Bulk Operations**: Batch create/delete for topics and consumer groups
- **SSE Streaming**: Real-time message streaming for large datasets

## Development

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Lint & Format

```bash
cargo clippy
cargo fmt
```

## Documentation

- [API Reference](./docs/api.md) - Complete API documentation
- [Architecture](./docs/architecture.md) - Technical architecture and design
- [Message Query Design](./docs/message-query-design.md) - Message query internals

## License

MIT License

## Contributing

Issues and Pull Requests are welcome!
