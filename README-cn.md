# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/tauri-2.x-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[English](README.md)** | **[中文文档](README-cn.md)**

跨平台桌面应用与 RESTful API，用于管理 Kafka 集群。浏览主题、探索消息、管理消费者组、监控集群健康状态。

![Kafka Manager](img/about.png)

## 主要功能

### 集群管理

- 多集群支持，支持分组组织
- 实时连接健康状态监控
- 集群重连与断开连接
- Broker 信息展示

### 主题管理

- 创建、删除、配置主题
- 分区详情与分布视图
- 主题模板快速创建
- 主题标签与收藏分组
- 主题变更历史追踪

### 消息浏览

- 跨分区浏览与搜索消息
- 时间范围过滤（最近 5 分钟至 1 天）
- SSE 实时流式获取消息
- 多种查看格式：JSON（带语法高亮）、原始文本、十六进制
- 消息导出（JSON/CSV/TXT）
- 向主题发送消息，支持自定义 Key 和分区

### 消费者组

- 查看消费者组状态与成员详情
- 检查每个分区的已提交偏移和积压
- 重置偏移到最早、最新或指定时间戳
- 主题级消费者组视图

### Schema Registry

- 连接和配置 Schema Registry
- 浏览和管理 Avro/Protobuf 格式的消息

### 桌面应用特性

- 自动更新，支持断点续传和进度显示
- 系统托盘后台运行
- 单实例模式
- 应用日志查看器
- 数据导入/导出（设置迁移）
- 深色/浅色主题切换
- 中英文双语界面

## 快速开始

### 方式一：桌面应用（推荐）

前置条件：安装 [Tauri 依赖](https://tauri.app/start/prerequisites/)

```bash
# 克隆仓库
git clone <repo-url>
cd kafka-manager

# 安装前端依赖
cd ui && npm install

# 开发模式
npm run tauri dev

# 生产构建
cd ui && npm run build
cd .. && npm run tauri build
```

构建的安装包位于 `src-tauri/target/release/bundle/`。

### 方式二：Web API + 前端开发服务器

```bash
# 启动后端服务（端口 9732）
cargo run

# 在另一个终端中启动前端（端口 9733）
cd ui && npm install && npm run dev
```

在浏览器中打开 `http://localhost:9733`。

## 配置

创建 `config.toml` 进行服务器设置和多集群配置：

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

### 环境变量

前缀：`KAFKA_MANAGER__`（双下划线表示嵌套键）

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `KAFKA_MANAGER__SERVER__PORT` | 服务端口 | `9732` |
| `HEALTH_CHECK_INTERVAL_SECS` | 健康检查间隔 | `30` |

## 数据库

使用 SQLite 进行本地数据持久化，首次运行时自动创建数据库。

| 表名 | 用途 |
|------|------|
| `kafka_clusters` | 集群配置 |
| `cluster_groups` | 集群分组 |
| `topic_metadata` | 主题元数据缓存 |
| `consumer_group_metadata` | 消费者组元数据缓存 |
| `consumer_group_offsets` | 消费者组偏移缓存 |
| `favorites` / `favorite_groups` | 主题收藏及分组 |
| `topic_templates` | 主题创建模板 |
| `json_highlight_templates` | JSON 语法高亮主题 |
| `resource_tags` | 资源标签 |
| `topic_history` | 主题变更历史 |
| `sent_message_history` | 发送消息记录 |
| `user_settings` | 用户设置（语言、主题等） |
| `schema_registry_configs` | Schema Registry 连接配置 |

## 技术栈

### 后端

| 技术 | 用途 |
|------|------|
| [Axum](https://github.com/tokio-rs/axum) 0.7 | Web 框架 |
| [Tokio](https://tokio.rs/) | 异步运行时 |
| [SQLx](https://github.com/launchbadge/sqlx) 0.8 | 异步 SQLite |
| [rdkafka](https://github.com/fede1024/rust-rdkafka) 0.39 | Kafka 客户端 |
| [deadpool](https://github.com/bikeshedder/deadpool) 0.12 | 连接池 |
| [Moka](https://github.com/moka-rs/moka) 0.12 | 内存缓存 |
| [apache-avro](https://github.com/apache/avro) 0.17 | Avro 编解码 |
| [prost](https://github.com/tokio-rs/prost) 0.12 | Protobuf 编解码 |
| [tracing](https://github.com/tokio-rs/tracing) | 结构化日志 |
| [arc-swap](https://github.com/vorner/arc-swap) | 无锁状态更新 |

### 前端

| 技术 | 用途 |
|------|------|
| [Vue 3](https://vuejs.org/) + TypeScript | 前端框架 |
| [Tailwind CSS 4](https://tailwindcss.com/) | 样式 |
| [DaisyUI 5](https://daisyui.com/) | 组件库 |
| [Pinia](https://pinia.vuejs.org/) | 状态管理 |
| [vue-router](https://router.vuejs.org/) 5 | 客户端路由 |
| [Chart.js](https://www.chartjs.org/) 4 | 数据可视化 |
| [Tauri 2](https://tauri.app/) | 桌面壳 |
| [Vite](https://vitejs.dev/) 7 | 构建工具 |

## 开发

```bash
# 构建前端
cd ui && npm run build

# 构建后端
cargo build --release

# 运行测试
cargo test

# 代码检查和格式化
cargo clippy
cargo fmt
```

## License

MIT License
