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

use std::sync::Arc;
use std::collections::HashSet;
use std::sync::Mutex;
use arc_swap::ArcSwap;

pub use config::Config;
pub use db::DbPool;
pub use kafka::KafkaClients;
pub use pool::ClusterPools;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub clients: Arc<ArcSwap<KafkaClients>>,
    pub config: Config,
    pub pools: ClusterPools,
    /// 刷新状态跟踪（用于防止重复刷新）
    pub refresh_state: Arc<Mutex<RefreshState>>,
    /// 导入导出全局锁（同一时间只能有一个导入或导出在进行）
    pub import_export_lock: Arc<Mutex<ImportExportLock>>,
}

/// 刷新状态跟踪结构
#[derive(Debug, Default)]
pub struct RefreshState {
    /// 正在刷新的集群（每个集群同一时间只能有一个 topic/consumer group 刷新）
    pub refreshing_clusters: HashSet<String>,
}

/// 导入导出全局状态（同一时间只能有一个导入或导出在进行）
#[derive(Debug, Default)]
pub struct ImportExportLock {
    pub is_busy: bool,
    pub operation: Option<String>, // "import" 或 "export"
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
