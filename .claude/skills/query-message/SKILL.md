---
skill_name: query-message
description: Kafka消息查询功能开发指南 - Tauri Channel 流式查询 + 统一非流式实现，含慢集群防丢数据机制（重试、饥饿检测、空分区提前退出等）
tags: [kafka, message, query, api, development, streaming, tauri]
---

# Kafka消息查询开发指南

## 概述

消息查询有两条路径：

1. **流式查询（主路径）**：消息界面「查询消息」按钮使用。后端按分区并发拉取，最小堆归并排序，通过 Tauri Channel 以 `start/batch/complete/error` 事件流式推送，前端可随时取消。
2. **非流式统一查询**：`message.list` / `message.export` 使用。一次性返回全量结果。

> 本项目为 Tauri 桌面应用（`no_axum` 分支），已**没有 HTTP 服务**。旧文档中的 `POST /api` + `X-API-Method` 已替换为 Tauri IPC 命令。

## 关键文件

| 文件 | 说明 |
|------|------|
| `src/api.rs` | 全部后端逻辑：`dispatch_request` 分发、`start_message_list_stream`、流式/非流式拉取、offset 计算 |
| `src-tauri/src/api_commands.rs` | Tauri IPC 命令：`api_request`、`message_list_stream`、`cancel_message_list` |
| `src/kafka/consumer.rs` | `KafkaMessage` 结构定义 |
| `ui/src/api/client.ts` | `getMessagesStream()`（流式）、`getMessages()`（非流式） |
| `ui/src/components/MessageQueryTool.vue` | 消息查询界面（「查询消息」按钮） |
| `ui/src/types/api.ts` | `MessageRecord` 等类型定义 |

## 调用链（流式）

```
MessageQueryTool.vue (查询按钮)
  → apiClient.getMessagesStream()            ui/src/api/client.ts:441
  → invoke('message_list_stream', {requestId, params, channel})
  → message_list_stream()                    src-tauri/src/api_commands.rs:46
  → start_message_list_stream()              src/api.rs:415
  → fetch_messages_streaming_sse()           src/api.rs:2261   (调度 + 堆归并)
  → fetch_partition_messages_streaming()     src/api.rs:3041   (每分区一个 tokio 任务)
```

## Tauri IPC 入口

`src-tauri/src/api_commands.rs`

- **`api_request(method, params)`**：等价于旧 `POST /api` + `X-API-Method`，转发到 `api::dispatch_request`。
- **`message_list_stream(request_id, params, channel)`**：
  - 为本次查询创建 `CancellationToken`，按 `request_id` 存入 `StreamRegistry`
  - **命令体内直接执行转发循环**（mpsc → Channel），事件流结束后命令才返回——前端 `invoke` 的 Promise 在流结束时 resolve。若 spawn 转发任务后立即返回，invoke 会提前 resolve，前端兜底逻辑补发 0 条 complete，导致界面先闪现"没数据"再渲染真实数据（2026-07-30 修复）
  - `channel.send` 失败（窗口关闭等）自动取消查询
  - **300s 超时保护**（与单分区 `MAX_POLL_TIME_SECS` 一致）
- **`cancel_message_list(request_id)`**：前端点取消/abort 时触发 `token.cancel()`。

## 请求参数

`src/api.rs` - `start_message_list_stream`（流式与非流式参数一致）

```rust
let cluster_id = get_string_param(&body, "cluster_id")?;
let topic = get_string_param(&body, "topic")?;
let partition = get_optional_i32_param(&body, "partition");
let offset = get_optional_i64_param(&body, "offset");
let max_messages = get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
let limit = get_optional_i64_param(&body, "limit").map(|v| v as usize);  // 与 max_messages 等价，优先
let start_time = get_optional_i64_param(&body, "start_time");
let end_time = get_optional_i64_param(&body, "end_time");
let search = get_optional_string_param(&body, "search");
let search_in = get_optional_string_param(&body, "search_in");  // "key" | "value" | "all"
let fetch_mode = get_optional_string_param(&body, "fetchMode"); // "oldest" | "newest"
let sort = get_optional_string_param(&body, "sort");            // "asc" | "desc"
// 前端加载 topic 详情时已查询过分区列表，可透传（可选），后端跳过 fetch_metadata
let partitions_hint: Option<Vec<i32>> = body.get("partitions") /* ... */;
```

