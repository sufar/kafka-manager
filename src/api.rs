/// 统一 API 分发器
/// 前端通过 Tauri command 调用，method 为字符串（如 "cluster.list"），参数为 JSON Value

use crate::config::KafkaConfig;
use crate::db::cluster::{ClusterStore, CreateClusterRequest, UpdateClusterRequest};
use crate::db::cluster_group::{ClusterGroupStore, CreateClusterGroupRequest, UpdateClusterGroupRequest};
use crate::db::favorite::{
    create_favorite, create_group, delete_favorite, delete_favorite_by_topic,
    delete_group, get_all_favorites_with_groups, get_all_groups_with_count, get_favorite_by_id,
    get_group_by_id, is_topic_favorite, update_favorite, update_group,
    CreateFavoriteRequest, CreateGroupRequest, UpdateFavoriteRequest, UpdateGroupRequest,
};
use crate::db::topic_history::{
    clear_history, delete_history, delete_history_by_topic, get_history_list,
    record_history,
};
use crate::db::sent_message::{
    clear_sent_message_history, delete_sent_message, delete_sent_messages_by_topic, get_sent_message_list, record_sent_message,
};
use crate::db::settings::SettingStore;
use crate::api_import_export::ImportDataRequest;
use crate::db::topic::TopicStore;
use crate::db::topic_template::{
    CreateTopicTemplateRequest, TopicTemplateStore,
    UpdateTopicTemplateRequest,
};
use crate::error::{AppError, Result};
use crate::kafka::offset::KafkaOffsetManager;
use crate::kafka::throughput::KafkaThroughputCalculator;
use crate::db::schema_registry::{SchemaRegistryStore, SchemaStore};
use base64::Engine;
use crate::kafka::avro::AvroCodec;
use crate::kafka::protobuf::ProtobufCodec;
use crate::telemetry;
use crate::AppState;
use crate::RefreshState;
use serde_json::Value;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use std::cmp::Reverse;
use tokio_util::sync::CancellationToken;

/// 流式消息事件（通过 Tauri Channel 发送给前端，data 为 JSON 字符串）
#[derive(Clone, Debug, serde::Serialize)]
pub struct StreamEvent {
    pub event: String,
    pub data: String,
}

impl StreamEvent {
    fn new(event: &str, data: String) -> Self {
        Self { event: event.to_string(), data }
    }
}

/// RefreshGuard - RAII guard to automatically clear refresh state when refresh completes
struct RefreshGuard {
    cluster_id: String,
    refresh_state: Arc<Mutex<RefreshState>>,
}

