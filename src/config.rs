use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    /// 单个 Kafka 配置（向后兼容，运行时使用）
    #[serde(skip)]
    pub kafka: KafkaConfig,
    /// 多个 Kafka 集群配置
    #[serde(default)]
    pub clusters: HashMap<String, KafkaConfig>,
    /// 连接池配置
    #[serde(default)]
    pub pool: PoolConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct KafkaConfig {
    #[serde(default)]
    pub brokers: String,
    #[serde(default)]
    pub request_timeout_ms: u32,
    #[serde(default = "default_operation_timeout")]
    pub operation_timeout_ms: u32,
}

/// 连接池配置
#[derive(Debug, Clone, Deserialize)]
pub struct PoolConfig {
    /// 最大连接数
    #[serde(default = "default_pool_max_size")]
    pub max_size: usize,
    /// 最小连接数
    #[serde(default = "default_pool_min_size")]
    pub min_size: usize,
    /// 连接获取超时时间（秒）
    #[serde(default = "default_pool_timeout")]
    pub acquire_timeout_secs: u64,
    /// 连接空闲超时时间（秒）
    #[serde(default = "default_pool_idle_timeout")]
    pub idle_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: default_pool_max_size(),
            min_size: default_pool_min_size(),
            acquire_timeout_secs: default_pool_timeout(),
            idle_timeout_secs: default_pool_idle_timeout(),
        }
    }
}

fn default_pool_max_size() -> usize {
    20
}

fn default_pool_min_size() -> usize {
    2
}

fn default_pool_timeout() -> u64 {
    30
}

fn default_pool_idle_timeout() -> u64 {
    600
}

fn default_operation_timeout() -> u32 {
    5000
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            // 默认配置
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 9732)?
            .set_default("kafka.request_timeout_ms", 5000)?
            // 加载配置文件
            .add_source(config::File::from(path.as_ref()).required(false))
            // 环境变量覆盖 (前缀：KAFKA_MANAGER)
            .add_source(
                config::Environment::with_prefix("KAFKA_MANAGER")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let mut config: Self = settings.try_deserialize()?;

        // 向后兼容：如果定义了单个 kafka 配置，将其添加到 clusters 中
        if !config.kafka.brokers.is_empty() {
            config.clusters.insert("default".to_string(), config.kafka.clone());
        } else if !config.clusters.is_empty() {
            // 如果没有单个 kafka 配置但有 clusters，使用第一个 cluster 作为 kafka
            if let Some((_, first_config)) = config.clusters.iter().next() {
                config.kafka = first_config.clone();
            }
        }

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut clusters = HashMap::new();
        let default_kafka = KafkaConfig {
            brokers: "localhost:9092".to_string(),
            request_timeout_ms: 5000,
            operation_timeout_ms: 5000,
        };
        clusters.insert("default".to_string(), default_kafka.clone());

        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 9732,
            },
            kafka: default_kafka,
            clusters,
            pool: PoolConfig::default(),
        }
    }
}
