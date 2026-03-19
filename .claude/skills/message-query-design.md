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
- **小批量（< 1000 条/分区）或 分区数 <= 5**：串行模式，单 consumer 多分区
- **大批量（>= 1000 条/分区）且 分区数 > 5**：并行模式，每分区独立 consumer，最多 10 个并发

**分区数优化**（关键）：
```rust
// 优化：分区数 <= 5 时使用串行模式，避免创建多个 consumer 的连接开销
let messages = if (max_messages >= 1000 || partition_count == 1) && partition_count > 5 {
    // 并行模式（仅当分区数>5时）
} else {
    // 串行模式（分区数<=5时使用单consumer）
}
```

**原理**：分区数较少时，串行模式的单连接比并行模式的多连接开销更小。实测3个分区时，串行模式比并行模式快10倍以上（3秒 vs 47秒）。

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

**自适应空轮次上限 + 分区末尾检测**：
```rust
// 优化前：固定值或过大
let max_empty_rounds = (partition_count * max_messages).max(20);  // 可能导致3600轮！

// 优化后：自适应公式 + 时间保底 + 分区末尾检测
let max_empty = 100 + max_messages / 100 * 10;  // 基础100轮 + 每100条加10轮
let max_poll_time = Duration::from_secs(30);    // 30秒时间保底

// 预获取所有分区的 watermark，用于判断分区末尾
let mut partition_watermarks: HashMap<i32, (i64, i64)> = HashMap::new();
for &part_id in &partitions {
    if let Ok((low, high)) = consumer.fetch_watermarks(&topic, part_id, Duration::from_millis(1000)) {
        partition_watermarks.insert(part_id, (low, high));
    }
}

// 记录每个分区是否已到达末尾
let mut partition_reached_end: HashMap<i32, bool> = HashMap::new();

while raw_messages.len() < max_messages
    && empty_count < max_empty
    && poll_start.elapsed() < max_poll_time
    && !all_partitions_finished(&partition_counts, &partition_reached_end) {  // 检查分区是否全部完成
    match consumer.poll(Duration::from_millis(10)) {
        Some(Ok(msg)) => {
            empty_count = 0;
            let part = msg.partition();
            let count = partition_counts.get(&part).copied().unwrap_or(0);

            // 检查是否到达分区末尾（消息数满 或 offset >= high watermark - 1）
            if let Some((_, high)) = partition_watermarks.get(&part) {
                if msg.offset() >= high.saturating_sub(1) || count + 1 >= max_messages {
                    partition_reached_end.insert(part, true);
                }
            }
            // 处理消息...
        }
        _ => { empty_count += 1; }
    }
}
```

**分区末尾检测逻辑**：
```rust
// 所有分区都完成时提前结束
let all_partitions_finished = |counts: &HashMap<i32, usize>, reached_end: &HashMap<i32, bool>| {
    partitions.iter().all(|&p| {
        let count = counts.get(&p).copied().unwrap_or(0);
        let reached = reached_end.get(&p).copied().unwrap_or(false);
        count >= max_messages || reached  // 该分区已满 或 已到达末尾
    })
};
```

**关键参数说明**：
| 场景 | max_messages | max_empty | 说明 |
|------|-------------|-----------|------|
| 小批量 | 100 | 110轮 | 快速返回 |
| 中批量 | 1000 | 200轮 | 适中等待 |
| 大批量 | 10000 | 1100轮 | 充足时间 |

**时间保底机制**：
- 轮次保证小数据量快速返回（数据收完即结束）
- 时间保证大数据量不中断（最多等待30秒）
- **分区末尾检测**：当分区数据较少时，检测到到达 watermark 后立即结束，避免空轮询

---

## 时间范围查询

### 7. 基于时间戳的 Offset 定位

当指定 `start_time` 或 `end_time` 时：

