# Consumer Group Last Commit Time

## 问题
获取 Consumer Group 的 last commit time（最后提交时间）是一个常见需求，但实现起来有坑。

## 方案对比

### 方案1: 直接读取 __consumer_offsets (不推荐)
- 需要解析 Kafka 内部消息格式
- 需要实现 murmur2 hash 算法定位分区
- 容易出错，版本兼容性差

### 方案2: 使用 committed_offsets API (推荐)
```rust
// 1. 获取 committed offset
let mut tpl = TopicPartitionList::new();
tpl.add_partition(topic, partition);
let committed = consumer.committed_offsets(tpl, timeout)?;

// 2. 解析 offset
let offset = committed.elements()
    .iter()
    .find(|e| e.topic() == topic && e.partition() == partition)
    .and_then(|e| match e.offset() {
        Offset::Offset(o) => Some(o),
        _ => None,
    });

// 3. 读取消息时间戳
// assign 到 offset-1 位置，poll 消息获取 timestamp
```

## 关键代码
见 `src/kafka/consumer_group.rs::get_partition_last_commit_time`

## 注意事项
1. 需要创建两个 consumer：一个用于获取 committed offset，一个用于读取消息时间戳
2. 读取时间戳时 assign 到 `offset-1`，因为 committed offset 是下一次要消费的位置
3. 使用唯一的 group.id 避免与现有 consumer group 冲突
4. 设置合理的 poll 超时时间（5秒左右）

## 调试技巧
```rust
tracing::info!("Committed offset: {} for {}/{}/{}", offset, group_id, topic, partition);
```
