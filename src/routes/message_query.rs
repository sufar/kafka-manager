/// 消息查询模块 - 统一的消息查询实现
///
/// 支持的查询模式：
/// - fetchMode=newest: 从最新消息开始查询
/// - fetchMode=oldest: 从最早消息开始查询
///
/// 支持的功能：
/// - 指定 partition 或查询所有 partition
/// - 时间范围过滤 (start_time, end_time)
/// - 搜索消息内容 (key, value)
/// - per-partition max messages
/// - 按时间戳排序

use crate::error::{AppError, Result};
use crate::kafka::consumer::KafkaMessage;
use crate::pool::KafkaConsumerPool;
use rdkafka::consumer::Consumer;
use rdkafka::Message;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// 查询模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FetchMode {
    /// 从最新消息开始查询（默认）
    Newest,
    /// 从最早消息开始查询
    Oldest,
}

impl Default for FetchMode {
    fn default() -> Self {
        FetchMode::Newest
    }
}

/// 排序方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    /// 按时间戳升序
    #[serde(alias = "timestamp_asc")]
    TimestampAsc,
    /// 按时间戳降序
    #[serde(alias = "timestamp_desc")]
    TimestampDesc,
    /// 按偏移量升序
    #[serde(alias = "offset_asc")]
    OffsetAsc,
    /// 按偏移量降序
    #[serde(alias = "offset_desc")]
    OffsetDesc,
}

impl Default for SortBy {
    fn default() -> Self {
        SortBy::TimestampDesc
    }
}

/// 搜索过滤条件
#[derive(Debug, Clone, Deserialize)]
pub struct SearchFilter {
    /// 搜索关键词
    pub query: String,
    /// 搜索范围: "key", "value", "all"
    #[serde(default = "default_search_in")]
    pub search_in: String,
}

fn default_search_in() -> String {
    "all".to_string()
}

/// 消息查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct MessageQueryParams {
    /// 查询模式
    #[serde(default)]
    pub fetch_mode: FetchMode,

    /// 指定分区 (None = 查询所有分区)
    pub partition: Option<i32>,

    /// 开始时间戳 (毫秒)
    pub start_time: Option<i64>,

    /// 结束时间戳 (毫秒)
    pub end_time: Option<i64>,

    /// 搜索过滤
    pub search: Option<String>,

    /// 搜索范围: "key", "value", "all"
    #[serde(default)]
    pub search_in: Option<String>,

    /// 每个分区最大消息数
    #[serde(default = "default_max_per_partition")]
    pub max_per_partition: usize,

    /// 最终返回数量限制
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// 排序方式
    #[serde(default)]
    pub sort_by: SortBy,

    /// 偏移量起点 (可选，用于分页)
    pub offset: Option<i64>,
}

fn default_max_per_partition() -> usize {
    100
}

fn default_limit() -> usize {
    100
}

impl Default for MessageQueryParams {
    fn default() -> Self {
        Self {
            fetch_mode: FetchMode::Newest,
            partition: None,
            start_time: None,
            end_time: None,
            search: None,
            search_in: None,
            max_per_partition: 100,
            limit: 100,
            sort_by: SortBy::TimestampDesc,
            offset: None,
        }
    }
}

/// 分区信息
struct PartitionInfo {
    id: i32,
    low: i64,
    high: i64,
}

/// 消息匹配器 trait
pub trait MessageMatcher: Send + Sync {
    fn matches(&self, msg: &KafkaMessage) -> bool;
}

/// 时间范围和搜索过滤器
pub struct FilterMatcher {
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    search_in: String,
}

impl FilterMatcher {
    pub fn new(params: &MessageQueryParams) -> Self {
        Self {
            start_time: params.start_time,
            end_time: params.end_time,
            search: params.search.clone(),
            search_in: params.search_in.clone().unwrap_or_else(|| "all".to_string()),
        }
    }
}