1. **计算起始 Offset**：使用 `offsets_for_times` API 根据时间戳查找对应的 offset
2. **计算结束 Offset**（优化）：如果指定了 `end_time`，同时查询该时间对应的 offset
3. **时间范围边界检测**：在 poll 时检查是否到达时间范围的结束 offset，提前结束查询
4. **每个分区独立**：时间范围计算在每个分区上独立执行

```rust
// 查询 start_time 对应的起始 offset
if let Some(start_time) = start_time {
    let mut time_tpl = TopicPartitionList::new();
    for &part_id in &partitions {
        time_tpl.add_partition_offset(&topic, part_id, rdkafka::Offset::Offset(start_time)).ok();
    }
    match consumer.offsets_for_times(time_tpl, Duration::from_secs(5)) {
        Ok(r) => {
            for elem in r.elements_for_topic(&topic) {
                if let Some(offset) = elem.offset().to_raw() {
                    time_based_offsets.insert(elem.partition(), offset);
                }
            }
        }
        Err(e) => { /* 使用 fetch_mode fallback */ }
    }
}

// 优化：查询 end_time 对应的结束 offset，用于提前结束检测
if let Some(end_time) = end_time {
    let mut time_tpl = TopicPartitionList::new();
    for &part_id in &partitions {
        time_tpl.add_partition_offset(&topic, part_id, rdkafka::Offset::Offset(end_time)).ok();
    }
    match consumer.offsets_for_times(time_tpl, Duration::from_secs(5)) {
        Ok(r) => {
            for elem in r.elements_for_topic(&topic) {
                if let Some(offset) = elem.offset().to_raw() {
                    // end_time 对应的 offset 是大于等于该时间的第一条消息
                    // 所以时间范围的有效结束 offset 是 offset - 1
                    time_range_end_offsets.insert(elem.partition(), offset.saturating_sub(1));
                }
            }
        }
        Err(e) => { /* 忽略错误，依赖时间戳过滤 */ }
    }
}

// poll 时检查是否到达时间范围结束
while ... {
    match consumer.poll(...) {
        Some(Ok(msg)) => {
            // 时间范围过滤
            if let Some(start) = start_time {
                if let Some(t) = ts { if t < start { continue; } }
            }
            if let Some(end) = end_time {
                if let Some(t) = ts { if t > end { continue; } }
            }

            // 优化：检查是否到达时间范围的结束 offset
            let reached_time_end = time_range_end_offsets.get(&part)
                .map(|end_off| msg.offset() >= *end_off)
                .unwrap_or(false);

            if reached_time_end {
                partition_reached_end.insert(part, true);
                tracing::info!("Partition {} reached time range end at offset {}", part, msg.offset());
            }
        }
    }
}
```

**为什么需要查询 end_time offset？**

**优化前的问题**：
- 只知道 start_time 对应的 offset
- 需要不断 poll 并检查消息时间戳是否超过 end_time
- 即使时间范围内没有更多消息，也要等待空轮次超时

**优化后的效果**：
- 同时知道 start_offset 和 end_offset
- 当消费到 end_offset 时立即知道时间范围已结束
- 可以提前结束该分区的查询，减少空轮询等待

**使用场景**：
- 查询 1 小时前的消息（start_time=now-1h, end_time=now）
- 如果该时间范围内只有 10 条消息，获取完后立即结束
- 不需要等待 max_messages 或空轮次超时

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

### Q7: 为什么分区数少时查询反而慢（47秒）？

**A**: 这是**并行模式的连接开销问题**。

**现象**：
- 3个分区，1200条消息，查询耗时47秒
- 实际上只有123条消息数据

**原因分析**：
1. 并行模式下每个分区创建独立 consumer
2. 每个 consumer 需要建立 TCP 连接、获取元数据
3. 3个分区 × 15秒连接建立 = 45秒
4. 实际数据查询只需2秒

**解决方案**：
```rust
// 分区数 <= 5 时使用串行模式
if (max_messages >= 1000 || partition_count == 1) && partition_count > 5 {
    // 并行模式
} else {
    // 串行模式（单consumer处理所有分区）
}
```

**效果**：从47秒优化到2秒。

