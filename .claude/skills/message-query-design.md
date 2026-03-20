# Kafka Manager 消息查询设计文档

## 核心设计原则

### 1. Max Messages 是 Per Partition（每分区）

**重要说明**：`max_messages` 参数表示**每个分区**要获取的消息数量，而不是所有分区的总和。

例如：
- 设置 `max_messages = 1000`
- Topic 有 3 个分区
- 实际最多可能返回 `1000 × 3 = 3000` 条消息

#### 为什么这样设计？

1. **并行处理**：每个分区独立获取消息，充分利用并行能力
2. **公平性**：确保每个分区都能获取到指定数量的消息
3. **性能优化**：分区之间无依赖，可以并发执行

---

## 查询模式

### 2. 串行模式 vs 并行模式

根据分区数自动选择查询策略：

#### 判断逻辑

```rust
// 分区数>1 时使用并行模式
let use_parallel = partition_count > 1;
```

#### 串行模式（单分区）

**适用场景**：分区数 = 1

**策略**：
- 单个 consumer 读取单个分区
- 无并发开销

#### 并行模式（多分区）

**适用场景**：分区数 > 1

**策略**：
- 每个分区创建独立的 consumer
- 使用最小堆归并排序，按时间戳有序输出
- 最多 10 个并发（使用 Semaphore 限制）

```rust
let semaphore = Arc::new(Semaphore::new(10));
```

**为什么这样设计？**

分区数 > 1 时，并行模式可以充分利用多分区的数据并行性，每个分区独立拉取消息，最后归并排序。

---

## 并发控制

### 3. 并行查询限制

为了防止对 Kafka 集群造成过大压力：

```rust
// 最多 10 个并发分区
let semaphore = Arc::new(Semaphore::new(10));
```

当分区数超过 10 个时，会排队执行，避免同时创建过多 consumer。

---

## 超时与限制

### 4. 查询超时配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| 任务超时 | 60 秒 | 每个并行任务的超时时间 |
| Poll 超时 | 500ms (首次) / 150ms (后续) | 首次 poll 等待更长时间建立连接 |
| 空轮次上限 | 10 次 | 连续空 poll 次数上限 |
| 总轮询时间 | 30 秒 | 轮询的最长等待时间 |

### 5. 空轮次处理

```rust
const FIRST_POLL_TIMEOUT_MS: u64 = 500;  // 首次等待 500ms
const POLL_TIMEOUT_MS: u64 = 150;        // 后续 150ms
const MAX_EMPTY_POLLS: usize = 10;       // 最多 10 次空轮
const MAX_POLL_TIME_SECS: u64 = 30;      // 总轮询时间 30 秒
```

**分区末尾检测**：
- 预获取 watermarks（low/high）用于判断分区边界
- 当消息 offset >= high_watermark - 1 时，立即退出
- 当分区为空（high <= low）时，跳过该分区

---

## 时间范围查询

### 6. 基于时间戳的 Offset 定位

当指定 `start_time` 或 `end_time` 时：

1. **计算起始 Offset**：使用 `offsets_for_times` API 根据时间戳查找对应的 offset
2. **计算结束 Offset**：如果指定了 `end_time`，同时查询该时间对应的 offset
3. **时间范围边界检测**：在 poll 时检查是否到达时间范围的结束 offset，提前结束查询
4. **每个分区独立**：时间范围计算在每个分区上独立执行

**为什么需要查询 end_time offset？**

优化前只知道 start_time 对应的 offset，需要不断 poll 并检查消息时间戳是否超过 end_time，即使时间范围内没有更多消息，也要等待空轮次超时。

优化后同时知道 start_offset 和 end_offset，当消费到 end_offset 时立即知道时间范围已结束，可以提前结束该分区的查询，减少空轮询等待。

---

## Consumer 配置优化

### 7. 统一 Consumer 配置

