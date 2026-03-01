use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use rdkafka::producer::{FutureProducer, FutureRecord};

pub struct KafkaProducer {
    producer: FutureProducer,
}

impl KafkaProducer {
    pub fn new(kafka_config: &KafkaConfig) -> Result<Self> {
        let client_config = super::create_client_config(kafka_config);
        let producer: FutureProducer = client_config.create()?;

        Ok(Self { producer })
    }

    /// 发送消息（不指定分区，由 Kafka 决定）
    pub async fn send(&self, topic: &str, key: Option<&str>, value: &str) -> Result<(i32, i64)> {
        self.send_to_partition(topic, None, key, value).await
    }

    /// 发送到指定分区
    pub async fn send_to_partition(
        &self,
        topic: &str,
        partition: Option<i32>,
        key: Option<&str>,
        value: &str,
    ) -> Result<(i32, i64)> {
        let mut record = FutureRecord::to(topic).payload(value);

        if let Some(k) = key {
            record = record.key(k);
        }

        if let Some(p) = partition {
            record = record.partition(p);
        }

        // 使用 send 方法等待结果
        match self.producer.send(record, None).await {
            Ok(delivery) => Ok((delivery.partition, delivery.offset)),
            Err((e, _)) => Err(AppError::Kafka(e)),
        }
    }
}
