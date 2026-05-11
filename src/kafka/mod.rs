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
use rdkafka::config::RDKafkaLogLevel;
use std::collections::HashMap;
use std::sync::Arc;

/// 自定义 rdkafka ClientContext，抑制 AllBrokersDown 噪音日志
struct KafkaClientContext;

impl rdkafka::ClientContext for KafkaClientContext {
    fn log(&self, level: RDKafkaLogLevel, _fac: &str, log_message: &str) {
        // 抑制 AllBrokersDown 噪音（broker 不可达时 librdkafka 会持续重试并打印）
        if log_message.contains("AllBrokersDown") {
            return;
        }
        match level {
            RDKafkaLogLevel::Emerg
            | RDKafkaLogLevel::Alert
            | RDKafkaLogLevel::Critical
            | RDKafkaLogLevel::Error => {
                tracing::error!("[rdkafka] {}", log_message);
            }
            RDKafkaLogLevel::Warning => {
                tracing::warn!("[rdkafka] {}", log_message);
            }
            _ => {
                tracing::debug!("[rdkafka] {}", log_message);
            }
        }
    }
}

/// 创建 rdkafka 客户端配置
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
    // 连接建立超时（DNS + TCP），避免无效地址等待过久
    let connect_timeout = (kafka_config.request_timeout_ms / 2).max(1000).min(3000);
    client_config.set(
        "socket.connection.setup.timeout.ms",
        &connect_timeout.to_string(),
    );
    // 限制最大重试次数，避免长时间重试等待
    client_config.set("retries", "3");
    client_config.set("retry.backoff.ms", "200");
    // 强制使用 IPv4，避免 IPv6 连接问题
    client_config.set("broker.address.family", "v4");

    client_config
}

/// 快速 TCP 探测 broker 是否可达（不依赖 Kafka 协议）
pub async fn test_brokers_connectivity(brokers: &str, timeout_ms: u64) -> bool {
    use std::time::Duration;

    let timeout = Duration::from_millis(timeout_ms);
    let broker_list: Vec<String> = brokers
        .split(',')
        .filter_map(|b| {
            let b = b.trim();
            if b.is_empty() { None } else { Some(b.to_string()) }
        })
        .collect();

    if broker_list.is_empty() {
        return false;
    }

    // 并行尝试所有 broker，只要有一个成功即可
    let mut handles = Vec::new();
    for broker in broker_list {
        handles.push(tokio::spawn(async move {
            match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&broker)).await {
                Ok(Ok(_stream)) => true,
                _ => false,
            }
        }));
    }

    let results = futures::future::join_all(handles).await;
    results.into_iter().any(|r| r.unwrap_or(false))
}

/// 管理多个 Kafka 集群的客户端
#[derive(Clone)]
pub struct KafkaClients {
    /// 集群 ID -> (Admin, Consumer, Producer, Config)
    clients: Arc<HashMap<String, (Arc<KafkaAdmin>, Arc<KafkaConsumer>, Arc<KafkaProducer>, Arc<KafkaConfig>)>>,
}

impl KafkaClients {
    /// 从配置创建多个 Kafka 客户端（并发创建）
    pub fn new(clusters: &HashMap<String, KafkaConfig>) -> Result<Self, AppError> {
        use std::thread;

        let cluster_configs: Vec<(String, KafkaConfig)> = clusters.iter()
            .map(|(id, cfg)| (id.clone(), cfg.clone()))
            .collect();

        let cluster_count = cluster_configs.len();
        let mut handles = Vec::with_capacity(cluster_count);

        // 并发创建每个集群的客户端
        for (cluster_id, config) in cluster_configs {
            let handle = thread::spawn(move || {
                let admin = KafkaAdmin::new(&config)?;
                let consumer = KafkaConsumer::new(&config)?;
                let producer = KafkaProducer::new(&config)?;
                Ok::<_, AppError>((cluster_id, admin, consumer, producer, config))
            });
            handles.push(handle);
        }

        let mut clients = HashMap::with_capacity(cluster_count);
        let mut errors = Vec::new();

        for handle in handles {
            match handle.join() {
                Ok(Ok((id, admin, consumer, producer, config))) => {
                    clients.insert(id, (
                        Arc::new(admin),
                        Arc::new(consumer),
                        Arc::new(producer),
                        Arc::new(config),
                    ));
                }
                Ok(Err(e)) => {
                    errors.push(e.to_string());
                }
                Err(_) => {
                    errors.push("Cluster creation panicked".to_string());
                }
            }
        }

        if !errors.is_empty() {
            return Err(AppError::Internal(format!(
                "Failed to create clients for {} cluster(s): {}",
                errors.len(),
                errors.join("; ")
            )));
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

    /// 重连指定集群（创建新的客户端实例，替换旧的连接）
    pub fn reconnect_cluster(&self, cluster_id: &str, config: &KafkaConfig) -> Result<Self, AppError> {
        let new_admin = Arc::new(KafkaAdmin::new(config)?);
        let new_consumer = Arc::new(KafkaConsumer::new(config)?);
        let new_producer = Arc::new(KafkaProducer::new(config)?);
        let new_config = Arc::new(config.clone());

        let mut new_clients = HashMap::with_capacity(self.clients.len());

        for (id, (admin, consumer, producer, cfg)) in self.clients.iter() {
            if id == cluster_id {
                // 用新的客户端替换
                new_clients.insert(
                    id.clone(),
                    (
                        Arc::clone(&new_admin),
                        Arc::clone(&new_consumer),
                        Arc::clone(&new_producer),
                        Arc::clone(&new_config),
                    ),
                );
            } else {
                new_clients.insert(
                    id.clone(),
                    (Arc::clone(admin), Arc::clone(consumer), Arc::clone(producer), Arc::clone(cfg)),
                );
            }
        }

        Ok(Self {
            clients: Arc::new(new_clients),
        })
    }
}

impl Default for KafkaClients {
    fn default() -> Self {
        Self {
            clients: Arc::new(HashMap::new()),
        }
    }
}
