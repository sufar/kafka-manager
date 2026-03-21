# Kafka Manager 消息查询设计文档

**版本**: 2026-03-21
**最后更新**: 时间戳边界修复 + 秒级精度支持

---

## 核心设计原则

### 1. Max Messages 是 Per Partition（每分区）

**重要说明**：`max_messages` 参数表示**每个分区**要获取的消息数量，而不是所有分区的总和。

例如：
- 设置 `max_messages = 100`
- Topic 有 3 个分区
- 实际最多可能返回 `100 × 3 = 300` 条消息

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
- 多分区并发读取

**为什么这样设计？**

分区数 > 1 时，并行模式可以充分利用多分区的数据并行性，每个分区独立拉取消息，最后归并排序。

---

## 时间范围查询

### 3. 基于时间戳的 Offset 定位（核心逻辑）

当指定 `start_time` 或 `end_time` 时：

1. **每个分区独立计算**：时间范围 offset 计算在每个分区上独立执行
2. **使用 Kafka offsets_for_times API**：根据时间戳查找对应的 offset
3. **时间戳单位**：毫秒（ms）

```rust
// 查询 start_time 对应的 offset
let mut tpl = TopicPartitionList::new();
tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(start_time)).ok();
match consumer.offsets_for_times(tpl, Duration::from_secs(3)) {
    Ok(r) => {
        for elem in r.elements_for_topic(topic) {
            if elem.partition() == partition {
                if let Some(offset) = elem.offset().to_raw() {
                    // offset >= 0: 正常 offset
                    // offset = -1: 时间戳晚于所有消息
                }
            }
        }
    }
    Err(_) => low, // 查询失败时使用 low watermark
}
```

### 4. Watermark 边界处理

**关键概念**：
- `low_watermark`: 分区最早可用消息的 offset
- `high_watermark`: 下一条消息的 offset（当前最大 offset + 1）
- 有效消息范围：`[low_watermark, high_watermark)`

**边界保护逻辑**：

```rust
// 1. 处理分区为空的情况（low >= high）
if low >= high {
    return Ok(TimeRangeInfo {
        start_offset: low,
        end_offset: low,
        low_watermark: low,
        high_watermark: high,
    });
}

// 2. 限制 offset 在 [low, high_offset] 范围内
let high_offset = high - 1; // 有效最大 offset
found_offset = offset.clamp(low, high_offset);

// 3. offsets_for_times 返回 -1 的处理
if offset >= 0 {
    found_offset = offset.clamp(low, high_offset);
} else {
    // 时间戳晚于所有消息，使用 high_offset
    found_offset = high_offset;
}
```

### 5. 时间范围有效性检查

```rust
// 1. 检查 start_time > end_time
if start_time > end_time {
    return Ok(TimeRangeInfo {
        start_offset: low,
        end_offset: low,
        ...
    });
}

// 2. 确保 start_offset <= end_offset
let final_start = start_offset.min(end_offset);
let final_end = end_offset.max(start_offset);
```

### 6. Poll 时的边界检查

**完整检查逻辑**：

```rust
let msg_offset = msg.offset();

// 检查起始 offset
if msg_offset < start_offset {
    continue; // 还没到有效范围
}

// 检查结束 offset
if let Some(end) = end_offset {
    if msg_offset >= end {
        break; // 超出范围，退出
    }
}

// 检查分区末尾
if msg_offset >= high_watermark - 1 {
    // 最后一条消息，时间戳过滤不通过也直接退出
    if let Some(t) = ts {
        if t < start_time { break; }
        if t > end_time { break; }
    }
    break;
}

// 时间戳过滤（升序 poll）
if let Some(t) = ts {
    if t < start_time { continue; } // 继续 poll
    if t > end_time { break; }      // 后续只会更大，退出
}
```

**关键优化点**：

| 条件 | 操作 | 原因 |
|------|------|------|
| `msg_offset < start_offset` | `continue` | 还没到有效范围 |
| `msg_offset >= end_offset` | `break` | 超出范围 |
| `ts > end_time` | `break` | 升序 poll，后续只会更大 |
| 最后一条消息且 `ts < start_time` | `break` | 已到末尾，无需继续 |
| 最后一条消息且 `ts > end_time` | `break` | 已到末尾，无需继续 |

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
    cfg.set("max.partition.fetch.bytes", "10485760");
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

## 超时与限制

### 9. 查询超时配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| 任务超时 | 60 秒 | 每个并行任务的超时时间 |
| Poll 超时 | 500ms (首次) / 150ms (后续) | 首次 poll 等待更长时间建立连接 |
| 空轮次上限 | 动态 (10-50 次) | 根据 max_messages 动态调整 |
| 总轮询时间 | 60 秒 | 轮询的最长等待时间 |

### 10. 动态空轮次上限

```rust
// 基础次数 10 次，每 1000 条消息增加 5 次，最多 50 次
let base_empty_polls = 10usize;
let additional_polls = (max_messages / 1000) * 5;
let max_empty_polls = (base_empty_polls + additional_polls).min(50);
```

