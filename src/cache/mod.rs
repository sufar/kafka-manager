/// 缓存模块
///
/// 使用 moka 实现高性能缓存，减少 Kafka 集群压力

use moka::future::Cache;
use std::time::Duration;

/// 元数据缓存
#[derive(Clone)]
pub struct MetadataCache {
    /// Topic 元数据缓存
    topic_metadata: Cache<String, TopicMetadataEntry>,
    /// Consumer Group 信息缓存
    consumer_group: Cache<String, ConsumerGroupEntry>,
    /// Broker 信息缓存
    broker_info: Cache<String, BrokerInfoEntry>,
}

/// Topic 元数据缓存条目
#[derive(Debug, Clone)]
pub struct TopicMetadataEntry {
    pub data: Vec<u8>,  // 序列化的元数据
    pub fetched_at: i64,
}

/// Consumer Group 缓存条目
#[derive(Debug, Clone)]
pub struct ConsumerGroupEntry {
    pub data: Vec<u8>,
    pub fetched_at: i64,
}

/// Broker 信息缓存条目
#[derive(Debug, Clone)]
pub struct BrokerInfoEntry {
    pub data: Vec<u8>,
    pub fetched_at: i64,
}

impl MetadataCache {
    pub fn new() -> Self {
        Self {
            // Topic 元数据：5 秒过期
            topic_metadata: Cache::builder()
                .time_to_live(Duration::from_secs(5))
                .time_to_idle(Duration::from_secs(3))
                .max_capacity(1000)
                .build(),
            // Consumer Group：3 秒过期
            consumer_group: Cache::builder()
                .time_to_live(Duration::from_secs(3))
                .time_to_idle(Duration::from_secs(1))
                .max_capacity(500)
                .build(),
            // Broker 信息：10 秒过期
            broker_info: Cache::builder()
                .time_to_live(Duration::from_secs(10))
                .time_to_idle(Duration::from_secs(5))
                .max_capacity(100)
                .build(),
        }
    }

    /// 获取 Topic 元数据
    pub async fn get_topic_metadata(&self, key: &str) -> Option<Vec<u8>> {
        self.topic_metadata.get(key).await.map(|e| e.data)
    }

    /// 设置 Topic 元数据
    pub async fn set_topic_metadata(&self, key: &str, data: Vec<u8>) {
        self.topic_metadata.insert(key.to_string(), TopicMetadataEntry {
            data,
            fetched_at: chrono::Utc::now().timestamp_millis(),
        }).await;
    }

    /// 获取 Consumer Group 信息
    pub async fn get_consumer_group(&self, key: &str) -> Option<Vec<u8>> {
        self.consumer_group.get(key).await.map(|e| e.data)
    }

    /// 设置 Consumer Group 信息
    pub async fn set_consumer_group(&self, key: &str, data: Vec<u8>) {
        self.consumer_group.insert(key.to_string(), ConsumerGroupEntry {
            data,
            fetched_at: chrono::Utc::now().timestamp_millis(),
        }).await;
    }

    /// 获取 Broker 信息
    pub async fn get_broker_info(&self, key: &str) -> Option<Vec<u8>> {
        self.broker_info.get(key).await.map(|e| e.data)
    }

    /// 设置 Broker 信息
    pub async fn set_broker_info(&self, key: &str, data: Vec<u8>) {
        self.broker_info.insert(key.to_string(), BrokerInfoEntry {
            data,
            fetched_at: chrono::Utc::now().timestamp_millis(),
        }).await;
    }

    /// 使 Topic 元数据缓存失效
    pub async fn invalidate_topic(&self, key: &str) {
        self.topic_metadata.invalidate(key).await;
    }

    /// 清除所有缓存
    pub async fn invalidate_all(&self) {
        self.topic_metadata.invalidate_all();
        self.consumer_group.invalidate_all();
        self.broker_info.invalidate_all();
    }

    /// 获取缓存统计信息（moka 0.12 不支持 stats，返回默认值）
    pub fn stats(&self) -> CacheStats {
        CacheStats::default()
    }
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 缓存统计信息
#[derive(Debug, Default)]
pub struct CacheStats {
    pub topic_metadata_hits: u64,
    pub topic_metadata_misses: u64,
    pub consumer_group_hits: u64,
    pub consumer_group_misses: u64,
    pub broker_info_hits: u64,
    pub broker_info_misses: u64,
}
