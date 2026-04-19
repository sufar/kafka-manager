/// Connection handlers for unified API
use crate::db::cluster::ClusterStore;
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;

pub async fn handle_connection_list(state: AppState) -> Result<Value> {
    let statuses = state.pools.get_all_connections_status().await;

    let connections: Vec<Value> = statuses.into_iter().map(|(cluster_id, status)| {
        let (status_str, error_message) = match status {
            crate::pool::ConnectionStatus::Connected => ("connected".to_string(), None::<String>),
            crate::pool::ConnectionStatus::Disconnected => ("disconnected".to_string(), None),
            crate::pool::ConnectionStatus::Error(msg) => ("error".to_string(), Some(msg)),
        };
        serde_json::json!({
            "cluster_id": cluster_id, "status": status_str, "error_message": error_message,
        })
    }).collect();

    Ok(serde_json::json!({ "connections": connections }))
}

pub async fn handle_connection_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let status = state.pools.check_connection(&cluster_id).await;

    match status {
        Some(conn_status) => {
            let (status_str, error_message) = match conn_status {
                crate::pool::ConnectionStatus::Connected => ("connected".to_string(), None::<String>),
                crate::pool::ConnectionStatus::Disconnected => ("disconnected".to_string(), None),
                crate::pool::ConnectionStatus::Error(msg) => ("error".to_string(), Some(msg)),
            };
            Ok(serde_json::json!({
                "cluster_id": cluster_id, "status": status_str, "error_message": error_message,
            }))
        }
        None => Err(AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id))),
    }
}

pub async fn handle_connection_disconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    state.pools.disconnect(&cluster_id).await?;
    let current_clients = state.get_clients();
    let new_clients = current_clients.without_cluster(&cluster_id);
    state.set_clients(new_clients.into());
    tracing::info!("Disconnected cluster: {}", cluster_id);
    Ok(serde_json::json!({
        "success": true,
        "message": format!("Cluster '{}' disconnected successfully", cluster_id),
    }))
}

pub async fn handle_connection_reconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_name = super::get_string_param(&body, "cluster_name")?;
    let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_name)
        .await?.ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_name)))?;
    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };
    state.pools.reconnect(&cluster_name, &config, &state.config.pool).await?;
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(&cluster_name, &config)?;
    state.set_clients(new_clients.into());
    tracing::info!("Reconnected cluster: {}", cluster_name);
    Ok(serde_json::json!({
        "success": true,
        "message": format!("Cluster '{}' reconnected successfully", cluster_name),
    }))
}

pub async fn handle_connection_health_check(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let statuses = state.pools.get_all_connections_status().await;

    match statuses.into_iter().find(|(id, _)| id == &cluster_id) {
        Some((_, conn_status)) => {
            let (healthy, status_str, error_message) = match conn_status {
                crate::pool::ConnectionStatus::Connected => (true, "connected".to_string(), None::<String>),
                crate::pool::ConnectionStatus::Disconnected => (false, "disconnected".to_string(), None),
                crate::pool::ConnectionStatus::Error(msg) => (false, "error".to_string(), Some(msg)),
            };
            Ok(serde_json::json!({
                "cluster_id": cluster_id, "healthy": healthy, "status": status_str, "error_message": error_message,
            }))
        }
        None => {
            let max_retries = 2;
            let mut last_error: Option<String> = None;
            for attempt in 1..=max_retries {
                tracing::warn!("[HealthCheck] Cluster '{}' not found in connection pool, attempting reconnect {}/{}", cluster_id, attempt, max_retries);
                match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
                    Ok(Some(cluster)) => {
                        let config = crate::config::KafkaConfig {
                            brokers: cluster.brokers,
                            request_timeout_ms: cluster.request_timeout_ms as u32,
                            operation_timeout_ms: cluster.operation_timeout_ms as u32,
                        };
                        match state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await {
                            Ok(_) => {
                                tracing::info!("[HealthCheck] Cluster '{}' reconnected successfully on attempt {}", cluster_id, attempt);
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                return Ok(serde_json::json!({
                                    "cluster_id": cluster_id,
                                    "healthy": true,
                                    "status": "reconnected",
                                    "error_message": null,
                                    "reconnected": true,
                                    "attempt": attempt,
                                }));
                            }
                            Err(e) => {
                                tracing::warn!("[HealthCheck] Failed to reconnect cluster '{}' on attempt {}: {}", cluster_id, attempt, e);
                                last_error = Some(format!("Reconnect attempt {} failed: {}", attempt, e));
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::error!("[HealthCheck] Cluster '{}' not found in database", cluster_id);
                        last_error = Some(format!("Cluster '{}' not found in database", cluster_id));
                        break;
                    }
                    Err(e) => {
                        tracing::error!("[HealthCheck] Failed to get cluster '{}' from database: {}", cluster_id, e);
                        last_error = Some(format!("Failed to get cluster config: {}", e));
                    }
                }
                if attempt < max_retries {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }
            }
            Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "healthy": false,
                "status": "not_found",
                "error_message": last_error.or_else(|| Some("Cluster not found in connection pool".to_string())),
                "reconnect_attempts": max_retries,
            }))
        }
    }
}