## 流式事件协议

后端通过 Channel 发送 `StreamEvent { event, data }`，`data` 为 JSON 字符串：

| event | data 内容 | 说明 |
|-------|-----------|------|
| `start` | `{partitions, total_target}` | 查询开始，`total_target = max_messages × 分区数` |
| `batch` | `{messages[], progress, total}` | 每攒够 **500 条**发一批；结束时补发不足一批的剩余 |
| `complete` | `{}` | 正常结束（由 `start_message_list_stream` 发送） |
| `error` | `{error}` | 失败 |

前端兼容处理 `order` 事件，但后端当前不发送（遗留）。

## 核心实现

### 1. 流式调度 + 归并

`src/api.rs` - `fetch_messages_streaming_sse`

```rust
// 1. 获取分区列表（优先级：指定 partition > 前端透传 partitions > fetch_topic_partitions 重试查询）
let partitions: Vec<i32> = if let Some(p) = partition {
    vec![p]
} else {
    match partitions_hint {
        Some(ref hint) if !hint.is_empty() => hint.clone(),  // 省掉一次 fetch_metadata
        _ => fetch_topic_partitions(&brokers, &topic)?,
    }
};

// 2. 每个分区一个 mpsc channel + 一个 tokio 任务
for &part_id in &partitions {
    let (tx, rx) = mpsc::channel::<KafkaMessage>(max_messages);
    tokio::spawn(fetch_partition_messages_streaming(
        brokers, topic, part_id, max_messages, part_offset,
        start_time, end_time, search, search_in, fetch_mode, tx, cancel_token,
    ));
}

// 3. 最小堆 K 路归并（HeapMessage 按 (timestamp, offset) 排序）
let mut heap = BinaryHeap::<Reverse<HeapMessage>>::with_capacity(partition_count);
// 先从每个分区 channel 取一条入堆，之后每弹一条就从对应分区补一条

// 4. 攒够 500 条发一个 batch 事件；tokio::select! { biased; } 优先检查取消
```

归并循环退出条件：`completed_partitions >= partition_count && heap.is_empty()`，之后补发剩余 batch、取消所有分区任务、等待任务结束。

### 2. 单分区流式拉取

`src/api.rs` - `fetch_partition_messages_streaming`

```rust
// 唯一 group.id 避免并发冲突
let unique_group_id = format!("kafka-mgr-{}-{}", partition, timestamp_ms);
cfg.set("enable.auto.commit", "false")
   .set("auto.offset.reset", "earliest");
// max_messages > 1000 时 fetch.min.bytes=64KB / fetch.max.bytes=50MB，否则小批量低延迟配置

// offset 计算（带重试，见第 3、4 节）
let time_range = calculate_partition_offset(...)?;

// 空分区提前退出（0 开销）
if high_watermark <= low_watermark { return; }
if start_offset >= high_watermark { return; }
if time_range_end > 0 && time_range_end < start_offset { return; }

// assign 后必须显式 seek
consumer.assign(&tpl)?;
consumer.seek(&topic, partition, seek_offset, Duration::from_secs(5))?;

// poll 循环退出条件：
const STARVATION_SECS: u64 = 30;
const MAX_POLL_TIME_SECS: u64 = 300;
loop {
    let caught_up = last_msg_offset.map_or(false, |o| o >= high_watermark - 1);
    let starved = last_msg_at.elapsed() >= Duration::from_secs(STARVATION_SECS);
    if sent_count >= max_messages
        || (empty_count >= max_empty_polls && (caught_up || starved))
        || poll_start.elapsed() >= Duration::from_secs(MAX_POLL_TIME_SECS)
    { break; }
    // ...
}
```

