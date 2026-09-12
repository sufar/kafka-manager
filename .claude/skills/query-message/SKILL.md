---
skill_name: query-message
description: Kafka消息查询功能开发指南 - 单 consumer 查询引擎 + Tauri Channel 流式推送 + EOF 权威完成信号，含慢集群防丢数据机制（重试、饥饿兜底、空分区提前退出等）
tags: [kafka, message, query, api, development, streaming, tauri]
---

# Kafka消息查询开发指南

## 概述

消息查询有三条路径，全部共用同一个查询引擎 `run_message_query`（`src/api.rs`）：

1. **流式查询（主路径）**：消息界面「查询消息」按钮使用。结果通过 Tauri Channel 以 `start/batch/complete/error` 事件流式推送，前端可随时取消。
2. **非流式查询**：`message.list` / `message.export` 使用。同一引擎，sink 改为收集到 Vec 一次性返回。
3. **单条精确获取**：`message.get` 按 partition+offset seek 拉一条完整消息，用于查看列表中被截断的大 value。

> 本项目为 Tauri 桌面应用（`no_axum` 分支），已**没有 HTTP 服务**。旧文档中的 `POST /api` + `X-API-Method` 已替换为 Tauri IPC 命令。

## 关键文件

| 文件 | 说明 |
|------|------|
| `src/api.rs` | 全部后端逻辑：`dispatch_request` 分发、`start_message_list_stream`、查询引擎（`run_message_query` / `calculate_offsets_batch` / `build_query_consumer_config`）、`handle_message_get` |
| `src-tauri/src/api_commands.rs` | Tauri IPC 命令：`api_request`、`message_list_stream`、`cancel_message_list` |
| `src/kafka/consumer.rs` | `KafkaMessage` 结构定义（含 `value_truncated` 标记） |
| `ui/src/api/client.ts` | `getMessagesStream()`（流式）、`getMessages()`（非流式）、`getMessage()`（单条） |
| `ui/src/components/MessageQueryTool.vue` | 消息查询界面（「查询消息」按钮） |
| `ui/src/types/api.ts` | `MessageRecord` 等类型定义 |

## 调用链（流式）

```
MessageQueryTool.vue (查询按钮)
  → apiClient.getMessagesStream()            ui/src/api/client.ts
  → invoke('message_list_stream', {requestId, params, channel})
  → message_list_stream()                    src-tauri/src/api_commands.rs
  → start_message_list_stream()              src/api.rs
  → fetch_messages_streaming_sse()           src/api.rs  (分区解析 + start 事件 + spawn_blocking)
  → run_message_query()                      src/api.rs  (单 consumer 引擎，blocking 线程)
  → StreamBatcher::emit/flush                src/api.rs  (攒批 500 条 + try_send 背压)
```

## Tauri IPC 入口

`src-tauri/src/api_commands.rs`

- **`api_request(method, params)`**：等价于旧 `POST /api` + `X-API-Method`，转发到 `api::dispatch_request`。
- **`message_list_stream(request_id, params, channel)`**：
  - 为本次查询创建 `CancellationToken`，按 `request_id` 存入 `StreamRegistry`
  - **命令体内直接执行转发循环**（mpsc → Channel），事件流结束后命令才返回——前端 `invoke` 的 Promise 在流结束时 resolve。若 spawn 转发任务后立即返回，invoke 会提前 resolve，前端兜底逻辑补发 0 条 complete，导致界面先闪现"没数据"再渲染真实数据（2026-07-30 修复）
  - `channel.send` 失败（窗口关闭等）自动取消查询
  - **120s 超时保护**（仅作卡死兜底；引擎自身 85s 总上限，前端 90s 超时）
- **`cancel_message_list(request_id)`**：前端点取消/abort 时触发 `token.cancel()`。

## 请求参数

`src/api.rs` - `start_message_list_stream`（流式与非流式参数一致）

