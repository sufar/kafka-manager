use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use crate::models::{
    ConsumerGroupPartitionThroughput, ConsumerGroupThroughputResponse,
    PartitionThroughput, ThroughputStats, TopicThroughputResponse,
};
use crate::kafka::offset::KafkaOffsetManager;
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
            let min_time = *all_timestamps.iter().min().unwrap();
            let max_time = *all_timestamps.iter().max().unwrap();
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

    /// 计算 Consumer Group 的消费吞吐量
    pub fn calculate_consumer_group_throughput(
        &self,
        kafka_config: &KafkaConfig,
        group_id: &str,
        topic: &str,
    ) -> Result<ConsumerGroupThroughputResponse> {
        // 获取 Topic 的 watermarks
        let topic_throughput = self.calculate_topic_throughput(kafka_config, topic)?;

        // 创建 consumer 获取 committed offsets
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", group_id);

        let consumer: BaseConsumer = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        let mut total_lag = 0i64;
        let mut partition_stats = Vec::new();
        let mut total_consume_rate = 0.0;

        for partition in &topic_throughput.partitions {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition(topic, partition.partition);

            // 获取 committed offset
            let committed_offsets = consumer
                .committed_offsets(tpl.clone(), std::time::Duration::from_millis(self.timeout_ms))
                .ok();

            let current_offset = committed_offsets
                .as_ref()
                .and_then(|offsets| {
                    offsets.elements().iter().find(|e| {
                        e.topic() == topic && e.partition() == partition.partition
                    }).and_then(|e| e.offset().to_raw())
                })
                .unwrap_or(-1);

            let lag = if current_offset >= 0 && partition.latest_offset > 0 {
                partition.latest_offset - current_offset
            } else {
                0
            };
            total_lag += lag.max(0);

            // 消费速率估算（基于 lag 和时间窗口）
            // 注意：这是近似值，因为没有历史 offset 数据
            let consume_rate = if lag > 0 && topic_throughput.produce_throughput.window_seconds > 0 {
                // 假设消费者在追赶，估算消费速率
                topic_throughput.produce_throughput.messages_per_second * 0.8
            } else {
                topic_throughput.produce_throughput.messages_per_second
            } as f64;

            total_consume_rate += consume_rate;

            partition_stats.push(ConsumerGroupPartitionThroughput {
                partition: partition.partition,
                current_offset: if current_offset >= 0 { current_offset } else { 0 },
                log_end_offset: partition.latest_offset,
                lag,
                consume_rate,
            });
        }

        // 预估追上时间
        let estimated_time_to_catch_up = if total_consume_rate > 0.0 && total_lag > 0 {
            Some(total_lag as f64 / total_consume_rate)
        } else {
            None
        };

        Ok(ConsumerGroupThroughputResponse {
            group_name: group_id.to_string(),
            topic: topic.to_string(),
            consume_throughput: ThroughputStats {
                messages_per_second: total_consume_rate,
                bytes_per_second: None,
                window_seconds: topic_throughput.produce_throughput.window_seconds,
            },
            total_lag,
            estimated_time_to_catch_up,
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
            .unwrap()
            .as_secs();
        self.cache.insert(key, OffsetCacheEntry { offset, timestamp: now });
    }

    /// 计算速率（条/秒）
    pub fn calculate_rate(&self, key: &str, current_offset: i64) -> Option<f64> {
        if let Some(entry) = self.cache.get(key) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
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

// ==================== 历史积压数据（用于折线图） ====================

use crate::models::{ConsumerGroupLagSummary, TopicConsumerLagResponse, ConsumerGroupPartitionDetail};

impl KafkaThroughputCalculator {
    /// 获取 Topic 所有 Consumer Group 的当前积压数据（用于实时折线图）
    ///
    /// 注意：由于 Kafka 不存储历史 offset 数据，此接口返回当前时刻的快照数据
    /// 前端可以定期调用此接口采集数据点，构建实时折线图
    pub fn get_topic_consumer_lag_snapshot(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        admin: &crate::kafka::KafkaAdmin,
    ) -> Result<TopicConsumerLagResponse> {
        // 获取所有 consumer groups
        let groups = admin.list_consumer_groups(kafka_config)?;

        let offset_manager = KafkaOffsetManager::new(kafka_config);
        let mut consumer_group_lags = Vec::new();
        let mut total_lag = 0i64;

        for group in &groups {
            // 获取该 group 在指定 topic 上的 offset
            let offsets = offset_manager.get_consumer_group_offsets(
                kafka_config,
                &group.name,
                Some(topic),
            )?;

            if offsets.is_empty() {
                // 该 group 没有消费过这个 topic
                continue;
            }

            let group_lag: i64 = offsets.iter().map(|o| o.lag).sum();
            total_lag += group_lag;

            let partitions: Vec<ConsumerGroupPartitionDetail> = offsets
                .into_iter()
                .map(|o| ConsumerGroupPartitionDetail {
                    partition: o.partition,
                    current_offset: o.current_offset,
                    log_end_offset: o.log_end_offset,
                    lag: o.lag,
                    state: if o.current_offset >= 0 {
                        "Active".to_string()
                    } else {
                        "Empty".to_string()
                    },
                    last_commit_time: None,
                    topic: o.topic.clone(),
                })
                .collect();

            consumer_group_lags.push(ConsumerGroupLagSummary {
                group_name: group.name.clone(),
                total_lag: group_lag,
                partitions: partitions.into_iter().map(|p| crate::models::PartitionLagDetail {
                    partition: p.partition,
                    current_offset: p.current_offset,
                    log_end_offset: p.log_end_offset,
                    lag: p.lag,
                    state: p.state,
                }).collect(),
            });
        }

        // 按 lag 降序排序
        consumer_group_lags.sort_by(|a, b| b.total_lag.cmp(&a.total_lag));

        Ok(TopicConsumerLagResponse {
            topic: topic.to_string(),
            total_lag,
            consumer_groups: consumer_group_lags,
        })
    }
}
