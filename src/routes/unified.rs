/// 统一 API 路由处理器
/// 所有 API 请求都通过 POST /api 发送，method 放在 header 中，参数放在 body 中

use crate::db::api_key::ApiKeyStore;
use crate::db::audit_log::AuditLogStore;
use crate::db::cluster::{ClusterStore, CreateClusterRequest, UpdateClusterRequest};
use crate::db::cluster_connection::ClusterConnectionStore;
use crate::db::notification::{CreateNotificationConfigRequest, NotificationStore};
use crate::db::settings::SettingStore;
use crate::db::tag::{TagStore};
use crate::db::topic::TopicStore;
use crate::db::topic_template::{
    CreateTopicTemplateRequest, TopicTemplateStore,
    UpdateTopicTemplateRequest,
};
use crate::db::user::{RoleStore, UserStore};
use crate::error::{AppError, Result};
use crate::kafka::offset::KafkaOffsetManager;
use crate::kafka::schema::SchemaRegistryClient;
use crate::kafka::throughput::KafkaThroughputCalculator;
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// 创建统一路由
pub fn routes() -> Router<AppState> {
    Router::new().route("/", post(handle_unified_request))
}

// Helper functions for parameter extraction
fn get_string_param(body: &Value, key: &str) -> Result<String> {
    body.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
}

/// 确保集群客户端已创建（如果未创建则自动创建）
/// 如果内存中不存在，则从数据库加载集群配置并建立连接
async fn ensure_cluster_client(
    state: &AppState,
    cluster_id: &str,
) -> Result<Arc<crate::config::KafkaConfig>> {
    use tokio::time::Duration;

    tracing::info!("ensure_cluster_client called for cluster: {}", cluster_id);

    // 首先尝试从内存中获取配置
    let clients = state.get_clients();
    if let Some(config) = clients.get_config(cluster_id) {
        tracing::info!("Found config in memory for cluster: {}", cluster_id);
        return Ok(config);
    }

    tracing::info!("Config not in memory, fetching from database for cluster: {}", cluster_id);

    // 从数据库获取集群配置（带重试）
    let mut cluster = None;
    let mut last_db_error = None;

    // 最多重试 3 次
    for attempt in 0..3 {
        tracing::info!("Attempting to fetch cluster '{}' from database (attempt {}/{})", cluster_id, attempt + 1, 3);

        match ClusterStore::get_by_name(state.db.inner(), cluster_id).await {
            Ok(Some(c)) => {
                tracing::info!("Successfully fetched cluster '{}' from database", cluster_id);
                cluster = Some(c);
                break;
            }
            Ok(None) => {
                tracing::warn!("Cluster '{}' not found in database (attempt {}), retrying...", cluster_id, attempt + 1);
                last_db_error = Some(AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)));
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                tracing::warn!("Database error fetching cluster '{}': {} (attempt {}), retrying...", cluster_id, e, attempt + 1);
                last_db_error = Some(e);
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
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

    tracing::info!("Creating connection pool for cluster: {}", cluster_id);

    // 建立连接池连接
    state.pools.add_cluster(cluster_id, &config, &state.config.pool).await?;

    // 更新 Kafka 客户端
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(cluster_id, &config)?;
    state.set_clients(new_clients.into());

    // 等待连接建立
    tokio::time::sleep(Duration::from_millis(200)).await;

    let updated_clients = state.get_clients();
    updated_clients.get_config(cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Failed to get config for cluster '{}'", cluster_id)))
}

/// 获取或创建 admin 客户端
/// 如果内存中不存在，则从数据库加载集群配置并建立连接
async fn get_or_create_admin_client(
    state: &AppState,
    cluster_id: &str,
) -> Result<Arc<crate::kafka::KafkaAdmin>> {
    use tokio::time::Duration;

    tracing::info!("get_or_create_admin_client called for cluster: {}", cluster_id);

    // 首先尝试从内存中获取
    let clients = state.get_clients();
    if let Some(admin) = clients.get_admin(cluster_id) {
        tracing::info!("Found admin client in memory for cluster: {}", cluster_id);
        return Ok(admin);
    }

    tracing::info!("Admin client not in memory, fetching from database for cluster: {}", cluster_id);

    // 从数据库获取集群配置（带重试）
    let mut cluster = None;
    let mut last_db_error = None;

    // 最多重试 3 次，因为数据库连接可能还没完全准备好
    for attempt in 0..3 {
        tracing::info!("Attempting to fetch cluster '{}' from database (attempt {}/{})", cluster_id, attempt + 1, 3);

        match ClusterStore::get_by_name(state.db.inner(), cluster_id).await {
            Ok(Some(c)) => {
                tracing::info!("Successfully fetched cluster '{}' from database", cluster_id);
                cluster = Some(c);
                break;
            }
            Ok(None) => {
                // 集群不存在，等待后重试
                tracing::warn!("Cluster '{}' not found in database (attempt {}), retrying...", cluster_id, attempt + 1);
                last_db_error = Some(AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)));
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                tracing::warn!("Database error fetching cluster '{}': {} (attempt {}), retrying...", cluster_id, e, attempt + 1);
                last_db_error = Some(e);
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
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

    tracing::info!("Creating connection pool for cluster: {}", cluster_id);

    // 建立连接池连接
    state.pools.add_cluster(cluster_id, &config, &state.config.pool).await?;

    // 更新 Kafka 客户端（使用 with_added_cluster 创建新的客户端集合）
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(cluster_id, &config)?;
    state.set_clients(new_clients.into());

    // 获取新的客户端
    let updated_clients = state.get_clients();

    // 如果获取 admin 失败，等待连接建立后重试
    match updated_clients.get_admin(cluster_id) {
        Some(admin) => {
            tracing::info!("Successfully created admin client for cluster: {}", cluster_id);
            Ok(admin)
        }
        None => {
            tracing::warn!("Admin client not found after creation, waiting and retrying for cluster: {}", cluster_id);
            // 等待连接完全建立
            tokio::time::sleep(Duration::from_millis(100)).await;
            let retry_clients = state.get_clients();
            retry_clients.get_admin(cluster_id)
                .ok_or_else(|| AppError::NotFound(format!("Failed to get admin client for cluster '{}'", cluster_id)))
        }
    }
}

/// 获取或创建 admin 客户端和配置
/// 如果内存中不存在，则从数据库加载集群配置并建立连接
async fn get_or_create_admin_client_and_config(
    state: &AppState,
    cluster_id: &str,
) -> Result<(Arc<crate::kafka::KafkaAdmin>, Arc<crate::config::KafkaConfig>)> {
    // 首先尝试从内存中获取
    let clients = state.get_clients();
    if let Some(admin) = clients.get_admin(cluster_id) {
        if let Some(config) = clients.get_config(cluster_id) {
            return Ok((admin, config));
        }
    }

    // 从数据库获取集群配置
    let cluster = ClusterStore::get_by_name(state.db.inner(), cluster_id)
        .await?
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };
    let config = Arc::new(config);

    // 建立连接池连接
    state.pools.add_cluster(cluster_id, &config, &state.config.pool).await?;

    // 获取新的客户端
    let new_clients = state.get_clients();
    let admin = new_clients.get_admin(cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Failed to get admin client for cluster '{}'", cluster_id)))?;

    Ok((admin, config))
}

fn get_i64_param(body: &Value, key: &str) -> Result<i64> {
    body.get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
}

fn get_i32_param(body: &Value, key: &str) -> Result<i32> {
    body.get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
}

fn get_optional_string_param(body: &Value, key: &str) -> Option<String> {
    body.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_optional_i64_param(body: &Value, key: &str) -> Option<i64> {
    body.get(key).and_then(|v| v.as_i64())
}

fn get_optional_i32_param(body: &Value, key: &str) -> Option<i32> {
    body.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

fn get_bool_param(body: &Value, key: &str) -> Result<bool> {
    body.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| AppError::BadRequest(format!("Missing or invalid parameter: {}", key)))
}

fn get_optional_bool_param(body: &Value, key: &str) -> Option<bool> {
    body.get(key).and_then(|v| v.as_bool())
}

fn get_hashmap_param(body: &Value, key: &str) -> HashMap<String, String> {
    body.get(key)
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn get_string_array_param(body: &Value, key: &str) -> Vec<String> {
    body.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

// Main request handler
async fn handle_unified_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    // Get method from header
    let method = headers
        .get("X-API-Method")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing X-API-Method header".to_string()))?;

    // Dispatch request
    let result = dispatch_request(method, state, body).await;

    match result {
        Ok(data) => {
            // 包装成前端期望的格式
            let response = serde_json::json!({
                "success": true,
                "data": data
            });
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Err(e) => {
            // 错误响应也包装成统一格式
            let (status, error_msg) = match &e {
                AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
                AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
                AppError::Kafka(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Kafka error: {}", err)),
                AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            };
            let response = serde_json::json!({
                "success": false,
                "error": error_msg
            });
            Ok((status, Json(response)).into_response())
        }
    }
}

// Dispatch function
async fn dispatch_request(method: &str, state: AppState, body: Value) -> Result<Value> {
    match method {
        // Health
        "health" => handle_health().await,

        // Cluster
        "cluster.list" => handle_cluster_list(state).await,
        "cluster.get" => handle_cluster_get(state, body).await,
        "cluster.create" => handle_cluster_create(state, body).await,
        "cluster.update" => handle_cluster_update(state, body).await,
        "cluster.delete" => handle_cluster_delete(state, body).await,
        "cluster.test" => handle_cluster_test(state, body).await,
        "cluster.stats" => handle_cluster_stats(state, body).await,

        // Topic
        "topic.list" => handle_topic_list(state, body).await,
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

        // Consumer Group
        "consumer_group.list" => handle_consumer_group_list(state, body).await,
        "consumer_group.get" => handle_consumer_group_get(state, body).await,
        "consumer_group.delete" => handle_consumer_group_delete(state, body).await,
        "consumer_group.offsets" => handle_consumer_group_offsets(state, body).await,
        "consumer_group.offsets_reset" => handle_consumer_group_offsets_reset(state, body).await,
        "consumer_group.throughput" => handle_consumer_group_throughput(state, body).await,
        "consumer_group.batch_delete" => handle_consumer_group_batch_delete(state, body).await,
        "consumer_group.consumer_offsets" => handle_consumer_group_consumer_offsets(state, body).await,

        // Consumer Lag
        "consumer_lag.get" => handle_consumer_lag_get(state, body).await,
        "consumer_lag.history" => handle_consumer_lag_history(state, body).await,

        // Message
        "message.list" => handle_message_list(state, body).await,
        "message.send" => handle_message_send(state, body).await,
        "message.export" => handle_message_export(state, body).await,

        // Cluster Connection
        "connection.list" => handle_connection_list(state).await,
        "connection.get" => handle_connection_get(state, body).await,
        "connection.disconnect" => handle_connection_disconnect(state, body).await,
        "connection.reconnect" => handle_connection_reconnect(state, body).await,
        "connection.health_check" => handle_connection_health_check(state, body).await,
        "connection.metrics" => handle_connection_metrics(state, body).await,
        "connection.history" => handle_connection_history(state, body).await,
        "connection.stats" => handle_connection_stats(state, body).await,
        "connection.batch_disconnect" => handle_connection_batch_disconnect(state, body).await,
        "connection.batch_reconnect" => handle_connection_batch_reconnect(state, body).await,

        // User
        "user.list" => handle_user_list(state).await,
        "user.get" => handle_user_get(state, body).await,
        "user.create" => handle_user_create(state, body).await,
        "user.update" => handle_user_update(state, body).await,
        "user.password_update" => handle_user_password_update(state, body).await,

        // Role
        "role.list" => handle_role_list(state).await,
        "role.get" => handle_role_get(state, body).await,
        "role.create" => handle_role_create(state, body).await,
        "role.update" => handle_role_update(state, body).await,

        // Notification
        "notification.list" => handle_notification_list(state).await,
        "notification.get" => handle_notification_get(state, body).await,
        "notification.create" => handle_notification_create(state, body).await,
        "notification.delete" => handle_notification_delete(state, body).await,
        "notification.enable" => handle_notification_enable(state, body).await,
        "notification.disable" => handle_notification_disable(state, body).await,

        // Alert History
        "alert_history.list" => handle_alert_history_list(state, body).await,

        // Schema Registry
        "schema.subjects" => handle_schema_subjects(body).await,
        "schema.versions" => handle_schema_versions(body).await,
        "schema.get" => handle_schema_get(body).await,
        "schema.register" => handle_schema_register(state, body).await,
        "schema.delete" => handle_schema_delete(body).await,
        "schema.version_delete" => handle_schema_version_delete(body).await,
        "schema.compatibility_level" => handle_schema_compatibility_level(body).await,

        // Settings
        "settings.get" => handle_settings_get(state, body).await,
        "settings.update" => handle_settings_update(state, body).await,

        // Cluster Stats/Monitor
        "monitor.stats" => handle_monitor_stats(state, body).await,
        "monitor.info" => handle_monitor_info(state, body).await,
        "monitor.metrics" => handle_monitor_metrics(state, body).await,
        "monitor.brokers" => handle_monitor_brokers(state, body).await,
        "monitor.broker_get" => handle_monitor_broker_get(state, body).await,

        // Topic Template
        "template.list" => handle_template_list(state).await,
        "template.get" => handle_template_get(state, body).await,
        "template.create" => handle_template_create(state, body).await,
        "template.update" => handle_template_update(state, body).await,
        "template.delete" => handle_template_delete(state, body).await,
        "template.presets" => handle_template_presets().await,
        "template.create_topic" => handle_template_create_topic(state, body).await,

        // Tag
        "tag.list" => handle_tag_list(state, body).await,
        "tag.create" => handle_tag_create(state, body).await,
        "tag.delete" => handle_tag_delete(state, body).await,
        "tag.topics" => handle_tag_topics(state, body).await,
        "tag.keys" => handle_tag_keys(state, body).await,
        "tag.values" => handle_tag_values(state, body).await,
        "tag.filter" => handle_tag_filter(state, body).await,
        "tag.batch_update" => handle_tag_batch_update(state, body).await,

        // Audit Log
        "audit_log.list" => handle_audit_log_list(state, body).await,

        // Auth
        "auth.api_keys" => handle_auth_api_keys(state).await,
        "auth.api_key_create" => handle_auth_api_key_create(state, body).await,
        "auth.api_key_revoke" => handle_auth_api_key_revoke(state, body).await,

        _ => Err(AppError::BadRequest(format!("Unknown method: {}", method))),
    }
}

// ==================== Health ====================

async fn handle_health() -> Result<Value> {
    Ok(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// ==================== Cluster ====================

async fn handle_cluster_list(state: AppState) -> Result<Value> {
    let clusters = ClusterStore::list(state.db.inner()).await?;

    let cluster_infos: Vec<Value> = clusters
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "brokers": c.brokers,
                "request_timeout_ms": c.request_timeout_ms,
                "operation_timeout_ms": c.operation_timeout_ms,
                "created_at": c.created_at,
                "updated_at": c.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!(cluster_infos))
}

async fn handle_cluster_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let cluster = ClusterStore::get(state.db.inner(), id).await?;

    Ok(serde_json::json!({
        "id": cluster.id,
        "name": cluster.name,
        "brokers": cluster.brokers,
        "request_timeout_ms": cluster.request_timeout_ms,
        "operation_timeout_ms": cluster.operation_timeout_ms,
        "created_at": cluster.created_at,
        "updated_at": cluster.updated_at,
    }))
}

async fn handle_cluster_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let brokers = get_string_param(&body, "brokers")?;
    let request_timeout_ms = get_optional_i64_param(&body, "request_timeout_ms").unwrap_or(5000);
    let operation_timeout_ms = get_optional_i64_param(&body, "operation_timeout_ms").unwrap_or(5000);

    // Check if name exists
    if let Some(_existing) = ClusterStore::get_by_name(state.db.inner(), &name).await? {
        return Err(AppError::BadRequest(format!(
            "Cluster name '{}' already exists",
            name
        )));
    }

    let req = CreateClusterRequest {
        name: name.clone(),
        brokers,
        request_timeout_ms,
        operation_timeout_ms,
    };

    let cluster = ClusterStore::create(state.db.inner(), &req).await?;

    // Reload Kafka clients
    reload_clients(&state).await?;

    Ok(serde_json::json!({
        "id": cluster.id,
        "name": cluster.name,
        "brokers": cluster.brokers,
        "request_timeout_ms": cluster.request_timeout_ms,
        "operation_timeout_ms": cluster.operation_timeout_ms,
        "created_at": cluster.created_at,
        "updated_at": cluster.updated_at,
    }))
}

async fn handle_cluster_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let name = get_optional_string_param(&body, "name");
    let brokers = get_optional_string_param(&body, "brokers");
    let request_timeout_ms = get_optional_i64_param(&body, "request_timeout_ms");
    let operation_timeout_ms = get_optional_i64_param(&body, "operation_timeout_ms");

    let old_cluster = ClusterStore::get(state.db.inner(), id).await?;

    // If name changed, check new name exists
    if let Some(ref new_name) = name {
        if new_name != &old_cluster.name {
            if let Some(_existing) = ClusterStore::get_by_name(state.db.inner(), new_name).await? {
                return Err(AppError::BadRequest(format!(
                    "Cluster name '{}' already exists",
                    new_name
                )));
            }
        }
    }

    let req = UpdateClusterRequest {
        name,
        brokers,
        request_timeout_ms,
        operation_timeout_ms,
    };

    let cluster = ClusterStore::update(state.db.inner(), id, &req).await?;

    // Reload Kafka clients
    reload_clients(&state).await?;

    Ok(serde_json::json!({
        "id": cluster.id,
        "name": cluster.name,
        "brokers": cluster.brokers,
        "request_timeout_ms": cluster.request_timeout_ms,
        "operation_timeout_ms": cluster.operation_timeout_ms,
        "created_at": cluster.created_at,
        "updated_at": cluster.updated_at,
    }))
}

async fn handle_cluster_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    ClusterStore::delete(state.db.inner(), id).await?;

    // Reload Kafka clients
    reload_clients(&state).await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_cluster_test(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let success = ClusterStore::test_connection(state.db.inner(), id).await?;

    Ok(serde_json::json!({ "success": success }))
}

async fn handle_cluster_stats(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // 使用 5 秒超时限制 Kafka 操作
    let timeout_duration = std::time::Duration::from_secs(5);

    // 在阻塞线程中执行所有 Kafka 操作
    let admin = admin.clone();
    let result = tokio::time::timeout(timeout_duration, tokio::task::spawn_blocking(move || -> Result<(crate::kafka::admin::ClusterInfo, Vec<String>, i32, i32, std::collections::HashMap<i32, i32>, std::collections::HashMap<i32, i32>)> {
        // 获取集群信息
        let cluster_info = admin.get_cluster_info()?;

        // 获取所有 topic 的分区信息
        let topics = admin.list_topics()?;
        let mut partition_count = 0;
        let mut under_replicated = 0;
        let mut broker_leader_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
        let mut broker_replica_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();

        for topic in &topics {
            let topic_info = admin.get_topic_info(topic)?;
            for partition in &topic_info.partitions {
                partition_count += 1;

                // 检查是否未完全复制
                if partition.isr.len() < partition.replicas.len() {
                    under_replicated += 1;
                }

                // 统计 leader 和 replica
                let leader = partition.leader;
                *broker_leader_counts.entry(leader).or_insert(0) += 1;
                for replica in &partition.replicas {
                    *broker_replica_counts.entry(*replica).or_insert(0) += 1;
                }
            }
        }

        Ok((cluster_info, topics, partition_count, under_replicated, broker_leader_counts, broker_replica_counts))
    }))
    .await;

    // 如果超时或失败，返回空数据
    let (cluster_info, topics, partition_count, under_replicated, broker_leader_counts, broker_replica_counts) = match result {
        Ok(Ok(Ok(data))) => data,
        Ok(Ok(Err(e))) => {
            tracing::warn!("Failed to get cluster stats for '{}': {}", cluster_id, e);
            return Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "broker_count": 0,
                "controller_id": null,
                "topic_count": 0,
                "partition_count": 0,
                "under_replicated_partitions": 0,
                "consumer_group_count": 0,
                "total_lag": 0,
                "broker_stats": [],
                "error": format!("Failed to connect: {}", e),
            }));
        }
        Ok(Err(e)) => {
            tracing::warn!("Task failed for cluster '{}': {}", cluster_id, e);
            return Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "broker_count": 0,
                "controller_id": null,
                "topic_count": 0,
                "partition_count": 0,
                "under_replicated_partitions": 0,
                "consumer_group_count": 0,
                "total_lag": 0,
                "broker_stats": [],
                "error": format!("Task failed: {}", e),
            }));
        }
        Err(_) => {
            tracing::warn!("Timeout getting cluster stats for '{}'", cluster_id);
            return Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "broker_count": 0,
                "controller_id": null,
                "topic_count": 0,
                "partition_count": 0,
                "under_replicated_partitions": 0,
                "consumer_group_count": 0,
                "total_lag": 0,
                "broker_stats": [],
                "error": "Timeout connecting to cluster",
            }));
        }
    };

    // 构建 broker 统计
    let broker_stats: Vec<Value> = cluster_info
        .brokers
        .iter()
        .map(|b| {
            let is_controller = cluster_info.controller_id == Some(b.id);
            serde_json::json!({
                "id": b.id,
                "host": b.host.clone(),
                "port": b.port,
                "is_controller": is_controller,
                "leader_partitions": *broker_leader_counts.get(&b.id).unwrap_or(&0),
                "replica_partitions": *broker_replica_counts.get(&b.id).unwrap_or(&0),
            })
        })
        .collect();

    // 注意：由于 rdkafka 限制，consumer group 数量暂时返回 0
    // 实际应用中可以通过 offsets topic 来估算
    let consumer_group_count = 0;
    let total_lag = 0;

    Ok(serde_json::json!({
        "cluster_id": cluster_id,
        "broker_count": cluster_info.brokers.len(),
        "controller_id": cluster_info.controller_id,
        "topic_count": topics.len(),
        "partition_count": partition_count,
        "under_replicated_partitions": under_replicated,
        "consumer_group_count": consumer_group_count,
        "total_lag": total_lag,
        "broker_stats": broker_stats,
    }))
}

