/// 统一 API 路由处理器
/// 所有 API 请求都通过 POST /api 发送，method 放在 header 中，参数放在 body 中

use crate::db::cluster::{ClusterStore, CreateClusterRequest, UpdateClusterRequest};
use crate::db::cluster_connection::ClusterConnectionStore;
use crate::db::cluster_group::{ClusterGroupStore, CreateClusterGroupRequest, UpdateClusterGroupRequest};
use crate::db::audit_log::AuditLogStore;
use crate::db::favorite::{
    create_favorite, create_group, delete_favorite, delete_favorite_by_topic,
    delete_group, get_all_favorites_with_groups, get_all_groups_with_count, get_favorite_by_id,
    get_group_by_id, is_topic_favorite, update_favorite, update_group,
    CreateFavoriteRequest, CreateGroupRequest, UpdateFavoriteRequest, UpdateGroupRequest,
};
use crate::db::settings::SettingStore;
use crate::db::topic::TopicStore;
use crate::db::topic_template::{
    CreateTopicTemplateRequest, TopicTemplateStore,
    UpdateTopicTemplateRequest,
};
use crate::error::{AppError, Result};
use crate::kafka::offset::KafkaOffsetManager;
use crate::kafka::throughput::KafkaThroughputCalculator;
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response, Sse},
    routing::post,
    Json, Router,
};
use axum::response::sse::Event;
use tokio_stream::wrappers::ReceiverStream;
use serde_json::Value;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::cmp::Reverse;

// 创建统一路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handle_unified_request))
        .route("/stream", post(handle_stream_request))
}

// Helper functions for parameter extraction
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

// Helper function for long text parameters (e.g., style_json, config JSON)
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

/// 获取集群 ID 参数，带格式验证
fn get_cluster_id_param(body: &Value) -> Result<String> {
    let cluster_id = get_string_param(body, "cluster_id")?;
    // 集群 ID 只能包含字母、数字、连字符和下划线
    if !cluster_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::BadRequest("Cluster ID can only contain letters, numbers, hyphens, and underscores".to_string()));
    }
    Ok(cluster_id)
}

/// 获取 Topic 名称参数，带格式验证
fn get_topic_name_param(body: &Value) -> Result<String> {
    let topic_name = get_string_param(body, "topic_name")?;
    // Topic 名称验证：不能包含某些特殊字符
    if topic_name.contains(' ') || topic_name.contains('"') || topic_name.contains(',') {
        return Err(AppError::BadRequest("Topic name cannot contain spaces, quotes, or commas".to_string()));
    }
    Ok(topic_name)
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
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
            Err(e) => {
                tracing::warn!("Database error fetching cluster '{}': {} (attempt {}), retrying...", cluster_id, e, attempt + 1);
                last_db_error = Some(e);
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(50)).await;
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
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
            Err(e) => {
                tracing::warn!("Database error fetching cluster '{}': {} (attempt {}), retrying...", cluster_id, e, attempt + 1);
                last_db_error = Some(e);
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(50)).await;
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
#[allow(dead_code)]
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

/// Get an optional i64 parameter that can be null (returns Some(None) for null, None for missing)
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

/// SSE 流式请求处理器
async fn handle_stream_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response> {
    // Get method from header
    let method = headers
        .get("X-API-Method")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing X-API-Method header".to_string()))?;

    match method {
        "message.list" => handle_message_list_stream(state, body).await,
        _ => Err(AppError::BadRequest(format!("Method '{}' does not support streaming", method))),
    }
}

/// 使用 SSE 流式返回消息列表
async fn handle_message_list_stream(state: AppState, body: Value) -> Result<Response> {
    use std::result::Result as StdResult;

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

    // 首先确保集群客户端已创建
    let config = ensure_cluster_client(&state, &cluster_id).await?;
    let max_msgs = limit.or(max_messages).unwrap_or(100);

    // 创建 SSE 事件流
    let (tx, rx) = mpsc::channel::<StdResult<Event, std::convert::Infallible>>(100);
    let brokers = config.brokers.clone();

    // 在后台任务中执行消息获取和流式发送
    tokio::spawn(async move {
        match fetch_messages_streaming_sse(
            &brokers,
            &topic,
            partition,
            offset,
            max_msgs,
            start_time,
            end_time,
            search,
            fetch_mode.as_deref(),
            sort.as_deref(),
            tx.clone(),
        ).await {
            Ok(_) => {
                // 发送完成标记
                let _ = tx.send(Ok(Event::default().event("complete").data("{}"))).await;
            }
            Err(e) => {
                // 发送错误
                let error_json = serde_json::json!({"error": e.to_string()}).to_string();
                let _ = tx.send(Ok(Event::default().event("error").data(error_json))).await;
            }
        }
    });

    // 返回 SSE 响应
    let stream = ReceiverStream::new(rx);
    Ok(Sse::new(stream).into_response())
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
        "cluster.test_config" => handle_cluster_test_with_config(state, body).await,
        "cluster.stats" => handle_cluster_stats(state, body).await,

        // Cluster Group
        "cluster_group.list" => handle_cluster_group_list(state).await,
        "cluster_group.get" => handle_cluster_group_get(state, body).await,
        "cluster_group.create" => handle_cluster_group_create(state, body).await,
        "cluster_group.update" => handle_cluster_group_update(state, body).await,
        "cluster_group.delete" => handle_cluster_group_delete(state, body).await,
        "cluster_group.clusters" => handle_cluster_group_clusters(state, body).await,
        "cluster_group.assign_cluster" => handle_cluster_group_assign_cluster(state, body).await,

        // Topic
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

        // Settings
        "settings.get" => handle_settings_get(state, body).await,
        "settings.update" => handle_settings_update(state, body).await,

        // JSON Highlight Templates
        "json_highlight.list" => handle_json_highlight_list(state).await,
        "json_highlight.get_current" => handle_json_highlight_get_current(state).await,
        "json_highlight.set_current" => handle_json_highlight_set_current(state, body).await,
        "json_highlight.create" => handle_json_highlight_create(state, body).await,
        "json_highlight.update" => handle_json_highlight_update(state, body).await,
        "json_highlight.delete" => handle_json_highlight_delete(state, body).await,

        // Topic Template
        "template.list" => handle_template_list(state).await,
        "template.get" => handle_template_get(state, body).await,
        "template.create" => handle_template_create(state, body).await,
        "template.update" => handle_template_update(state, body).await,
        "template.delete" => handle_template_delete(state, body).await,
        "template.presets" => handle_template_presets().await,
        "template.create_topic" => handle_template_create_topic(state, body).await,

        // Audit Log
        "audit_log.list" => handle_audit_log_list(state, body).await,

        // Favorite
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
                "group_id": c.group_id,
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
        "group_id": cluster.group_id,
        "created_at": cluster.created_at,
        "updated_at": cluster.updated_at,
    }))
}

async fn handle_cluster_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let brokers = get_string_param(&body, "brokers")?;
    let request_timeout_ms = get_optional_i64_param(&body, "request_timeout_ms").unwrap_or(5000);
    let operation_timeout_ms = get_optional_i64_param(&body, "operation_timeout_ms").unwrap_or(5000);
    let group_id = get_optional_i64_param(&body, "group_id");

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
        group_id,
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
        "group_id": cluster.group_id,
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
    let group_id = get_nullable_i64_param(&body, "group_id");

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
        group_id,
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
        "group_id": cluster.group_id,
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

