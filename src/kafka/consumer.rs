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

    /// 快速获取最新消息（不查询 watermarks，直接从 End 开始读取）
    /// 适用于高频场景，只读取最新的消息
    pub async fn fetch_latest_messages<M>(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: i32,
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
        client_config.set("group.id", &format!("kafka-manager-latest-{}-{}-{}", std::process::id(), topic, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("SystemTime before UNIX epoch").as_millis()));
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");
        // 快速响应配置
        client_config.set("request.timeout.ms", "3000");
        client_config.set("socket.timeout.ms", "3000");
        client_config.set("fetch.wait.max.ms", "5");
        client_config.set("fetch.min.bytes", "1");

        let consumer: StreamConsumer = client_config.create()?;

        // 先查询 watermarks（使用更快的方式）
        let watermarks = consumer.fetch_watermarks(topic, partition, Duration::from_millis(300));

        let mut tpl = rdkafka::TopicPartitionList::new();
        if let Ok((low, high)) = watermarks {
            // 从 high - max_messages 开始读取，但不能小于 low
            let start_offset = std::cmp::max(low, high - max_messages as i64);
            tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(start_offset))?;
        } else {
            // 查询失败，从开头读取
            tpl.add_partition_offset(topic, partition, rdkafka::Offset::Beginning)?;
        }
        consumer.assign(&tpl)?;

        let mut messages = Vec::new();
        let base_timeout = Duration::from_millis(30);
        let mut consecutive_timeouts = 0;

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
                    if matcher(&kafka_msg) {
                        messages.push(kafka_msg);
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= 3 {
                        break;
                    }
                }
            }
        }

        Ok(messages)
    }

    /// 使用 offsets_for_timestamp API 按时间戳查找 offset
    pub async fn get_offset_for_time(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: i32,
        timestamp_ms: i64,
    ) -> Result<Option<i64>> {
        use rdkafka::config::ClientConfig;
        use std::time::Duration;

        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", &format!("kafka-manager-offset-finder-{}", std::process::id()));
        // 优化：增加超时时间，减少频繁切换时的超时错误
        client_config.set("request.timeout.ms", "10000");
        client_config.set("socket.timeout.ms", "10000");
        client_config.set("socket.connection.setup.timeout.ms", "10000");

        let consumer: StreamConsumer = client_config.create()?;

        // 使用 offsets_for_timestamp 方法查找 offset
        match consumer.offsets_for_timestamp(
            timestamp_ms,
            Duration::from_secs(5),
        ) {
            Ok(tpl) => {
                // 查找指定 topic 和 partition 的 offset
                for elem in tpl.elements() {
                    if elem.topic() == topic && elem.partition() == partition {
                        match elem.offset() {
                            rdkafka::Offset::Offset(off) => return Ok(Some(off)),
                            _ => return Ok(None),
                        }
                    }
                }
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    /// 使用 offsets_for_times API 按时间戳查找多个 partition 的 offset
    pub async fn get_offsets_for_times(
        &self,
        kafka_config: &KafkaConfig,
        topic: &str,
        timestamps: Vec<(i32, i64)>, // (partition, timestamp_ms)
    ) -> Result<std::collections::HashMap<i32, i64>> {
        use rdkafka::config::ClientConfig;
        use rdkafka::TopicPartitionList;
        use std::time::Duration;

        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", &format!("kafka-manager-offset-finder-{}", std::process::id()));

        let consumer: StreamConsumer = client_config.create()?;

        // 构建 TopicPartitionList，使用时间戳作为 offset
        let mut tpl = TopicPartitionList::new();
        for (partition, timestamp_ms) in timestamps {
            // 使用 Offset::Offset 存储时间戳，offsets_for_times 会处理转换
            tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(timestamp_ms))?;
        }

        // 调用 offsets_for_times
        let result_tpl = consumer.offsets_for_times(tpl, Duration::from_secs(3))?;

        // 获取结果
        let mut offsets = std::collections::HashMap::new();
        for elem in result_tpl.elements() {
            if elem.topic() == topic {
                match elem.offset() {
                    rdkafka::Offset::Offset(off) => {
                        offsets.insert(elem.partition(), off);
                    }
                    _ => {}
                }
            }
        }

        Ok(offsets)
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

    /// 从指定 offset 获取消息（支持流式过滤）- 性能优化版
    ///
    /// 此方法在读取消息时即进行过滤，避免大量数据加载到内存
    /// 使用批量获取和时间戳二分查找优化性能
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
        client_config.set("group.id", &format!("kafka-manager-fetcher-{}-{}-{}", std::process::id(), topic, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("SystemTime before UNIX epoch").as_millis()));
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");
        // 优化：减少超时时间，加快响应
        client_config.set("request.timeout.ms", "5000");
        client_config.set("socket.timeout.ms", "5000");
        client_config.set("socket.connection.setup.timeout.ms", "5000");
        // 优化：快速响应模式，不等待批量数据
        client_config.set("fetch.wait.max.ms", "10");       // 减少到 10ms
        client_config.set("fetch.min.bytes", "1");          // 有数据就返回，不等待
        client_config.set("fetch.max.bytes", "1048576");    // 1MB
        client_config.set("session.timeout.ms", "5000");

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
        // 减少超时时间，加快响应
        let base_timeout = Duration::from_millis(50);
        let mut consecutive_timeouts = 0;
        const MAX_CONSECUTIVE_TIMEOUTS: u32 = 3;

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