async fn reload_clients(state: &AppState) -> Result<()> {
    use crate::config::KafkaConfig;

    // Get all clusters from database
    let clusters = ClusterStore::list(state.db.inner()).await?;

    let mut new_clusters = std::collections::HashMap::new();
    for cluster in &clusters {
        new_clusters.insert(
            cluster.name.clone(),
            KafkaConfig {
                brokers: cluster.brokers.clone(),
                request_timeout_ms: cluster.request_timeout_ms as u32,
                operation_timeout_ms: cluster.operation_timeout_ms as u32,
            },
        );
    }

    // Create new KafkaClients
    let new_clients = crate::kafka::KafkaClients::new(&new_clusters)?;

    // Clone clients for background sync (to avoid holding reference)
    let clients_for_sync = new_clients.clone();
    let db_pool = state.db.clone();
    let cluster_names: Vec<String> = clusters.iter().map(|c| c.name.clone()).collect();

    // Spawn background task for topic sync to avoid blocking the response
    tokio::spawn(async move {
        for cluster_name in cluster_names {
            let db = db_pool.inner();
            if let Some(admin) = clients_for_sync.get_admin(&cluster_name) {
                // Use spawn_blocking for synchronous list_topics
                let admin_clone = admin.clone();
                match tokio::task::spawn_blocking(move || admin_clone.list_topics())
                    .await
                {
                    Ok(Ok(topics)) => {
                        let _ = TopicStore::sync_topics(db, &cluster_name, &topics).await;
                        tracing::info!(
                            "Synced {} topics for cluster '{}'",
                            topics.len(),
                            cluster_name
                        );
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("Failed to list topics for cluster '{}': {}", cluster_name, e);
                    }
                    Err(e) => {
                        tracing::warn!("Topic list task panicked for cluster '{}': {}", cluster_name, e);
                    }
                }
            }
            // Small delay between clusters to avoid overwhelming
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    // Atomically update Kafka clients immediately (don't wait for sync)
    state.set_clients(new_clients.into());

    tracing::info!("Reloaded Kafka clients (topic sync in background)");

    Ok(())
}

// ==================== Topic ====================

async fn handle_topic_list(state: AppState, body: Value) -> Result<Value> {
    // cluster_id is optional - when not provided, fetch topics from all clusters
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    // When cluster_id is not provided, fetch topics from all clusters
    let cluster_id = match cluster_id {
        Some(id) => id,
        None => return handle_topic_list_all_clusters(state).await,
    };

    // 首先尝试从数据库获取（最快）
    let db_topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await.ok();

    // 如果有数据库缓存，先返回（快速响应）
    if let Some(topics) = db_topics {
        if !topics.is_empty() {
            // 后台异步同步 Kafka（不阻塞响应）
            let state_clone = state.clone();
            let cluster_id_clone = cluster_id.clone();
            tokio::spawn(async move {
                let _ = sync_topics_from_kafka(state_clone, &cluster_id_clone).await;
            });
            // 只返回 topic 名称字符串数组
            let topic_names: Vec<String> = topics.into_iter().map(|t| t.topic_name).collect();
            return Ok(serde_json::json!({ "topics": topic_names }));
        }
    }

    // 数据库没有数据，从 Kafka 获取
    sync_topics_from_kafka(state, &cluster_id).await
}

/// Fetch topics from all clusters
async fn handle_topic_list_all_clusters(state: AppState) -> Result<Value> {
    use crate::db::cluster::ClusterStore;

    // Get all clusters from database
    let clusters = ClusterStore::list(state.db.inner()).await?;

    let mut all_topics: Vec<String> = Vec::new();

    // Fetch topics from each cluster
    for cluster in clusters {
        let cluster_name = &cluster.name;

        // Try to get topics from database first
        if let Ok(topics) = TopicStore::list_by_cluster(state.db.inner(), cluster_name).await {
            for topic in topics {
                all_topics.push(topic.topic_name);
            }
        }

        // Background sync for this cluster (non-blocking)
        let state_clone = state.clone();
        let cluster_name_clone = cluster_name.clone();
        tokio::spawn(async move {
            let _ = sync_topics_from_kafka(state_clone, &cluster_name_clone).await;
        });
    }

    // Remove duplicates and sort
    all_topics.sort();
    all_topics.dedup();

    Ok(serde_json::json!({ "topics": all_topics }))
}

/// 从 Kafka 同步主题列表到数据库，并返回结果
async fn sync_topics_from_kafka(state: AppState, cluster_id: &str) -> Result<Value> {
    let admin = get_or_create_admin_client(&state, cluster_id).await?;

    // Use spawn_blocking to avoid blocking async runtime
    // Add retry logic for initial connection establishment
    let mut last_error = None;
    for attempt in 0..3 {
        let admin_clone = admin.clone();
        let result = tokio::task::spawn_blocking(move || admin_clone.list_topics())
            .await
            .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?;

        match result {
            Ok(topics) => {
                // 同步到数据库
                let _ = TopicStore::sync_topics(state.db.inner(), cluster_id, &topics).await;

                // 写入内存缓存
                state
                    .cache
                    .set_topic_list(cluster_id, topics.clone())
                    .await;

                return Ok(serde_json::json!({ "topics": topics }));
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < 2 {
                    // Wait before retry (initial connection may need time to establish)
                    let wait_ms = 200 * (attempt + 1) as u64;
                    tracing::warn!("list_topics failed (attempt {}), retrying in {}ms: {}", attempt + 1, wait_ms, last_error.as_ref().map(|e| format!("{}", e)).unwrap_or_default());
                    tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
                }
            }
        }
    }

    last_error.map_or_else(
        || Err(AppError::Internal("Unknown error occurred".to_string())),
        Err
    )
}

async fn handle_topic_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;

    let admin = get_or_create_admin_client(&state, &cluster_id).await?;

    let topic_info = admin.get_topic_info(&name)?;

    Ok(serde_json::json!({
        "name": topic_info.name,
        "partitions": topic_info
            .partitions
            .into_iter()
            .map(|p| serde_json::json!({
                "id": p.id,
                "leader": p.leader,
                "replicas": p.replicas,
                "isr": p.isr,
            }))
            .collect::<Vec<_>>(),
    }))
}

async fn handle_topic_create(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;
    let num_partitions = get_optional_i32_param(&body, "num_partitions").unwrap_or(1);
    let replication_factor = get_optional_i32_param(&body, "replication_factor").unwrap_or(1);
    let config = get_hashmap_param(&body, "config");

    let admin = get_or_create_admin_client(&state, &cluster_id).await?;

    admin
        .create_topic(&name, num_partitions, replication_factor, config)
        .await?;

    // Invalidate cache
    state.cache.invalidate_topic_list(&cluster_id).await;

    Ok(serde_json::json!({ "name": name }))
}

async fn handle_topic_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;

    let admin = get_or_create_admin_client(&state, &cluster_id).await?;

    admin.delete_topic(&name).await?;

    // Invalidate cache
    state.cache.invalidate_topic_list(&cluster_id).await;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_topic_batch_create(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topics = body
        .get("topics")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::BadRequest("Missing topics array".to_string()))?;
    let continue_on_error = get_optional_bool_param(&body, "continue_on_error").unwrap_or(false);

    let admin = get_or_create_admin_client(&state, &cluster_id).await?;

    let mut created = Vec::new();
    let mut failed = Vec::new();

    for topic_req in topics {
        let name = topic_req
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let num_partitions = topic_req
            .get("num_partitions")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(1);
        let replication_factor = topic_req
            .get("replication_factor")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(1);
        let config = topic_req
            .get("config")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        match admin
            .create_topic(&name, num_partitions, replication_factor, config)
            .await
        {
            Ok(_) => created.push(name),
            Err(e) => {
                failed.push(serde_json::json!({
                    "name": name,
                    "error": e.to_string()
                }));
                if !continue_on_error {
                    return Ok(serde_json::json!({
                        "success": false,
                        "created": created,
                        "failed": failed
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "success": failed.is_empty(),
        "created": created,
        "failed": failed
    }))
}

async fn handle_topic_batch_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topics = get_string_array_param(&body, "topics");
    let continue_on_error = get_optional_bool_param(&body, "continue_on_error").unwrap_or(false);

    let admin = get_or_create_admin_client(&state, &cluster_id).await?;

    let mut deleted = Vec::new();
    let mut failed = Vec::new();

    for topic_name in topics {
        match admin.delete_topic(&topic_name).await {
            Ok(_) => deleted.push(topic_name),
            Err(e) => {
                failed.push(serde_json::json!({
                    "name": topic_name,
                    "error": e.to_string()
                }));
                if !continue_on_error {
                    return Ok(serde_json::json!({
                        "success": false,
                        "deleted": deleted,
                        "failed": failed
                    }));
                }
            }
        }
    }

    // Invalidate cache
    state.cache.invalidate_topic_list(&cluster_id).await;

    Ok(serde_json::json!({
        "success": failed.is_empty(),
        "deleted": deleted,
        "failed": failed
    }))
}

/// 删除集群下所有 topic（仅删除数据库元数据）
async fn handle_topic_delete_all(state: AppState, body: Value) -> Result<Value> {
    use crate::db::topic::TopicStore;

    let cluster_id = get_string_param(&body, "cluster_id")?;

    // 从数据库获取该集群下的所有 topic
    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let topic_names: Vec<String> = topics.iter().map(|t| t.topic_name.clone()).collect();

    if topic_names.is_empty() {
        return Ok(serde_json::json!({
            "success": true,
            "deleted": Vec::<String>::new(),
            "failed": Vec::<serde_json::Value>::new(),
            "total_deleted": 0,
            "total_failed": 0
        }));
    }

    // 从数据库删除所有 topic 元数据
    for topic_name in &topic_names {
        let _ = TopicStore::delete(state.db.inner(), &cluster_id, topic_name).await;
    }

    // Invalidate cache
    state.cache.invalidate_topic_list(&cluster_id).await;

    Ok(serde_json::json!({
        "success": true,
        "deleted": topic_names,
        "failed": Vec::<serde_json::Value>::new(),
        "total_deleted": topic_names.len(),
        "total_failed": 0
    }))
}

async fn handle_topic_offsets(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let offset_manager = KafkaOffsetManager::new(&config);
    let partition_offsets = offset_manager.get_topic_partition_offsets(&config, &name)?;

    let details: Vec<Value> = partition_offsets
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "topic": p.topic,
                "partition": p.partition,
                "leader": p.leader,
                "replicas": p.replicas,
                "isr": p.isr,
                "earliest_offset": p.earliest_offset,
                "latest_offset": p.latest_offset,
                "first_commit_time": p.first_commit_time,
                "last_commit_time": p.last_commit_time,
            })
        })
        .collect();

    Ok(serde_json::json!({ "offsets": details }))
}