```rust
let cluster_id = get_string_param(&body, "cluster_id")?;
let topic = get_string_param(&body, "topic")?;
let partition = get_optional_i32_param(&body, "partition");
let offset = get_optional_i64_param(&body, "offset");        // 仅指定 partition 时生效
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

注意：`max_messages` 是**每分区**最多拉取条数（产品设计，不要改成全局总量）。

## 流式事件协议

后端通过 Channel 发送 `StreamEvent { event, data }`，**`data` 为结构化 JSON 对象**（serde_json::Value，Tauri IPC 直接传输，前端拿到即 JS 对象，**不要 JSON.parse**——旧版双重序列化已移除）：

| event | data 内容 | 说明 |
|-------|-----------|------|
| `start` | `{partitions, total_target, has_filter}` | 查询开始。`has_filter=true`（有搜索词或时间范围）时总数无法预估，前端进度按不确定模式展示 |
| `batch` | `{messages[], progress, total}` | 每攒够 **500 条**发一批；结束时补发不足一批的剩余 |
| `complete` | `{actual_total}` | 正常结束，带实际发送条数（过滤查询时远小于 total_target） |
| `error` | `{error}` | 失败 |

**desc 查询直接降序推送**（引擎用大顶堆），没有 `order` 事件，前端也不要再 reverse。

## 核心实现：单 consumer 查询引擎

`src/api.rs` - `run_message_query`（4.0 重写，替代旧的"每分区一个 consumer + mpsc + 最小堆归并"）

设计要点：

1. **单 consumer 手动 assign 所有分区**：整个查询只占 1 条 broker 连接（旧实现 N 分区 = N 个 consumer + N 条 TCP 连接）。`assign` 时直接带起始 offset，**不需要 seek**。
2. **同步阻塞实现 + `spawn_blocking` 包裹**：`consumer.poll` / `fetch_watermarks` / `offsets_for_times` 都是阻塞调用，绝不能直接跑在 tokio worker 线程上（旧实现的 bug：分区数 > CPU 核数时饿死所有并发请求）。
3. **`enable.partition.eof=true`**：PartitionEOF 是"分区已读完"的 broker 权威信号——搜索/时间范围无匹配的查询从 30s+ 饥饿等待降为秒级结束。85s 总时长 / 75s 首条消息 / 30s 饥饿仅作 EOF 未触发的兜底。
4. **K 路归并**：分区内消息按 offset 顺序到达，堆顶只有在**所有活跃分区都有候选**时才弹出（`unrepresented` 计数），保证全局按 (timestamp, offset) 有序；desc 用大顶堆直接降序输出（与旧"升序 + 前端反转"序列完全一致，含 None 时间戳排最前）。
5. **`offsets_for_times` 批量调用**：start_time / end_time 各一次 RPC 覆盖全部分区（`calculate_offsets_batch`，旧实现每分区各 2 次 RPC）。
6. **背压与取消**：`StreamBatcher` 用 `try_send` 循环 + 取消检查（**禁止 `send().await`/`blocking_send`**——channel 满时取消信号无法唤醒，旧实现因此存在死锁/task 泄漏）。

```rust
// 引擎骨架
let consumer: BaseConsumer = build_query_consumer_config(&brokers, &group_id, large_fetch).create()?;
let ranges = calculate_offsets_batch(&consumer, &topic, &partitions, ...)?;  // watermark 重试 + 批量 offsets_for_times
// 空分区/空范围直接标记完成，不参与 assign
consumer.assign(&tpl)?;
loop {
    if unrepresented == 0 { 弹出堆顶 emit; continue; }
    if active_count == 0 { 排空堆; break; }
    match consumer.poll(200ms) {
        Some(Ok(msg))  => { 范围/时间/搜索过滤 → 入堆；达 max_messages → 分区完成 }
        Some(Err(PartitionEOF(p))) => { 分区完成 }
        Some(Err(e))   => { 连续 50 次 → 返回错误 }
        None           => {}
    }
}
```

**consumer 配置统一走 `build_query_consumer_config`**（流式/非流式/单条获取共用），`large_fetch`（max_messages > 1000）切换 50MB/10MB fetch 配额。旧实现 `max.partition.fetch.bytes` 被设两次、大批量分支的 50MB 总被覆盖回 10MB——不要再复制配置代码。

**搜索零分配**：`SearchTerm`（ASCII needle 直接在原始字节上 `eq_ignore_ascii_case` 滑窗匹配；非 ASCII 回退 Unicode 小写）。旧实现每条消息 `to_lowercase()` 分配一个等长 String。

**value 截断**：流式列表路径 `truncate_value = Some(MAX_INLINE_VALUE_BYTES = 128KB)`，超出截断到 UTF-8 字符边界并置 `value_truncated=true`；前端详情面板提示"已截断"并可点按钮调 `message.get` 拉完整内容。导出/非流式路径 `truncate_value = None` 保留完整内容。

## Offset 计算

`calculate_offsets_batch`（替代旧 `calculate_partition_offset` + `calculate_time_range_offsets`）：

优先级：用户指定 `offset`（仅单分区）> 时间范围（`offsets_for_times` 批量换算）> `fetchMode`（newest: `high - max_messages`；oldest: `low`）。end offset 一律 inclusive。`start_time > end_time` → 空范围。

**慢集群防丢数据（2026-07-28 修复，仍然有效）**：

| 问题 | 旧行为 | 现状 |
|------|--------|------|
| `fetch_metadata(5s)` 超时 | `Err(_) => vec![0]`，只查 partition 0，**其余分区数据全部丢失** | `fetch_topic_partitions()`：3 次重试 × 10s，最终失败返回错误让前端报错 |
| `fetch_watermarks(5s)` 超时 | `unwrap_or((0,0))`，分区被误判为空**整个跳过** | `fetch_watermarks_with_retry()`：3 次重试 × 10s，失败传播错误；真·空分区（成功返回 0,0）行为不变 |
| 空轮询计数到上限 | 不管是否追到 high watermark 直接退出 | **PartitionEOF 权威信号**为主；30s 饥饿 / 85s 总上限仅作兜底 |
| `socket.timeout.ms=10s`（2026-08-05 修复） | 慢 broker 的 FetchRequest 超过 10s 被掐断（REQTMOUT），重试再超时，**分区永远收不到消息** | 60s（librdkafka 默认）；首条消息等待窗口 75s > socket 超时 |

## 前端调用

`ui/src/api/client.ts`

```typescript
// 流式（消息界面查询按钮）：返回 StreamHandle，abort() 取消
getMessagesStream(clusterId, topic, params, {
  onStart,    // {partitions, total_target, has_filter}
  onBatch,    // (messages, progress, total) — messages 已是对象数组
  onComplete, // {actual_total}
  onError
}): StreamHandle