impl Drop for RefreshGuard {
    fn drop(&mut self) {
        let mut state = self.refresh_state.lock().expect("refresh state poisoned");
        state.refreshing_clusters.remove(&self.cluster_id);
    }
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

fn get_i64_param_opt(body: &Value, key: &str) -> Option<i64> {
    body.get(key).and_then(|v| v.as_i64())
}

fn get_string_param_opt(body: &Value, key: &str) -> Option<String> {
    body.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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
/// 启动流式消息查询，返回事件接收端（由 Tauri command 转发到前端 Channel）
///
/// 事件流：start -> batch* -> order? -> complete / error
/// 调用方负责：消费 receiver 并转发、超时/取消时调用 cancel_token.cancel()
pub async fn start_message_list_stream(
    state: AppState,
    body: Value,
    cancel_token: CancellationToken,
) -> Result<mpsc::Receiver<StreamEvent>> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;
    let partition = get_optional_i32_param(&body, "partition");
    let offset = get_optional_i64_param(&body, "offset");
    let max_messages = get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
    let limit = get_optional_i64_param(&body, "limit").map(|v| v as usize);
    let start_time = get_optional_i64_param(&body, "start_time");
    let end_time = get_optional_i64_param(&body, "end_time");
    let search = get_optional_string_param(&body, "search");
    let search_in = get_optional_string_param(&body, "search_in");
    let fetch_mode = get_optional_string_param(&body, "fetchMode");
    let sort = get_optional_string_param(&body, "sort");

    // 首先确保集群客户端已创建
    let config = ensure_cluster_client(&state, &cluster_id).await?;
    let max_msgs = limit.or(max_messages).unwrap_or(100);

    let (tx, rx) = mpsc::channel::<StreamEvent>(100);
    let brokers = config.brokers.clone();
    let cancel_token_clone = cancel_token.clone();

    // 在后台任务中执行消息获取和流式发送
    tokio::spawn(async move {
        let result = fetch_messages_streaming_sse(
            &brokers,
            &topic,
            partition,
            offset,
            max_msgs,
            start_time,
            end_time,
            search,
            search_in,
            fetch_mode.as_deref(),
            sort.as_deref(),
            tx.clone(),
            cancel_token_clone,
        ).await;

        match result {
            Ok(_) => {
                // 发送完成标记
                let _ = tx.send(StreamEvent::new("complete", "{}".to_string())).await;
            }
            Err(e) => {
                // 发送错误
                let error_json = serde_json::json!({"error": e.to_string()}).to_string();
                let _ = tx.send(StreamEvent::new("error", error_json)).await;
            }
        }
    });

    Ok(rx)
}

// Dispatch function
pub async fn dispatch_request(method: &str, state: AppState, body: Value) -> Result<Value> {
    match method {
        // Health
        "health" => handle_health().await,

        // Cluster
        "cluster.list" => handle_cluster_list(state, body).await,
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
        "refresh.status" => handle_refresh_status(state).await,
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
        "connection.batch_disconnect" => handle_connection_batch_disconnect(state, body).await,
        "connection.batch_reconnect" => handle_connection_batch_reconnect(state, body).await,

        // Settings
        "settings.get" => handle_settings_get(state, body).await,
        "settings.update" => handle_settings_update(state, body).await,
        "settings.export" => handle_settings_export(state).await,
        "settings.import" => handle_settings_import(state, body).await,

        // App
        "app.version" => handle_app_version().await,
        "app.logs" => handle_app_logs().await,
        "app.logs.clear" => handle_app_logs_clear().await,

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

        // Topic History
        "topic_history.list" => handle_topic_history_list(state, body).await,
        "topic_history.record" => handle_topic_history_record(state, body).await,
        "topic_history.delete" => handle_topic_history_delete(state, body).await,
        "topic_history.delete_by_topic" => handle_topic_history_delete_by_topic(state, body).await,
        "topic_history.clear" => handle_topic_history_clear(state).await,

        // Sent Message History
        "sent_message.list" => handle_sent_message_list(state, body).await,
        "sent_message.record" => handle_sent_message_record(state, body).await,
        "sent_message.delete" => handle_sent_message_delete(state, body).await,
        "sent_message.clear" => handle_sent_message_clear(state).await,

        // Consumer Group
        "consumer_group.list" => handle_consumer_group_list(state, body).await,
        "consumer_group.list_by_topic" => handle_consumer_group_list_by_topic(state, body).await,
        "consumer_group.get" => handle_consumer_group_get(state, body).await,
        "consumer_group.offsets" => handle_consumer_group_offsets(state, body).await,
        "consumer_group.refresh" => handle_consumer_group_refresh(state, body).await,
        "consumer_group.saved" => handle_consumer_group_saved(state, body).await,
        "consumer_group.reset_offset" => handle_consumer_group_reset_offset(state, body).await,
        "consumer_group.delete" => handle_consumer_group_delete(state, body).await,

        // Schema Registry
        "schema_registry.config.get" => crate::api_schema_registry::handle_config_get(state, body).await,
        "schema_registry.config.save" => crate::api_schema_registry::handle_config_save(state, body).await,
        "schema_registry.config.delete" => crate::api_schema_registry::handle_config_delete(state, body).await,
        "schema_registry.config.test" => crate::api_schema_registry::handle_config_test(state, body).await,
        "schema_registry.subject.list" => crate::api_schema_registry::handle_subject_list(state, body).await,
        "schema_registry.version.list" => crate::api_schema_registry::handle_version_list(state, body).await,
        "schema_registry.get" => crate::api_schema_registry::handle_schema_get(state, body).await,
        "schema_registry.get_latest" => crate::api_schema_registry::handle_schema_get_latest(state, body).await,
        "schema_registry.register" => crate::api_schema_registry::handle_schema_register(state, body).await,
        "schema_registry.compatibility.test" => crate::api_schema_registry::handle_compatibility_test(state, body).await,
        "schema_registry.compatibility.get" => crate::api_schema_registry::handle_compatibility_get(state, body).await,
        "schema_registry.compatibility.set" => crate::api_schema_registry::handle_compatibility_set(state, body).await,
        "schema_registry.list" => crate::api_schema_registry::handle_schema_list(state, body).await,
        "schema_registry.delete" => crate::api_schema_registry::handle_schema_delete(state, body).await,

        // Telemetry
        "telemetry.check_connection" => handle_telemetry_check_connection(state).await,
        "telemetry.report" => handle_telemetry_report(state).await,
        "telemetry.submit_feedback" => handle_telemetry_submit_feedback(state, body).await,

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

// ==================== App ====================

async fn handle_app_version() -> Result<Value> {
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn handle_app_logs() -> Result<Value> {
    use std::fs;
    use crate::utils::app_log_path;

    let log_path = app_log_path();

    let logs_content = fs::read_to_string(&log_path).unwrap_or_default();

    Ok(serde_json::json!({
        "logs": logs_content,
        "log_file": log_path.to_string_lossy()
    }))
}

async fn handle_app_logs_clear() -> Result<Value> {
    use std::fs;
    use dirs::cache_dir;
    use std::path::PathBuf;

    // 清空 Tauri 日志文件
    let tauri_log_path = cache_dir()
        .map(|d| d.join("kafka-manager").join("kafka-manager.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager.log"));
    let _ = fs::write(&tauri_log_path, "");

    // 清空 Rolling 日志目录中的所有文件
    let log_dir = cache_dir()
        .map(|d| d.join("kafka-manager").join("logs"))
        .unwrap_or_else(|| PathBuf::from("/tmp/kafka-manager/logs"));

    if let Ok(entries) = fs::read_dir(&log_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let _ = fs::remove_file(path);
            }
        }
    }

    Ok(serde_json::json!({
        "success": true
    }))
}

// ==================== Cluster ====================

async fn handle_cluster_list(state: AppState, body: Value) -> Result<Value> {
    let group_id = get_i64_param_opt(&body, "group_id");
    let search = get_string_param_opt(&body, "search");

    let clusters = ClusterStore::list(state.db.inner(), group_id, search).await?;

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

    // Incremental: add only the new cluster without rebuilding all connections
    add_client_for_cluster(&state, &cluster.name, KafkaConfig {
        brokers: cluster.brokers.clone(),
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    }).await?;

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

    // Incremental: reconnect only the updated cluster without rebuilding all connections
    reconnect_client_for_cluster(&state, &cluster.name, KafkaConfig {
        brokers: cluster.brokers.clone(),
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    }).await?;

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

    // 获取集群信息
    let cluster = ClusterStore::get(state.db.inner(), id).await?;
    let cluster_name = cluster.name.clone();

    // 删除该集群下的所有关联数据（按依赖顺序）
    // 1. 删除 Topic 收藏
    sqlx::query("DELETE FROM favorite_items WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;

    // 2. 删除 Topic 浏览历史
    sqlx::query("DELETE FROM topic_history WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;

    // 3. 删除 Topic 发送历史
    sqlx::query("DELETE FROM sent_messages WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;

    // 4. 删除 Consumer Group 元数据和 Offset
    sqlx::query("DELETE FROM consumer_group_offsets WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;
    sqlx::query("DELETE FROM consumer_group_metadata WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;

    // 5. 删除 Schema Registry 配置和 Schema 缓存
    sqlx::query("DELETE FROM schema_registry_configs WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;
    sqlx::query("DELETE FROM schemas WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;

    // 6. 删除资源标签
    sqlx::query("DELETE FROM resource_tags WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;

    // 删除集群
    ClusterStore::delete(state.db.inner(), id).await?;

    // 重新加载 Kafka 客户端
    reload_clients(&state).await?;

    // 删除本地缓存的 Topic 元数据
    sqlx::query("DELETE FROM topic_metadata WHERE cluster_id = ?")
        .bind(&cluster_name)
        .execute(state.db.inner())
        .await?;

    Ok(serde_json::json!({ "success": true }))
}

async fn handle_cluster_test(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;
    let (success, error_msg) = ClusterStore::test_connection(state.db.inner(), id).await?;

    Ok(serde_json::json!({
        "success": success,
        "error": error_msg
    }))
}

/// Test cluster connection with temporary configuration (without saving to database)
async fn handle_cluster_test_with_config(_state: AppState, body: Value) -> Result<Value> {
    let brokers = get_string_param(&body, "brokers")?;
    let request_timeout_ms = get_optional_i64_param(&body, "request_timeout_ms").unwrap_or(5000);
    let operation_timeout_ms = get_optional_i64_param(&body, "operation_timeout_ms").unwrap_or(5000);

    use crate::config::KafkaConfig;
    use crate::kafka::{KafkaAdmin, test_brokers_connectivity};

    // 1. 先用 TCP 快速探测 broker 是否可达（控制 DNS + TCP 超时）
    let tcp_timeout = std::cmp::min(request_timeout_ms as u64, 5000);
    if !test_brokers_connectivity(&brokers, tcp_timeout).await {
        return Ok(serde_json::json!({
            "success": false,
            "error": format!("无法连接到 broker: {}", brokers)
        }));
    }

    // 2. TCP 连通后再做 Kafka 协议级验证
    let config = KafkaConfig {
        brokers,
        request_timeout_ms: request_timeout_ms as u32,
        operation_timeout_ms: operation_timeout_ms as u32,
    };

    match KafkaAdmin::new(&config) {
        Ok(admin) => {
            match admin.list_topics() {
                Ok(_) => Ok(serde_json::json!({ "success": true })),
                Err(e) => Ok(serde_json::json!({
                    "success": false,
                    "error": format!("Connected, but failed to list topics: {}", e)
                })),
            }
        }
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": format!("Failed to create connection: {}", e)
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
        // 预分配 HashMap 容量：估计 broker 数量 = sqrt(分区数)
        let estimated_brokers = 4;
        let mut broker_leader_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::with_capacity(estimated_brokers);
        let mut broker_replica_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::with_capacity(estimated_brokers);

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

/// Incremental: add a single cluster's clients without rebuilding all connections
async fn add_client_for_cluster(state: &AppState, cluster_name: &str, config: KafkaConfig) -> Result<()> {
    use crate::db::topic::TopicStore;

    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(cluster_name, &config)?;
    state.set_clients(new_clients);

    // Sync topics in background
    let db = state.db.inner().clone();
    let cluster_name_owned = cluster_name.to_string();
    if let Some(admin) = state.get_clients().get_admin(cluster_name) {
        let admin = admin.clone();
        tokio::spawn(async move {
            match tokio::task::spawn_blocking(move || admin.list_topics()).await {
                Ok(Ok(topics)) => {
                    let _ = TopicStore::sync_topics(&db, &cluster_name_owned, &topics).await;
                    tracing::info!("Synced {} topics for new cluster '{}'", topics.len(), cluster_name_owned);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to list topics for cluster '{}': {}", cluster_name_owned, e);
                }
                Err(e) => {
                    tracing::warn!("Topic list task panicked for cluster '{}': {}", cluster_name_owned, e);
                }
            }
        });
    }

    tracing::info!("Added Kafka client for cluster '{}'", cluster_name);
    Ok(())
}

/// Incremental: reconnect a single cluster's clients without rebuilding all connections
async fn reconnect_client_for_cluster(state: &AppState, cluster_name: &str, config: KafkaConfig) -> Result<()> {
    use crate::db::topic::TopicStore;

    let current_clients = state.get_clients();
    let new_clients = current_clients.reconnect_cluster(cluster_name, &config)?;
    state.set_clients(new_clients);

    // Sync topics in background
    let db = state.db.inner().clone();
    let cluster_name_owned = cluster_name.to_string();
    if let Some(admin) = state.get_clients().get_admin(cluster_name) {
        let admin = admin.clone();
        tokio::spawn(async move {
            match tokio::task::spawn_blocking(move || admin.list_topics()).await {
                Ok(Ok(topics)) => {
                    let _ = TopicStore::sync_topics(&db, &cluster_name_owned, &topics).await;
                    tracing::info!("Synced {} topics for reconnected cluster '{}'", topics.len(), cluster_name_owned);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to list topics for cluster '{}': {}", cluster_name_owned, e);
                }
                Err(e) => {
                    tracing::warn!("Topic list task panicked for cluster '{}': {}", cluster_name_owned, e);
                }
            }
        });
    }

    tracing::info!("Reconnected Kafka client for cluster '{}'", cluster_name);
    Ok(())
}

async fn reload_clients(state: &AppState) -> Result<()> {

    // Get all clusters from database
    let clusters = ClusterStore::list(state.db.inner(), None, None).await?;

    let mut new_clusters = std::collections::HashMap::with_capacity(clusters.len());
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

    // 从数据库获取（纯读，不触发 Kafka 同步）
    if let Some(topics) = db_topics {
        if !topics.is_empty() {
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
            ClusterStore::list(state.db.inner(), None, None).await.ok().unwrap_or_default()
                .into_iter().map(|c| (c.id, c.name)).collect()
        } else {
            // Fetch only specified clusters
            let mut result = Vec::with_capacity(ids.len());
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
        ClusterStore::list(state.db.inner(), None, None).await.ok().unwrap_or_default()
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

    // No search query - use database-level sorting and pagination
    // 不再将所有 topic 加载到内存中排序，改为在数据库层面完成
    let cluster_names: Vec<String> = clusters_to_fetch.iter().map(|(_, n)| n.clone()).collect();

    let total = if cluster_names.is_empty() {
        0
    } else {
        TopicStore::count_by_clusters(state.db.inner(), &cluster_names).await.unwrap_or(0)
    };

    let paginated_topics = if total > 0 && offset < total as usize {
        TopicStore::list_by_clusters_with_pagination(
            state.db.inner(),
            &cluster_names,
            offset as u32,
            limit as u32,
        ).await.unwrap_or_default()
            .into_iter()
            .map(|(name, cluster)| TopicWithCluster { name, cluster })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let end = (offset + limit).min(total as usize);

    Ok(serde_json::json!({
        "topics": paginated_topics,
        "total": total,
        "offset": offset,
        "limit": limit,
        "has_more": end < total as usize
    }))
}

/// Fetch topics from all clusters
async fn handle_topic_list_all_clusters(state: AppState) -> Result<Value> {
    use crate::db::cluster::ClusterStore;

    // Get all clusters from database
    let clusters = ClusterStore::list(state.db.inner(), None, None).await?;

    let mut all_topics: Vec<String> = Vec::with_capacity(clusters.len() * 50);

    // Fetch topics from each cluster
    for cluster in clusters {
        let cluster_name = &cluster.name;

        if let Ok(topics) = TopicStore::list_by_cluster(state.db.inner(), cluster_name).await {
            for topic in topics {
                all_topics.push(topic.topic_name);
            }
        }
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

    Ok(serde_json::json!({ "name": name }))
}

async fn handle_topic_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    // 支持 topic 或 name 参数（前端使用 topic）
    let name = get_string_param(&body, "topic")
        .or_else(|_| get_string_param(&body, "name"))?;

    let admin = get_or_create_admin_client(&state, &cluster_id).await?;

    admin.delete_topic(&name).await?;

    // 清理 SQLite 中该 topic 的所有关联数据
    let _ = TopicStore::delete(state.db.inner(), &cluster_id, &name).await;
    let _ = delete_history_by_topic(&state.db, &cluster_id, &name).await;
    let _ = delete_favorite_by_topic(&state.db, &cluster_id, &name).await;
    let _ = delete_sent_messages_by_topic(&state.db, &cluster_id, &name).await;

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

    let mut created = Vec::with_capacity(topics.len());
    let mut failed = Vec::with_capacity(topics.len());

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

    let mut deleted = Vec::with_capacity(topics.len());
    let mut failed = Vec::with_capacity(topics.len());

    for topic_name in topics {
        match admin.delete_topic(&topic_name).await {
            Ok(_) => {
                // 清理 SQLite 中该 topic 的所有关联数据
                let _ = TopicStore::delete(state.db.inner(), &cluster_id, &topic_name).await;
                let _ = delete_history_by_topic(&state.db, &cluster_id, &topic_name).await;
                let _ = delete_favorite_by_topic(&state.db, &cluster_id, &topic_name).await;
                let _ = delete_sent_messages_by_topic(&state.db, &cluster_id, &topic_name).await;
                deleted.push(topic_name);
            }
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

    // 从数据库删除所有 topic 元数据及关联数据
    for topic_name in &topic_names {
        let _ = TopicStore::delete(state.db.inner(), &cluster_id, topic_name).await;
        let _ = delete_history_by_topic(&state.db, &cluster_id, topic_name).await;
        let _ = delete_favorite_by_topic(&state.db, &cluster_id, topic_name).await;
        let _ = delete_sent_messages_by_topic(&state.db, &cluster_id, topic_name).await;
    }

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

/// 查询当前正在刷新的集群列表
async fn handle_refresh_status(state: AppState) -> Result<Value> {
    let refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
    let clusters: Vec<String> = refresh_state.refreshing_clusters.iter().cloned().collect();
    Ok(serde_json::json!({
        "refreshing_clusters": clusters
    }))
}

async fn handle_topic_refresh(state: AppState, body: Value) -> Result<Value> {
    // cluster_id 是可选参数，未指定时刷新所有集群
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(String::from);
    let topic_name = body.get("topic_name").and_then(|v| v.as_str()).map(String::from);

    // 如果有导入导出正在进行，跳过刷新
    {
        let lock = state.import_export_lock.lock().expect("import_export_lock poisoned");
        if lock.is_busy {
            return Ok(serde_json::json!({
                "success": true,
                "message": "Import/export in progress, skipping refresh",
            }));
        }
    }

    // 检查并设置刷新状态
    {
        let refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        if let Some(ref cluster) = cluster_id {
            if refresh_state.refreshing_clusters.contains(cluster) {
                return Err(AppError::BadRequest(format!(
                    "Cluster '{}' is already being refreshed, please wait",
                    cluster
                )));
            }
        }
    }

    // 立即返回成功，后台异步同步
    // 这样不会阻塞后续的 topic.list 请求
    let is_single_topic = topic_name.is_some();
    tokio::spawn(async move {
        if let (Some(cluster_id), Some(topic_name)) = (&cluster_id, &topic_name) {
            // 刷新指定集群中的单个 topic
            refresh_single_topic(state, cluster_id.clone(), topic_name.clone()).await;
        } else if let Some(cluster_id) = cluster_id {
            // 刷新指定集群
            refresh_single_cluster(state, cluster_id).await;
        } else {
            // 刷新所有集群
            refresh_all_clusters(state).await;
        }
    });

    // 立即返回成功
    let message = if is_single_topic {
        "Single topic refresh started in background"
    } else {
        "Topic refresh started in background"
    };
    Ok(serde_json::json!({
        "success": true,
        "message": message,
    }))
}

/// 刷新单个集群中指定 Topic（只拉取该 topic 的元数据，不遍历全部 topic）
pub async fn refresh_single_topic(state: AppState, cluster_id: String, topic_name: String) {
    use crate::db::cluster::ClusterStore;
    use crate::db::topic::TopicStore;

    // 标记为正在刷新
    {
        let mut refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        refresh_state.refreshing_clusters.insert(cluster_id.clone());
    }

    let _guard = RefreshGuard {
        cluster_id: cluster_id.clone(),
        refresh_state: state.refresh_state.clone(),
    };

    let cluster = match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
        Ok(Some(cluster)) => cluster,
        Ok(None) => {
            tracing::error!("Cluster '{}' not found in database", cluster_id);
            return;
        }
        Err(e) => {
            tracing::error!("Failed to get cluster config: {}", e);
            return;
        }
    };

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    let clients = state.get_clients();
    let clients = if clients.get_admin(&cluster_id).is_some() {
        match clients.reconnect_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
                new_clients
            }
            Err(e) => {
                tracing::error!("Failed to reconnect cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    } else {
        match clients.with_added_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
                new_clients
            }
            Err(e) => {
                tracing::error!("Failed to add cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    };

    let admin = match clients.get_admin(&cluster_id) {
        Some(admin) => admin,
        None => {
            tracing::error!("Failed to get admin client for cluster '{}'", cluster_id);
            return;
        }
    };

    let _ = state.pools.reconnect(&cluster_id, &config, &state.config.pool).await;

    // 只拉取指定 topic 的元数据（fetch_metadata(Some(topic_name)) 不会遍历全部 topic）
    let topic_info = {
        let admin = admin.clone();
        let topic_for_closure = topic_name.clone();
        match tokio::task::spawn_blocking(move || admin.get_topic_info(&topic_for_closure)).await {
            Ok(Ok(info)) => Some(info),
            Ok(Err(e)) => {
                tracing::warn!("Topic '{}' not found in Kafka cluster '{}': {}", topic_name, cluster_id, e);
                None
            }
            Err(e) => {
                tracing::error!("Task join error: {}", e);
                return;
            }
        }
    };

    if let Some(info) = topic_info {
        let db = state.db.clone();
        let partition_count = info.partitions.len() as i32;
        let empty_config = std::collections::HashMap::with_capacity(0);
        if let Err(e) = TopicStore::upsert(db.inner(), &cluster_id, &topic_name, partition_count, 1, &empty_config).await {
            tracing::error!("Failed to upsert topic '{}': {}", topic_name, e);
        } else {
            tracing::info!("Refreshed single topic '{}' in cluster '{}': {} partitions", topic_name, cluster_id, partition_count);
        }
    } else {
        // topic 在 Kafka 中不存在，从数据库中删除
        match TopicStore::delete(state.db.inner(), &cluster_id, &topic_name).await {
            Ok(_) => {
                tracing::info!("Removed non-existent topic '{}' from database for cluster '{}'", topic_name, cluster_id);
            }
            Err(e) => {
                tracing::error!("Failed to delete topic '{}': {}", topic_name, e);
            }
        }
    }

    // 替换 rdkafka 内部缓存
    admin.clear_metadata_cache();
}

/// 刷新单个集群的 Topic 列表
pub async fn refresh_single_cluster(state: AppState, cluster_id: String) {
    use crate::db::cluster::ClusterStore;
    use crate::db::topic::TopicStore;

    tracing::info!("[refresh] Starting refresh for cluster '{}'", cluster_id);

    // 标记为正在刷新
    {
        let mut refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        refresh_state.refreshing_clusters.insert(cluster_id.clone());
    }

    // 确保退出时清除标记
    let _guard = RefreshGuard {
        cluster_id: cluster_id.clone(),
        refresh_state: state.refresh_state.clone(),
    };

    // 从数据库获取集群配置
    let cluster = match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
        Ok(Some(cluster)) => cluster,
        Ok(None) => {
            tracing::error!("Cluster '{}' not found in database", cluster_id);
            return;
        }
        Err(e) => {
            tracing::error!("Failed to get cluster config: {}", e);
            return;
        }
    };

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    // 重连集群（创建新的客户端连接）
    let clients = state.get_clients();
    tracing::info!("[refresh] Reconnecting/adding cluster '{}' (admin exists: {})", cluster_id, clients.get_admin(&cluster_id).is_some());
    let clients = if clients.get_admin(&cluster_id).is_some() {
        // 已存在则重连
        match clients.reconnect_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
                new_clients
            }
            Err(e) => {
                tracing::error!("Failed to reconnect cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    } else {
        // 不存在则添加
        match clients.with_added_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
                new_clients
            }
            Err(e) => {
                tracing::error!("Failed to add cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    };

    let admin = match clients.get_admin(&cluster_id) {
        Some(admin) => admin,
        None => {
            tracing::error!("Failed to get admin client for cluster '{}'", cluster_id);
            return;
        }
    };

    // 同时重连连接池
    let _ = state.pools.reconnect(&cluster_id, &config, &state.config.pool).await;

    // 一次性获取所有 topic 名称 + 分区信息（只调用一次 fetch_metadata）
    tracing::info!("[refresh] Fetching metadata for cluster '{}' (timeout: {}ms)", cluster_id, config.operation_timeout_ms);
    let topics_with_partitions = {
        let admin = admin.clone();
        match tokio::task::spawn_blocking(move || admin.list_topics_with_partitions()).await {
            Ok(Ok(topics)) => {
                tracing::info!("[refresh] Fetched {} topics from Kafka for cluster '{}'", topics.len(), cluster_id);
                topics
            }
            Ok(Err(e)) => {
                tracing::error!("[refresh] Failed to fetch metadata from Kafka for cluster '{}': {} (timeout: {}ms)", cluster_id, e, config.operation_timeout_ms);
                return;
            }
            Err(e) => {
                tracing::error!("[refresh] Task join error for cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    };

    let current_topics: Vec<String> = topics_with_partitions.iter()
        .map(|(name, _)| name.clone())
        .collect();

    // Sync to database (batch SQL optimization)
    match TopicStore::sync_topics(state.db.inner(), &cluster_id, &current_topics).await {
        Ok(sync_result) => {
            tracing::info!("[refresh] Synced topics for cluster '{}': +{} -{} (total in DB: {})", cluster_id, sync_result.added.len(), sync_result.removed.len(), current_topics.len());

            // 直接使用第一次 fetch 的结果，不需要二次 Kafka 查询
            let details: Vec<(String, i32)> = topics_with_partitions
                .into_iter()
                .map(|(name, n)| (name, n as i32))
                .collect();

            if let Err(e) = TopicStore::batch_upsert_details(state.db.inner(), &cluster_id, &details).await {
                tracing::error!("Failed to batch upsert topic details: {}", e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to sync topics: {}", e);
        }
    }

    // 替换 rdkafka 内部缓存：用单 topic metadata 替换 70k topic 的全量缓存
    admin.clear_metadata_cache();
    tracing::info!("[refresh] Completed refresh for cluster '{}'", cluster_id);
}

/// 刷新所有集群的 Topic 列表（在单个任务中，各集群并行刷新）
async fn refresh_all_clusters(state: AppState) {
    use crate::db::cluster::ClusterStore;

    // 获取所有集群
    let clusters = match ClusterStore::list(state.db.inner(), None, None).await {
        Ok(clusters) => clusters,
        Err(e) => {
            tracing::error!("Failed to list clusters: {}", e);
            return;
        }
    };

    tracing::info!("Refreshing all {} clusters in parallel", clusters.len());

    // 并行刷新所有集群
    let mut tasks = Vec::with_capacity(clusters.len());
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

    tracing::info!("Completed refreshed all clusters");
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
    let clusters = ClusterStore::list(state.db.inner(), None, None).await?;
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
    let search_in = get_optional_string_param(&body, "search_in");
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
        search_in,
        fetch_mode.as_deref(),
        sort.as_deref(),
    )
    .await?;

    // 获取 Schema Registry 配置，用于消息解码
    let schema_info = {
        let pool = state.get_pool();
        let cfg = SchemaRegistryStore::get_config(&pool, &cluster_id).await.ok().flatten();
        let schema = if cfg.is_some() {
            SchemaStore::get_latest_schema(&pool, &cluster_id, &topic).await.ok().flatten()
        } else {
            None
        };
        cfg.zip(schema)
    };

    // 优化：预分配 Vec 容量，减少扩容开销
    let records: Vec<Value> = messages
        .into_iter()
        .map(|msg| {
            // 优化：避免不必要的 clone，直接使用 value
            let mut value = msg.value.unwrap_or_default();

            // 尝试使用 Schema 解码消息
            if let Some((ref _config, ref schema)) = schema_info {
                // 检查是否是 base64 编码的二进制数据（Avro/Protobuf）
                if let Ok(decoded_bytes) = base64::engine::general_purpose::STANDARD.decode(&value) {
                    match schema.schema_type.as_str() {
                        "AVRO" => {
                            if let Ok(json_value) = AvroCodec::decode(&schema.schema_json, &decoded_bytes) {
                                value = serde_json::to_string(&json_value).unwrap_or(value);
                            }
                        }
                        "PROTOBUF" => {
                            if let Ok(json_value) = ProtobufCodec::decode_simple(&schema.schema_json, &decoded_bytes) {
                                value = serde_json::to_string(&json_value).unwrap_or(value);
                            }
                        }
                        _ => {}
                    }
                }
            }

            serde_json::json!({
                "partition": msg.partition,
                "offset": msg.offset,
                "key": msg.key,
                "value": value,
                "timestamp": msg.timestamp,
            })
        })
        .collect();

    Ok(serde_json::json!({ "messages": records }))
}

/// SSE 流式消息获取
/// 并行读取 + 最小堆归并 + 实时 SSE 发送
///
/// 优化说明：
/// 1. 使用 Arc 共享字符串数据，避免每个分区 task 都 clone
/// 2. 预分配 Vec 容量，减少扩容开销
/// 3. 使用 BinaryHeap 归并，O(log n) 时间复杂度
async fn fetch_messages_streaming_sse(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    search_in: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
    sse_tx: mpsc::Sender<StreamEvent>,
    cancel_token: CancellationToken,
) -> Result<()> {
    use rdkafka::consumer::{Consumer, BaseConsumer};
    use rdkafka::ClientConfig;
    use std::time::Duration;

    let start_time_total = std::time::Instant::now();
    let topic = topic.to_string();
    let brokers = brokers.to_string();
    let search = search.clone();
    let search_in = search_in.clone();
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
    let _is_desc = sort.as_deref() == Some("desc") || (sort.is_none() && fetch_mode.as_deref() != Some("oldest"));

    tracing::info!("[SSE Stream] {} partitions, {} messages per partition, total target {}",
        partition_count, max_messages, total_target);

    // 发送开始事件
    let start_event = serde_json::json!({
        "event": "start",
        "partitions": partition_count,
        "total_target": total_target
    });
    sse_tx.send(StreamEvent::new("start", start_event.to_string())).await
        .map_err(|_| AppError::Internal("Stream channel closed".to_string()))?;

    // 为每个分区创建 channel，预分配 Vec 容量
    let mut rxs: Vec<(i32, mpsc::Receiver<crate::kafka::consumer::KafkaMessage>)> = Vec::with_capacity(partition_count);
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::with_capacity(partition_count);

    for &part_id in &partitions {
        let (tx, rx) = mpsc::channel::<crate::kafka::consumer::KafkaMessage>(max_messages);
        rxs.push((part_id, rx));

        // 为每个分区 clone 数据
        let brokers_clone = brokers.clone();
        let topic_clone = topic.clone();
        let search_clone = search.clone();
        let search_in_clone = search_in.clone();
        let fetch_mode_clone = fetch_mode.clone();
        let cancel_token_clone = cancel_token.clone();
        let part_offset = if partition.is_some() { offset } else { None };

        let handle = tokio::spawn(async move {
            fetch_partition_messages_streaming(
                brokers_clone, topic_clone, part_id, max_messages,
                part_offset, start_time, end_time, search_clone, search_in_clone, fetch_mode_clone, tx, cancel_token_clone,
            ).await;
        });
        handles.push(handle);
    }

    // 使用最小堆进行流式归并
    // 预分配堆容量，减少扩容开销
    let mut heap = BinaryHeap::<Reverse<HeapMessage>>::with_capacity(partition_count);
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

    loop {
        // 检查取消信号（使用 biased 模式，优先检查取消）
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                tracing::info!("[SSE Stream] Cancelled by client, stopping at sent_count={}", sent_count);
                break;
            }
            // 默认分支，继续执行主逻辑
            () = std::future::ready(()) => {}
        }

        // 处理堆中的消息
        if let Some(Reverse(heap_msg)) = heap.pop() {
            let part_id = heap_msg.message.partition;

            // 优化：直接使用 to_json_value() 避免重复代码
            let msg_json = heap_msg.message.to_json_value();
            batch.push(msg_json);
            sent_count += 1;

            // 批次满则发送
            if batch.len() >= BATCH_SIZE {
                // 零拷贝优化：使用 to_vec 直接序列化为字节，避免 to_string 的额外分配
                // serde_json 保证输出是有效 UTF-8，所以 from_utf8_unchecked 是安全的
                let messages_array = serde_json::Value::Array(std::mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE)));
                let batch_data = serde_json::json!({
                    "event": "batch",
                    "messages": messages_array,
                    "progress": sent_count,
                    "total": total_target
                });
                let batch_bytes = match serde_json::to_vec(&batch_data) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::error!("[SSE Stream] Failed to serialize batch: {}", e);
                        break;
                    }
                };
                // 安全：serde_json 输出的字节保证是有效 UTF-8
                let batch_json = unsafe { String::from_utf8_unchecked(batch_bytes) };
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        tracing::info!("[SSE Stream] Cancelled while sending batch, stopping");
                        break;
                    }
                    result = sse_tx.send(StreamEvent::new("batch", batch_json)) => {
                        if result.is_err() {
                            tracing::info!("[SSE Stream] Stream channel closed, stopping");
                            break;
                        }
                    }
                }
            }

            // 从对应分区获取下一条
            if let Some((_, rx)) = rxs.iter_mut().find(|(p, _)| *p == part_id) {
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    msg = rx.recv() => {
                        match msg {
                            Some(m) => {
                                heap.push(Reverse(HeapMessage {
                                    timestamp: m.timestamp,
                                    offset: m.offset,
                                    message: m,
                                }));
                            }
                            None => {
                                completed_partitions += 1;
                                tracing::info!("[SSE Stream] Partition {} completed, total sent: {}", part_id, sent_count);
                            }
                        }
                    }
                }
            }
        } else {
            // 堆为空但还在接收中，使用可中断的等待替代固定 sleep
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        if completed_partitions >= partition_count && heap.is_empty() {
            break;
        }
    }

    // 发送剩余批次
    if !batch.is_empty() {
        let messages_array = serde_json::Value::Array(batch);
        let batch_data = serde_json::json!({
            "event": "batch",
            "messages": messages_array,
            "progress": sent_count,
            "total": total_target
        });
        let batch_json = match serde_json::to_string(&batch_data) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("[SSE Stream] Failed to serialize final batch: {}", e);
                return Err(AppError::Internal(format!("Serialization failed: {}", e)));
            }
        };
        let _ = sse_tx.send(StreamEvent::new("batch", batch_json)).await;
    }

    // 取消所有分区任务
    cancel_token.cancel();

    // 等待所有任务完成
    for handle in handles {
        let _ = handle.await;
    }

    // 清空接收缓冲区，释放内存
    for (_, rx) in &mut rxs {
        while let Ok(_msg) = rx.try_recv() {
            // 丢弃所有剩余消息，释放内存
        }
    }

    tracing::info!("[SSE Stream] Completed: sent {} messages from {} partitions (target: {}) in {:?}",
        sent_count, partition_count, total_target, start_time_total.elapsed());

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
    search_in: Option<String>,
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
    let search_in = search_in.clone();
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
        let mut rxs = Vec::with_capacity(partitions.len());
        let mut handles = Vec::with_capacity(partitions.len());

        for &part_id in &partitions {
            let (tx, rx) = mpsc::channel::<crate::kafka::consumer::KafkaMessage>(msgs_per_partition);
            rxs.push((part_id, rx));

            let brokers = brokers.clone();
            let topic = topic.clone();
            let search = search.clone();
            let search_in = search_in.clone();
            let fetch_mode = fetch_mode.clone();
            let part_offset = if partition_has_specific_offset { partition_offset_val } else { None };

            let handle = tokio::spawn(async move {
                fetch_partition_messages_streaming(
                    brokers, topic, part_id, msgs_per_partition, part_offset,
                    start_time, end_time, search, search_in, fetch_mode, tx, CancellationToken::new(),
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
                start_time, end_time, search, search_in, fetch_mode,
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
    search_in: &Option<String>,
) -> bool {
    let key_str = key_bytes.as_ref().and_then(|k| std::str::from_utf8(k).ok());
    let value_str = value_bytes.as_ref().and_then(|v| std::str::from_utf8(v).ok());

    let search_in = search_in.as_deref().unwrap_or("all");

    match search_in {
        "key" => {
            // 只搜索键
            key_str.map_or(false, |k| k.to_lowercase().contains(search_term))
        }
        "value" => {
            // 只搜索值
            value_str.map_or(false, |v| v.to_lowercase().contains(search_term))
        }
        _ => {
            // 默认搜索键和值
            let key_match = key_str.map_or(false, |k| k.to_lowercase().contains(search_term));
            let value_match = value_str.map_or(false, |v| v.to_lowercase().contains(search_term));
            key_match || value_match
        }
    }
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
    search_in: Option<String>,
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
    let mut consecutive_empty = 0usize; // 连续空轮询计数（用于退避）

    // 自适应poll超时 + 指数退避，应对慢 broker
    const FIRST_POLL_TIMEOUT_MS: u64 = 500;  // 首次poll等待更长时间，因为可能需要建立连接
    const BASE_POLL_TIMEOUT_MS: u64 = 200;
    const MAX_POLL_TIMEOUT_MS: u64 = 2000;
    const MAX_EMPTY_POLLS: usize = 25;
    const MAX_POLL_TIME_SECS: u64 = 120;

    let poll_start = std::time::Instant::now();
    let mut got_first = false;

    while raw_messages.len() < max_messages
        && empty_count < MAX_EMPTY_POLLS
        && poll_start.elapsed() < Duration::from_secs(MAX_POLL_TIME_SECS)
    {
        // 自适应poll超时：首次500ms，正常200ms，连续空轮询时指数退避
        let poll_timeout = if !got_first {
            Duration::from_millis(FIRST_POLL_TIMEOUT_MS)
        } else if consecutive_empty == 0 {
            Duration::from_millis(BASE_POLL_TIMEOUT_MS)
        } else {
            let backoff = (BASE_POLL_TIMEOUT_MS.saturating_mul(1u64 << consecutive_empty.min(4)))
                .min(MAX_POLL_TIMEOUT_MS);
            Duration::from_millis(backoff)
        };

        match consumer.poll(poll_timeout) {
            Some(Ok(msg)) => {
                if !got_first {
                    got_first = true;
                    tracing::info!("[Unified Partition] First message received after {:?}", poll_start.elapsed());
                }
                empty_count = 0;
                consecutive_empty = 0;

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
                            if !message_matches_search(&key_bytes, &value_bytes, term, &search_in) {
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
                        if !message_matches_search(&key_bytes, &value_bytes, term, &search_in) {
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
                consecutive_empty += 1;
            }
            None => {
                empty_count += 1;
                consecutive_empty += 1;
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
    search_in: Option<String>,
    fetch_mode: Option<String>,
    tx: mpsc::Sender<crate::kafka::consumer::KafkaMessage>,
    cancel_token: CancellationToken,
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
    // 注意：request.timeout.ms 仅用于 Producer，Consumer 不需要
    cfg.set("enable.partition.eof", "false");
    cfg.set("connections.max.idle.ms", "540000");
    cfg.set("reconnect.backoff.ms", "50");
    cfg.set("reconnect.backoff.max.ms", "500");
    cfg.set("socket.connection.setup.timeout.ms", "3000");
    cfg.set("metadata.max.age.ms", "5000");
    cfg.set("partition.assignment.strategy", "range");
    cfg.set("broker.address.family", "v4");
    // P0 性能优化：批量 fetch 配置
    cfg.set("fetch.message.max.bytes", "10485760"); // 10MB
    cfg.set("max.partition.fetch.bytes", "10485760"); // 10MB
    cfg.set("fetch.max.bytes", "52428800"); // 50MB

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
    let mut consecutive_empty = 0usize; // 连续空轮询计数（用于退避）

    // === 动态调整空轮询次数 ===
    // 基础次数 20 次，每 1000 条消息增加 5 次，最多 50 次
    let base_empty_polls = 20usize;
    let additional_polls = (max_messages / 1000) * 5;
    let max_empty_polls = (base_empty_polls + additional_polls).min(50);
    tracing::info!("[Streaming] Partition {} dynamic max_empty_polls: {} (max_messages: {})",
        partition, max_empty_polls, max_messages);

    const FIRST_POLL_TIMEOUT_MS: u64 = 500;
    const BASE_POLL_TIMEOUT_MS: u64 = 200;    // 基础 poll 超时（收到数据后使用）
    const MAX_POLL_TIMEOUT_MS: u64 = 2000;    // 最大 poll 超时（退避上限）
    const MAX_POLL_TIME_SECS: u64 = 120;

    let poll_start = std::time::Instant::now();
    let mut got_first = false;

    loop {
        // 检查取消信号
        if cancel_token.is_cancelled() {
            tracing::info!("[Streaming] Partition {} cancelled by client, stopping at sent_count={}", partition, sent_count);
            break;
        }

        if sent_count >= max_messages
            || empty_count >= max_empty_polls
            || poll_start.elapsed() >= Duration::from_secs(MAX_POLL_TIME_SECS)
        {
            break;
        }

        // 自适应 poll 超时：连续空轮询时指数退避，收到数据后恢复
        let poll_timeout = if !got_first {
            Duration::from_millis(FIRST_POLL_TIMEOUT_MS)
        } else if consecutive_empty == 0 {
            Duration::from_millis(BASE_POLL_TIMEOUT_MS)
        } else {
            // 指数退避：200ms → 400ms → 800ms → 1600ms → 2000ms(capped)
            let backoff = (BASE_POLL_TIMEOUT_MS.saturating_mul(1u64 << consecutive_empty.min(4)))
                .min(MAX_POLL_TIMEOUT_MS);
            Duration::from_millis(backoff)
        };

        match consumer.poll(poll_timeout) {
            Some(Ok(msg)) => {
                if !got_first {
                    got_first = true;
                    tracing::info!("[Streaming] First message received for partition {} after {:?}", partition, poll_start.elapsed());
                }
                empty_count = 0;
                consecutive_empty = 0;

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
                            if !message_matches_search(&key_bytes, &value_bytes, term, &search_in) {
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
                        if !message_matches_search(&key_bytes, &value_bytes, term, &search_in) {
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

                if tx.send(kafka_msg).await.is_err() {
                    tracing::warn!("[Streaming] Channel closed for partition {}", partition);
                    return;
                }
                sent_count += 1;
            }
            Some(Err(e)) => {
                tracing::warn!("[Streaming] Poll error for partition {}: {}", partition, e);
                empty_count += 1;
                consecutive_empty += 1;
            }
            None => {
                empty_count += 1;
                consecutive_empty += 1;
            }
        }
    }

    tracing::info!("[Streaming] Partition {} completed: sent {} messages, empty_count={}/{}, elapsed={:?}",
        partition, sent_count, empty_count, max_empty_polls, poll_start.elapsed());
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
    let mut value = get_long_string_param(&body, "value")?;
    let partition = get_optional_i32_param(&body, "partition");
    let headers = get_hashmap_param(&body, "headers");

    // 检查是否有 Schema Registry 配置，尝试自动序列化消息
    let pool = state.get_pool();
    if let Ok(_config) = SchemaRegistryStore::get_config(&pool, &cluster_id).await {
        if let Some(schema) = SchemaStore::get_latest_schema(&pool, &cluster_id, &topic).await.ok().flatten() {
            // 根据 schema 类型进行编码
            match schema.schema_type.as_str() {
                "AVRO" => {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&value) {
                        match AvroCodec::encode(&schema.schema_json, &json_value) {
                            Ok(encoded_bytes) => {
                                // 使用 base64 编码二进制数据
                                value = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
                                tracing::info!("Message encoded with Avro schema for topic: {}", topic);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to encode message with Avro: {}", e);
                                // 编码失败时保持原始值
                            }
                        }
                    }
                }
                "PROTOBUF" => {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&value) {
                        // 尝试从 schema_json 中提取 descriptor 信息
                        // 这里假设 schema_json 包含必要的 protobuf 描述符
                        match ProtobufCodec::encode_simple(&schema.schema_json, &json_value) {
                            Ok(encoded_bytes) => {
                                value = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
                                tracing::info!("Message encoded with Protobuf schema for topic: {}", topic);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to encode message with Protobuf: {}", e);
                            }
                        }
                    }
                }
                "JSON" => {
                    // JSON Schema 不需要编码，保持原样
                }
                _ => {}
            }
        }
    }

    let clients = state.get_clients();
    let producer = clients
        .get_producer(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let headers_opt = if headers.is_empty() { None } else { Some(&headers) };
    let (partition_result, offset) = producer
        .send_to_partition(&topic, partition, key.as_deref(), &value, headers_opt)
        .await?;

    // 记录发送消息历史
    let headers_json = if headers.is_empty() {
        None
    } else {
        serde_json::to_string(&headers).ok()
    };
    let _ = record_sent_message(
        &state.db,
        &cluster_id,
        &topic,
        partition_result,
        key.as_deref(),
        &value,
        headers_json.as_deref(),
        Some(offset),
    ).await;

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
    let search_in = get_optional_string_param(&body, "search_in");
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
        search_in,
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
            // 连接池中不存在，尝试自动重连 2 次
            let max_retries = 2;
            let mut last_error: Option<String> = None;

            for attempt in 1..=max_retries {
                tracing::warn!(
                    "[HealthCheck] Cluster '{}' not found in connection pool, attempting reconnect {}/{}",
                    cluster_id,
                    attempt,
                    max_retries
                );

                // 尝试从数据库获取集群配置并重新建立连接
                match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
                    Ok(Some(cluster)) => {
                        let config = crate::config::KafkaConfig {
                            brokers: cluster.brokers,
                            request_timeout_ms: cluster.request_timeout_ms as u32,
                            operation_timeout_ms: cluster.operation_timeout_ms as u32,
                        };

                        // 尝试重新添加集群到连接池
                        match state.pools.add_cluster(&cluster_id, &config, &state.config.pool).await {
                            Ok(_) => {
                                tracing::info!("[HealthCheck] Cluster '{}' reconnected successfully on attempt {}", cluster_id, attempt);
                                // 重连成功，返回连接状态（需要等待连接建立）
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                return Ok(serde_json::json!({
                                    "cluster_id": cluster_id,
                                    "healthy": true,
                                    "status": "reconnected",
                                    "error_message": None::<String>,
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
                        break; // 数据库中不存在，不再重试
                    }
                    Err(e) => {
                        tracing::error!("[HealthCheck] Failed to get cluster '{}' from database: {}", cluster_id, e);
                        last_error = Some(format!("Failed to get cluster config: {}", e));
                    }
                }

                // 如果不是最后一次尝试，等待一段时间后继续
                if attempt < max_retries {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }
            }

            // 所有重连尝试都失败，返回错误状态
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

async fn handle_connection_batch_disconnect(state: AppState, body: Value) -> Result<Value> {
    let cluster_names = get_string_array_param(&body, "cluster_names");
    let mut results = Vec::with_capacity(cluster_names.len());
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
    let mut results = Vec::with_capacity(cluster_names.len());

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
        let mut result = Vec::with_capacity(keys.len());
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
    let value = body.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();

    SettingStore::set(state.db.inner(), &key, &value).await?;

    Ok(serde_json::json!({
        "key": key,
        "value": value,
    }))
}

// ==================== Settings Import/Export ====================

async fn handle_settings_export(state: AppState) -> Result<Value> {
    let data = crate::api_import_export::export_data(&state).await?;
    Ok(serde_json::to_value(&data)?)
}

async fn handle_settings_import(state: AppState, body: Value) -> Result<Value> {
    let req: ImportDataRequest = serde_json::from_value(body)?;
    crate::api_import_export::import_data(state, req).await
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

// ==================== Topic History ====================

async fn handle_topic_history_list(state: AppState, body: Value) -> Result<Value> {
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
    let offset = body.get("offset").and_then(|v| v.as_i64()).unwrap_or(0);

    let histories = get_history_list(&state.db, Some(limit), Some(offset)).await?;

    let histories_json: Vec<Value> = histories
        .into_iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id,
                "cluster_id": h.cluster_id,
                "topic_name": h.topic_name,
                "viewed_at": h.viewed_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "histories": histories_json }))
}

async fn handle_topic_history_record(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic_name = get_string_param(&body, "topic_name")?;

    let history = record_history(&state.db, &cluster_id, &topic_name).await?;

    Ok(serde_json::json!({
        "id": history.id,
        "cluster_id": history.cluster_id,
        "topic_name": history.topic_name,
        "viewed_at": history.viewed_at,
    }))
}

async fn handle_topic_history_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let deleted = delete_history(&state.db, id).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "History deleted successfully" }))
    } else {
        Err(AppError::NotFound("History not found".to_string()))
    }
}

async fn handle_topic_history_delete_by_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic_name = get_string_param(&body, "topic_name")?;

    let deleted = delete_history_by_topic(&state.db, &cluster_id, &topic_name).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "History deleted successfully" }))
    } else {
        Err(AppError::NotFound("History not found".to_string()))
    }
}

async fn handle_topic_history_clear(state: AppState) -> Result<Value> {
    let count = clear_history(&state.db).await?;
    Ok(serde_json::json!({ "count": count, "message": "History cleared successfully" }))
}

// ==================== Sent Message History ====================

async fn handle_sent_message_list(state: AppState, body: Value) -> Result<Value> {
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
    let offset = body.get("offset").and_then(|v| v.as_i64()).unwrap_or(0);
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(String::from);
    let topic_name = body.get("topic_name").and_then(|v| v.as_str()).map(String::from);

    let messages = get_sent_message_list(
        &state.db,
        cluster_id.as_deref(),
        topic_name.as_deref(),
        Some(limit),
        Some(offset),
    ).await?;

    let messages_json: Vec<Value> = messages
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "cluster_id": m.cluster_id,
                "topic_name": m.topic_name,
                "partition": m.partition,
                "message_key": m.message_key,
                "message_value": m.message_value,
                "headers": m.headers.and_then(|h| serde_json::from_str::<Value>(&h).ok()),
                "offset": m.offset,
                "sent_at": m.sent_at,
            })
        })
        .collect();

    Ok(serde_json::json!({ "messages": messages_json }))
}

async fn handle_sent_message_record(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic_name = get_string_param(&body, "topic_name")?;
    let partition = body.get("partition").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let message_key = body.get("key").and_then(|v| v.as_str()).map(String::from);
    let message_value = get_long_string_param(&body, "value")?;
    let headers = body.get("headers").and_then(|v| serde_json::to_string(v).ok());
    let offset = body.get("offset").and_then(|v| v.as_i64());

    let message = record_sent_message(
        &state.db,
        &cluster_id,
        &topic_name,
        partition,
        message_key.as_deref(),
        &message_value,
        headers.as_deref(),
        offset,
    ).await?;

    Ok(serde_json::json!({
        "id": message.id,
        "cluster_id": message.cluster_id,
        "topic_name": message.topic_name,
        "partition": message.partition,
        "message_key": message.message_key,
        "message_value": message.message_value,
        "offset": message.offset,
        "sent_at": message.sent_at,
    }))
}

async fn handle_sent_message_delete(state: AppState, body: Value) -> Result<Value> {
    let id = get_i64_param(&body, "id")?;

    let deleted = delete_sent_message(&state.db, id).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "Message history deleted successfully" }))
    } else {
        Err(AppError::NotFound("Message history not found".to_string()))
    }
}

async fn handle_sent_message_clear(state: AppState) -> Result<Value> {
    let count = clear_sent_message_history(&state.db).await?;
    Ok(serde_json::json!({ "count": count, "message": "Message history cleared successfully" }))
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

// ==================== Consumer Group ====================

use crate::db::consumer_group::ConsumerGroupStore;
use crate::kafka::consumer_group::KafkaConsumerGroupManager;

/// 刷新 Consumer Group 列表（从 Kafka 集群同步到数据库）
async fn handle_consumer_group_refresh(state: AppState, body: Value) -> Result<Value> {
    // cluster_id 是可选参数，未指定时刷新所有集群
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(String::from);
    let group_name = body.get("group_name").and_then(|v| v.as_str()).map(String::from);

    // 如果有导入导出正在进行，跳过刷新
    {
        let lock = state.import_export_lock.lock().expect("import_export_lock poisoned");
        if lock.is_busy {
            return Ok(serde_json::json!({
                "success": true,
                "message": "Import/export in progress, skipping refresh",
            }));
        }
    }

    // 检查并设置刷新状态
    {
        let refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        if let Some(ref cluster) = cluster_id {
            if refresh_state.refreshing_clusters.contains(cluster) {
                return Err(AppError::BadRequest(format!(
                    "Cluster '{}' is already being refreshed, please wait",
                    cluster
                )));
            }
        }
    }

    // 立即返回，后台异步刷新
    let is_single_group = group_name.is_some();
    tokio::spawn(async move {
        if let (Some(cluster_id), Some(group_name)) = (&cluster_id, &group_name) {
            refresh_single_consumer_group_by_name(state, cluster_id.clone(), group_name.clone()).await;
        } else if let Some(cluster_id) = cluster_id {
            refresh_single_consumer_group(state, cluster_id).await;
        } else {
            refresh_all_consumer_groups(state).await;
        }
    });

    let message = if is_single_group {
        "Single consumer group refresh started in background"
    } else {
        "Consumer group refresh started in background"
    };
    Ok(serde_json::json!({
        "success": true,
        "message": message,
    }))
}

/// 刷新单个集群中指定 Consumer Group（只拉取该 group 的信息）
pub async fn refresh_single_consumer_group_by_name(state: AppState, cluster_id: String, group_name: String) {
    use crate::db::cluster::ClusterStore;
    use crate::db::consumer_group::ConsumerGroupStore;

    {
        let mut refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        refresh_state.refreshing_clusters.insert(cluster_id.clone());
    }

    let _guard = RefreshGuard {
        cluster_id: cluster_id.clone(),
        refresh_state: state.refresh_state.clone(),
    };

    let cluster = match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
        Ok(Some(cluster)) => cluster,
        Ok(None) => {
            tracing::error!("Cluster '{}' not found in database", cluster_id);
            return;
        }
        Err(e) => {
            tracing::error!("Failed to get cluster config: {}", e);
            return;
        }
    };

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    let clients = state.get_clients();
    if clients.get_admin(&cluster_id).is_some() {
        match clients.reconnect_cluster(&cluster_id, &config) {
            Ok(new_clients) => { state.set_clients(new_clients.clone()); }
            Err(e) => {
                tracing::error!("Failed to reconnect cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    } else {
        match clients.with_added_cluster(&cluster_id, &config) {
            Ok(new_clients) => { state.set_clients(new_clients.clone()); }
            Err(e) => {
                tracing::error!("Failed to add cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    };
    let _ = state.pools.reconnect(&cluster_id, &config, &state.config.pool).await;

    // 创建 consumer manager 复用连接
    let unique_suffix = format!("{}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(), rand::random::<u32>());
    let reusable_consumer = {
        let mut client_config = crate::kafka::create_client_config(&config);
        client_config.set("group.id", &format!("kafka-manager-cg-refresh-{}", unique_suffix));
        client_config.set("enable.auto.commit", "false");
        match client_config.create::<rdkafka::consumer::BaseConsumer>() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create consumer for {}: {}", cluster_id, e);
                return;
            }
        }
    };

    let cg_manager = match KafkaConsumerGroupManager::with_consumer(&config, reusable_consumer) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to create ConsumerGroupManager for {}: {}", cluster_id, e);
            return;
        }
    };

    // 只拉取指定 group 的信息
    let cg_name = group_name.clone();
    let result = tokio::task::spawn_blocking(move || cg_manager.get_single_consumer_group(&cg_name)).await;

    let (group_state, topics) = match result {
        Ok(Ok((s, t))) => (s, t),
        Ok(Err(e)) => {
            tracing::warn!("Consumer group '{}' not found in Kafka cluster '{}': {}", group_name, cluster_id, e);
            return;
        }
        Err(e) => {
            tracing::error!("Task join error: {}", e);
            return;
        }
    };

    // 写入数据库：同步 group 名称
    let group_names = vec![group_name.clone()];
    if let Err(e) = ConsumerGroupStore::sync_consumer_groups(state.db.inner(), &cluster_id, &group_names).await {
        tracing::error!("Failed to sync consumer group '{}': {}", group_name, e);
        return;
    }

    // 清理旧的 group-topic 关系
    let _ = ConsumerGroupStore::cleanup_group(state.db.inner(), &cluster_id, &group_name).await;

    // 写入新的 group-topic 关系
    for topic in &topics {
        let _ = ConsumerGroupStore::upsert_topic_relation(state.db.inner(), &cluster_id, &group_name, topic).await;
    }

    tracing::info!("Refreshed consumer group '{}' in cluster '{}': state={}, topics={}", group_name, cluster_id, group_state, topics.len());
}

/// 刷新单个集群的 Consumer Group 列表
async fn refresh_single_consumer_group(state: AppState, cluster_id: String) {
    use crate::db::cluster::ClusterStore;
    use crate::db::consumer_group::ConsumerGroupStore;

    // 标记为正在刷新（如果已在刷新则直接返回）
    {
        let mut refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        if !refresh_state.refreshing_clusters.insert(cluster_id.clone()) {
            tracing::info!("Cluster '{}' consumer group refresh already in progress, skipping", cluster_id);
            return;
        }
    }

    // 确保退出时清除标记
    let _guard = RefreshGuard {
        cluster_id: cluster_id.clone(),
        refresh_state: state.refresh_state.clone(),
    };

    // 从数据库获取集群配置
    let cluster = match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
        Ok(Some(cluster)) => cluster,
        Ok(None) => {
            tracing::error!("Cluster '{}' not found in database", cluster_id);
            return;
        }
        Err(e) => {
            tracing::error!("Failed to get cluster config: {}", e);
            return;
        }
    };

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    // 重连集群（创建新的客户端连接）
    let clients = state.get_clients();
    if clients.get_admin(&cluster_id).is_some() {
        match clients.reconnect_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
            }
            Err(e) => {
                tracing::error!("Failed to reconnect cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    } else {
        match clients.with_added_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
            }
            Err(e) => {
                tracing::error!("Failed to add cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    };

    // 同时重连连接池
    let _ = state.pools.reconnect(&cluster_id, &config, &state.config.pool).await;

    // 创建一个复用的 consumer，整个刷新过程复用同一个连接
    // 使用随机 group.id 避免多个刷新任务冲突
    let unique_suffix = format!("{}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(), rand::random::<u32>());
    let reusable_consumer = {
        let mut client_config = crate::kafka::create_client_config(&config);
        client_config.set("group.id", &format!("kafka-manager-cg-refresh-{}", unique_suffix));
        client_config.set("enable.auto.commit", "false");
        match client_config.create::<rdkafka::consumer::BaseConsumer>() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create reusable consumer for {}: {}", cluster_id, e);
                return;
            }
        }
    };

    let cg_manager = match KafkaConsumerGroupManager::with_consumer(&config, reusable_consumer) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to create ConsumerGroupManager for {}: {}", cluster_id, e);
            return;
        }
    };

    // 步骤 1: 一次 fetch_group_list 同时获取 group 名称和 topics
    let groups_with_topics = match tokio::task::spawn_blocking({
        let cg_manager = cg_manager.clone();
        move || cg_manager.list_consumer_groups_with_topics()
    })
    .await
    {
        Ok(Ok(groups)) => groups,
        Ok(Err(e)) => {
            tracing::warn!("Failed to list consumer groups for {}: {}", cluster_id, e);
            return;
        }
        Err(e) => {
            tracing::error!("Failed to list consumer groups for {}: {}", cluster_id, e);
            return;
        }
    };

    let group_names: Vec<String> = groups_with_topics.iter().map(|(name, _)| name.clone()).collect();

    // 步骤 2: 同步 group 名称到元数据表
    if let Err(e) = ConsumerGroupStore::sync_consumer_groups(state.db.inner(), &cluster_id, &group_names).await {
        tracing::error!("Failed to sync consumer groups for {}: {}", cluster_id, e);
        return;
    }

    // 步骤 3: 清理该集群下所有旧的 group-topic 关系
    if let Err(e) = ConsumerGroupStore::cleanup_all_cluster_topic_relations(state.db.inner(), &cluster_id).await {
        tracing::warn!("Failed to cleanup topic relations for {}: {}", cluster_id, e);
    }

    // 步骤 4: 批量将 group-topic 关系写入多对多表（使用步骤 1 已提取的 topics）
    let group_count = groups_with_topics.len();
    let relations: Vec<(String, String)> = groups_with_topics
        .into_iter()
        .flat_map(|(group, topics)| {
            topics.into_iter().map(move |topic| (group.clone(), topic))
        })
        .collect();

    if !relations.is_empty() {
        if let Err(e) = ConsumerGroupStore::batch_upsert_topic_relations(
            state.db.inner(),
            &cluster_id,
            &relations,
        ).await {
            tracing::warn!("Failed to batch upsert topic relations for {}: {}", cluster_id, e);
        }
    }

    tracing::info!("Refreshed {} consumer groups for cluster {}", group_count, cluster_id);

    // 替换 rdkafka 内部缓存：用单 topic metadata 替换全量缓存
    if let Some(admin) = state.get_clients().get_admin(&cluster_id) {
        admin.clear_metadata_cache();
    }
}

/// 刷新所有集群的 Consumer Group 列表（在单个任务中，各集群并行刷新）
async fn refresh_all_consumer_groups(state: AppState) {
    use crate::db::cluster::ClusterStore;

    // 获取所有集群
    let clusters = match ClusterStore::list(state.db.inner(), None, None).await {
        Ok(clusters) => clusters,
        Err(e) => {
            tracing::error!("Failed to list clusters: {}", e);
            return;
        }
    };

    tracing::info!("Refreshing all {} clusters consumer groups in parallel", clusters.len());

    // 并行刷新所有集群（如果某个集群正在刷新则静默跳过）
    let mut tasks = Vec::with_capacity(clusters.len());
    for cluster in &clusters {
        let cluster_id = cluster.name.clone();
        // 检查是否正在刷新该集群，如果是则跳过
        let refreshing = {
            let refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
            refresh_state.refreshing_clusters.contains(&cluster_id)
        };
        if refreshing {
            tracing::debug!("Skipping consumer group refresh for cluster {} (already refreshing)", cluster_id);
            continue;
        }
        let state = state.clone();
        tasks.push(tokio::spawn(async move {
            refresh_single_consumer_group(state, cluster_id).await;
        }));
    }

    // 等待所有任务完成
    for task in tasks {
        let _ = task.await;
    }

    tracing::info!("Completed refreshing all clusters consumer groups");
}

/// 获取已保存的 Consumer Groups（从数据库）
async fn handle_consumer_group_saved(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_cluster_id_param(&body)?;

    let groups = ConsumerGroupStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let group_names: Vec<String> = groups.into_iter().map(|g| g.group_name).collect();

    Ok(serde_json::json!({ "groups": group_names }))
}

/// 列出 Consumer Groups（支持分页）
async fn handle_consumer_group_list(state: AppState, body: Value) -> Result<Value> {
    // Get pagination parameters (default: offset=0, limit=10000)
    let offset = body.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(10000) as usize;

    // Get cluster_ids array for multi-cluster selection
    let cluster_ids: Option<Vec<String>> = body.get("cluster_ids").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    });

    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Get search query for filtering
    let search_query: Option<String> = body.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Determine which clusters to fetch consumer groups from
    let clusters_to_fetch: Vec<String> = if let Some(ref ids) = cluster_ids {
        if ids.is_empty() {
            // Empty array means "all clusters"
            crate::db::cluster::ClusterStore::list(state.db.inner(), None, None).await.ok().unwrap_or_default()
                .into_iter().map(|c| c.name).collect()
        } else {
            ids.clone()
        }
    } else if let Some(ref id) = cluster_id {
        vec![id.clone()]
    } else {
        // Default to all clusters
        crate::db::cluster::ClusterStore::list(state.db.inner(), None, None).await.ok().unwrap_or_default()
            .into_iter().map(|c| c.name).collect()
    };

    // Fetch all consumer groups from specified clusters
    let mut all_groups: Vec<serde_json::Value> = Vec::with_capacity(clusters_to_fetch.len() * 20);

    for cluster_name in &clusters_to_fetch {
        if let Ok(groups) = crate::db::consumer_group::ConsumerGroupStore::list_by_cluster(
            state.db.inner(),
            cluster_name,
        ).await {
            for group in groups {
                let topics: Vec<String> = serde_json::from_str(&group.topics).unwrap_or_default();
                all_groups.push(serde_json::json!({
                    "id": group.id,
                    "cluster_id": group.cluster_id,
                    "group_name": group.group_name,
                    "topics": topics,
                    "fetched_at": group.fetched_at,
                }));
            }
        }
    }

    // Apply search filter if provided
    if let Some(ref query) = search_query {
        let q = query.to_lowercase();
        all_groups.retain(|g| {
            let name = g["group_name"].as_str().map(|s| s.to_lowercase()).unwrap_or_default();
            let cluster = g["cluster_id"].as_str().map(|s| s.to_lowercase()).unwrap_or_default();
            name.contains(&q) || cluster.contains(&q)
        });
    }

    // Sort by cluster then by group name
    all_groups.sort_by(|a, b| {
        let cluster_cmp = a["cluster_id"].as_str().cmp(&b["cluster_id"].as_str());
        if cluster_cmp == std::cmp::Ordering::Equal {
            a["group_name"].as_str().cmp(&b["group_name"].as_str())
        } else {
            cluster_cmp
        }
    });

    // Apply pagination
    let total = all_groups.len();
    let end = (offset + limit).min(total);
    let paginated_groups = if offset < total {
        all_groups.into_iter().skip(offset).take(limit).collect()
    } else {
        Vec::new()
    };

    Ok(serde_json::json!({
        "groups": paginated_groups,
        "total": total,
        "offset": offset,
        "limit": limit,
        "has_more": end < total
    }))
}

/// 获取消费指定 Topic 的 Consumer Groups（从 Kafka 获取最新 offset 数据）
async fn handle_consumer_group_list_by_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_cluster_id_param(&body)?;
    let topic = get_string_param(&body, "topic")?;

    // 步骤 1: 从多对多关系表查询订阅了该 topic 的 consumer group names
    let group_names = match crate::db::consumer_group::ConsumerGroupStore::list_group_names_by_topic(
        state.db.inner(),
        &cluster_id,
        &topic,
    )
    .await
    {
        Ok(groups) => groups,
        Err(e) => {
            tracing::error!("Failed to query consumer groups by topic: {}", e);
            Vec::new()
        }
    };

    tracing::info!("[handle_consumer_group_list_by_topic] found {} groups from DB for topic '{}'", group_names.len(), topic);

    // 步骤 2: 确保集群客户端已创建，并复用同一个 consumer 连接
    let config = ensure_cluster_client(&state, &cluster_id).await?;

    // 使用随机 group.id 避免冲突
    let unique_suffix = format!("{}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(), rand::random::<u32>());
    let reusable_consumer = {
        let mut client_config = crate::kafka::create_client_config(&config);
        client_config.set("group.id", &format!("kafka-manager-cg-list-by-topic-{}", unique_suffix));
        client_config.set("enable.auto.commit", "false");
        match client_config.create::<rdkafka::consumer::BaseConsumer>() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create reusable consumer for {}: {}", cluster_id, e);
                return Err(e.into());
            }
        }
    };

    let cg_manager = KafkaConsumerGroupManager::with_consumer(&config, reusable_consumer)?;

    // 步骤 3: 有界并发获取每个 group 的指定 topic offset 信息
    const CONCURRENCY_LIMIT: usize = 3;
    let semaphore = Arc::new(Semaphore::new(CONCURRENCY_LIMIT));
    let cg_manager = Arc::new(cg_manager);

    let mut handles: Vec<_> = Vec::new();
    for group_name in &group_names {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let mgr = cg_manager.clone();
        let group_name = group_name.clone();
        let topic = topic.clone();

        let handle = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            mgr.get_consumer_group_offsets_for_topic(&group_name, &topic)
                .map(|offsets| (group_name, offsets))
        });
        handles.push(handle);
    }

    let mut all_offsets: Vec<(String, crate::kafka::consumer_group::PartitionOffsetDetail)> = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(Ok((group_name, offsets))) => {
                for offset in offsets {
                    all_offsets.push((group_name.clone(), offset));
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Failed to get offsets for group: {}", e);
            }
            Err(e) => {
                tracing::warn!("Task panicked while getting offsets: {}", e);
            }
        }
    }

    // 步骤 4: 有界并发获取所有分区的最后提交时间
    let mut last_commit_times: std::collections::HashMap<(String, String, i32), Option<i64>> = std::collections::HashMap::new();

    // 只对有 offset 数据的 group 获取 last_commit_time
    let groups_with_offsets: Vec<String> = group_names
        .iter()
        .filter(|g| all_offsets.iter().any(|(group, _)| group.as_str() == g.as_str()))
        .cloned()
        .collect();

    let mut commit_handles: Vec<_> = Vec::new();
    for group_name in &groups_with_offsets {
        let group_parts: Vec<(String, i32)> = all_offsets
            .iter()
            .filter(|(g, _)| g.as_str() == group_name.as_str())
            .map(|(_, o)| (o.topic.clone(), o.partition))
            .collect();

        if group_parts.is_empty() {
            continue;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let mgr = cg_manager.clone();
        let group_name = group_name.clone();

        let handle = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            mgr.get_partitions_last_commit_time(&group_name, &group_parts)
                .map(|times| (group_name, group_parts, times))
        });
        commit_handles.push(handle);
    }

    for handle in commit_handles {
        match handle.await {
            Ok(Ok((group_name, group_parts, times))) => {
                for (i, time) in times.iter().enumerate() {
                    if i < group_parts.len() {
                        let key = (group_name.clone(), group_parts[i].0.clone(), group_parts[i].1);
                        last_commit_times.insert(key, *time);
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Failed to get last commit time for group: {}", e);
            }
            Err(e) => {
                tracing::warn!("Task panicked while getting last commit time: {}", e);
            }
        }
    }

    // 步骤 5: 构建返回结果
    let mut topic_offsets: Vec<serde_json::Value> = Vec::new();

    for (group_name, offset) in all_offsets {
        let key = (group_name.clone(), offset.topic.clone(), offset.partition);
        let last_commit_time = last_commit_times.get(&key).copied().flatten();

        topic_offsets.push(serde_json::json!({
            "group": group_name,
            "topic": offset.topic,
            "partition": offset.partition,
            "start_offset": offset.start_offset,
            "end_offset": offset.end_offset,
            "committed_offset": offset.committed_offset,
            "lag": offset.lag,
            "last_commit_time": last_commit_time,
        }));
    }

    // 按 group name，然后 partition 排序
    topic_offsets.sort_by(|a, b| {
        let group_cmp = a["group"].as_str().cmp(&b["group"].as_str());
        if group_cmp != std::cmp::Ordering::Equal {
            return group_cmp;
        }
        a["partition"].as_i64().cmp(&b["partition"].as_i64())
    });

    tracing::info!("[handle_consumer_group_list_by_topic] returning {} offsets", topic_offsets.len());

    Ok(serde_json::json!({
        "offsets": topic_offsets,
    }))
}

/// 获取 Consumer Group 详情
async fn handle_consumer_group_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_cluster_id_param(&body)?;
    let group_name = get_string_param(&body, "group_name")?;

    // 从数据库获取 group 信息
    let _group = ConsumerGroupStore::get_by_name(state.db.inner(), &cluster_id, &group_name)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Consumer group '{}' not found", group_name)))?;

    // 从多对多关系表获取 topics
    let topics = ConsumerGroupStore::list_topics_by_group(state.db.inner(), &cluster_id, &group_name)
        .await
        .unwrap_or_default();

    // 确保集群客户端已创建
    let config = ensure_cluster_client(&state, &cluster_id).await?;

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;

    // 在阻塞线程中执行 Kafka 操作
    let group_info = tokio::task::spawn_blocking(move || {
        cg_manager.get_consumer_group_info(&group_name, &topics)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(serde_json::json!({
        "group_id": group_info.group_id,
        "cluster_id": cluster_id,
        "state": group_info.state,
        "topics": group_info.topics,
    }))
}

/// 获取 Consumer Group 的 offset 和 lag 信息
async fn handle_consumer_group_offsets(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_cluster_id_param(&body)?;
    let group_name = get_string_param(&body, "group_name")?;

    tracing::info!("[handle_consumer_group_offsets] cluster_id={}, group_name={}", cluster_id, group_name);

    // 确保集群客户端已创建
    let config = ensure_cluster_client(&state, &cluster_id).await?;

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;
    let group_name_clone = group_name.clone();
    let cg_manager_clone = cg_manager.clone();

    // 首先尝试从数据库获取保存的 topics
    let db_topics = ConsumerGroupStore::get_by_name(state.db.inner(), &cluster_id, &group_name)
        .await
        .ok()
        .flatten()
        .and_then(|g| serde_json::from_str::<Vec<String>>(&g.topics).ok())
        .unwrap_or_default();

    tracing::info!("[handle_consumer_group_offsets] DB topics: {:?}", db_topics);

    // 在阻塞线程中执行 Kafka 操作 - 使用数据库中的 topics
    let mut offsets = tokio::task::spawn_blocking(move || {
        if db_topics.is_empty() {
            // 如果数据库中没有 topics，尝试从 Kafka 自动获取
            cg_manager_clone.get_consumer_group_offsets_auto(&group_name_clone)
        } else {
            // 使用数据库中的 topics 获取 offsets
            cg_manager_clone.get_consumer_group_offsets(&group_name_clone, &db_topics)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    tracing::info!("[handle_consumer_group_offsets] Got {} offsets", offsets.len());

    // 批量获取所有分区的最后提交时间
    let partitions: Vec<(String, i32)> = offsets
        .iter()
        .map(|o| (o.topic.clone(), o.partition))
        .collect();

    if !partitions.is_empty() {
        match cg_manager.get_partitions_last_commit_time(&group_name, &partitions) {
            Ok(times) => {
                for (i, offset) in offsets.iter_mut().enumerate() {
                    if i < times.len() {
                        offset.last_commit_time = times[i];
                        tracing::info!("[handle_consumer_group_offsets] Got last_commit_time for {}/{}: {:?}",
                            offset.topic, offset.partition, times[i]);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[handle_consumer_group_offsets] Failed to get last_commit_time: {}", e);
            }
        }
    }

    let offsets_json: Vec<Value> = offsets
        .into_iter()
        .map(|o| {
            serde_json::json!({
                "topic": o.topic,
                "partition": o.partition,
                "start_offset": o.start_offset,
                "end_offset": o.end_offset,
                "committed_offset": o.committed_offset,
                "lag": o.lag,
                "last_commit_time": o.last_commit_time,
            })
        })
        .collect();

    Ok(serde_json::json!({ "offsets": offsets_json }))
}

/// 重置 Consumer Group 的 offset
async fn handle_consumer_group_reset_offset(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_cluster_id_param(&body)?;
    let group_name = get_string_param(&body, "group_name")?;
    let topic = get_string_param(&body, "topic")?;
    let partition = get_i32_param(&body, "partition")?;
    let reset_to = get_string_param(&body, "reset_to")?;  // "earliest", "latest", "offset", or "timestamp"
    let timestamp = get_optional_i64_param(&body, "timestamp");
    let offset = get_optional_i64_param(&body, "offset");

    tracing::info!("Reset offset request: cluster={}, group={}, topic={}, partition={}, reset_to={}",
                   cluster_id, group_name, topic, partition, reset_to);

    // 确保集群客户端已创建
    let config = ensure_cluster_client(&state, &cluster_id).await?;
    tracing::info!("Got config for cluster: {}", cluster_id);

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;
    tracing::info!("Created ConsumerGroupManager");

    // 在阻塞线程中执行 Kafka 操作
    let new_offset = tokio::task::spawn_blocking(move || -> Result<i64> {
        tracing::info!("Spawn blocking task for reset offset");
        let result = match reset_to.as_str() {
            "earliest" => cg_manager.reset_consumer_group_offset_to_earliest(&group_name, &topic, partition),
            "latest" => cg_manager.reset_consumer_group_offset_to_latest(&group_name, &topic, partition),
            "offset" => {
                let off = offset.ok_or_else(|| AppError::BadRequest("offset is required when reset_to is 'offset'".to_string()))?;
                cg_manager.reset_consumer_group_offset(&group_name, &topic, partition, off)
            }
            "timestamp" => {
                let ts = timestamp.ok_or_else(|| AppError::BadRequest("timestamp is required when reset_to is 'timestamp'".to_string()))?;
                cg_manager.reset_consumer_group_offset_to_timestamp(&group_name, &topic, partition, ts)
            }
            _ => Err(AppError::BadRequest(format!("Invalid reset_to value: {}. Must be 'earliest', 'latest', 'offset', or 'timestamp'", reset_to))),
        };
        tracing::info!("Spawn blocking task completed: {:?}", result.as_ref().map(|o| o.to_string()).unwrap_or_else(|e| format!("Error: {}", e)));
        result
    })
    .await
    .map_err(|e| {
        tracing::error!("Task join error: {}", e);
        AppError::Internal(format!("Task failed: {}", e))
    })??;

    tracing::info!("Reset offset successful: new_offset={}", new_offset);

    Ok(serde_json::json!({
        "success": true,
        "new_offset": new_offset,
    }))
}

/// 删除 Consumer Group
async fn handle_consumer_group_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_cluster_id_param(&body)?;
    let group_name = get_string_param(&body, "group_name")?;

    // 确保集群客户端已创建
    let config = ensure_cluster_client(&state, &cluster_id).await?;

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;

    // 克隆 group_name 用于闭包和后续使用
    let group_name_clone = group_name.clone();

    // 在阻塞线程中执行 Kafka 操作
    tokio::task::spawn_blocking(move || {
        cg_manager.delete_empty_consumer_group(&group_name_clone)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    // 从数据库中删除
    ConsumerGroupStore::delete(state.db.inner(), &cluster_id, &group_name).await?;
    ConsumerGroupStore::delete_offsets(state.db.inner(), &cluster_id, &group_name).await?;

    Ok(serde_json::json!({ "success": true }))
}

// ==================== Telemetry ====================

/// 检查 MySQL 连接是否可用
async fn handle_telemetry_check_connection(_state: AppState) -> Result<Value> {
    // 只检查 TCP 连接，不建立 MySQL 连接（更快，不阻塞用户）
    let tcp_connected = telemetry::check_mysql_connection().await;

    if !tcp_connected {
        return Ok(serde_json::json!({
            "connected": false,
            "reason": "TCP connection failed"
        }));
    }

    Ok(serde_json::json!({
        "connected": true,
        "hostname": telemetry::get_hostname(),
        "username": telemetry::get_username(),
        "local_ip": telemetry::get_local_ip(),
        "app_version": telemetry::get_app_version(),
        "platform": telemetry::get_platform(),
        "install_method": telemetry::get_install_method()
    }))
}

/// 上报遥测数据
async fn handle_telemetry_report(state: AppState) -> Result<Value> {
    // 检查 TCP 连接
    if !telemetry::check_mysql_connection().await {
        return Ok(serde_json::json!({
            "success": false,
            "reason": "MySQL TCP connection not available"
        }));
    }

    // 建立 MySQL 连接
    let mysql_pool = telemetry::connect_mysql().await
        .map_err(|e| AppError::Internal(format!("MySQL connection error: {}", e)))?;

    // 执行遥测上报
    let reported = telemetry::do_telemetry_report(state.db.inner(), &mysql_pool).await
        .map_err(|e| AppError::Internal(format!("Telemetry report error: {}", e)))?;

    Ok(serde_json::json!({
        "success": true,
        "reported": reported,
        "hostname": telemetry::get_hostname(),
        "username": telemetry::get_username(),
        "local_ip": telemetry::get_local_ip(),
        "app_version": telemetry::get_app_version(),
        "platform": telemetry::get_platform(),
        "install_method": telemetry::get_install_method()
    }))
}

/// 提交意见反馈
async fn handle_telemetry_submit_feedback(_state: AppState, body: Value) -> Result<Value> {
    let feedback_content = get_long_string_param(&body, "feedback_content")?;

    // 验证反馈内容长度（最大 2000 字符）
    if feedback_content.len() > 2000 {
        return Err(AppError::BadRequest("Feedback content exceeds maximum length of 2000 characters".to_string()));
    }

    // 检查 TCP 连接
    if !telemetry::check_mysql_connection().await {
        // TCP 连接失败，静默返回（不影响用户）
        tracing::warn!("[Telemetry] MySQL TCP connection not available, feedback skipped");
        return Ok(serde_json::json!({
            "success": true,
            "skipped": true,
            "reason": "MySQL connection not available"
        }));
    }

    // 建立 MySQL 连接（失败时静默返回）
    let mysql_pool = match telemetry::connect_mysql().await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::warn!("[Telemetry] MySQL connection failed: {}, feedback skipped", e);
            return Ok(serde_json::json!({
                "success": true,
                "skipped": true,
                "reason": format!("MySQL connection error: {}", e)
            }));
        }
    };

    // 获取系统信息
    let hostname = telemetry::get_hostname();
    let username = telemetry::get_username();
    let local_ip = telemetry::get_local_ip();
    let app_version = telemetry::get_app_version();
    let platform = telemetry::get_platform();
    let install_method = telemetry::get_install_method();
    let submitted_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 提交反馈到 MySQL（失败时静默返回）
    match telemetry::submit_feedback_to_mysql(
        &mysql_pool,
        &hostname,
        &username,
        &local_ip,
        &app_version,
        &platform,
        &install_method,
        &feedback_content,
        &submitted_at,
    ).await {
        Ok(feedback_id) => {
            Ok(serde_json::json!({
                "success": true,
                "feedback_id": feedback_id,
                "hostname": hostname,
                "username": username,
                "local_ip": local_ip,
                "app_version": app_version,
                "platform": platform,
                "install_method": install_method,
                "submitted_at": submitted_at
            }))
        }
        Err(e) => {
            tracing::warn!("[Telemetry] Feedback submission failed: {}, skipped", e);
            Ok(serde_json::json!({
                "success": true,
                "skipped": true,
                "reason": format!("Submission error: {}", e)
            }))
        }
    }
}