/// 获取分区的 watermarks（low 和 high offset）
async fn handle_topic_partition_watermarks(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;
    let partition = get_i32_param(&body, "partition")?;

    let clients = state.get_clients();
    let config = clients.get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let watermarks = admin.get_partition_watermarks(&topic, partition, &config.brokers)?;

    Ok(serde_json::json!({
        "low_offset": watermarks.0,
        "high_offset": watermarks.1,
    }))
}

async fn handle_topic_config_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let config = admin.get_topic_config(&name).await?;

    Ok(serde_json::json!(config))
}

async fn handle_topic_config_alter(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;
    let config = get_hashmap_param(&body, "config");

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    admin.alter_topic_config(&name, config).await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_topic_partitions_add(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;
    let new_partitions = get_i32_param(&body, "new_partitions")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    admin.create_partitions(&name, new_partitions).await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_topic_throughput(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let calculator = KafkaThroughputCalculator::new(&config);
    let throughput = calculator.calculate_topic_throughput(&config, &name)?;

    Ok(serde_json::json!({
        "topic": throughput.topic,
        "produce_throughput": {
            "messages_per_second": throughput.produce_throughput.messages_per_second,
            "bytes_per_second": throughput.produce_throughput.bytes_per_second,
            "window_seconds": throughput.produce_throughput.window_seconds,
        },
        "total_messages": throughput.total_messages,
        "partitions": throughput.partitions.iter().map(|p| serde_json::json!({
            "partition": p.partition,
            "earliest_offset": p.earliest_offset,
            "latest_offset": p.latest_offset,
            "message_count": p.message_count,
            "produce_rate": p.produce_rate,
            "first_message_time": p.first_message_time,
            "last_message_time": p.last_message_time,
        })).collect::<Vec<_>>(),
    }))
}

async fn handle_topic_refresh(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    // 首先尝试从内存中获取 admin 客户端
    let clients = state.get_clients();
    let admin = clients.get_admin(&cluster_id);

    // 如果不存在，尝试从数据库获取集群配置并建立连接
    let admin = if let Some(existing_admin) = admin {
        existing_admin
    } else {
        let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_id)
            .await?
            .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

        let config = crate::config::KafkaConfig {
            brokers: cluster.brokers,
            request_timeout_ms: cluster.request_timeout_ms as u32,
            operation_timeout_ms: cluster.operation_timeout_ms as u32,
        };

        // 建立连接池连接
        let _ = state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await;

        // 获取新的客户端（包含刚添加的集群）
        let new_clients = state.get_clients();
        new_clients.get_admin(&cluster_id)
            .ok_or_else(|| AppError::NotFound(format!("Failed to get admin client for cluster '{}'", cluster_id)))?
    };

    // Get current topics from Kafka (blocking operation)
    let current_topics = {
        let admin = admin.clone();
        tokio::task::spawn_blocking(move || admin.list_topics())
            .await
            .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??
    };

    // Sync to database
    let sync_result = TopicStore::sync_topics(state.db.inner(), &cluster_id, &current_topics).await?;

    // Save new topics to database (blocking operations in spawn_blocking)
    for topic_name in &sync_result.added {
        let topic_name = topic_name.clone();
        let admin = admin.clone();
        let cluster_id = cluster_id.clone();
        let db = state.db.clone();

        tokio::task::spawn_blocking(move || {
            if let Ok(topic_info) = admin.get_topic_info(&topic_name) {
                let config = std::collections::HashMap::new();
                let _ = TopicStore::upsert(
                    db.inner(),
                    &cluster_id,
                    &topic_name,
                    topic_info.partitions.len() as i32,
                    1,
                    &config,
                );
            }
        }).await.ok();
    }

    // Get total count
    let all_topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;

    Ok(serde_json::json!({
        "success": true,
        "added": sync_result.added,
        "removed": sync_result.removed,
        "total": all_topics.len(),
    }))
}

async fn handle_topic_search(state: AppState, body: Value) -> Result<Value> {
    use crate::db::topic::TopicStore;

    // 获取搜索关键词（可选参数）
    let keyword = body.get("keyword").and_then(|v| v.as_str()).filter(|s| !s.is_empty());

    let start = std::time::Instant::now();
    tracing::info!("[search] keyword: {:?}", keyword);

    let topics = if let Some(kw) = keyword {
        // 有搜索关键词时，使用模糊查询
        TopicStore::search_topics(state.db.inner(), kw).await?
    } else {
        // 无关键词时，返回所有 topic（限制 100 条）
        TopicStore::list_all_limit(state.db.inner(), 100).await?
    };

    tracing::info!("[search] found {} topics in {:?}", topics.len(), start.elapsed());

    // 转换为响应格式
    let results: Vec<Value> = topics
        .into_iter()
        .map(|topic| {
            serde_json::json!({
                "cluster": topic.cluster_id,
                "topic": topic.topic_name,
            })
        })
        .collect();

    Ok(serde_json::json!({ "results": results }))
}

async fn handle_topic_count(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;

    Ok(serde_json::json!({ "count": topics.len() }))
}

// ==================== Consumer Group ====================

async fn handle_consumer_group_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    // Try cache first
    if let Some(cached_groups) = state.cache.get_consumer_group_list(&cluster_id).await {
        let groups: Vec<Value> = cached_groups
            .into_iter()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "state": "Unknown"
                })
            })
            .collect();
        return Ok(serde_json::json!({ "groups": groups }));
    }

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let groups = admin.list_consumer_groups(&config)?;
    let group_names: Vec<String> = groups.iter().map(|g| g.name.clone()).collect();

    // Write to cache
    state
        .cache
        .set_consumer_group_list(&cluster_id, group_names.clone())
        .await;

    let group_summaries: Vec<Value> = groups
        .into_iter()
        .map(|g| {
            serde_json::json!({
                "name": g.name,
                "state": g.state,
            })
        })
        .collect();

    Ok(serde_json::json!({ "groups": group_summaries }))
}

async fn handle_consumer_group_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let group_info = admin.get_consumer_group_info(&config, &name)?;

    Ok(serde_json::json!({
        "name": group_info.name,
        "state": group_info.state,
        "protocol": group_info.protocol,
        "members": group_info
            .members
            .into_iter()
            .map(|m| serde_json::json!({
                "client_id": m.client_id,
                "host": m.host,
            }))
            .collect::<Vec<_>>(),
        "offsets": [],
    }))
}