**空轮询退出必须满足 `caught_up || starved`**（2026-07-28 修复）：慢集群下 poll 经常返回 None/Err，旧逻辑空轮询计数到上限就退出，数据没取完就放弃了。现在：

- **caught_up**：已追到分区末尾（`last_msg_offset >= high_watermark - 1`），后面确实没数据 → 保持快速退出；
- **starved**：连续 30s 没收到任何消息 → 覆盖 compacted topic offset 空洞等永远追不到末尾的边界情况。

空轮询上限动态计算：基础 20 次 + 每 1000 条 +5 次，封顶 50。poll 超时自适应：首次 500ms，有数据 200ms，连续空轮询指数退避到 2s。

**就地过滤**（不匹配的消息不进 channel）：offset 范围 → 时间范围（`t > end_time` 直接 break）→ 搜索词（key/value 小写包含，`search_in` 限定 key/value/all）。分区末尾检测：`msg_offset >= high_watermark - 1` 处理完最后一条立即退出。

### 3. 非流式统一查询

`src/api.rs` - `fetch_messages_with_temp_consumer`（`message.list`、`message.export` 使用）

- 分区数 > 1：与流式相同，每分区一个任务跑 `fetch_partition_messages_streaming`（传入独立 channel + 新建 CancellationToken），最小堆归并后按 sort 排序一次性返回；
- 单分区/指定分区：`fetch_partition_messages_unified`（同步函数，`spawn_blocking` 执行），用 `RawMessage` 存原始字节、最后统一转 UTF-8（延迟字符串转换），其余逻辑（重试、饥饿检测、就地过滤）与流式一致，总时长上限 120s。

### 4. Offset 计算与慢集群重试

`calculate_partition_offset` → `calculate_time_range_offsets`，优先级：用户指定 `offset` > 时间范围（`offsets_for_times` 换算，10s 超时）> `fetchMode`（newest: `high - max_messages`；oldest: `low`）。

**慢集群防丢数据（2026-07-28 修复）**，三个曾导致静默丢数据的点：

| 问题 | 旧行为 | 现状 |
|------|--------|------|
| `fetch_metadata(5s)` 超时 | `Err(_) => vec![0]`，只查 partition 0，**其余分区数据全部丢失** | `fetch_topic_partitions()`：3 次重试 × 10s，最终失败返回错误让前端报错 |
| `fetch_watermarks(5s)` 超时 | `unwrap_or((0,0))`，分区被误判为空**整个跳过** | `fetch_watermarks_with_retry()`：3 次重试 × 10s，失败传播错误；真·空分区（成功返回 0,0）行为不变 |
| 空轮询计数到上限 | 不管是否追到 high watermark 直接退出 | 必须 `caught_up \|\| starved(30s)` 才退出；总时长 120s→300s |

```rust
fn fetch_topic_partitions(brokers: &str, topic: &str) -> Result<Vec<i32>>;
fn fetch_watermarks_with_retry(
    consumer: &BaseConsumer, topic: &str, partition: i32,
) -> std::result::Result<(i64, i64), rdkafka::error::KafkaError>;
```

## 关键优化点

