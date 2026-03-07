# Kafka Manager

[![Rust](https://img.shields.io/badge/rust-v0.1.0-orange.svg)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/tauri-2.x-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

一个功能强大的 Kafka 集群管理工具，提供 Web API 和跨平台桌面应用。支持多集群管理、Topic 运维、消息浏览、Consumer Group 监控等功能。

## 功能特性

### 核心功能

| 模块 | 功能描述 |
|------|------|
| **多集群管理** | 支持管理多个 Kafka 集群，实时连接状态监控，一键断开/重连 |
| **Topic 管理** | 创建/删除/查看 Topic，分区管理，配置修改，批量操作，自动同步元数据 |
| **消息管理** | 浏览消息、搜索过滤、按分区/时间查询、导出 (JSON/CSV/TXT)、发送消息 |
| **Consumer Group** | 查看消费进度、Lag 监控、Offset 重置、消费速率统计 |
| **集群监控** | Broker 信息、分区分布、未完全复制分区监控、吞吐量统计 |
| **Schema Registry** | Confluent Schema Registry 集成，Schema CRUD，兼容性检查 |
| **RBAC 权限** | 用户管理、角色管理、API Key 认证、细粒度权限控制 |
| **告警通知** | Consumer Lag 告警、速率告警，支持 Webhook/钉钉/企业微信/Slack/邮件 |
| **审计日志** | 自动记录所有 API 操作，支持查询和清理 |
| **资源标签** | 为 Topic 和 Consumer Group 添加自定义标签，便于分类管理 |
| **Topic 模板** | 预定义配置模板，一键创建标准化 Topic |

### UI 特性

- **现代化界面** - 使用 Vue 3 + Tailwind CSS 4 + DaisyUI 5 构建
- **多集群视图** - 侧边栏集群列表，支持复选多选
- **实时状态** - 集群连接健康状态实时显示
- **响应式设计** - 适配不同屏幕尺寸
- **跨平台桌面应用** - 基于 Tauri 2，支持 Windows/macOS/Linux

## 快速开始

### 方法一：运行 Web 后端

```bash
# 克隆项目
git clone <repo-url>
cd kafka-manager

# 运行后端
cargo run
```

后端服务将在 `http://localhost:3000` 启动。

### 方法二：运行桌面应用

```bash
# 安装前端依赖
cd ui
npm install

# 开发模式运行
npm run tauri dev

# 或构建发布版本
npm run tauri build
```

### 首次运行

首次运行时自动创建 SQLite 数据库 `kafka_manager.db`。

## 配置

创建 `config.toml` 配置文件：

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

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `API_KEYS` | API Key 列表（逗号分隔） | - |
| `AUTH_ENABLED` | 是否启用认证 | `false` |
| `HEALTH_CHECK_INTERVAL_SECS` | 健康检查间隔 | `30` |

## 项目结构

```
kafka-manager/
├── src/                      # Rust 后端源码
│   ├── main.rs              # 应用入口
│   ├── config.rs            # 配置管理
│   ├── error.rs             # 错误处理
│   ├── lib.rs               # API 路由注册
│   ├── cache/               # 缓存模块
│   ├── db/                  # 数据库层
│   │   ├── cluster.rs       # 集群管理
│   │   ├── topic.rs         # Topic 管理
│   │   ├── user.rs          # 用户和角色
│   │   ├── audit_log.rs     # 审计日志
│   │   └── ...
│   ├── kafka/               # Kafka 客户端封装
│   │   ├── admin.rs         # Admin 客户端
│   │   ├── consumer.rs      # Consumer 封装
│   │   ├── producer.rs      # Producer 封装
│   │   └── throughput.rs    # 吞吐量统计
│   ├── middleware/          # 中间件
│   │   ├── auth.rs          # 认证中间件
│   │   ├── audit.rs         # 审计中间件
│   │   └── performance.rs   # 性能监控
│   ├── pool/                # 连接池管理
│   ├── routes/              # API 路由处理器
│   └── task/                # 后台任务
├── ui/                       # Vue 前端源码
│   ├── src/
│   │   ├── api/             # API 客户端
│   │   ├── views/           # 页面组件
│   │   ├── components/      # 可复用组件
│   │   ├── layouts/         # 布局组件
│   │   └── stores/          # Pinia 状态管理
│   └── src-tauri/           # Tauri 配置
├── config.toml              # 配置文件
├── kafka_manager.db         # SQLite 数据库
└── docs/                    # 文档目录
```

## API 端点

### 统一 API 风格

本项目采用**统一的 POST API**设计，所有请求都通过 `POST /api` 发送：

- **请求 URL**: `POST http://localhost:3000/api`
- **Method Header**: `X-API-Method: <method_name>`
- **请求 Body**: JSON 格式参数

**示例**:
```bash
# 获取集群列表
curl -X POST http://localhost:3000/api \
  -H "X-API-Method: cluster.list" \
  -H "Content-Type: application/json"

# 创建 Topic
curl -X POST http://localhost:3000/api \
  -H "X-API-Method: topic.create" \
  -H "Content-Type: application/json" \
  -d '{
    "cluster_id": "development",
    "name": "test-topic",
    "num_partitions": 3,
    "replication_factor": 1
  }'

# 获取消息列表
curl -X POST http://localhost:3000/api \
  -H "X-API-Method: message.list" \
  -H "Content-Type: application/json" \
  -d '{
    "cluster_id": "development",
    "topic": "test-topic",
    "partition": 0,
    "max_messages": 100
  }'
```

### API Method 列表

#### 集群管理
| Method | 描述 | 参数 |
|--------|------|------|
| `cluster.list` | 获取集群列表 | 无 |
| `cluster.get` | 获取集群详情 | `cluster_id` |
| `cluster.create` | 创建集群 | `name`, `brokers`, `request_timeout_ms`, `operation_timeout_ms` |
| `cluster.update` | 更新集群 | `cluster_id`, `name?`, `brokers?`, `request_timeout_ms?`, `operation_timeout_ms?` |
| `cluster.delete` | 删除集群 | `cluster_id` |
| `cluster.test` | 测试集群连接 | `cluster_id` |

#### Topic 管理
| Method | 描述 | 参数 |
|--------|------|------|
| `topic.list` | 获取 Topic 列表 | `cluster_id` |
| `topic.get` | 获取 Topic 详情 | `cluster_id`, `name` |
| `topic.create` | 创建 Topic | `cluster_id`, `name`, `num_partitions`, `replication_factor`, `config?` |
| `topic.delete` | 删除 Topic | `cluster_id`, `name` |
| `topic.batch_create` | 批量创建 Topic | `cluster_id`, `topics[]`, `continue_on_error` |
| `topic.batch_delete` | 批量删除 Topic | `cluster_id`, `topics[]`, `continue_on_error` |
| `topic.offsets` | 获取 Topic Offset 信息 | `cluster_id`, `name` |
| `topic.config_get` | 获取 Topic 配置 | `cluster_id`, `name` |
| `topic.config_alter` | 修改 Topic 配置 | `cluster_id`, `name`, `config` |
| `topic.partitions_add` | 增加分区数 | `cluster_id`, `name`, `new_partitions` |
| `topic.throughput` | 获取 Topic 吞吐量 | `cluster_id`, `name` |
| `topic.refresh` | 刷新 Topic 列表 | `cluster_id` |
| `topic.saved` | 获取已保存的 Topic | `cluster_id` |
| `topic.search` | 搜索 Topic | `cluster_id`, `search` |
| `topic.count` | 获取 Topic 数量 | `cluster_id` |

#### Consumer Group 管理
| Method | 描述 | 参数 |
|--------|------|------|
| `consumer_group.list` | 获取 Consumer Group 列表 | `cluster_id` |
| `consumer_group.get` | 获取 Consumer Group 详情 | `cluster_id`, `name` |
| `consumer_group.delete` | 删除 Consumer Group | `cluster_id`, `name` |
| `consumer_group.offsets` | 获取 Consumer Group Offset | `cluster_id`, `name` |
| `consumer_group.offsets_reset` | 重置 Consumer Group Offset | `cluster_id`, `name`, `topic`, `partition?`, `offset` |
| `consumer_group.throughput` | 获取 Consumer Group 吞吐量 | `cluster_id`, `name`, `topic` |
| `consumer_group.batch_delete` | 批量删除 Consumer Group | `cluster_id`, `group_names[]`, `continue_on_error` |
| `consumer_group.consumer_offsets` | 获取消费进度详情 | `cluster_id`, `group_name`, `topic?` |
| `consumer_lag.get` | 获取 Topic 消费积压 | `cluster_id`, `topic` |
| `consumer_lag.history` | 获取消费积压历史 | `cluster_id`, `topic`, `start_time?`, `end_time?` |

#### 消息管理
| Method | 描述 | 参数 |
|--------|------|------|
| `message.list` | 获取消息列表 | `cluster_id`, `topic`, `partition?`, `offset?`, `max_messages` |
| `message.send` | 发送消息 | `cluster_id`, `topic`, `key?`, `value`, `partition?` |
| `message.export` | 导出消息 | `cluster_id`, `topic`, `format`, `partition?`, `start_time?`, `end_time?` |

## 开发

### 编译

```bash
cargo build --release
```

### 测试

```bash
cargo test
```

### 检查代码

```bash
cargo clippy
cargo fmt
```

## 技术栈

### 后端
- **Web 框架**: [Axum](https://github.com/tokio-rs/axum) - Tokio 团队的轻量级 Web 框架
- **异步运行时**: [Tokio](https://tokio.rs/)
- **数据库**: [SQLx](https://github.com/launchbadge/sqlx) - 异步 SQL 库，使用 SQLite
- **Kafka 客户端**: [rdkafka](https://github.com/fede1024/rust-rdkafka)
- **连接池**: [deadpool](https://github.com/bikeshedder/deadpool)
- **缓存**: [moka](https://github.com/moka-rs/moka)
- **认证**: [bcrypt](https://crates.io/crates/bcrypt) 密码哈希

### 前端
- **框架**: [Vue 3](https://vuejs.org/) + TypeScript
- **UI 库**: [DaisyUI 5](https://daisyui.com/) + [Tailwind CSS 4](https://tailwindcss.com/)
- **状态管理**: [Pinia](https://pinia.vuejs.org/)
- **桌面应用**: [Tauri 2](https://tauri.app/)

## 数据库表

| 表名 | 描述 |
|------|------|
| `kafka_clusters` | Kafka 集群配置 |
| `cluster_connection_history` | 集群连接历史 |
| `topic_metadata` | Topic 元数据缓存 |
| `users` | 用户账户 |
| `roles` | 角色定义 |
| `api_keys` | API Key 存储 |
| `audit_logs` | API 审计日志 |
| `notification_configs` | 通知渠道配置 |
| `alert_rules` | 告警规则 |
| `consumer_lag_history` | Consumer Lag 历史记录 |
| `resource_tags` | 资源标签 |
| `topic_templates` | Topic 配置模板 |
| `user_settings` | 用户个性化设置 |

## 安全

- **API Key 认证**: 支持多 API Key，请求头 `X-API-Key` 验证
- **RBAC 权限**: 基于角色的访问控制，权限格式 `resource:action`
- **密码加密**: 使用 bcrypt 进行密码哈希
- **审计日志**: 所有 API 操作自动记录

## 性能优化

- **连接池**: Kafka Producer/Consumer 连接池，避免频繁创建连接
- **查询缓存**: 使用 moka 缓存热点数据
- **异步 IO**: 全异步架构，高并发支持
- **批量操作**: 支持批量创建/删除 Topic 和 Consumer Group

## 相关文档

- [API 使用示例](./docs/API_EXAMPLES.md) - 完整 API 使用文档
- [UI 开发指南](./ui/README.md) - 前端开发文档
- [Tauri 应用](./TAURI_README.md) - 桌面应用开发文档
- [Topic 刷新功能](./TOPIC_REFRESH_FEATURE.md) - Topic 同步机制说明

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