async fn handle_consumer_group_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let name = get_string_param(&body, "name")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    admin.delete_consumer_group(&name).await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_consumer_group_offsets(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let group_name = get_string_param(&body, "group_name")?;
    let topic = get_optional_string_param(&body, "topic");

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let offset_manager = KafkaOffsetManager::new(&config);
    let offsets =
        offset_manager.get_consumer_group_offsets(&config, &group_name, topic.as_deref())?;

    let topic_name = offsets.first().map(|o| o.topic.clone()).unwrap_or_default();
    let total_lag: i64 = offsets.iter().map(|o| o.lag).sum();

    let partitions: Vec<Value> = offsets
        .into_iter()
        .map(|o| {
            serde_json::json!({
                "partition": o.partition,
                "current_offset": o.current_offset,
                "log_end_offset": o.log_end_offset,
                "lag": o.lag,
                "state": if o.current_offset >= 0 { "Active" } else { "Empty" },
                "topic": o.topic,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "group_name": group_name,
        "topic": topic_name,
        "partitions": partitions,
        "total_lag": total_lag,
    }))
}

async fn handle_consumer_group_offsets_reset(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let group_name = get_string_param(&body, "group_name")?;
    let topic = get_string_param(&body, "topic")?;
    let partition = get_optional_i32_param(&body, "partition");

    // Parse offset type
    let offset_type = if let Some(offset_val) = get_optional_i64_param(&body, "value") {
        crate::kafka::admin::OffsetType::Value(offset_val)
    } else if let Some(ts) = get_optional_i64_param(&body, "timestamp") {
        crate::kafka::admin::OffsetType::Timestamp(ts)
    } else if body.get("earliest").is_some() {
        crate::kafka::admin::OffsetType::Earliest
    } else {
        crate::kafka::admin::OffsetType::Latest
    };

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    if let Some(partition) = partition {
        admin
            .reset_consumer_group_offset(&config, &group_name, &topic, partition, offset_type)
            .await?;
    } else {
        admin
            .reset_consumer_group_offsets(&config, &group_name, &topic, offset_type)
            .await?;
    }

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_consumer_group_throughput(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let group_name = get_string_param(&body, "group_name")?;
    let topic = get_string_param(&body, "topic")?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let calculator = KafkaThroughputCalculator::new(&config);
    let throughput = calculator.calculate_consumer_group_throughput(&config, &group_name, &topic)?;

    Ok(serde_json::json!({
        "group_name": throughput.group_name,
        "topic": throughput.topic,
        "consume_throughput": {
            "messages_per_second": throughput.consume_throughput.messages_per_second,
            "bytes_per_second": throughput.consume_throughput.bytes_per_second,
            "window_seconds": throughput.consume_throughput.window_seconds,
        },
        "total_lag": throughput.total_lag,
        "estimated_time_to_catch_up": throughput.estimated_time_to_catch_up,
        "partitions": throughput.partitions.iter().map(|p| serde_json::json!({
            "partition": p.partition,
            "current_offset": p.current_offset,
            "log_end_offset": p.log_end_offset,
            "lag": p.lag,
            "consume_rate": p.consume_rate,
        })).collect::<Vec<_>>(),
    }))
}

async fn handle_consumer_group_batch_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let group_names = get_string_array_param(&body, "group_names");
    let continue_on_error = get_optional_bool_param(&body, "continue_on_error").unwrap_or(false);

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let mut deleted = Vec::new();
    let mut failed = Vec::new();

    for group_name in group_names {
        match admin.delete_consumer_group(&group_name).await {
            Ok(_) => deleted.push(group_name),
            Err(e) => {
                failed.push(serde_json::json!({
                    "name": group_name,
                    "error": e.to_string()
                }));
                if !continue_on_error {
                    return Ok(serde_json::json!({
                        "success": false,
                        "deleted": deleted,
                        "failed": failed
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "success": failed.is_empty(),
        "deleted": deleted,
        "failed": failed
    }))
}

async fn handle_consumer_group_consumer_offsets(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let offset_manager = KafkaOffsetManager::new(&config);
    let all_offsets = offset_manager.get_all_consumer_offsets(&config)?;

    let consumer_groups: Vec<Value> = all_offsets
        .into_iter()
        .map(|group| {
            serde_json::json!({
                "group_name": group.group_name,
                "state": group.state,
                "total_lag": group.total_lag,
                "topics": group.topics.iter().map(|topic| serde_json::json!({
                    "topic": topic.topic,
                    "total_lag": topic.total_lag,
                    "partitions": topic.partitions.iter().map(|p| serde_json::json!({
                        "partition": p.partition,
                        "start_offset": p.start_offset,
                        "end_offset": p.end_offset,
                        "current_offset": p.current_offset,
                        "lag": p.lag,
                        "state": if p.current_offset >= 0 { "Active" } else { "Empty" },
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "cluster_id": cluster_id,
        "consumer_groups": consumer_groups,
    }))
}

// ==================== Message ====================

async fn handle_message_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;
    let partition = get_optional_i32_param(&body, "partition");
    let offset = get_optional_i64_param(&body, "offset");
    let max_messages = get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
    let limit = get_optional_i64_param(&body, "limit").map(|v| v as usize);
    let start_time = get_optional_i64_param(&body, "start_time");
    let end_time = get_optional_i64_param(&body, "end_time");
    let search = get_optional_string_param(&body, "search");
    let fetch_mode = get_optional_string_param(&body, "fetchMode");
    let sort = get_optional_string_param(&body, "sort");

    // 首先确保集群客户端已创建（如果未创建则自动创建）
    let config = ensure_cluster_client(&state, &cluster_id).await?;

    let max_msgs = limit.or(max_messages).unwrap_or(100);

    // 直接使用临时 consumer 获取消息（避免连接池状态问题）
    let messages = fetch_messages_with_temp_consumer(
        &config.brokers,
        &topic,
        partition,
        offset,
        max_msgs,
        start_time,
        end_time,
        search,
        fetch_mode.as_deref(),
        sort.as_deref(),
    )
    .await?;

    let records: Vec<Value> = messages
        .into_iter()
        .map(|msg| {
            serde_json::json!({
                "partition": msg.partition,
                "offset": msg.offset,
                "key": msg.key,
                "value": msg.value,
                "timestamp": msg.timestamp,
            })
        })
        .collect();

    Ok(serde_json::json!({ "messages": records }))
}

/// 使用临时 consumer 获取消息（支持过滤）- 自动适配本地/远程Kafka
async fn fetch_messages_with_temp_consumer(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<crate::kafka::consumer::KafkaMessage>> {
    // 检测是否是本地Kafka
    let is_local = brokers.contains("localhost") ||
                   brokers.contains("127.0.0.1") ||
                   brokers.contains("::1") ||
                   brokers.contains("host.docker.internal");

    if is_local {
        fetch_messages_local(brokers, topic, partition, offset, max_messages, start_time, end_time, search, fetch_mode, sort).await
    } else {
        fetch_messages_remote(brokers, topic, partition, offset, max_messages, start_time, end_time, search, fetch_mode, sort).await
    }
}

/// 本地Docker优化版 - 单consumer多分区
/// 并行获取单个分区的消息
fn fetch_partition_messages_parallel(
    brokers: String,
    topic: String,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<String>,
) -> Vec<crate::kafka::consumer::KafkaMessage> {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::Message;
    use rdkafka::TopicPartitionList;
    use rdkafka::ClientConfig;
    use std::time::Duration;

    tracing::info!("[Parallel] Starting fetch for partition {} of topic {} (max_messages: {})", partition, topic, max_messages);

    // 创建独立的 consumer（无 group.id，直接分配分区）
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers);
    cfg.set("group.id", &format!("kafka-mgr-parallel-{}", partition));
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    // 大批量优化配置
    cfg.set("fetch.min.bytes", "1");
    cfg.set("fetch.wait.max.ms", "10");
    cfg.set("fetch.max.bytes", "104857600");       // 100MB
    cfg.set("max.partition.fetch.bytes", "104857600"); // 100MB per partition
    cfg.set("socket.nagle.disable", "true");
    cfg.set("socket.receive.buffer.bytes", "2097152"); // 2MB
    cfg.set("enable.partition.eof", "false");
    // 强制使用 IPv4，避免 IPv6 连接问题
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[Parallel] Failed to create consumer for partition {}: {}", partition, e);
            return Vec::new();
        }
    };

    // 计算起始 offset
    let start_offset = match calculate_partition_offset(&consumer, &topic, partition, max_messages, offset, start_time, fetch_mode.as_deref()) {
        Ok(o) => o,
        Err(e) => {
            tracing::error!("[Parallel] Failed to calculate offset for partition {}: {}", partition, e);
            return Vec::new();
        }
    };
    tracing::info!("[Parallel] Partition {} start_offset: {}", partition, start_offset);

    // 分配到指定分区
    let mut tpl = TopicPartitionList::new();
    let seek_offset = if start_offset < 0 {
        rdkafka::Offset::Beginning
    } else {
        rdkafka::Offset::Offset(start_offset)
    };
    if let Err(e) = tpl.add_partition_offset(&topic, partition, seek_offset) {
        tracing::error!("[Parallel] Failed to add partition {}: {}", partition, e);
        return Vec::new();
    }
    if let Err(e) = consumer.assign(&tpl) {
        tracing::error!("[Parallel] Failed to assign partition {}: {}", partition, e);
        return Vec::new();
    }

    // 计算结束 offset
    let end_offset = if fetch_mode.as_deref() == Some("newest") {
        match consumer.fetch_watermarks(&topic, partition, Duration::from_secs(5)) {
            Ok((_, high)) if high > 0 => Some(high),
            Err(e) => {
                tracing::warn!("[Parallel] Failed to fetch watermarks for partition {}: {}", partition, e);
                None
            }
            _ => None,
        }
    } else {
        None
    };

    let search_lower = search.map(|s| s.to_lowercase());
    let mut raw_messages = Vec::with_capacity(max_messages);
    let mut empty_count = 0;
    let mut polled_count = 0;
    let max_empty = max_messages / 100 + 5; // 自适应空轮次上限，最小5
    // 无时间范围时：每个分区最多获取 max_messages 条，然后在这些消息中搜索过滤
    // 有时间范围时：在时间范围内获取最多 max_messages 条，再进行搜索过滤

    // 先收集原始消息（最多 max_messages 条）
    while raw_messages.len() < max_messages && empty_count < max_empty {
        match consumer.poll(Duration::from_millis(200)) {
            Some(Ok(msg)) => {
                polled_count += 1;
                empty_count = 0;

                // newest 模式下检查是否到达末尾
                if let Some(end) = end_offset {
                    if msg.offset() >= end {
                        break;
                    }
                }

                let ts = msg.timestamp().to_millis();

                // 时间范围过滤（在时间范围内才收集）
                if let Some(start) = start_time {
                    if let Some(t) = ts { if t < start { continue; } }
                }
                if let Some(end) = end_time {
                    if let Some(t) = ts { if t > end { continue; } }
                }

                let key = msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                let value = msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                raw_messages.push(crate::kafka::consumer::KafkaMessage {
                    partition,
                    offset: msg.offset(),
                    key,
                    value,
                    timestamp: ts,
                });
            }
            Some(Err(e)) => {
                tracing::warn!("Poll error for partition {}: {}", partition, e);
                empty_count += 1;
            }
            None => {
                empty_count += 1;
            }
        }
    }

    // 对已收集的消息进行搜索过滤
    let messages: Vec<_> = if let Some(ref term) = search_lower {
        raw_messages
            .into_iter()
            .filter(|msg| {
                let km = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(term));
                let vm = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(term));
                km || vm
            })
            .collect()
    } else {
        raw_messages
    };

    tracing::info!("[Parallel] Fetched {} messages from partition {} (polled: {}, after search filter: {})",
        messages.len(), partition, polled_count, messages.len());
    messages
}

/// 计算分区的起始 offset
fn calculate_partition_offset(
    consumer: &rdkafka::consumer::BaseConsumer,
    topic: &str,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    fetch_mode: Option<&str>,
) -> Result<i64> {
    use rdkafka::consumer::Consumer;
    use rdkafka::TopicPartitionList;
    use std::time::Duration;

    // 如果用户指定了特定 offset，优先使用
    if let Some(off) = offset {
        if off >= 0 {
            return Ok(off);
        }
    }

    if let Some(time) = start_time {
        if time > 0 {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(time)).ok();
            match consumer.offsets_for_times(tpl, Duration::from_secs(5)) {
                Ok(r) => {
                    let elements = r.elements_for_topic(topic);
                    for elem in elements {
                        if elem.partition() == partition {
                            if let Some(offset) = elem.offset().to_raw() {
                                return Ok(offset);
                            }
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to get offset for time: {}", e),
            }
        }
    }

    match fetch_mode {
        Some("newest") | None => {
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, high)) if high > 0 => {
                    let latest = high.saturating_sub(1);
                    let start = latest.saturating_sub((max_messages.saturating_sub(1)) as i64).max(low);
                    Ok(start)
                }
                _ => Ok(0),
            }
        }
        Some("oldest") => {
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, _)) => Ok(low),
                Err(_) => Ok(0),
            }
        }
        _ => Ok(0),
    }
}

/// 本地集群 - 分层优化策略
async fn fetch_messages_local(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<crate::kafka::consumer::KafkaMessage>> {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::Message;
    use rdkafka::TopicPartitionList;
    use rdkafka::ClientConfig;
    use std::time::Duration;
    use std::collections::HashMap;

    let start_time_total = std::time::Instant::now();
    let topic = topic.to_string();
    let brokers = brokers.to_string();
    let search = search.clone();
    let fetch_mode = fetch_mode.map(|s| s.to_string());
    let sort = sort.map(|s| s.to_string());
    let offset = offset;

    // 获取分区列表
    let partitions: Vec<i32> = {
        let cfg = ClientConfig::new()
            .set("bootstrap.servers", &brokers)
            .set("broker.address.family", "v4")
            .create::<BaseConsumer>()?;
        if partition.is_none() {
            match cfg.fetch_metadata(Some(&topic), Duration::from_secs(5)) {
                Ok(metadata) => metadata.topics().first()
                    .map(|t| t.partitions().iter().map(|p| p.id()).collect())
                    .unwrap_or_else(|| vec![0]),
                Err(_) => vec![0],
            }
        } else {
            vec![partition.unwrap_or(0)]
        }
    };

    let partition_count = partitions.len();
    let total_target = max_messages * partition_count;

    tracing::info!("[Local] {} partitions, {} messages per partition, total {}. Mode: {:?}",
        partition_count, max_messages, total_target,
        if max_messages >= 1000 { "parallel" } else { "sequential" });

    let messages = if max_messages >= 1000 && partition_count > 1 {
        // === 大批量并行模式（>=1000条/分区）===
        // 每个分区都请求 max_messages 条（max_messages 是 per partition 的）
        // 限制并发数以避免对 Kafka 造成过大压力
        let msgs_per_partition = max_messages;
        tracing::info!("[Parallel Mode] {} partitions, {} messages per partition, total target {}",
            partition_count, msgs_per_partition, total_target);
        let partition_has_specific_offset = partition.is_some();
        let partition_offset_val = if partition_has_specific_offset { offset } else { None };

        use tokio::sync::Semaphore;
        use tokio::time::{timeout, Duration as TokioDuration};
        use std::sync::Arc;

        // 最多 10 个并发
        let semaphore = Arc::new(Semaphore::new(10));
        let mut handles = vec![];

        for &part_id in &partitions {
            let brokers = brokers.clone();
            let topic = topic.clone();
            let search = search.clone();
            let fetch_mode = fetch_mode.clone();
            let part_offset = if partition_has_specific_offset { partition_offset_val } else { None };
            let sem = semaphore.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("Semaphore acquire failed");
                timeout(TokioDuration::from_secs(30), tokio::task::spawn_blocking(move || {
                    fetch_partition_messages_parallel(
                        brokers, topic, part_id, msgs_per_partition, part_offset,
                        start_time, end_time, search, fetch_mode,
                    )
                })).await
            });
            handles.push(handle);
        }

        let mut all_msgs: Vec<crate::kafka::consumer::KafkaMessage> = Vec::with_capacity(total_target);
        for handle in handles {
            match handle.await {
                Ok(Ok(Ok(msgs))) => {
                    tracing::info!("[Parallel] Got {} messages from task", msgs.len());
                    all_msgs.extend(msgs);
                }
                Ok(Ok(Err(_))) => tracing::warn!("Partition fetch timeout"),
                Ok(Err(e)) => tracing::warn!("Task join error: {}", e),
                Err(e) => tracing::warn!("Spawn error: {}", e),
            }
        }
        all_msgs
    } else {
        // === 小批量串行模式（<1000条/分区）===
        // 捕获 partition 变量用于判断是否应该使用 offset
        let partition_filter = partition;
        tokio::task::spawn_blocking(move || {
            let mut cfg = ClientConfig::new();
            cfg.set("bootstrap.servers", &brokers);
            cfg.set("group.id", "kafka-mgr-fast");
            cfg.set("enable.auto.commit", "false");
            cfg.set("auto.offset.reset", "earliest");
            cfg.set("fetch.min.bytes", "1");
            cfg.set("fetch.wait.max.ms", "10");
            cfg.set("fetch.max.bytes", "52428800");
            cfg.set("socket.nagle.disable", "true");
            cfg.set("broker.address.family", "v4");

            let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };

            // 只有指定了特定分区时，才传递 offset 参数
            let effective_offset = if partition_filter.is_some() { offset } else { None };

            let mut tpl = TopicPartitionList::new();
            for &part_id in &partitions {
                let start_offset = calculate_partition_offset(&consumer, &topic, part_id, max_messages, effective_offset, start_time, fetch_mode.as_deref())
                    .unwrap_or(0);
                let seek_offset = if start_offset < 0 {
                    rdkafka::Offset::Beginning
                } else {
                    rdkafka::Offset::Offset(start_offset)
                };
                tpl.add_partition_offset(&topic, part_id, seek_offset).ok();
            }

            if consumer.assign(&tpl).is_err() {
                return Vec::new();
            }

            // Seek 到正确位置
            for &part_id in &partitions {
                let start_offset = calculate_partition_offset(&consumer, &topic, part_id, max_messages, effective_offset, start_time, fetch_mode.as_deref())
                    .unwrap_or(0);
                let seek_offset = if start_offset < 0 {
                    rdkafka::Offset::Beginning
                } else {
                    rdkafka::Offset::Offset(start_offset)
                };
                if let Err(e) = consumer.seek(&topic, part_id, seek_offset, Duration::from_secs(5)) {
                    tracing::warn!("[Local] Failed to seek partition {} to offset {:?}: {}", part_id, seek_offset, e);
                }
            }

            let search_lower = search.as_ref().map(|s| s.to_lowercase());
            let is_desc = sort.as_deref() == Some("desc") || (sort.is_none() && fetch_mode.as_deref() == Some("newest"));

            // 第一步：收集原始消息（每个分区最多 max_messages 条）
            let mut partition_counts: HashMap<i32, usize> = HashMap::with_capacity(partition_count);
            let mut raw_msgs: Vec<crate::kafka::consumer::KafkaMessage> = Vec::with_capacity(max_messages * partition_count);
            let mut total_empty = 0;
            let max_empty_rounds = (partition_count * max_messages).max(20); // 空轮次上限

            // 收集消息，每个分区最多 max_messages 条
            while raw_msgs.len() < max_messages * partition_count && total_empty < max_empty_rounds {
                match consumer.poll(Duration::from_millis(200)) {
                    Some(Ok(msg)) => {
                        let part = msg.partition();
                        let count = partition_counts.get(&part).copied().unwrap_or(0);
                        if count >= max_messages {
                            // 该分区已满，跳过
                            continue;
                        }
                        total_empty = 0;

                        let ts = msg.timestamp().to_millis();
                        // 时间范围过滤
                        if let Some(start) = start_time {
                            if let Some(t) = ts { if t < start { continue; } }
                        }
                        if let Some(end) = end_time {
                            if let Some(t) = ts { if t > end { continue; } }
                        }

                        let key = msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                        let value = msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                        partition_counts.insert(part, count + 1);
                        raw_msgs.push(crate::kafka::consumer::KafkaMessage {
                            partition: part,
                            offset: msg.offset(),
                            key,
                            value,
                            timestamp: ts,
                        });
                    }
                    _ => { total_empty += 1; }
                }
            }

            // 第二步：对已收集的消息进行搜索过滤
            let mut all_msgs: Vec<crate::kafka::consumer::KafkaMessage> = if let Some(ref term) = search_lower {
                raw_msgs
                    .into_iter()
                    .filter(|msg| {
                        let km = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(term));
                        let vm = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(term));
                        km || vm
                    })
                    .collect()
            } else {
                raw_msgs
            };

            all_msgs.sort_by(|a, b| {
                match (a.timestamp, b.timestamp) {
                    (Some(ta), Some(tb)) => if is_desc { tb.cmp(&ta) } else { ta.cmp(&tb) },
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => if is_desc { b.offset.cmp(&a.offset) } else { a.offset.cmp(&b.offset) },
                }
            });

            all_msgs
        }).await.map_err(|e| AppError::Internal(format!("Join error: {}", e)))?
    };

    tracing::info!("[Local] Fetched {} messages from {} partitions in {:?}",
        messages.len(), partition_count, start_time_total.elapsed());

    Ok(messages)
}