/// Test cluster connection with temporary configuration (without saving to database)
async fn handle_cluster_test_with_config(_state: AppState, body: Value) -> Result<Value> {
    let brokers = get_string_param(&body, "brokers")?;
    let request_timeout_ms = get_optional_i64_param(&body, "request_timeout_ms").unwrap_or(5000);
    let operation_timeout_ms = get_optional_i64_param(&body, "operation_timeout_ms").unwrap_or(5000);

    use crate::config::KafkaConfig;
    use crate::kafka::KafkaAdmin;

    let config = KafkaConfig {
        brokers,
        request_timeout_ms: request_timeout_ms as u32,
        operation_timeout_ms: operation_timeout_ms as u32,
    };

    match KafkaAdmin::new(&config) {
        Ok(_) => Ok(serde_json::json!({ "success": true })),
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
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

    // Clone clients for background sync before moving
    let clients_for_sync = new_clients.clone();

    // Update the state with new clients
    state.set_clients(new_clients);

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

/// Fetch topics with cluster info from all clusters
async fn handle_topic_list_with_cluster(state: AppState, body: Value) -> Result<Value> {
    use crate::db::cluster::ClusterStore;

    // Use Vec of {name, cluster} objects
    #[derive(serde::Serialize)]
    struct TopicWithCluster {
        name: String,
        cluster: String,
    }

    // Check if cluster_ids array is provided (for multi-cluster selection)
    let cluster_ids: Option<Vec<String>> = body.get("cluster_ids").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    });

    // Check if a specific cluster_id is provided (single cluster, for backward compatibility)
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Get pagination parameters (default: offset=0, limit=10000)
    let offset = body.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(10000) as usize;

    // Get search query parameter
    let search_query = body.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Determine which clusters to fetch topics from (for non-search mode)
    let clusters_to_fetch: Vec<(i64, String)> = if let Some(ref ids) = cluster_ids {
        // Multi-cluster selection: fetch topics from specified cluster IDs
        if ids.is_empty() {
            // Empty array means "all clusters"
            ClusterStore::list(state.db.inner()).await.ok().unwrap_or_default()
                .into_iter().map(|c| (c.id, c.name)).collect()
        } else {
            // Fetch only specified clusters
            let mut result = Vec::new();
            for id in ids {
                if let Ok(Some(cluster)) = ClusterStore::get_by_name(state.db.inner(), id).await {
                    result.push((cluster.id, cluster.name.clone()));
                }
            }
            result
        }
    } else if let Some(ref id) = cluster_id {
        // Single cluster selection (backward compatibility)
        if let Ok(Some(cluster)) = ClusterStore::get_by_name(state.db.inner(), id).await {
            vec![(cluster.id, cluster.name.clone())]
        } else {
            vec![]
        }
    } else {
        // No cluster selection - fetch from all clusters
        ClusterStore::list(state.db.inner()).await.ok().unwrap_or_default()
            .into_iter().map(|c| (c.id, c.name)).collect()
    };

    // If search query is provided, use database search with filter
    if let Some(query) = search_query.filter(|q| !q.is_empty()) {
        // Convert cluster_ids for search: empty or None means all clusters
        let search_cluster_ids: Vec<String> = cluster_ids
            .as_ref()
            .map(|ids| {
                if ids.is_empty() {
                    Vec::new() // Empty means all clusters
                } else {
                    ids.clone()
                }
            })
            .unwrap_or_default();

        let (topics, total) = TopicStore::search_topics_with_filter(
            state.db.inner(),
            &query,
            &search_cluster_ids,
            offset as u32,
            limit as u32,
        ).await?;

        let all_topics: Vec<TopicWithCluster> = topics
            .into_iter()
            .map(|t| TopicWithCluster {
                name: t.topic_name,
                cluster: t.cluster_id,
            })
            .collect();

        let end = (offset + limit).min(total as usize);
        let has_more = end < total as usize;

        return Ok(serde_json::json!({
            "topics": all_topics,
            "total": total,
            "offset": offset,
            "limit": limit,
            "has_more": has_more
        }));
    }

    // No search query - use original logic
    let mut all_topics: Vec<TopicWithCluster> = Vec::new();

    for (_cluster_id, cluster_name) in &clusters_to_fetch {
        if let Ok(topics) = TopicStore::list_by_cluster(state.db.inner(), cluster_name).await {
            for topic in topics {
                all_topics.push(TopicWithCluster {
                    name: topic.topic_name,
                    cluster: cluster_name.clone(),
                });
            }
        }

        // Background sync
        let state_clone = state.clone();
        let cluster_name_clone = cluster_name.clone();
        tokio::spawn(async move {
            let _ = sync_topics_from_kafka(state_clone, &cluster_name_clone).await;
        });
    }

    // Sort by cluster then by name
    all_topics.sort_by(|a, b| {
        a.cluster.cmp(&b.cluster).then(a.name.cmp(&b.name))
    });

    // Apply pagination
    let total = all_topics.len();
    let end = (offset + limit).min(total);
    let paginated_topics = if offset < total {
        all_topics.into_iter().skip(offset).take(limit).collect()
    } else {
        Vec::new()
    };

    Ok(serde_json::json!({
        "topics": paginated_topics,
        "total": total,
        "offset": offset,
        "limit": limit,
        "has_more": end < total
    }))
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
    // 支持 topic 或 name 参数（前端使用 topic）
    let name = get_string_param(&body, "topic")
        .or_else(|_| get_string_param(&body, "name"))?;

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
    // cluster_id 是可选参数，未指定时刷新所有集群
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(String::from);

    // 立即返回成功，后台异步同步
    // 这样不会阻塞后续的 topic.list 请求
    tokio::spawn(async move {
        if let Some(cluster_id) = cluster_id {
            // 刷新指定集群
            refresh_single_cluster(state, cluster_id).await;
        } else {
            // 刷新所有集群
            refresh_all_clusters(state).await;
        }
    });

    // 立即返回成功
    Ok(serde_json::json!({
        "success": true,
        "message": "Topic refresh started in background",
    }))
}

/// 刷新单个集群的 Topic 列表
async fn refresh_single_cluster(state: AppState, cluster_id: String) {
    use crate::db::cluster::ClusterStore;
    use crate::db::topic::TopicStore;

    // 首先尝试从内存中获取 admin 客户端
    let clients = state.get_clients();
    let admin = clients.get_admin(&cluster_id);

    // 如果不存在，尝试从数据库获取集群配置并建立连接
    let admin = if let Some(existing_admin) = admin {
        existing_admin
    } else {
        match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
            Ok(Some(cluster)) => {
                let config = crate::config::KafkaConfig {
                    brokers: cluster.brokers,
                    request_timeout_ms: cluster.request_timeout_ms as u32,
                    operation_timeout_ms: cluster.operation_timeout_ms as u32,
                };

                // 建立连接池连接
                let _ = state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await;

                // 获取新的客户端（包含刚添加的集群）
                let new_clients = state.get_clients();
                match new_clients.get_admin(&cluster_id) {
                    Some(admin) => admin,
                    None => {
                        tracing::error!("Failed to get admin client for cluster '{}'", cluster_id);
                        return;
                    }
                }
            }
            Ok(None) => {
                tracing::error!("Cluster '{}' not found in database", cluster_id);
                return;
            }
            Err(e) => {
                tracing::error!("Failed to get cluster config: {}", e);
                return;
            }
        }
    };

    // Get current topics from Kafka (blocking operation)
    let current_topics = {
        let admin = admin.clone();
        match tokio::task::spawn_blocking(move || admin.list_topics()).await {
            Ok(Ok(topics)) => topics,
            Ok(Err(e)) => {
                tracing::error!("Failed to list topics: {}", e);
                return;
            }
            Err(e) => {
                tracing::error!("Task join error: {}", e);
                return;
            }
        }
    };

    // Sync to database
    match TopicStore::sync_topics(state.db.inner(), &cluster_id, &current_topics).await {
        Ok(sync_result) => {
            tracing::info!("Refreshed topics for cluster '{}': +{} -{}", cluster_id, sync_result.added.len(), sync_result.removed.len());

            // Save new topics to database (blocking operations in spawn_blocking)
            for topic_name in &sync_result.added {
                let topic_name = topic_name.clone();
                let admin = admin.clone();
                let cluster_id = cluster_id.clone();
                let db = state.db.clone();

                let _ = tokio::task::spawn_blocking(move || {
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
                }).await;
            }

            // Update cache
            state.cache.set_topic_list(&cluster_id, current_topics).await;
        }
        Err(e) => {
            tracing::error!("Failed to sync topics: {}", e);
        }
    }
}