| 优化项 | 实现 | 效果 |
|--------|------|------|
| **流式推送** | Tauri Channel + 每 500 条一个 batch | 大数据量即时可见，内存可控 |
| **可取消** | `CancellationToken` + `biased select` 优先检查 | 前端取消/关窗立即停拉取 |
| **最小堆归并** | 每分区先取一条入堆，弹一条补一条 | 多分区按时间戳有序，O(log P) |
| **显式 Seek 定位** | `assign()` 后必须 `seek()` | 确保从正确 offset 开始消费 |
| **唯一 group.id** | `kafka-mgr-{partition}-{毫秒时间戳}` | 避免并发冲突 |
| **空分区提前退出** | high≤low / start≥high / end<start | 空分区 0ms 返回 |
| **分区末尾检测** | `offset >= high_watermark - 1` | 数据取完立即退出 |
| **饥饿检测** | 30s 无消息才允许空轮询退出 | 慢集群不丢数据，compacted topic 不死等 |
| **metadata/watermark 重试** | 3 次 × 10s | 慢集群控制面调用不再误判 |
| **搜索就地过滤** | `message_matches_search`，不匹配不进 channel | 减少内存占用 |
| **延迟字符串转换** | 非流式路径先存字节，最后统一转 UTF-8 | 减少不必要分配 |
| **自适应 poll 退避** | 首次 500ms，基础 200ms，指数退避至 2s | 快主题低延迟，慢主题不空转 |

## 前端调用

`ui/src/api/client.ts`

```typescript
// 流式（消息界面查询按钮）：返回 StreamHandle，abort() 取消
getMessagesStream(clusterId, topic, params, {
  onStart, onBatch, onOrder, onComplete, onError
}): StreamHandle

// 非流式（60s 超时）
async getMessages(clusterId, topic, params): Promise<MessageRecord[]>
```

流式客户端细节：`invoke` 的 Promise 在后端事件流结束时 resolve；若流结束但未收到 `complete`/`error` 终态事件，按已收到消息数补发 `onComplete`；`abort()` 调 `cancel_message_list`。

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

| Method / 命令 | 功能 | 所在文件 |
|---------------|------|----------|
| `message_list_stream` (Tauri) | 流式查询消息 | `src-tauri/src/api_commands.rs` → `src/api.rs` |
| `cancel_message_list` (Tauri) | 取消流式查询 | `src-tauri/src/api_commands.rs` |
| `message.list` | 非流式查询消息 | `src/api.rs` (`handle_message_list`) |
| `message.send` | 发送消息 | `src/api.rs` (`handle_message_send`) |
| `message.export` | 导出消息 | `src/api.rs` (`handle_message_export`) |

## 开发注意事项

1. **没有 HTTP 服务**：所有 API 走 Tauri IPC（`api_request` 分发 / `message_list_stream` Channel），不要再引用 axum、`POST /api` 或 `src/routes/`（已删除）。
2. **显式 Seek**：`assign()` 后必须调用 `seek()`，否则 consumer 不会定位到指定 offset。
3. **控制面调用必须带重试**：`fetch_metadata` / `fetch_watermarks` 用 `fetch_topic_partitions` / `fetch_watermarks_with_retry`，禁止单次 5s 超时后静默降级（会丢数据）。
4. **空轮询退出必须判断 `caught_up || starved`**：不能只看空轮询计数，否则慢集群数据没取完就放弃。
5. **超时一致性**：单分区 `MAX_POLL_TIME_SECS`（流式 300s / 非流式 120s）与 Tauri 侧 300s 保护、前端非流式 60s 超时要一起考虑。
6. **唯一 group.id**：每个分区任务使用 `kafka-mgr-{partition}-{毫秒时间戳}`，避免并发冲突。
7. **取消传播**：流式路径所有等待点（poll 循环、batch 发送、channel 接收）都要检查 `cancel_token`。
8. **日志标识**：`[SSE Stream]`（调度归并）、`[Streaming]`（流式单分区）、`[Unified]` / `[Unified Partition]`（非流式）。
9. **真正空的分区**（watermark 成功返回 0,0）走提前退出，行为与重试修复前一致，不会误报错误。

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 3.0 | 2026-07-28 | 重写为 no_axum 现状：Tauri Channel 流式查询 + 最小堆归并 + 可取消；慢集群防丢数据（metadata/watermark 重试、空轮询退出需 caught_up\|\|starved、总时长 300s） |
| 2.0 | 2026-03-18 | 合并本地/远程模式为统一实现，分区数>1进入并行，空轮询限制1.5秒，延迟字符串转换，空分区提前退出，显式seek定位修复 |
