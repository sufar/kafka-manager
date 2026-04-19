/// Unified API - thin router with business logic in sub-modules
mod cluster;
mod cluster_group;
mod connection;
mod consumer_group;
mod favorite;
mod health;
mod highlight;
mod message;
mod settings;
mod topic;
mod topic_history;
mod topic_template;

use crate::db::cluster::ClusterStore;
use crate::error::{AppError, Result};
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;

// Re-export handler functions from sub-modules
pub use self::cluster::*;
pub use self::cluster_group::*;
pub use self::connection::*;
pub use self::consumer_group::*;
pub use self::favorite::*;
pub use self::health::*;
pub use self::highlight::*;
pub use self::message::*;
pub use self::settings::*;
pub use self::topic::*;
pub use self::topic_history::*;
pub use self::topic_template::*;

// 创建统一路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handle_unified_request))
        .route("/stream", post(handle_stream_request))
}

// ==================== Helper Functions ====================

fn get_string_param(body: &Value, key: &str) -> Result<String> {
    body.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Err(AppError::BadRequest(format!("Parameter '{}' cannot be empty", key)))
            } else if trimmed.len() > 256 {
                Err(AppError::BadRequest(format!("Parameter '{}' exceeds maximum length of 256 characters", key)))
            } else {
                Ok(trimmed.to_string())
            }
        })
}

fn get_long_string_param(body: &Value, key: &str) -> Result<String> {
    body.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Err(AppError::BadRequest(format!("Parameter '{}' cannot be empty", key)))
            } else {
                Ok(trimmed.to_string())
            }
        })
}

fn get_cluster_id_param(body: &Value) -> Result<String> {
    let cluster_id = get_string_param(body, "cluster_id")?;
    if !cluster_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::BadRequest("Cluster ID can only contain letters, numbers, hyphens, and underscores".to_string()));
    }
    Ok(cluster_id)
}

fn get_topic_name_param(body: &Value) -> Result<String> {
    let topic_name = get_string_param(body, "topic_name")?;
    if topic_name.contains(' ') || topic_name.contains('"') || topic_name.contains(',') {
        return Err(AppError::BadRequest("Topic name cannot contain spaces, quotes, or commas".to_string()));
    }
    Ok(topic_name)
}

fn get_i64_param(body: &Value, key: &str) -> Result<i64> {
    body.get(key).and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
}

fn get_i32_param(body: &Value, key: &str) -> Result<i32> {
    body.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
}

fn get_optional_string_param(body: &Value, key: &str) -> Option<String> {
    body.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_optional_i64_param(body: &Value, key: &str) -> Option<i64> {
    body.get(key).and_then(|v| v.as_i64())
}

fn get_nullable_i64_param(body: &Value, key: &str) -> Option<Option<i64>> {
    match body.get(key) {
        Some(v) if v.is_null() => Some(None),
        Some(v) => v.as_i64().map(Some),
        None => None,
    }
}

fn get_optional_i32_param(body: &Value, key: &str) -> Option<i32> {
    body.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

#[allow(dead_code)]
fn get_bool_param(body: &Value, key: &str) -> Result<bool> {
    body.get(key).and_then(|v| v.as_bool())
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
}

fn get_optional_bool_param(body: &Value, key: &str) -> Option<bool> {
    body.get(key).and_then(|v| v.as_bool())
}

fn get_hashmap_param(body: &Value, key: &str) -> std::collections::HashMap<String, String> {
    body.get(key).and_then(|v| v.as_object())
        .map(|obj| obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
        .unwrap_or_default()
}

fn get_string_array_param(body: &Value, key: &str) -> Vec<String> {
    body.get(key).and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
}

async fn ensure_cluster_client(state: &AppState, cluster_id: &str) -> Result<Arc<crate::config::KafkaConfig>> {
    use tokio::time::Duration;

    let clients = state.get_clients();
    if let Some(config) = clients.get_config(cluster_id) {
        return Ok(config);
    }

    let mut cluster = None;
    let mut last_db_error = None;
    for attempt in 0..3 {
        match ClusterStore::get_by_name(state.db.inner(), cluster_id).await {
            Ok(Some(c)) => { cluster = Some(c); break; }
            Ok(None) => {
                last_db_error = Some(AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)));
                if attempt < 2 { tokio::time::sleep(Duration::from_millis(50)).await; }
            }
            Err(e) => {
                last_db_error = Some(e);
                if attempt < 2 { tokio::time::sleep(Duration::from_millis(50)).await; }
            }
        }
    }

    let cluster = cluster.ok_or_else(|| last_db_error.unwrap_or_else(|| {
        AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id))
    }))?;

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    state.pools.add_cluster(cluster_id, &config, &state.config.pool).await?;
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(cluster_id, &config)?;
    state.set_clients(new_clients.into());
    tokio::time::sleep(Duration::from_millis(200)).await;

    state.get_clients().get_config(cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Failed to get config for cluster '{}'", cluster_id)))
}

