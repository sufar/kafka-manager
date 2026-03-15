/// 集群健康检查后台任务模块
///
/// 定期检查所有集群的连接状态并记录历史

use crate::db::cluster_connection::ClusterConnectionStore;
use crate::db::cluster::ClusterStore;
use crate::pool::{ClusterPools, ConnectionStatus};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use chrono::Timelike;

/// 健康检查配置
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// 检查间隔（秒）
    pub check_interval_secs: u64,
    /// 历史数据保留时间（小时）
    pub history_retention_hours: i64,
    /// 是否启用自动重连
    pub auto_reconnect: bool,
    /// 自动重连最大重试次数
    pub max_reconnect_retries: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            history_retention_hours: 24,
            auto_reconnect: false,
            max_reconnect_retries: 3,
        }
    }
}

/// 集群健康状态（内存缓存）
#[derive(Debug, Clone)]
pub struct ClusterHealthStatus {
    pub cluster_name: String,
    pub status: ConnectionStatus,
    pub last_check_time: i64,
    pub consecutive_failures: u32,
}

/// 健康检查管理器
#[derive(Clone)]
pub struct HealthChecker {
    config: HealthCheckConfig,
    statuses: Arc<RwLock<std::collections::HashMap<String, ClusterHealthStatus>>>,
}

impl HealthChecker {
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            statuses: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &HealthCheckConfig {
        &self.config
    }

    /// 获取所有集群的当前健康状态
    pub async fn get_all_statuses(&self) -> Vec<ClusterHealthStatus> {
        let statuses = self.statuses.read().await;
        statuses.values().cloned().collect()
    }

    /// 获取指定集群的健康状态
    pub async fn get_status(&self, cluster_name: &str) -> Option<ClusterHealthStatus> {
        let statuses = self.statuses.read().await;
        statuses.get(cluster_name).cloned()
    }

    /// 启动后台健康检查任务
    pub fn start(
        &self,
        pools: ClusterPools,
        db_pool: sqlx::SqlitePool,
    ) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let statuses = Arc::clone(&self.statuses);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.check_interval_secs));

            loop {
                interval.tick().await;

                // 获取所有集群
                match ClusterStore::list(&db_pool).await {
                    Ok(clusters) => {
                        // 并行检查所有集群，避免顺序执行导致长时间阻塞
                        let mut tasks = Vec::new();
                        for cluster in clusters {
                            let pools_clone = pools.clone();
                            let db_pool_clone = db_pool.clone();
                            let statuses_clone = statuses.clone();
                            let cluster_name = cluster.name.clone();
                            let cluster_brokers = cluster.brokers.clone();
                            let cluster_request_timeout = cluster.request_timeout_ms;
                            let cluster_operation_timeout = cluster.operation_timeout_ms;
                            let auto_reconnect = config.auto_reconnect;
                            let max_reconnect_retries = config.max_reconnect_retries;

                            tasks.push(tokio::spawn(async move {
                                let start = std::time::Instant::now();

                                // 检查连接状态（带超时）
                                let status = tokio::time::timeout(
                                    Duration::from_secs(5),
                                    pools_clone.check_connection(&cluster_name)
                                ).await
                                .unwrap_or(None)
                                .unwrap_or(ConnectionStatus::Disconnected);

                                let latency_ms = start.elapsed().as_millis() as i64;

                                let conn_status = status;

                                // 更新内存状态
                                {
                                    let mut status_map = statuses_clone.write().await;
                                    let entry = status_map.entry(cluster_name.clone())
                                        .or_insert_with(|| ClusterHealthStatus {
                                            cluster_name: cluster_name.clone(),
                                            status: conn_status.clone(),
                                            last_check_time: chrono::Utc::now().timestamp_millis(),
                                            consecutive_failures: 0,
                                        });

                                    entry.status = conn_status.clone();
                                    entry.last_check_time = chrono::Utc::now().timestamp_millis();

                                    if matches!(conn_status, ConnectionStatus::Error(_) | ConnectionStatus::Disconnected) {
                                        entry.consecutive_failures += 1;
                                    } else {
                                        entry.consecutive_failures = 0;
                                    }
                                }

                                // 记录到数据库
                                let status_str = match &conn_status {
                                    ConnectionStatus::Connected => "connected",
                                    ConnectionStatus::Disconnected => "disconnected",
                                    ConnectionStatus::Error(_) => "error",
                                };

                                let error_message = match &conn_status {
                                    ConnectionStatus::Error(msg) => Some(msg.as_str()),
                                    _ => None,
                                };

                                let _ = ClusterConnectionStore::record(
                                    &db_pool_clone,
                                    &cluster_name,
                                    status_str,
                                    error_message,
                                    Some(latency_ms),
                                ).await;

                                // 自动重连逻辑
                                if auto_reconnect && matches!(conn_status, ConnectionStatus::Error(_)) {
                                    let current_failures = {
                                        let status_map = statuses_clone.read().await;
                                        status_map.get(&cluster_name)
                                            .map(|s| s.consecutive_failures)
                                            .unwrap_or(0)
                                    };

                                    if current_failures <= max_reconnect_retries {
                                        tracing::info!(
                                            "Auto-reconnecting cluster {} (attempt {}/{})",
                                            cluster_name,
                                            current_failures,
                                            max_reconnect_retries
                                        );

                                        let kafka_config = crate::config::KafkaConfig {
                                            brokers: cluster_brokers,
                                            request_timeout_ms: cluster_request_timeout as u32,
                                            operation_timeout_ms: cluster_operation_timeout as u32,
                                        };

                                        if let Err(e) = pools_clone.reconnect(&cluster_name, &kafka_config, &crate::config::PoolConfig::default()).await {
                                            tracing::error!(
                                                "Auto-reconnect failed for cluster {}: {}",
                                                cluster_name,
                                                e
                                            );
                                        }
                                    }
                                }
                            }));
                        }

                        // 等待所有集群检查完成
                        for task in tasks {
                            let _ = task.await;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to list clusters for health check: {}", e);
                    }
                }

                // 定期清理旧数据（每小时）
                if i64::from(chrono::Utc::now().num_seconds_from_midnight() % 3600) < config.check_interval_secs as i64 {
                    match ClusterConnectionStore::cleanup_old(&db_pool, config.history_retention_hours).await {
                        Ok(count) if count > 0 => {
                            tracing::info!("Cleaned up {} old health check records", count);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Failed to cleanup old health check records: {}", e);
                        }
                    }
                }
            }
        })
    }
}