/// 远程集群优化版 - 单consumer多分区，复用连接
async fn fetch_messages_remote(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<crate::kafka::consumer::KafkaMessage>> {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::Message;
    use rdkafka::TopicPartitionList;
    use rdkafka::ClientConfig;
    use std::time::Duration;
    use std::collections::HashMap;

    let start_time_total = std::time::Instant::now();
    let topic_owned = topic.to_string();
    let brokers_owned = brokers.to_string();
    let search_lower = search.as_ref().map(|s| s.to_lowercase());
    let fetch_mode_owned = fetch_mode.map(|s| s.to_string());
    let sort_owned = sort.map(|s| s.to_string());
    // 只有指定了特定分区时，才使用 offset
    let offset_owned = if partition.is_some() { offset } else { None };

    let messages = tokio::task::spawn_blocking(move || {
        // 第一步：创建consumer并获取分区信息
        let mut cfg = ClientConfig::new();
        cfg.set("bootstrap.servers", &brokers_owned);
        cfg.set("group.id", "kafka-manager-query");
        cfg.set("enable.auto.commit", "false");
        cfg.set("auto.offset.reset", "earliest");
        // 优化：增大批量大小减少RTT
        cfg.set("fetch.min.bytes", "50000");          // 50KB
        cfg.set("fetch.wait.max.ms", "100");
        cfg.set("fetch.max.bytes", "52428800");       // 50MB
        cfg.set("max.partition.fetch.bytes", "5242880"); // 5MB per partition
        cfg.set("socket.nagle.disable", "true");
        cfg.set("session.timeout.ms", "30000");
        cfg.set("socket.connection.setup.timeout.ms", "10000");
        // 强制使用 IPv4，避免 IPv6 连接问题
        cfg.set("broker.address.family", "v4");

        let consumer: BaseConsumer<DefaultConsumerContext> = cfg.create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        // 获取分区列表
        let partitions: Vec<i32> = if partition.is_none() {
            let metadata = consumer.fetch_metadata(Some(&topic_owned), Duration::from_millis(1000))
                .map_err(|e| AppError::Internal(format!("Failed to fetch metadata: {}", e)))?;
            metadata.topics().first()
                .map(|t| t.partitions().iter().map(|p| p.id()).collect())
                .unwrap_or_else(|| vec![0])
        } else {
            vec![partition.unwrap_or(0)]
        };

        let partition_count = partitions.len();

        // 第二步：为每个分区计算起始offset
        let mut tpl = TopicPartitionList::new();

        // 优化：首先批量获取所有分区的 watermarks（单次调用更高效）
        // 注意：这里不能并行化，因为 consumer 不是线程安全的
        let mut watermark_cache: std::collections::HashMap<i32, (i64, i64)> = std::collections::HashMap::new();

        // 只在需要 newest/oldest 模式时预获取 watermarks
        let need_watermarks = start_time.is_none() && offset_owned.is_none();

        if need_watermarks {
            for &part_id in &partitions {
                match consumer.fetch_watermarks(&topic_owned, part_id, Duration::from_millis(5000)) {
                    Ok(w) => { watermark_cache.insert(part_id, w); }
                    Err(e) => {
                        tracing::warn!("Failed to fetch watermarks for partition {}: {}", part_id, e);
                    }
                }
            }
        }

        for &part_id in &partitions {
            let start_offset = if let Some(start_time) = start_time {
                if start_time <= 0 {
                    offset_owned.unwrap_or(0)
                } else {
                    let mut time_tpl = TopicPartitionList::new();
                    time_tpl.add_partition_offset(&topic_owned, part_id, rdkafka::Offset::Offset(start_time)).ok();
                    // 优化：增加超时时间到 3 秒
                    match consumer.offsets_for_times(time_tpl, Duration::from_millis(3000)) {
                        Ok(r) => r.elements_for_topic(&topic_owned)
                            .iter().find(|e| e.partition() == part_id)
                            .and_then(|e| e.offset().to_raw())
                            .unwrap_or_else(|| offset_owned.unwrap_or(0)),
                        Err(_) => offset_owned.unwrap_or(0),
                    }
                }
            } else if (fetch_mode_owned.as_deref() == Some("newest") || fetch_mode_owned.is_none()) && offset_owned.is_none() {
                watermark_cache.get(&part_id)
                    .map(|(low, high)| {
                        if *high > 0 {
                            let latest = high.saturating_sub(1);
                            latest.saturating_sub((max_messages.saturating_sub(1)) as i64).max(*low)
                        } else {
                            *low
                        }
                    })
                    .unwrap_or(0)
            } else if fetch_mode_owned.as_deref() == Some("oldest") && offset_owned.is_none() {
                watermark_cache.get(&part_id).map(|(low, _)| *low).unwrap_or(0)
            } else {
                offset_owned.unwrap_or(0)
            };

            tpl.add_partition_offset(&topic_owned, part_id, rdkafka::Offset::Offset(start_offset))
                .map_err(|e| AppError::Internal(format!("Failed to set offset: {}", e)))?;
        }

        // 第三步：assign所有分区
        consumer.assign(&tpl)
            .map_err(|e| AppError::Internal(format!("Failed to assign partitions: {}", e)))?;

        // 第四步：批量消费消息
        let mut partition_counts: HashMap<i32, usize> = HashMap::with_capacity(partition_count);
        let mut raw_msgs: Vec<crate::kafka::consumer::KafkaMessage> = Vec::with_capacity(max_messages * partition_count);
        let mut total_empty = 0;
        let max_empty_rounds = (partition_count * max_messages).max(20);

        // 计算需要获取的总消息数
        let target_total = max_messages * partition_count;

        // 第一步：收集原始消息（每个分区最多 max_messages 条）
        while raw_msgs.len() < target_total && total_empty < max_empty_rounds {
            match consumer.poll(Duration::from_millis(200)) {
                Some(Ok(msg)) => {
                    let part = msg.partition();

                    // 检查该分区是否已达上限
                    let count = partition_counts.get(&part).copied().unwrap_or(0);
                    if count >= max_messages {
                        // 分区已满，跳过
                        continue;
                    }

                    // 成功获取消息，重置计数器
                    total_empty = 0;

                    let ts = msg.timestamp().to_millis();

                    // 时间范围过滤
                    if let Some(start) = start_time {
                        if let Some(t) = ts { if t < start { continue; } }
                    }
                    if let Some(end) = end_time {
                        if let Some(t) = ts { if t > end { continue; } }
                    }

                    // 解码key/value
                    let key = msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                    let value = msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                    partition_counts.insert(part, count + 1);
                    raw_msgs.push(crate::kafka::consumer::KafkaMessage {
                        partition: part,
                        offset: msg.offset(),
                        key,
                        value,
                        timestamp: ts,
                    });
                }
                Some(Err(_)) => {
                    total_empty += 1;
                }
                None => {
                    total_empty += 1;
                }
            }
        }

        // 第二步：对已收集的消息进行搜索过滤
        let mut all_msgs: Vec<crate::kafka::consumer::KafkaMessage> = if let Some(ref term) = search_lower {
            raw_msgs
                .into_iter()
                .filter(|msg| {
                    let km = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(term));
                    let vm = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(term));
                    km || vm
                })
                .collect()
        } else {
            raw_msgs
        };

        // 第五步：排序
        let is_desc = sort_owned.as_deref() == Some("desc") || (sort_owned.is_none() && fetch_mode_owned.as_deref() != Some("oldest"));
        all_msgs.sort_by(|a, b| {
            match (a.timestamp, b.timestamp) {
                (Some(ta), Some(tb)) => if is_desc { tb.cmp(&ta) } else { ta.cmp(&tb) },
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => if is_desc { b.offset.cmp(&a.offset) } else { a.offset.cmp(&b.offset) },
            }
        });

        tracing::info!("[Remote] Fetched {} messages from {} partitions in {:?}",
            all_msgs.len(), partition_count, start_time_total.elapsed());
        Ok::<Vec<_>, AppError>(all_msgs)
    }).await.map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;

    Ok(messages)
}

