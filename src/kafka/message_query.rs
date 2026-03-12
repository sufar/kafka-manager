/// 高性能 Kafka 消息查询模块
///
/// 设计目标：
/// 1. 无需缓存也能快速查询
/// 2. 支持多种查询模式（newest/oldest/指定offset/时间范围）
/// 3. 支持指定partition或不指定（自动查询所有partition）
/// 4. 支持搜索key/value内容
/// 5. 支持per-partition最大消息数限制
/// 6. 支持按时间戳排序
/// 7. 页面不卡顿 - 流式返回，避免大数据量内存占用

use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use crate::models::MessageRecord;
use crate::pool::KafkaConsumerPool;
use rdkafka::consumer::Consumer;
use rdkafka::Message as KafkaMessageTrait;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// 消息查询参数
#[derive(Debug, Clone)]
pub struct QueryParams {
    /// 指定分区（None表示查询所有分区）
    pub partition: Option<i32>,
    /// 起始offset（None表示根据fetch_mode自动确定）
    pub offset: Option<i64>,
    /// 最大返回消息数
    pub max_messages: usize,
    /// 每个分区最大消息数（仅在查询多分区时有效）
    pub per_partition_max: usize,
    /// 获取模式
    pub fetch_mode: FetchMode,
    /// 开始时间戳（毫秒）
    pub start_time: Option<i64>,
    /// 结束时间戳（毫秒）
    pub end_time: Option<i64>,
    /// 搜索关键词
    pub search: Option<String>,
    /// 搜索范围
    pub search_in: SearchIn,
    /// 排序方式
    pub sort_by: SortBy,
    /// 排序方向
    pub sort_order: SortOrder,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            partition: None,
            offset: None,
            max_messages: 100,
            per_partition_max: 100,
            fetch_mode: FetchMode::Newest,
            start_time: None,
            end_time: None,
            search: None,
            search_in: SearchIn::All,
            sort_by: SortBy::Timestamp,
            sort_order: SortOrder::Desc,
        }
    }
}

/// 获取模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchMode {
    /// 从最新消息开始
    Newest,
    /// 从最旧消息开始
    Oldest,
    /// 从指定offset开始
    Offset,
}

/// 搜索范围
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchIn {
    Key,
    Value,
    All,
}

impl SearchIn {
    pub fn from_str(s: &str) -> Self {
        match s {
            "key" => SearchIn::Key,
            "value" => SearchIn::Value,
            _ => SearchIn::All,
        }
    }
}

/// 排序字段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Timestamp,
    Offset,
}

/// 排序方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 分区查询任务结果
#[derive(Debug)]
struct PartitionResult {
    partition: i32,
    messages: Vec<MessageRecord>,
}

/// 消息查询器
pub struct MessageQuerier;

impl MessageQuerier {
    /// 执行消息查询
    pub async fn query(
        pool: &KafkaConsumerPool,
        config: &KafkaConfig,
        topic: &str,
        params: QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        let start_time = std::time::Instant::now();

        // 1. 确定要查询的分区列表
        let partitions = Self::resolve_partitions(pool, topic, params.partition).await?;

        tracing::info!(
            "[MessageQuerier] Querying topic={}, partitions={:?}, max_messages={}, fetch_mode={:?}",
            topic, partitions, params.max_messages, params.fetch_mode
        );

        // 2. 并发查询各个分区
        let messages = if partitions.len() == 1 {
            // 单分区查询：直接查询
            Self::query_single_partition(
                pool, config, topic, partitions[0], &params
            ).await?
        } else {
            // 多分区查询：并行查询后合并
            Self::query_multiple_partitions(
                pool, config, topic, &partitions, &params
            ).await?
        };

        // 3. 排序和限制返回数量
        let result = Self::sort_and_limit(messages, &params);

        tracing::info!(
            "[MessageQuerier] Query completed in {:?}, returned {} messages",
            start_time.elapsed(),
            result.len()
        );

        Ok(result)
    }

    /// 解析要查询的分区列表
    async fn resolve_partitions(
        pool: &KafkaConsumerPool,
        topic: &str,
        specified: Option<i32>,
    ) -> Result<Vec<i32>> {
        if let Some(p) = specified {
            return Ok(vec![p]);
        }

        // 从消费者池获取一个消费者来查询分区信息
        let consumer = pool.get().await.map_err(|e| {
            AppError::Internal(format!("Failed to get consumer from pool: {}", e))
        })?;

        let mut partitions = Vec::new();

        // 尝试发现分区（最多尝试1000个分区）
        for pid in 0..1000 {
            match consumer.fetch_watermarks(topic, pid, Duration::from_millis(100)) {
                Ok(_) => {
                    partitions.push(pid);
                }
                Err(_) => {
                    // 分区不存在，停止发现
                    break;
                }
            }
        }

        // 释放消费者回池中
        drop(consumer);

        if partitions.is_empty() {
            // 如果没发现任何分区，默认尝试分区0
            partitions.push(0);
        }

        Ok(partitions)
    }

