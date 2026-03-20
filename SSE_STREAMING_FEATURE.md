# SSE 流式消息功能文档

## 概述

SSE (Server-Sent Events) 流式消息功能允许前端实时接收 Kafka 消息，而无需等待所有消息加载完成。这对于大数据量场景尤为重要，可以显著提升用户体验。

## 架构设计

### 后端实现 (`src/routes/unified.rs`)

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Frontend       │     │  SSE Handler │     │  Kafka Consumer │
│  (Browser)      │────▶│  (unified.rs) │────▶│  (consumer.rs)  │
│                 │◀────│               │◀────│                 │
└─────────────────┘  Event Stream      └─────────────────┘
```

**核心流程：**

1. **请求入口** - `handle_message_list_stream()` (第 390 行)
   - 解析请求参数（cluster_id, topic, partition, max_messages 等）
   - 创建 `mpsc::channel` 用于 SSE 事件传输

2. **流式处理** - `fetch_messages_streaming_sse()` (第 1681 行)
   - 并行读取多个分区的消息
   - 使用最小堆归并排序
   - 实时通过 channel 发送 SSE 事件

3. **SSE 事件类型：**
   - `start` - 流式开始，包含分区数和目标总数
   - `batch` - 批次消息数据，包含进度信息
   - `order` - 排序方式确认
   - `complete` - 流式完成
   - `error` - 错误信息

### 前端实现 (`ui/src/api/client.ts`)

**SSE 客户端实现** - `getMessagesStream()` (第 434 行)

```typescript
getMessagesStream(
  clusterId: string,
  topic: string,
  params?: { ... },
  callbacks?: {
    onStart?: (data: { partitions: number; total_target: number }) => void;
    onBatch?: (messages: MessageRecord[], progress: number, total: number) => void;
    onOrder?: (sort: string) => void;
    onComplete?: () => void;
    onError?: (error: string) => void;
  }
): AbortController
```

**关键特性：**
- 使用 `fetch` + `ReadableStream` 读取 SSE 流
- 支持 `event:` 和 `data:` 行解析
- 支持 buffer 处理（跨边界数据）
- 支持 AbortController 取消请求

## 前端 UI 组件

### MessagesClassicView.vue

**虚拟滚动集成：**
```vue
<RecycleScroller
  ref="scrollerRefDesktop"
  :items="sortedMessages"
  :item-size="16"
  key-field="uid"
  :buffer="200"
/>
```

**流式状态管理：**
```typescript
const streamingProgress = ref({
  received: 0,
  total: 0,
  isStreaming: false
});

function scheduleUpdate() {
  // 每 200ms 批量更新 UI
  messages.value = [...messages.value, ...pendingMessages];
  // 强制刷新虚拟滚动
  nextTick(() => {
    scrollerRefDesktop.value?.refresh();
  });
}
```

### MessageQueryTool.vue

简洁模式的消息查询工具，使用相同的流式架构。

## 进度条显示

**状态栏组件：**
```vue
<div v-if="streamingProgress.isStreaming && streamingProgress.total > 0">
  <div class="w-full bg-base-300 rounded-full h-1.5">
    <div
      class="bg-info h-full rounded-full transition-all duration-300"
      :style="{ width: Math.min((streamingProgress.received / streamingProgress.total) * 100, 100) + '%' }"
    ></div>
  </div>
</div>
```

## 性能优化

### 1. 节流更新 (Throttle)
- UI 更新频率：每 200ms 一次
- 避免频繁渲染导致卡顿

### 2. 虚拟滚动 (Virtual Scrolling)
- 仅渲染可见区域的消息
- `item-size`: 消息行高
- `buffer`: 预加载缓冲区大小

### 3. shallowRef 优化
```typescript
const messages = shallowRef<Message[]>([]);
```
- 避免深度响应式带来的性能开销
- 手动触发数组更新

### 4. 非响应式缓存
```typescript
let pendingMessages: Message[] = [];
```
- 批次数据先存入非响应式数组
- 批量合并时触发一次响应式更新

## 使用示例

### 简单查询
```typescript
import { apiClient } from '@/api/client';

const controller = apiClient.getMessagesStream(
  'cluster-1',
  'my-topic',
  { max_messages: 1000, fetchMode: 'newest' },
  {
    onStart: (data) => {
      console.log(`开始：${data.partitions} 分区，目标 ${data.total_target} 条消息`);
    },
    onBatch: (messages, progress, total) => {
      console.log(`批次：收到 ${messages.length} 条，进度 ${progress}/${total}`);
      // 更新 UI...
    },
    onComplete: () => {
      console.log('完成');
    },
    onError: (error) => {
      console.error(`错误：${error}`);
    }
  }
);

// 取消请求
// controller.abort();
```

### 带过滤的查询
```typescript
apiClient.getMessagesStream(
  'cluster-1',
  'my-topic',
  {
    partition: 0,           // 指定分区
    max_messages: 500,      // 每分区最大消息数
    search: 'keyword',      // 搜索关键词
    search_in: 'value',     // 搜索值字段
    start_time: 1234567890, // 开始时间戳
    end_time: 9876543210,   // 结束时间戳
    fetchMode: 'oldest',    // 从最旧消息开始
    sort: 'asc'             // 升序排列
  },
  { ...callbacks }
);
```

## 后端 SSE 事件格式

### start 事件
```json
event: start
data: {"partitions":3,"total_target":300}
```

### batch 事件
```json
event: batch
data: {"messages":[...],"progress":100,"total":300}
```

### order 事件
```json
event: order
data: {"sort":"desc"}
```

### complete 事件
```json
event: complete
data: {}
```

### error 事件
```json
event: error
data: {"error":"Connection timeout"}
```

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/routes/unified.rs` | SSE 请求处理器 |
| `src/kafka/consumer.rs` | Kafka 消费者实现 |
| `ui/src/api/client.ts` | SSE 客户端 |
| `ui/src/views/MessagesClassicView.vue` | 经典消息视图 |
| `ui/src/components/MessageQueryTool.vue` | 简洁消息查询工具 |

## 故障排查

### 常见问题

1. **进度条不更新**
   - 检查 `streamingProgress` 是否正确绑定
   - 确认 `onBatch` 回调被调用

2. **虚拟滚动不刷新**
   - 确保 `messages.value` 是新数组引用
   - 调用 `scrollerRef.value?.refresh()`

3. **SSE 连接中断**
   - 检查网络状态
   - 确认后端 `mpsc::channel` 未关闭

4. **内存泄漏**
   - 组件卸载时调用 `controller.abort()`
   - 清理 `updateTimer` 定时器

## 未来优化方向

1. **增量加载** - 支持滚动到底部时加载更多
2. **分区并行度优化** - 动态调整并发数
3. **消息压缩** - 减少传输数据量
4. **断线重连** - 自动恢复 SSE 连接