### Q8: 为什么大数据量查询有时成功有时失败？

**A**: **空轮次上限设置过低**。

**现象**：
- max_messages=1200
- 消息很多的topic有时能查到，有时返回空

**原因**：
```rust
// 过低的上限（54轮）
let max_empty = 30 + max_messages / 100 * 2;  // 1200条 = 54轮
```

大数据量时需要更多轮次才能消费完，54轮可能不够用。

**解决方案**：
```rust
// 提高上限并添加时间保底
let max_empty = 100 + max_messages / 100 * 10;  // 1200条 = 220轮
let max_poll_time = Duration::from_secs(30);    // 30秒保底
```

### Q9: 为什么数据很少的 topic 查询也要 5-6 秒？

**A**: **空轮次等待问题**。

**现象**：
- max_messages=100，3个分区
- topic 里只有 1 条消息
- 查询耗时 5-6 秒

**原因分析**：
1. 设置 max_messages=100，期望获取 100×3=300 条消息
2. 实际只有 1 条消息，后续 poll 都是空轮询
3. 空轮次上限为 110 轮，每轮 50ms，总共 5.5 秒

**解决方案 - 分区边界检测 + 空分区预标记**：

```rust
// 预获取 watermark（包含 low 和 high），同时预标记空分区
let mut partition_watermarks: HashMap<i32, (i64, i64)> = HashMap::new();
let mut partition_reached_end: HashMap<i32, bool> = HashMap::new();
for &part_id in &partitions {
    if let Ok((low, high)) = consumer.fetch_watermarks(&topic, part_id, Duration::from_millis(1000)) {
        partition_watermarks.insert(part_id, (low, high));
        // 关键：预标记空分区（high <= low 或 high == 0），避免空轮询
        if high <= low || high == 0 {
            partition_reached_end.insert(part_id, true);
            tracing::info!("[Local] Partition {} is empty (low={}, high={}), marked as finished", part_id, low, high);
        }
    }
}

// 检查是否到达分区边界
while ... && !all_partitions_finished(&partition_counts, &partition_reached_end) {
    match consumer.poll(...) {
        Some(Ok(msg)) => {
            let part = msg.partition();
            let count = partition_counts.get(&part).copied().unwrap_or(0);

            // 检测分区开头（消息过期）或末尾（最新消息）
            if let Some((low, high)) = partition_watermarks.get(&part) {
                let reached_beginning = msg.offset() <= *low;  // 到达最早消息
                let reached_end = msg.offset() >= high - 1;     // 到达最新消息
                let partition_full = count + 1 >= max_messages;

                if reached_beginning || reached_end || partition_full {
                    partition_reached_end.insert(part, true);  // 标记完成
                }
            }
        }
    }
}

// 所有分区都完成时提前结束
let all_partitions_finished = |counts: &HashMap<i32, usize>, reached_end: &HashMap<i32, bool>| {
    partitions.iter().all(|&p| {
        let count = counts.get(&p).copied().unwrap_or(0);
        let reached = reached_end.get(&p).copied().unwrap_or(false);
        count >= max_messages || reached
    })
};
```

**关键优化点**：
1. **预标记空分区**：在 poll 开始前，通过 `high <= low || high == 0` 识别空分区并标记为完成
2. **避免空轮询**：当所有分区都标记为完成时，`all_partitions_finished` 返回 true，立即结束循环
3. **效果**：从 5-6 秒优化到 <500ms（当分区为空时立即结束）

**效果**：当检测到所有分区都到达边界时，立即结束查询，从 5 秒优化到 <500ms。

**关键逻辑**：
- `reached_beginning`: offset <= low（消息过期或已消费到最早）
- `reached_end`: offset >= high - 1（已消费到最新）
- `partition_full`: 已获取 max_messages 条消息

---

## 版本 1.6 统一实现与并行模式

### 统一消息查询入口

合并本地/远程模式为单一实现 `fetch_partition_messages_unified`：

