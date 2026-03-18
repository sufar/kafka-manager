---
skill_name: query-message
description: Kafka消息查询功能开发指南 - 统一实现，支持并行/串行模式自动切换、空分区提前退出、延迟字符串转换等优化
tags: [kafka, message, query, api, development, unified, parallel]
---

# Kafka消息查询开发指南

## 概述

本项目实现了高性能的统一Kafka消息查询功能，自动根据分区数选择并行或串行模式，包含多项性能优化。

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
let start_time = get_optional_i64_param(&body, "start_time");
let end_time = get_optional_i64_param(&body, "end_time");
let search = get_optional_string_param(&body, "search");
let fetch_mode = get_optional_string_param(&body, "fetchMode");
let sort = get_optional_string_param(&body, "sort");
```

## 核心实现

### 1. 统一消息查询入口

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
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<crate::kafka::consumer::KafkaMessage>> {
    // 获取分区列表
    let partitions: Vec<i32> = { /* 从metadata获取 */ };
    let partition_count = partitions.len();

    // 分区数>1时使用并行模式
    let use_parallel = partition_count > 1;

    // 预计算排序方向（在fetch_mode被move之前）
    let is_desc = sort.as_deref() == Some("desc")
        || (sort.is_none() && fetch_mode.as_deref() != Some("oldest"));

    let messages = if use_parallel {
        // === 并行模式（分区数>1）===
        // 最多10个并发
        let semaphore = Arc::new(Semaphore::new(10));
        let mut handles = vec![];

        for &part_id in &partitions {
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await;
                timeout(Duration::from_secs(30), tokio::task::spawn_blocking(move || {
                    fetch_partition_messages_unified(
                        brokers, topic, part_id, msgs_per_partition, part_offset,
                        start_time, end_time, search, fetch_mode,
                    )
                })).await
            });
            handles.push(handle);
        }

        // 收集结果
        let mut all_msgs = Vec::new();
        for handle in handles { /* 聚合消息 */ }
        all_msgs
    } else {
        // === 单分区串行模式 ===
        tokio::task::spawn_blocking(move || {
            fetch_partition_messages_unified(
                brokers, topic, 0, max_messages, offset,
                start_time, end_time, search, fetch_mode,
            )
        }).await?
    };

    // 排序
    all_msgs.sort_by(|a, b| { /* 时间戳/offset排序 */ });
    Ok(all_msgs)
}
```

### 2. 单分区消息获取（统一实现）

`src/routes/unified.rs` - `fetch_partition_messages_unified`

```rust
fn fetch_partition_messages_unified(
    brokers: String,
    topic: String,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<String>,
) -> Vec<crate::kafka::consumer::KafkaMessage> {
    // 创建consumer
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers)
        .set("group.id", "kafka-mgr-unified")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "3000")
        .set("heartbeat.interval.ms", "500");

    // 批量fetch配置优化
    if max_messages > 1000 {
        cfg.set("fetch.min.bytes", "65536");        // 64KB
        cfg.set("fetch.wait.max.ms", "100");
        cfg.set("fetch.max.bytes", "52428800");     // 50MB
    } else {
        cfg.set("fetch.min.bytes", "1");
        cfg.set("fetch.wait.max.ms", "10");
    }

    let consumer: BaseConsumer = cfg.create()?;

    // 计算时间范围offset
    let time_range = calculate_partition_offset(...)?;
    let start_offset = time_range.start_offset;
    let time_range_end = time_range.end_offset;

    // 优化1: 提前退出 - 如果分区没有数据
    if start_offset >= time_range_end && time_range_end >= 0 {
        tracing::info!("[Unified Partition] Partition {} has no data, skipping", partition);
        return Vec::new();
    }
    if time_range.high_watermark <= time_range.low_watermark {
        tracing::info!("[Unified Partition] Partition {} is empty, skipping", partition);
        return Vec::new();
    }

    // 分配到指定分区
    let mut tpl = TopicPartitionList::new();
    tpl.add_partition_offset(&topic, partition, seek_offset)?;
    consumer.assign(&tpl)?;

    // Consumer预热（关键优化）
    let warmup_start = std::time::Instant::now();
    while warmup_start.elapsed() < Duration::from_millis(1000) {
        match consumer.poll(Duration::from_millis(100)) {
            Some(Ok(_)) => {
                consumer.seek(&topic, partition, seek_offset, Duration::from_secs(1))?;
                break;
            }
            _ => continue,
        }
    }

    // 延迟字符串转换
    struct RawMessage {
        partition: i32,
        offset: i64,
        key_bytes: Option<Vec<u8>>,
        value_bytes: Option<Vec<u8>>,
        timestamp: Option<i64>,
    }

    let mut raw_messages: Vec<RawMessage> = Vec::with_capacity(max_messages);
    let mut empty_count = 0;

    // 优化2: 空轮询150ms一次，最多10次（1.5秒）
    const POLL_TIMEOUT_MS: u64 = 150;
    const MAX_EMPTY_POLLS: usize = 10;
    const MAX_POLL_TIME_SECS: u64 = 30;

    let poll_start = std::time::Instant::now();
    let mut got_first = false;

    while raw_messages.len() < max_messages
        && empty_count < MAX_EMPTY_POLLS
        && poll_start.elapsed() < Duration::from_secs(MAX_POLL_TIME_SECS)
    {
        match consumer.poll(Duration::from_millis(POLL_TIMEOUT_MS)) {
            Some(Ok(msg)) => {
                if !got_first {
                    got_first = true;
                    tracing::info!("First message received after {:?}", poll_start.elapsed());
                }
                empty_count = 0;

                // 优化3: 检查是否超过结束offset，提前退出
                if let Some(end) = end_offset {
                    if msg.offset() >= end {
                        break;
                    }
                }

                // 延迟转换：只保存字节
                let key_bytes = msg.key().map(|k| k.to_vec());
                let value_bytes = msg.payload().map(|p| p.to_vec());

                // 优化4: 如果需要搜索，立即过滤（避免保存不需要的消息）
                if need_search {
                    let key_str = key_bytes.as_ref().and_then(|k| std::str::from_utf8(k).ok());
                    let value_str = value_bytes.as_ref().and_then(|v| std::str::from_utf8(v).ok());
                    let matches = key_str.map_or(false, |k| k.to_lowercase().contains(term))
                        || value_str.map_or(false, |v| v.to_lowercase().contains(term));
                    if !matches {
                        continue;  // 跳过不匹配的消息
                    }
                }

                raw_messages.push(RawMessage {
                    partition,
                    offset: msg.offset(),
                    key_bytes,
                    value_bytes,
                    timestamp: msg.timestamp().to_millis(),
                });
            }
            Some(Err(_)) => {}
            None => {
                empty_count += 1;  // 空轮询计数
            }
        }
    }

    // 转换为最终消息格式（延迟字符串转换）
    let messages: Vec<KafkaMessage> = raw_messages
        .into_iter()
        .map(|raw| KafkaMessage {
            partition: raw.partition,
            offset: raw.offset,
            key: raw.key_bytes.and_then(|k| std::str::from_utf8(&k).ok().map(String::from)),
            value: raw.value_bytes.and_then(|v| std::str::from_utf8(&v).ok().map(String::from)),
            timestamp: raw.timestamp,
        })
        .collect();

    messages
}
```