async fn handle_message_send(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;
    let key = get_optional_string_param(&body, "key");
    let value = get_string_param(&body, "value")?;
    let partition = get_optional_i32_param(&body, "partition");

    let clients = state.get_clients();
    let producer = clients
        .get_producer(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let (partition_result, offset) = producer
        .send_to_partition(&topic, partition, key.as_deref(), &value)
        .await?;

    Ok(serde_json::json!({
        "partition": partition_result,
        "offset": offset,
    }))
}

async fn handle_message_export(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;
    let partition = get_optional_i32_param(&body, "partition");
    let offset = get_optional_i64_param(&body, "offset");
    let max_messages = get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
    let start_time = get_optional_i64_param(&body, "start_time");
    let end_time = get_optional_i64_param(&body, "end_time");
    let search = get_optional_string_param(&body, "search");
    let fetch_mode = get_optional_string_param(&body, "fetchMode");

    // 首先确保集群客户端已创建（如果未创建则自动创建）
    let config = ensure_cluster_client(&state, &cluster_id).await?;

    let max_msgs = max_messages.unwrap_or(1000);

    // 使用临时 consumer 获取消息（避免连接池状态问题）
    let messages = fetch_messages_with_temp_consumer(
        &config.brokers,
        &topic,
        partition,
        offset,
        max_msgs,
        start_time,
        end_time,
        search,
        fetch_mode.as_deref(),
        Some("asc"), // 导出默认按升序
    )
    .await?;

    let records: Vec<Value> = messages
        .into_iter()
        .map(|msg| {
            serde_json::json!({
                "partition": msg.partition,
                "offset": msg.offset,
                "key": msg.key,
                "value": msg.value,
                "timestamp": msg.timestamp,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "topic": topic,
        "format": "json",
        "messages": records,
        "count": records.len(),
    }))
}

// ==================== Cluster Connection ====================

async fn handle_connection_list(state: AppState) -> Result<Value> {
    let statuses = state.pools.get_all_connections_status().await;

    let connections: Vec<Value> = statuses
        .into_iter()
        .map(|(cluster_id, status)| {
            let (status_str, error_message) = match status {
                crate::pool::ConnectionStatus::Connected => {
                    ("connected".to_string(), None::<String>)
                }
                crate::pool::ConnectionStatus::Disconnected => {
                    ("disconnected".to_string(), None)
                }
                crate::pool::ConnectionStatus::Error(msg) => ("error".to_string(), Some(msg)),
            };

            serde_json::json!({
                "cluster_id": cluster_id,
                "status": status_str,
                "error_message": error_message,
            })
        })
        .collect();

    Ok(serde_json::json!({ "connections": connections }))
}

async fn handle_connection_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let status = state.pools.check_connection(&cluster_id).await;

    match status {
        Some(conn_status) => {
            let (status_str, error_message) = match conn_status {
                crate::pool::ConnectionStatus::Connected => {
                    ("connected".to_string(), None::<String>)
                }
                crate::pool::ConnectionStatus::Disconnected => {
                    ("disconnected".to_string(), None)
                }
                crate::pool::ConnectionStatus::Error(msg) => ("error".to_string(), Some(msg)),
            };

            Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "status": status_str,
                "error_message": error_message,
            }))
        }
        None => Err(AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id))),
    }
}

async fn handle_connection_disconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    // Disconnect from pool
    state.pools.disconnect(&cluster_id).await?;

    // Remove from Kafka clients
    let current_clients = state.get_clients();
    let new_clients = current_clients.without_cluster(&cluster_id);
    state.set_clients(new_clients.into());

    tracing::info!("Disconnected cluster: {}", cluster_id);

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Cluster '{}' disconnected successfully", cluster_id),
    }))
}

async fn handle_connection_reconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_name = get_string_param(&body, "cluster_name")?;

    // Get cluster config from database
    let cluster = ClusterStore::get_by_name(state.db.inner(), &cluster_name)
        .await?
        .ok_or_else(|| {
            AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_name))
        })?;

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    // Reconnect pool
    state
        .pools
        .reconnect(&cluster.name, &config, &state.config.pool)
        .await?;

    // Update Kafka clients
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(&cluster.name, &config)?;
    state.set_clients(new_clients.into());

    tracing::info!("Reconnected cluster: {}", cluster_name);

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Cluster '{}' reconnected successfully", cluster_name),
    }))
}

async fn handle_connection_health_check(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let status = state.pools.check_connection(&cluster_id).await;

    match status {
        Some(conn_status) => {
            let (healthy, status_str, error_message) = match conn_status {
                crate::pool::ConnectionStatus::Connected => {
                    (true, "healthy".to_string(), None::<String>)
                }
                crate::pool::ConnectionStatus::Disconnected => {
                    (false, "disconnected".to_string(), None)
                }
                crate::pool::ConnectionStatus::Error(msg) => {
                    (false, "error".to_string(), Some(msg))
                }
            };

            Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "healthy": healthy,
                "status": status_str,
                "error_message": error_message,
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
                            crate::pool::ConnectionStatus::Connected => {
                                (true, "healthy".to_string(), None::<String>)
                            }
                            crate::pool::ConnectionStatus::Disconnected => {
                                (false, "disconnected".to_string(), None)
                            }
                            crate::pool::ConnectionStatus::Error(msg) => {
                                (false, "error".to_string(), Some(msg))
                            }
                        },
                        None => (false, "error".to_string(), Some("Failed to check connection after adding cluster".to_string())),
                    };

                    Ok(serde_json::json!({
                        "cluster_id": cluster_id,
                        "healthy": healthy,
                        "status": status_str,
                        "error_message": error_message,
                    }))
                }
                Err(e) => Ok(serde_json::json!({
                    "cluster_id": cluster_id,
                    "healthy": false,
                    "status": "error",
                    "error_message": format!("Failed to add cluster: {}", e),
                })),
            }
        }
    }
}

async fn handle_connection_metrics(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    // Get pool status
    let consumer_pool_status = state
        .pools
        .get_consumer_pool(&cluster_id)
        .await
        .map(|pool| (pool.status().size, pool.status().available));
    let producer_pool_status = state
        .pools
        .get_producer_pool(&cluster_id)
        .await
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
    let consumer_pool_status = state
        .pools
        .get_consumer_pool(&cluster_id)
        .await
        .map(|pool| (pool.status().size, pool.status().available));
    let producer_pool_status = state
        .pools
        .get_producer_pool(&cluster_id)
        .await
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

async fn handle_connection_history(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let limit = get_optional_i64_param(&body, "limit").unwrap_or(100);

    let history = ClusterConnectionStore::get_history(state.db.inner(), &cluster_id, limit).await?;

    let entries: Vec<Value> = history
        .into_iter()
        .map(|h| {
            serde_json::json!({
                "status": h.status,
                "error_message": h.error_message,
                "latency_ms": h.latency_ms,
                "checked_at": h.checked_at,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "cluster_id": cluster_id,
        "history": entries,
    }))
}

async fn handle_connection_stats(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let stats = ClusterConnectionStore::get_stats(state.db.inner(), &cluster_id).await?;

    match stats {
        Some(s) => {
            let success_rate = if s.total_checks > 0 {
                (s.successful_checks as f64 / s.total_checks as f64) * 100.0
            } else {
                0.0
            };

            Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "total_checks": s.total_checks,
                "successful_checks": s.successful_checks,
                "failed_checks": s.failed_checks,
                "success_rate": success_rate,
                "avg_latency_ms": s.avg_latency_ms,
                "last_status": s.last_status,
                "last_checked_at": s.last_checked_at,
            }))
        }
        None => Ok(serde_json::json!({
            "cluster_id": cluster_id,
            "total_checks": 0,
            "successful_checks": 0,
            "failed_checks": 0,
            "success_rate": 0.0,
            "avg_latency_ms": None::<f64>,
            "last_status": "unknown",
            "last_checked_at": None::<String>,
        })),
    }
}

async fn handle_connection_batch_disconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_names = get_string_array_param(&body, "cluster_names");
    let mut results = Vec::new();
    let mut successful = 0;

    for cluster_name in &cluster_names {
        match state.pools.disconnect(cluster_name).await {
            Ok(_) => {
                // Remove from Kafka clients
                let current_clients = state.get_clients();
                let new_clients = current_clients.without_cluster(cluster_name);
                state.set_clients(new_clients.into());

                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": true,
                    "message": "Disconnected successfully"
                }));
                successful += 1;
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": false,
                    "message": e.to_string()
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

async fn handle_connection_batch_reconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_names = get_string_array_param(&body, "cluster_names");
    let mut results = Vec::new();

    for cluster_name in &cluster_names {
        // Get cluster config from database
        match ClusterStore::get_by_name(state.db.inner(), cluster_name).await {
            Ok(Some(cluster)) => {
                let config = crate::config::KafkaConfig {
                    brokers: cluster.brokers,
                    request_timeout_ms: cluster.request_timeout_ms as u32,
                    operation_timeout_ms: cluster.operation_timeout_ms as u32,
                };

                // Reconnect pool
                match state
                    .pools
                    .reconnect(cluster_name, &config, &state.config.pool)
                    .await
                {
                    Ok(_) => {
                        // Update Kafka clients
                        let current_clients = state.get_clients();
                        match current_clients.with_added_cluster(cluster_name, &config) {
                            Ok(new_clients) => {
                                state.set_clients(new_clients.into());
                                results.push(serde_json::json!({
                                    "cluster_name": cluster_name,
                                    "success": true,
                                    "message": "Reconnected successfully"
                                }));
                            }
                            Err(e) => {
                                results.push(serde_json::json!({
                                    "cluster_name": cluster_name,
                                    "success": false,
                                    "message": format!("Failed to update client: {}", e)
                                }));
                            }
                        }
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "cluster_name": cluster_name,
                            "success": false,
                            "message": format!("Reconnect failed: {}", e)
                        }));
                    }
                }
            }
            Ok(None) => {
                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": false,
                    "message": "Cluster is not connected"
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "cluster_name": cluster_name,
                    "success": false,
                    "message": format!("Database error: {}", e)
                }));
            }
        }
    }

    let successful = results.iter().filter(|r| r.get("success").and_then(|v| v.as_bool()).unwrap_or(false)).count();

    Ok(serde_json::json!({
        "total": cluster_names.len(),
        "successful": successful,
        "failed": cluster_names.len() - successful,
        "results": results,
    }))
}

// ==================== User ====================

async fn handle_user_list(state: AppState) -> Result<Value> {
    let users = UserStore::list_with_roles(state.db.inner()).await?;

    let user_list: Vec<Value> = users
        .into_iter()
        .map(|u| {
            serde_json::json!({
                "id": u.id,
                "username": u.username,
                "email": u.email,
                "role_id": u.role_id,
                "role_name": u.role_name,
                "is_active": u.is_active,
                "created_at": u.created_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "users": user_list }))
}

async fn handle_user_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let user = UserStore::get_with_role(state.db.inner(), id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

    Ok(serde_json::json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "role_id": user.role_id,
        "role_name": user.role_name,
        "is_active": user.is_active,
        "created_at": user.created_at,
    }))
}

async fn handle_user_create(state: AppState, body: Value) -> Result<Value> {
    let username = get_string_param(&body, "username")?;
    let password = get_string_param(&body, "password")?;
    let email = get_optional_string_param(&body, "email");
    let role_id = get_i64_param(&body, "role_id")?;

    // Hash password
    let password_hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;

    let id = UserStore::create(
        state.db.inner(),
        &username,
        &password_hash,
        email.as_deref(),
        role_id,
    )
    .await?;

    Ok(serde_json::json!({
        "id": id,
        "username": username,
    }))
}

async fn handle_user_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let email = get_optional_string_param(&body, "email");
    let role_id = get_optional_i64_param(&body, "role_id");
    let is_active = get_optional_bool_param(&body, "is_active");

    UserStore::update(
        state.db.inner(),
        id,
        email.as_deref(),
        role_id,
        is_active,
    )
    .await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_user_password_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let old_password = get_string_param(&body, "old_password")?;
    let new_password = get_string_param(&body, "new_password")?;

    // Verify old password
    let user = UserStore::get_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

    let valid = bcrypt::verify(&old_password, &user.password_hash)
        .map_err(|e| AppError::Internal(format!("Failed to verify password: {}", e)))?;

    if !valid {
        return Err(AppError::Unauthorized("Invalid old password".to_string()));
    }

    // Update password
    let new_hash = bcrypt::hash(&new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;

    UserStore::update_password(state.db.inner(), id, &new_hash).await?;

    Ok(serde_json::json!({ "success": true }))
}

// ==================== Role ====================

async fn handle_role_list(state: AppState) -> Result<Value> {
    let roles = RoleStore::list(state.db.inner()).await?;

    let role_list: Vec<Value> = roles
        .into_iter()
        .map(|r| {
            let permissions: Vec<String> =
                serde_json::from_str(&r.permissions).unwrap_or_default();
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "description": r.description,
                "permissions": permissions,
                "created_at": r.created_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "roles": role_list }))
}