    /// 查询单个分区
    async fn query_single_partition(
        pool: &KafkaConsumerPool,
        config: &KafkaConfig,
        topic: &str,
        partition: i32,
        params: &QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        let consumer = pool.get().await.map_err(|e| {
            AppError::Internal(format!("Failed to get consumer from pool: {}", e))
        })?;

        // 确定起始offset
        let start_offset = Self::resolve_start_offset(
            &consumer, topic, partition, params
        ).await?;

        // 确定最大扫描消息数
        let max_scan = params.max_messages;

        // 执行查询
        let messages = Self::fetch_from_partition(
            &consumer, topic, partition, start_offset, max_scan, params
        ).await?;

        Ok(messages)
    }

    /// 查询多个分区（并行）
    async fn query_multiple_partitions(
        pool: &KafkaConsumerPool,
        config: &KafkaConfig,
        topic: &str,
        partitions: &[i32],
        params: &QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        // 限制并发数，避免耗尽连接池
        let concurrency = std::cmp::min(partitions.len(), 10);
        let semaphore = Arc::new(Semaphore::new(concurrency));

        let mut tasks = Vec::new();

        for &partition in partitions {
            let pool = pool.clone();
            let params = params.clone();
            let topic = topic.to_string();
            let sem = semaphore.clone();

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;

                let consumer = pool.get().await.ok()?;

                let start_offset = Self::resolve_start_offset(
                    &consumer, &topic, partition, &params
                ).await.ok()?;

                let max_scan = params.per_partition_max;

                let messages = Self::fetch_from_partition(
                    &consumer, &topic, partition, start_offset, max_scan, &params
                ).await.ok()?;

                Some(PartitionResult { partition, messages })
            });