// 非流式（60s 超时）
async getMessages(clusterId, topic, params): Promise<MessageRecord[]>

// 单条完整消息（查看被截断的大 value）
async getMessage(clusterId, topic, partition, offset): Promise<MessageRecord>
```

流式客户端细节：`invoke` 的 Promise 在后端事件流结束时 resolve；若流结束但未收到 `complete`/`error` 终态事件，按已收到消息数补发 `onComplete`；`abort()` 调 `cancel_message_list`。

`MessageQueryTool.vue` 渲染优化：消息先入非响应式 `pendingMessages`，定时器批量合并；**摊平合并**——pending 未达 `max(500, 现有条数 × 25%)` 不合并（避免 `[...messages, ...pending]` 全量拷贝退化为 O(n²)），`onComplete` 时强制合并。

## 类型定义

`ui/src/types/api.ts`

```typescript
export interface MessageRecord {
    partition: number;
    offset: number;
    key?: string;
    value?: string;
    timestamp?: number;
    value_truncated?: boolean;  // value 超 128KB 被截断，用 message.get 拉完整内容
}
```

## 相关API方法

| Method / 命令 | 功能 | 所在文件 |
|---------------|------|----------|
| `message_list_stream` (Tauri) | 流式查询消息 | `src-tauri/src/api_commands.rs` → `src/api.rs` |
| `cancel_message_list` (Tauri) | 取消流式查询 | `src-tauri/src/api_commands.rs` |
| `message.list` | 非流式查询消息 | `src/api.rs` (`handle_message_list`) |
| `message.get` | 单条完整消息（partition+offset 精确 seek，15s 超时） | `src/api.rs` (`handle_message_get`) |
| `message.send` | 发送消息 | `src/api.rs` (`handle_message_send`) |
| `message.export` | 导出消息 | `src/api.rs` (`handle_message_export`) |

## 开发注意事项

1. **没有 HTTP 服务**：所有 API 走 Tauri IPC（`api_request` 分发 / `message_list_stream` Channel），不要再引用 axum、`POST /api` 或 `src/routes/`（已删除）。
2. **一切阻塞调用进 `spawn_blocking`**：`consumer.poll` / `fetch_watermarks` / `offsets_for_times` / `fetch_metadata`（含 `std::thread::sleep` 的重试逻辑）都是阻塞的，直接跑在 tokio worker 上会饿死并发请求。
3. **背压处禁止 `send().await` / `blocking_send`**：channel 满时取消信号无法唤醒会死锁，用 `try_send` 循环 + 取消检查（见 `StreamBatcher::flush`）。
4. **consumer 配置只用 `build_query_consumer_config`**，不要复制粘贴（历史 bug：重复设置互相覆盖）。
5. **控制面调用必须带重试**：`fetch_metadata` / `fetch_watermarks` 用 `fetch_topic_partitions` / `fetch_watermarks_with_retry`，禁止单次超时后静默降级（会丢数据）。
6. **分区完成优先用 PartitionEOF**（`enable.partition.eof=true` 已配置），不要回退到"空轮询计数"判断；85s/75s/30s 超时仅作兜底。
7. **超时一致性**：引擎总上限 85s < 前端流式超时 90s < Tauri 兜底 120s（后端先收尾发 complete，而不是被前端杀掉）；首条消息窗口 75s 必须 > socket.timeout.ms 60s。
8. **`max_messages` 是每分区语义**（产品决定），`total_target = max_messages × 分区数`；有搜索/时间过滤时实际条数远小于此，以 `complete.actual_total` 为准。
9. **desc 查询引擎直接降序输出**，不要在前端或引擎外再 reverse。
10. **日志标识**：`[Query]`（引擎/非流式）、`[Stream]`（流式调度）。
11. **单元测试**：`api::query_engine_tests` 覆盖搜索匹配、payload 截断（含 UTF-8 边界）、堆升降序及"desc ≡ reverse(asc)"一致性，改动这些逻辑必须先跑测试。

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 4.0 | 2026-10 | 单 consumer 查询引擎重写：1 条连接替代每分区 1 个 consumer；`spawn_blocking` 修复阻塞 tokio worker；PartitionEOF 权威完成信号（搜索无匹配 30s+ → 秒级）；`offsets_for_times` 批量 RPC；try_send 背压修复取消死锁；consumer 配置统一（修复 50MB 被覆盖 bug）；搜索零分配；desc 大顶堆直推（删前端 reverse）；StreamEvent 结构化（去双重序列化）；complete 带 actual_total；value 超 128KB 截断 + `message.get` 按需拉取；前端摊平合并 |
| 3.0 | 2026-07-28 | 重写为 no_axum 现状：Tauri Channel 流式查询 + 最小堆归并 + 可取消；慢集群防丢数据（metadata/watermark 重试、空轮询退出需 caught_up\|\|starved、总时长 300s） |
| 2.0 | 2026-03-18 | 合并本地/远程模式为统一实现，分区数>1进入并行，空轮询限制1.5秒，延迟字符串转换，空分区提前退出，显式seek定位修复 |
