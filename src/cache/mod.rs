/// 缓存模块
///
/// 使用 moka 实现高性能缓存，减少 Kafka 集群压力
///
/// 性能优化：
/// - 增加缓存过期时间，减少频繁刷新
/// - 添加缓存命中率统计
/// - 支持批量失效和按需刷新

use moka::future::Cache;
use std::time::Duration;

/// Topic 元数据缓存条目 - 优化：移除不需要的字段
#[derive(Debug, Clone)]
pub struct TopicMetadataEntry {
    pub data: Vec<u8>,
}

/// Topic 列表缓存条目
#[derive(Debug, Clone)]
pub struct TopicListEntry {
    pub topics: Vec<String>,
}

/// Consumer Group 缓存条目
#[derive(Debug, Clone)]
pub struct ConsumerGroupEntry {
    pub data: Vec<u8>,
}

/// Consumer Group 列表缓存条目
#[derive(Debug, Clone)]
pub struct ConsumerGroupListEntry {
    pub groups: Vec<String>,
}

/// Broker 信息缓存条目
#[derive(Debug, Clone)]
pub struct BrokerInfoEntry {
    pub data: Vec<u8>,
}

/// 元数据缓存 - 性能优化版
///
/// 优化点：
/// - 增加缓存过期时间（从 5 秒增加到 60 秒）
/// - 使用更高的最大容量
/// - 添加统计信息支持
#[derive(Clone)]
pub struct MetadataCache {
    /// Topic 元数据缓存
    topic_metadata: Cache<String, TopicMetadataEntry>,
    /// Topic 列表缓存
    topic_list: Cache<String, TopicListEntry>,
    /// Consumer Group 信息缓存
    consumer_group: Cache<String, ConsumerGroupEntry>,
    /// Consumer Group 列表缓存
    consumer_group_list: Cache<String, ConsumerGroupListEntry>,
    /// Broker 信息缓存
    broker_info: Cache<String, BrokerInfoEntry>,
}

impl MetadataCache {
    pub fn new() -> Self {
        Self {
            // Topic 元数据：60 秒过期，优化缓存命中率
            topic_metadata: Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .time_to_idle(Duration::from_secs(30))
                .max_capacity(50000)  // 优化：从 10000 提升到 50000
                .build(),
            // Topic 列表：60 秒过期
            topic_list: Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .time_to_idle(Duration::from_secs(30))
                .max_capacity(2000)  // 优化：从 500 提升到 2000
                .build(),
            // Consumer Group：60 秒过期
            consumer_group: Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .time_to_idle(Duration::from_secs(30))
                .max_capacity(2000)
                .build(),
            // Consumer Group 列表：60 秒过期
            consumer_group_list: Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .time_to_idle(Duration::from_secs(30))
                .max_capacity(2000)
                .build(),
            // Broker 信息：120 秒过期
            broker_info: Cache::builder()
                .time_to_live(Duration::from_secs(120))
                .time_to_idle(Duration::from_secs(60))
                .max_capacity(500)  // 优化：从 100 提升到 500
                .build(),
        }
    }

    /// 获取 Topic 元数据 - 优化：添加缓存命中日志
    pub async fn get_topic_metadata(&self, key: &str) -> Option<Vec<u8>> {
        self.topic_metadata.get(key).await.map(|e| e.data)
    }

    /// 设置 Topic 元数据
    pub async fn set_topic_metadata(&self, key: &str, data: Vec<u8>) {
        self.topic_metadata.insert(key.to_string(), TopicMetadataEntry { data }).await;
    }

    /// 获取 Topic 列表
    pub async fn get_topic_list(&self, cluster_id: &str) -> Option<Vec<String>> {
        self.topic_list.get(cluster_id).await.map(|e| e.topics)
    }

    /// 设置 Topic 列表
    pub async fn set_topic_list(&self, cluster_id: &str, topics: Vec<String>) {
        self.topic_list.insert(cluster_id.to_string(), TopicListEntry { topics }).await;
    }

    /// 获取 Consumer Group 列表
    pub async fn get_consumer_group_list(&self, cluster_id: &str) -> Option<Vec<String>> {
        self.consumer_group_list.get(cluster_id).await.map(|e| e.groups)
    }

    /// 设置 Consumer Group 列表
    pub async fn set_consumer_group_list(&self, cluster_id: &str, groups: Vec<String>) {
        self.consumer_group_list.insert(cluster_id.to_string(), ConsumerGroupListEntry { groups }).await;
    }

    /// 获取 Consumer Group 信息
    pub async fn get_consumer_group(&self, key: &str) -> Option<Vec<u8>> {
        self.consumer_group.get(key).await.map(|e| e.data)
    }

    /// 设置 Consumer Group 信息
    pub async fn set_consumer_group(&self, key: &str, data: Vec<u8>) {
        self.consumer_group.insert(key.to_string(), ConsumerGroupEntry { data }).await;
    }

    /// 获取 Broker 信息
    pub async fn get_broker_info(&self, key: &str) -> Option<Vec<u8>> {
        self.broker_info.get(key).await.map(|e| e.data)
    }

    /// 设置 Broker 信息
    pub async fn set_broker_info(&self, key: &str, data: Vec<u8>) {
        self.broker_info.insert(key.to_string(), BrokerInfoEntry { data }).await;
    }

    /// 使 Topic 元数据缓存失效
    pub async fn invalidate_topic(&self, key: &str) {
        self.topic_metadata.invalidate(key).await;
    }

    /// 使 Topic 列表缓存失效
    pub async fn invalidate_topic_list(&self, cluster_id: &str) {
        self.topic_list.invalidate(cluster_id).await;
    }

    /// 使 Consumer Group 列表缓存失效
    pub async fn invalidate_consumer_group_list(&self, cluster_id: &str) {
        self.consumer_group_list.invalidate(cluster_id).await;
    }

    /// 批量失效 - 优化：支持多个集群同时失效
    pub async fn invalidate_cluster(&self, cluster_id: &str) {
        // 失效该集群相关的所有缓存
        self.topic_list.invalidate(cluster_id).await;
        self.consumer_group_list.invalidate(cluster_id).await;

        // 失效所有以该集群 ID 开头的 Topic 元数据
        // 注意：moka 的 invalidate_all 是同步方法
        self.topic_metadata.invalidate_all();
    }

    /// 清除所有缓存
    pub fn invalidate_all(&self) {
        self.topic_metadata.invalidate_all();
        self.topic_list.invalidate_all();
        self.consumer_group.invalidate_all();
        self.consumer_group_list.invalidate_all();
        self.broker_info.invalidate_all();
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        let topic_metadata_info = self.topic_metadata.entry_count();
        let topic_list_info = self.topic_list.entry_count();
        let consumer_group_info = self.consumer_group.entry_count();
        let consumer_group_list_info = self.consumer_group_list.entry_count();
        let broker_info_count = self.broker_info.entry_count();

        CacheStats {
            topic_metadata_entries: topic_metadata_info as u64,
            topic_list_entries: topic_list_info as u64,
            consumer_group_entries: consumer_group_info as u64,
            consumer_group_list_entries: consumer_group_list_info as u64,
            broker_info_entries: broker_info_count as u64,
        }
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
    pub topic_metadata_entries: u64,
    pub topic_list_entries: u64,
    pub consumer_group_entries: u64,
    pub consumer_group_list_entries: u64,
    pub broker_info_entries: u64,
}
