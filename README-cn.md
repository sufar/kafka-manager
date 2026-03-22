# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/tauri-2.x-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[English](README.md)** | **[中文文档](README-cn.md)**

一个强大的 Kafka 集群管理工具，提供 Web API 和跨平台桌面应用程序。轻松管理多个 Kafka 集群、浏览主题、探索消息和管理收藏。

## 快速开始

### 方式一：运行 Web 后端

```bash
# 克隆仓库
git clone <repo-url>
cd kafka-manager

# 运行后端服务
cargo run
```

后端服务将启动在 `http://localhost:9732`

然后启动前端开发服务器：

```bash
cd ui
npm install
npm run dev
```

前端将启动在 `http://localhost:9733`

### 方式二：运行桌面应用

```bash
# 安装前端依赖
cd ui
npm install

# 开发模式运行
npm run tauri dev

# 或构建生产版本
npm run tauri build
```

## 主要功能

### 核心功能

| 模块 | 描述 |
|------|------|
| **多集群管理** | 管理多个 Kafka 集群，支持分组管理，实时连接状态和健康监控 |
| **主题管理** | 创建、删除、配置主题，支持分区管理、批量操作和模板功能 |
| **消息浏览** | 浏览、搜索、过滤消息，支持时间范围查询、SSE 流式传输和导出 (JSON/CSV/TXT) |
| **集群监控** | Broker 信息、分区分布、水位信息、吞吐量统计 |
| **收藏管理** | 收藏常用 Topic，支持分组管理、备注和排序 |
| **多语言支持** | 中英文界面切换 |
| **数据持久化** | SQLite 数据库存储集群配置、用户设置、收藏和审计日志 |

### UI 特性

- 现代化响应式界面，基于 Vue 3 + Tailwind CSS 4 + DaisyUI 5
- 实时集群连接状态显示
- 左侧树形集群导航，支持分组管理
- 基于 Tauri 2 的跨平台桌面应用

## 技术栈

### 后端

| 技术 | 用途 |
|------|------|
| [Axum](https://github.com/tokio-rs/axum) | Web 框架 |
| [Tokio](https://tokio.rs/) | 异步运行时 |
| [SQLx](https://github.com/launchbadge/sqlx) | 异步数据库 (SQLite) |
| [rdkafka](https://github.com/fede1024/rust-rdkafka) | Kafka 客户端 |
| [deadpool](https://github.com/bikeshedder/deadpool) | 连接池 |
| [Moka](https://github.com/moka-rs/moka) | 缓存 |

### 前端

| 技术 | 用途 |
|------|------|
| [Vue 3](https://vuejs.org/) + TypeScript | 前端框架 |
| [Tailwind CSS 4](https://tailwindcss.com/) | CSS 框架 |
| [DaisyUI 5](https://daisyui.com/) | 组件库 |
| [Pinia](https://pinia.vuejs.org/) | 状态管理 |
| [Tauri 2](https://tauri.app/) | 桌面应用框架 |

## 配置

创建 `config.toml` 进行服务器配置：

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

### 环境变量

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `API_KEYS` | 逗号分隔的 API Keys | - |
| `AUTH_ENABLED` | 启用认证 | `false` |
| `HEALTH_CHECK_INTERVAL_SECS` | 健康检查间隔 | `30` |

## 数据库

首次运行时会自动创建 SQLite 数据库 `kafka_manager.db`。

### 核心表

| 表名 | 描述 |
|------|------|
| `kafka_clusters` | 集群配置 |
| `cluster_group` | 集群分组 |
| `cluster_connection_history` | 连接历史 |
| `topic_metadata` | Topic 元数据缓存 |
| `user_settings` | 用户设置（语言、侧边栏模式、选中分组等） |
| `favorites` | Topic 收藏 |
| `favorite_groups` | 收藏分组 |
| `audit_logs` | 审计日志 |
| `topic_templates` | Topic 模板 |

## API 文档

详细 API 文档请参考：
- [API 参考](./docs/api.md) - 完整 API 文档（英文）
- [API 参考](./docs/api-cn.md) - 完整 API 文档（中文）
- [架构设计](./docs/architecture.md) - 技术架构和设计（英文）
- [架构设计](./docs/architecture-cn.md) - 技术架构和设计（中文）

## 开发

### 构建

```bash
cargo build --release
```

### 测试

```bash
cargo test
```

### 代码检查和格式化

```bash
cargo clippy
cargo fmt
```

## 文档

- [API 参考](./docs/api.md) - 完整 API 文档（英文）
- [API 参考](./docs/api-cn.md) - 完整 API 文档（中文）
- [架构设计](./docs/architecture.md) - 技术架构和设计（英文）
- [架构设计](./docs/architecture-cn.md) - 技术架构和设计（中文）

## License

MIT License