/// 刷新所有集群的 Topic 列表
async fn refresh_all_clusters(state: AppState) {
    use crate::db::cluster::ClusterStore;

    // 获取所有集群
    let clusters = match ClusterStore::list(state.db.inner()).await {
        Ok(clusters) => clusters,
        Err(e) => {
            tracing::error!("Failed to list clusters: {}", e);
            return;
        }
    };

    tracing::info!("Refreshing all {} clusters", clusters.len());

    // 并行刷新所有集群
    let mut tasks = Vec::new();
    for cluster in clusters {
        let state = state.clone();
        let cluster_id = cluster.name;
        tasks.push(tokio::spawn(async move {
            refresh_single_cluster(state, cluster_id).await;
        }));
    }

    // 等待所有任务完成
    for task in tasks {
        let _ = task.await;
    }

    tracing::info!("Completed refreshing all clusters");
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

async fn handle_topic_cleanup_orphans(state: AppState, _body: Value) -> Result<Value> {
    use crate::db::cluster::ClusterStore;
    use crate::db::topic::TopicStore;

    // 获取所有有效的集群 ID
    let clusters = ClusterStore::list(state.db.inner()).await?;
    let valid_cluster_ids: Vec<String> = clusters.into_iter().map(|c| c.name).collect();

    // 清理孤儿 Topic
    let removed = TopicStore::cleanup_orphan_topics(state.db.inner(), &valid_cluster_ids).await?;

    Ok(serde_json::json!({
        "success": true,
        "removed": removed,
        "count": removed.len()
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

/// SSE 流式消息获取
/// 并行读取 + 最小堆归并 + 实时 SSE 发送
async fn fetch_messages_streaming_sse(
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
    sse_tx: mpsc::Sender<std::result::Result<Event, std::convert::Infallible>>,
) -> Result<()> {
    use rdkafka::consumer::{Consumer, BaseConsumer};
    use rdkafka::ClientConfig;
    use std::time::Duration;

    let start_time_total = std::time::Instant::now();
    let topic = topic.to_string();
    let brokers = brokers.to_string();
    let search = search.clone();
    let fetch_mode = fetch_mode.map(|s| s.to_string());
    let sort = sort.map(|s| s.to_string());

    tracing::info!("[SSE Stream] Topic: {}, partition: {:?}, max_messages: {}, fetch_mode: {:?}",
                   topic, partition, max_messages, fetch_mode);

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
    let is_desc = sort.as_deref() == Some("desc") || (sort.is_none() && fetch_mode.as_deref() != Some("oldest"));

    tracing::info!("[SSE Stream] {} partitions, {} messages per partition, total target {}",
        partition_count, max_messages, total_target);

    // 发送开始事件
    let start_event = serde_json::json!({
        "event": "start",
        "partitions": partition_count,
        "total_target": total_target
    });
    sse_tx.send(Ok(Event::default().event("start").data(start_event.to_string()))).await
        .map_err(|_| AppError::Internal("SSE channel closed".to_string()))?;

    // 为每个分区创建 channel
    let mut rxs = Vec::new();
    let mut handles = vec![];

    for &part_id in &partitions {
        let (tx, rx) = mpsc::channel::<crate::kafka::consumer::KafkaMessage>(max_messages);
        rxs.push((part_id, rx));

        let brokers = brokers.clone();
        let topic = topic.clone();
        let search = search.clone();
        let fetch_mode = fetch_mode.clone();
        let part_offset = if partition.is_some() { offset } else { None };

        let handle = tokio::spawn(async move {
            fetch_partition_messages_streaming(
                brokers, topic, part_id, max_messages, part_offset,
                start_time, end_time, search, fetch_mode, tx,
            ).await;
        });
        handles.push(handle);
    }

    // 使用最小堆进行流式归并
    let mut heap = BinaryHeap::new();
    let mut completed_partitions = 0;
    let mut sent_count = 0usize;
    const BATCH_SIZE: usize = 500; // 每批发送500条，平衡实时性和性能

    // 从每个分区先取一条消息放入堆
    for (part_id, rx) in &mut rxs {
        match rx.recv().await {
            Some(msg) => {
                heap.push(Reverse(HeapMessage {
                    timestamp: msg.timestamp,
                    offset: msg.offset,
                    message: msg,
                }));
            }
            None => {
                completed_partitions += 1;
                tracing::info!("[SSE Stream] Partition {} completed immediately", part_id);
            }
        }
    }

    // 流式归并并发送
    let mut batch: Vec<Value> = Vec::with_capacity(BATCH_SIZE);

    while completed_partitions < partition_count || !heap.is_empty() {
        // 处理堆中的消息
        if let Some(Reverse(heap_msg)) = heap.pop() {
            let part_id = heap_msg.message.partition;

            // 将消息加入批次
            let msg_json = serde_json::json!({
                "partition": heap_msg.message.partition,
                "offset": heap_msg.message.offset,
                "key": heap_msg.message.key,
                "value": heap_msg.message.value,
                "timestamp": heap_msg.message.timestamp,
            });
            batch.push(msg_json);
            sent_count += 1;

            // 批次满则发送
            if batch.len() >= BATCH_SIZE {
                let batch_event = serde_json::json!({
                    "event": "batch",
                    "messages": batch,
                    "progress": sent_count,
                    "total": total_target
                });
                sse_tx.send(Ok(Event::default().event("batch").data(batch_event.to_string()))).await
                    .map_err(|_| AppError::Internal("SSE channel closed".to_string()))?;
                batch.clear();
            }

            // 从对应分区获取下一条
            if let Some((_, rx)) = rxs.iter_mut().find(|(p, _)| *p == part_id) {
                match rx.recv().await {
                    Some(msg) => {
                        heap.push(Reverse(HeapMessage {
                            timestamp: msg.timestamp,
                            offset: msg.offset,
                            message: msg,
                        }));
                    }
                    None => {
                        completed_partitions += 1;
                        tracing::info!("[SSE Stream] Partition {} completed, total sent: {}", part_id, sent_count);
                    }
                }
            }
        } else {
            // 堆为空但还在接收中，等待消息
            // 给一点时间让消息到达
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    // 发送剩余批次
    if !batch.is_empty() {
        let batch_event = serde_json::json!({
            "event": "batch",
            "messages": batch,
            "progress": sent_count,
            "total": total_target
        });
        sse_tx.send(Ok(Event::default().event("batch").data(batch_event.to_string()))).await
            .map_err(|_| AppError::Internal("SSE channel closed".to_string()))?;
    }

    // 等待所有任务完成
    for handle in handles {
        let _ = handle.await;
    }

    // 如果是降序，需要通知前端反转（因为我们是按升序发送的）
    if is_desc {
        let order_event = serde_json::json!({"event": "order", "sort": "desc"});
        sse_tx.send(Ok(Event::default().event("order").data(order_event.to_string()))).await
            .map_err(|_| AppError::Internal("SSE channel closed".to_string()))?;
    }

    tracing::info!("[SSE Stream] Completed: sent {} messages from {} partitions in {:?}",
        sent_count, partition_count, start_time_total.elapsed());

    Ok(())
}

/// 使用临时 consumer 获取消息（支持过滤）- 统一优化版
/// 分区数>1时使用并行模式，空轮询最多10次×150ms=1.5秒
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
    use rdkafka::consumer::{Consumer, BaseConsumer};
    use rdkafka::ClientConfig;
    use std::time::Duration;

    let start_time_total = std::time::Instant::now();
    let topic = topic.to_string();
    let brokers = brokers.to_string();
    let search = search.clone();
    let fetch_mode = fetch_mode.map(|s| s.to_string());
    let sort = sort.map(|s| s.to_string());

    tracing::info!("[Unified] Topic: {}, partition: {:?}, max_messages: {}, fetch_mode: {:?}",
                   topic, partition, max_messages, fetch_mode);

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

    // 分区数>1时使用并行模式
    let use_parallel = partition_count > 1;
    tracing::info!("[Unified] {} partitions, {} messages per partition, total {}. Mode: {}",
        partition_count, max_messages, total_target,
        if use_parallel { "parallel" } else { "sequential" });

    // 预计算排序方向（在fetch_mode被move之前）
    let is_desc = sort.as_deref() == Some("desc") || (sort.is_none() && fetch_mode.as_deref() != Some("oldest"));

    let messages = if use_parallel {
        // === 并行流式归并模式（分区数>1）===
        let msgs_per_partition = max_messages;
        let partition_has_specific_offset = partition.is_some();
        let partition_offset_val = if partition_has_specific_offset { offset } else { None };

        // 为每个分区创建一个 channel
        let mut rxs = Vec::new();
        let mut handles = vec![];

        for &part_id in &partitions {
            let (tx, rx) = mpsc::channel::<crate::kafka::consumer::KafkaMessage>(msgs_per_partition);
            rxs.push((part_id, rx));

            let brokers = brokers.clone();
            let topic = topic.clone();
            let search = search.clone();
            let fetch_mode = fetch_mode.clone();
            let part_offset = if partition_has_specific_offset { partition_offset_val } else { None };

            let handle = tokio::spawn(async move {
                fetch_partition_messages_streaming(
                    brokers, topic, part_id, msgs_per_partition, part_offset,
                    start_time, end_time, search, fetch_mode, tx,
                ).await;
            });
            handles.push(handle);
        }

        // 使用最小堆进行流式归并
        // 从每个分区先取一条消息放入堆
        let mut heap = BinaryHeap::new();
        let mut completed_partitions = 0;

        for (part_id, rx) in &mut rxs {
            match rx.recv().await {
                Some(msg) => {
                    heap.push(Reverse(HeapMessage {
                        timestamp: msg.timestamp,
                        offset: msg.offset,
                        message: msg,
                    }));
                }
                None => {
                    completed_partitions += 1;
                    tracing::info!("[Merge] Partition {} completed immediately", part_id);
                }
            }
        }

        // 归并排序 - 继续直到所有分区完成且堆为空
        let mut all_msgs: Vec<crate::kafka::consumer::KafkaMessage> = Vec::with_capacity(total_target.min(50000));

        loop {
            // 优先从堆中取出消息，直到达到目标或堆空
            if let Some(Reverse(heap_msg)) = heap.pop() {
                let part_id = heap_msg.message.partition;
                all_msgs.push(heap_msg.message);

                // 如果达到目标，停止从 channel 接收，只继续清空堆
                let stop_receiving = all_msgs.len() >= total_target;

                // 只有在未停止接收时，才从对应分区获取下一条
                if !stop_receiving {
                    if let Some((_, rx)) = rxs.iter_mut().find(|(p, _)| *p == part_id) {
                        match rx.recv().await {
                            Some(msg) => {
                                heap.push(Reverse(HeapMessage {
                                    timestamp: msg.timestamp,
                                    offset: msg.offset,
                                    message: msg,
                                }));
                            }
                            None => {
                                completed_partitions += 1;
                                tracing::info!("[Merge] Partition {} completed, total merged: {}", part_id, all_msgs.len());
                            }
                        }
                    }
                }
            } else if completed_partitions >= partition_count {
                // 堆为空且所有分区已完成
                break;
            } else {
                // 堆为空但分区未完成，等待消息
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        // 等待所有任务完成
        for handle in handles {
            let _ = handle.await;
        }

        all_msgs
    } else {
        // === 单分区串行模式 ===
        tokio::task::spawn_blocking(move || {
            fetch_partition_messages_unified(
                brokers, topic, 0, max_messages, offset,
                start_time, end_time, search, fetch_mode,
            )
        }).await.map_err(|e| AppError::Internal(format!("Join error: {}", e)))?
    };

    // 排序（最小堆归并已经有序，但可能需要根据 is_desc 反转）
    let mut all_msgs = messages;

    // 如果是降序，需要反转
    if is_desc {
        all_msgs.reverse();
    }

    tracing::info!("[Unified] Fetched {} messages from {} partitions in {:?}",
        all_msgs.len(), partition_count, start_time_total.elapsed());

    Ok(all_msgs)
}

/// 检查消息是否匹配搜索条件
fn message_matches_search(
    key_bytes: &Option<Vec<u8>>,
    value_bytes: &Option<Vec<u8>>,
    search_term: &str,
) -> bool {
    let key_str = key_bytes.as_ref().and_then(|k| std::str::from_utf8(k).ok());
    let value_str = value_bytes.as_ref().and_then(|v| std::str::from_utf8(v).ok());
    let key_match = key_str.map_or(false, |k| k.to_lowercase().contains(search_term));
    let value_match = value_str.map_or(false, |v| v.to_lowercase().contains(search_term));
    key_match || value_match
}

/// 统一分区消息获取 - 优化版
/// 空轮询：150ms一次，最多10次（总共1.5秒）
fn fetch_partition_messages_unified(
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

    tracing::info!("[Unified Partition] Starting fetch for partition {} of topic {} (max_messages: {})", partition, topic, max_messages);

    // 创建独立的 consumer
    // 使用唯一的group.id避免并发冲突：包含分区ID和时间戳+随机数
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let unique_group_id = format!("kafka-mgr-{}-{}", partition, unique_suffix);
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers);
    cfg.set("group.id", &unique_group_id);
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("session.timeout.ms", "3000");
    cfg.set("heartbeat.interval.ms", "500");

    // 优化批量fetch配置：大数据量时增加批处理大小
    if max_messages > 1000 {
        cfg.set("fetch.min.bytes", "65536");           // 64KB
        cfg.set("fetch.wait.max.ms", "100");           // 最多等100ms
        cfg.set("fetch.max.bytes", "52428800");        // 50MB
        cfg.set("max.partition.fetch.bytes", "52428800"); // 50MB per partition
    } else {
        cfg.set("fetch.min.bytes", "1");
        cfg.set("fetch.wait.max.ms", "10");
        cfg.set("fetch.max.bytes", "10485760");        // 10MB
        cfg.set("max.partition.fetch.bytes", "10485760"); // 10MB per partition
    }

    cfg.set("socket.nagle.disable", "true");
    cfg.set("socket.receive.buffer.bytes", "262144");  // 256KB
    cfg.set("socket.timeout.ms", "10000");             // socket操作超时10秒
    cfg.set("request.timeout.ms", "30000");            // API请求超时30秒
    cfg.set("enable.partition.eof", "false");
    cfg.set("connections.max.idle.ms", "540000");
    cfg.set("reconnect.backoff.ms", "50");
    cfg.set("reconnect.backoff.max.ms", "500");
    cfg.set("socket.connection.setup.timeout.ms", "3000");
    cfg.set("metadata.max.age.ms", "5000");
    cfg.set("partition.assignment.strategy", "range");
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[Unified Partition] Failed to create consumer for partition {}: {}", partition, e);
            return Vec::new();
        }
    };

    // 计算时间范围 offset 信息
    let time_range = match calculate_partition_offset(&consumer, &topic, partition, max_messages, offset, start_time, end_time, fetch_mode.as_deref()) {
        Ok(tr) => tr,
        Err(e) => {
            tracing::error!("[Unified Partition] Failed to calculate offset for partition {}: {}", partition, e);
            return Vec::new();
        }
    };
    let start_offset = time_range.start_offset;
    let time_range_end = time_range.end_offset;
    tracing::info!("[Unified Partition] Partition {} start_offset: {}, end_offset: {}", partition, start_offset, time_range_end);

    // 提前退出：如果分区完全没有数据（high <= low，表示没有消息）
    if time_range.high_watermark <= time_range.low_watermark {
        tracing::info!("[Unified Partition] Partition {} has no data (high {} <= low {}), skipping",
            partition, time_range.high_watermark, time_range.low_watermark);
        return Vec::new();
    }

    // 提前退出：如果起始offset已经超过或等于high_watermark，说明没有新数据可读
    // 有效数据范围是 [low_watermark, high_watermark)，即 high_watermark > offset >= low_watermark
    if start_offset >= time_range.high_watermark {
        tracing::info!("[Unified Partition] Partition {} start_offset {} >= high_watermark {}, no new data, skipping",
            partition, start_offset, time_range.high_watermark);
        return Vec::new();
    }

    // 提前退出：如果结束offset已经小于等于起始offset，说明没有数据可读
    if time_range_end > 0 && time_range_end < start_offset {
        tracing::info!("[Unified Partition] Partition {} end_offset {} < start_offset {}, no data in range, skipping",
            partition, time_range_end, start_offset);
        return Vec::new();
    }

    // 提取 high_watermark 供后续使用，避免重复获取
    let high_watermark = time_range.high_watermark;

    // 分配到指定分区
    let mut tpl = TopicPartitionList::new();
    let seek_offset = if start_offset < 0 {
        rdkafka::Offset::Beginning
    } else {
        rdkafka::Offset::Offset(start_offset)
    };
    if let Err(e) = tpl.add_partition_offset(&topic, partition, seek_offset) {
        tracing::error!("[Unified Partition] Failed to add partition {}: {}", partition, e);
        return Vec::new();
    }
    if let Err(e) = consumer.assign(&tpl) {
        tracing::error!("[Unified Partition] Failed to assign partition {}: {}", partition, e);
        return Vec::new();
    }

    // 显式 seek 到指定 offset（assign 不会自动 seek）
    if let Err(e) = consumer.seek(&topic, partition, seek_offset, Duration::from_secs(5)) {
        tracing::error!("[Unified Partition] Failed to seek partition {} to offset {:?}: {}",
            partition, seek_offset, e);
        return Vec::new();
    }
    tracing::info!("[Unified Partition] Partition {} seek to offset {:?}", partition, seek_offset);

    // 计算结束 offset
    let end_offset = if time_range_end > 0 {
        Some(time_range_end + 1)
    } else if fetch_mode.as_deref() == Some("newest") {
        // 直接使用已获取的 high_watermark，避免重复请求
        if high_watermark > 0 {
            Some(high_watermark)
        } else {
            None
        }
    } else {
        None
    };

    // 延迟字符串转换：先收集原始字节，需要搜索时再转换
    let search_lower = search.as_ref().map(|s| s.to_lowercase());
    let need_search = search_lower.is_some();

    // 原始消息存储结构
    struct RawMessage {
        partition: i32,
        offset: i64,
        key_bytes: Option<Vec<u8>>,
        value_bytes: Option<Vec<u8>>,
        timestamp: Option<i64>,
    }

    let mut raw_messages: Vec<RawMessage> = Vec::with_capacity(max_messages);
    let mut empty_count = 0;

    // 优化：空轮询150ms一次，最多10次（总共1.5秒）
    const FIRST_POLL_TIMEOUT_MS: u64 = 500;  // 首次poll等待更长时间，因为可能需要建立连接
    const POLL_TIMEOUT_MS: u64 = 150;
    const MAX_EMPTY_POLLS: usize = 10;
    const MAX_POLL_TIME_SECS: u64 = 30;

    let poll_start = std::time::Instant::now();
    let mut got_first = false;

    while raw_messages.len() < max_messages
        && empty_count < MAX_EMPTY_POLLS
        && poll_start.elapsed() < Duration::from_secs(MAX_POLL_TIME_SECS)
    {
        // 自适应poll超时：首次等待500ms，后续150ms
        let poll_timeout = if !got_first {
            Duration::from_millis(FIRST_POLL_TIMEOUT_MS)
        } else {
            Duration::from_millis(POLL_TIMEOUT_MS)
        };

        match consumer.poll(poll_timeout) {
            Some(Ok(msg)) => {
                if !got_first {
                    got_first = true;
                    tracing::info!("[Unified Partition] First message received after {:?}", poll_start.elapsed());
                }
                empty_count = 0;

                let msg_offset = msg.offset();

                // 检查起始 offset - 如果小于 start_offset，说明还没到有效范围，继续
                if msg_offset < start_offset {
                    tracing::debug!("[Unified Partition] Partition {} msg offset {} < start_offset {}, skipping",
                        partition, msg_offset, start_offset);
                    continue;
                }

                // newest 模式下检查是否到达末尾
                if let Some(end) = end_offset {
                    if msg_offset >= end {
                        break;
                    }
                }

                // 检查是否已到达分区末尾（已消费到最后一条消息）
                if msg_offset >= high_watermark - 1 {
                    tracing::info!("[Unified Partition] Reached end of partition {} at offset {} (high_watermark: {})",
                        partition, msg_offset, high_watermark);
                    // 处理完这条消息后退出
                    let ts = msg.timestamp().to_millis();

                    // 时间范围过滤 - 最后一条消息，不符合就直接退出
                    if let Some(start) = start_time {
                        if let Some(t) = ts {
                            if t < start {
                                tracing::info!("[Unified Partition] Last message timestamp {} < start_time {}, stopping",
                                    t, start);
                                break;
                            }
                        }
                    }
                    if let Some(end) = end_time {
                        if let Some(t) = ts {
                            if t > end {
                                tracing::info!("[Unified Partition] Last message timestamp {} > end_time {}, stopping",
                                    t, end);
                                break;
                            }
                        }
                    }

                    let key_bytes = msg.key().map(|k| k.to_vec());
                    let value_bytes = msg.payload().map(|p| p.to_vec());

                    // 搜索过滤
                    if need_search {
                        if let Some(term) = search_lower.as_ref() {
                            if !message_matches_search(&key_bytes, &value_bytes, term) {
                                // 不匹配，直接退出（已经到末尾了）
                                break;
                            }
                        }
                    }

                    raw_messages.push(RawMessage {
                        partition,
                        offset: msg_offset,
                        key_bytes,
                        value_bytes,
                        timestamp: ts,
                    });
                    break; // 到达末尾，立即退出
                }

                let ts = msg.timestamp().to_millis();

                // 时间范围过滤
                if let Some(start) = start_time {
                    if let Some(t) = ts { if t < start { continue; } }
                }
                if let Some(end) = end_time {
                    if let Some(t) = ts { if t > end { continue; } }
                }

                // 延迟转换：只保存字节，需要搜索时再转换
                let key_bytes = msg.key().map(|k| k.to_vec());
                let value_bytes = msg.payload().map(|p| p.to_vec());

                // 如果需要搜索，立即进行过滤（避免保存不需要的消息）
                if need_search {
                    if let Some(term) = search_lower.as_ref() {
                        if !message_matches_search(&key_bytes, &value_bytes, term) {
                            continue; // 不匹配搜索条件，跳过
                        }
                    }
                }

                raw_messages.push(RawMessage {
                    partition,
                    offset: msg.offset(),
                    key_bytes,
                    value_bytes,
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

    tracing::info!("[Unified Partition] Poll loop took {:?}, empty_count={}, messages_collected={}",
        poll_start.elapsed(), empty_count, raw_messages.len());

    // 转换为最终消息格式
    let messages: Vec<crate::kafka::consumer::KafkaMessage> = raw_messages
        .into_iter()
        .map(|raw| crate::kafka::consumer::KafkaMessage {
            partition: raw.partition,
            offset: raw.offset,
            key: raw.key_bytes.and_then(|k| std::str::from_utf8(&k).ok().map(String::from)),
            value: raw.value_bytes.and_then(|v| std::str::from_utf8(&v).ok().map(String::from)),
            timestamp: raw.timestamp,
        })
        .collect();

    tracing::info!("[Unified Partition] Fetched {} messages from partition {}", messages.len(), partition);
    messages
}

/// 流式分区消息获取 - 通过 channel 实时发送消息
/// 支持动态调整空轮询次数
async fn fetch_partition_messages_streaming(
    brokers: String,
    topic: String,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<String>,
    tx: mpsc::Sender<crate::kafka::consumer::KafkaMessage>,
) {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::Message;
    use rdkafka::TopicPartitionList;
    use rdkafka::ClientConfig;
    use std::time::Duration;

    tracing::info!("[Streaming] Starting fetch for partition {} of topic {} (max_messages: {})", partition, topic, max_messages);

    // 使用唯一的group.id避免并发冲突
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let unique_group_id = format!("kafka-mgr-{}-{}", partition, unique_suffix);
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers);
    cfg.set("group.id", &unique_group_id);
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("session.timeout.ms", "3000");
    cfg.set("heartbeat.interval.ms", "500");

    // 优化批量fetch配置
    if max_messages > 1000 {
        cfg.set("fetch.min.bytes", "65536");
        cfg.set("fetch.wait.max.ms", "100");
        cfg.set("fetch.max.bytes", "52428800");
        cfg.set("max.partition.fetch.bytes", "52428800");
    } else {
        cfg.set("fetch.min.bytes", "1");
        cfg.set("fetch.wait.max.ms", "10");
        cfg.set("fetch.max.bytes", "10485760");
        cfg.set("max.partition.fetch.bytes", "10485760");
    }

    cfg.set("socket.nagle.disable", "true");
    cfg.set("socket.receive.buffer.bytes", "262144");
    cfg.set("socket.timeout.ms", "10000");
    cfg.set("request.timeout.ms", "30000");
    cfg.set("enable.partition.eof", "false");
    cfg.set("connections.max.idle.ms", "540000");
    cfg.set("reconnect.backoff.ms", "50");
    cfg.set("reconnect.backoff.max.ms", "500");
    cfg.set("socket.connection.setup.timeout.ms", "3000");
    cfg.set("metadata.max.age.ms", "5000");
    cfg.set("partition.assignment.strategy", "range");
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[Streaming] Failed to create consumer for partition {}: {}", partition, e);
            return;
        }
    };

    // 计算时间范围 offset 信息
    let time_range = match calculate_partition_offset(&consumer, &topic, partition, max_messages, offset, start_time, end_time, fetch_mode.as_deref()) {
        Ok(tr) => tr,
        Err(e) => {
            tracing::error!("[Streaming] Failed to calculate offset for partition {}: {}", partition, e);
            return;
        }
    };
    let start_offset = time_range.start_offset;
    let time_range_end = time_range.end_offset;
    let high_watermark = time_range.high_watermark;

    // 提前退出检查
    if time_range.high_watermark <= time_range.low_watermark {
        tracing::info!("[Streaming] Partition {} has no data, skipping", partition);
        return;
    }
    if start_offset >= time_range.high_watermark {
        tracing::info!("[Streaming] Partition {} start_offset >= high_watermark, no new data", partition);
        return;
    }
    if time_range_end > 0 && time_range_end < start_offset {
        tracing::info!("[Streaming] Partition {} end_offset < start_offset, no data in range", partition);
        return;
    }

    // 分配到指定分区
    let mut tpl = TopicPartitionList::new();
    let seek_offset = if start_offset < 0 {
        rdkafka::Offset::Beginning
    } else {
        rdkafka::Offset::Offset(start_offset)
    };
    if let Err(e) = tpl.add_partition_offset(&topic, partition, seek_offset) {
        tracing::error!("[Streaming] Failed to add partition {}: {}", partition, e);
        return;
    }
    if let Err(e) = consumer.assign(&tpl) {
        tracing::error!("[Streaming] Failed to assign partition {}: {}", partition, e);
        return;
    }
    if let Err(e) = consumer.seek(&topic, partition, seek_offset, Duration::from_secs(5)) {
        tracing::error!("[Streaming] Failed to seek partition {}: {}", partition, e);
        return;
    }

    // 计算结束 offset
    let end_offset = if time_range_end > 0 {
        Some(time_range_end + 1)
    } else if fetch_mode.as_deref() == Some("newest") {
        if high_watermark > 0 {
            Some(high_watermark)
        } else {
            None
        }
    } else {
        None
    };

    // 搜索准备
    let search_lower = search.as_ref().map(|s| s.to_lowercase());
    let need_search = search_lower.is_some();

    let mut sent_count = 0usize;
    let mut empty_count = 0usize;

    // === 动态调整空轮询次数 ===
    // 基础次数 10 次，每 1000 条消息增加 5 次，最多 50 次
    let base_empty_polls = 10usize;
    let additional_polls = (max_messages / 1000) * 5;
    let max_empty_polls = (base_empty_polls + additional_polls).min(50);
    tracing::info!("[Streaming] Partition {} dynamic max_empty_polls: {} (max_messages: {})",
        partition, max_empty_polls, max_messages);

    const FIRST_POLL_TIMEOUT_MS: u64 = 500;
    const POLL_TIMEOUT_MS: u64 = 150;
    const MAX_POLL_TIME_SECS: u64 = 60;

    let poll_start = std::time::Instant::now();
    let mut got_first = false;

    while sent_count < max_messages
        && empty_count < max_empty_polls
        && poll_start.elapsed() < Duration::from_secs(MAX_POLL_TIME_SECS)
    {
        let poll_timeout = if !got_first {
            Duration::from_millis(FIRST_POLL_TIMEOUT_MS)
        } else {
            Duration::from_millis(POLL_TIMEOUT_MS)
        };

        match consumer.poll(poll_timeout) {
            Some(Ok(msg)) => {
                if !got_first {
                    got_first = true;
                    tracing::info!("[Streaming] First message received for partition {} after {:?}", partition, poll_start.elapsed());
                }
                empty_count = 0;

                let msg_offset = msg.offset();

                // 检查起始 offset - 如果小于 start_offset，说明还没到有效范围，继续
                if msg_offset < start_offset {
                    tracing::debug!("[Streaming] Partition {} msg offset {} < start_offset {}, skipping",
                        partition, msg_offset, start_offset);
                    continue;
                }

                // 检查结束 offset - 如果大于等于 end_offset，说明超出范围，退出
                if let Some(end) = end_offset {
                    if msg_offset >= end {
                        tracing::info!("[Streaming] Partition {} msg offset {} >= end_offset {}, stopping",
                            partition, msg_offset, end);
                        break;
                    }
                }

                // 检查是否到达分区末尾
                if msg_offset >= high_watermark - 1 {
                    let ts = msg.timestamp().to_millis();

                    // 时间范围过滤 - 最后一条消息，不符合就直接退出，不再继续 poll
                    if let Some(start) = start_time {
                        if let Some(t) = ts {
                            if t < start {
                                tracing::info!("[Streaming] Partition {} last message timestamp {} < start_time {}, stopping",
                                    partition, t, start);
                                break;
                            }
                        }
                    }
                    if let Some(end) = end_time {
                        if let Some(t) = ts {
                            if t > end {
                                tracing::info!("[Streaming] Partition {} last message timestamp {} > end_time {}, stopping",
                                    partition, t, end);
                                break;
                            }
                        }
                    }

                    let key_bytes = msg.key().map(|k| k.to_vec());
                    let value_bytes = msg.payload().map(|p| p.to_vec());

                    // 搜索过滤
                    if need_search {
                        if let Some(term) = search_lower.as_ref() {
                            if !message_matches_search(&key_bytes, &value_bytes, term) {
                                break;
                            }
                        }
                    }

                    let kafka_msg = crate::kafka::consumer::KafkaMessage {
                        partition,
                        offset: msg_offset,
                        key: key_bytes.and_then(|k| std::str::from_utf8(&k).ok().map(String::from)),
                        value: value_bytes.and_then(|v| std::str::from_utf8(&v).ok().map(String::from)),
                        timestamp: ts,
                    };

                    if tx.send(kafka_msg).await.is_err() {
                        tracing::warn!("[Streaming] Channel closed for partition {}", partition);
                        return;
                    }
                    sent_count += 1;
                    break;
                }

                let ts = msg.timestamp().to_millis();

                // 时间范围过滤
                if let Some(start) = start_time {
                    if let Some(t) = ts {
                        if t < start {
                            continue;  // 时间戳太小，继续 poll 后面的消息
                        }
                    }
                }
                if let Some(end) = end_time {
                    if let Some(t) = ts {
                        if t > end {
                            // 时间戳太大，后面的消息时间戳只会更大，直接退出
                            tracing::info!("[Streaming] Partition {} message timestamp {} > end_time {}, stopping",
                                partition, t, end);
                            break;
                        }
                    }
                }

                let key_bytes = msg.key().map(|k| k.to_vec());
                let value_bytes = msg.payload().map(|p| p.to_vec());

                // 搜索过滤
                if need_search {
                    if let Some(term) = search_lower.as_ref() {
                        if !message_matches_search(&key_bytes, &value_bytes, term) {
                            continue;
                        }
                    }
                }

                let kafka_msg = crate::kafka::consumer::KafkaMessage {
                    partition,
                    offset: msg_offset,
                    key: key_bytes.and_then(|k| std::str::from_utf8(&k).ok().map(String::from)),
                    value: value_bytes.and_then(|v| std::str::from_utf8(&v).ok().map(String::from)),
                    timestamp: ts,
                };

                if tx.try_send(kafka_msg).is_err() {
                    tracing::warn!("[Streaming] Channel closed for partition {}", partition);
                    return;
                }
                sent_count += 1;
            }
            Some(Err(e)) => {
                tracing::warn!("[Streaming] Poll error for partition {}: {}", partition, e);
                empty_count += 1;
            }
            None => {
                empty_count += 1;
            }
        }
    }

    tracing::info!("[Streaming] Partition {} completed: sent {} messages, empty_count={}, elapsed={:?}",
        partition, sent_count, empty_count, poll_start.elapsed());
}

/// Kafka消息包装器，用于堆排序
#[derive(Debug)]
struct HeapMessage {
    timestamp: Option<i64>,
    offset: i64,
    message: crate::kafka::consumer::KafkaMessage,
}

impl Ord for HeapMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 按时间戳排序，时间戳相同则按 offset 排序
        match (self.timestamp, other.timestamp) {
            (Some(a), Some(b)) => a.cmp(&b).then_with(|| self.offset.cmp(&other.offset)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => self.offset.cmp(&other.offset),
        }
    }
}

impl PartialOrd for HeapMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for HeapMessage {}

impl PartialEq for HeapMessage {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && self.offset == other.offset
    }
}

/// 时间范围信息
#[derive(Debug, Clone)]
struct TimeRangeInfo {
    /// 时间范围起始 offset（对应 start_time）
    start_offset: i64,
    /// 时间范围结束 offset（对应 end_time，如果未指定则为 high watermark - 1）
    end_offset: i64,
    /// 分区的 low watermark
    low_watermark: i64,
    /// 分区的 high watermark
    high_watermark: i64,
}

/// 计算分区的时间范围 offset 信息
/// 返回 (start_offset, end_offset) 以及 watermark 信息
fn calculate_time_range_offsets(
    consumer: &rdkafka::consumer::BaseConsumer,
    topic: &str,
    partition: i32,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> Result<TimeRangeInfo> {
    use rdkafka::consumer::Consumer;
    use rdkafka::TopicPartitionList;
    use std::time::Duration;

    // 获取 watermark
    let (low, high) = consumer.fetch_watermarks(topic, partition, Duration::from_secs(5))
        .unwrap_or((0, 0));

    // 处理分区为空的情况（low == high）
    if low >= high {
        tracing::info!("[calculate_time_range_offsets] Partition {} has no data (low={} >= high={}), returning empty range",
            partition, low, high);
        return Ok(TimeRangeInfo {
            start_offset: low,
            end_offset: low,
            low_watermark: low,
            high_watermark: high,
        });
    }

    let high_offset = high - 1; // high > low >= 0, 所以不会溢出

    // 检查时间范围有效性
    if let (Some(start), Some(end)) = (start_time, end_time) {
        if start > end {
            tracing::warn!("[calculate_time_range_offsets] start_time {} > end_time {}, using low watermark", start, end);
            return Ok(TimeRangeInfo {
                start_offset: low,
                end_offset: low,
                low_watermark: low,
                high_watermark: high,
            });
        }
    }

    // 查询 start_time 对应的 offset
    let start_offset = if let Some(start_time) = start_time {
        if start_time > 0 {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(start_time)).ok();
            match consumer.offsets_for_times(tpl, Duration::from_secs(3)) {
                Ok(r) => {
                    let mut found_offset = low;
                    for elem in r.elements_for_topic(topic) {
                        if elem.partition() == partition {
                            if let Some(offset) = elem.offset().to_raw() {
                                // offsets_for_times 返回 -1 表示时间戳晚于所有消息
                                if offset >= 0 {
                                    tracing::info!("[calculate_time_range_offsets] start_time={} -> offset={} on partition {}",
                                        start_time, offset, partition);
                                    // 限制在 [low, high_offset] 范围内
                                    found_offset = offset.clamp(low, high_offset);
                                } else {
                                    // 时间戳晚于所有消息，使用 high_offset
                                    tracing::info!("[calculate_time_range_offsets] start_time={} -> offset=-1 (late), using high_offset={} on partition {}",
                                        start_time, high_offset, partition);
                                    found_offset = high_offset;
                                }
                                break;
                            }
                        }
                    }
                    found_offset
                }
                Err(_) => low,
            }
        } else {
            low
        }
    } else {
        low
    };

    // 查询 end_time 对应的 offset
    let end_offset = if let Some(end_time) = end_time {
        if end_time > 0 {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(end_time)).ok();
            match consumer.offsets_for_times(tpl, Duration::from_secs(3)) {
                Ok(r) => {
                    let mut found_offset = high_offset;
                    for elem in r.elements_for_topic(topic) {
                        if elem.partition() == partition {
                            if let Some(offset) = elem.offset().to_raw() {
                                // offsets_for_times 返回 -1 表示时间戳晚于所有消息
                                if offset >= 0 {
                                    // end_time 对应的 offset 是大于等于该时间的第一条消息
                                    // 所以时间范围的有效结束 offset 是 offset - 1
                                    let effective_end = offset.saturating_sub(1);
                                    tracing::info!("[calculate_time_range_offsets] end_time={} -> raw_offset={}, effective_end={} on partition {}",
                                        end_time, offset, effective_end, partition);
                                    // 限制在 [low, high_offset] 范围内
                                    found_offset = effective_end.clamp(low, high_offset);
                                } else {
                                    // 时间戳晚于所有消息，使用 high_offset
                                    tracing::info!("[calculate_time_range_offsets] end_time={} -> offset=-1 (late), using high_offset={} on partition {}",
                                        end_time, high_offset, partition);
                                    found_offset = high_offset;
                                }
                                break;
                            }
                        }
                    }
                    found_offset
                }
                Err(_) => high_offset,
            }
        } else {
            high_offset
        }
    } else {
        high_offset
    };

    // 最终检查：确保 start_offset <= end_offset
    let final_start = start_offset.min(end_offset);
    let final_end = end_offset.max(start_offset);

    Ok(TimeRangeInfo {
        start_offset: final_start,
        end_offset: final_end,
        low_watermark: low,
        high_watermark: high,
    })
}

/// 计算分区的起始 offset
/// 当同时指定了时间范围和 fetch_mode 时：
/// - oldest: 从时间范围的 start_offset 开始，向前读取
/// - newest: 从时间范围的 end_offset 开始，向后读取（需要 reverse seek）
fn calculate_partition_offset(
    consumer: &rdkafka::consumer::BaseConsumer,
    topic: &str,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    fetch_mode: Option<&str>,
) -> Result<TimeRangeInfo> {
    use rdkafka::consumer::Consumer;
    use std::time::Duration;

    // 如果用户指定了特定 offset，优先使用
    if let Some(off) = offset {
        if off >= 0 {
            tracing::info!("[calculate_partition_offset] Using user-specified offset: {}", off);
            // 获取 watermark 来构建 TimeRangeInfo
            let (low, high) = consumer.fetch_watermarks(topic, partition, Duration::from_secs(5))
                .unwrap_or((0, 0));
            return Ok(TimeRangeInfo {
                start_offset: off,
                end_offset: high.saturating_sub(1).max(0),
                low_watermark: low,
                high_watermark: high,
            });
        }
    }

    // 计算时间范围 offset
    let time_range = calculate_time_range_offsets(consumer, topic, partition, start_time, end_time)?;

    // 如果指定了时间范围，根据 fetch_mode 决定起始位置
    let has_time_range = start_time.is_some() || end_time.is_some();

    if has_time_range {
        match fetch_mode {
            Some("newest") => {
                // newest 模式：从 end_offset 开始，向后读取
                // 需要在时间范围 [start_offset, end_offset] 内从后往前取 max_messages 条
                let range_size = time_range.end_offset.saturating_sub(time_range.start_offset) + 1;
                let messages_to_fetch = (max_messages as i64).min(range_size);
                let actual_start = time_range.end_offset.saturating_sub(messages_to_fetch - 1)
                    .max(time_range.start_offset);

                tracing::info!(
                    "[calculate_partition_offset] fetch_mode=newest with time range: start_offset={}, end_offset={}, range_size={}, messages_to_fetch={}, actual_start={}",
                    time_range.start_offset, time_range.end_offset, range_size, messages_to_fetch, actual_start
                );

                return Ok(TimeRangeInfo {
                    start_offset: actual_start,
                    end_offset: time_range.end_offset,
                    low_watermark: time_range.low_watermark,
                    high_watermark: time_range.high_watermark,
                });
            }
            Some("oldest") | _ => {
                // oldest 模式：从 start_offset 开始，向前读取
                tracing::info!(
                    "[calculate_partition_offset] fetch_mode=oldest with time range: start_offset={}, end_offset={}",
                    time_range.start_offset, time_range.end_offset
                );

                return Ok(TimeRangeInfo {
                    start_offset: time_range.start_offset,
                    end_offset: time_range.end_offset,
                    low_watermark: time_range.low_watermark,
                    high_watermark: time_range.high_watermark,
                });
            }
        }
    }

    // 没有时间范围，使用传统的 fetch_mode 逻辑
    match fetch_mode {
        Some("newest") | None => {
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, high)) if high > 0 => {
                    let latest = high.saturating_sub(1);
                    let start = latest.saturating_sub((max_messages.saturating_sub(1)) as i64).max(low);
                    tracing::info!("[calculate_partition_offset] fetch_mode={:?}, watermarks=({}, {}), start_offset={}, latest={}, max_messages={}",
                                   fetch_mode, low, high, start, latest, max_messages);
                    Ok(TimeRangeInfo {
                        start_offset: start,
                        end_offset: latest,
                        low_watermark: low,
                        high_watermark: high,
                    })
                }
                _ => {
                    tracing::info!("[calculate_partition_offset] watermarks invalid or high=0, using offset 0");
                    Ok(TimeRangeInfo {
                        start_offset: 0,
                        end_offset: 0,
                        low_watermark: 0,
                        high_watermark: 0,
                    })
                }
            }
        }
        Some("oldest") => {
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, high)) => {
                    tracing::info!("[calculate_partition_offset] fetch_mode=oldest, watermark low={}, using offset {}", low, low);
                    Ok(TimeRangeInfo {
                        start_offset: low,
                        end_offset: high.saturating_sub(1).max(0),
                        low_watermark: low,
                        high_watermark: high,
                    })
                }
                Err(_) => Ok(TimeRangeInfo {
                    start_offset: 0,
                    end_offset: 0,
                    low_watermark: 0,
                    high_watermark: 0,
                }),
            }
        }
        _ => Ok(TimeRangeInfo {
            start_offset: 0,
            end_offset: 0,
            low_watermark: 0,
            high_watermark: 0,
        }),
    }
}


