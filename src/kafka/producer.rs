use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use crate::kafka::KafkaClientContext;
use rdkafka::producer::{FutureProducer, FutureRecord};

pub struct KafkaProducer {
    producer: FutureProducer<KafkaClientContext>,
}

impl KafkaProducer {
    pub fn new(kafka_config: &KafkaConfig) -> Result<Self> {
        let mut client_config = super::create_client_config(kafka_config);
        // Producer 需要 request.timeout.ms
        client_config.set("request.timeout.ms", &kafka_config.request_timeout_ms.to_string());
        let producer: FutureProducer<KafkaClientContext> = client_config.create_with_context(KafkaClientContext)?;

        Ok(Self { producer })
    }

    /// 发送消息（不指定分区，由 Kafka 决定）
    pub async fn send(&self, topic: &str, key: Option<&str>, value: &str, headers: Option<&std::collections::HashMap<String, String>>) -> Result<(i32, i64)> {
        self.send_to_partition(topic, None, key, value, headers).await
    }

    /// 发送到指定分区
    pub async fn send_to_partition(
        &self,
        topic: &str,
        partition: Option<i32>,
        key: Option<&str>,
        value: &str,
        headers: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<(i32, i64)> {
        let mut record = FutureRecord::to(topic).payload(value);

        if let Some(k) = key {
            record = record.key(k);
        }

        if let Some(p) = partition {
            record = record.partition(p);
        }

        // 添加 headers
        if let Some(hdrs) = headers {
            let mut owned_headers = rdkafka::message::OwnedHeaders::new();
            for (k, v) in hdrs {
                owned_headers = owned_headers.insert(rdkafka::message::Header {
                    key: k,
                    value: Some(v.as_bytes()),
                });
            }
            record = record.headers(owned_headers);
        }

        // 使用 send 方法等待结果
        match self.producer.send(record, None).await {
            Ok(delivery) => Ok((delivery.partition, delivery.offset)),
            Err((e, _)) => Err(AppError::Kafka(e)),
        }
    }
}
