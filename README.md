# Kafka Manager

使用 Rust + Axum 实现的 Kafka 管理工具后端。

## 功能

| 模块 | 功能 |
|------|------|
| **集群管理** | 多集群配置、连接监控 |
| **Topic 管理** | CRUD、批量操作、自动同步 |
| **消息管理** | 发送、浏览、搜索、导出 |
| **Consumer Group** | 消费进度、Offset 重置 |
| **Schema Registry** | Confluent Schema Registry 集成 |
| **RBAC** | 用户、角色、API Key 认证 |
| **监控告警** | Consumer Lag、速率告警、通知渠道 |
| **审计日志** | 自动记录 API 操作 |

## 快速开始

```bash
cargo run
```

首次运行自动创建 SQLite 数据库 `kafka_manager.db`。

## 配置

```toml
[server]
host = "127.0.0.1"
port = 3000
```

## API 文档

完整 API 文档请查看 [API.md](./API.md)。

## 项目结构

```
src/
├── main.rs          # 入口
├── config.rs        # 配置
├── pool/            # 连接池
├── db/              # 数据库
├── kafka/           # Kafka 客户端
├── routes/          # API 路由
├── middleware/      # 中间件
└── models/          # 数据模型
```

## 数据库表

| 表名 | 描述 |
|------|------|
| `kafka_clusters` | 集群配置 |
| `topic_metadata` | Topic 元数据 |
| `users`, `roles` | 用户和角色 |
| `audit_logs` | 审计日志 |

## 测试

```bash
cargo test
```
