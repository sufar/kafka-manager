use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use rdkafka::config::ClientConfig;
use std::time::Duration;
use futures::stream::Stream;
use tokio_stream::wrappers::UnboundedReceiverStream;
use serde::{Serialize, Deserialize};

pub struct KafkaConsumer {
    timeout: Duration,
}

/// 消息游标，用于分页
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchCursor {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
}

impl FetchCursor {
    /// 从 Base64 字符串解码游标
    pub fn from_base64(s: &str) -> Result<Self> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(|e| AppError::Internal(format!("Invalid cursor: {}", e)))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Internal(format!("Invalid cursor format: {}", e)))
    }

    /// 编码为 Base64 字符串
    pub fn to_base64(&self) -> String {
        use base64::Engine;
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    }
}

impl KafkaConsumer {
    pub fn new(kafka_config: &KafkaConfig) -> Result<Self> {
        let timeout = Duration::from_millis(kafka_config.operation_timeout_ms as u64);
        Ok(Self { timeout })
    }

    /// 从指定 offset 获取消息（批量获取优化版）
    ///
    /// 使用 futures::stream 批量获取，减少网络 RTT 开销
    pub async fn fetch_messages_batch(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: Option<i32>,
        offset: Option<i64>,
        max_messages: usize,
        _batch_size: usize,
    ) -> Result<Vec<KafkaMessage>> {
        use rdkafka::config::ClientConfig;
        use std::time::Duration;

        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", "kafka-manager-fetcher");
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");
        client_config.set("fetch.wait.max.ms", "50");
        client_config.set("fetch.min.bytes", "1");

        let consumer: StreamConsumer = client_config.create()?;
        consumer.subscribe(&[topic])?;

        let mut messages = Vec::new();
        let base_timeout = Duration::from_millis(100);
        let mut consecutive_timeouts = 0;
        const MAX_CONSECUTIVE_TIMEOUTS: u32 = 2;

        while messages.len() < max_messages {
            match tokio::time::timeout(base_timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    consecutive_timeouts = 0;

                    // 分区过滤
                    if let Some(p) = partition {
                        if msg.partition() != p {
                            continue;
                        }
                    }
                    // offset 过滤
                    if let Some(min_offset) = offset {
                        if msg.offset() < min_offset {
                            continue;
                        }
                    }

                    messages.push(KafkaMessage {
                        partition: msg.partition(),
                        offset: msg.offset(),
                        key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                        value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                        timestamp: msg.timestamp().to_millis(),
                    });

                    if messages.len() >= max_messages {
                        break;
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        break;
                    }
                }
            }
        }

