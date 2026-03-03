//! Kafka Manager API
//!
//! A Kafka management tool with RESTful API

pub mod config;
pub mod db;
pub mod error;
pub mod kafka;
pub mod middleware;
pub mod models;
pub mod pool;
pub mod routes;
pub mod task;
pub mod cache;

use std::sync::Arc;
use arc_swap::ArcSwap;

pub use config::Config;
pub use db::DbPool;
pub use kafka::KafkaClients;
pub use pool::ClusterPools;
pub use cache::MetadataCache;
pub use task::{TaskStore, HealthChecker, HealthCheckConfig};
pub use middleware::auth::{auth_middleware, AuthMiddleware};

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub clients: Arc<ArcSwap<KafkaClients>>,
    pub config: Config,
    pub auth: AuthMiddleware,
    pub pools: ClusterPools,
    pub cache: MetadataCache,
    pub task_store: TaskStore,
    pub health_checker: HealthChecker,
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
}

pub use routes::create_router;
