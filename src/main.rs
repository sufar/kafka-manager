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
use std::path::PathBuf;
use arc_swap::ArcSwap;
use tower_http::{cors::CorsLayer, trace::TraceLayer, timeout::TimeoutLayer, compression::CompressionLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use crate::config::Config;
use crate::db::DbPool;
use crate::kafka::KafkaClients;
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
    /// 正在刷新的集群（无论是 topic 还是 consumer group，每个集群同一时间只能有一个刷新）
    pub refreshing_clusters: HashSet<String>,
    /// 是否正在刷新所有集群的 topic
    pub refreshing_all_topics: bool,
    /// 是否正在刷新所有集群的 consumer group
    pub refreshing_all_consumer_groups: bool,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志文件目录
    let log_dir = dirs::cache_dir()
        .map(|d| d.join("kafka-manager").join("logs"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager/logs"));

    // 确保日志目录存在
    let _ = std::fs::create_dir_all(&log_dir);

    // 启动时删除今天之前的旧日志
    cleanup_old_logs(&log_dir);

    // 按天滚动的日志文件配置
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("kafka-manager")
        .filename_suffix("log")
        .max_log_files(7)  // 保留最近 7 天的日志
        .build(&log_dir)
        .expect("Failed to create rolling file appender");

    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(file_appender);

    // 初始化日志（同时输出到控制台和文件）
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kafka_manager_api=info,tower_http=info,rdkafka=warn".into()),
        )
        // 控制台输出带颜色
        .with(tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true))
        // 文件输出不带颜色（纯文本），使用自定义时间格式
        .with(tracing_subscriber::fmt::layer()
            .with_writer(non_blocking_file)
            .with_ansi(false)
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                "%Y-%m-%d %H:%M:%S".to_string()
            )))
        .init();

    // 测试日志是否正常工作
    tracing::info!("=== Kafka Manager API starting ===");
    tracing::debug!("Debug logging enabled");
    tracing::info!("Log directory: {:?}", log_dir);

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

    // 清理过期的发送历史记录（保留最近 30 天）
    match crate::db::sent_message::cleanup_old_sent_messages(db.inner(), 30).await {
        Ok(count) => tracing::info!("Startup cleanup: cleaned up {} expired message records", count),
        Err(e) => tracing::warn!("Failed to cleanup old sent messages: {}", e),
    }

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

    // 预热 Consumer Pool（可选配置）
    let warmup_size = config.pool.min_size;
    if warmup_size > 0 {
        for (cluster_id, _) in &clusters {
            if let Some(consumer_pool) = pools.get_consumer_pool(cluster_id).await {
                let size = warmup_size as usize;
                tokio::spawn(async move {
                    let _ = crate::pool::kafka_consumer::warmup_consumer_pool(&consumer_pool, size).await;
                });
            }
        }
        tracing::info!("Consumer pool warmup started for {} clusters (size: {} per cluster)", clusters.len(), warmup_size);
    }

    // 初始化缓存
    let cache = MetadataCache::new();
    tracing::info!("Metadata cache initialized");

    // 初始化刷新状态跟踪
    let refresh_state = Arc::new(Mutex::new(RefreshState::default()));
    tracing::info!("Refresh state tracking initialized");

    // 应用状态
    let state = AppState {
        db: db.clone(),
        clients,
        config: config.clone(),
        pools: pools.clone(),
        cache: cache.clone(),
        refresh_state,
    };

    // 构建路由
    let app = routes::create_router(state.clone())
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

    let db_clusters = ClusterStore::list(pool).await?;
    let mut clusters = std::collections::HashMap::with_capacity(db_clusters.len());

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

/// 清理今天之前的旧日志文件
fn cleanup_old_logs(log_dir: &std::path::Path) {
    use std::fs;
    use chrono::NaiveDate;

    let today = chrono::Local::now().date_naive();

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // 检查文件名格式：kafka-manager.YYYY-MM-DD.log (RollingFileAppender 使用点号分隔)
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("kafka-manager.") && file_name.ends_with(".log") {
                    // 提取日期部分
                    let date_str = file_name
                        .trim_start_matches("kafka-manager.")
                        .trim_end_matches(".log");

                    // 解析日期
                    if let Ok(file_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        // 如果文件日期早于今天，删除
                        if file_date < today {
                            let _ = fs::remove_file(&path);
                            tracing::debug!("Deleted old log file: {:?}", path);
                        }
                    }
                }
            }
        }
    }
}