impl MessageMatcher for FilterMatcher {
    fn matches(&self, msg: &KafkaMessage) -> bool {
        // 时间范围过滤
        if let Some(start) = self.start_time {
            if let Some(ts) = msg.timestamp {
                if ts < start {
                    return false;
                }
            }
        }
        if let Some(end) = self.end_time {
            if let Some(ts) = msg.timestamp {
                if ts > end {
                    return false;
                }
            }
        }

        // 搜索过滤
        if let Some(ref search_term) = self.search {
            let search_lower = search_term.to_lowercase();
            let matches = match self.search_in.as_str() {
                "key" => msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower)),
                "value" => msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower)),
                _ => {
                    let key_match = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower));
                    let value_match = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower));
                    key_match || value_match
                }
            };
            if !matches {
                return false;
            }
        }

        true
    }
}

/// 统一查询入口
pub async fn query_messages(
    pool: &KafkaConsumerPool,
    topic: &str,
    params: MessageQueryParams,
) -> Result<Vec<KafkaMessage>> {
    tracing::info!(
        "[query_messages] topic={}, fetch_mode={:?}, partition={:?}, limit={}",
        topic, params.fetch_mode, params.partition, params.limit
    );

    // 获取一个 consumer 用于获取 metadata
    let consumer = pool.get().await
        .map_err(|e| AppError::Internal(format!("Failed to get consumer from pool: {}", e)))?;

    // 获取分区列表
    let partitions = get_partition_list(&consumer, topic, params.partition).await?;

    if partitions.is_empty() {
        tracing::warn!("[query_messages] No partitions found for topic {}", topic);
        return Ok(Vec::new());
    }

    // 释放 consumer（后续查询会重新获取）
    drop(consumer);

    // 根据查询模式执行查询
    let messages = match params.fetch_mode {
        FetchMode::Newest => fetch_newest(pool, topic, &partitions, &params).await?,
        FetchMode::Oldest => fetch_oldest(pool, topic, &partitions, &params).await?,
    };

    // 排序并限制数量
    let sorted = sort_and_limit(messages, &params);

    tracing::info!("[query_messages] Returning {} messages", sorted.len());
    Ok(sorted)
}

/// 获取分区列表
async fn get_partition_list(
    consumer: &rdkafka::consumer::StreamConsumer,
    topic: &str,
    partition: Option<i32>,
) -> Result<Vec<i32>> {
    use rdkafka::consumer::Consumer;

    // 如果指定了分区，直接返回
    if let Some(p) = partition {
        return Ok(vec![p]);
    }

    // 获取 topic 的所有分区
    let metadata = consumer.fetch_metadata(Some(topic), Duration::from_millis(500))
        .map_err(|e| AppError::Internal(format!("Failed to fetch metadata: {}", e)))?;

    let partitions: Vec<i32> = metadata
        .topics()
        .first()
        .map(|t| t.partitions().iter().map(|p| p.id()).collect())
        .unwrap_or_default();

    Ok(partitions)
}