---

## 前端 UI 优化

### 11. 时间输入支持秒级精度

**输入格式**：`YYYY-MM-DD HH:mm:ss`

```html
<input
  v-model="filters.startTime"
  type="text"
  class="input input-bordered input-sm w-40 font-mono"
  placeholder="YYYY-MM-DD HH:mm:ss"
  @blur="formatDateTime('startTime')"
/>
```

**格式化函数**：

```typescript
function formatDateTime(field: 'startTime' | 'endTime') {
  const input = filters[field];
  if (!input) return;

  const date = parseDateTime(input);
  if (date) {
    const pad = (n: number) => String(n).padStart(2, '0');
    filters[field] = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  }
}
```

### 12. 时间格式验证

```typescript
async function queryMessages() {
  // 验证时间格式
  if (filters.startTime) {
    const startDate = parseDateTime(filters.startTime);
    if (!startDate) {
      showError(`${t.value.messages.startTime} 格式无效`);
      return;
    }
  }

  // 验证时间范围
  if (filters.startTime && filters.endTime) {
    const startDate = parseDateTime(filters.startTime);
    const endDate = parseDateTime(filters.endTime);
    if (startDate && endDate && startDate > endDate) {
      showError(`${t.value.messages.startTime} 不能大于 ${t.value.messages.endTime}`);
      return;
    }
  }

  // 发送请求...
  params.start_time = startDate.getTime(); // 毫秒
  params.end_time = endDate.getTime();     // 毫秒
}
```

### 13. 快捷时间选择

提供预设时间范围按钮：
- 5 分钟
- 15 分钟
- 30 分钟
- 1 小时
- 1 天

---

## 性能优化

### 14. 关键优化点

| 优化点 | 说明 |
|--------|------|
| **唯一 group.id** | 每个分区使用唯一的 group.id 避免消费者组协调开销 |
| **短 session timeout** | 3 秒超时，不依赖 group 协调 |
| **显式 seek** | assign 后必须显式 seek 到指定 offset |
| **分区末尾检测** | 通过 watermark 判断分区末尾，提前退出避免空轮询 |
| **空分区预标记** | 提前标记空分区（high <= low），避免空轮询 |
| **自适应 poll 超时** | 首次 500ms 建立连接，后续 150ms 快速轮询 |
| **offset 边界保护** | 限制 offset 在 [low, high_offset] 范围内，防止 panic |
| **时间戳过滤优化** | ts > end_time 时 break 而非 continue |

### 15. 耗时对比

| 场景 | 不指定时间戳 | 指定时间戳 |
|------|-------------|-----------|
| RPC 调用 | 1 次 (fetch_watermarks) | 3 次 (1+2) |
| offsets_for_times | 无 | 2 次，每次最多 3 秒超时 |
| 典型耗时 | <1 秒 | 2-10 秒 |

---

## SSE 流式传输

### 整体流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  分区 0 读取   │────→│   最小堆    │────→│  SSE 批次    │────→ 前端实时显示
└─────────────┘     │  归并排序   │     │  (500 条/批) │
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

### SSE 事件类型

| 事件 | 数据格式 | 说明 |
|------|----------|------|
| `start` | `{"partitions": 3, "total_target": 300}` | 开始获取消息 |
| `batch` | `{"messages": [...], "progress": 100, "total": 300}` | 一批消息（500 条） |
| `order` | `{"sort": "desc"}` | 排序方向通知 |
| `complete` | `{}` | 获取完成 |
| `error` | `{"error": "..."}` | 错误信息 |

---

## 常见问题

### Q1: 为什么设置了 max_messages=100，但返回了 300 条？

**A**: `max_messages` 是 **per partition** 的。如果 Topic 有 3 个分区，每个分区获取 100 条，总共最多可能返回 300 条。

### Q2: offsets_for_times 返回 -1 是什么意思？

**A**: 表示时间戳晚于该分区的所有消息。处理方式：
- `start_time` 返回 -1：使用 `high_offset`（分区最新 offset）
- `end_time` 返回 -1：使用 `high_offset`（分区最新 offset）

### Q3: 为什么时间戳查询比不指定时间戳慢？

**A**: 主要原因：
1. 额外 2 次 `offsets_for_times` RPC 调用（每次最多 3 秒）
2. 时间戳转换的不确定性（Kafka 内部索引查找）
3. 二次过滤导致更多无效 poll

### Q4: 如何避免 clamp() panic？

**A**: 必须在调用 `clamp(low, high_offset)` 前确保：
1. 分区不为空（`low < high`）
2. `high_offset = high - 1`（不为负数）
3. 处理 `offsets_for_times` 返回 -1 的情况

---

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/routes/unified.rs` | SSE 请求处理器 + offset 计算逻辑 |
| `src/kafka/consumer.rs` | Kafka 消费者实现 |
| `ui/src/api/client.ts` | SSE 客户端 |
| `ui/src/components/MessageQueryTool.vue` | 消息查询工具（支持秒级时间输入） |
