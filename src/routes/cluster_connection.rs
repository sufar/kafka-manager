/// 集群连接管理路由
///
/// 提供集群连接的断开、重连、状态查询和健康检查功能

use crate::db::cluster::ClusterStore;
use crate::db::cluster_connection::ClusterConnectionStore;
use crate::error::{AppError, Result};
use crate::pool::ConnectionStatus;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_all_connections_status))
        .route("/:id/status", get(get_connection_status))
        .route("/:id/disconnect", post(disconnect_cluster))
        .route("/:id/reconnect", post(reconnect_cluster))
        .route("/:id/health-check", post(health_check_cluster))
        .route("/:id/metrics", get(get_connection_metrics))
        .route("/:id/history", get(get_connection_history))
        .route("/:id/stats", get(get_connection_stats))
        .route("/batch/disconnect", post(batch_disconnect))
        .route("/batch/reconnect", post(batch_reconnect))
}

#[derive(Debug, Serialize)]
pub struct ConnectionStatusResponse {
    pub cluster_id: String,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AllConnectionsStatusResponse {
    pub connections: Vec<ConnectionStatusResponse>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub cluster_id: String,
    pub healthy: bool,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DisconnectResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ReconnectResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectionMetricsResponse {
    pub cluster_id: String,
    pub consumer_pool_size: usize,
    pub producer_pool_size: usize,
    pub consumer_pool_available: usize,
    pub producer_pool_available: usize,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default = "default_history_limit")]
    pub limit: i64,
}

fn default_history_limit() -> i64 {
    100
}

#[derive(Debug, Serialize)]
pub struct ConnectionHistoryResponse {
    pub cluster_id: String,
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub status: String,
    pub error_message: Option<String>,
    pub latency_ms: Option<i64>,
    pub checked_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectionStatsResponse {
    pub cluster_id: String,
    pub total_checks: i64,
    pub successful_checks: i64,
    pub failed_checks: i64,
    pub success_rate: f64,
    pub avg_latency_ms: Option<f64>,
    pub last_status: String,
    pub last_checked_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchClustersRequest {
    pub cluster_names: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchOperationResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BatchResultItem>,
}

#[derive(Debug, Serialize)]
pub struct BatchResultItem {
    pub cluster_name: String,
    pub success: bool,
    pub message: Option<String>,
}

/// 获取所有集群连接状态
async fn get_all_connections_status(
    State(state): State<AppState>,
) -> Result<Json<AllConnectionsStatusResponse>> {
    let statuses = state.pools.get_all_connections_status().await;

    let connections: Vec<ConnectionStatusResponse> = statuses
        .into_iter()
        .map(|(cluster_id, status)| {
            let (status_str, error_message) = match status {
                ConnectionStatus::Connected => ("connected".to_string(), None),
                ConnectionStatus::Disconnected => ("disconnected".to_string(), None),
                ConnectionStatus::Error(msg) => ("error".to_string(), Some(msg)),
            };

            ConnectionStatusResponse {
                cluster_id,
                status: status_str,
                error_message,
            }
        })
        .collect();

    Ok(Json(AllConnectionsStatusResponse { connections }))
}

/// 获取指定集群连接状态
async fn get_connection_status(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ConnectionStatusResponse>> {
    let status = state.pools.check_connection(&cluster_id).await;

    match status {
        Some(conn_status) => {
            let (status_str, error_message) = match conn_status {
                ConnectionStatus::Connected => ("connected".to_string(), None),
                ConnectionStatus::Disconnected => ("disconnected".to_string(), None),
                ConnectionStatus::Error(msg) => ("error".to_string(), Some(msg)),
            };

            Ok(Json(ConnectionStatusResponse {
                cluster_id,
                status: status_str,
                error_message,
            }))
        }
        None => {
            // 连接池中不存在，尝试从数据库获取集群配置
            let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_id)
                .await?
                .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

            let config = crate::config::KafkaConfig {
                brokers: cluster.brokers,
                request_timeout_ms: cluster.request_timeout_ms as u32,
                operation_timeout_ms: cluster.operation_timeout_ms as u32,
            };

            // 临时建立连接进行状态检查
            match state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await {
                Ok(_) => {
                    let new_status = state.pools.check_connection(&cluster_id).await;
                    let (status_str, error_message) = match new_status {
                        Some(conn_status) => match conn_status {
                            ConnectionStatus::Connected => ("connected".to_string(), None),
                            ConnectionStatus::Disconnected => ("disconnected".to_string(), None),
                            ConnectionStatus::Error(msg) => ("error".to_string(), Some(msg)),
                        },
                        None => ("error".to_string(), Some("Failed to check connection after adding cluster".to_string())),
                    };

                    Ok(Json(ConnectionStatusResponse {
                        cluster_id,
                        status: status_str,
                        error_message,
                    }))
                }
                Err(e) => Ok(Json(ConnectionStatusResponse {
                    cluster_id,
                    status: "error".to_string(),
                    error_message: Some(format!("Failed to add cluster: {}", e)),
                })),
            }
        }
    }
}

/// 断开集群连接
async fn disconnect_cluster(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<DisconnectResponse>> {
    // 从连接池断开
    state.pools.disconnect(&cluster_id).await?;

    // 从 Kafka 客户端移除（使用 ArcSwap 无锁更新）
    let current_clients = state.get_clients();
    let new_clients = current_clients.without_cluster(&cluster_id);
    state.set_clients(new_clients);

    tracing::info!("Disconnected cluster: {}", cluster_id);

    Ok(Json(DisconnectResponse {
        success: true,
        message: format!("Cluster '{}' disconnected successfully", cluster_id),
    }))
}

/// 重连集群
async fn reconnect_cluster(
    State(state): State<AppState>,
    Path(cluster_name): Path<String>,
) -> Result<Json<ReconnectResponse>> {
    // 从数据库获取集群配置
    let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_name)
        .await?
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_name)))?;

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    // 重连连接池
    state.pools.reconnect(&cluster.name, &config, &state.config.pool).await?;

    // 更新 Kafka 客户端（使用 ArcSwap 无锁更新）
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(&cluster.name, &config)?;
    state.set_clients(new_clients.into());

    tracing::info!("Reconnected cluster: {}", cluster_name);

    Ok(Json(ReconnectResponse {
        success: true,
        message: format!("Cluster '{}' reconnected successfully", cluster_name),
    }))
}