```rust
// 分区数>1时使用并行模式
let use_parallel = partition_count > 1;

let messages = if use_parallel {
    // === 并行模式（分区数>1）===
    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = vec![];

    for &part_id in &partitions {
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let result = timeout(Duration::from_secs(60),
                tokio::task::spawn_blocking(move || {
                    fetch_partition_messages_unified(...)
                })
            ).await;
            (part_id, result)
        });
        handles.push(handle);
    }
    // 收集结果...
} else {
    // === 单分区串行模式 ===
    tokio::task::spawn_blocking(move || {
        fetch_partition_messages_unified(...)
    }).await?
};
```

### 关键优化点

#### 1. 唯一 group.id 避免冲突

```rust
// 每个分区使用唯一的group.id避免并发冲突
let unique_suffix = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis();
let unique_group_id = format!("kafka-mgr-{}-{}", partition, unique_suffix);
cfg.set("group.id", &unique_group_id);
```

**问题**：相同 `group.id` 导致Kafka认为这些consumer属于同一消费者组，触发不必要的group协调，可能：
- 导致某些分区consumer被踢出组
- 引发rebalance延迟
- 查询结果不稳定

**解决**：使用包含分区ID和时间戳的唯一 `group.id`。

#### 2. 显式 Seek 定位（关键修复）

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

#### 3. 分区末尾检测

```rust
// 获取high watermark用于末尾检测
let high_watermark = time_range.high_watermark;

// 检查是否已到达分区末尾
if msg_offset >= high_watermark - 1 {
    tracing::info!("Reached end of partition {} at offset {}",
        partition, msg_offset);
    // 处理完这条消息后退出
    raw_messages.push(...);
    break; // 立即退出，避免空轮询
}
```

**效果**：数据较少的分区获取到最后一条消息后立即退出，不等待空轮询超时。

#### 3. 提前退出条件

```rust
// 1. 分区完全无数据
if time_range.high_watermark <= time_range.low_watermark {
    return Vec::new();
}

// 2. 起始offset已经超过high_watermark
if start_offset >= time_range.high_watermark {
    return Vec::new();
}

// 3. 时间范围结束offset小于起始offset
if time_range_end > 0 && time_range_end < start_offset {
    return Vec::new();
}
```

#### 4. 并行连接建立

```
分区0 task ──┐
             ├─→ 3个连接并行建立（各自独立线程）
分区1 task ──┤
             │
分区2 task ──┘
```

**关键点**：
- 3个分区 = 3个并发的异步任务
- 每个任务在 `spawn_blocking` 线程中创建 consumer
- 3个TCP连接是**同时/并行**建立的
- Semaphore(10) 限制最多10个并发

### 性能对比

| 场景 | 串行模式 | 并行模式 | 提升 |
|------|---------|---------|------|
| 3分区 × 10000条 | ~15秒 | ~5秒 | **3x** |
| 连接建立时间 | 3×串行 | ≈单次连接 | 3x |
| 数据拉取吞吐 | 单连接 | 3连接并行 | 3x |

**结论**：大批量查询（max_messages > 1000/分区）时，并行模式优势明显。

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-03-15 | 初始文档，记录两阶段查询优化 |
| 1.1 | 2026-03-16 | 添加 Consumer 预热机制、固定 group.id 策略、性能优化成果（30x 提升）|
| 1.2 | 2026-03-16 | 添加分区数判断优化（<=5分区用串行模式）、空轮次上限调整（100+n/10）、时间保底机制（30秒）、Q7/Q8常见问题|
| 1.3 | 2026-03-16 | 添加分区边界检测优化（检测 low/high watermark）、Q9常见问题（数据少的topic查询慢）|
| 1.4 | 2026-03-16 | 添加时间范围查询优化（查询 end_time offset 用于提前结束）、更新 Q7/Q9 文档|
| 1.5 | 2026-03-16 | 修复空分区检测（high=0 的分区也要预标记为完成），添加远程模式的空分区预标记|
| 1.6 | 2026-03-18 | 统一本地/远程模式为 `fetch_partition_messages_unified`，分区数>1自动并行，添加分区末尾检测（msg.offset >= high_watermark - 1），修复 group.id 冲突（每个分区使用唯一 group.id），优化内存预分配、提取搜索过滤函数、避免重复获取 watermark、添加 socket 超时、自适应 poll 超时 |
| 1.7 | 2026-03-18 | 添加 SSE 流式传输支持：并行读取+最小堆归并+流式发送到前端，修复 partition.assignment.strategy 为空字符串的问题，动态调整空轮询次数（基础10次+每1000条增加5次，上限50次） |