async fn handle_role_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let role = RoleStore::get_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))?;

    let permissions: Vec<String> = serde_json::from_str(&role.permissions).unwrap_or_default();

    Ok(serde_json::json!({
        "id": role.id,
        "name": role.name,
        "description": role.description,
        "permissions": permissions,
        "created_at": role.created_at,
    }))
}

async fn handle_role_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let description = get_optional_string_param(&body, "description");
    let permissions = get_string_array_param(&body, "permissions");

    let id = RoleStore::create(
        state.db.inner(),
        &name,
        description.as_deref(),
        &permissions,
    )
    .await?;

    Ok(serde_json::json!({
        "id": id,
        "name": name,
    }))
}

async fn handle_role_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let name = get_optional_string_param(&body, "name");
    let description = get_optional_string_param(&body, "description");
    let permissions = if body.get("permissions").is_some() {
        Some(get_string_array_param(&body, "permissions"))
    } else {
        None
    };

    RoleStore::update(
        state.db.inner(),
        id,
        name.as_deref(),
        description.as_deref(),
        permissions.as_deref(),
    )
    .await?;

    Ok(serde_json::json!({ "success": true }))
}

// ==================== Notification ====================

async fn handle_notification_list(state: AppState) -> Result<Value> {
    let configs = NotificationStore::list(state.db.inner()).await?;
    Ok(serde_json::json!({ "notifications": configs }))
}

async fn handle_notification_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let config = NotificationStore::get_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Notification config {} not found", id)))?;

    Ok(serde_json::json!(config))
}

async fn handle_notification_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let config_type = get_string_param(&body, "config_type")?;
    let webhook_url = get_optional_string_param(&body, "webhook_url");

    // email_recipients is Option<Vec<String>>
    let email_recipients = body.get("email_recipients")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });

    let dingtalk_webhook = get_optional_string_param(&body, "dingtalk_webhook");
    let dingtalk_secret = get_optional_string_param(&body, "dingtalk_secret");
    let wechat_webhook = get_optional_string_param(&body, "wechat_webhook");
    let slack_webhook = get_optional_string_param(&body, "slack_webhook");

    let req = CreateNotificationConfigRequest {
        name,
        config_type,
        webhook_url,
        email_recipients,
        dingtalk_webhook,
        dingtalk_secret,
        wechat_webhook,
        slack_webhook,
    };

    let id = NotificationStore::create(state.db.inner(), &req).await?;

    Ok(serde_json::json!({
        "id": id,
        "success": true
    }))
}

async fn handle_notification_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    NotificationStore::delete(state.db.inner(), id).await?;
    Ok(serde_json::json!({ "success": true }))
}

async fn handle_notification_enable(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    NotificationStore::update(state.db.inner(), id, Some(true)).await?;
    Ok(serde_json::json!({ "success": true }))
}

async fn handle_notification_disable(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    NotificationStore::update(state.db.inner(), id, Some(false)).await?;
    Ok(serde_json::json!({ "success": true }))
}

// ==================== Alert History ====================

async fn handle_alert_history_list(state: AppState, body: Value) -> Result<Value> {
    let limit = get_optional_i64_param(&body, "limit");
    let cluster_id = get_optional_string_param(&body, "cluster_id");
    let rule_id = get_optional_i64_param(&body, "rule_id");
    let severity = get_optional_string_param(&body, "severity");

    let query = crate::db::notification::AlertHistoryQuery {
        limit,
        cluster_id,
        rule_id,
        severity,
    };

    let history = NotificationStore::list_history(state.db.inner(), &query).await?;

    let responses: Vec<Value> = history
        .into_iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id,
                "rule_id": h.rule_id,
                "cluster_id": h.cluster_id,
                "alert_type": h.alert_type,
                "alert_message": h.alert_message,
                "alert_value": h.alert_value,
                "threshold": h.threshold,
                "severity": h.severity,
                "notified": h.notified,
                "created_at": h.created_at
            })
        })
        .collect();

    Ok(serde_json::json!({ "history": responses }))
}

// ==================== Schema Registry ====================

async fn handle_schema_subjects(body: Value) -> Result<Value> {
    let registry_url = get_string_param(&body, "schema_registry_url")?;

    let client = SchemaRegistryClient::new(&registry_url);
    let subjects = client.get_subjects().await?;

    Ok(serde_json::json!({ "subjects": subjects }))
}

async fn handle_schema_versions(body: Value) -> Result<Value> {
    let registry_url = get_string_param(&body, "schema_registry_url")?;
    let subject = get_string_param(&body, "subject")?;

    let client = SchemaRegistryClient::new(&registry_url);
    let versions = client.get_versions(&subject).await?;

    Ok(serde_json::json!({
        "subject": subject,
        "versions": versions,
    }))
}

async fn handle_schema_get(body: Value) -> Result<Value> {
    let registry_url = get_string_param(&body, "schema_registry_url")?;
    let subject = get_string_param(&body, "subject")?;
    let version = get_string_param(&body, "version")?;

    let client = SchemaRegistryClient::new(&registry_url);
    let schema = client.get_schema(&subject, &version).await?;

    Ok(serde_json::json!(schema))
}

async fn handle_schema_register(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let subject = get_string_param(&body, "subject")?;
    let schema = get_string_param(&body, "schema")?;
    let schema_type = get_string_param(&body, "schema_type")?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // Get Schema Registry URL from env or construct from brokers
    let registry_url = std::env::var("SCHEMA_REGISTRY_URL").unwrap_or_else(|_| {
        format!(
            "http://{}:8081",
            config.brokers.split(',').next().unwrap_or("localhost")
        )
    });

    let client = SchemaRegistryClient::new(&registry_url);
    let id = client
        .register_schema(&subject, &schema, &schema_type)
        .await?;

    Ok(serde_json::json!({
        "id": id,
        "subject": subject,
        "success": true
    }))
}

async fn handle_schema_delete(body: Value) -> Result<Value> {
    let registry_url = get_string_param(&body, "schema_registry_url")?;
    let subject = get_string_param(&body, "subject")?;

    let client = SchemaRegistryClient::new(&registry_url);
    let deleted_versions = client.delete_subject(&subject).await?;

    Ok(serde_json::json!({
        "deleted_versions": deleted_versions,
        "subject": subject,
        "success": true
    }))
}

async fn handle_schema_version_delete(body: Value) -> Result<Value> {
    let registry_url = get_string_param(&body, "schema_registry_url")?;
    let subject = get_string_param(&body, "subject")?;
    let version = get_string_param(&body, "version")?;

    let client = SchemaRegistryClient::new(&registry_url);
    let deleted_version = client.delete_schema_version(&subject, &version).await?;

    Ok(serde_json::json!({
        "deleted_version": deleted_version,
        "subject": subject,
        "success": true
    }))
}

async fn handle_schema_compatibility_level(body: Value) -> Result<Value> {
    let registry_url = get_string_param(&body, "schema_registry_url")?;

    let client = SchemaRegistryClient::new(&registry_url);

    // If level is provided, update it; otherwise, get it
    if let Some(level) = get_optional_string_param(&body, "level") {
        client.update_compatibility_level(&level).await?;
        Ok(serde_json::json!({
            "level": level,
            "success": true
        }))
    } else {
        let level = client.get_compatibility_level().await?;
        Ok(serde_json::json!({ "level": level }))
    }
}

// ==================== Settings ====================

async fn handle_settings_get(state: AppState, body: Value) -> Result<Value> {
    let keys = get_string_array_param(&body, "keys");

    let settings = if keys.is_empty() {
        // Get all settings
        let all: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM user_settings ORDER BY key"
        )
        .fetch_all(state.db.inner())
        .await?;
        all.into_iter()
            .map(|(k, v)| serde_json::json!({ "key": k, "value": v }))
            .collect()
    } else {
        // Get specified settings
        let mut result = Vec::new();
        for key in keys {
            if let Some(value) = SettingStore::get(state.db.inner(), &key).await? {
                result.push(serde_json::json!({ "key": key, "value": value }));
            }
        }
        result
    };

    Ok(serde_json::json!({ "settings": settings }))
}

async fn handle_settings_update(state: AppState, body: Value) -> Result<Value> {
    let key = get_string_param(&body, "key")?;
    let value = get_string_param(&body, "value")?;

    SettingStore::set(state.db.inner(), &key, &value).await?;

    Ok(serde_json::json!({
        "key": key,
        "value": value,
    }))
}

// ==================== Cluster Stats/Monitor ====================

async fn handle_monitor_stats(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // Get all info in blocking thread
    let admin = admin.clone();
    let (cluster_info, topics, partition_count, under_replicated, broker_leader_counts, broker_replica_counts) =
        tokio::task::spawn_blocking(move || {
            let info = admin.get_cluster_info()?;
            let topics = admin.list_topics()?;

            let mut partition_count = 0;
            let mut under_replicated = 0;
            let mut broker_leader_counts: std::collections::HashMap<i32, i32> =
                std::collections::HashMap::new();
            let mut broker_replica_counts: std::collections::HashMap<i32, i32> =
                std::collections::HashMap::new();

            for topic in &topics {
                let topic_info = admin.get_topic_info(topic)?;
                for partition in &topic_info.partitions {
                    partition_count += 1;

                    // Check under replicated
                    if partition.isr.len() < partition.replicas.len() {
                        under_replicated += 1;
                    }

                    // Count leader and replica
                    let leader = partition.leader;
                    *broker_leader_counts.entry(leader).or_insert(0) += 1;
                    for replica in &partition.replicas {
                        *broker_replica_counts.entry(*replica).or_insert(0) += 1;
                    }
                }
            }

            Result::<_>::Ok((info, topics, partition_count, under_replicated, broker_leader_counts, broker_replica_counts))
        }).await.map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;

    // Build broker stats
    let broker_stats: Vec<Value> = cluster_info
        .brokers
        .iter()
        .map(|b| {
            let is_controller = cluster_info.controller_id == Some(b.id);
            serde_json::json!({
                "id": b.id,
                "host": b.host,
                "port": b.port,
                "is_controller": is_controller,
                "leader_partitions": broker_leader_counts.get(&b.id).unwrap_or(&0),
                "replica_partitions": broker_replica_counts.get(&b.id).unwrap_or(&0),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "cluster_id": cluster_id,
        "broker_count": cluster_info.brokers.len() as i32,
        "controller_id": cluster_info.controller_id,
        "topic_count": topics.len() as i32,
        "partition_count": partition_count,
        "under_replicated_partitions": under_replicated,
        "consumer_group_count": 0, // Not implemented in original
        "total_lag": 0,
        "broker_stats": broker_stats,
    }))
}

async fn handle_monitor_info(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let info = tokio::task::spawn_blocking(move || admin.get_cluster_info())
        .await
        .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(serde_json::json!({
        "brokers": info.brokers.iter().map(|b| serde_json::json!({
            "id": b.id,
            "host": b.host,
            "port": b.port,
        })).collect::<Vec<_>>(),
        "controller_id": info.controller_id,
        "cluster_id": info.cluster_id,
        "topic_count": info.topic_count,
        "total_partitions": info.total_partitions,
    }))
}

async fn handle_monitor_metrics(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let metrics = admin.get_cluster_metrics()?;

    Ok(serde_json::json!({
        "broker_count": metrics.broker_count,
        "controller_id": metrics.controller_id,
        "topic_count": metrics.topic_count,
        "partition_count": metrics.partition_count,
        "under_replicated_partitions": metrics.under_replicated_partitions,
    }))
}

async fn handle_monitor_brokers(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let info = tokio::task::spawn_blocking(move || admin.get_cluster_info())
        .await
        .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(serde_json::json!({
        "brokers": info.brokers.iter().map(|b| serde_json::json!({
            "id": b.id,
            "host": b.host,
            "port": b.port,
        })).collect::<Vec<_>>(),
    }))
}

async fn handle_monitor_broker_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let broker_id = get_i32_param(&body, "broker_id")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let broker = tokio::task::spawn_blocking(move || admin.get_broker_info(broker_id))
        .await
        .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(serde_json::json!({
        "id": broker.id,
        "host": broker.host,
        "port": broker.port,
        "is_controller": broker.is_controller,
        "leader_partitions": broker.leader_partitions,
        "replica_partitions": broker.replica_partitions,
    }))
}

// ==================== Topic Template ====================

async fn handle_template_list(state: AppState) -> Result<Value> {
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let templates = store.list().await?;

    let template_list: Vec<Value> = templates
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "num_partitions": t.num_partitions,
                "replication_factor": t.replication_factor,
                "config": t.config_json,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "templates": template_list }))
}

async fn handle_template_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let store = TopicTemplateStore::new(state.db.inner().clone());
    let template = store
        .get(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Template {} not found", id)))?;

    Ok(serde_json::json!({
        "id": template.id,
        "name": template.name,
        "description": template.description,
        "num_partitions": template.num_partitions,
        "replication_factor": template.replication_factor,
        "config": template.config_json,
        "created_at": template.created_at,
        "updated_at": template.updated_at,
    }))
}

async fn handle_template_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let description = get_optional_string_param(&body, "description");
    let num_partitions = get_optional_i32_param(&body, "num_partitions").unwrap_or(3);
    let replication_factor = get_optional_i32_param(&body, "replication_factor").unwrap_or(1);
    let config = get_hashmap_param(&body, "config");

    let req = CreateTopicTemplateRequest {
        name,
        description,
        num_partitions,
        replication_factor,
        config,
    };

    let store = TopicTemplateStore::new(state.db.inner().clone());
    let id = store.create(&req).await?;

    let template = store
        .get(id)
        .await?
        .ok_or_else(|| AppError::Internal("Failed to get created template".to_string()))?;

    Ok(serde_json::json!({
        "id": template.id,
        "name": template.name,
        "description": template.description,
        "num_partitions": template.num_partitions,
        "replication_factor": template.replication_factor,
        "config": template.config_json,
        "created_at": template.created_at,
        "updated_at": template.updated_at,
    }))
}

