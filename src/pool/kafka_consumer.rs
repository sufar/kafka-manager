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

    /// 创建客户端配置 - 高性能查询优化版
    ///
    /// 优化策略：
    /// 1. fetch.min.bytes=1: 有数据立即返回，不等待批量
    /// 2. fetch.wait.max.ms=10: 最多等待10ms，降低延迟
    /// 3. fetch.max.bytes=10MB: 单次获取最大数据量，降低内存占用
    /// 4. socket.keepalive.enable=true: 保持连接活跃
    /// 5. socket.nagle.disable=true: 禁用Nagle算法，降低延迟
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

        // ========== 高性能查询优化配置 ==========

        // fetch.wait.max.ms: broker 等待数据的最大时间
        // 设置为 10ms，快速返回，不等待更多数据
        client_config.set("fetch.wait.max.ms", "10");

        // fetch.min.bytes: 最小返回数据量
        // 设置为 1，有数据就立即返回，不强制等待批量
        client_config.set("fetch.min.bytes", "1");

        // fetch.max.bytes: 单次 fetch 最大数据量 (10MB)
        // 降低内存占用，提高响应速度
        client_config.set("fetch.max.bytes", "10485760");

        // fetch.message.max.bytes: 单条消息最大 (1MB)
        client_config.set("fetch.message.max.bytes", "1048576");

        // max.partition.fetch.bytes: 每个分区最大获取字节数 (1MB)
        client_config.set("max.partition.fetch.bytes", "1048576");

        // connections.max.idle.ms: 连接空闲超时时间
        // 设置为 9 分钟，避免连接被 broker 关闭
        client_config.set("connections.max.idle.ms", "540000");

        // socket.keepalive.enable: 启用 TCP keepalive
        client_config.set("socket.keepalive.enable", "true");

        // socket.nagle.disable: 禁用 Nagle 算法，降低延迟（对远程连接很重要）
        client_config.set("socket.nagle.disable", "true");

        // reconnect.backoff.ms: 重连间隔
        client_config.set("reconnect.backoff.ms", "50");
        client_config.set("reconnect.backoff.max.ms", "1000");

        // retry.backoff.ms: 重试间隔
        client_config.set("retry.backoff.ms", "100");

        // ========== 稳定性配置 ==========

        // socket 连接超时
        client_config.set("socket.connection.setup.timeout.ms", "10000");

        // session timeout
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
        use std::time::Duration;

        // 1. 清理消费者的分区分配，避免影响下次使用
        if let Err(e) = conn.unassign() {
            tracing::warn!("Failed to unassign consumer partitions: {}", e);
            // 如果无法清理，放弃这个连接
            return Err(managed::RecycleError::Message(format!("Failed to unassign: {}", e).into()));
        }

        // 2. 健康检查：尝试获取消费者元数据来判断连接是否有效
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
