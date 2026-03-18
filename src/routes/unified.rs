/// 统一 API 路由处理器
/// 所有 API 请求都通过 POST /api 发送，method 放在 header 中，参数放在 body 中

use crate::db::cluster::{ClusterStore, CreateClusterRequest, UpdateClusterRequest};
use crate::db::cluster_connection::ClusterConnectionStore;
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
        "cluster.test_config" => handle_cluster_test_with_config(state, body).await,
        "cluster.stats" => handle_cluster_stats(state, body).await,

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

/// Fetch topics with cluster info from all clusters
async fn handle_topic_list_with_cluster(state: AppState, body: Value) -> Result<Value> {
    use crate::db::cluster::ClusterStore;

    // Use Vec of {name, cluster} objects
    #[derive(serde::Serialize)]
    struct TopicWithCluster {
        name: String,
        cluster: String,
    }

    // Check if a specific cluster_id is provided
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    if let Some(id) = cluster_id {
        // Fetch topics from a specific cluster
        let db_topics = TopicStore::list_by_cluster(state.db.inner(), &id).await.ok();

        let mut topics: Vec<TopicWithCluster> = Vec::new();

        if let Some(topics_list) = db_topics {
            for topic in topics_list {
                topics.push(TopicWithCluster {
                    name: topic.topic_name,
                    cluster: id.clone(),
                });
            }
        }

        // Background sync
        let state_clone = state.clone();
        let id_clone = id.clone();
        tokio::spawn(async move {
            let _ = sync_topics_from_kafka(state_clone, &id_clone).await;
        });

        return Ok(serde_json::json!({ "topics": topics }));
    }

    // No cluster_id provided - fetch from all clusters
    let clusters = ClusterStore::list(state.db.inner()).await?;

    let mut all_topics: Vec<TopicWithCluster> = Vec::new();

    for cluster in clusters {
        let cluster_name = &cluster.name;

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

    Ok(serde_json::json!({ "topics": all_topics }))
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
        // === 并行模式（分区数>1）===
        let msgs_per_partition = max_messages;
        let partition_has_specific_offset = partition.is_some();
        let partition_offset_val = if partition_has_specific_offset { offset } else { None };

        use tokio::sync::Semaphore;
        use tokio::time::{timeout, Duration as TokioDuration};
        use std::sync::Arc;

        // 最多10个并发
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
                // 统一使用60秒超时，给搜索足够的处理时间
                let fetch_timeout = TokioDuration::from_secs(60);
                let result = timeout(fetch_timeout, tokio::task::spawn_blocking(move || {
                    fetch_partition_messages_unified(
                        brokers, topic, part_id, msgs_per_partition, part_offset,
                        start_time, end_time, search, fetch_mode,
                    )
                })).await;
                (part_id, result)
            });
            handles.push(handle);
        }

        // 预估实际消息数：有搜索条件时通常返回较少，无搜索时可能接近max_messages per partition
        let estimated = if search.is_some() {
            max_messages * 2  // 搜索时预估更少
        } else {
            total_target
        }.min(50000);  // 最大预分配5万条，避免过大
        let mut all_msgs: Vec<crate::kafka::consumer::KafkaMessage> = Vec::with_capacity(estimated);
        for handle in handles {
            match handle.await {
                Ok((part_id, task_result)) => match task_result {
                    Ok(Ok(msgs)) => {
                        tracing::info!("[Parallel] Got {} messages from partition {}", msgs.len(), part_id);
                        all_msgs.extend(msgs);
                    }
                    Ok(Err(_)) => {
                        tracing::warn!("[Parallel] Partition {} fetch timeout", part_id);
                    }
                    Err(e) => {
                        tracing::warn!("[Parallel] Partition {} task join error: {}", part_id, e);
                    }
                },
                Err(e) => {
                    tracing::warn!("[Parallel] Spawn error: {}", e);
                }
            }
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

    // 排序
    let mut all_msgs = messages;
    all_msgs.sort_by(|a, b| {
        match (a.timestamp, b.timestamp) {
            (Some(ta), Some(tb)) => if is_desc { tb.cmp(&ta) } else { ta.cmp(&tb) },
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => if is_desc { b.offset.cmp(&a.offset) } else { a.offset.cmp(&b.offset) },
        }
    });

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
    cfg.set("partition.assignment.strategy", "");
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

                    // 时间范围过滤
                    if let Some(start) = start_time {
                        if let Some(t) = ts { if t < start { continue; } }
                    }
                    if let Some(end) = end_time {
                        if let Some(t) = ts { if t > end { continue; } }
                    }

                    let key_bytes = msg.key().map(|k| k.to_vec());
                    let value_bytes = msg.payload().map(|p| p.to_vec());

                    // 搜索过滤
                    if need_search {
                        let term = search_lower.as_ref().unwrap();
                        if !message_matches_search(&key_bytes, &value_bytes, term) {
                            // 不匹配，直接退出（已经到末尾了）
                            break;
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
                    let term = search_lower.as_ref().unwrap();
                    if !message_matches_search(&key_bytes, &value_bytes, term) {
                        continue; // 不匹配搜索条件，跳过
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
    let high_offset = high.saturating_sub(1).max(0);

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
                                tracing::info!("[calculate_time_range_offsets] start_time={} -> offset={} on partition {}",
                                    start_time, offset, partition);
                                // 确保不小于 low watermark
                                found_offset = offset.max(low);
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
                                // end_time 对应的 offset 是大于等于该时间的第一条消息
                                // 所以时间范围的有效结束 offset 是 offset - 1
                                let effective_end = offset.saturating_sub(1);
                                tracing::info!("[calculate_time_range_offsets] end_time={} -> raw_offset={}, effective_end={} on partition {}",
                                    end_time, offset, effective_end, partition);
                                // 确保不超过 high watermark
                                found_offset = effective_end.min(high_offset);
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

    Ok(TimeRangeInfo {
        start_offset,
        end_offset,
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