            tasks.push(task);
        }

        // 收集所有结果
        let mut all_messages = Vec::new();
        for task in tasks {
            if let Ok(Some(result)) = task.await {
                all_messages.extend(result.messages);
            }
        }

        Ok(all_messages)
    }

    /// 解析起始offset
    async fn resolve_start_offset(
        consumer: &rdkafka::consumer::StreamConsumer,
        topic: &str,
        partition: i32,
        params: &QueryParams,
    ) -> Result<i64> {
        // 如果指定了offset，直接使用
        if let Some(offset) = params.offset {
            return Ok(offset);
        }

        // 如果指定了开始时间，使用offsets_for_times查询
        if let Some(start_time) = params.start_time {
            let mut tpl = rdkafka::TopicPartitionList::new();
            tpl.add_partition_offset(
                topic,
                partition,
                rdkafka::Offset::Offset(start_time),
            )?;

            match consumer.offsets_for_times(tpl, Duration::from_secs(3)) {
                Ok(result) => {
                    for elem in result.elements() {
                        if elem.topic() == topic && elem.partition() == partition {
                            if let rdkafka::Offset::Offset(off) = elem.offset() {
                                return Ok(off);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("offsets_for_times failed: {}", e);
                }
            }
        }

        // 根据fetch_mode确定起始offset
        match params.fetch_mode {
            FetchMode::Newest => {
                // 查询high watermark，从后往前读
                match consumer.fetch_watermarks(topic, partition, Duration::from_millis(500)) {
                    Ok((low, high)) => {
                        // 从 high - max_messages 开始，但不能小于low
                        let start = std::cmp::max(low, high - params.max_messages as i64);
                        Ok(start)
                    }
                    Err(_) => Ok(0),
                }
            }
            FetchMode::Oldest | FetchMode::Offset => {
                // 查询low watermark
                match consumer.fetch_watermarks(topic, partition, Duration::from_millis(500)) {
                    Ok((low, _)) => Ok(low),
                    Err(_) => Ok(0),
                }
            }
        }
    }

    /// 从指定分区获取消息
    async fn fetch_from_partition(
        consumer: &rdkafka::consumer::StreamConsumer,
        topic: &str,
        partition: i32,
        start_offset: i64,
        max_messages: usize,
        params: &QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        // 分配分区
        let mut tpl = rdkafka::TopicPartitionList::new();
        tpl.add_partition_offset(
            topic,
            partition,
            rdkafka::Offset::Offset(start_offset),
        )?;
        consumer.assign(&tpl)?;

        let mut messages = Vec::new();
        let search_lower = params.search.as_ref().map(|s| s.to_lowercase());

        // 超时配置：根据消息数量动态调整
        let base_timeout = if max_messages > 1000 {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(30)
        };
        let max_timeouts = if max_messages > 5000 { 10 } else { 3 };
        let mut consecutive_timeouts = 0;

        // 批量消费消息
        while messages.len() < max_messages {
            match tokio::time::timeout(base_timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    consecutive_timeouts = 0;

                    let timestamp = msg.timestamp().to_millis();

                    // 时间范围过滤
                    if let Some(start) = params.start_time {
                        if let Some(ts) = timestamp {
                            if ts < start {
                                continue;
                            }
                        }
                    }
                    if let Some(end) = params.end_time {
                        if let Some(ts) = timestamp {
                            if ts > end {
                                // 超过了结束时间，停止读取
                                break;
                            }
                        }
                    }

                    let key = msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                    let value = msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                    // 搜索过滤
                    if let Some(ref search) = search_lower {
                        let matches = match params.search_in {
                            SearchIn::Key => {
                                key.as_ref().map_or(false, |k| k.to_lowercase().contains(search))
                            }
                            SearchIn::Value => {
                                value.as_ref().map_or(false, |v| v.to_lowercase().contains(search))
                            }
                            SearchIn::All => {
                                let key_match = key.as_ref().map_or(false, |k| k.to_lowercase().contains(search));
                                let value_match = value.as_ref().map_or(false, |v| v.to_lowercase().contains(search));
                                key_match || value_match
                            }
                        };
                        if !matches {
                            continue;
                        }
                    }

                    messages.push(MessageRecord {
                        partition: msg.partition(),
                        offset: msg.offset(),
                        key,
                        value,
                        timestamp,
                    });
                }
                Ok(Err(e)) => {
                    tracing::warn!("Consumer error: {}", e);
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= max_timeouts {
                        break;
                    }
                }
                Err(_) => {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= max_timeouts {
                        break;
                    }
                }
            }
        }

        // 清理分区分配
        let empty_tpl = rdkafka::TopicPartitionList::new();
        let _ = consumer.assign(&empty_tpl);

        Ok(messages)
    }

    /// 排序和限制返回数量
    fn sort_and_limit(messages: Vec<MessageRecord>, params: &QueryParams) -> Vec<MessageRecord> {
        // 辅助函数：比较时间戳（升序）
        fn cmp_ts_asc(a: &MessageRecord, b: &MessageRecord) -> std::cmp::Ordering {
            match (a.timestamp, b.timestamp) {
                (Some(ts_a), Some(ts_b)) => ts_a.cmp(&ts_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.offset.cmp(&b.offset),
            }
        }

        // 辅助函数：比较时间戳（降序）
        fn cmp_ts_desc(a: &MessageRecord, b: &MessageRecord) -> std::cmp::Ordering {
            match (a.timestamp, b.timestamp) {
                (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => b.offset.cmp(&a.offset),
            }
        }

        let limit = params.max_messages;

        if messages.len() <= limit {
            // 消息数量少于限制，直接排序返回
            return Self::sort_messages(messages, params);
        }

        // 使用堆进行TopK排序，避免全量排序
        match (params.sort_by, params.sort_order) {
            (SortBy::Timestamp, SortOrder::Desc) => {
                // 按时间戳降序，取最大的limit个
                let mut heap: BinaryHeap<std::cmp::Reverse<TimestampAsc>> = BinaryHeap::new();
                for msg in messages {
                    let item = std::cmp::Reverse(TimestampAsc(msg));
                    if heap.len() < limit {
                        heap.push(item);
                    } else if let Some(&std::cmp::Reverse(ref min)) = heap.peek() {
                        // 比较时间戳，不移动原始值
                        if cmp_ts_asc(&item.0.0, &min.0) == std::cmp::Ordering::Greater {
                            heap.pop();
                            heap.push(item);
                        }
                    }
                }
                let mut result: Vec<MessageRecord> = heap.into_iter().map(|r| r.0 .0).collect();
                result.sort_by(|a, b| {
                    let ts_a = a.timestamp.unwrap_or(0);
                    let ts_b = b.timestamp.unwrap_or(0);
                    ts_b.cmp(&ts_a)
                });
                result
            }
            (SortBy::Timestamp, SortOrder::Asc) => {
                // 按时间戳升序，取最小的limit个
                let mut heap: BinaryHeap<TimestampDesc> = BinaryHeap::new();
                for msg in messages {
                    let item = TimestampDesc(msg);
                    if heap.len() < limit {
                        heap.push(item);
                    } else if let Some(max) = heap.peek() {
                        // 比较时间戳，不移动原始值
                        if cmp_ts_desc(&item.0, &max.0) == std::cmp::Ordering::Less {
                            heap.pop();
                            heap.push(item);
                        }
                    }
                }
                let mut result: Vec<MessageRecord> = heap.into_iter().map(|r| r.0).collect();
                result.sort_by(|a, b| {
                    let ts_a = a.timestamp.unwrap_or(i64::MAX);
                    let ts_b = b.timestamp.unwrap_or(i64::MAX);
                    ts_a.cmp(&ts_b)
                });
                result
            }
            (SortBy::Offset, SortOrder::Desc) => {
                // 按offset降序
                let mut heap: BinaryHeap<std::cmp::Reverse<OffsetAsc>> = BinaryHeap::new();
                for msg in messages {
                    let item = std::cmp::Reverse(OffsetAsc(msg));
                    if heap.len() < limit {
                        heap.push(item);
                    } else if let Some(&std::cmp::Reverse(ref min)) = heap.peek() {
                        // 直接比较offset
                        if item.0.0.offset > min.0.offset {
                            heap.pop();
                            heap.push(item);
                        }
                    }
                }
                let mut result: Vec<MessageRecord> = heap.into_iter().map(|r| r.0 .0).collect();
                result.sort_by(|a, b| b.offset.cmp(&a.offset));
                result
            }
            (SortBy::Offset, SortOrder::Asc) => {
                // 按offset升序
                let mut heap: BinaryHeap<OffsetDesc> = BinaryHeap::new();
                for msg in messages {
                    let item = OffsetDesc(msg);
                    if heap.len() < limit {
                        heap.push(item);
                    } else if let Some(max) = heap.peek() {
                        // 直接比较offset
                        if item.0.offset < max.0.offset {
                            heap.pop();
                            heap.push(item);
                        }
                    }
                }
                let mut result: Vec<MessageRecord> = heap.into_iter().map(|r| r.0).collect();
                result.sort_by(|a, b| a.offset.cmp(&b.offset));
                result
            }
        }
    }

    /// 对消息进行排序
    fn sort_messages(mut messages: Vec<MessageRecord>, params: &QueryParams) -> Vec<MessageRecord> {
        match (params.sort_by, params.sort_order) {
            (SortBy::Timestamp, SortOrder::Desc) => {
                messages.sort_by(|a, b| {
                    match (a.timestamp, b.timestamp) {
                        (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a),
                        (Some(_), None) => std::cmp::Ordering::Greater,
                        (None, Some(_)) => std::cmp::Ordering::Less,
                        (None, None) => b.offset.cmp(&a.offset),
                    }
                });
            }
            (SortBy::Timestamp, SortOrder::Asc) => {
                messages.sort_by(|a, b| {
                    match (a.timestamp, b.timestamp) {
                        (Some(ts_a), Some(ts_b)) => ts_a.cmp(&ts_b),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.offset.cmp(&b.offset),
                    }
                });
            }
            (SortBy::Offset, SortOrder::Desc) => {
                messages.sort_by(|a, b| b.offset.cmp(&a.offset));
            }
            (SortBy::Offset, SortOrder::Asc) => {
                messages.sort_by(|a, b| a.offset.cmp(&b.offset));
            }
        }
        messages
    }
}

/// 用于堆排序的包装器（时间戳升序）
#[derive(Debug, PartialEq, Eq)]
struct TimestampAsc(MessageRecord);

impl Ord for TimestampAsc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.0.timestamp, other.0.timestamp) {
            (Some(ts_a), Some(ts_b)) => ts_a.cmp(&ts_b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => self.0.offset.cmp(&other.0.offset),
        }
    }
}

impl PartialOrd for TimestampAsc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 用于堆排序的包装器（时间戳降序）
#[derive(Debug, PartialEq, Eq)]
struct TimestampDesc(MessageRecord);

impl Ord for TimestampDesc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.0.timestamp, other.0.timestamp) {
            (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => other.0.offset.cmp(&self.0.offset),
        }
    }
}

impl PartialOrd for TimestampDesc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 用于堆排序的包装器（offset升序）
#[derive(Debug, PartialEq, Eq)]
struct OffsetAsc(MessageRecord);

impl Ord for OffsetAsc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.offset.cmp(&other.0.offset)
    }
}

impl PartialOrd for OffsetAsc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 用于堆排序的包装器（offset降序）
#[derive(Debug, PartialEq, Eq)]
struct OffsetDesc(MessageRecord);

impl Ord for OffsetDesc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.offset.cmp(&self.0.offset)
    }
}

impl PartialOrd for OffsetDesc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