---

## 版本 1.7 SSE 流式传输与动态优化

### 1. partition.assignment.strategy 修复

**问题**：配置设置为空字符串 `""` 是非法的 Kafka 配置，可能导致 consumer 创建失败或行为异常。

**修复**：
```rust
// 修复前
.cfg.set("partition.assignment.strategy", "");

// 修复后
cfg.set("partition.assignment.strategy", "range");
```

### 2. 动态空轮询调整

**问题**：固定 10 次空轮询对于大数据量（如 10000 条/分区）可能不够，导致查询提前结束。

**解决方案**：
```rust
// 基础 10 次，每 1000 条消息增加 5 次，最多 50 次
let base_empty_polls = 10usize;
let additional_polls = (max_messages / 1000) * 5;
let max_empty_polls = (base_empty_polls + additional_polls).min(50);

// 示例：
// max_messages=100  → 10 次
// max_messages=1000 → 15 次
// max_messages=10000→ 60 → 上限 50 次
```

### 3. SSE 流式传输架构

#### 3.1 整体流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  分区0读取   │────→│             │     │             │
└─────────────┘     │   最小堆    │────→│  SSE批次    │────→ 前端实时显示
┌─────────────┐     │  归并排序   │     │  (100条/批) │
│  分区1读取   │────→│             │     │             │
└─────────────┘     │             │     │             │
┌─────────────┐     │             │     │             │
│  分区2读取   │────→│             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
      ↑                    ↑                  ↑
   channel              BinaryHeap        EventStream
   实时发送              动态归并           实时接收
```

#### 3.2 后端实现

**新增路由**：`/api/stream` - 专用于 SSE 流式请求

**核心组件**：

1. **流式分区读取** (`fetch_partition_messages_streaming`)：
```rust
async fn fetch_partition_messages_streaming(
    brokers: String,
    topic: String,
    partition: i32,
    max_messages: usize,
    // ... 其他参数
    tx: mpsc::Sender<KafkaMessage>,  // 通过 channel 实时发送
) {
    // 创建 consumer（唯一 group.id）
    // 计算 offset 范围
    // poll 消息并通过 tx.send() 实时发送
}
```

2. **最小堆归并** (`HeapMessage`)：
```rust
#[derive(Debug)]
struct HeapMessage {
    timestamp: Option<i64>,
    offset: i64,
    message: KafkaMessage,
}

