/// Kafka 消息查询构建器和执行器
///
/// 提供统一的查询接口，支持：
/// - fetchMode: newest/oldest/offset/timestamp
/// - 指定 partition 或全部分区
/// - 时间范围过滤
/// - 搜索 key/value
/// - per-partition 最大消息数

use crate::error::{AppError, Result};
use crate::kafka::consumer::KafkaMessage;
use crate::pool::KafkaConsumerPool;
use rdkafka::consumer::Consumer;
use rdkafka::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// 获取模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FetchMode {
    /// 从最旧的消息开始
    Oldest,
    /// 从最新的消息开始
    Newest,
    /// 从指定 offset 开始
    Offset(i64),
    /// 从指定时间戳开始
    Timestamp(i64),
}

impl FetchMode {
    pub fn from_str(s: Option<&str>, offset: Option<i64>, timestamp: Option<i64>) -> Self {
        match s {
            Some("oldest") => FetchMode::Oldest,
            Some("newest") => FetchMode::Newest,
            _ => {
                if let Some(off) = offset {
                    FetchMode::Offset(off)
                } else if let Some(ts) = timestamp {
                    FetchMode::Timestamp(ts)
                } else {
                    FetchMode::Newest // 默认
                }
            }
        }
    }
}

/// 搜索范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchScope {
    Key,
    Value,
    All,
}

impl SearchScope {
    pub fn from_str(s: Option<&str>) -> Self {
        match s {
            Some("key") => SearchScope::Key,
            Some("value") => SearchScope::Value,
            _ => SearchScope::All,
        }
    }
}

/// 搜索过滤器
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchFilter {
    pub keyword: String,
    pub scope: SearchScope,
}

impl SearchFilter {
    pub fn matches(&self, msg: &KafkaMessage) -> bool {
        let search_lower = self.keyword.to_lowercase();

        match self.scope {
            SearchScope::Key => {
                msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower))
            }
            SearchScope::Value => {
                msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower))
            }
            SearchScope::All => {
                let key_match = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower));
                let value_match = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower));
                key_match || value_match
            }
        }
    }
}

/// 时间范围
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

impl TimeRange {
    pub fn matches(&self, msg: &KafkaMessage) -> bool {
        if let Some(ts) = msg.timestamp {
            ts >= self.start && ts <= self.end
        } else {
            true // 没有时间戳的消息，默认匹配
        }
    }
}

/// 消息查询参数
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageQueryParams {
    /// 主题名称
    pub topic: String,
    /// 分区 ID，None 表示所有分区
    pub partition: Option<i32>,
    /// 获取模式
    pub fetch_mode: FetchMode,
    /// 时间范围过滤
    pub time_range: Option<TimeRange>,
    /// 搜索过滤器
    pub search: Option<SearchFilter>,
    /// 最大消息数
    pub limit: usize,
    /// 是否每个 partition 独立限制数量
    pub per_partition_max: bool,
    /// 扫描深度（用于搜索模式）
    pub scan_depth: Option<usize>,
    /// 排序方式：timestamp 或 offset
    pub order_by: OrderBy,
    /// 排序方向
    pub sort_order: SortOrder,
}

/// 排序字段
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderBy {
    #[default]
    Offset,
    Timestamp,
}

/// 排序方向
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

impl MessageQueryParams {
    /// 检查消息是否匹配过滤条件
    pub fn matches(&self, msg: &KafkaMessage) -> bool {
        // 时间范围过滤
        if let Some(ref time_range) = self.time_range {
            if !time_range.matches(msg) {
                return false;
            }
        }

        // 搜索过滤
        if let Some(ref search) = self.search {
            if !search.matches(msg) {
                return false;
            }
        }

        true
    }

    /// 计算扫描深度
    pub fn calculate_scan_depth(&self, partition_count: usize) -> usize {
        if let Some(depth) = self.scan_depth {
            return depth;
        }

        let has_search = self.search.is_some();
        let base = if has_search { 1000 } else { 100 };

        if self.per_partition_max {
            // Per-partition 模式：每个分区独立扫描
            self.limit.max(base)
        } else {
            // 所有分区共享 limit：平均分配
            (self.limit / partition_count.max(1)).max(base)
        }
    }
}

/// 查询统计信息
#[derive(Debug, Clone, Default, Serialize)]
pub struct QueryStats {
    /// 查询的分区数量
    pub partitions_queried: usize,
    /// 扫描的总消息数
    pub total_scanned: usize,
    /// 匹配的消息数
    pub matched: usize,
    /// 查询耗时（毫秒）
    pub query_time_ms: u64,
    /// 每个分区的统计
    pub per_partition_stats: HashMap<i32, PartitionQueryStats>,
}

