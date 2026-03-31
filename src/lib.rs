//! Kafka Manager API
//!
//! A Kafka management tool with RESTful API
#![allow(dead_code)]

pub mod config;
pub mod db;
pub mod error;
pub mod kafka;
pub mod middleware;
pub mod models;
pub mod pool;
pub mod routes;
pub mod cache;

use std::sync::Arc;
use std::collections::HashSet;
use std::sync::Mutex;
use arc_swap::ArcSwap;

pub use config::Config;
pub use db::DbPool;
pub use kafka::KafkaClients;
pub use pool::ClusterPools;
pub use cache::MetadataCache;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub clients: Arc<ArcSwap<KafkaClients>>,
    pub config: Config,
    pub pools: ClusterPools,
    pub cache: MetadataCache,
    /// 刷新状态跟踪（用于防止重复刷新）
    pub refresh_state: Arc<Mutex<RefreshState>>,
}

/// 刷新状态跟踪结构
#[derive(Debug, Default)]
pub struct RefreshState {
    /// 正在刷新 topic 的集群
    pub refreshing_topics: HashSet<String>,
    /// 正在刷新 consumer group 的集群
    pub refreshing_consumer_groups: HashSet<String>,
}

impl AppState {
    /// 获取 Kafka 客户端（无锁读取）
    pub fn get_clients(&self) -> Arc<KafkaClients> {
        self.clients.load_full()
    }

    /// 更新 Kafka 客户端（原子操作）
    pub fn set_clients(&self, clients: KafkaClients) {
        self.clients.store(clients.into());
    }

    /// 获取数据库连接池
    pub fn get_pool(&self) -> sqlx::Pool<sqlx::Sqlite> {
        self.db.inner().clone()
    }
}

pub use routes::create_router;