/// 健康检查
async fn health_check_cluster(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<HealthCheckResponse>> {
    // 首先检查连接池中是否存在
    let status = state.pools.check_connection(&cluster_id).await;

    match status {
        Some(conn_status) => {
            let (healthy, status_str, error_message) = match conn_status {
                ConnectionStatus::Connected => (true, "healthy".to_string(), None),
                ConnectionStatus::Disconnected => (false, "disconnected".to_string(), None),
                ConnectionStatus::Error(msg) => (false, "error".to_string(), Some(msg)),
            };

            Ok(Json(HealthCheckResponse {
                cluster_id,
                healthy,
                status: status_str,
                error_message,
            }))
        }
        None => {
            // 连接池中不存在，尝试从数据库获取集群配置并建立临时连接
            let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_id)
                .await?
                .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

            let config = crate::config::KafkaConfig {
                brokers: cluster.brokers,
                request_timeout_ms: cluster.request_timeout_ms as u32,
                operation_timeout_ms: cluster.operation_timeout_ms as u32,
            };

            // 临时建立连接进行健康检查
            match state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await {
                Ok(_) => {
                    // 连接建立后再次检查
                    let new_status = state.pools.check_connection(&cluster_id).await;
                    let (healthy, status_str, error_message) = match new_status {
                        Some(conn_status) => match conn_status {
                            ConnectionStatus::Connected => (true, "healthy".to_string(), None),
                            ConnectionStatus::Disconnected => (false, "disconnected".to_string(), None),
                            ConnectionStatus::Error(msg) => (false, "error".to_string(), Some(msg)),
                        },
                        None => (false, "error".to_string(), Some("Failed to check connection after adding cluster".to_string())),
                    };

                    Ok(Json(HealthCheckResponse {
                        cluster_id,
                        healthy,
                        status: status_str,
                        error_message,
                    }))
                }
                Err(e) => Ok(Json(HealthCheckResponse {
                    cluster_id,
                    healthy: false,
                    status: "error".to_string(),
                    error_message: Some(format!("Failed to add cluster: {}", e)),
                })),
            }
        }
    }
}

/// 获取连接池指标
async fn get_connection_metrics(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ConnectionMetricsResponse>> {
    // 获取连接池状态
    let consumer_pool_status = state.pools.get_consumer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));
    let producer_pool_status = state.pools.get_producer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));

    // 如果连接池中不存在，尝试从数据库获取集群配置并建立临时连接
    if consumer_pool_status.is_none() && producer_pool_status.is_none() {
        let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_id)
            .await?
            .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

        let config = crate::config::KafkaConfig {
            brokers: cluster.brokers,
            request_timeout_ms: cluster.request_timeout_ms as u32,
            operation_timeout_ms: cluster.operation_timeout_ms as u32,
        };

        // 临时建立连接
        let _ = state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await;
    }

    // 重新获取连接池状态
    let consumer_pool_status = state.pools.get_consumer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));
    let producer_pool_status = state.pools.get_producer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));

    let (consumer_pool_size, consumer_pool_available) = consumer_pool_status.unwrap_or((0, 0));
    let (producer_pool_size, producer_pool_available) = producer_pool_status.unwrap_or((0, 0));

    Ok(Json(ConnectionMetricsResponse {
        cluster_id,
        consumer_pool_size,
        producer_pool_size,
        consumer_pool_available,
        producer_pool_available,
    }))
}

