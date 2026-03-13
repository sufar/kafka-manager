/// 高性能 Kafka 消息查询模块 - 优化版
///
/// 优化策略：
/// 1. 批量查询 watermarks，减少网络往返
/// 2. 批量查询 offsets_for_times，支持多分区时间戳查询
/// 3. 预分配内存，减少 Vec 扩容
/// 4. 自适应超时，根据数据量动态调整
/// 5. 使用 Seek 操作替代 assign+offset，更高效
/// 6. 连接池复用，减少创建开销

use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use crate::models::MessageRecord;
use crate::pool::KafkaConsumerPool;
use crate::pool::kafka_consumer::KafkaConsumerManager;
use rdkafka::consumer::Consumer;
use rdkafka::message::Message as KafkaMessageTrait;
use rdkafka::TopicPartitionList;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// 消息查询参数
#[derive(Debug, Clone)]
pub struct QueryParams {
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    /// 每个分区最多获取的消息数（从Kafka获取的最大数量）
    pub max_messages: usize,
    pub fetch_mode: FetchMode,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub search: Option<String>,
    pub search_in: SearchIn,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            partition: None,
            offset: None,
            max_messages: 100,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchMode {
    Newest,
    Oldest,
    Offset,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Timestamp,
    Offset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 分区水位信息
#[derive(Debug, Clone)]
struct PartitionWatermark {
    partition: i32,
    low: i64,
    high: i64,
}

/// 消息查询器 - 优化版
pub struct MessageQuerier;

impl MessageQuerier {
    /// 执行消息查询
    pub async fn query(
        pool: &KafkaConsumerPool,
        _config: &KafkaConfig,
        topic: &str,
        params: QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        let start_time = std::time::Instant::now();

        // 1. 确定要查询的分区列表
        let partitions = Self::resolve_partitions(pool, topic, params.partition).await?;
        tracing::info!(
            "[MessageQuerier] Topic {} has {} partitions: {:?}",
            topic, partitions.len(), partitions
        );

        // 2. 批量查询水位信息（减少网络往返）
        let watermarks = Self::fetch_watermarks_batch(pool, topic, &partitions).await?;
        tracing::info!(
            "[MessageQuerier] Watermarks: {:?}",
            watermarks.iter().map(|w| (w.partition, w.low, w.high)).collect::<Vec<_>>()
        );

        // 3. 解析每个分区的起始offset
        let partition_offsets = Self::resolve_partition_offsets(
            pool, topic, &watermarks, &params
        ).await?;
        tracing::info!(
            "[MessageQuerier] Partition offsets: {:?}",
            partition_offsets
        );

        // 4. 并发查询各个分区
        let messages = if partitions.len() == 1 {
            let (partition, start_offset) = partition_offsets[0];
            let consumer = pool.get().await.map_err(|e| {
                AppError::Internal(format!("Failed to get consumer: {}", e))
            })?;
            Self::fetch_from_partition(
                &consumer, topic, partition, start_offset, params.max_messages, &params
            ).await?
        } else {
            Self::query_multiple_partitions(
                pool, topic, &partition_offsets, &params
            ).await?
        };

        // 5. 排序和限制返回数量
        let result = Self::sort_and_limit(messages, &params);

        tracing::info!(
            "[MessageQuerier] Query completed in {:?}, returned {} messages",
            start_time.elapsed(),
            result.len()
        );

        Ok(result)
    }

    /// 带重试获取消费者
    async fn get_consumer_with_retry(pool: &KafkaConsumerPool, max_retries: usize) -> Option<deadpool::managed::Object<KafkaConsumerManager>> {
        for attempt in 0..max_retries {
            match pool.get().await {
                Ok(consumer) => return Some(consumer),
                Err(e) => {
                    tracing::warn!("Failed to get consumer (attempt {}/{}): {}", attempt + 1, max_retries, e);
                    if attempt < max_retries - 1 {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        }
        None
    }

    /// 解析要查询的分区列表
    /// 优化策略：
    /// 1. 使用元数据 API 获取分区数（如果可用）
    /// 2. 限制二分查找次数（最多10次，支持1024个分区）
    /// 3. 合理设置超时，平衡速度和成功率
    async fn resolve_partitions(
        pool: &KafkaConsumerPool,
        topic: &str,
        specified: Option<i32>,
    ) -> Result<Vec<i32>> {
        if let Some(p) = specified {
            return Ok(vec![p]);
        }

        let consumer = pool.get().await.map_err(|e| {
            AppError::Internal(format!("Failed to get consumer from pool: {}", e))
        })?;

        let start = std::time::Instant::now();

        // 方法1：尝试从元数据获取分区信息（最快）
        let client = consumer.client();
        match client.fetch_metadata(Some(topic), Duration::from_secs(2)) {
            Ok(metadata) => {
                if let Some(topic_meta) = metadata.topics().iter().find(|t| t.name() == topic) {
                    let partitions: Vec<i32> = topic_meta.partitions().iter()
                        .map(|p| p.id())
                        .collect();
                    if !partitions.is_empty() {
                        tracing::info!(
                            "[resolve_partitions] Found {} partitions from metadata in {:?}",
                            partitions.len(),
                            start.elapsed()
                        );
                        drop(consumer);
                        return Ok(partitions);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch metadata for {}: {}", topic, e);
            }
        }

        // 方法2：二分查找（限制最多10次迭代，支持最多1024个分区）
        // 先找到最大分区号
        let mut max_partition = -1i32;
        let mut low = 0;
        let mut high = 1024; // 最大支持1024个分区
        let mut iterations = 0;
        const MAX_ITERATIONS: i32 = 10;

        while low < high && iterations < MAX_ITERATIONS {
            iterations += 1;
            let mid = (low + high) / 2;
            match consumer.fetch_watermarks(topic, mid, Duration::from_millis(100)) {
                Ok(_) => {
                    max_partition = mid;
                    low = mid + 1;
                }
                Err(_) => {
                    high = mid;
                }
            }
        }

        // 生成 0 到 max_partition 的所有分区
        let partitions: Vec<i32> = if max_partition >= 0 {
            (0..=max_partition).collect()
        } else {
            // 如果没找到任何分区，尝试前10个分区（快速回退）
            let mut fallback = Vec::new();
            for pid in 0..10 {
                if let Ok(_) = consumer.fetch_watermarks(topic, pid, Duration::from_millis(50)) {
                    fallback.push(pid);
                }
            }
            fallback
        };

        drop(consumer);

        tracing::info!(
            "[resolve_partitions] Found {} partitions in {:?}",
            partitions.len(),
            start.elapsed()
        );

        if partitions.is_empty() {
            Ok(vec![0])
        } else {
            Ok(partitions)
        }
    }

    /// 批量查询分区水位信息
    /// 优化：限制每个分区的超时时间，避免大topic查询太慢
    async fn fetch_watermarks_batch(
        pool: &KafkaConsumerPool,
        topic: &str,
        partitions: &[i32],
    ) -> Result<Vec<PartitionWatermark>> {
        let consumer = pool.get().await.map_err(|e| {
            AppError::Internal(format!("Failed to get consumer: {}", e))
        })?;

        let start = std::time::Instant::now();

        // 批量查询水位，根据分区数量动态调整超时
        // 分区越多，单个分区超时越短，避免整体太慢
        let timeout_per_partition = if partitions.len() > 50 {
            Duration::from_millis(50)  // 大topic，快速超时
        } else if partitions.len() > 10 {
            Duration::from_millis(100) // 中等topic
        } else {
            Duration::from_millis(200) // 小topic，给更多时间
        };

        let mut watermarks = Vec::with_capacity(partitions.len());
        let mut success_count = 0;
        let mut fail_count = 0;

        for &partition in partitions {
            match consumer.fetch_watermarks(topic, partition, timeout_per_partition) {
                Ok((low, high)) => {
                    watermarks.push(PartitionWatermark { partition, low, high });
                    success_count += 1;
                }
                Err(e) => {
                    tracing::debug!("Failed to fetch watermark for partition {}: {}", partition, e);
                    fail_count += 1;
                    // 使用默认值
                    watermarks.push(PartitionWatermark { partition, low: 0, high: 0 });
                }
            }
        }

        drop(consumer);

        tracing::info!(
            "[fetch_watermarks_batch] Fetched {} watermarks (success: {}, fail: {}) in {:?}",
            watermarks.len(),
            success_count,
            fail_count,
            start.elapsed()
        );

        Ok(watermarks)
    }

    /// 解析每个分区的起始offset
    async fn resolve_partition_offsets(
        pool: &KafkaConsumerPool,
        topic: &str,
        watermarks: &[PartitionWatermark],
        params: &QueryParams,
    ) -> Result<Vec<(i32, i64)>> {
        // 如果指定了offset，直接使用
        if let Some(offset) = params.offset {
            return Ok(watermarks.iter().map(|w| (w.partition, offset)).collect());
        }

        // 如果指定了开始时间，批量查询时间戳对应的offset
        if let Some(start_time) = params.start_time {
            return Self::resolve_offsets_by_time(pool, topic, watermarks, start_time).await;
        }

        // 根据fetch_mode确定起始offset
        let offsets: Vec<(i32, i64)> = watermarks
            .iter()
            .map(|w| {
                let offset = match params.fetch_mode {
                    FetchMode::Newest => {
                        // 从最新消息往前读
                        let max_msgs = params.max_messages as i64;
                        std::cmp::max(w.low, w.high - max_msgs)
                    }
                    FetchMode::Oldest | FetchMode::Offset => w.low,
                };
                (w.partition, offset)
            })
            .collect();

        Ok(offsets)
    }

    /// 批量根据时间戳查询offset
    async fn resolve_offsets_by_time(
        pool: &KafkaConsumerPool,
        topic: &str,
        watermarks: &[PartitionWatermark],
        timestamp_ms: i64,
    ) -> Result<Vec<(i32, i64)>> {
        let consumer = pool.get().await.map_err(|e| {
            AppError::Internal(format!("Failed to get consumer: {}", e))
        })?;

        // 构建批量查询的 TopicPartitionList
        let mut tpl = TopicPartitionList::new();
        for w in watermarks {
            tpl.add_partition_offset(topic, w.partition, rdkafka::Offset::Offset(timestamp_ms))?;
        }

        // 批量查询
        let mut offsets = HashMap::new();
        match consumer.offsets_for_times(tpl, Duration::from_secs(2)) {
            Ok(result) => {
                for elem in result.elements() {
                    if elem.topic() == topic {
                        if let rdkafka::Offset::Offset(off) = elem.offset() {
                            offsets.insert(elem.partition(), off);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("offsets_for_times failed: {}", e);
            }
        }

        drop(consumer);

        // 组装结果
        let result: Vec<(i32, i64)> = watermarks
            .iter()
            .map(|w| {
                let offset = offsets.get(&w.partition).copied().unwrap_or(w.low);
                (w.partition, offset)
            })
            .collect();

        Ok(result)
    }

    /// 查询多个分区（并行）
    async fn query_multiple_partitions(
        pool: &KafkaConsumerPool,
        topic: &str,
        partition_offsets: &[(i32, i64)],
        params: &QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        // 动态并发度：根据分区数量调整，但不超过10
        let concurrency = std::cmp::min(partition_offsets.len(), 10);
        let semaphore = Arc::new(Semaphore::new(concurrency));

        let mut tasks = Vec::with_capacity(partition_offsets.len());

        for &(partition, start_offset) in partition_offsets {
            let pool = pool.clone();
            let params = params.clone();
            let topic = topic.to_string();
            let sem = semaphore.clone();

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;

                // 带重试获取消费者
                let consumer = match Self::get_consumer_with_retry(&pool, 3).await {
                    Some(c) => c,
                    None => {
                        tracing::error!("[query_multiple_partitions] Failed to get consumer for partition {} after retries", partition);
                        return None;
                    }
                };

                tracing::debug!("[query_multiple_partitions] Fetching from partition {} at offset {}", partition, start_offset);

                let result = Self::fetch_from_partition(
                    &consumer, &topic, partition, start_offset, params.max_messages, &params
                ).await;

                match result {
                    Ok(messages) => {
                        tracing::debug!("[query_multiple_partitions] Partition {} returned {} messages", partition, messages.len());
                        Some(messages)
                    }
                    Err(e) => {
                        tracing::warn!("[query_multiple_partitions] Failed to fetch from partition {}: {}", partition, e);
                        None
                    }
                }
            });

            tasks.push((partition, task));
        }

        // 收集所有结果
        let mut all_messages = Vec::new();
        let mut success_count = 0;
        let mut fail_count = 0;

        for (partition, task) in tasks {
            match task.await {
                Ok(Some(msgs)) => {
                    all_messages.extend(msgs);
                    success_count += 1;
                }
                Ok(None) => {
                    tracing::warn!("[query_multiple_partitions] Partition {} returned no messages", partition);
                    fail_count += 1;
                }
                Err(e) => {
                    tracing::warn!("[query_multiple_partitions] Task for partition {} failed: {}", partition, e);
                    fail_count += 1;
                }
            }
        }

        tracing::info!(
            "[query_multiple_partitions] Total: {} messages from {} partitions ({} success, {} failed)",
            all_messages.len(), partition_offsets.len(), success_count, fail_count
        );

        Ok(all_messages)
    }

    /// 从指定分区获取消息
    /// 流式处理：边拉取边过滤
    /// - 时间范围（start_time/end_time）是硬性条件，超出范围的消息会被跳过
    /// - max_messages 是第二个硬性条件，满足时间范围的消息最多取 max_messages 条
    async fn fetch_from_partition(
        consumer: &rdkafka::consumer::StreamConsumer,
        topic: &str,
        partition: i32,
        start_offset: i64,
        max_messages: usize,
        params: &QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        // 使用 seek 更高效
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(start_offset))?;
        consumer.assign(&tpl)?;

        // 预分配内存
        let mut messages = Vec::with_capacity(std::cmp::min(max_messages, 500));
        let search_lower = params.search.as_ref().map(|s| s.to_lowercase());

        // 判断是否设置了时间范围
        let has_time_range = params.start_time.is_some() || params.end_time.is_some();

        // 自适应超时策略
        // 远程 Kafka 查询需要更长的超时时间（网络延迟）
        let base_timeout = Duration::from_millis(500); // 增加到 500ms
        // 大数据量时需要更多重试次数
        let max_timeouts = if has_time_range { 200 } else { 50 };
        let mut consecutive_timeouts = 0;
        let start = std::time::Instant::now();

        // 流式消费：边拉取边过滤

        loop {
            // 动态超时：保持相对稳定的超时时间
            let dynamic_timeout = base_timeout;

            match tokio::time::timeout(dynamic_timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    consecutive_timeouts = 0;
                    let timestamp = msg.timestamp().to_millis();

                    // 时间范围过滤（硬性条件）
                    // start_time：消息时间戳小于开始时间时跳过
                    if let Some(start) = params.start_time {
                        if let Some(ts) = timestamp {
                            if ts < start {
                                continue;
                            }
                        }
                    }
                    // end_time：消息时间戳大于结束时间时退出循环（时间范围结束）
                    if let Some(end) = params.end_time {
                        if let Some(ts) = timestamp {
                            if ts > end {
                                break;
                            }
                        }
                    }

                    let key = msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                    let value = msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                    // 搜索过滤（流式过滤，边拉取边判断）
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
                            continue; // 不匹配搜索条件，跳过这条消息
                        }
                    }

                    // max_messages 检查：满足所有条件的消息数达到上限时退出
                    // 注意：这里是在搜索过滤之后检查，确保返回足够的结果
                    if messages.len() >= max_messages {
                        break;
                    }

                    // 匹配成功，加入结果
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

            // 如果查询超过 30 秒（无时间范围）或 60 秒（有时间范围），强制退出
            let max_query_time = if has_time_range { 60 } else { 30 };
            if start.elapsed().as_secs() > max_query_time {
                tracing::warn!("Query timeout after {} seconds", max_query_time);
                break;
            }
        }

        // 清理分区分配
        consumer.unassign()?;

        Ok(messages)
    }

    /// 排序和限制返回数量
    /// 注意：max_messages 是 per-partition 的，这里只排序，不限制总数
    fn sort_and_limit(messages: Vec<MessageRecord>, params: &QueryParams) -> Vec<MessageRecord> {
        // 只排序，不限制数量
        // 因为 max_messages 已经在 fetch_from_partition 中作为 per-partition 限制应用了
        Self::sort_messages(messages, params)
    }

    /// TopK 选择算法
    fn topk_select(
        mut messages: Vec<MessageRecord>,
        k: usize,
        params: &QueryParams,
    ) -> Vec<MessageRecord> {
        use std::cmp::Ordering;

        // 根据排序方向选择比较函数
        let cmp = |a: &MessageRecord, b: &MessageRecord| -> Ordering {
            match (params.sort_by, params.sort_order) {
                (SortBy::Timestamp, SortOrder::Desc) => {
                    match (a.timestamp, b.timestamp) {
                        (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a),
                        (Some(_), None) => Ordering::Greater,
                        (None, Some(_)) => Ordering::Less,
                        (None, None) => b.offset.cmp(&a.offset),
                    }
                }
                (SortBy::Timestamp, SortOrder::Asc) => {
                    match (a.timestamp, b.timestamp) {
                        (Some(ts_a), Some(ts_b)) => ts_a.cmp(&ts_b),
                        (Some(_), None) => Ordering::Less,
                        (None, Some(_)) => Ordering::Greater,
                        (None, None) => a.offset.cmp(&b.offset),
                    }
                }
                (SortBy::Offset, SortOrder::Desc) => b.offset.cmp(&a.offset),
                (SortBy::Offset, SortOrder::Asc) => a.offset.cmp(&b.offset),
            }
        };

        // 使用标准库的 sort_by 进行排序
        // cmp 函数已经根据 sort_order 返回正确的比较结果
        messages.sort_by(cmp);

        messages.truncate(k);
        messages
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