async fn handle_message_send(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;
    let key = get_optional_string_param(&body, "key");
    let value = get_string_param(&body, "value")?;
    let partition = get_optional_i32_param(&body, "partition");
    let headers = get_hashmap_param(&body, "headers");

    let clients = state.get_clients();
    let producer = clients
        .get_producer(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let headers_opt = if headers.is_empty() { None } else { Some(&headers) };
    let (partition_result, offset) = producer
        .send_to_partition(&topic, partition, key.as_deref(), &value, headers_opt)
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

    // 轻量级健康检查：只检查内存中的连接池状态，不实际连接 Kafka
    // 避免在页面加载时触发重型 Kafka 元数据获取操作
    let statuses = state.pools.get_all_connections_status().await;

    match statuses.into_iter().find(|(id, _)| id == &cluster_id) {
        Some((_, conn_status)) => {
            let (healthy, status_str, error_message) = match conn_status {
                crate::pool::ConnectionStatus::Connected => {
                    (true, "connected".to_string(), None::<String>)
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
            // 连接池中不存在，返回未找到状态（不重连）
            Ok(serde_json::json!({
                "cluster_id": cluster_id,
                "healthy": false,
                "status": "not_found",
                "error_message": "Cluster not found in connection pool",
            }))
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

// ==================== JSON Highlight Templates ====================

use crate::db::json_highlight::JsonHighlightTemplate;

async fn handle_json_highlight_list(state: AppState) -> Result<Value> {
    let templates: Vec<Value> = JsonHighlightTemplate::get_all_templates(state.db.inner())
        .await?
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "is_builtin": t.is_builtin,
                "style_json": t.style_json,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "templates": templates }))
}

async fn handle_json_highlight_get_current(state: AppState) -> Result<Value> {
    let name = SettingStore::get(state.db.inner(), "ui.json_highlight_template")
        .await?
        .unwrap_or_else(|| "default".to_string());

    Ok(serde_json::json!({ "name": name }))
}

async fn handle_json_highlight_set_current(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;

    SettingStore::set(state.db.inner(), "ui.json_highlight_template", &name).await?;

    Ok(serde_json::json!({ "name": name }))
}

async fn handle_json_highlight_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let description = get_string_param(&body, "description")?;
    let style_json = get_long_string_param(&body, "style_json")?;

    // 验证 style_json 是否是有效的 JSON 并包含所有必需字段
    if let Err(e) = JsonHighlightTemplate::validate_style_json(&style_json) {
        return Err(AppError::BadRequest(format!("模板样式验证失败：{}", e)));
    }

    // 检查是否已存在同名模板
    let templates = JsonHighlightTemplate::get_all_templates(state.db.inner()).await?;
    if templates.iter().any(|t| t.name == name) {
        return Err(AppError::BadRequest(format!("模板名称 '{}' 已存在，请使用其他名称", name)));
    }

    let id = JsonHighlightTemplate::save_template(
        state.db.inner(),
        &name,
        &description,
        false,
        &style_json,
    )
    .await?;

    Ok(serde_json::json!({ "id": id }))
}

async fn handle_json_highlight_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let description = get_string_param(&body, "description")?;
    let style_json = get_long_string_param(&body, "style_json")?;

    // Get template info
    let template = JsonHighlightTemplate::get_template_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Template {} not found", id)))?;

    // Cannot update built-in templates
    if template.is_builtin {
        return Err(AppError::BadRequest("Cannot update built-in templates".to_string()));
    }

    // 验证 style_json 是否是有效的 JSON 并包含所有必需字段
    if let Err(e) = JsonHighlightTemplate::validate_style_json(&style_json) {
        return Err(AppError::BadRequest(format!("模板样式验证失败：{}", e)));
    }

    JsonHighlightTemplate::save_template(
        state.db.inner(),
        &template.name,
        &description,
        false,
        &style_json,
    )
    .await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_json_highlight_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let deleted = JsonHighlightTemplate::delete_template(state.db.inner(), id).await?;

    if !deleted {
        return Err(AppError::NotFound(
            "Template not found or cannot delete built-in templates".to_string()
        ));
    }

    Ok(serde_json::json!({ "success": true }))
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

// ==================== Additional Topic Handlers ====================

async fn handle_topic_saved(state: AppState, body: Value) -> Result<Value> {
    use crate::db::topic::TopicStore;

    let cluster_id = get_string_param(&body, "cluster_id")?;

    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let topic_names: Vec<String> = topics.into_iter().map(|t| t.topic_name).collect();

    Ok(serde_json::json!({ "topics": topic_names }))
}

// ==================== Favorite ====================

async fn handle_favorite_group_list(state: AppState) -> Result<Value> {
    let groups = get_all_groups_with_count(&state.db).await?;
    Ok(serde_json::json!(groups))
}

async fn handle_favorite_group_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let description = get_optional_string_param(&body, "description");
    let sort_order = get_optional_i64_param(&body, "sort_order").map(|v| v as i32);

    let req = CreateGroupRequest {
        name,
        description,
        sort_order,
    };

    let group = create_group(&state.db, &req).await?;
    Ok(serde_json::json!(group))
}

async fn handle_favorite_group_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let group = get_group_by_id(&state.db, id).await?;
    match group {
        Some(g) => Ok(serde_json::json!(g)),
        None => Err(AppError::NotFound(format!("Group {} not found", id))),
    }
}

