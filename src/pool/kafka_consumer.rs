/// Kafka Consumer 连接池实现

use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use deadpool::managed;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::config::ClientConfig;

/// Kafka Consumer 连接池类型
pub type KafkaConsumerPool = managed::Pool<KafkaConsumerManager>;

/// Kafka Consumer 管理器
pub struct KafkaConsumerManager {
    config: KafkaConfig,
}

impl KafkaConsumerManager {
    pub fn new(config: &KafkaConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// 创建客户端配置 - 平衡响应速度和数据获取
    fn create_client_config(&self, group_id: &str) -> ClientConfig {
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &self.config.brokers);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");
        client_config.set(
            "request.timeout.ms",
            &self.config.request_timeout_ms.to_string(),
        );
        client_config.set(
            "socket.timeout.ms",
            &self.config.request_timeout_ms.to_string(),
        );
        // 优化：平衡配置，确保能读取大数据量 topic
        // fetch.wait.max.ms: broker 等待数据的最大时间
        // 设置为 50ms，给 broker 足够时间准备数据，同时保持较快响应
        client_config.set("fetch.wait.max.ms", "50");
        // fetch.min.bytes: 最小返回数据量
        // 设置为 1，有数据就返回，不强制等待批量
        client_config.set("fetch.min.bytes", "1");
        // fetch.max.bytes: 单次获取最大数据量 (50MB)
        client_config.set("fetch.max.bytes", "52428800");
        // fetch.message.max.bytes: 单条消息最大 (10MB)
        client_config.set("fetch.message.max.bytes", "10485760");
        // 增加 socket 连接超时，确保连接稳定
        client_config.set("socket.connection.setup.timeout.ms", "10000");
        // 增加 session timeout，避免被踢出
        client_config.set("session.timeout.ms", "30000");
        // heartbeat 间隔
        client_config.set("heartbeat.interval.ms", "3000");
        // max.poll.interval.ms: 两次 poll 之间的最大间隔
        client_config.set("max.poll.interval.ms", "300000");
        client_config
    }
}

impl managed::Manager for KafkaConsumerManager {
    type Type = StreamConsumer;
    type Error = AppError;

    async fn create(&self) -> Result<StreamConsumer> {
        let client_config = self.create_client_config("kafka-manager-pool");
        let consumer: StreamConsumer = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;
        Ok(consumer)
    }

    async fn recycle(&self, conn: &mut StreamConsumer, _: &managed::Metrics) -> managed::RecycleResult<AppError> {
        // 健康检查：尝试获取消费者元数据来判断连接是否有效
        use std::time::Duration;

        // 使用 client().metadata() 进行健康检查
        let client = conn.client();
        match client.fetch_metadata(None, Duration::from_secs(2)) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::warn!("Consumer health check failed: {}", e);
                Err(managed::RecycleError::Message(format!("Health check failed: {}", e).into()))
            }
        }
    }
}
