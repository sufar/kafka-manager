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
2. **公平性**：确保每个分区都能获取到指定数量的消息，避免某些分区消息被"饿死"
3. **性能优化**：分区之间无依赖，可以并发执行

#### 实现细节

```rust
// 计算目标总消息数
target_total = max_messages * partition_count

// 每个分区独立计数
while raw_msgs.len() < target_total {
    // 检查当前分区是否已满
    if partition_counts.get(&part) >= max_messages {
        continue;  // 跳过已满的分区
    }
    // 收集消息...
}
```

---

## 消息查询优化策略

### 2. 两阶段查询（收集 + 过滤）

为了提高搜索效率，采用**两阶段查询策略**：

#### 第一阶段：收集原始消息
- 每个分区最多获取 `max_messages` 条消息
- 如果有时间范围（start_time/end_time），在时间范围内获取
- **不执行搜索过滤**，只进行时间范围过滤

#### 第二阶段：搜索过滤
- 对已收集的所有消息进行搜索过滤
- 支持 key 和 value 的模糊匹配（不区分大小写）

```rust
// 第一阶段：收集
while raw_msgs.len() < target_total {
    // 只进行时间范围过滤
    if timestamp_in_range(msg) {
        raw_msgs.push(msg);
    }
}

// 第二阶段：搜索过滤
let filtered = if has_search_term {
    raw_msgs.into_iter()
        .filter(|msg| msg.key.contains(term) || msg.value.contains(term))
        .collect()
} else {
    raw_msgs
}
```

#### 为什么这样优化？

**优化前的问题**：
- 边 poll 边搜索过滤
- 为了找到 10万条匹配消息，可能需要 poll 100万条甚至更多
- 30秒超时后返回 0 条

**优化后的效果**：
- 只 poll 指定数量的消息（如 10万条）
- 然后在这些消息中搜索
- 查询速度提升 10 倍以上

---

## 查询模式

### 3. 本地模式 vs 远程模式

根据 Kafka 集群位置自动选择查询策略：

#### 本地模式（Local）

**判断条件**：
```rust
is_local = brokers.contains("localhost") ||
           brokers.contains("127.0.0.1") ||
           brokers.contains("host.docker.internal");
```

**策略**：
- **小批量（< 1000 条/分区）**：串行模式，单 consumer 多分区
- **大批量（>= 1000 条/分区）**：并行模式，每分区独立 consumer，最多 10 个并发

#### 远程模式（Remote）

**策略**：
- 单 consumer 多分区，复用连接
- 减少网络 RTT 开销
- 优化 poll 超时策略

---

## 并发控制

### 4. 并行查询限制

为了防止对 Kafka 集群造成过大压力：

```rust
// 最多 10 个并发分区
let semaphore = Arc::new(Semaphore::new(10));
```

当分区数超过 10 个时，会排队执行，避免同时创建过多 consumer。

---

## 超时与限制

### 5. 查询超时配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| 任务超时 | 30 秒 | 每个并行任务的超时时间 |
| Poll 超时 | 200 ms | 单次 poll 等待时间 |
| 空轮次上限 | max(分区数 × 每分区消息数, 20) | 连续空 poll 次数上限 |

### 6. 空轮次处理

```rust
let max_empty_rounds = (partition_count * max_messages).max(20);

while total_empty < max_empty_rounds {
    match consumer.poll(Duration::from_millis(200)) {
        Some(Ok(msg)) => {
            total_empty = 0;  // 重置空轮次计数
            // 处理消息...
        }
        _ => { total_empty += 1; }
    }
}
```

---

## 时间范围查询

### 7. 基于时间戳的 Offset 定位

当指定 `start_time` 或 `end_time` 时：

1. **计算起始 Offset**：使用 `offsets_for_times` API 根据时间戳查找对应的 offset
2. **时间范围过滤**：在 poll 消息时进行时间戳过滤
3. **每个分区独立**：时间范围计算在每个分区上独立执行

```rust
// 计算基于时间戳的起始 offset
if let Some(time) = start_time {
    let offset = consumer.offsets_for_times(time)?;
    // 从该 offset 开始消费
}

// poll 时进行时间过滤
if timestamp < start_time || timestamp > end_time {
    continue;  // 跳过不在时间范围内的消息
}
```

---

## 配置优化

### 8. Consumer 配置

#### 并行模式优化（关键）

**Consumer 预热机制**是解决首次 poll 延迟的关键：