async fn handle_favorite_group_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let name = get_optional_string_param(&body, "name");
    let description = get_optional_string_param(&body, "description");
    let sort_order = get_optional_i64_param(&body, "sort_order").map(|v| v as i32);

    let req = UpdateGroupRequest {
        name,
        description,
        sort_order,
    };

    let group = update_group(&state.db, id, &req).await?;
    match group {
        Some(g) => Ok(serde_json::json!(g)),
        None => Err(AppError::NotFound(format!("Group {} not found", id))),
    }
}

async fn handle_favorite_group_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let deleted = delete_group(&state.db, id).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "Group deleted successfully" }))
    } else {
        Err(AppError::NotFound(format!("Group {} not found", id)))
    }
}

async fn handle_favorite_list(state: AppState) -> Result<Value> {
    let favorites = get_all_favorites_with_groups(&state.db).await?;
    Ok(serde_json::json!(favorites))
}

async fn handle_favorite_create(state: AppState, body: Value) -> Result<Value> {
    let group_id = get_i64_param(&body, "group_id")?;
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic_name = get_string_param(&body, "topic_name")?;
    let description = get_optional_string_param(&body, "description");
    let sort_order = get_optional_i64_param(&body, "sort_order").map(|v| v as i32);

    let req = CreateFavoriteRequest {
        group_id,
        cluster_id,
        topic_name,
        description,
        sort_order,
    };

    let item = create_favorite(&state.db, &req).await?;
    Ok(serde_json::json!(item))
}