/// 获取连接历史
async fn get_connection_history(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<ConnectionHistoryResponse>> {
    let history = ClusterConnectionStore::get_history(state.db.inner(), &cluster_id, query.limit)
        .await?;

    let history_entries: Vec<HistoryEntry> = history
        .into_iter()
        .map(|h| HistoryEntry {
            status: h.status,
            error_message: h.error_message,
            latency_ms: h.latency_ms,
            checked_at: h.checked_at,
        })
        .collect();

    Ok(Json(ConnectionHistoryResponse {
        cluster_id,
        history: history_entries,
    }))
}

/// 获取连接统计
async fn get_connection_stats(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ConnectionStatsResponse>> {
    let stats = ClusterConnectionStore::get_stats(state.db.inner(), &cluster_id)
        .await?;

    match stats {
        Some(s) => {
            let success_rate = if s.total_checks > 0 {
                (s.successful_checks as f64 / s.total_checks as f64) * 100.0
            } else {
                0.0
            };

            Ok(Json(ConnectionStatsResponse {
                cluster_id,
                total_checks: s.total_checks,
                successful_checks: s.successful_checks,
                failed_checks: s.failed_checks,
                success_rate,
                avg_latency_ms: s.avg_latency_ms,
                last_status: s.last_status,
                last_checked_at: s.last_checked_at,
            }))
        }
        None => Ok(Json(ConnectionStatsResponse {
            cluster_id,
            total_checks: 0,
            successful_checks: 0,
            failed_checks: 0,
            success_rate: 0.0,
            avg_latency_ms: None,
            last_status: "unknown".to_string(),
            last_checked_at: None,
        })),
    }
}

/// 批量断开集群
async fn batch_disconnect(
    State(state): State<AppState>,
    Json(req): Json<BatchClustersRequest>,
) -> Result<Json<BatchOperationResponse>> {
    let mut results = Vec::new();
    let mut successful = 0;

    for cluster_name in &req.cluster_names {
        match state.pools.disconnect(cluster_name).await {
            Ok(_) => {
                // 从 Kafka 客户端移除（使用 ArcSwap 无锁更新）
                let current_clients = state.get_clients();
                let new_clients = current_clients.without_cluster(cluster_name);
                state.set_clients(new_clients.into());

                results.push(BatchResultItem {
                    cluster_name: cluster_name.clone(),
                    success: true,
                    message: Some("Disconnected successfully".to_string()),
                });
                successful += 1;
            }
            Err(e) => {
                results.push(BatchResultItem {
                    cluster_name: cluster_name.clone(),
                    success: false,
                    message: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(BatchOperationResponse {
        total: req.cluster_names.len(),
        successful,
        failed: req.cluster_names.len() - successful,
        results,
    }))
}

/// 批量重连集群
async fn batch_reconnect(
    State(state): State<AppState>,
    Json(req): Json<BatchClustersRequest>,
) -> Result<Json<BatchOperationResponse>> {
    use futures::future::join_all;

    let tasks: Vec<_> = req.cluster_names.iter().map(|cluster_name| {
        let state = state.clone();
        let cluster_name = cluster_name.clone();
        async move {
            // 从数据库获取集群配置
            match ClusterStore::get_by_name(state.db.inner(), &cluster_name).await {
                Ok(Some(cluster)) => {
                    let config = crate::config::KafkaConfig {
                        brokers: cluster.brokers,
                        request_timeout_ms: cluster.request_timeout_ms as u32,
                        operation_timeout_ms: cluster.operation_timeout_ms as u32,
                    };

                    // 重连连接池
                    match state.pools.reconnect(&cluster_name, &config, &state.config.pool).await {
                        Ok(_) => {
                            // 更新 Kafka 客户端（使用 ArcSwap 无锁更新）
                            let current_clients = state.get_clients();
                            match current_clients.with_added_cluster(&cluster_name, &config) {
                                Ok(new_clients) => {
                                    state.set_clients(new_clients.into());
                                    BatchResultItem {
                                        cluster_name: cluster_name.clone(),
                                        success: true,
                                        message: Some("Reconnected successfully".to_string()),
                                    }
                                }
                                Err(e) => BatchResultItem {
                                    cluster_name: cluster_name.clone(),
                                    success: false,
                                    message: Some(format!("Failed to update client: {}", e)),
                                }
                            }
                        }
                        Err(e) => BatchResultItem {
                            cluster_name: cluster_name.clone(),
                            success: false,
                            message: Some(format!("Reconnect failed: {}", e)),
                        }
                    }
                }
                Ok(None) => BatchResultItem {
                    cluster_name: cluster_name.clone(),
                    success: false,
                    message: Some("Cluster is not connected".to_string()),
                },
                Err(e) => BatchResultItem {
                    cluster_name: cluster_name.clone(),
                    success: false,
                    message: Some(format!("Database error: {}", e)),
                },
            }
        }
    }).collect();

    let results = join_all(tasks).await;
    let successful = results.iter().filter(|r| r.success).count();

    Ok(Json(BatchOperationResponse {
        total: req.cluster_names.len(),
        successful,
        failed: req.cluster_names.len() - successful,
        results,
    }))
}
