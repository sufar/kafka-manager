# Kafka Manager 性能优化文档

本文档记录了 Kafka Manager 中实现的性能优化策略和实现细节。

---

## 目录

1. [流式过滤优化](#流式过滤优化)
2. [流式排序优化](#流式排序优化)
3. [Kafka 查询性能优化](#kafka-查询性能优化)
4. [Topic 搜索优化](#topic-搜索优化)
5. [缓存优化](#缓存优化)

---

## 流式过滤优化

### 问题背景

早期的实现在内存中加载所有消息后再进行过滤：

```rust
// 旧代码：先加载所有消息到内存
let mut messages = consumer.fetch_messages(...).await?;

// 然后在内存中过滤
messages.retain(|msg| matches_filter(msg));
```

**问题**：当搜索匹配率很低时（比如 1%），会浪费 99% 的内存。

### 优化方案

新增 `fetch_messages_filtered` 方法，支持在读取消息时就进行过滤：

```rust
/// 从指定 offset 获取消息（支持流式过滤）
///
/// 此方法在读取消息时即进行过滤，避免大量数据加载到内存
pub async fn fetch_messages_filtered<M>(
    &self,
    kafka_config: &KafkaConfig,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    matcher: &M,  // 过滤器闭包
) -> Result<Vec<KafkaMessage>>
where
    M: Fn(&KafkaMessage) -> bool,
```

**流式过滤实现**：

```rust
while read_count < max_messages {
    match tokio::time::timeout(self.timeout, consumer.recv()).await {
        Ok(Ok(msg)) => {
            // 分区过滤
            if let Some(p) = partition {
                if msg.partition() != p { continue; }
            }
            // offset 过滤
            if let Some(min_offset) = offset {
                if msg.offset() < min_offset { continue; }
            }

            let kafka_msg = KafkaMessage { ... };

            // 流式过滤：匹配才添加
            if matcher(&kafka_msg) {
                messages.push(kafka_msg);
            }
        }
        _ => break,
    }
}
```

### 性能改进

| 场景 | 优化前 | 优化后 | 内存节省 |
|------|--------|--------|----------|
| 搜索匹配率 10% | 加载 10000 条，保留 1000 条 | 直接加载 1000 条 | 90% |
| 搜索匹配率 1% | 加载 10000 条，保留 100 条 | 直接加载 100 条 | 99% |
| 时间范围过滤 | 加载所有消息后过滤 | 只加载范围内的消息 | 显著 |

### 支持的过滤条件

| 参数 | 说明 | 示例 |
|------|------|------|
| `partition` | 分区过滤 | `partition=0` |
| `offset` | Offset 过滤（>=） | `offset=1000` |
| `start_time` | 开始时间戳（毫秒） | `start_time=1708876800000` |
| `end_time` | 结束时间戳（毫秒） | `end_time=1708880400000` |
| `search` | 搜索关键词 | `search=error` |
| `search_in` | 搜索范围 | `search_in=value` |

### 使用示例

```rust
let params_clone = params.clone();
let raw_messages = consumer
    .fetch_messages_filtered(
        config, &topic, partition, offset,
        max_messages * 5,
        &move |msg: &KafkaMessage| -> bool {
            // 时间范围过滤
            if let Some(start) = params_clone.start_time {
                if let Some(ts) = msg.timestamp {
                    if ts < start { return false; }
                }
            }
            // 搜索关键词过滤
            if let Some(search) = &params_clone.search {
                if !msg.matches_search(search) { return false; }
            }
            true
        },
    )
    .await?;
```

### 代码位置

- `src/kafka/consumer.rs:115` - `fetch_messages_filtered` 方法
- `src/routes/message.rs:77` - `get_messages` 函数（主接口）
- `src/routes/message.rs:250` - `get_messages_enhanced` 函数（增强接口）
- `src/routes/message.rs:448` - `export_messages` 函数（导出接口）

---

## 流式排序优化

### 问题背景

排序场景下，传统实现需要先加载所有数据到内存，排序后再截断：

```rust
// 旧代码：先加载所有消息
let mut messages = consumer.fetch_messages(...).await?;

// 然后在内存中全量排序
messages.sort_by(|a, b| ts_b.cmp(&ts_a));

// 最后截断到 limit
if limit < messages.len() {
    messages.truncate(limit);
}
```

**问题**：当 `limit=100` 但读取了 10000 条消息时，浪费 99% 的内存和 CPU。

### 优化方案

使用 **堆（Heap）** 数据结构实现流式 TopK 排序，只维护大小为 `limit` 的堆：

#### 降序排序（最新的在前）

```rust
use std::collections::BinaryHeap;
use std::cmp::Reverse;

// 使用最小堆，维护最大的 limit 个元素
let mut heap: BinaryHeap<Reverse<MessageRecord>> = BinaryHeap::new();
for msg in raw_messages {
    let record = MessageRecord { ... };
    if heap.len() < limit {
        heap.push(Reverse(record));
    } else if let Some(min) = heap.peek() {
        if record.timestamp.unwrap_or(0) > min.0.timestamp.unwrap_or(0) {
            heap.pop();
            heap.push(Reverse(record));
        }
    }
}
// 最后一次性排序输出
let mut result: Vec<MessageRecord> = heap.into_iter().map(|Reverse(r)| r).collect();
result.sort_by(|a, b| ts_b.cmp(&ts_a));
```

#### 升序排序（最旧的在前）

```rust
use std::collections::BinaryHeap;

// 使用最大堆，维护最小的 limit 个元素
let mut heap: BinaryHeap<MessageRecord> = BinaryHeap::new();
for msg in raw_messages {
    let record = MessageRecord { ... };
    if heap.len() < limit {
        heap.push(record);
    } else if let Some(max) = heap.peek() {
        if record.timestamp.unwrap_or(i64::MAX) < max.timestamp.unwrap_or(i64::MAX) {
            heap.pop();
            heap.push(record);
        }
    }
}
```

### 内存优化效果

| 场景 | 优化前 | 优化后 | 改进 |
|------|--------|--------|------|
| 读取 10000 条，limit=100 | 加载 10000 条 → 排序 → 截断 | 只维护 100 个元素的堆 | 内存节省 **99%** |
| 读取 5000 条，limit=50 | 加载 5000 条 → 排序 → 截断 | 只维护 50 个元素的堆 | 内存节省 **99%** |

### 时间复杂度对比

| 方法 | 时间复杂度 | 说明 |
|------|-----------|------|
| 优化前 | O(n log n) | 全量排序 |
| 优化后 | O(n log k) | 堆维护，k = limit |

### 完整流程

```
Kafka → 流式过滤 → 堆排序 (TopK) → 最终排序 → 返回
        ↓           ↓              ↓
    fetch_messages  维护大小为    最后一次性
    filtered        limit 的堆     排序输出
```

### 核心代码

```rust
use std::collections::BinaryHeap;
use std::cmp::Reverse;

// 判断是否需要排序
let need_sort = params.order_by.as_deref() == Some("timestamp");
let desc = params.sort.as_deref() == Some("desc");
let limit = params.limit.unwrap_or(max_messages);

let messages = if need_sort {
    if desc {
        // 降序：使用最小堆
        let mut heap: BinaryHeap<Reverse<MessageRecord>> = BinaryHeap::new();
        for msg in raw_messages {
            let record = MessageRecord { ... };
            if heap.len() < limit {
                heap.push(Reverse(record));
            } else if let Some(min) = heap.peek() {
                if record.timestamp.unwrap_or(0) > min.0.timestamp.unwrap_or(0) {
                    heap.pop();
                    heap.push(Reverse(record));
                }
            }
        }
        let mut result: Vec<MessageRecord> = heap.into_iter().map(|Reverse(r)| r).collect();
        result.sort_by(|a, b| ts_b.cmp(&ts_a));
        result
    } else {
        // 升序：使用最大堆
        let mut heap: BinaryHeap<MessageRecord> = BinaryHeap::new();
        for msg in raw_messages {
            let record = MessageRecord { ... };
            if heap.len() < limit {
                heap.push(record);
            } else if let Some(max) = heap.peek() {
                if record.timestamp.unwrap_or(i64::MAX) < max.timestamp.unwrap_or(i64::MAX) {
                    heap.pop();
                    heap.push(record);
                }
            }
        }
        let mut result: Vec<MessageRecord> = heap.into_iter().collect();
        result.sort_by(|a, b| ts_a.cmp(&ts_b));
        result
    }
} else {
    // 不需要排序，直接取前 limit 条
    raw_messages.into_iter().take(limit).collect()
};
```

### 代码位置

- `src/routes/message.rs:77` - `get_messages` 函数
- `src/models/mod.rs:61` - `MessageRecord` 结构体（添加了 `Ord` trait）

### 注意事项

1. **MessageRecord 需要实现 `Ord` trait**：
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   pub struct MessageRecord {
       pub partition: i32,
       pub offset: i64,
       pub key: Option<String>,
       pub value: Option<String>,
       pub timestamp: Option<i64>,
   }
   ```

2. **降序排序使用 `Reverse` 包装器**：将最大值优先转换为最小值优先

3. **时间戳为 `None` 的处理**：使用默认值参与比较（降序用 0，升序用 `i64::MAX`）

---

## Kafka 查询性能优化

### 问题背景

Kafka 的 AdminClient 查询是同步阻塞操作，如果在 tokio 异步运行时中直接调用，会阻塞整个线程池，导致其他请求无法处理。

### 优化方案

将所有同步 Kafka 操作包装在 `tokio::task::spawn_blocking` 中：

```rust
let admin = admin.clone();
let result = tokio::task::spawn_blocking(move || {
    admin.get_cluster_info()
})
.await
.map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;
```

### 优化范围

- `cluster_monitor.rs` - 5 个接口
- `cluster_stats.rs` - 1 个接口
- `topic.rs` - 2 个接口

---

## Topic 搜索优化

### 问题背景

原来的 Topic 搜索实现是 N+1 查询问题：

```rust
// 旧代码：N 次查询（N = 集群数量）
for cluster in &clusters {
    let topics = TopicStore::list_by_cluster(...).await?; // 每次查询数据库
    ...
}
```

### 优化方案

改为单次查询所有 Topic：

```rust
// 新代码：1 次查询
let topics = TopicStore::list_all(state.db.inner()).await?;
```

新增方法：
- `TopicStore::list_all()` - 一次性查询所有 Topic
- `TopicStore::search()` - 支持模糊搜索

### 性能改进

| 集群数量 | 优化前 | 优化后 | 改进 |
|----------|--------|--------|------|
| 10 个 | 10 次查询 | 1 次查询 | **90%** |
| 100 个 | 100 次查询 | 1 次查询 | **99%** |

---

## 缓存优化

### 问题背景

原来的缓存时间过短（3-5秒），导致频繁查询 Kafka 集群，增加集群压力。

### 优化方案

延长缓存时间：

| 缓存类型 | 优化前 | 优化后 |
|----------|--------|--------|
| Topic 元数据 | 5 秒 | 30 秒 |
| Topic 列表 | 3 秒 | 30 秒 |
| Consumer Group | 3 秒 | 30 秒 |
| Broker 信息 | 10 秒 | 60 秒 |

### 代码位置

- `src/cache/mod.rs` - 缓存配置

---

## 总结

通过以下优化，Kafka Manager 的性能得到显著提升：

1. **流式过滤** - 节省 90-99% 内存
2. **流式排序** - 节省 99% 内存
3. **异步化 Kafka 查询** - 避免阻塞 tokio 运行时
4. **数据库查询优化** - 减少 N+1 查询
5. **缓存优化** - 减少 Kafka 集群压力