async fn handle_favorite_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let item = get_favorite_by_id(&state.db, id).await?;
    match item {
        Some(i) => Ok(serde_json::json!(i)),
        None => Err(AppError::NotFound(format!("Favorite {} not found", id))),
    }
}

async fn handle_favorite_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let group_id = get_optional_i64_param(&body, "group_id");
    let description = get_optional_string_param(&body, "description");
    let sort_order = get_optional_i64_param(&body, "sort_order").map(|v| v as i32);

    let req = UpdateFavoriteRequest {
        group_id,
        description,
        sort_order,
    };

    let item = update_favorite(&state.db, id, &req).await?;
    match item {
        Some(i) => Ok(serde_json::json!(i)),
        None => Err(AppError::NotFound(format!("Favorite {} not found", id))),
    }
}

async fn handle_favorite_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let deleted = delete_favorite(&state.db, id).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "Favorite deleted successfully" }))
    } else {
        Err(AppError::NotFound(format!("Favorite {} not found", id)))
    }
}

async fn handle_favorite_check(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic_name = get_string_param(&body, "topic_name")?;

    let is_fav = is_topic_favorite(&state.db, &cluster_id, &topic_name).await?;
    Ok(serde_json::json!({ "is_favorite": is_fav }))
}

async fn handle_favorite_delete_by_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic_name = get_string_param(&body, "topic_name")?;

    let deleted = delete_favorite_by_topic(&state.db, &cluster_id, &topic_name).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "Favorite deleted successfully" }))
    } else {
        Err(AppError::NotFound("Favorite not found".to_string()))
    }
}