```rust
// 1. 使用固定 group.id（避免 Unknown group 错误）
cfg.set("group.id", "kafka-mgr-query");
cfg.set("session.timeout.ms", "3000");      // 短会话超时
cfg.set("heartbeat.interval.ms", "500");

// 2. Consumer 预热（关键优化）
// librdkafka 的 fetcher 是异步的，assign 后需要等待它从 broker 拉取第一批数据
let warmup_start = std::time::Instant::now();
let mut warmed_up = false;
while !warmed_up && warmup_start.elapsed() < Duration::from_millis(1000) {
    match consumer.poll(Duration::from_millis(100)) {
        Some(Ok(_)) => {
            // 拿到消息了，fetcher 已准备好，seek 回起始位置
            consumer.seek(&topic, partition, seek_offset, Duration::from_secs(1))?;
            warmed_up = true;
        }
        _ => continue,  // 还没准备好，继续等待
    }
}

// 3. 预热后使用短超时快速轮询（数据已准备好）
while raw_messages.len() < max_messages {
    match consumer.poll(Duration::from_millis(10)) {  // 短超时
        Some(Ok(msg)) => { /* 处理消息 */ }
        _ => { empty_count += 1; }
    }
}
```

**为什么需要预热？**
- `assign()` 只是逻辑分配，librdkafka 的 fetcher 线程需要异步建立连接
- 如果不预热，第一次正式 poll 时 fetcher 还没准备好，返回 None
- 导致需要多次空轮询才能拿到数据（从 3 秒优化到 <100ms）

#### 本地模式优化
```rust
cfg.set("fetch.min.bytes", "1");              // 最小获取字节，立即返回
cfg.set("fetch.wait.max.ms", "1");            // 最大等待 1ms
cfg.set("fetch.max.bytes", "10485760");       // 10MB（减少缓冲延迟）
cfg.set("socket.nagle.disable", "true");      // 禁用 Nagle 算法
cfg.set("broker.address.family", "v4");       // 强制 IPv4
cfg.set("reconnect.backoff.ms", "50");        // 快速重连
```

#### 远程模式优化
```rust
cfg.set("group.id", "kafka-mgr-remote");      // 固定 group.id
cfg.set("session.timeout.ms", "3000");        // 短会话超时
cfg.set("fetch.min.bytes", "1");              // 快速返回
cfg.set("fetch.wait.max.ms", "10");
cfg.set("fetch.max.bytes", "10485760");       // 10MB
```

---

## 性能优化成果

### 9. 优化前后对比

| 场景 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 本地单分区（9条消息） | ~3秒 | ~100ms | **30x** |
| 本地多分区（300条消息） | ~3秒 | ~200ms | **15x** |
| 远程集群 | ~5秒 | ~500ms | **10x** |

### 10. 关键优化点

1. **Consumer 预热**：assign 后主动 poll 直到 fetcher 准备好，避免空轮询
2. **固定 group.id**：避免每次请求动态生成导致的 group 协调开销
3. **短 session timeout**：3秒超时，不依赖 group 协调
4. **预热后短 poll 超时**：数据已准备好，10ms 快速轮询

---

## 日志与监控

### 11. 关键日志

查询执行时会记录以下关键信息：

```
[Local] 3 partitions, 100000 messages per partition, total 300000. Mode: "parallel"
[Parallel Mode] 3 partitions, 100000 messages per partition, total target 300000
[Parallel] Starting fetch for partition 0 of topic xxx (max_messages: 100000)
[Parallel] Partition 0 start_offset: 0
[Parallel] Fetched 1620 messages from partition 0 (polled: 33400, after search filter: 1620)
[Local] Fetched 5420 messages from 3 partitions in 2.345s
```

---

## 常见问题

### Q1: 为什么设置了 max_messages=1000，但返回了 3000 条？

**A**: `max_messages` 是 **per partition** 的。如果 Topic 有 3 个分区，每个分区获取 1000 条，总共最多可能返回 3000 条。

### Q2: 搜索时为什么不需要设置很大的 max_messages？

**A**: 搜索只在已收集的 `max_messages` 条消息内进行过滤，不需要额外 poll 更多消息来"寻找"匹配项。

### Q3: 有时间范围时，搜索范围是什么？

**A**: 先在时间范围内获取 `max_messages` 条消息，然后在这些消息中进行搜索过滤。

### Q4: 为什么并行模式下某些分区返回的消息很少？

**A**: 可能原因：
1. 该分区消息总数不足 `max_messages` 条
2. 搜索过滤后匹配的消息很少
3. 达到了空轮次上限（该分区没有更多消息）

### Q5: 为什么 Consumer 需要预热？

**A**: `assign()` 只是逻辑分配分区，librdkafka 的 fetcher 线程需要异步建立连接并从 broker 拉取数据。如果不预热，首次 poll 时数据还没准备好，需要多次空轮询。预热后数据立即可用，查询速度从 3 秒优化到 <100ms。

### Q6: 为什么使用固定 group.id 而不是动态生成？

**A**: 动态生成 group.id 会导致每次请求都经历完整的消费者组协调流程（发现 coordinator → join group → sync group），耗时 500-1000ms。固定 group.id 配合 `assign()` 模式（非 subscribe），可以避免 group 协调开销。

---

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-03-15 | 初始文档，记录两阶段查询优化 |
| 1.1 | 2026-03-16 | 添加 Consumer 预热机制、固定 group.id 策略、性能优化成果（30x 提升）|
