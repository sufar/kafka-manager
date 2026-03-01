use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::{ClientConfig, Message, Offset};

/// Kafka Offset 管理
pub struct KafkaOffsetManager {
    timeout_ms: u64,
}

impl KafkaOffsetManager {
    pub fn new(kafka_config: &KafkaConfig) -> Self {
        Self {
            timeout_ms: kafka_config.operation_timeout_ms as u64,
        }
    }

    /// 获取 Topic 所有分区的 offset 信息（包括时间戳）
    pub fn get_topic_partition_offsets(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
    ) -> Result<Vec<PartitionOffset>> {
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

        let mut partition_offsets = Vec::new();

        for partition in topic_metadata.partitions() {
            let partition_id = partition.id();

            // 获取 earliest offset (low watermark) 和 latest offset (high watermark)
            let (low_offset, high_offset) = client
                .fetch_watermarks(topic, partition_id, std::time::Duration::from_millis(self.timeout_ms))
                .map_err(|e| AppError::Internal(format!("Failed to fetch watermarks: {}", e)))?;

            // 获取最早和最新消息的时间戳
            let (first_commit_time, last_commit_time) = if high_offset > low_offset {
                self.get_partition_timestamps(kafka_config, topic, partition_id, low_offset, high_offset)
            } else {
                (None, None)
            };

            partition_offsets.push(PartitionOffset {
                topic: topic.to_string(),
                partition: partition_id,
                earliest_offset: low_offset,
                latest_offset: high_offset,
                leader: partition.leader(),
                replicas: partition.replicas().to_vec(),
                isr: partition.isr().to_vec(),
                first_commit_time,
                last_commit_time,
            });
        }

        Ok(partition_offsets)
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
        // 获取最早消息的时间戳
        let first_time = self.fetch_message_timestamp(kafka_config, topic, partition, earliest_offset);

        // 获取最新消息的时间戳
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
        use rdkafka::consumer::DefaultConsumerContext;

        // 创建临时 consumer
        let mut temp_tpl = TopicPartitionList::new();
        temp_tpl.add_partition_offset(topic, partition, Offset::Offset(offset)).ok()?;

        let temp_consumer: BaseConsumer<DefaultConsumerContext> =
            ClientConfig::new()
                .set("bootstrap.servers", &kafka_config.brokers)
                .set("enable.auto.commit", "false")
                .set("auto.offset.reset", "earliest")
                .create()
                .ok()?;

        temp_consumer.assign(&temp_tpl).ok()?;

        // 尝试获取消息
        match temp_consumer.poll(std::time::Duration::from_millis(self.timeout_ms)) {
            Some(Ok(msg)) => msg.timestamp().to_millis(),
            _ => None,
        }
    }

    /// 获取 Consumer Group 在指定 Topic 上的 offset 和 lag
    pub fn get_consumer_group_offsets(
        &self,
        kafka_config: &KafkaConfig,
        group_id: &str,
        topic: Option<&str>,
    ) -> Result<Vec<ConsumerGroupOffsetDetail>> {
        // 使用低层级 consumer 来获取 offset
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", group_id);

        let consumer: BaseConsumer = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        let mut details = Vec::new();

        // 首先获取 topic 列表
        let topics_to_check: Vec<String> = if let Some(t) = topic {
            vec![t.to_string()]
        } else {
            // 由于 API 限制，需要指定 topic 才能获取 offset
            tracing::warn!("Cannot list all topics for consumer group without explicit topic");
            return Ok(vec![]);
        };

        for topic_name in topics_to_check {
            let metadata = consumer
                .client()
                .fetch_metadata(Some(&topic_name), std::time::Duration::from_millis(self.timeout_ms))
                .ok();

            if let Some(meta) = metadata {
                if let Some(topic_meta) = meta.topics().iter().find(|t| t.name() == topic_name) {
                    for partition in topic_meta.partitions() {
                        let partition_id = partition.id();

                        // 构建 topic partition
                        let mut tpl = TopicPartitionList::new();
                        tpl.add_partition(&topic_name, partition_id);

                        // 获取 committed offset
                        let committed_offsets = consumer
                            .committed_offsets(
                                tpl.clone(),
                                std::time::Duration::from_millis(self.timeout_ms),
                            )
                            .ok();

                        // 获取 latest offset (watermark)
                        let (_low, high) = consumer
                            .client()
                            .fetch_watermarks(
                                &topic_name,
                                partition_id,
                                std::time::Duration::from_millis(self.timeout_ms),
                            )
                            .unwrap_or((0, 0));

                        let current_offset = committed_offsets
                            .as_ref()
                            .and_then(|offsets| {
                                offsets.elements().iter().find(|e| {
                                    e.topic() == topic_name && e.partition() == partition_id
                                }).and_then(|e| e.offset().to_raw())
                            })
                            .unwrap_or(-1);

                        let lag = if current_offset >= 0 && high > 0 {
                            high - current_offset
                        } else {
                            0
                        };

                        details.push(ConsumerGroupOffsetDetail {
                            topic: topic_name.clone(),
                            partition: partition_id,
                            current_offset: if current_offset >= 0 { current_offset } else { 0 },
                            log_end_offset: high,
                            lag,
                        });
                    }
                }
            }
        }

        Ok(details)
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

/// Partition Offset 信息
#[derive(Debug, Clone)]
pub struct PartitionOffset {
    pub topic: String,
    pub partition: i32,
    pub earliest_offset: i64,
    pub latest_offset: i64,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
    pub first_commit_time: Option<i64>,  // 最早消息的时间戳
    pub last_commit_time: Option<i64>,   // 最新消息的时间戳
}

/// Consumer Group Offset 详情
#[derive(Debug, Clone)]
pub struct ConsumerGroupOffsetDetail {
    pub topic: String,
    pub partition: i32,
    pub current_offset: i64,
    pub log_end_offset: i64,
    pub lag: i64,
}