/// 分区查询统计
#[derive(Debug, Clone, Default, Serialize)]
pub struct PartitionQueryStats {
    pub scanned: usize,
    pub matched: usize,
    pub start_offset: i64,
    pub end_offset: i64,
}

/// 消息查询执行器
pub struct MessageQueryExecutor {
    pool: KafkaConsumerPool,
}

impl MessageQueryExecutor {
    pub fn new(pool: KafkaConsumerPool) -> Self {
        Self { pool }
    }

    /// 执行消息查询
    pub async fn execute(&self, params: &MessageQueryParams) -> Result<(Vec<KafkaMessage>, QueryStats)> {
        let start_time = std::time::Instant::now();
        let mut stats = QueryStats::default();

        // 判断是否是全部分区查询
        let all_partitions = params.partition.is_none();

        let messages = if all_partitions {
            // 查询所有分区
            self.fetch_all_partitions(params, &mut stats).await?
        } else {
            // 查询单个分区
            let pid = params.partition.unwrap_or(0);
            self.fetch_single_partition(params, pid, &mut stats).await?
        };

        stats.matched = messages.len();
        stats.query_time_ms = start_time.elapsed().as_millis() as u64;

        Ok((messages, stats))
    }

    /// 获取所有分区列表
    async fn get_partition_ids(&self, topic: &str) -> Result<Vec<i32>> {
        // 从 Pool 获取一个 consumer 来查询 watermarks
        let consumer = self.pool.get().await
            .map_err(|e| AppError::Internal(format!("Failed to get consumer from pool: {}", e)))?;

        use std::time::Duration;
        let mut partition_ids = Vec::new();

        // 探测分区 0-999，直到 watermarks 查询失败
        for pid in 0..1000 {
            match consumer.fetch_watermarks(topic, pid, Duration::from_millis(100)) {
                Ok((low, high)) => {
                    partition_ids.push(pid);
                    // 如果分区为空且不是第一个分区，停止发现
                    if high <= low && pid > 0 {
                        // 继续检查下一个，确认是否真的有分区
                        // 这是为了处理稀疏分区的情况
                    }
                }
                Err(_) => {
                    // partition 不存在，停止发现
                    break;
                }
            }
        }

        Ok(partition_ids)
    }

