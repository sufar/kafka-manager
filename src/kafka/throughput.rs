use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use crate::models::{
    PartitionThroughput, ThroughputStats, TopicThroughputResponse,
};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::{ClientConfig, Message, Offset};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Kafka 吞吐量计算器
pub struct KafkaThroughputCalculator {
    timeout_ms: u64,
}

impl KafkaThroughputCalculator {
    pub fn new(kafka_config: &KafkaConfig) -> Self {
        Self {
            timeout_ms: kafka_config.operation_timeout_ms as u64,
        }
    }

    /// 计算 Topic 的生产吞吐量
    pub fn calculate_topic_throughput(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
    ) -> Result<TopicThroughputResponse> {
        let consumer: BaseConsumer = self.create_consumer(kafka_config)?;
        let client = consumer.client();

        // 获取 topic 元数据
        let metadata = client
            .fetch_metadata(Some(topic), std::time::Duration::from_millis(self.timeout_ms))
            .map_err(|e| AppError::Internal(format!("Failed to fetch metadata: {}", e)))?;

        let topic_metadata = metadata
            .topics()
            .iter()
            .find(|t| t.name() == topic)
            .ok_or_else(|| AppError::NotFound(format!("Topic '{}' not found", topic)))?;

        let mut total_messages = 0i64;
        let mut partition_stats = Vec::new();
        let mut all_timestamps: Vec<i64> = Vec::new();

        for partition in topic_metadata.partitions() {
            let partition_id = partition.id();

            // 获取 watermarks
            let (low_offset, high_offset) = client
                .fetch_watermarks(topic, partition_id, std::time::Duration::from_millis(self.timeout_ms))
                .map_err(|e| AppError::Internal(format!("Failed to fetch watermarks: {}", e)))?;

            let message_count = high_offset - low_offset;
            total_messages += message_count.max(0);

            // 获取时间戳来计算速率
            let (first_time, last_time) = if message_count > 0 {
                self.get_partition_timestamps(kafka_config, topic, partition_id, low_offset, high_offset)
            } else {
                (None, None)
            };

            if let Some(first) = first_time {
                all_timestamps.push(first);
            }
            if let Some(last) = last_time {
                all_timestamps.push(last);
            }

            // 计算生产速率
            let produce_rate = if let (Some(first), Some(last)) = (first_time, last_time) {
                if last > first && message_count > 0 {
                    let duration_seconds = (last - first) as f64 / 1000.0;
                    message_count as f64 / duration_seconds
                } else {
                    0.0
                }
            } else {
                0.0
            };

            partition_stats.push(PartitionThroughput {
                partition: partition_id,
                earliest_offset: low_offset,
                latest_offset: high_offset,
                message_count: message_count.max(0),
                produce_rate,
                first_message_time: first_time,
                last_message_time: last_time,
            });
        }

        // 计算整体吞吐量
        let (messages_per_second, window_seconds) = if all_timestamps.is_empty() {
            (0.0, 0i64)
        } else {
            let min_time = *all_timestamps.iter().min().expect("non-empty timestamps");
            let max_time = *all_timestamps.iter().max().expect("non-empty timestamps");
            let window = ((max_time - min_time) / 1000).max(1);
            let rate = if window > 0 {
                total_messages as f64 / window as f64
            } else {
                0.0
            };
            (rate, window)
        };

        Ok(TopicThroughputResponse {
            topic: topic.to_string(),
            produce_throughput: ThroughputStats {
                messages_per_second,
                bytes_per_second: None, // 需要额外的消息大小信息
                window_seconds,
            },
            total_messages,
            partitions: partition_stats,
        })
    }

    /// 获取分区的时间戳（最早和最新消息）
    fn get_partition_timestamps(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: i32,
        earliest_offset: i64,
        latest_offset: i64,
    ) -> (Option<i64>, Option<i64>) {
        let first_time = self.fetch_message_timestamp(kafka_config, topic, partition, earliest_offset);
        let last_time = if latest_offset > 0 {
            self.fetch_message_timestamp(kafka_config, topic, partition, latest_offset - 1)
        } else {
            None
        };
        (first_time, last_time)
    }

    /// 获取指定 offset 消息的时间戳
    fn fetch_message_timestamp(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> Option<i64> {
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, Offset::Offset(offset)).ok()?;

        let temp_consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_config.brokers)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("broker.address.family", "v4")
            .create()
            .ok()?;

        temp_consumer.assign(&tpl).ok()?;

        match temp_consumer.poll(std::time::Duration::from_millis(self.timeout_ms)) {
            Some(Ok(msg)) => msg.timestamp().to_millis(),
            _ => None,
        }
    }

    fn create_consumer(&self, kafka_config: &KafkaConfig) -> Result<BaseConsumer> {
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("enable.auto.commit", "false");
        // 强制使用 IPv4，避免 IPv6 连接问题
        client_config.set("broker.address.family", "v4");

        let consumer: BaseConsumer = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        Ok(consumer)
    }
}

/// 简单的内存缓存，用于存储之前的 offset 以计算实时速率
pub struct OffsetCache {
    cache: HashMap<String, OffsetCacheEntry>,
}

struct OffsetCacheEntry {
    offset: i64,
    timestamp: u64,
}

impl OffsetCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn record(&mut self, key: String, offset: i64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("SystemTime before UNIX epoch")
            .as_secs();
        self.cache.insert(key, OffsetCacheEntry { offset, timestamp: now });
    }

    /// 计算速率（条/秒）
    pub fn calculate_rate(&self, key: &str, current_offset: i64) -> Option<f64> {
        if let Some(entry) = self.cache.get(key) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("SystemTime before UNIX epoch")
                .as_secs();
            let elapsed = (now - entry.timestamp) as f64;
            if elapsed > 0.0 {
                let offset_delta = (current_offset - entry.offset) as f64;
                return Some(offset_delta / elapsed);
            }
        }
        None
    }
}

impl Default for OffsetCache {
    fn default() -> Self {
        Self::new()
    }
}
