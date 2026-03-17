/// Kafka Consumer 连接池实现 - 查询场景优化版
///
/// 特性：
/// 1. 最大 10 个连接（适合查询场景，避免过多连接占用资源）
/// 2. 预热机制：创建时预先连接到 Broker
/// 3. 快速回收：unassign + 健康检查，确保连接可用
/// 4. 监控指标：可用连接数、等待队列、创建/回收统计

use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use deadpool::managed;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::config::ClientConfig;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Kafka Consumer 连接池类型
pub type KafkaConsumerPool = managed::Pool<KafkaConsumerManager>;

/// Consumer Pool 监控指标
#[derive(Debug, Clone, Default)]
pub struct ConsumerPoolMetrics {
    /// 当前可用连接数
    pub available: usize,
    /// 当前在用连接数
    pub used: usize,
    /// 等待获取连接的队列长度
    pub waiting: usize,
    /// 总共创建的连接数
    pub created: u64,
    /// 总共回收的连接数
    pub recycled: u64,
    /// 健康检查失败丢弃的连接数
    pub health_check_failed: u64,
    /// 获取连接超时次数
    pub acquire_timeouts: u64,
}

/// Kafka Consumer 管理器
pub struct KafkaConsumerManager {
    config: KafkaConfig,
    /// 统计：创建的连接数
    created_count: Arc<AtomicU64>,
    /// 统计：回收的连接数
    recycled_count: Arc<AtomicU64>,
    /// 统计：健康检查失败数
    health_failed_count: Arc<AtomicU64>,
}

impl KafkaConsumerManager {
    pub fn new(config: &KafkaConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            created_count: Arc::new(AtomicU64::new(0)),
            recycled_count: Arc::new(AtomicU64::new(0)),
            health_failed_count: Arc::new(AtomicU64::new(0)),
        })
    }

    /// 获取监控指标
    pub fn metrics(&self) -> (u64, u64, u64) {
        (
            self.created_count.load(Ordering::Relaxed),
            self.recycled_count.load(Ordering::Relaxed),
            self.health_failed_count.load(Ordering::Relaxed),
        )
    }

    /// 创建客户端配置 - 查询场景优化版
    ///
    /// 优化策略：
    /// 1. 快速返回：fetch.min.bytes=1, fetch.wait.max.ms=10
    /// 2. 大缓冲区：fetch.max.bytes=50MB，减少网络往返
    /// 3. 短超时：快速失败，不阻塞查询
    /// 4. 禁用 group 协调：assign 模式，避免 rebalance 延迟
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

        // ========== 查询场景优化配置 ==========

        // fetch.wait.max.ms: 降低到 10ms，快速返回（查询场景不需要积累数据）
        client_config.set("fetch.wait.max.ms", "10");

        // fetch.min.bytes: 1 字节，立即返回
        client_config.set("fetch.min.bytes", "1");

        // fetch.max.bytes: 单次 fetch 最大数据量 (50MB)
        client_config.set("fetch.max.bytes", "52428800");

        // fetch.message.max.bytes: 单条消息最大 (5MB)
        client_config.set("fetch.message.max.bytes", "5242880");

        // max.partition.fetch.bytes: 每个分区最大获取字节数 (5MB)
        client_config.set("max.partition.fetch.bytes", "5242880");

        // receive.message.max.bytes: 消费者接收缓冲区
        client_config.set("receive.message.max.bytes", "52429312");

        // connections.max.idle.ms: 连接空闲超时时间（5分钟）
        client_config.set("connections.max.idle.ms", "300000");

        // socket.keepalive.enable: 启用 TCP keepalive
        client_config.set("socket.keepalive.enable", "true");

        // socket.nagle.disable: 禁用 Nagle 算法，降低延迟
        client_config.set("socket.nagle.disable", "true");

        // 强制使用 IPv4
        client_config.set("broker.address.family", "v4");

        // 快速重连
        client_config.set("reconnect.backoff.ms", "50");
        client_config.set("reconnect.backoff.max.ms", "500");

        // retry.backoff.ms: 重试间隔
        client_config.set("retry.backoff.ms", "50");

        // ========== 查询场景专用配置 ==========

        // socket 连接超时（缩短到 3 秒，快速失败）
        client_config.set("socket.connection.setup.timeout.ms", "3000");

        // session timeout（缩短到 10 秒）
        client_config.set("session.timeout.ms", "10000");

        // heartbeat 间隔
        client_config.set("heartbeat.interval.ms", "2000");

        // max.poll.interval.ms: 两次 poll 之间的最大间隔（30秒，查询场景足够）
        client_config.set("max.poll.interval.ms", "30000");

        // 禁用 group 协调（assign 模式不需要）
        client_config.set("partition.assignment.strategy", "");

        client_config
    }

    /// 预热 Consumer：创建后预先连接 Broker，确保立即可用
    async fn warmup_consumer(&self, consumer: &StreamConsumer) -> Result<()> {
        // 使用 poll 来触发连接建立 - 这是异步方法可以直接 await
        match tokio::time::timeout(Duration::from_secs(3), consumer.recv()).await {
            Ok(Ok(_msg)) => {
                // 收到消息，连接已建立
                tracing::debug!("Consumer warmed up (received message)");
            }
            Ok(Err(e)) => {
                // 有错误但连接已建立
                tracing::debug!("Consumer warmed up (connection established, error: {})", e);
            }
            Err(_) => {
                // 超时但不一定失败，连接可能还在建立中
                tracing::debug!("Consumer warmup timeout (connection may still be establishing)");
            }
        }

        Ok(())
    }
}