    /// 查询所有分区
    async fn fetch_all_partitions(
        &self,
        params: &MessageQueryParams,
        stats: &mut QueryStats,
    ) -> Result<Vec<KafkaMessage>> {
        let partition_ids = self.get_partition_ids(&params.topic).await?;
        let partition_count = partition_ids.len();

        if partition_count == 0 {
            return Ok(Vec::new());
        }

        stats.partitions_queried = partition_count;

        // 计算每个分区的扫描深度
        let scan_per_partition = params.calculate_scan_depth(partition_count);

        // 限制并发度
        let semaphore = Arc::new(Semaphore::new(50));
        let mut tasks = Vec::new();

        for pid in partition_ids {
            let pool_clone = self.pool.clone();
            let params_clone = params.clone();
            let sem_clone = semaphore.clone();

            let task = tokio::spawn(async move {
                // 获取信号量许可
                let _permit = sem_clone.acquire().await
                    .map_err(|e| AppError::Internal(format!("Semaphore error: {}", e)))?;

                Self::fetch_from_partition(pool_clone, params_clone, pid, scan_per_partition).await
            });

            tasks.push(task);
        }

        // 收集所有结果
        let mut all_messages = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok((msgs, pid_stats))) => {
                    all_messages.extend(msgs);
                    if let Some((pid, pstats)) = pid_stats {
                        stats.per_partition_stats.insert(pid, pstats.clone());
                        stats.total_scanned += pstats.scanned;
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("Partition fetch error: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Task join error: {}", e);
                }
            }
        }

        // 排序
        let mut sorted = self.sort_messages(all_messages, params);

        // 限制返回数量
        if !params.per_partition_max && sorted.len() > params.limit {
            sorted.truncate(params.limit);
        }

        Ok(sorted)
    }

    /// 查询单个分区
    async fn fetch_single_partition(
        &self,
        params: &MessageQueryParams,
        partition: i32,
        stats: &mut QueryStats,
    ) -> Result<Vec<KafkaMessage>> {
        let scan_depth = params.calculate_scan_depth(1);
        let (messages, pid_stats) = Self::fetch_from_partition(
            self.pool.clone(),
            params.clone(),
            partition,
            scan_depth,
        ).await?;

        stats.partitions_queried = 1;
        if let Some((pid, pstats)) = pid_stats {
            stats.per_partition_stats.insert(pid, pstats.clone());
            stats.total_scanned += pstats.scanned;
        }

        let mut sorted = self.sort_messages(messages, params);
        if sorted.len() > params.limit {
            sorted.truncate(params.limit);
        }

        Ok(sorted)
    }

    /// 从单个分区获取消息
    async fn fetch_from_partition(
        pool: KafkaConsumerPool,
        params: MessageQueryParams,
        partition: i32,
        scan_depth: usize,
    ) -> Result<(Vec<KafkaMessage>, Option<(i32, PartitionQueryStats)>)> {
        use std::time::Duration;

        let consumer = pool.get().await
            .map_err(|e| AppError::Internal(format!("Failed to get consumer from pool: {}", e)))?;

        // 查询 watermarks
        let (low, high) = match consumer.fetch_watermarks(&params.topic, partition, Duration::from_millis(1000)) {
            Ok((l, h)) => (l, h),
            Err(_) => return Ok((Vec::new(), None)),
        };

        let mut pid_stats = PartitionQueryStats {
            scanned: 0,
            matched: 0,
            start_offset: low,
            end_offset: high,
        };

        // 根据 fetch_mode 确定起始 offset
        let start_offset = match params.fetch_mode {
            FetchMode::Oldest => low,
            FetchMode::Newest => std::cmp::max(low, high - scan_depth as i64),
            FetchMode::Offset(off) => std::cmp::max(low, off),
            FetchMode::Timestamp(ts) => {
                // 使用时间戳查询 offset
                let mut query_tpl = rdkafka::TopicPartitionList::new();
                query_tpl.add_partition_offset(&params.topic, partition, rdkafka::Offset::Offset(ts))?;

                match consumer.offsets_for_times(query_tpl, Duration::from_secs(3)) {
                    Ok(tpl) => {
                        let found_offset = tpl.elements().iter().find_map(|elem| {
                            if elem.topic() == params.topic && elem.partition() == partition {
                                if let rdkafka::Offset::Offset(off) = elem.offset() {
                                    return Some(off);
                                }
                            }
                            None
                        });
                        found_offset.map_or(std::cmp::max(low, high - scan_depth as i64), |off| std::cmp::max(low, off))
                    }
                    Err(_) => std::cmp::max(low, high - scan_depth as i64),
                }
            }
        };

        // 分配分区
        let mut tpl = rdkafka::TopicPartitionList::new();
        tpl.add_partition_offset(&params.topic, partition, rdkafka::Offset::Offset(start_offset))?;
        consumer.assign(&tpl)?;

        // 获取消息
        let mut messages = Vec::new();
        let base_timeout = if scan_depth > 5000 {
            Duration::from_millis(100)
        } else if scan_depth > 1000 {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(30)
        };

        let mut consecutive_timeouts = 0;
        let max_consecutive_timeouts = if scan_depth > 5000 { 10 } else if scan_depth > 1000 { 5 } else { 3 };

        while messages.len() < scan_depth {
            match tokio::time::timeout(base_timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    consecutive_timeouts = 0;
                    pid_stats.scanned += 1;

                    let kafka_msg = KafkaMessage {
                        partition: msg.partition(),
                        offset: msg.offset(),
                        key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                        value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                        timestamp: msg.timestamp().to_millis(),
                    };

                    // 应用过滤器
                    if params.matches(&kafka_msg) {
                        messages.push(kafka_msg);
                        pid_stats.matched += 1;
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= max_consecutive_timeouts {
                        break;
                    }
                }
            }
        }

        // 清理 partition assignment
        let empty_tpl = rdkafka::TopicPartitionList::new();
        let _ = consumer.assign(&empty_tpl);

        Ok((messages, Some((partition, pid_stats))))
    }

    /// 排序消息
    fn sort_messages(&self, messages: Vec<KafkaMessage>, params: &MessageQueryParams) -> Vec<KafkaMessage> {
        let mut sorted = messages;

        match params.order_by {
            OrderBy::Timestamp => {
                match params.sort_order {
                    SortOrder::Asc => {
                        sorted.sort_by(|a, b| {
                            match (a.timestamp, b.timestamp) {
                                (Some(ts_a), Some(ts_b)) => ts_a.cmp(&ts_b),
                                (Some(_), None) => std::cmp::Ordering::Less,
                                (None, Some(_)) => std::cmp::Ordering::Greater,
                                (None, None) => a.offset.cmp(&b.offset),
                            }
                        });
                    }
                    SortOrder::Desc => {
                        sorted.sort_by(|a, b| {
                            match (a.timestamp, b.timestamp) {
                                (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a),
                                (Some(_), None) => std::cmp::Ordering::Greater,
                                (None, Some(_)) => std::cmp::Ordering::Less,
                                (None, None) => b.offset.cmp(&a.offset),
                            }
                        });
                    }
                }
            }
            OrderBy::Offset => {
                match params.sort_order {
                    SortOrder::Asc => {
                        sorted.sort_by(|a, b| a.offset.cmp(&b.offset));
                    }
                    SortOrder::Desc => {
                        sorted.sort_by(|a, b| b.offset.cmp(&a.offset));
                    }
                }
            }
        }

        sorted
    }
}