pub async fn handle_connection_metrics(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;

    let consumer_pool_status = state.pools.get_consumer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));
    let producer_pool_status = state.pools.get_producer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));

    if consumer_pool_status.is_none() && producer_pool_status.is_none() {
        let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_id)
            .await?
            .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
        let config = crate::config::KafkaConfig {
            brokers: cluster.brokers,
            request_timeout_ms: cluster.request_timeout_ms as u32,
            operation_timeout_ms: cluster.operation_timeout_ms as u32,
        };
        let _ = state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await;
    }

    let consumer_pool_status = state.pools.get_consumer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));
    let producer_pool_status = state.pools.get_producer_pool(&cluster_id).await
        .map(|pool| (pool.status().size, pool.status().available));

    let (consumer_pool_size, consumer_pool_available) = consumer_pool_status.unwrap_or((0, 0));
    let (producer_pool_size, producer_pool_available) = producer_pool_status.unwrap_or((0, 0));

    Ok(serde_json::json!({
        "cluster_id": cluster_id,
        "consumer_pool_size": consumer_pool_size,
        "producer_pool_size": producer_pool_size,
        "consumer_pool_available": consumer_pool_available,
        "producer_pool_available": producer_pool_available,
    }))
}

pub async fn handle_connection_batch_disconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_names = super::get_string_array_param(&body, "cluster_names");
    let mut results = Vec::with_capacity(cluster_names.len());
    let mut successful = 0;

    for cluster_name in &cluster_names {
        match state.pools.disconnect(cluster_name).await {
            Ok(_) => {
                let current_clients = state.get_clients();
                let new_clients = current_clients.without_cluster(cluster_name);
                state.set_clients(new_clients.into());
                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": true,
                    "message": "Disconnected successfully",
                }));
                successful += 1;
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": false,
                    "message": e.to_string(),
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "total": cluster_names.len(),
        "successful": successful,
        "failed": cluster_names.len() - successful,
        "results": results,
    }))
}

pub async fn handle_connection_batch_reconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_names = super::get_string_array_param(&body, "cluster_names");
    let mut results = Vec::with_capacity(cluster_names.len());
    let mut successful = 0;

    for cluster_name in &cluster_names {
        match ClusterStore::get_by_name(state.db.inner(), cluster_name).await {
            Ok(Some(cluster)) => {
                let config = crate::config::KafkaConfig {
                    brokers: cluster.brokers.clone(),
                    request_timeout_ms: cluster.request_timeout_ms as u32,
                    operation_timeout_ms: cluster.operation_timeout_ms as u32,
                };
                match state.pools.reconnect(cluster_name, &config, &state.config.pool).await {
                    Ok(_) => {
                        let current_clients = state.get_clients();
                        match current_clients.with_added_cluster(cluster_name, &config) {
                            Ok(new_clients) => {
                                state.set_clients(new_clients.into());
                                results.push(serde_json::json!({
                                    "cluster_name": cluster_name,
                                    "success": true,
                                    "message": "Reconnected successfully",
                                }));
                                successful += 1;
                            }
                            Err(e) => {
                                results.push(serde_json::json!({
                                    "cluster_name": cluster_name,
                                    "success": false,
                                    "message": format!("Failed to update client: {}", e),
                                }));
                            }
                        }
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "cluster_name": cluster_name,
                            "success": false,
                            "message": e.to_string(),
                        }));
                    }
                }
            }
            Ok(None) => {
                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": false,
                    "message": "Cluster not found in database",
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": false,
                    "message": format!("Failed to get cluster config: {}", e),
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "total": cluster_names.len(),
        "successful": successful,
        "failed": cluster_names.len() - successful,
        "results": results,
    }))
}