impl Ord for HeapMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        // 按时间戳排序，时间戳相同则按 offset 排序
        match (self.timestamp, other.timestamp) {
            (Some(a), Some(b)) => a.cmp(&b).then_with(|| self.offset.cmp(&other.offset)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => self.offset.cmp(&other.offset),
        }
    }
}
```

3. **流式归并** (`fetch_messages_streaming_sse`)：
```rust
async fn fetch_messages_streaming_sse(
    // ... 参数
    sse_tx: mpsc::Sender<std::result::Result<Event, Infallible>>,
) -> Result<()> {
    // 1. 为每个分区创建 channel
    // 2. 并行启动分区读取任务
    // 3. 最小堆归并
    // 4. 每满 BATCH_SIZE (100条) 发送 SSE 事件
}
```

#### 3.3 SSE 事件类型

| 事件 | 数据格式 | 说明 |
|------|----------|------|
| `start` | `{"partitions": 3, "total_target": 30000}` | 开始获取消息 |
| `batch` | `{"messages": [...], "progress": 100, "total": 30000}` | 一批消息（100条） |
| `order` | `{"sort": "desc"}` | 排序方向通知 |
| `complete` | `{}` | 获取完成 |
| `error` | `{"error": "..."}` | 错误信息 |

#### 3.4 前端实现

**API Client 方法** (`getMessagesStream`)：
```typescript
getMessagesStream(
  clusterId: string,
  topic: string,
  params?: MessageQueryParams,
  callbacks?: {
    onStart?: (data: { partitions: number; total_target: number }) => void;
    onBatch?: (messages: MessageRecord[], progress: number, total: number) => void;
    onOrder?: (sort: string) => void;
    onComplete?: () => void;
    onError?: (error: string) => void;
  }
): AbortController
```

**使用示例**：
```typescript
const abortController = apiClient.getMessagesStream(
  clusterId,
  topic,
  { max_messages: 10000, fetchMode: 'newest' },
  {
    onStart: (data) => {
      console.log(`从 ${data.partitions} 个分区获取消息`);
      messages.value = []; // 清空列表
    },
    onBatch: (newMessages, progress, total) => {
      messages.value.push(...newMessages); // 实时追加
      console.log(`进度: ${progress}/${total}`);
    },
    onComplete: () => {
      loading.value = false;
    },
    onError: (error) => {
      showError(error);
      loading.value = false;
    }
  }
);

// 取消请求
abortController.abort();
```

#### 3.5 与传统方案对比

| 指标 | 传统一次性返回 | SSE 流式传输 |
|------|----------------|--------------|
| 首条消息时间 | 等待全部完成 | 几乎立即显示 |
| 内存占用（服务端） | 所有消息同时加载 | 堆大小 = 分区数（3条） |
| 内存占用（前端） | 所有消息同时渲染 | 流式追加，渐进渲染 |
| 用户体验 | 白屏等待 | 实时看到进度和消息 |
| 取消支持 | 部分支持 | 完全支持（AbortController）|
| 代码复杂度 | 简单 | 中等（需要处理归并和流式） |

### 4. 性能优化总结

#### 4.1 大数据量场景（3分区 × 10000条）

**优化前**：
- 内存：30000 条消息同时在内存中
- 等待时间：全部获取完成后才返回
- 超时风险：60 秒可能不够

**优化后**：
- 内存：堆中仅 3 条消息（每分区最新一条）
- 响应：首条消息立即显示，每批 100 条实时更新
- 超时：每分区独立 60 秒超时，更宽松

#### 4.2 关键技术点

1. **并行读取**：3 个分区同时读取，通过 channel 实时回传
2. **最小堆归并**：维护全局有序，O(log n) 插入/弹出
3. **批量发送**：每 100 条发送一次 SSE，减少网络开销
4. **动态空轮询**：根据数据量自动调整，避免过早退出
5. **AbortController**：支持随时取消正在进行的查询

### 5. 常见问题

#### Q10: 为什么使用 SSE 而不是 WebSocket？

**A**:
- **SSE 优势**：基于 HTTP，自动重连，支持浏览器原生 EventSource，服务器单向推送更适合"查询"场景
- **WebSocket 适用**：需要双向通信的场景（如聊天、实时协作）
- **当前场景**：消息查询是服务器单向推送结果，SSE 更轻量

#### Q11: 流式传输时前端如何保持有序？

**A**:
- 服务端通过最小堆保证全局有序（按时间戳）
- 每批消息在批次内是有序的
- 如果是降序（desc），服务端发送 `order` 事件，前端收到后整体反转列表

#### Q12: 可以取消正在进行的流式查询吗？

**A**:
- 可以，`getMessagesStream` 返回 `AbortController`
- 调用 `abortController.abort()` 即可取消
- 后端会检测到 channel 关闭并停止读取

#### Q13: 流式传输对 Kafka 集群压力更大吗？

**A**:
- **不会**，反而更小：
- 查询逻辑相同，只是发送方式不同
- 流式允许更快消费完数据，减少 consumer 持有时间
- 取消查询时 consumer 立即关闭，不继续 poll

#### Q14: 为什么批次大小设置为 100 条？

**A**:
- **平衡实时性和性能**：
- 太小（如 10 条）：网络开销大，频繁触发前端更新
- 太大（如 1000 条）：实时性降低，用户感知延迟
- 100 条是一个经验值，既能保证实时性又不会过度频繁更新 UI

---

## 版本 1.8 前端 SSE 性能优化

### 问题背景

在查询 Kafka 消息时，后端通过 SSE 流式发送最多 30000 条消息（3 分区 × 10000 条/分区），但前端遇到两个严重问题：

1. **消息数量不匹配**：后端发送 30000 条，前端只显示约 19000 条
2. **浏览器崩溃**：查询几次后浏览器崩溃，无法正常使用

### 根本原因分析

#### 问题 1：SSE 解析边界情况

原始前端代码使用逐行解析 SSE 事件：

```javascript
// 原始代码 - 逐行解析
for (const line of lines) {
  if (line.startsWith('event: ')) {
    currentEvent = line.slice(7).trim();
  } else if (line.startsWith('data: ')) {
    currentData = line.slice(6).trim();
  } else if (line === '' && currentEvent) {
    // 处理事件
  }
}
```

**问题**：当 TCP 包拆分导致 `event:` 和 `data:` 不在同一个 buffer 中时，事件解析会出错，导致部分消息丢失。

#### 问题 2：Vue 响应式性能问题

原始代码每个 batch 都触发响应式更新：

```javascript
// 原始代码 - 每次 batch 都更新
onBatch: (newMessages) => {
  messages.value = [...messages.value, ...newMessages];
}
```

**问题**：
- 60 个批次 = 60 次数组展开 + 响应式更新
- 每次更新触发虚拟滚动重计算
- Vue 深度追踪 30000 个消息对象的响应式依赖
- 内存开销巨大，导致浏览器崩溃

### 解决方案

#### 1. 改进 SSE 解析逻辑（client.ts）

使用 `\n\n` 分割完整的 SSE 事件，而不是逐行解析：

```typescript
// 改进后 - 按 \n\n 分割完整事件
const parts = buffer.split('\n\n');
buffer = parts.pop() || '';

