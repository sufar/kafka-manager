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
use crate::pool::{KafkaConsumerPool, kafka_consumer::KafkaConsumerManager};
use rdkafka::consumer::Consumer;
use rdkafka::message::Message as KafkaMessageTrait;
use rdkafka::TopicPartitionList;
use std::collections::HashMap;
use std::time::Duration;

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
                        // 验证分区：使用 watermarks 检查 metadata 返回的分区是否有效
                        let mut valid_partitions = Vec::new();
                        for &pid in &partitions {
                            match consumer.fetch_watermarks(topic, pid, Duration::from_millis(500)) {
                                Ok(_) => valid_partitions.push(pid),
                                Err(e) => tracing::warn!("Partition {} failed watermarks check: {}", pid, e),
                            }
                        }
                        tracing::info!(
                            "[resolve_partitions] Found {} partitions from metadata in {:?}: {:?} (valid: {})",
                            partitions.len(), start.elapsed(), partitions, valid_partitions.len()
                        );
                        // 使用 metadata 返回的所有分区，即使 watermarks 验证失败
                        // watermarks 失败可能是暂时的，不影响分区存在的事实
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

    /// 查询多个分区（全并行）
    /// 场景：最多10个分区，单topic查询，无高并发
    /// 策略：所有分区同时查询，不限制并发度
    async fn query_multiple_partitions(
        pool: &KafkaConsumerPool,
        topic: &str,
        partition_offsets: &[(i32, i64)],
        params: &QueryParams,
    ) -> Result<Vec<MessageRecord>> {

        // 全并行：所有分区同时查询
        let futures: Vec<_> = partition_offsets
            .iter()
            .map(|&(partition, start_offset)| {
                let pool = pool.clone();
                let params = params.clone();
                let topic = topic.to_string();

                async move {
                    // 获取消费者，带重试
                    let consumer = match Self::get_consumer_with_retry(&pool, partition, 3).await {
                        Some(c) => c,
                        None => {
                            tracing::error!("Failed to get consumer for partition {} after retries", partition);
                            return Vec::new();
                        }
                    };

                    // 查询该分区
                    match Self::fetch_from_partition(
                        &*consumer, &topic, partition, start_offset, params.max_messages, &params
                    ).await {
                        Ok(messages) => {
                            tracing::debug!("Partition {} returned {} messages", partition, messages.len());
                            messages
                        }
                        Err(e) => {
                            tracing::warn!("Failed to fetch from partition {}: {}", partition, e);
                            Vec::new()
                        }
                    }
                }
            })
            .collect();

        // 等待所有分区查询完成
        let results: Vec<Vec<MessageRecord>> = futures::future::join_all(futures).await;

        // 合并结果
        let mut all_messages = Vec::new();
        for msgs in results {
            all_messages.extend(msgs);
        }

        tracing::info!(
            "[query_multiple_partitions] Total: {} messages from {} partitions",
            all_messages.len(),
            partition_offsets.len()
        );

        Ok(all_messages)
    }

    /// 带重试的消费者获取
    async fn get_consumer_with_retry(
        pool: &KafkaConsumerPool,
        partition: i32,
        max_retries: u32,
    ) -> Option<deadpool::managed::Object<KafkaConsumerManager>> {
        for attempt in 1..=max_retries {
            match pool.get().await {
                Ok(c) => return Some(c),
                Err(e) => {
                    tracing::warn!(
                        "Failed to get consumer for partition {} (attempt {}/{}): {}",
                        partition, attempt, max_retries, e
                    );
                    if attempt < max_retries {
                        // 指数退避
                        tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                    }
                }
            }
        }
        None
    }

    /// 从指定分区获取消息
    /// 优化：减少超时等待，快速返回
    async fn fetch_from_partition(
        consumer: &rdkafka::consumer::StreamConsumer,
        topic: &str,
        partition: i32,
        start_offset: i64,
        max_messages: usize,
        params: &QueryParams,
    ) -> Result<Vec<MessageRecord>> {
        use rdkafka::consumer::Consumer;

        // 先 unassign 确保状态干净
        if let Err(e) = consumer.unassign() {
            tracing::warn!("Failed to unassign before fetch partition {}: {}", partition, e);
        }

        // 等待 unassign 生效
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 使用 seek 更高效
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(start_offset))?;
        consumer.assign(&tpl)?;

        // 等待 assign 生效
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut messages = Vec::with_capacity(std::cmp::min(max_messages, 100));
        let search_lower = params.search.as_ref().map(|s| s.to_lowercase());

        let has_time_range = params.start_time.is_some() || params.end_time.is_some();

        // 优化：减少超时时间，快速消费
        // 场景：最多10分区，无高并发，可以更快返回
        let timeout = if has_time_range {
            Duration::from_millis(1500) // 有时间范围时给更多时间（从 1000ms 增加）
        } else {
            Duration::from_millis(300) // 正常查询给更多时间（从 200ms 增加）
        };

        let start = std::time::Instant::now();
        let mut empty_polls = 0;
        const MAX_EMPTY_POLLS: i32 = 15; // 从 5 增加到 15，提高容错

        loop {
            // 检查是否已获取足够消息
            if messages.len() >= max_messages {
                break;
            }

            // 检查总耗时
            if start.elapsed().as_secs() > 5 {
                tracing::debug!("Partition {} query timeout after 5s", partition);
                break;
            }

            match tokio::time::timeout(timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    empty_polls = 0;
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
                    tracing::debug!("Consumer error in partition {}: {}", partition, e);
                    break;
                }
                Err(_) => {
                    // 超时，增加空轮询计数
                    empty_polls += 1;
                    if empty_polls >= MAX_EMPTY_POLLS {
                        tracing::debug!("Partition {}: max empty polls reached", partition);
                        break;
                    }
                }
            }
        }

        // 确保消费者状态被正确重置，使用更长的超时
        if let Err(e) = consumer.unassign() {
            tracing::warn!("Failed to unassign consumer for partition {}: {}", partition, e);
            // 即使 unassign 失败，也继续返回已获取的消息
        }

        // 短暂等待确保 unassign 生效
        tokio::time::sleep(Duration::from_millis(50)).await;

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
