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
use std::time::Duration;

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
    tracing::info!("[fetch_newest] Fetching from {} partitions, max_per_partition={}", partitions.len(), params.max_per_partition);

    // 获取一个 consumer
    let consumer = pool.get().await
        .map_err(|e| AppError::Internal(format!("Failed to get consumer: {}", e)))?;

    // 获取每个分区的 watermarks 并计算起始偏移量
    let mut partition_offsets: Vec<(i32, i64)> = Vec::new();
    for &pid in partitions {
        match consumer.fetch_watermarks(topic, pid, Duration::from_millis(1000)) {
            Ok((low, high)) => {
                // 计算起始偏移量：从 high - max_per_partition 开始
                let start_offset = if high > low {
                    let scan_count = std::cmp::min(params.max_per_partition as i64, high - low);
                    std::cmp::max(low, high - scan_count)
                } else {
                    low
                };
                partition_offsets.push((pid, start_offset));
            }
            Err(e) => {
                tracing::warn!("[fetch_newest] Failed to get watermarks for partition {}: {}", pid, e);
            }
        }
    }

    if partition_offsets.is_empty() {
        drop(consumer);
        return Ok(Vec::new());
    }

    // 为所有分区设置起始偏移量
    use rdkafka::TopicPartitionList;
    let mut tpl = TopicPartitionList::new();
    for (pid, offset) in &partition_offsets {
        tpl.add_partition_offset(topic, *pid, rdkafka::Offset::Offset(*offset))?;
    }
    consumer.assign(&tpl)?;

    // 创建过滤器
    let matcher = FilterMatcher::new(params);

    // 读取消息 - 使用更短的超时和更积极的策略
    let total_target = params.max_per_partition * partitions.len();
    let mut messages = Vec::new();
    let timeout = Duration::from_millis(20);  // 每次等 20ms
    let mut empty_count = 0;
    let max_empty = 3;  // 连续 3 次空返回就结束

    while messages.len() < total_target && empty_count < max_empty {
        match tokio::time::timeout(timeout, consumer.recv()).await {
            Ok(Ok(msg)) => {
                empty_count = 0;
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
                tracing::debug!("[fetch_newest] Error: {}", e);
            }
            Err(_) => {
                empty_count += 1;
            }
        }
    }

    // 清理 assignment
    let empty_tpl = TopicPartitionList::new();
    let _ = consumer.assign(&empty_tpl);
    drop(consumer);

    tracing::info!("[fetch_newest] Collected {} messages", messages.len());
    Ok(messages)
}

/// Oldest 模式查询 - 从最早消息开始
async fn fetch_oldest(
    pool: &KafkaConsumerPool,
    topic: &str,
    partitions: &[i32],
    params: &MessageQueryParams,
) -> Result<Vec<KafkaMessage>> {
    tracing::info!("[fetch_oldest] Fetching from {} partitions, max_per_partition={}", partitions.len(), params.max_per_partition);

    // 获取一个 consumer
    let consumer = pool.get().await
        .map_err(|e| AppError::Internal(format!("Failed to get consumer: {}", e)))?;

    // 获取每个分区的起始偏移量
    use rdkafka::TopicPartitionList;
    let mut partition_offsets: Vec<(i32, i64)> = Vec::new();
    for &pid in partitions {
        let start_offset = if let Some(start_time) = params.start_time {
            // 使用时间戳定位偏移量
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(topic, pid, rdkafka::Offset::Offset(start_time))?;
            match consumer.offsets_for_times(tpl, Duration::from_millis(1000)) {
                Ok(result) => {
                    result.elements_for_topic(topic)
                        .iter()
                        .find(|e| e.partition() == pid)
                        .and_then(|e| e.offset().to_raw())
                }
                Err(_) => None,
            }
        } else {
            // 使用 watermarks 获取最早偏移量
            consumer.fetch_watermarks(topic, pid, Duration::from_millis(1000))
                .ok()
                .map(|(low, _)| low)
        };

        if let Some(offset) = start_offset {
            partition_offsets.push((pid, offset));
        }
    }

    if partition_offsets.is_empty() {
        drop(consumer);
        return Ok(Vec::new());
    }

    // 为所有分区设置起始偏移量
    let mut tpl = TopicPartitionList::new();
    for (pid, offset) in &partition_offsets {
        tpl.add_partition_offset(topic, *pid, rdkafka::Offset::Offset(*offset))?;
    }
    consumer.assign(&tpl)?;

    // 创建过滤器
    let matcher = FilterMatcher::new(params);

    // 读取消息
    let total_target = params.max_per_partition * partitions.len();
    let mut messages = Vec::new();
    let timeout = Duration::from_millis(20);
    let mut empty_count = 0;
    let max_empty = 3;

    while messages.len() < total_target && empty_count < max_empty {
        match tokio::time::timeout(timeout, consumer.recv()).await {
            Ok(Ok(msg)) => {
                empty_count = 0;
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
                tracing::debug!("[fetch_oldest] Error: {}", e);
            }
            Err(_) => {
                empty_count += 1;
            }
        }
    }

    // 清理 assignment
    let empty_tpl = TopicPartitionList::new();
    let _ = consumer.assign(&empty_tpl);
    drop(consumer);

    tracing::info!("[fetch_oldest] Collected {} messages", messages.len());
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