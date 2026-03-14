---
skill_name: query-message
description: Kafka消息查询功能开发指南 - 包含API端点、请求参数、实现细节和代码示例
tags: [kafka, message, query, api, development]
---

# Kafka消息查询开发指南

## 概述

本项目实现了高性能的Kafka消息查询功能，支持本地/远程集群自动适配策略。

## 关键文件

| 文件 | 说明 |
|------|------|
| `src/routes/unified.rs` | 统一API路由，处理 `message.list` 请求 |
| `src/kafka/consumer.rs` | KafkaConsumer 封装，消息消费逻辑 |
| `ui/src/api/client.ts` | 前端API客户端，`getMessages()` 方法 |
| `ui/src/types/api.ts` | TypeScript类型定义，`MessageRecord` 等 |
| `ui/src/views/MessagesView.vue` | 消息查询界面 |

## API端点

```
POST /api
Header: X-API-Method: message.list
```

## 请求参数

`src/routes/unified.rs`

```rust
let cluster_id = get_string_param(&body, "cluster_id")?;
let topic = get_string_param(&body, "topic")?;
let partition = get_optional_i32_param(&body, "partition");
let offset = get_optional_i64_param(&body, "offset");
let max_messages = get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
let limit = get_optional_i64_param(&body, "limit").map(|v| v as usize);
let start_time = get_optional_i64_param(&body, "start_time");
let end_time = get_optional_i64_param(&body, "end_time");
let search = get_optional_string_param(&body, "search");
let fetch_mode = get_optional_string_param(&body, "fetchMode");
let sort = get_optional_string_param(&body, "sort");
```

## 核心实现

### 1. 自动适配策略入口

`src/routes/unified.rs` - `fetch_messages_with_temp_consumer`

```rust
async fn fetch_messages_with_temp_consumer(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<String>,
    sort: Option<String>,
) -> Result<Vec<KafkaMessage>> {
    // 自动检测本地Kafka
    let is_local = brokers.contains("localhost") ||
                   brokers.contains("127.0.0.1") ||
                   brokers.contains("::1") ||
                   brokers.contains("host.docker.internal");

    if is_local {
        fetch_messages_local(...).await  // 单consumer策略
    } else {
        fetch_messages_remote(...).await // 多consumer并发策略
    }
}
```

### 2. 本地/Docker优化策略

`src/routes/unified.rs` - `fetch_messages_local`

```rust
async fn fetch_messages_local(...) -> Result<Vec<KafkaMessage>> {
    tokio::task::spawn_blocking({
        // 配置：最小延迟
        client_config.set("fetch.min.bytes", "1");
        client_config.set("fetch.wait.max.ms", "10");

        // 单consumer订阅所有分区
        let mut tpl = TopicPartitionList::new();
        for part_id in partitions {
            tpl.add_partition(topic, part_id);
            // 计算起始offset
            let seek_offset = match offset {
                Some(o) => o,
                None => match fetch_mode.as_deref() {
                    Some("oldest") => low,
                    _ => latest.saturating_sub(...), // 最新消息
                },
            };
            tpl.set_partition_offset(topic, part_id, Offset::Offset(seek_offset))?;
        }
        consumer.assign(&tpl)?;

        // 轮询获取消息
        while all_msgs.len() < max_messages * partition_count {
            match consumer.poll(Duration::from_millis(10)) {
                Some(Ok(msg)) => { /* 过滤、检查时间戳、搜索匹配 */ }
                ...
            }
        }

        // 按时间戳排序
        all_msgs.sort_by(|a, b| {
            if is_desc { b.timestamp.cmp(&a.timestamp) }
            else { a.timestamp.cmp(&b.timestamp) }
        });
    }).await
}
```

### 3. 远程集群并发策略

`src/routes/unified.rs` - `fetch_messages_remote`

```rust
async fn fetch_messages_remote(...) -> Result<Vec<KafkaMessage>> {
    // 为每个分区创建独立consumer
    let mut tasks = Vec::new();
    for part_id in partitions {
        let task = tokio::task::spawn_blocking({
            // 配置：优化网络传输
            client_config.set("fetch.min.bytes", "1024");
            client_config.set("fetch.wait.max.ms", "30");

            let consumer: StreamConsumer = client_config.create()?;
            consumer.assign(&tpl)?;
            consumer.seek(topic, part_id, seek_offset, ...)?;

            // 轮询获取消息
            while msgs.len() < max_messages {
                match consumer.poll(Duration::from_millis(30)) { ... }
            }
            msgs
        });
        tasks.push(task);
    }

    // 聚合所有分区结果
    let mut all_msgs = Vec::new();
    for task in tasks {
        if let Ok(msgs) = task.await {
            all_msgs.extend(msgs);
        }
    }

    // 全局排序
    all_msgs.sort_by(...);
}
```

## 前端调用

`ui/src/api/client.ts`

```typescript
async getMessages(clusterId: string, topic: string, params?: {
    partition?: number;
    offset?: number;
    max_messages?: number;
    order_by?: 'timestamp' | 'offset';
    sort?: 'asc' | 'desc';
    search?: string;
    start_time?: number;
    end_time?: number;
    fetchMode?: 'oldest' | 'newest';
}): Promise<MessageRecord[]> {
    const data = await this.request<{ messages: MessageRecord[] }>(
        'message.list',
        { cluster_id: clusterId, topic, ...params },
        60000  // 60秒超时
    );
    return data.messages;
}
```

## 类型定义

`ui/src/types/api.ts`

```typescript
export interface MessageRecord {
    partition: number;
    offset: number;
    key?: string;
    value?: string;
    timestamp?: number;
}

export interface SendMessageRequest {
    partition: number;
    key?: string;
    value: string;
    headers?: Record<string, string>;
}

export interface SendMessageResponse {
    partition: number;
    offset: number;
}
```

## 相关API方法

| Method | 功能 | 所在文件 |
|--------|------|----------|
| `message.list` | 查询消息 | `src/routes/unified.rs` |
| `message.send` | 发送消息 | `src/routes/unified.rs` |
| `message.export` | 导出消息 | `src/routes/unified.rs` |

## 开发注意事项

1. **超时设置**: 消息查询使用60秒超时（普通API为30秒）
2. **取消请求**: 前端支持 `cancelGetMessages()` 取消正在进行的查询
3. **本地检测**: 通过broker地址自动选择查询策略
4. **排序逻辑**: 默认按时间戳降序，可通过 `sort=asc` 改为升序
5. **搜索范围**: `search` 参数同时搜索key和value字段
