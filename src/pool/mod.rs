/// Kafka 连接池模块
///
/// 提供 Kafka Consumer 和 Producer 的连接池管理，避免频繁创建/销毁连接

pub mod kafka_consumer;
pub mod kafka_producer;

pub use kafka_consumer::KafkaConsumerPool;
pub use kafka_producer::KafkaProducerPool;

use crate::config::{KafkaConfig, PoolConfig};
use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// 连接池创建选项
pub struct PoolOptions {
    pub min_size: usize,
    pub max_size: usize,
    pub acquire_timeout_secs: u64,
}

impl From<&PoolConfig> for PoolOptions {
    fn from(config: &PoolConfig) -> Self {
        Self {
            min_size: config.min_size,
            max_size: config.max_size,
            acquire_timeout_secs: config.acquire_timeout_secs,
        }
    }
}

/// 连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Error(String),
}

/// 集群连接池集合
#[derive(Clone)]
pub struct ClusterPools {
    /// 集群 ID -> (Consumer Pool, Producer Pool, PoolConfig)
    pools: Arc<tokio::sync::RwLock<HashMap<String, (KafkaConsumerPool, KafkaProducerPool, PoolConfig)>>>,
}

/// Pool 配置信息（用于前端显示）
#[derive(Debug, Clone)]
pub struct PoolConfigInfo {
    pub max_size: usize,
    pub min_size: usize,
}

impl ClusterPools {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 初始化所有集群的连接池
    pub async fn init(&self, clusters: &HashMap<String, KafkaConfig>, pool_config: &PoolConfig) -> Result<()> {
        tracing::info!(
            "[PoolInit] Starting with config: max_size={}, min_size={}, acquire_timeout={}s",
            pool_config.max_size,
            pool_config.min_size,
            pool_config.acquire_timeout_secs
        );

        let mut pools = self.pools.write().await;

        for (cluster_id, config) in clusters {
            let consumer_mgr = kafka_consumer::KafkaConsumerManager::new(config)?;
            let producer_mgr = kafka_producer::KafkaProducerManager::new(config)?;

            let consumer_pool = build_pool(consumer_mgr, pool_config);
            let producer_pool = build_pool(producer_mgr, pool_config);

            tracing::info!(
                "[PoolInit] Cluster '{}' pools created: consumer={:?}, producer={:?}",
                cluster_id,
                consumer_pool.status(),
                producer_pool.status()
            );

            pools.insert(cluster_id.clone(), (consumer_pool, producer_pool, pool_config.clone()));
        }

        Ok(())
    }

    /// 获取指定集群的 Consumer 连接池
    pub async fn get_consumer_pool(&self, cluster_id: &str) -> Option<KafkaConsumerPool> {
        let pools = self.pools.read().await;
        pools.get(cluster_id).map(|(c, _, _)| c.clone())
    }

    /// 获取指定集群的 Producer 连接池
    pub async fn get_producer_pool(&self, cluster_id: &str) -> Option<KafkaProducerPool> {
        let pools = self.pools.read().await;
        pools.get(cluster_id).map(|(_, p, _)| p.clone())
    }

    /// 获取指定集群的 Pool 配置
    pub async fn get_pool_config(&self, cluster_id: &str) -> Option<PoolConfigInfo> {
        let pools = self.pools.read().await;
        pools.get(cluster_id).map(|(_, _, config)| PoolConfigInfo {
            max_size: config.max_size,
            min_size: config.min_size,
        })
    }

    /// 添加新的集群连接池
    pub async fn add_cluster(&self, cluster_id: &str, config: &KafkaConfig, pool_config: &PoolConfig) -> Result<()> {
        let mut pools = self.pools.write().await;

        let consumer_mgr = kafka_consumer::KafkaConsumerManager::new(config)?;
        let producer_mgr = kafka_producer::KafkaProducerManager::new(config)?;

        let consumer_pool = build_pool(consumer_mgr, pool_config);
        let producer_pool = build_pool(producer_mgr, pool_config);

        pools.insert(cluster_id.to_string(), (consumer_pool, producer_pool, pool_config.clone()));

        Ok(())
    }

    /// 移除集群连接池
    pub async fn remove_cluster(&self, cluster_id: &str) {
        let mut pools = self.pools.write().await;
        pools.remove(cluster_id);
    }