async fn get_or_create_admin_client(state: &AppState, cluster_id: &str) -> Result<Arc<crate::kafka::KafkaAdmin>> {
    use tokio::time::Duration;

    let clients = state.get_clients();
    if let Some(admin) = clients.get_admin(cluster_id) {
        return Ok(admin);
    }

    let mut cluster = None;
    let mut last_db_error = None;
    for attempt in 0..3 {
        match ClusterStore::get_by_name(state.db.inner(), cluster_id).await {
            Ok(Some(c)) => { cluster = Some(c); break; }
            Ok(None) => {
                last_db_error = Some(AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)));
                if attempt < 2 { tokio::time::sleep(Duration::from_millis(50)).await; }
            }
            Err(e) => {
                last_db_error = Some(e);
                if attempt < 2 { tokio::time::sleep(Duration::from_millis(50)).await; }
            }
        }
    }

    let cluster = cluster.ok_or_else(|| last_db_error.unwrap_or_else(|| {
        AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id))
    }))?;

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    state.pools.add_cluster(cluster_id, &config, &state.config.pool).await?;
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(cluster_id, &config)?;
    state.set_clients(new_clients.into());

    let updated_clients = state.get_clients();
    match updated_clients.get_admin(cluster_id) {
        Some(admin) => Ok(admin),
        None => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            state.get_clients().get_admin(cluster_id)
                .ok_or_else(|| AppError::NotFound(format!("Failed to get admin client for cluster '{}'", cluster_id)))
        }
    }
}

#[allow(dead_code)]
async fn _get_or_create_admin_client_and_config(
    state: &AppState, cluster_id: &str,
) -> Result<(Arc<crate::kafka::KafkaAdmin>, Arc<crate::config::KafkaConfig>)> {
    use std::sync::Arc;
    let clients = state.get_clients();
    if let (Some(admin), Some(config)) = (clients.get_admin(cluster_id), clients.get_config(cluster_id)) {
        return Ok((admin, config));
    }
    let cluster = ClusterStore::get_by_name(state.db.inner(), cluster_id).await?
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let config = Arc::new(crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    });
    state.pools.add_cluster(cluster_id, &config, &state.config.pool).await?;
    let admin = state.get_clients().get_admin(cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Failed to get admin client for cluster '{}'", cluster_id)))?;
    Ok((admin, config))
}

// ==================== Request Dispatch ====================

