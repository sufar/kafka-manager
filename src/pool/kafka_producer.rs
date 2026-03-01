/// Kafka Producer 连接池实现

use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use deadpool::managed;
use rdkafka::producer::{FutureProducer, Producer};
use rdkafka::config::ClientConfig;

/// Kafka Producer 连接池类型
pub type KafkaProducerPool = managed::Pool<KafkaProducerManager>;

/// Kafka Producer 管理器
pub struct KafkaProducerManager {
    config: KafkaConfig,
}

impl KafkaProducerManager {
    pub fn new(config: &KafkaConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// 创建客户端配置
    fn create_client_config(&self) -> ClientConfig {
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &self.config.brokers);
        client_config.set(
            "request.timeout.ms",
            &self.config.request_timeout_ms.to_string(),
        );
        client_config.set(
            "socket.timeout.ms",
            &self.config.request_timeout_ms.to_string(),
        );
        // Producer 特定配置
        client_config.set("message.timeout.ms", "30000");
        client_config.set("delivery.timeout.ms", "300000");
        client_config
    }
}

impl managed::Manager for KafkaProducerManager {
    type Type = FutureProducer;
    type Error = AppError;

    async fn create(&self) -> Result<FutureProducer> {
        let client_config = self.create_client_config();
        let producer: FutureProducer = client_config.create()
            .map_err(|e| AppError::Internal(format!("Failed to create producer: {}", e)))?;
        Ok(producer)
    }

    async fn recycle(&self, producer: &mut FutureProducer, _: &managed::Metrics) -> managed::RecycleResult<AppError> {
        // 检查 Producer 是否有效
        if producer.in_flight_count() > 1000 {
            Err(managed::RecycleError::Message(
                "Too many in-flight messages".into()
            ))
        } else {
            Ok(())
        }
    }
}
