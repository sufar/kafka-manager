pub mod admin;
pub mod consumer;
pub mod import_export;
pub mod message_query;
pub mod offset;
pub mod schema;
pub mod schema_registry;
pub mod throughput;
pub mod transaction;
mod producer;

pub use admin::KafkaAdmin;
pub use consumer::KafkaConsumer;
pub use message_query::{FetchMode, MessageQueryExecutor, MessageQueryParams, OrderBy, QueryStats, SearchScope, SortOrder};
pub use producer::KafkaProducer;

use crate::config::KafkaConfig;
use crate::error::AppError;
use rdkafka::ClientConfig;
use std::collections::HashMap;
use std::sync::Arc;

pub fn create_client_config(kafka_config: &KafkaConfig) -> ClientConfig {
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", &kafka_config.brokers);
    client_config.set(
        "request.timeout.ms",
        &kafka_config.request_timeout_ms.to_string(),
    );
    client_config.set(
        "socket.timeout.ms",
        &kafka_config.request_timeout_ms.to_string(),
    );
    client_config
}

/// 管理多个 Kafka 集群的客户端
#[derive(Clone)]
pub struct KafkaClients {
    /// 集群 ID -> (Admin, Consumer, Producer, Config)
    clients: Arc<HashMap<String, (Arc<KafkaAdmin>, Arc<KafkaConsumer>, Arc<KafkaProducer>, Arc<KafkaConfig>)>>,
}

impl KafkaClients {
    /// 从配置创建多个 Kafka 客户端
    pub fn new(clusters: &HashMap<String, KafkaConfig>) -> Result<Self, AppError> {
        let mut clients = HashMap::new();

        for (cluster_id, config) in clusters {
            let admin = Arc::new(KafkaAdmin::new(config)?);
            let consumer = Arc::new(KafkaConsumer::new(config)?);
            let producer = Arc::new(KafkaProducer::new(config)?);
            clients.insert(cluster_id.clone(), (admin, consumer, producer, Arc::new(config.clone())));
        }

        Ok(Self {
            clients: Arc::new(clients),
        })
    }

    /// 获取指定集群的 Admin 客户端
    pub fn get_admin(&self, cluster_id: &str) -> Option<Arc<KafkaAdmin>> {
        self.clients.get(cluster_id).map(|(admin, _, _, _)| Arc::clone(admin))
    }

    /// 获取指定集群的 Consumer 客户端
    pub fn get_consumer(&self, cluster_id: &str) -> Option<Arc<KafkaConsumer>> {
        self.clients.get(cluster_id).map(|(_, consumer, _, _)| Arc::clone(consumer))
    }

    /// 获取指定集群的 Producer 客户端
    pub fn get_producer(&self, cluster_id: &str) -> Option<Arc<KafkaProducer>> {
        self.clients.get(cluster_id).map(|(_, _, producer, _)| Arc::clone(producer))
    }

    /// 获取指定集群的配置
    pub fn get_config(&self, cluster_id: &str) -> Option<Arc<KafkaConfig>> {
        self.clients.get(cluster_id).map(|(_, _, _, config)| Arc::clone(config))
    }

    /// 获取所有集群 ID
    pub fn cluster_ids(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    /// 创建一个新的 KafkaClients 实例，移除指定集群
    pub fn without_cluster(&self, cluster_id: &str) -> Self {
        let mut new_clients = HashMap::new();

        for (id, (admin, consumer, producer, config)) in self.clients.iter() {
            if id != cluster_id {
                new_clients.insert(
                    id.clone(),
                    (Arc::clone(admin), Arc::clone(consumer), Arc::clone(producer), Arc::clone(config)),
                );
            }
        }

        Self {
            clients: Arc::new(new_clients),
        }
    }

    /// 创建一个新的 KafkaClients 实例，添加/替换指定集群
    pub fn with_added_cluster(&self, cluster_id: &str, config: &KafkaConfig) -> Result<Self, AppError> {
        let admin = Arc::new(KafkaAdmin::new(config)?);
        let consumer = Arc::new(KafkaConsumer::new(config)?);
        let producer = Arc::new(KafkaProducer::new(config)?);

        let mut new_clients = HashMap::new();

        // Copy existing clients
        for (id, (admin, consumer, producer, config)) in self.clients.iter() {
            new_clients.insert(
                id.clone(),
                (Arc::clone(admin), Arc::clone(consumer), Arc::clone(producer), Arc::clone(config)),
            );
        }

        // Add new cluster
        new_clients.insert(cluster_id.to_string(), (admin, consumer, producer, Arc::new(config.clone())));

        Ok(Self {
            clients: Arc::new(new_clients),
        })
    }
}