// ==================== Cluster Group ====================

async fn handle_cluster_group_list(state: AppState) -> Result<Value> {
    let groups = ClusterGroupStore::list(state.db.inner()).await?;

    let groups_json: Vec<Value> = groups
        .into_iter()
        .map(|g| {
            serde_json::json!({
                "id": g.id,
                "name": g.name,
                "description": g.description,
                "sort_order": g.sort_order,
                "created_at": g.created_at,
                "updated_at": g.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!(groups_json))
}

async fn handle_cluster_group_get(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let group = ClusterGroupStore::get(state.db.inner(), id).await?;

    Ok(serde_json::json!({
        "id": group.id,
        "name": group.name,
        "description": group.description,
        "sort_order": group.sort_order,
        "created_at": group.created_at,
        "updated_at": group.updated_at,
    }))
}

async fn handle_cluster_group_create(state: AppState, body: Value) -> Result<Value> {
    let name = get_string_param(&body, "name")?;
    let description = get_optional_string_param(&body, "description");
    let sort_order = get_optional_i64_param(&body, "sort_order").unwrap_or(0);

    // Check if name already exists
    if let Some(_existing) = ClusterGroupStore::get_by_name(state.db.inner(), &name).await? {
        return Err(AppError::BadRequest(format!(
            "Group name '{}' already exists",
            name
        )));
    }

    let req = CreateClusterGroupRequest {
        name: name.clone(),
        description,
        sort_order,
    };

    let group = ClusterGroupStore::create(state.db.inner(), &req).await?;

    Ok(serde_json::json!({
        "id": group.id,
        "name": group.name,
        "description": group.description,
        "sort_order": group.sort_order,
        "created_at": group.created_at,
        "updated_at": group.updated_at,
    }))
}

async fn handle_cluster_group_update(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let name = get_optional_string_param(&body, "name");
    let description = get_optional_string_param(&body, "description");
    let sort_order = get_optional_i64_param(&body, "sort_order");

    // If name changed, check new name exists
    if let Some(ref new_name) = name {
        if let Some(existing) = ClusterGroupStore::get_by_name(state.db.inner(), new_name).await? {
            if existing.id != id {
                return Err(AppError::BadRequest(format!(
                    "Group name '{}' already exists",
                    new_name
                )));
            }
        }
    }

    let req = UpdateClusterGroupRequest {
        name,
        description,
        sort_order,
    };

    let group = ClusterGroupStore::update(state.db.inner(), id, &req).await?;

    Ok(serde_json::json!({
        "id": group.id,
        "name": group.name,
        "description": group.description,
        "sort_order": group.sort_order,
        "created_at": group.created_at,
        "updated_at": group.updated_at,
    }))
}

async fn handle_cluster_group_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    ClusterGroupStore::delete(state.db.inner(), id).await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_cluster_group_clusters(state: AppState, body: Value) -> Result<Value> {
    let group_id = get_i64_param(&body, "group_id")?;

    let clusters = ClusterGroupStore::get_clusters_in_group(state.db.inner(), group_id).await?;

    let clusters_json: Vec<Value> = clusters
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "brokers": c.brokers,
                "request_timeout_ms": c.request_timeout_ms,
                "operation_timeout_ms": c.operation_timeout_ms,
                "group_id": c.group_id,
                "created_at": c.created_at,
                "updated_at": c.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!(clusters_json))
}

async fn handle_cluster_group_assign_cluster(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_i64_param(&body, "cluster_id")?;
    let group_id = get_i64_param(&body, "group_id")?;

    ClusterGroupStore::assign_cluster_to_group(state.db.inner(), cluster_id, group_id).await?;

    Ok(serde_json::json!({ "success": true }))
}