/// Newest 模式查询 - 从最新消息开始
async fn fetch_newest(
    pool: &KafkaConsumerPool,
    topic: &str,
    partitions: &[i32],
    params: &MessageQueryParams,
) -> Result<Vec<KafkaMessage>> {
    tracing::info!("[fetch_newest] Fetching from {} partitions", partitions.len());

    // 获取每个分区的 watermarks
    let consumer = pool.get().await
        .map_err(|e| AppError::Internal(format!("Failed to get consumer: {}", e)))?;

    let mut partition_infos: Vec<PartitionInfo> = Vec::new();
    for &pid in partitions {
        match consumer.fetch_watermarks(topic, pid, Duration::from_millis(1000)) {
            Ok((low, high)) => {
                tracing::debug!("[fetch_newest] Partition {} watermarks: low={}, high={}", pid, low, high);
                partition_infos.push(PartitionInfo { id: pid, low, high });
            }
            Err(e) => {
                tracing::warn!("[fetch_newest] Failed to get watermarks for partition {}: {}", pid, e);
            }
        }
    }
    drop(consumer);

    // 并行查询每个分区
    let semaphore = Arc::new(Semaphore::new(50)); // 最大 50 并发
    let mut tasks = Vec::new();

    for pi in partition_infos {
        let pool_clone = pool.clone();
        let topic_clone = topic.to_string();
        let params_clone = params.clone();
        let sem_clone = semaphore.clone();
        let max_per_partition = params.max_per_partition;

        // 计算起始偏移量
        let start_offset = if pi.high > pi.low {
            let scan_count = std::cmp::max(max_per_partition as i64, 100);
            std::cmp::max(pi.low, pi.high - scan_count)
        } else {
            pi.low
        };

        let task = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await
                .map_err(|e| AppError::Internal(format!("Semaphore error: {}", e)))?;

            fetch_from_partition(
                &pool_clone,
                &topic_clone,
                pi.id,
                Some(start_offset),
                max_per_partition,
                &params_clone,
            ).await
        });
        tasks.push(task);
    }

    // 收集结果
    let mut all_messages = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(msgs)) => all_messages.extend(msgs),
            Ok(Err(e)) => tracing::warn!("[fetch_newest] Partition fetch error: {}", e),
            Err(e) => tracing::warn!("[fetch_newest] Task join error: {}", e),
        }
    }

    Ok(all_messages)
}

/// Oldest 模式查询 - 从最早消息开始
async fn fetch_oldest(
    pool: &KafkaConsumerPool,
    topic: &str,
    partitions: &[i32],
    params: &MessageQueryParams,
) -> Result<Vec<KafkaMessage>> {
    tracing::info!("[fetch_oldest] Fetching from {} partitions", partitions.len());

    // 获取每个分区的 watermarks 和起始偏移量
    let consumer = pool.get().await
        .map_err(|e| AppError::Internal(format!("Failed to get consumer: {}", e)))?;

    let mut partition_offsets: Vec<(i32, Option<i64>)> = Vec::new();
    for &pid in partitions {
        // 获取 watermarks
        let watermarks = consumer.fetch_watermarks(topic, pid, Duration::from_millis(1000));

        // 计算起始偏移量
        let start_offset = if let Some(start_time) = params.start_time {
            // 使用时间戳定位偏移量
            use rdkafka::TopicPartitionList;
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(topic, pid, rdkafka::Offset::Offset(start_time))
                .map_err(|e| AppError::Internal(format!("Failed to add partition: {}", e)))?;

            match consumer.offsets_for_times(tpl, Duration::from_millis(500)) {
                Ok(result) => {
                    result.elements_for_topic(topic)
                        .iter()
                        .find(|e| e.partition() == pid)
                        .and_then(|e| e.offset().to_raw())
                        .or_else(|| watermarks.ok().map(|(low, _)| low))
                }
                Err(_) => watermarks.ok().map(|(low, _)| low),
            }
        } else if let Some(offset) = params.offset {
            Some(offset)
        } else {
            watermarks.ok().map(|(low, _)| low)
        };

        partition_offsets.push((pid, start_offset));
    }
    drop(consumer);

    // 并行查询每个分区
    let semaphore = Arc::new(Semaphore::new(50));
    let mut tasks = Vec::new();

    for (pid, start_offset) in partition_offsets {
        let pool_clone = pool.clone();
        let topic_clone = topic.to_string();
        let params_clone = params.clone();
        let sem_clone = semaphore.clone();

        let task = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await
                .map_err(|e| AppError::Internal(format!("Semaphore error: {}", e)))?;

            fetch_from_partition(
                &pool_clone,
                &topic_clone,
                pid,
                start_offset,
                params_clone.max_per_partition,
                &params_clone,
            ).await
        });
        tasks.push(task);
    }

    // 收集结果
    let mut all_messages = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(msgs)) => all_messages.extend(msgs),
            Ok(Err(e)) => tracing::warn!("[fetch_oldest] Partition fetch error: {}", e),
            Err(e) => tracing::warn!("[fetch_oldest] Task join error: {}", e),
        }
    }

    Ok(all_messages)
}