```rust
// 使用唯一的 group.id 避免并发冲突
let unique_suffix = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis();
let unique_group_id = format!("kafka-mgr-{}-{}", partition, unique_suffix);

let mut cfg = ClientConfig::new();
cfg.set("bootstrap.servers", &brokers);
cfg.set("group.id", &unique_group_id);
cfg.set("enable.auto.commit", "false");
cfg.set("auto.offset.reset", "earliest");
cfg.set("session.timeout.ms", "3000");      // 短会话超时
cfg.set("heartbeat.interval.ms", "500");

// 根据消息量动态调整 fetch 配置
if max_messages > 1000 {
    cfg.set("fetch.min.bytes", "65536");           // 64KB
    cfg.set("fetch.wait.max.ms", "100");
    cfg.set("fetch.max.bytes", "52428800");        // 50MB
    cfg.set("max.partition.fetch.bytes", "52428800");
} else {
    cfg.set("fetch.min.bytes", "1");
    cfg.set("fetch.wait.max.ms", "10");
    cfg.set("fetch.max.bytes", "10485760");        // 10MB
    cfg.set("max.partition_fetch.bytes", "10485760");
}

cfg.set("socket.nagle.disable", "true");
cfg.set("socket.receive.buffer.bytes", "262144");
cfg.set("socket.timeout.ms", "10000");
cfg.set("request.timeout.ms", "30000");
cfg.set("enable.partition.eof", "false");
cfg.set("reconnect.backoff.ms", "50");
cfg.set("reconnect.backoff.max.ms", "500");
cfg.set("broker.address.family", "v4");
```

### 8. 显式 Seek 定位（关键修复）

`assign()` 只是逻辑分配分区，不会自动定位到指定 offset，必须显式调用 `seek()`：

```rust
// 1. 创建分配列表
let mut tpl = TopicPartitionList::new();
tpl.add_partition_offset(&topic, partition, seek_offset)?;

// 2. assign 只是注册分配
consumer.assign(&tpl)?;

// 3. 必须显式 seek 到指定位置（关键！）
consumer.seek(&topic, partition, seek_offset, Duration::from_secs(5))?;
```

**注意**：不加 `seek()` 会导致 consumer 从任意位置开始消费，查询结果不稳定，大数据量时可能返回空或部分数据。

---

## 性能优化

### 9. 关键优化点

| 优化点 | 说明 |
|--------|------|
| **唯一 group.id** | 每个分区使用唯一的 group.id 避免消费者组协调开销 |
| **短 session timeout** | 3 秒超时，不依赖 group 协调 |
| **显式 seek** | assign 后必须显式 seek 到指定 offset |
| **分区末尾检测** | 通过 watermark 判断分区末尾，提前退出避免空轮询 |
| **空分区预标记** | 提前标记空分区（high <= low），避免空轮询 |
| **自适应 poll 超时** | 首次 500ms 建立连接，后续 150ms 快速轮询 |
| **延迟字符串转换** | 先收集原始字节，需要搜索时再转换为字符串 |

### 10. 性能数据

| 场景 | 耗时 |
|------|------|
| 本地单分区（9 条消息） | ~100ms |
| 本地多分区（300 条消息） | ~200ms |
| 远程集群 | ~500ms |

---

## 常见问题

### Q1: 为什么设置了 max_messages=1000，但返回了 3000 条？

**A**: `max_messages` 是 **per partition** 的。如果 Topic 有 3 个分区，每个分区获取 1000 条，总共最多可能返回 3000 条。

### Q2: 为什么并行模式下某些分区返回的消息很少？

**A**: 可能原因：
1. 该分区消息总数不足 `max_messages` 条
2. 搜索过滤后匹配的消息很少
3. 达到了空轮次上限（该分区没有更多消息）

### Q3: 为什么分区数少时查询反而慢？

**A**: 这是**并行模式的连接开销问题**。

**现象**：3 个分区，1200 条消息，查询耗时 47 秒，实际上只有 123 条消息数据

**原因分析**：
1. 并行模式下每个分区创建独立 consumer
2. 每个 consumer 需要建立 TCP 连接、获取元数据
3. 3 个分区 × 15 秒连接建立 = 45 秒
4. 实际数据查询只需 2 秒

**解决方案**：当前实现分区数 > 1 时使用并行模式，多分区场景下并行优势明显。

**效果**：从 47 秒优化到 5 秒（3 分区 × 10000 条场景）。