for (const part of parts) {
  const trimmed = part.trim();
  if (!trimmed) continue;

  const lines = trimmed.split('\n');
  let event = '';
  let data = '';

  for (const line of lines) {
    if (line.startsWith('event: ')) {
      event = line.slice(7).trim();
    } else if (line.startsWith('data: ')) {
      data = line.slice(6).trim();
    }
  }

  if (!event || !data) continue;

  const parsed = JSON.parse(data);
  // 处理事件...
}
```

**优势**：
- 按完整事件边界解析，不受 TCP 包拆分影响
- 逻辑更清晰，减少边界情况
- 添加详细日志，方便调试

#### 2. 非响应式缓存 + 批量更新（Debounce 模式）

```typescript
// 非响应式缓存数组
let pendingMessages: Message[] = [];
let updateTimer: ReturnType<typeof setTimeout> | null = null;

// 批量更新函数 - debounce 模式
function scheduleUpdate() {
  // 清除之前的定时器，重新设置
  if (updateTimer) {
    clearTimeout(updateTimer);
  }
  updateTimer = setTimeout(() => {
    updateTimer = null;
    if (pendingMessages.length > 0) {
      messages.value = pendingMessages.slice();
      pendingMessages = [];
    }
  }, 100);
}

// onBatch 回调 - 只追加到缓存，不触发响应式
onBatch: (newMessages) => {
  for (const msg of newMessages) {
    pendingMessages.push(formatMessage(msg));
  }
  scheduleUpdate(); // 调度 UI 更新
}