async fn handle_unified_request(
    State(state): State<AppState>, headers: HeaderMap, Json(body): Json<Value>,
) -> Result<Response> {
    let method = headers.get("X-API-Method").and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing X-API-Method header".to_string()))?;
    let result = dispatch_request(method, state, body).await;
    match result {
        Ok(data) => Ok((StatusCode::OK, Json(serde_json::json!({"success": true, "data": data}))).into_response()),
        Err(e) => {
            let (status, error_msg) = match &e {
                AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
                AppError::Kafka(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Kafka error: {}", err)),
                AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            };
            Ok((status, Json(serde_json::json!({"success": false, "error": error_msg}))).into_response())
        }
    }
}

async fn handle_stream_request(
    State(state): State<AppState>, headers: HeaderMap, Json(body): Json<Value>,
) -> Result<Response> {
    let method = headers.get("X-API-Method").and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing X-API-Method header".to_string()))?;
    match method {
        "message.list" => handle_message_list_stream(state, body).await,
        _ => Err(AppError::BadRequest(format!("Method '{}' does not support streaming", method))),
    }
}

async fn dispatch_request(method: &str, state: AppState, body: Value) -> Result<Value> {
    match method {
        "health" => handle_health().await,
        "cluster.list" => handle_cluster_list(state).await,
        "cluster.get" => handle_cluster_get(state, body).await,
        "cluster.create" => handle_cluster_create(state, body).await,
        "cluster.update" => handle_cluster_update(state, body).await,
        "cluster.delete" => handle_cluster_delete(state, body).await,
        "cluster.test" => handle_cluster_test(state, body).await,
        "cluster.test_config" => handle_cluster_test_with_config(state, body).await,
        "cluster.stats" => handle_cluster_stats(state, body).await,
        "cluster_group.list" => handle_cluster_group_list(state).await,
        "cluster_group.get" => handle_cluster_group_get(state, body).await,
        "cluster_group.create" => handle_cluster_group_create(state, body).await,
        "cluster_group.update" => handle_cluster_group_update(state, body).await,
        "cluster_group.delete" => handle_cluster_group_delete(state, body).await,
        "cluster_group.clusters" => handle_cluster_group_clusters(state, body).await,
        "cluster_group.assign_cluster" => handle_cluster_group_assign_cluster(state, body).await,
        "topic.list" => handle_topic_list(state, body).await,
        "topic.list_with_cluster" => handle_topic_list_with_cluster(state, body).await,
        "topic.get" => handle_topic_get(state, body).await,
        "topic.create" => handle_topic_create(state, body).await,
        "topic.delete" => handle_topic_delete(state, body).await,
        "topic.batch_create" => handle_topic_batch_create(state, body).await,
        "topic.batch_delete" => handle_topic_batch_delete(state, body).await,
        "topic.delete_all" => handle_topic_delete_all(state, body).await,
        "topic.offsets" => handle_topic_offsets(state, body).await,
        "topic.config_get" => handle_topic_config_get(state, body).await,
        "topic.config_alter" => handle_topic_config_alter(state, body).await,
        "topic.partitions_add" => handle_topic_partitions_add(state, body).await,
        "topic.partition.watermarks" => handle_topic_partition_watermarks(state, body).await,
        "topic.throughput" => handle_topic_throughput(state, body).await,
        "topic.refresh" => handle_topic_refresh(state, body).await,
        "topic.saved" => handle_topic_saved(state, body).await,
        "topic.search" => handle_topic_search(state, body).await,
        "topic.count" => handle_topic_count(state, body).await,
        "topic.cleanup_orphans" => handle_topic_cleanup_orphans(state, body).await,
        "message.list" => handle_message_list(state, body).await,
        "message.send" => handle_message_send(state, body).await,
        "message.export" => handle_message_export(state, body).await,
        "connection.list" => handle_connection_list(state).await,
        "connection.get" => handle_connection_get(state, body).await,
        "connection.disconnect" => handle_connection_disconnect(state, body).await,
        "connection.reconnect" => handle_connection_reconnect(state, body).await,
        "connection.health_check" => handle_connection_health_check(state, body).await,
        "connection.metrics" => handle_connection_metrics(state, body).await,
        "connection.batch_disconnect" => handle_connection_batch_disconnect(state, body).await,
        "connection.batch_reconnect" => handle_connection_batch_reconnect(state, body).await,
        "settings.get" => handle_settings_get(state, body).await,
        "settings.update" => handle_settings_update(state, body).await,
        "settings.export" => handle_settings_export(state).await,
        "settings.import" => handle_settings_import(state, body).await,
        "app.version" => handle_app_version().await,
        "app.logs" => handle_app_logs().await,
        "app.logs.clear" => handle_app_logs_clear().await,
        "json_highlight.list" => handle_json_highlight_list(state).await,
        "json_highlight.get_current" => handle_json_highlight_get_current(state).await,
        "json_highlight.set_current" => handle_json_highlight_set_current(state, body).await,
        "json_highlight.create" => handle_json_highlight_create(state, body).await,
        "json_highlight.update" => handle_json_highlight_update(state, body).await,
        "json_highlight.delete" => handle_json_highlight_delete(state, body).await,
        "template.list" => handle_template_list(state).await,
        "template.get" => handle_template_get(state, body).await,
        "template.create" => handle_template_create(state, body).await,
        "template.update" => handle_template_update(state, body).await,
        "template.delete" => handle_template_delete(state, body).await,
        "template.presets" => handle_template_presets().await,
        "template.create_topic" => handle_template_create_topic(state, body).await,
        "favorite.group.list" => handle_favorite_group_list(state).await,
        "favorite.group.create" => handle_favorite_group_create(state, body).await,
        "favorite.group.get" => handle_favorite_group_get(state, body).await,
        "favorite.group.update" => handle_favorite_group_update(state, body).await,
        "favorite.group.delete" => handle_favorite_group_delete(state, body).await,
        "favorite.list" => handle_favorite_list(state).await,
        "favorite.create" => handle_favorite_create(state, body).await,
        "favorite.get" => handle_favorite_get(state, body).await,
        "favorite.update" => handle_favorite_update(state, body).await,
        "favorite.delete" => handle_favorite_delete(state, body).await,
        "favorite.check" => handle_favorite_check(state, body).await,
        "favorite.delete_by_topic" => handle_favorite_delete_by_topic(state, body).await,
        "topic_history.list" => handle_topic_history_list(state, body).await,
        "topic_history.record" => handle_topic_history_record(state, body).await,
        "topic_history.delete" => handle_topic_history_delete(state, body).await,
        "topic_history.delete_by_topic" => handle_topic_history_delete_by_topic(state, body).await,
        "topic_history.clear" => handle_topic_history_clear(state).await,
        "sent_message.list" => handle_sent_message_list(state, body).await,
        "sent_message.record" => handle_sent_message_record(state, body).await,
        "sent_message.delete" => handle_sent_message_delete(state, body).await,
        "sent_message.clear" => handle_sent_message_clear(state).await,
        "consumer_group.list" => handle_consumer_group_list(state, body).await,
        "consumer_group.list_by_topic" => handle_consumer_group_list_by_topic(state, body).await,
        "consumer_group.get" => handle_consumer_group_get(state, body).await,
        "consumer_group.offsets" => handle_consumer_group_offsets(state, body).await,
        "consumer_group.refresh" => handle_consumer_group_refresh(state, body).await,
        "consumer_group.saved" => handle_consumer_group_saved(state, body).await,
        "consumer_group.reset_offset" => handle_consumer_group_reset_offset(state, body).await,
        "consumer_group.delete" => handle_consumer_group_delete(state, body).await,
        "schema_registry.config.get" => crate::routes::schema_registry::handle_config_get(state, body).await,
        "schema_registry.config.save" => crate::routes::schema_registry::handle_config_save(state, body).await,
        "schema_registry.config.delete" => crate::routes::schema_registry::handle_config_delete(state, body).await,
        "schema_registry.config.test" => crate::routes::schema_registry::handle_config_test(state, body).await,
        "schema_registry.subject.list" => crate::routes::schema_registry::handle_subject_list(state, body).await,
        "schema_registry.version.list" => crate::routes::schema_registry::handle_version_list(state, body).await,
        "schema_registry.get" => crate::routes::schema_registry::handle_schema_get(state, body).await,
        "schema_registry.get_latest" => crate::routes::schema_registry::handle_schema_get_latest(state, body).await,
        "schema_registry.register" => crate::routes::schema_registry::handle_schema_register(state, body).await,
        "schema_registry.compatibility.test" => crate::routes::schema_registry::handle_compatibility_test(state, body).await,
        "schema_registry.compatibility.get" => crate::routes::schema_registry::handle_compatibility_get(state, body).await,
        "schema_registry.compatibility.set" => crate::routes::schema_registry::handle_compatibility_set(state, body).await,
        "schema_registry.list" => crate::routes::schema_registry::handle_schema_list(state, body).await,
        "schema_registry.delete" => crate::routes::schema_registry::handle_schema_delete(state, body).await,
        _ => Err(AppError::BadRequest(format!("Unknown method: {}", method))),
    }
}