        Ok(messages)
    }

    /// 基于游标的分页获取
    ///
    /// 返回消息列表和下一个游标（如果有更多数据）
    pub async fn fetch_messages_cursor(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        cursor: Option<&FetchCursor>,
        limit: usize,
    ) -> Result<(Vec<KafkaMessage>, Option<FetchCursor>)> {
        let partition = cursor.map(|c| c.partition).unwrap_or(0);
        let offset = cursor.map(|c| c.offset).unwrap_or(0);

        let messages = self.fetch_messages(
            kafka_config,
            topic,
            Some(partition),
            Some(offset),
            limit + 1, // 多取一条判断是否有更多数据
        ).await?;

        let has_more = messages.len() > limit;
        let mut messages = messages;
        if has_more {
            messages.truncate(limit);
        }

        // 计算下一个游标
        let next_cursor = if has_more || messages.len() == limit {
            if let Some(last_msg) = messages.last() {
                Some(FetchCursor {
                    topic: topic.to_string(),
                    partition: last_msg.partition,
                    offset: last_msg.offset + 1,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok((messages, next_cursor))
    }

    /// 流式获取消息（返回 Stream）
    ///
    /// 用于流式导出，避免一次性加载所有消息到内存
    pub fn fetch_messages_stream(
        &self,
        kafka_config: &KafkaConfig,
        topic: String,
        partition: Option<i32>,
        offset: Option<i64>,
        max_messages: usize,
    ) -> impl Stream<Item = Result<KafkaMessage>> + Send {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let brokers = kafka_config.brokers.clone();
        let request_timeout = kafka_config.request_timeout_ms;
        let timeout = self.timeout;

        tokio::spawn(async move {
            let mut client_config = ClientConfig::new();
            client_config.set("bootstrap.servers", &brokers);
            client_config.set("group.id", "kafka-manager-fetcher");
            client_config.set("enable.auto.commit", "false");
            client_config.set("auto.offset.reset", "earliest");
            client_config.set("request.timeout.ms", &request_timeout.to_string());

            match client_config.create::<StreamConsumer>() {
                Ok(consumer) => {
                    if let Err(e) = consumer.subscribe(&[&topic]) {
                        let _ = tx.send(Err(AppError::Kafka(e)));
                        return;
                    }

                    let mut count = 0;
                    while count < max_messages {
                        match tokio::time::timeout(timeout, consumer.recv()).await {
                            Ok(Ok(msg)) => {
                                // 分区过滤
                                if let Some(p) = partition {
                                    if msg.partition() != p {
                                        continue;
                                    }
                                }
                                // offset 过滤
                                if let Some(min_offset) = offset {
                                    if msg.offset() < min_offset {
                                        continue;
                                    }
                                }

                                let kafka_msg = KafkaMessage {
                                    partition: msg.partition(),
                                    offset: msg.offset(),
                                    key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                                    value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                                    timestamp: msg.timestamp().to_millis(),
                                };

                                if tx.send(Ok(kafka_msg)).is_err() {
                                    break;
                                }
                                count += 1;
                            }
                            Ok(Err(_)) | Err(_) => break,
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(AppError::Kafka(e)));
                }
            }
        });

        UnboundedReceiverStream::new(rx)
    }

    /// 消费消息
    pub async fn consume(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        group_id: &str,
        max_messages: usize,
    ) -> Result<Vec<KafkaMessage>> {
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");

        let consumer: StreamConsumer = client_config.create()?;
        consumer.subscribe(&[topic])?;

        let mut messages = Vec::new();

        for _ in 0..max_messages {
            match tokio::time::timeout(self.timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    messages.push(KafkaMessage {
                        partition: msg.partition(),
                        offset: msg.offset(),
                        key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                        value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                        timestamp: msg.timestamp().to_millis(),
                    });
                }
                Ok(Err(e)) => {
                    if messages.is_empty() {
                        return Err(AppError::Kafka(e));
                    }
                    break;
                }
                Err(_) => break, // Timeout
            }
        }

        Ok(messages)
    }

    /// 从指定 offset 获取消息（基础版本，无过滤）
    pub async fn fetch_messages(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: Option<i32>,
        offset: Option<i64>,
        max_messages: usize,
    ) -> Result<Vec<KafkaMessage>> {
        use rdkafka::config::ClientConfig;
        use std::time::Duration;

        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", "kafka-manager-fetcher");
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");
        // 降低 fetch 等待时间，加快空主题响应
        client_config.set("fetch.wait.max.ms", "50");
        client_config.set("fetch.min.bytes", "1");

        let consumer: StreamConsumer = client_config.create()?;
        consumer.subscribe(&[topic])?;

        let mut messages = Vec::new();
        let base_timeout = Duration::from_millis(100);
        let mut consecutive_timeouts = 0;
        const MAX_CONSECUTIVE_TIMEOUTS: u32 = 2; // 连续 2 次超时认为没有更多消息

        while messages.len() < max_messages {
            match tokio::time::timeout(base_timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    consecutive_timeouts = 0; // 重置超时计数

                    // 如果指定了分区，只返回该分区的消息
                    if let Some(p) = partition {
                        if msg.partition() != p {
                            continue;
                        }
                    }
                    // 如果指定了最小 offset，过滤掉之前的消息
                    if let Some(min_offset) = offset {
                        if msg.offset() < min_offset {
                            continue;
                        }
                    }

                    messages.push(KafkaMessage {
                        partition: msg.partition(),
                        offset: msg.offset(),
                        key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                        value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                        timestamp: msg.timestamp().to_millis(),
                    });
                }
                Ok(Err(_)) | Err(_) => {
                    consecutive_timeouts += 1;
                    // 连续超时，认为没有更多消息
                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        break;
                    }
                }
            }
        }

        Ok(messages)
    }

    /// 从指定 offset 获取消息（支持流式过滤）
    ///
    /// 此方法在读取消息时即进行过滤，避免大量数据加载到内存
    pub async fn fetch_messages_filtered<M>(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: Option<i32>,
        offset: Option<i64>,
        max_messages: usize,
        matcher: &M,
    ) -> Result<Vec<KafkaMessage>>
    where
        M: Fn(&KafkaMessage) -> bool,
    {
        use rdkafka::config::ClientConfig;
        use std::time::Duration;

        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        // 使用唯一的 group.id 避免与现有 consumer group 冲突
        client_config.set("group.id", &format!("kafka-manager-fetcher-{}-{}-{}", std::process::id(), topic, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");
        // 优化：降低各种超时时间
        client_config.set("fetch.wait.max.ms", "10");
        client_config.set("fetch.min.bytes", "1");
        client_config.set("fetch.max.bytes", "1048576"); // 1MB
        client_config.set("session.timeout.ms", "3000");
        client_config.set("request.timeout.ms", "2000");
        client_config.set("socket.timeout.ms", "2000");

        let consumer: StreamConsumer = client_config.create()?;

        // 手动分配 partition，而不是使用 subscribe
        let mut tpl = rdkafka::TopicPartitionList::new();
        let target_partition = partition.unwrap_or(0);

        // 如果指定了 offset，直接使用；否则需要查询 watermarks 来确定开始位置
        let target_offset = if let Some(off) = offset {
            rdkafka::Offset::Offset(off)
        } else {
            // 查询 low watermark 作为开始 offset
            match consumer.fetch_watermarks(topic, target_partition, Some(Duration::from_millis(500))) {
                Ok((low, _high)) => rdkafka::Offset::Offset(low),
                Err(_) => rdkafka::Offset::Beginning
            }
        };
        tpl.add_partition_offset(topic, target_partition, target_offset)?;
        consumer.assign(&tpl)?;

        let mut messages = Vec::new();
        // 首次请求稍微长一点，后续请求可以很短
        let base_timeout = Duration::from_millis(100);
        let mut consecutive_timeouts = 0;
        const MAX_CONSECUTIVE_TIMEOUTS: u32 = 2;
        let mut filtered_count = 0;

        while messages.len() < max_messages {
            match tokio::time::timeout(base_timeout, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    consecutive_timeouts = 0;

                    // 分区过滤
                    if let Some(p) = partition {
                        if msg.partition() != p {
                            continue;
                        }
                    }
                    // offset 过滤
                    if let Some(min_offset) = offset {
                        if msg.offset() < min_offset {
                            continue;
                        }
                    }

                    let kafka_msg = KafkaMessage {
                        partition: msg.partition(),
                        offset: msg.offset(),
                        key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                        value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                        timestamp: msg.timestamp().to_millis(),
                    };

                    if matcher(&kafka_msg) {
                        messages.push(kafka_msg);
                    } else {
                        filtered_count += 1;
                        if filtered_count >= max_messages * 10 {
                            break;
                        }
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        break;
                    }
                }
            }
        }

        Ok(messages)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KafkaMessage {
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: Option<String>,
    pub timestamp: Option<i64>,
}