/// 从指定分区获取消息
async fn fetch_from_partition(
    pool: &KafkaConsumerPool,
    topic: &str,
    partition: i32,
    start_offset: Option<i64>,
    max_messages: usize,
    params: &MessageQueryParams,
) -> Result<Vec<KafkaMessage>> {
    use rdkafka::TopicPartitionList;

    let consumer = pool.get().await
        .map_err(|e| AppError::Internal(format!("Failed to get consumer: {}", e)))?;

    // 设置分区和偏移量
    let mut tpl = TopicPartitionList::new();
    let offset = start_offset
        .map(rdkafka::Offset::Offset)
        .unwrap_or(rdkafka::Offset::Beginning);
    tpl.add_partition_offset(topic, partition, offset)?;
    consumer.assign(&tpl)?;

    // 创建过滤器
    let matcher = FilterMatcher::new(params);

    // 读取消息
    let mut messages = Vec::new();
    let base_timeout = Duration::from_millis(30);
    let mut consecutive_timeouts = 0;
    let max_consecutive_timeouts = if max_messages > 5000 { 10 } else { 3 };

    while messages.len() < max_messages {
        match tokio::time::timeout(base_timeout, consumer.recv()).await {
            Ok(Ok(msg)) => {
                consecutive_timeouts = 0;

                let kafka_msg = KafkaMessage {
                    partition: msg.partition(),
                    offset: msg.offset(),
                    key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                    value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                    timestamp: msg.timestamp().to_millis(),
                };

                if matcher.matches(&kafka_msg) {
                    messages.push(kafka_msg);
                }
            }
            Ok(Err(e)) => {
                tracing::debug!("[fetch_from_partition] Consumer error: {}", e);
                consecutive_timeouts += 1;
                if consecutive_timeouts >= max_consecutive_timeouts {
                    break;
                }
            }
            Err(_) => {
                consecutive_timeouts += 1;
                if consecutive_timeouts >= max_consecutive_timeouts {
                    break;
                }
            }
        }
    }

    // 清理 assignment
    let empty_tpl = TopicPartitionList::new();
    let _ = consumer.assign(&empty_tpl);

    tracing::debug!(
        "[fetch_from_partition] Partition {} returned {} messages",
        partition, messages.len()
    );

    Ok(messages)
}

/// 排序并限制数量
fn sort_and_limit(mut messages: Vec<KafkaMessage>, params: &MessageQueryParams) -> Vec<KafkaMessage> {
    // 排序
    match params.sort_by {
        SortBy::TimestampAsc => {
            messages.sort_by(|a, b| {
                a.timestamp.unwrap_or(0).cmp(&b.timestamp.unwrap_or(0))
            });
        }
        SortBy::TimestampDesc => {
            messages.sort_by(|a, b| {
                b.timestamp.unwrap_or(0).cmp(&a.timestamp.unwrap_or(0))
            });
        }
        SortBy::OffsetAsc => {
            messages.sort_by(|a, b| a.offset.cmp(&b.offset));
        }
        SortBy::OffsetDesc => {
            messages.sort_by(|a, b| b.offset.cmp(&a.offset));
        }
    }

    // 限制数量
    messages.truncate(params.limit);
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_mode_default() {
        let params = MessageQueryParams::default();
        assert_eq!(params.fetch_mode, FetchMode::Newest);
    }

    #[test]
    fn test_sort_by_default() {
        let params = MessageQueryParams::default();
        assert_eq!(params.sort_by, SortBy::TimestampDesc);
    }
}