### 3. 时间范围Offset计算

```rust
fn calculate_partition_offset(
    consumer: &BaseConsumer,
    topic: &str,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    fetch_mode: Option<&str>,
) -> Result<TimeRangeInfo> {
    // 获取watermark
    let (low, high) = consumer.fetch_watermarks(topic, partition, Duration::from_secs(5))?;

    // 计算start_offset
    let start_offset = if let Some(off) = offset {
        off.max(low).min(high)
    } else if let Some(st) = start_time {
        // 使用offsets_for_times查找时间对应的offset
        let ts = st * 1000;
        match consumer.offsets_for_times(vec![(partition, ts)], Duration::from_secs(5)) {
            Ok(results) => /* 提取offset */,
            Err(_) => low,
        }
    } else if fetch_mode == Some("newest") {
        (high - max_messages as i64).max(low)
    } else {
        low
    };

    // 计算end_offset
    let end_offset = if let Some(et) = end_time {
        // 查找end_time对应的offset
        let ts = et * 1000;
        match consumer.offsets_for_times(vec![(partition, ts)], Duration::from_secs(5)) {
            Ok(results) => offset.saturating_sub(1).min(high),
            Err(_) => high,
        }
    } else {
        high
    };

    Ok(TimeRangeInfo {
        start_offset,
        end_offset,
        low_watermark: low,
        high_watermark: high,
    })
}
```

## 关键优化点

| 优化项 | 实现 | 效果 |
|--------|------|------|
| **并行模式触发** | 分区数>1时自动并行，Semaphore(10)限制并发 | 多分区场景3x加速 |
| **显式Seek定位** | assign()后必须调用seek() | 确保从正确offset开始消费 |
| **唯一group.id** | `kafka-mgr-{partition}-{timestamp}` | 避免并发冲突导致结果不稳定 |
| **空轮询限制** | 首次500ms，后续150ms × 10次 | 首次更宽容，无数据快速返回 |
| **提前退出** | start≥high或high≤low时立即返回 | 空分区0ms返回 |
| **分区末尾检测** | offset ≥ high_watermark - 1 | 数据取完立即退出 |
| **延迟字符串转换** | 先存字节，需要时再转UTF-8 | 减少不必要分配 |
| **搜索提前过滤** | 提取`message_matches_search`函数复用 | 减少内存占用，代码更简洁 |
| **内存预分配优化** | 搜索时预估更少，上限5万条 | 避免大内存分配 |
| **避免重复watermark** | 复用已获取的high_watermark | 减少一次网络请求 |
| **Socket超时配置** | socket.timeout.ms=10s, request.timeout.ms=30s | 避免无限等待 |

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
```

## 相关API方法

| Method | 功能 | 所在文件 |
|--------|------|----------|
| `message.list` | 查询消息 | `src/routes/unified.rs` |
| `message.send` | 发送消息 | `src/routes/unified.rs` |
| `message.export` | 导出消息 | `src/routes/unified.rs` |

## 开发注意事项

1. **统一实现**: 不再区分本地/远程模式，使用统一的 `fetch_partition_messages_unified`
2. **显式Seek**: `assign()` 后必须调用 `seek()`，否则consumer不会定位到指定offset
3. **并行阈值**: 分区数>1自动进入并行模式，使用Semaphore限制最多10并发
4. **空分区优化**: 通过watermark预检查，无数据的分区立即返回
5. **延迟转换**: RawMessage结构存储字节，最后统一转换为String
6. **搜索过滤**: 在消息收集阶段就进行过滤，不存储不匹配的消息
7. **唯一group.id**: 每个分区使用包含分区ID和时间戳的唯一group.id，避免并发冲突
8. **分区末尾检测**: 当 `msg.offset >= high_watermark - 1` 时立即退出，避免空轮询
9. **日志标识**: 使用 `[Unified]` 和 `[Unified Partition]` 标识日志

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 2.0 | 2026-03-18 | 合并本地/远程模式为统一实现，分区数>1进入并行，空轮询限制1.5秒，延迟字符串转换，空分区提前退出，显式seek定位修复 |
