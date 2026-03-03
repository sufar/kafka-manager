mod cache;
mod config;
mod db;
mod error;
mod kafka;
mod middleware;
mod models;
mod pool;
mod routes;
mod task;

#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::sync::Arc;
use arc_swap::ArcSwap;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum::Extension;
use std::time::Duration;

use crate::config::Config;
use crate::db::DbPool;
use crate::kafka::KafkaClients;
use crate::middleware::audit::audit_middleware;
use crate::middleware::auth::{auth_middleware, AuthMiddleware};
use crate::middleware::performance::performance_tracker;
use crate::pool::ClusterPools;
use crate::cache::MetadataCache;
use crate::task::{TaskStore, HealthChecker, HealthCheckConfig};

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub clients: Arc<ArcSwap<KafkaClients>>,
    pub config: Config,
    pub auth: AuthMiddleware,
    /// Kafka 连接池
    pub pools: ClusterPools,
    /// 元数据缓存
    pub cache: MetadataCache,
    /// 异步任务队列
    pub task_store: TaskStore,
    /// 健康检查器
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kafka_manager=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let config = Config::load("config.toml")?;
    tracing::info!(
        "Starting server on {}:{}",
        config.server.host,
        config.server.port
    );

    // 初始化数据库
    let db = DbPool::new("kafka_manager.db").await?;
    db.init().await?;
    tracing::info!("Database initialized: kafka_manager.db");

    // 从数据库加载集群配置并创建 Kafka 客户端
    let clusters = load_clusters_from_db(db.inner()).await?;
    let clients = KafkaClients::new(&clusters)?;
    let clients: Arc<ArcSwap<KafkaClients>> = Arc::new(ArcSwap::new(clients.into()));
    tracing::info!(
        "Connected to Kafka clusters: {:?}",
        clients.load_full().cluster_ids()
    );

    // 初始化连接池
    let pools = ClusterPools::new();
    pools.init(&clusters, &config.pool).await?;
    tracing::info!("Kafka connection pools initialized");

    // 初始化缓存
    let cache = MetadataCache::new();
    tracing::info!("Metadata cache initialized");

    // 初始化任务队列
    let task_store = TaskStore::new();
    tracing::info!("Task queue initialized");

    // 初始化健康检查器
    let health_check_config = HealthCheckConfig {
        check_interval_secs: std::env::var("HEALTH_CHECK_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30),
        history_retention_hours: std::env::var("HEALTH_CHECK_RETENTION_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24),
        auto_reconnect: std::env::var("HEALTH_CHECK_AUTO_RECONNECT")
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false),
        max_reconnect_retries: std::env::var("HEALTH_CHECK_MAX_RECONNECT_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3),
    };
    let health_checker = HealthChecker::new(health_check_config);

    // 启动后台健康检查任务
    let health_checker_clone = health_checker.clone();
    let pools_clone = pools.clone();
    let db_pool = db.inner().clone();
    tokio::spawn(async move {
        let _ = health_checker_clone.start(pools_clone, db_pool).await;
    });
    tracing::info!("Health checker started");

    // 从数据库加载 API Keys 并初始化认证中间件
    let api_keys = load_api_keys_from_db(db.inner()).await?;
    let auth_enabled = std::env::var("AUTH_ENABLED")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    let auth = AuthMiddleware::new(api_keys, auth_enabled);
    if auth_enabled {
        tracing::info!("API authentication enabled");
    }

    // 应用状态
    let state = AppState {
        db: db.clone(),
        clients,
        config: config.clone(),
        auth: auth.clone(),
        pools: pools.clone(),
        cache: cache.clone(),
        task_store: task_store.clone(),
        health_checker: health_checker.clone(),
    };

    // 构建路由
    let app = routes::create_router(state.clone())
        .layer(axum::middleware::from_fn(audit_middleware))
        .layer(axum::middleware::from_fn(auth_middleware))
        .layer(Extension(state.auth.clone()))
        .layer(axum::middleware::from_fn(performance_tracker))
        .layer(TimeoutLayer::new(Duration::from_secs(300)))
        .layer(CompressionLayer::new()
            .gzip(true)
            .br(true))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // 启动服务器
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .expect("Invalid address");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on {}", addr);

    // 启动后台任务：定期清理旧任务
    let cleanup_store = task_store.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await; // 每小时清理一次
            let removed = cleanup_store.cleanup_old(24).await;
            if removed > 0 {
                tracing::info!("Cleaned up {} old tasks", removed);
            }
        }
    });

    axum::serve(listener, app).await?;

    Ok(())
}

/// 从数据库加载集群配置
async fn load_clusters_from_db(
    pool: &sqlx::SqlitePool,
) -> Result<std::collections::HashMap<String, crate::config::KafkaConfig>, crate::error::AppError> {
    use crate::config::KafkaConfig;
    use crate::db::cluster::ClusterStore;

    let mut clusters = std::collections::HashMap::new();
    let db_clusters = ClusterStore::list(pool).await?;

    for cluster in db_clusters {
        clusters.insert(
            cluster.name,
            KafkaConfig {
                brokers: cluster.brokers,
                request_timeout_ms: cluster.request_timeout_ms as u32,
                operation_timeout_ms: cluster.operation_timeout_ms as u32,
            },
        );
    }

    Ok(clusters)
}

/// 从数据库加载 API Keys
async fn load_api_keys_from_db(
    _pool: &sqlx::SqlitePool,
) -> Result<Vec<String>, crate::error::AppError> {
    // 从环境变量加载 API Keys
    // 注意：数据库中的 API Keys 用于管理（添加/删除），但验证时仍使用环境变量
    // 如果需要完全基于数据库的认证，可以修改 validate_key 函数查询数据库
    let api_keys = std::env::var("API_KEYS")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    Ok(api_keys)
}