### Q4: 为什么数据很少的 topic 查询也要 5-6 秒？

**A**: **空轮次等待问题**。

**现象**：max_messages=100，3 个分区，topic 里只有 1 条消息，查询耗时 5-6 秒

**原因分析**：
1. 设置 max_messages=100，期望获取 100×3=300 条消息
2. 实际只有 1 条消息，后续 poll 都是空轮询
3. 空轮次上限为 10 轮，每轮 150ms，加上总时间 30 秒保底

**解决方案 - 分区边界检测 + 空分区预标记**：

```rust
// 预获取 watermark（包含 low 和 high）
let mut partition_watermarks: HashMap<i32, (i64, i64)> = HashMap::new();
for &part_id in &partitions {
    if let Ok((low, high)) = consumer.fetch_watermarks(&topic, part_id, Duration::from_millis(1000)) {
        partition_watermarks.insert(part_id, (low, high));
        // 预标记空分区（high <= low 或 high == 0）
        if high <= low || high == 0 {
            partition_reached_end.insert(part_id, true);
        }
    }
}

// 检查是否到达分区边界
if msg_offset >= high_watermark - 1 {
    // 到达分区末尾，立即退出
    break;
}
```

**关键优化点**：
1. **预标记空分区**：在 poll 开始前，通过 `high <= low || high == 0` 识别空分区并标记为完成
2. **避免空轮询**：当所有分区都标记为完成时，立即结束循环
3. **效果**：从 5-6 秒优化到 <500ms（当分区为空时立即结束）

---

## SSE 流式传输

### 整体流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  分区 0 读取   │────→│   最小堆    │────→│  SSE 批次    │────→ 前端实时显示
└─────────────┘     │  归并排序   │     │  (100 条/批) │
┌─────────────┐     │             │     │             │
│  分区 1 读取   │────→│             │     │             │
└─────────────┘     │             │     │             │
┌─────────────┐     │             │     │             │
│  分区 2 读取   │────→│             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
      ↑                    ↑                  ↑
   channel              BinaryHeap        EventStream
   实时发送              动态归并           实时接收
```

### 后端实现

**核心组件**：

1. **流式分区读取** (`fetch_partition_messages_streaming`)：通过 channel 实时发送消息
2. **最小堆归并** (`HeapMessage`)：按时间戳排序，时间戳相同则按 offset 排序
3. **流式归并** (`fetch_messages_streaming_sse`)：并行读取 + 最小堆归并 + 每 100 条发送 SSE 事件

### SSE 事件类型

| 事件 | 数据格式 | 说明 |
|------|----------|------|
| `start` | `{\"partitions\": 3, \"total_target\": 30000}` | 开始获取消息 |
| `batch` | `{\"messages\": [...], \"progress\": 100, \"total\": 30000}` | 一批消息（100 条） |
| `order` | `{\"sort\": \"desc\"}` | 排序方向通知 |
| `complete` | `{}` | 获取完成 |
| `error` | `{\"error\": \"...\"}` | 错误信息 |

### 前端 SSE 性能优化

**关键优化点**：

1. **改进 SSE 解析逻辑**：使用 `\\n\\n` 分割完整的 SSE 事件，而不是逐行解析
2. **非响应式缓存 + 批量更新**：batch 到达时不触发响应式，每 200ms 批量更新一次 UI
3. **使用 shallowRef**：Vue 只追踪数组引用的变化，不追踪数组内部元素

**性能对比**：

| 指标 | 优化前 | 优化后 |
|------|--------|--------|
| 消息完整性 | ~19000/30000 | 30000/30000 |
| 响应式更新次数 | 60+ 次 | ~3-5 次 |
| 浏览器稳定性 | 多次查询后崩溃 | 稳定 |

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/routes/unified.rs` | SSE 请求处理器 |
| `src/kafka/consumer.rs` | Kafka 消费者实现 |
| `ui/src/api/client.ts` | SSE 客户端 |
| `ui/src/views/MessagesClassicView.vue` | 经典消息视图 |
| `ui/src/components/MessageQueryTool.vue` | 简洁消息查询工具 |