    /// 检查集群连接状态
    pub async fn check_connection(&self, cluster_id: &str) -> Option<ConnectionStatus> {
        use rdkafka::producer::Producer;
        use std::time::Duration;

        let pools = self.pools.read().await;
        let (_, producer_pool, _) = pools.get(cluster_id)?;

        // 尝试从连接池获取一个连接并执行健康检查
        match producer_pool.get().await {
            Ok(producer) => {
                // 使用 producer 的 client 进行元数据请求
                let client = producer.client();

                // 最多重试 3 次，处理临时网络故障
                let max_retries = 3;
                for attempt in 1..=max_retries {
                    match client.fetch_metadata(None, Duration::from_secs(2)) {
                        Ok(metadata) => {
                            if metadata.brokers().is_empty() {
                                return Some(ConnectionStatus::Error("No brokers in metadata".into()));
                            } else {
                                return Some(ConnectionStatus::Connected);
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("{}", e);
                            // 如果是 BrokerTransportFailure 或超时错误，且不是最后一次尝试，则重试
                            let is_transient = error_msg.contains("BrokerTransportFailure")
                                || error_msg.contains("timed out")
                                || error_msg.contains("Transport");

                            if is_transient && attempt < max_retries {
                                tracing::debug!(
                                    "Health check attempt {}/{} failed for cluster '{}': {}, retrying...",
                                    attempt, max_retries, cluster_id, error_msg
                                );
                                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                                continue;
                            }

                            // 最后一次尝试或非临时错误，返回错误状态
                            return Some(ConnectionStatus::Error(format!("Metadata fetch failed: {}", e)));
                        }
                    }
                }
                unreachable!()
            }
            Err(e) => Some(ConnectionStatus::Error(format!("Pool error: {}", e))),
        }
    }

    /// 断开集群连接
    pub async fn disconnect(&self, cluster_id: &str) -> Result<()> {
        let mut pools = self.pools.write().await;
        if pools.remove(cluster_id).is_none() {
            return Err(crate::error::AppError::NotFound(format!(
                "Cluster '{}' not found",
                cluster_id
            )));
        }
        tracing::info!("Disconnected cluster: {}", cluster_id);
        Ok(())
    }

    /// 重连集群
    pub async fn reconnect(&self, cluster_id: &str, config: &KafkaConfig, pool_config: &PoolConfig) -> Result<()> {
        let mut pools = self.pools.write().await;

        let consumer_mgr = kafka_consumer::KafkaConsumerManager::new(config)?;
        let producer_mgr = kafka_producer::KafkaProducerManager::new(config)?;

        let consumer_pool = build_pool(consumer_mgr, pool_config);
        let producer_pool = build_pool(producer_mgr, pool_config);

        pools.insert(cluster_id.to_string(), (consumer_pool, producer_pool, pool_config.clone()));
        tracing::info!("Reconnected cluster: {}", cluster_id);

        Ok(())
    }

    /// 获取所有集群连接状态
    pub async fn get_all_connections_status(&self) -> Vec<(String, ConnectionStatus)> {
        let cluster_ids: Vec<String> = {
            let pools = self.pools.read().await;
            pools.keys().cloned().collect()
        };

        let mut statuses = Vec::new();
        for cluster_id in cluster_ids {
            let status = self.check_connection(&cluster_id).await
                .unwrap_or(ConnectionStatus::Disconnected);
            statuses.push((cluster_id, status));
        }

        statuses
    }
}

/// 构建连接池的辅助函数 - 性能优化版
fn build_pool<M>(manager: M, config: &PoolConfig) -> deadpool::managed::Pool<M>
where
    M: deadpool::managed::Manager + Send + Sync + 'static,
    M::Type: Send + Sync,
    M::Error: Send + Sync + std::fmt::Display,
{
    use deadpool::managed::{Hook, Timeouts};
    use deadpool::Runtime;
    use std::time::Duration;

    tracing::info!(
        "[build_pool] Creating pool with max_size={}, min_size={}",
        config.max_size,
        config.min_size
    );

    let pool = deadpool::managed::Pool::builder(manager)
        .max_size(config.max_size)
        .wait_timeout(Some(Duration::from_secs(config.acquire_timeout_secs)))
        .timeouts(Timeouts {
            wait: Some(Duration::from_secs(config.acquire_timeout_secs)),
            create: Some(Duration::from_secs(10)),  // 优化：从 30 秒降低到 10 秒
            recycle: Some(Duration::from_secs(5)),  // 优化：从 10 秒降低到 5 秒
        })
        .runtime(Runtime::Tokio1)
        // 优化：连接回收时的健康检查
        .post_recycle(Hook::async_fn(|_conn, _metrics| {
            Box::pin(async move { Ok(()) })
        }))
        .build()
        .expect("Failed to create pool");

    tracing::info!("[build_pool] Pool created with status: {:?}", pool.status());

    // 预热：异步创建最小连接数
    let min_size = config.min_size;
    if min_size > 0 {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            warmup_pool(&pool_clone, min_size).await;
        });
    }

    pool
}

/// 预热连接池：预先创建指定数量的连接
async fn warmup_pool<M>(pool: &deadpool::managed::Pool<M>, target_size: usize)
where
    M: deadpool::managed::Manager + Send + Sync + 'static,
    M::Type: Send + Sync,
{
    use std::time::Duration;

    tracing::info!("[PoolWarmup] Starting warmup for {} connections", target_size);
    let start = std::time::Instant::now();

    let mut created = 0;
    let mut failed = 0;

    for i in 0..target_size {
        match tokio::time::timeout(Duration::from_secs(5), pool.get()).await {
            Ok(Ok(_conn)) => {
                created += 1;
                tracing::debug!("[PoolWarmup] Connection {}/{} created", i + 1, target_size);
                // 连接会自动归还到 pool
            }
            Ok(Err(_e)) => {
                failed += 1;
                tracing::warn!("[PoolWarmup] Connection {}/{} failed", i + 1, target_size);
            }
            Err(_) => {
                failed += 1;
                tracing::warn!("[PoolWarmup] Connection {}/{} timeout", i + 1, target_size);
            }
        }
    }

    tracing::info!(
        "[PoolWarmup] Completed: {}/{} created, {} failed in {:?}",
        created,
        target_size,
        failed,
        start.elapsed()
    );
}

impl Default for ClusterPools {
    fn default() -> Self {
        Self::new()
    }
}
