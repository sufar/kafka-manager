#![allow(dead_code)]

mod cache;
mod config;
mod db;
mod error;
mod kafka;
mod middleware;
mod models;
mod pool;
mod routes;

#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashSet;
use std::sync::Mutex;
use arc_swap::ArcSwap;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::db::DbPool;
use crate::kafka::KafkaClients;
use crate::middleware::audit::audit_middleware;
use crate::pool::ClusterPools;
use crate::cache::MetadataCache;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub clients: Arc<ArcSwap<KafkaClients>>,
    pub config: Config,
    /// Kafka 连接池
    pub pools: ClusterPools,
    /// 元数据缓存
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kafka_manager_api=info,tower_http=info,rdkafka=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 测试日志是否正常工作
    tracing::info!("=== Kafka Manager API starting ===");
    tracing::debug!("Debug logging enabled");

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

    // 初始化刷新状态跟踪
    let refresh_state = Arc::new(Mutex::new(RefreshState::default()));
    tracing::info!("Refresh state tracking initialized");

    // 应用状态
    let state = AppState {
        db: db.clone(),
        clients: clients.clone(),
        config: config.clone(),
        pools: pools.clone(),
        cache: cache.clone(),
        refresh_state: refresh_state.clone(),
    };

    // 启动后台 Lag 数据采集任务
    let lag_collection_state = state.clone();
    tokio::spawn(async move {
        collect_lag_history_task(lag_collection_state).await;
    });
    tracing::info!("Consumer Group Lag collection task started (interval: 5s)");

    // 应用状态（用于 HTTP 服务）
    let http_state = AppState {
        db: db.clone(),
        clients: clients.clone(),
        config: config.clone(),
        pools: pools.clone(),
        cache: cache.clone(),
        refresh_state: refresh_state.clone(),
    };

    // 构建路由
    let app = routes::create_router(http_state.clone())
        .layer(axum::middleware::from_fn(audit_middleware))
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

/// Consumer Group Lag 数据采集任务
/// 每 5 秒采集一次所有 Consumer Group 的 lag 数据并存储到数据库
async fn collect_lag_history_task(state: AppState) {
    use crate::db::cluster::ClusterStore;
    use crate::db::consumer_group::ConsumerGroupStore;
    use crate::kafka::consumer_group::KafkaConsumerGroupManager;
    use tokio::time::{interval, Duration};

    let mut interval_timer = interval(Duration::from_secs(5));

    loop {
        interval_timer.tick().await;

        // 获取所有集群
        let clusters = match ClusterStore::list(state.db.inner()).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("[LagCollector] Failed to list clusters: {}", e);
                continue;
            }
        };

        for cluster in clusters {
            // 获取该集群的所有 Consumer Groups
            let groups = match ConsumerGroupStore::list_by_cluster(state.db.inner(), &cluster.name).await {
                Ok(g) => g,
                Err(e) => {
                    tracing::warn!("[LagCollector] Failed to list consumer groups for cluster {}: {}", cluster.name, e);
                    continue;
                }
            };

            for group in groups {
                // 构建 Kafka 配置
                let kafka_config = crate::config::KafkaConfig {
                    brokers: cluster.brokers.clone(),
                    request_timeout_ms: cluster.request_timeout_ms as u32,
                    operation_timeout_ms: cluster.operation_timeout_ms as u32,
                };

                // 创建 Consumer Group 管理器
                let cg_manager = match KafkaConsumerGroupManager::new(&kafka_config) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("[LagCollector] Failed to create consumer group manager for cluster {}: {}", cluster.name, e);
                        continue;
                    }
                };

                // 获取 lag 数据
                let offsets = match cg_manager.get_consumer_group_offsets_auto(&group.group_name) {
                    Ok(o) => o,
                    Err(e) => {
                        tracing::warn!("[LagCollector] Failed to get consumer group offsets for cluster {}/group {}: {}",
                            cluster.name, group.group_name, e);
                        continue;
                    }
                };

                if offsets.is_empty() {
                    continue;
                }

                // 计算总 lag
                let total_lag: i64 = offsets.iter().map(|o| o.lag).sum();
                let timestamp = chrono::Utc::now().timestamp_millis();

                // 保存每个分区的 lag
                for offset in offsets {
                    let _ = sqlx::query(
                        r#"
                        INSERT INTO consumer_group_lag_history
                        (cluster_id, group_name, timestamp, total_lag, topic, partition, partition_lag)
                        VALUES (?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(&cluster.name)
                    .bind(&group.group_name)
                    .bind(timestamp)
                    .bind(total_lag)
                    .bind(&offset.topic)
                    .bind(offset.partition)
                    .bind(offset.lag)
                    .execute(state.db.inner())
                    .await;
                }
            }
        }

        // 清理旧数据（保留 24 小时）
        let cutoff = chrono::Utc::now().timestamp_millis() - 24 * 60 * 60 * 1000;
        let _ = sqlx::query("DELETE FROM consumer_group_lag_history WHERE timestamp < ?")
            .bind(cutoff)
            .execute(state.db.inner())
            .await;
    }
}