async fn handle_template_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let name = get_optional_string_param(&body, "name");
    let description = get_optional_string_param(&body, "description");
    let num_partitions = get_optional_i32_param(&body, "num_partitions");
    let replication_factor = get_optional_i32_param(&body, "replication_factor");
    let config = if body.get("config").is_some() {
        Some(get_hashmap_param(&body, "config"))
    } else {
        None
    };

    let req = UpdateTopicTemplateRequest {
        name,
        description,
        num_partitions,
        replication_factor,
        config,
    };

    let store = TopicTemplateStore::new(state.db.inner().clone());
    store.update(id, &req).await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_template_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let store = TopicTemplateStore::new(state.db.inner().clone());
    let deleted = store.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound(format!("Template {} not found", id)));
    }

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_template_presets() -> Result<Value> {
    use crate::db::topic_template::preset_templates::get_preset_templates;

    let presets = get_preset_templates();

    let preset_list: Vec<Value> = presets
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "description": p.description,
                "num_partitions": p.num_partitions,
                "replication_factor": p.replication_factor,
                "config": p.config,
            })
        })
        .collect();

    Ok(serde_json::json!({ "presets": preset_list }))
}

async fn handle_template_create_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic_name = get_string_param(&body, "topic_name")?;
    let template_id = get_optional_i64_param(&body, "template_id");
    let template_name = get_optional_string_param(&body, "template_name");
    let override_config = if body.get("override_config").is_some() {
        Some(get_hashmap_param(&body, "override_config"))
    } else {
        None
    };

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let store = TopicTemplateStore::new(state.db.inner().clone());

    // Get template
    let template = if let Some(id) = template_id {
        store.get(id).await?
    } else if let Some(name) = &template_name {
        store.get_by_name(&name).await?
    } else {
        // Default to default template
        store.get_by_name("default").await?
    }
    .ok_or_else(|| AppError::NotFound("Template not found".to_string()))?;

    // Merge config
    let mut final_config: HashMap<String, String> =
        serde_json::from_str(&template.config_json).unwrap_or_default();
    if let Some(override_config) = override_config {
        for (key, value) in override_config {
            final_config.insert(key, value);
        }
    }

    // Create topic
    admin
        .create_topic(
            &topic_name,
            template.num_partitions,
            template.replication_factor,
            final_config,
        )
        .await?;

    Ok(serde_json::json!({
        "success": true,
        "topic": topic_name,
        "template": template.name,
        "num_partitions": template.num_partitions,
        "replication_factor": template.replication_factor,
    }))
}

// ==================== Tag ====================

async fn handle_tag_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let resource_type = get_string_param(&body, "resource_type")?;
    let resource_name = get_string_param(&body, "resource_name")?;

    let store = TagStore::new(state.db.inner().clone());
    let tags = store
        .get_tags(&cluster_id, &resource_type, &resource_name)
        .await?;

    let tag_list: Vec<Value> = tags
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "tag_key": t.tag_key,
                "tag_value": t.tag_value,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "tags": tag_list }))
}

async fn handle_tag_create(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let resource_type = get_string_param(&body, "resource_type")?;
    let resource_name = get_string_param(&body, "resource_name")?;
    let key = get_string_param(&body, "key")?;
    let value = get_string_param(&body, "value")?;

    let store = TagStore::new(state.db.inner().clone());
    store
        .add_tag(&cluster_id, &resource_type, &resource_name, &key, &value)
        .await?;

    Ok(serde_json::json!({
        "cluster_id": cluster_id,
        "resource_type": resource_type,
        "resource_name": resource_name,
        "key": key,
        "value": value,
        "success": true,
    }))
}

async fn handle_tag_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let resource_type = get_string_param(&body, "resource_type")?;
    let resource_name = get_string_param(&body, "resource_name")?;
    let key = get_string_param(&body, "key")?;

    let store = TagStore::new(state.db.inner().clone());
    let deleted = store
        .delete_tag(&cluster_id, &resource_type, &resource_name, &key)
        .await?;

    if !deleted {
        return Err(AppError::NotFound(format!("Tag '{}' not found", key)));
    }

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_tag_topics(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let resource_type = get_optional_string_param(&body, "resource_type");
    let resource_name = get_optional_string_param(&body, "resource_name");

    let store = TagStore::new(state.db.inner().clone());

    // If no filter, return empty list
    let Some(resource_type) = resource_type else {
        return Ok(serde_json::json!({ "tags": [] }));
    };
    let Some(resource_name) = resource_name else {
        return Ok(serde_json::json!({ "tags": [] }));
    };

    let tags = store
        .get_tags(
            &cluster_id,
            &resource_type,
            &resource_name,
        )
        .await?;

    let tag_list: Vec<Value> = tags
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "tag_key": t.tag_key,
                "tag_value": t.tag_value,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "tags": tag_list }))
}

async fn handle_tag_keys(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let resource_type = get_optional_string_param(&body, "resource_type");

    let store = TagStore::new(state.db.inner().clone());
    let keys = store
        .get_all_tag_keys(&cluster_id, resource_type.as_deref())
        .await?;

    Ok(serde_json::json!({ "keys": keys }))
}

async fn handle_tag_values(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let key = get_string_param(&body, "key")?;
    let resource_type = get_optional_string_param(&body, "resource_type")
        .unwrap_or_else(|| "topic".to_string());

    let store = TagStore::new(state.db.inner().clone());
    let values = store
        .get_tag_values(&cluster_id, &resource_type, &key)
        .await?;

    Ok(serde_json::json!({ "values": values }))
}

async fn handle_tag_filter(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let resource_type = get_string_param(&body, "resource_type")?;
    let tag_key = get_string_param(&body, "tag_key")?;
    let tag_value = get_optional_string_param(&body, "tag_value");

    let store = TagStore::new(state.db.inner().clone());
    let resources = store
        .filter_by_tag(&cluster_id, &resource_type, &tag_key, tag_value.as_deref())
        .await?;

    Ok(serde_json::json!({ "resources": resources }))
}

async fn handle_tag_batch_update(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let resource_type = get_string_param(&body, "resource_type")?;
    let resource_name = get_string_param(&body, "resource_name")?;

    let tags = body
        .get("tags")
        .and_then(|v| v.as_object())
        .ok_or_else(|| AppError::BadRequest("Missing tags object".to_string()))?;

    let tag_map: HashMap<String, String> = tags
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect();

    let store = TagStore::new(state.db.inner().clone());
    store
        .batch_update_tags(&cluster_id, &resource_type, &resource_name, &tag_map)
        .await?;

    Ok(serde_json::json!({ "success": true }))
}

// ==================== Audit Log ====================

async fn handle_audit_log_list(state: AppState, body: Value) -> Result<Value> {
    let limit = get_optional_i64_param(&body, "limit").unwrap_or(100);
    let offset = get_optional_i64_param(&body, "offset").unwrap_or(0);
    let action = get_optional_string_param(&body, "action");
    let cluster_id = get_optional_string_param(&body, "cluster_id");
    let status = get_optional_i64_param(&body, "status").map(|v| v as i32);

    let total = AuditLogStore::count(
        state.db.inner(),
        action.as_deref(),
        cluster_id.as_deref(),
        status,
    )
    .await?;

    let logs = AuditLogStore::list(
        state.db.inner(),
        limit,
        offset,
        action.as_deref(),
        cluster_id.as_deref(),
        status,
    )
    .await?;

    use chrono::{DateTime, Utc};

    let log_infos: Vec<Value> = logs
        .into_iter()
        .map(|log| {
            // Parse timestamp
            let timestamp = DateTime::parse_from_rfc3339(&log.timestamp)
                .unwrap_or_else(|_| DateTime::from_timestamp(0, 0).expect("valid epoch timestamp").into())
                .with_timezone(&Utc);

            serde_json::json!({
                "id": log.id,
                "timestamp": timestamp,
                "method": log.method,
                "path": log.path,
                "cluster_id": log.cluster_id,
                "resource": log.resource,
                "action": log.action,
                "api_key": log.api_key,
                "status": log.status,
                "duration_ms": log.duration_ms,
                "client_ip": log.client_ip,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "logs": log_infos,
        "total": total,
        "limit": limit,
        "offset": offset,
    }))
}

// ==================== Auth ====================

async fn handle_auth_api_keys(state: AppState) -> Result<Value> {
    let keys = ApiKeyStore::list(state.db.inner()).await?;

    let key_infos: Vec<Value> = keys
        .into_iter()
        .map(|k| {
            serde_json::json!({
                "id": k.id,
                "key_prefix": k.key_prefix,
                "name": k.name,
                "created_at": k.created_at,
                "expires_at": k.expires_at,
                "is_active": k.is_active,
            })
        })
        .collect();

    Ok(serde_json::json!({ "keys": key_infos }))
}

async fn handle_auth_api_key_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_optional_string_param(&body, "name");
    let expires_in_days = get_optional_i64_param(&body, "expires_in_days");

    // Generate new API Key
    let key = ApiKeyStore::generate_key();
    let key_prefix = key[..7].to_string();
    let key_hash = ApiKeyStore::hash_key(&key);

    // Calculate expiration time
    let expires_at = if let Some(days) = expires_in_days {
        let expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::days(days))
            .ok_or_else(|| AppError::Internal("Invalid date calculation".to_string()))?
            .to_rfc3339();
        Some(expires)
    } else {
        None
    };

    // Store in database
    let record = ApiKeyStore::create(
        state.db.inner(),
        &key_hash,
        &key_prefix,
        name.as_deref(),
        expires_at.as_deref(),
    )
    .await?;

    Ok(serde_json::json!({
        "id": record.id,
        "key": key,
        "key_prefix": record.key_prefix,
        "name": record.name,
        "created_at": record.created_at,
        "expires_at": record.expires_at,
    }))
}

async fn handle_auth_api_key_revoke(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let deleted = ApiKeyStore::delete(state.db.inner(), id).await?;

    Ok(serde_json::json!({ "success": deleted }))
}

// ==================== Additional Topic Handlers ====================

async fn handle_topic_saved(state: AppState, body: Value) -> Result<Value> {
    use crate::db::topic::TopicStore;

    let cluster_id = get_string_param(&body, "cluster_id")?;

    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let topic_names: Vec<String> = topics.into_iter().map(|t| t.topic_name).collect();

    Ok(serde_json::json!({ "topics": topic_names }))
}

// ==================== Consumer Lag Handlers ====================

async fn handle_consumer_lag_get(state: AppState, body: Value) -> Result<Value> {
    use crate::kafka::offset::KafkaOffsetManager;

    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // 获取所有 consumer groups
    let groups = admin.list_consumer_groups(&config)?;

    let offset_manager = KafkaOffsetManager::new(&config);
    let mut consumer_group_lags = Vec::new();
    let mut total_lag = 0i64;

    for group in &groups {
        // 获取该 group 在指定 topic 上的 offset
        let offsets = offset_manager.get_consumer_group_offsets(
            &config,
            &group.name,
            Some(&topic),
        )?;

        if offsets.is_empty() {
            // 该 group 没有消费过这个 topic
            continue;
        }

        let group_lag: i64 = offsets.iter().map(|o| o.lag).sum();
        total_lag += group_lag;

        let partitions: Vec<Value> = offsets
            .into_iter()
            .map(|o| {
                serde_json::json!({
                    "partition": o.partition,
                    "current_offset": o.current_offset,
                    "log_end_offset": o.log_end_offset,
                    "lag": o.lag,
                    "state": if o.current_offset >= 0 { "Active" } else { "Empty" },
                })
            })
            .collect();

        consumer_group_lags.push(serde_json::json!({
            "name": group.name,
            "total_lag": group_lag,
            "partitions": partitions,
        }));
    }

    // 按 lag 降序排序
    consumer_group_lags.sort_by(|a: &Value, b: &Value| {
        let a_lag = a.get("total_lag").and_then(|v| v.as_i64()).unwrap_or(0);
        let b_lag = b.get("total_lag").and_then(|v| v.as_i64()).unwrap_or(0);
        b_lag.cmp(&a_lag)
    });

    Ok(serde_json::json!({
        "topic": topic,
        "total_lag": total_lag,
        "consumer_groups": consumer_group_lags,
    }))
}

async fn handle_consumer_lag_history(state: AppState, body: Value) -> Result<Value> {
    use crate::kafka::throughput::KafkaThroughputCalculator;

    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let calculator = KafkaThroughputCalculator::new(&config);
    let snapshot = calculator.get_topic_consumer_lag_snapshot(&config, &topic, &admin)?;

    // 获取当前时间戳
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("SystemTime before UNIX epoch")
        .as_millis() as i64;

    let groups: Vec<Value> = snapshot
        .consumer_groups
        .into_iter()
        .map(|g| {
            let partitions: Vec<Value> = g
                .partitions
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "partition": p.partition,
                        "current_offset": p.current_offset,
                        "latest_offset": p.log_end_offset,
                        "lag": p.lag,
                    })
                })
                .collect();

            serde_json::json!({
                "name": g.group_name,
                "total_lag": g.total_lag,
                "partitions": partitions,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "topic": snapshot.topic,
        "total_lag": snapshot.total_lag,
        "groups": groups,
        "timestamp": now,
    }))
}