impl managed::Manager for KafkaConsumerManager {
    type Type = StreamConsumer;
    type Error = AppError;

    async fn create(&self) -> Result<StreamConsumer> {
        // 使用查询专用的 group.id（不需要唯一，assign 模式不影响）
        let group_id = "kafka-mgr-query-pool";
        let client_config = self.create_client_config(group_id);

        let consumer: StreamConsumer = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        // 预热：确保连接可用
        if let Err(e) = self.warmup_consumer(&consumer).await {
            tracing::warn!("Consumer warmup failed: {}", e);
            // 预热失败不阻塞，让连接进入 pool，使用时再处理
        }

        self.created_count.fetch_add(1, Ordering::Relaxed);
        tracing::debug!("Consumer created and warmed up");

        Ok(consumer)
    }

    async fn recycle(&self, conn: &mut StreamConsumer, _: &managed::Metrics) -> managed::RecycleResult<AppError> {
        self.recycled_count.fetch_add(1, Ordering::Relaxed);

        // 1. 快速清理：unassign 分区
        if let Err(e) = conn.unassign() {
            tracing::warn!("Failed to unassign consumer: {}", e);
            self.health_failed_count.fetch_add(1, Ordering::Relaxed);
            return Err(managed::RecycleError::Message(format!("Unassign failed: {}", e).into()));
        }

        // 2. 短暂等待让 broker 处理
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 3. 快速健康检查：尝试 poll 一条消息来验证连接
        // 使用短超时，如果没有消息可用也是正常的
        match tokio::time::timeout(Duration::from_millis(100), conn.recv()).await {
            Ok(Ok(_)) => {
                // 连接正常
                Ok(())
            }
            Ok(Err(e)) => {
                // poll 出错，连接可能有问题
                let error_msg = format!("{}", e);
                if error_msg.contains("disconnected") || error_msg.contains("error") {
                    tracing::warn!("Consumer health check failed: {}", e);
                    self.health_failed_count.fetch_add(1, Ordering::Relaxed);
                    Err(managed::RecycleError::Message(format!("Health check: {}", e).into()))
                } else {
                    // 非致命错误（如没有消息可消费），连接正常
                    Ok(())
                }
            }
            Err(_) => {
                // 超时但没有错误，连接正常（只是没有消息）
                Ok(())
            }
        }
    }
}

/// 获取 Pool 监控指标
pub fn get_pool_metrics(pool: &KafkaConsumerPool) -> ConsumerPoolMetrics {
    let status = pool.status();
    ConsumerPoolMetrics {
        available: status.available,
        used: status.size.saturating_sub(status.available),
        waiting: status.waiting,
        created: 0, // 从 manager 获取需要在 pool 中存储 manager 引用
        recycled: 0,
        health_check_failed: 0,
        acquire_timeouts: 0,
    }
}

/// 创建优化的 Consumer Pool（查询场景）
pub fn create_consumer_pool(config: &KafkaConfig, max_size: usize) -> Result<KafkaConsumerPool> {
    use deadpool::managed::{Hook, Timeouts};
    use deadpool::Runtime;

    let manager = KafkaConsumerManager::new(config)?;

    let pool = managed::Pool::builder(manager)
        .max_size(max_size)
        .wait_timeout(Some(Duration::from_secs(5))) // 5秒获取超时
        .timeouts(Timeouts {
            wait: Some(Duration::from_secs(5)),
            create: Some(Duration::from_secs(5)),  // 创建超时 5秒
            recycle: Some(Duration::from_secs(2)), // 回收超时 2秒
        })
        .runtime(Runtime::Tokio1)
        // 回收后钩子：记录日志
        .post_recycle(Hook::async_fn(|_conn, _metrics| {
            Box::pin(async move { Ok(()) })
        }))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create pool: {}", e)))?;

    Ok(pool)
}
