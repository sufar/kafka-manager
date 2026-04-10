pub mod admin;
pub mod consumer;
pub mod consumer_group;
pub mod import_export;
pub mod offset;
pub mod throughput;
pub mod transaction;
pub mod avro;
pub mod protobuf;
mod producer;
pub mod schema_registry_client;

pub use admin::KafkaAdmin;
pub use consumer::KafkaConsumer;
pub use producer::KafkaProducer;

use crate::config::KafkaConfig;
use crate::error::AppError;
use rdkafka::ClientConfig;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// 全局代理 URL 存储（用于 HTTP 请求，如更新下载、Schema Registry 等）
pub static GLOBAL_PROXY_URL: Mutex<Option<String>> = Mutex::new(None);

pub fn set_global_proxy(url: String) {
    let mut proxy = GLOBAL_PROXY_URL.lock().unwrap();
    *proxy = if url.is_empty() { None } else { Some(url) };
}

pub fn get_global_proxy() -> Option<String> {
    GLOBAL_PROXY_URL.lock().unwrap().clone()
}

/// 检测代理是否为 SOCKS5 类型
pub fn is_socks_proxy(url: &str) -> bool {
    url.starts_with("socks5://") || url.starts_with("socks5h://") || url.starts_with("socks://")
}

/// 为已有的 ClientConfig 应用 SOCKS5 代理（如果已设置）
pub fn apply_proxy_if_socks(config: &mut ClientConfig) {
    if let Some(proxy_url) = get_global_proxy() {
        if is_socks_proxy(&proxy_url) {
            config.set("socks", &proxy_url);
        }
    }
}

/// 创建 rdkafka 客户端配置，自动应用 SOCKS5 代理（如果已设置）
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
    // 强制使用 IPv4，避免 IPv6 连接问题
    client_config.set("broker.address.family", "v4");

    // 应用 SOCKS5 代理（rdkafka 不支持 HTTP 代理）
    apply_proxy_if_socks(&mut client_config);

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
        let mut clients = HashMap::with_capacity(clusters.len());

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
        let mut new_clients = HashMap::with_capacity(self.clients.len().saturating_sub(1));

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

        let mut new_clients = HashMap::with_capacity(self.clients.len() + 1);

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
