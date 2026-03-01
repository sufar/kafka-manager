/// Kafka 连接池模块
///
/// 提供 Kafka Consumer 和 Producer 的连接池管理，避免频繁创建/销毁连接

pub mod kafka_consumer;
pub mod kafka_producer;

pub use kafka_consumer::KafkaConsumerPool;
pub use kafka_producer::KafkaProducerPool;

use crate::config::KafkaConfig;
use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;

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
    /// 集群 ID -> (Consumer Pool, Producer Pool)
    pools: Arc<tokio::sync::RwLock<HashMap<String, (KafkaConsumerPool, KafkaProducerPool)>>>,
}

impl ClusterPools {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 初始化所有集群的连接池
    pub async fn init(&self, clusters: &HashMap<String, KafkaConfig>) -> Result<()> {
        use deadpool::managed::Pool;

        let mut pools = self.pools.write().await;

        for (cluster_id, config) in clusters {
            let consumer_mgr = kafka_consumer::KafkaConsumerManager::new(config)?;
            let producer_mgr = kafka_producer::KafkaProducerManager::new(config)?;

            let consumer_pool = Pool::builder(consumer_mgr)
                .max_size(10)
                .build()
                .map_err(|e| crate::error::AppError::Internal(format!("Failed to create consumer pool: {}", e)))?;

            let producer_pool = Pool::builder(producer_mgr)
                .max_size(10)
                .build()
                .map_err(|e| crate::error::AppError::Internal(format!("Failed to create producer pool: {}", e)))?;

            pools.insert(cluster_id.clone(), (consumer_pool, producer_pool));
        }

        Ok(())
    }

    /// 获取指定集群的 Consumer 连接池
    pub async fn get_consumer_pool(&self, cluster_id: &str) -> Option<KafkaConsumerPool> {
        let pools = self.pools.read().await;
        pools.get(cluster_id).map(|(c, _)| c.clone())
    }

    /// 获取指定集群的 Producer 连接池
    pub async fn get_producer_pool(&self, cluster_id: &str) -> Option<KafkaProducerPool> {
        let pools = self.pools.read().await;
        pools.get(cluster_id).map(|(_, p)| p.clone())
    }

    /// 添加新的集群连接池
    pub async fn add_cluster(&self, cluster_id: &str, config: &KafkaConfig) -> Result<()> {
        use deadpool::managed::Pool;

        let mut pools = self.pools.write().await;

        let consumer_mgr = kafka_consumer::KafkaConsumerManager::new(config)?;
        let producer_mgr = kafka_producer::KafkaProducerManager::new(config)?;

        let consumer_pool = Pool::builder(consumer_mgr)
            .max_size(10)
            .build()
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to create consumer pool: {}", e)))?;

        let producer_pool = Pool::builder(producer_mgr)
            .max_size(10)
            .build()
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to create producer pool: {}", e)))?;

        pools.insert(cluster_id.to_string(), (consumer_pool, producer_pool));

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
        let (_, producer_pool) = pools.get(cluster_id)?;

        // 尝试从连接池获取一个连接并执行健康检查
        match producer_pool.get().await {
            Ok(producer) => {
                // 使用 producer 的 client 进行元数据请求
                // 使用较短的超时时间 (2 秒) 并强制刷新元数据
                let client = producer.client();
                // topic: None 表示获取所有 topic 的元数据
                // timeout: 使用较短的超时时间以便快速失败
                match client.fetch_metadata(None, Duration::from_secs(2)) {
                    Ok(metadata) => {
                        // 额外检查：确保获取到了 broker 信息
                        if metadata.brokers().is_empty() {
                            Some(ConnectionStatus::Error("No brokers in metadata".into()))
                        } else {
                            Some(ConnectionStatus::Connected)
                        }
                    }
                    Err(e) => Some(ConnectionStatus::Error(format!("Metadata fetch failed: {}", e))),
                }
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
    pub async fn reconnect(&self, cluster_id: &str, config: &KafkaConfig) -> Result<()> {
        use deadpool::managed::Pool;

        let mut pools = self.pools.write().await;

        let consumer_mgr = kafka_consumer::KafkaConsumerManager::new(config)?;
        let producer_mgr = kafka_producer::KafkaProducerManager::new(config)?;

        let consumer_pool = Pool::builder(consumer_mgr)
            .max_size(10)
            .build()
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to create consumer pool: {}", e)))?;

        let producer_pool = Pool::builder(producer_mgr)
            .max_size(10)
            .build()
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to create producer pool: {}", e)))?;

        pools.insert(cluster_id.to_string(), (consumer_pool, producer_pool));
        tracing::info!("Reconnected cluster: {}", cluster_id);

        Ok(())
    }

    /// 获取所有集群连接状态
    pub async fn get_all_connections_status(&self) -> Vec<(String, ConnectionStatus)> {
        // 先获取所有集群 ID 列表
        let cluster_ids: Vec<String> = {
            let pools = self.pools.read().await;
            pools.keys().cloned().collect()
        };

        // 然后逐个检查连接状态
        let mut statuses = Vec::new();
        for cluster_id in cluster_ids {
            let status = self.check_connection(&cluster_id).await
                .unwrap_or(ConnectionStatus::Disconnected);
            statuses.push((cluster_id, status));
        }

        statuses
    }
}

impl Default for ClusterPools {
    fn default() -> Self {
        Self::new()
    }
}