// onComplete - 强制刷新剩余消息
onComplete: () => {
  if (updateTimer) {
    clearTimeout(updateTimer);
    updateTimer = null;
  }
  if (pendingMessages.length > 0) {
    messages.value = pendingMessages.slice();
    pendingMessages = [];
  }
  // 处理排序等...
}
```

**优势**：
- batch 到达时不触发响应式，只追加到普通数组
- 使用 debounce 模式：每次 batch 都重置定时器，保证最后一批消息后 100ms 才更新
- 每 100ms 最多更新一次 UI（而不是每个 batch 都更新）
- 保证所有消息最终都被渲染，不会丢失

#### 3. 使用 shallowRef

```typescript
// 使用 shallowRef 替代 ref
const messages = shallowRef<Message[]>([]);
```

**优势**：
- Vue 只追踪数组引用的变化，不追踪数组内部元素
- 减少响应式依赖收集开销
- 配合不可变更新（`slice()`）保证响应式正确触发

### 性能对比

| 指标 | 优化前 | 优化后 |
|------|--------|--------|
| 消息完整性 | ~19000/30000 | 30000/30000 |
| 响应式更新次数 | 60+ 次 | ~3-5 次 |
| 浏览器稳定性 | 多次查询后崩溃 | 稳定 |
| 内存开销 | 高（深度响应式） | 低（浅响应式） |

### 关键设计决策

#### 为什么选择 debounce 而不是 throttle？

- **Debounce**：每次调用都重置定时器，保证最后一次调用后 100ms 才执行
- **Throttle**：固定时间间隔执行一次

选择 debounce 的原因：
1. 消息密集到达时，减少不必要的中间更新
2. 消息稀疏到达时，也能及时更新（不会等待固定间隔）
3. 保证所有消息最终都被渲染，不会丢失

#### 为什么是 100ms？

- 人眼对 <100ms 的延迟不敏感
- 100ms 足够让多个 batch 累积，减少更新次数
- 对于 30000 条消息（约 60 个 batch），总延迟约 100-200ms，可接受

### 文件清单

- `ui/src/api/client.ts` - SSE 流式解析逻辑
- `ui/src/components/MessageQueryTool.vue` - 消息查询组件
- `ui/src/views/MessagesClassicView.vue` - 经典消息视图

---

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-03-15 | 初始文档，记录两阶段查询优化 |
| 1.1 | 2026-03-16 | 添加 Consumer 预热机制、固定 group.id 策略、性能优化成果（30x 提升）|
| 1.2 | 2026-03-16 | 添加分区数判断优化（<=5 分区用串行模式）、空轮次上限调整（100+n/10）、时间保底机制（30 秒）、Q7/Q8 常见问题 |
| 1.3 | 2026-03-16 | 添加分区边界检测优化（检测 low/high watermark）、Q9 常见问题（数据少的 topic 查询慢）|
| 1.4 | 2026-03-16 | 添加时间范围查询优化（查询 end_time offset 用于提前结束）、更新 Q7/Q9 文档 |
| 1.5 | 2026-03-16 | 修复空分区检测（high=0 的分区也要预标记为完成），添加远程模式的空分区预标记 |
| 1.6 | 2026-03-18 | 统一本地/远程模式为 `fetch_partition_messages_unified`，分区数>1 自动并行，添加分区末尾检测（msg.offset >= high_watermark - 1），修复 group.id 冲突（每个分区使用唯一 group.id），优化内存预分配、提取搜索过滤函数、避免重复获取 watermark、添加 socket 超时、自适应 poll 超时 |
| 1.7 | 2026-03-18 | 添加 SSE 流式传输支持：并行读取 + 最小堆归并 + 流式发送到前端，修复 partition.assignment.strategy 为空字符串的问题，动态调整空轮询次数（基础 10 次 + 每 1000 条增加 5 次，上限 50 次） |
| 1.8 | 2026-03-19 | 前端 SSE 性能优化：改进 SSE 解析逻辑（使用 \n\n 分割）、非响应式缓存+Debounce 批量更新、使用 shallowRef 减少内存开销，修复消息丢失和浏览器崩溃问题 |
