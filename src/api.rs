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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

/// 流式消息事件（通过 Tauri Channel 发送给前端，data 为结构化 JSON 对象，
/// 前端直接使用，无需 JSON.parse——避免消息体被双重序列化）
#[derive(Clone, Debug, serde::Serialize)]
pub struct StreamEvent {
    pub event: String,
    pub data: Value,
}

impl StreamEvent {
    fn new(event: &str, data: Value) -> Self {
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
/// 事件流：start -> batch* -> complete / error（desc 查询直接降序推送，无 order 事件）
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
    // 前端已查询过 topic 详情时可透传分区列表，省掉一次 fetch_metadata
    let partitions_hint: Option<Vec<i32>> = body.get("partitions")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_i64().map(|n| n as i32)).filter(|p| *p >= 0).collect());

    // 首先确保集群客户端已创建
    let config = ensure_cluster_client(&state, &cluster_id).await?;
    let max_msgs = limit.or(max_messages).unwrap_or(100);

    // Schema Registry 配置：流式列表与非流式一致解码 Avro/Protobuf
    let schema_info = {
        let pool = state.get_pool();
        let cfg = SchemaRegistryStore::get_config(&pool, &cluster_id).await.ok().flatten();
        if cfg.is_some() {
            SchemaStore::get_latest_schema(&pool, &cluster_id, &topic).await.ok().flatten()
                .map(|s| (s.schema_type, s.schema_json))
        } else {
            None
        }
    };

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
            partitions_hint,
            schema_info,
            tx.clone(),
            cancel_token_clone,
        ).await;

        match result {
            Ok(actual_total) => {
                // 发送完成标记（带实际发送条数，过滤查询时远小于 total_target）
                let _ = tx.send(StreamEvent::new("complete", serde_json::json!({ "actual_total": actual_total }))).await;
            }
            Err(e) => {
                // 发送错误
                let error_json = serde_json::json!({"error": e.to_string()});
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
        "message.get" => handle_message_get(state, body).await,
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

            // 尝试使用 Schema 解码消息（与流式路径共用同一 helper）
            if let Some((ref _config, ref schema)) = schema_info {
                if let Some(decoded) = try_decode_schema_value(&schema.schema_type, &schema.schema_json, &value) {
                    value = decoded;
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

// ============================================================
// 消息查询引擎（流式 / 非流式 / 单条获取共用）
//
// 设计要点：
// 1. 单 consumer 手动 assign 所有分区：整个查询只占 1 条 broker 连接，
//    metadata / offsets_for_times 批量完成（原实现每分区一个 consumer + 一条 TCP 连接）
// 2. enable.partition.eof=true：以 broker 的 PartitionEOF 作为分区读完的权威信号，
//    搜索/时间范围无匹配时秒级结束（原实现要靠 30s+ 饥饿等待兜底）
// 3. K 路归并：分区内的消息按 offset 顺序到达，堆顶只有在所有活跃分区都有候选时才弹出，
//    保证全局按 (timestamp, offset) 有序输出；desc 用大顶堆直接降序输出，
//    前端无需再整体反转
// 4. 引擎为同步阻塞实现，调用方必须用 spawn_blocking 包裹，
//    避免 consumer.poll 阻塞 tokio worker 线程（原实现直接跑在 worker 上）
// ============================================================

/// 列表内联 value 上限（128KB）：超出则截断并置 value_truncated 标记，
/// 前端通过 message.get 按需拉取完整内容
const MAX_INLINE_VALUE_BYTES: usize = 128 * 1024;
/// 流式推送每批消息条数
const STREAM_BATCH_SIZE: usize = 500;
/// 流式推送每批字节数上限：大 value 查询时按条数攒批内存不可控（500×128KB≈64MB/批）
const STREAM_BATCH_BYTES: usize = 8 * 1024 * 1024;
/// 查询总时长上限：留 5s 余量，确保在前端 90s 超时前完成收尾
const MAX_QUERY_TIME_SECS: u64 = 85;
/// 收到首条消息前的等待上限（必须超过 socket.timeout.ms=60s，慢 broker 的首个 Fetch 才有机会完成）
const FIRST_MESSAGE_TIMEOUT_SECS: u64 = 75;
/// 收到首条消息后的饥饿等待上限（正常路径由 EOF 结束，这里仅作 EOF 未触发的兜底）
const STARVATION_SECS: u64 = 30;

/// 查询类 consumer 的统一配置（流式/非流式/单条获取共用，避免多处复制导致配置漂移。
/// 原实现 max.partition.fetch.bytes 被设置两次，大批量分支的 50MB 总是被覆盖回 10MB）
///
/// 注意：查询路径必须保持手动 assign + 不提交 offset，禁止 subscribe / commit——
/// 手动 assign 不进组（无心跳/rebalance），不 commit 则不写 __consumer_offsets，
/// broker 上不会注册消费者组实体，查询结束断开即无残留。
fn build_query_consumer_config(brokers: &str, group_id: &str, large_fetch: bool) -> rdkafka::ClientConfig {
    let mut cfg = rdkafka::ClientConfig::new();
    cfg.set("bootstrap.servers", brokers)
        .set("group.id", group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        // EOF 作为分区读完的权威信号
        .set("enable.partition.eof", "true")
        // 强制使用 IPv4，避免 IPv6 连接问题
        .set("broker.address.family", "v4")
        .set("socket.nagle.disable", "true")
        // 不显式设置 socket.receive.buffer.bytes：Linux 下显式 SO_RCVBUF 会关闭内核
        // autotune，固定 256KB 窗口把高延迟链路吞吐钉死在 256KB/RTT；交给 OS autotune
        // FetchRequest 超时 = socket.timeout.ms。慢 broker 响应可能超过 10s，放宽到 60s（librdkafka 默认值）
        .set("socket.timeout.ms", "60000")
        .set("connections.max.idle.ms", "540000")
        .set("reconnect.backoff.ms", "50")
        .set("reconnect.backoff.max.ms", "500")
        .set("socket.connection.setup.timeout.ms", "3000")
        // 不设置 metadata.max.age.ms（默认 5min）：临时 consumer 全新启动必然现取 metadata，
        // 过激的 5s 刷新只会在长查询中途往 fetch 流水线里插入额外 RTT
        // 允许大消息（必须 >= max.partition.fetch.bytes）
        .set("fetch.message.max.bytes", "52428800");
    if large_fetch {
        cfg.set("fetch.min.bytes", "65536")
            .set("fetch.wait.max.ms", "100")
            .set("fetch.max.bytes", "52428800")
            .set("max.partition.fetch.bytes", "52428800");
    } else {
        cfg.set("fetch.min.bytes", "1")
            .set("fetch.wait.max.ms", "50")
            .set("fetch.max.bytes", "10485760")
            .set("max.partition.fetch.bytes", "10485760");
    }
    cfg
}

/// 不区分大小写的搜索词（ASCII 路径零分配；非 ASCII 回退到 Unicode 小写比较）
enum SearchTerm {
    Ascii(Vec<u8>),
    Unicode(String),
}

fn prepare_search_term(term: &str) -> SearchTerm {
    if term.is_ascii() {
        SearchTerm::Ascii(term.to_ascii_lowercase().into_bytes())
    } else {
        SearchTerm::Unicode(term.to_lowercase())
    }
}

fn bytes_contain_ci(haystack: &[u8], term: &SearchTerm) -> bool {
    match term {
        SearchTerm::Ascii(needle) => {
            !needle.is_empty()
                && haystack.len() >= needle.len()
                && haystack.windows(needle.len()).any(|w| w.eq_ignore_ascii_case(needle))
        }
        SearchTerm::Unicode(needle) => std::str::from_utf8(haystack)
            .map_or(false, |s| s.to_lowercase().contains(needle.as_str())),
    }
}

/// 检查消息是否匹配搜索条件（直接在原始字节上匹配，不做 UTF-8 转换和逐消息小写分配）
fn message_matches_search(
    key: Option<&[u8]>,
    value: Option<&[u8]>,
    term: &SearchTerm,
    search_in: Option<&str>,
) -> bool {
    match search_in.unwrap_or("all") {
        "key" => key.map_or(false, |k| bytes_contain_ci(k, term)),
        "value" => value.map_or(false, |v| bytes_contain_ci(v, term)),
        _ => {
            key.map_or(false, |k| bytes_contain_ci(k, term))
                || value.map_or(false, |v| bytes_contain_ci(v, term))
        }
    }
}

/// 转换 payload 为 String；超过 limit 时截断到 UTF-8 字符边界并返回 truncated=true
fn convert_payload(bytes: Option<&[u8]>, limit: Option<usize>) -> (Option<String>, bool) {
    let bytes = match bytes {
        Some(b) => b,
        None => return (None, false),
    };
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return (None, false),
    };
    if let Some(max) = limit {
        if s.len() > max {
            let mut end = max;
            while !s.is_char_boundary(end) {
                end -= 1;
            }
            return (Some(s[..end].to_string()), true);
        }
    }
    (Some(s.to_string()), false)
}

/// 按 Schema Registry 的 schema 解码消息 value（base64 文本 → Avro/Protobuf → JSON 字符串）
/// 流式/非流式列表路径与 message.get 共用；非 base64、解码失败、非 AVRO/PROTOBUF 均返回 None（保留原值）
fn try_decode_schema_value(schema_type: &str, schema_json: &str, value: &str) -> Option<String> {
    let decoded_bytes = base64::engine::general_purpose::STANDARD.decode(value).ok()?;
    let json_value = match schema_type {
        "AVRO" => AvroCodec::decode(schema_json, &decoded_bytes).ok()?,
        "PROTOBUF" => ProtobufCodec::decode_simple(schema_json, &decoded_bytes).ok()?,
        _ => return None,
    };
    serde_json::to_string(&json_value).ok()
}

/// 批量计算所有分区的读取范围（start/end offset 均 inclusive）
/// offsets_for_times 一次 RPC 覆盖全部分区（原实现每分区各 2 次 RPC）
fn calculate_offsets_batch(
    consumer: &rdkafka::consumer::BaseConsumer,
    topic: &str,
    partitions: &[i32],
    max_messages: usize,
    specific_offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    fetch_mode: Option<&str>,
) -> Result<HashMap<i32, TimeRangeInfo>> {
    use rdkafka::consumer::Consumer;
    use rdkafka::TopicPartitionList;
    use std::time::Duration;

    let empty_range = |low: i64, high: i64| TimeRangeInfo {
        start_offset: low,
        end_offset: low - 1, // start > end 表示空范围
        low_watermark: low,
        high_watermark: high,
    };

    // 1. watermarks：两次批量 ListOffsets RPC 覆盖全部分区（原实现每分区串行一次 RPC，
    //    慢链路 N×RTT）；批量失败的分区回退串行 fetch_watermarks（共享同一 consumer/连接）
    let watermarks = fetch_watermarks_batch(consumer, topic, partitions)?;

    // 2. 批量 offsets_for_times：start_time / end_time 各一次 RPC
    let query_offsets_for_time = |ts: i64| -> HashMap<i32, i64> {
        let mut tpl = TopicPartitionList::new();
        let mut has_valid = false;
        for &p in partitions {
            let (low, high) = watermarks[&p];
            if low < high {
                tpl.add_partition_offset(topic, p, rdkafka::Offset::Offset(ts)).ok();
                has_valid = true;
            }
        }
        let mut map = HashMap::new();
        if !has_valid {
            return map;
        }
        // 30s 超时 + 1 次重试：慢 broker 上一次 15s 超时会静默退化为全窗口扫描
        for attempt in 1..=2 {
            match consumer.offsets_for_times(tpl.clone(), Duration::from_secs(30)) {
                Ok(r) => {
                    for elem in r.elements_for_topic(topic) {
                        // 分区级错误（如 LeaderNotAvailable）不得入图：其 offset 为哨兵负值，
                        // 原逻辑会落到 Some(_) 分支被当成"时间戳晚于所有消息"→ 空范围，
                        // 静默丢失该分区数据。跳过让它走 None 回退（watermark 全窗口，保守多查不丢数据）
                        if elem.error().is_err() {
                            continue;
                        }
                        map.insert(elem.partition(), elem.offset().to_raw().unwrap_or(-1));
                    }
                    break;
                }
                Err(e) => {
                    tracing::warn!("[Query] offsets_for_times(ts={}) attempt {}/2 failed: {}", ts, attempt, e);
                }
            }
        }
        if map.is_empty() {
            tracing::warn!("[Query] offsets_for_times(ts={}) failed after retries, falling back to watermarks", ts);
        }
        map
    };

    let start_offsets = match start_time {
        Some(ts) if ts > 0 => Some(query_offsets_for_time(ts)),
        _ => None,
    };
    let end_offsets = match end_time {
        Some(ts) if ts > 0 => Some(query_offsets_for_time(ts)),
        _ => None,
    };
    let has_time_range = start_offsets.is_some() || end_offsets.is_some();

    // start_time > end_time 的范围整体无效
    let invalid_range = matches!((start_time, end_time), (Some(s), Some(e)) if s > e);

    let mut result = HashMap::with_capacity(partitions.len());
    for &p in partitions {
        let (low, high) = watermarks[&p];
        if low >= high || invalid_range {
            result.insert(p, empty_range(low, high));
            continue;
        }
        let high_offset = high - 1;

        // 用户指定 offset（仅单分区查询时传入）：从该 offset 读到末尾
        if let Some(off) = specific_offset {
            if off >= 0 {
                result.insert(p, TimeRangeInfo {
                    start_offset: off,
                    end_offset: high_offset,
                    low_watermark: low,
                    high_watermark: high,
                });
                continue;
            }
        }

        if has_time_range {
            let mut start_off = match &start_offsets {
                Some(m) => match m.get(&p) {
                    Some(&raw) if raw >= 0 => raw.clamp(low, high_offset),
                    Some(_) => high_offset, // -1：时间戳晚于所有消息
                    None => low,            // RPC 失败/缺失：回退 low watermark
                },
                None => low,
            };
            let mut end_off = match &end_offsets {
                Some(m) => match m.get(&p) {
                    // end_time 对应的 offset 是 >= 该时间的第一条消息，有效结束为其 - 1
                    Some(&raw) if raw >= 0 => raw.saturating_sub(1).clamp(low, high_offset),
                    Some(_) => high_offset,
                    None => high_offset,
                },
                None => high_offset,
            };
            if start_off > end_off {
                std::mem::swap(&mut start_off, &mut end_off);
            }
            // newest：从范围尾部向前取 max_messages 条
            let start_off = match fetch_mode {
                Some("newest") => {
                    let range_size = end_off - start_off + 1;
                    let to_fetch = (max_messages as i64).min(range_size);
                    (end_off - to_fetch + 1).max(start_off)
                }
                _ => start_off,
            };
            result.insert(p, TimeRangeInfo {
                start_offset: start_off,
                end_offset: end_off,
                low_watermark: low,
                high_watermark: high,
            });
            continue;
        }

        // 无时间范围：按 fetch_mode 取头部/尾部窗口
        match fetch_mode {
            Some("oldest") => {
                result.insert(p, TimeRangeInfo {
                    start_offset: low,
                    end_offset: high_offset,
                    low_watermark: low,
                    high_watermark: high,
                });
            }
            _ => {
                // newest / 默认：从尾部向前取 max_messages 条
                let start = (high_offset - (max_messages.saturating_sub(1)) as i64).max(low);
                result.insert(p, TimeRangeInfo {
                    start_offset: start,
                    end_offset: high_offset,
                    low_watermark: low,
                    high_watermark: high,
                });
            }
        }
    }

    Ok(result)
}

/// 归并堆节点：asc 时堆顶是最早消息（最小堆），desc 时堆顶是最晚消息（最大堆）
struct HeapEntry {
    desc: bool,
    timestamp: Option<i64>,
    offset: i64,
    part: usize, // 分区状态数组下标
    msg: crate::kafka::consumer::KafkaMessage,
}

impl HeapEntry {
    /// 升序基准比较：时间戳（None 排最后），再 offset
    fn asc_cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self.timestamp, other.timestamp) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| self.offset.cmp(&other.offset))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let c = self.asc_cmp(other);
        // BinaryHeap 弹出"最大"元素：asc 反转为最小堆，desc 保持最大堆
        if self.desc { c } else { c.reverse() }
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for HeapEntry {}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && self.offset == other.offset
    }
}

/// 引擎回调返回值：Continue 继续拉取，Stop 表示下游关闭/取消（立即停止）
enum Emit {
    Continue,
    Stop,
}

struct QueryParams {
    brokers: String,
    topic: String,
    /// 目标分区；空 = 由查询 consumer 自己的 metadata 解析（复用同一连接，
    /// 省掉原实现为拿分区列表单独建 consumer 的一次 TCP 连接 + metadata RTT）
    partitions: Vec<i32>,
    /// 用户指定 offset（仅单分区查询有效）
    offset: Option<i64>,
    /// 每分区最多拉取条数
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    search_in: Option<String>,
    fetch_mode: Option<String>,
    is_desc: bool,
    /// value 内联上限（流式列表路径 Some；导出等非流式路径 None 保留完整内容）
    truncate_value: Option<usize>,
    /// 分区解析完成回调（流式路径用来推迟发送 start 事件，带上准确的分区数）
    on_partitions_resolved: Option<Box<dyn FnOnce(usize) + Send>>,
}

/// 消息查询引擎：单 consumer 读取所有分区 + K 路归并有序输出
/// 阻塞实现，调用方必须用 spawn_blocking 包裹。返回成功发送的消息条数
fn run_message_query(
    mut params: QueryParams,
    cancel: CancellationToken,
    mut emit: impl FnMut(crate::kafka::consumer::KafkaMessage) -> Emit,
) -> Result<usize> {
    use rdkafka::consumer::{BaseConsumer, Consumer, DefaultConsumerContext};
    use rdkafka::{Message, TopicPartitionList};
    use std::time::{Duration, Instant};

    let query_start = Instant::now();
    tracing::info!(
        "[Query] topic={}, partitions={:?}, max_messages={}/partition, fetch_mode={:?}, desc={}",
        params.topic, params.partitions, params.max_messages, params.fetch_mode, params.is_desc
    );

    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let group_id = format!("kafka-mgr-query-{}-{}", std::process::id(), unique_suffix);
    let cfg = build_query_consumer_config(&params.brokers, &group_id, params.max_messages > 1000);
    let consumer: BaseConsumer<DefaultConsumerContext> = cfg.create()?;

    // 分区列表：调用方未提供时用这个 consumer 自己的 metadata 解析
    if params.partitions.is_empty() {
        params.partitions = resolve_partitions(&consumer, &params.topic)?;
    }
    if let Some(cb) = params.on_partitions_resolved.take() {
        cb(params.partitions.len());
    }
    let partitions = params.partitions.as_slice();

    // 批量计算每个分区的读取范围
    let ranges = calculate_offsets_batch(
        &consumer,
        &params.topic,
        partitions,
        params.max_messages,
        params.offset,
        params.start_time,
        params.end_time,
        params.fetch_mode.as_deref(),
    )?;

    // 分区状态 + assign（空分区/空范围直接标记完成，不参与拉取）
    struct PState {
        start_offset: i64,
        end_offset: i64, // inclusive
        remaining: usize,
        done: bool,
        in_heap: usize,
    }
    let mut states: Vec<PState> = Vec::with_capacity(params.partitions.len());
    let mut part_index: HashMap<i32, usize> = HashMap::with_capacity(params.partitions.len());
    let mut tpl = TopicPartitionList::new();
    let mut active_count = 0usize;
    for (idx, &p) in params.partitions.iter().enumerate() {
        let tr = &ranges[&p];
        let done = params.max_messages == 0
            || tr.start_offset > tr.end_offset
            || tr.high_watermark <= tr.low_watermark
            || tr.start_offset >= tr.high_watermark;
        if !done {
            tpl.add_partition_offset(&params.topic, p, rdkafka::Offset::Offset(tr.start_offset))?;
            active_count += 1;
        }
        states.push(PState {
            start_offset: tr.start_offset,
            end_offset: tr.end_offset,
            remaining: params.max_messages,
            done,
            in_heap: 0,
        });
        part_index.insert(p, idx);
    }

    if active_count == 0 {
        tracing::info!("[Query] no partition has data in range, nothing to fetch");
        return Ok(0);
    }
    consumer.assign(&tpl)?;

    let search_term = params
        .search
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(prepare_search_term);

    let desc = params.is_desc;
    let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::with_capacity(states.len());
    // 活跃但没有堆候选的分区数：归 0 时才允许弹出堆顶（保证全局有序）
    let mut unrepresented = active_count;
    let mut sent = 0usize;
    let mut stopped = false;
    let mut got_any = false;
    let mut last_msg_at = Instant::now();
    let mut consecutive_errors = 0u32;

    macro_rules! mark_done {
        ($idx:expr) => {{
            let st = &mut states[$idx];
            if !st.done {
                st.done = true;
                active_count -= 1;
                if st.in_heap == 0 {
                    unrepresented -= 1;
                }
            }
        }};
    }

    loop {
        if cancel.is_cancelled() {
            tracing::info!("[Query] cancelled by client, sent={}", sent);
            break;
        }

        // 所有活跃分区都有候选 → 弹出堆顶发送
        if unrepresented == 0 {
            if let Some(top) = heap.pop() {
                let st = &mut states[top.part];
                st.in_heap -= 1;
                if !st.done && st.in_heap == 0 {
                    unrepresented += 1;
                }
                sent += 1;
                if let Emit::Stop = emit(top.msg) {
                    stopped = true;
                    break;
                }
                continue;
            }
        }

        // 全部分区完成 → 排空堆中剩余后结束
        if active_count == 0 {
            while let Some(top) = heap.pop() {
                sent += 1;
                if let Emit::Stop = emit(top.msg) {
                    stopped = true;
                    break;
                }
            }
            break;
        }

        // 全局兜底超时（正常路径由 PartitionEOF 结束）
        let stall_limit = if got_any { STARVATION_SECS } else { FIRST_MESSAGE_TIMEOUT_SECS };
        if query_start.elapsed() >= Duration::from_secs(MAX_QUERY_TIME_SECS)
            || last_msg_at.elapsed() >= Duration::from_secs(stall_limit)
        {
            tracing::warn!("[Query] timeout fallback: sent={}, active_partitions={}", sent, active_count);
            break;
        }

        match consumer.poll(Duration::from_millis(200)) {
            Some(Ok(msg)) => {
                got_any = true;
                last_msg_at = Instant::now();
                consecutive_errors = 0;

                let idx = match part_index.get(&msg.partition()) {
                    Some(&i) => i,
                    None => continue,
                };
                if states[idx].done {
                    continue;
                }
                let msg_offset = msg.offset();

                // offset 被重置（如 assign 后消息被 compact 清理）→ 跳到范围起点前的不算
                if msg_offset < states[idx].start_offset {
                    continue;
                }
                // 超出范围末尾 → 分区完成
                if msg_offset > states[idx].end_offset {
                    mark_done!(idx);
                    continue;
                }

                let ts = msg.timestamp().to_millis();
                // 时间范围过滤（seek 已对齐，这里防御时间戳乱序/CreateTime 场景）
                if let (Some(start), Some(t)) = (params.start_time, ts) {
                    if t < start {
                        continue;
                    }
                }
                if let (Some(end), Some(t)) = (params.end_time, ts) {
                    if t > end {
                        // 后面的消息时间戳只会更大（近似），直接结束该分区
                        mark_done!(idx);
                        continue;
                    }
                }

                // 搜索过滤
                if let Some(term) = &search_term {
                    if !message_matches_search(msg.key(), msg.payload(), term, params.search_in.as_deref()) {
                        continue;
                    }
                }

                let (key, _) = convert_payload(msg.key(), None);
                let (value, value_truncated) = convert_payload(msg.payload(), params.truncate_value);
                let kafka_msg = crate::kafka::consumer::KafkaMessage {
                    partition: msg.partition(),
                    offset: msg_offset,
                    key,
                    value,
                    timestamp: ts,
                    value_truncated,
                };
                let st = &mut states[idx];
                if st.in_heap == 0 {
                    unrepresented -= 1;
                }
                st.in_heap += 1;
                st.remaining -= 1;
                let reached_limit = st.remaining == 0;
                heap.push(HeapEntry { desc, timestamp: ts, offset: msg_offset, part: idx, msg: kafka_msg });
                if reached_limit {
                    mark_done!(idx);
                }
            }
            Some(Err(rdkafka::error::KafkaError::PartitionEOF(p))) => {
                // broker 权威信号：该分区已读到末尾
                if let Some(&idx) = part_index.get(&p) {
                    tracing::debug!("[Query] partition {} reached EOF", p);
                    mark_done!(idx);
                }
            }
            Some(Err(e)) => {
                consecutive_errors += 1;
                tracing::warn!("[Query] poll error ({} consecutive): {}", consecutive_errors, e);
                if consecutive_errors >= 50 {
                    return Err(AppError::Kafka(e));
                }
            }
            None => {}
        }
    }

    if !stopped {
        // 取消/超时路径：把堆中已收集的消息发完
        while let Some(top) = heap.pop() {
            sent += 1;
            if let Emit::Stop = emit(top.msg) {
                break;
            }
        }
    }

    tracing::info!(
        "[Query] done: sent={} from {} partitions in {:?}",
        sent, params.partitions.len(), query_start.elapsed()
    );
    Ok(sent)
}

/// 流式查询的批次发送器：攒批 + try_send 背压
/// 不能用 blocking_send/send().await——channel 满时取消信号无法唤醒，会造成死锁
struct StreamBatcher {
    tx: mpsc::Sender<StreamEvent>,
    cancel: CancellationToken,
    batch: Vec<Value>,
    /// 当前批的字节数估计（key+value）：大 value 查询时按条数攒批内存不可控
    batch_bytes: usize,
    sent: usize,
    /// 目标总数（分区解析完成后才有准确值），随批事件带给前端做进度
    total: Arc<AtomicUsize>,
    /// Schema Registry 解码（schema_type, schema_json），与非流式列表路径行为一致
    schema: Option<(String, String)>,
}

impl StreamBatcher {
    fn emit(&mut self, mut msg: crate::kafka::consumer::KafkaMessage) -> Emit {
        if let (Some((ref ty, ref js)), Some(v)) = (&self.schema, &msg.value) {
            if let Some(decoded) = try_decode_schema_value(ty, js, v) {
                msg.value = Some(decoded);
            }
        }
        self.batch_bytes += msg.key.as_deref().map_or(0, str::len)
            + msg.value.as_deref().map_or(0, str::len);
        self.batch.push(msg.to_json_value());
        self.sent += 1;
        if self.batch.len() >= STREAM_BATCH_SIZE || self.batch_bytes >= STREAM_BATCH_BYTES {
            return self.flush();
        }
        Emit::Continue
    }

    fn flush(&mut self) -> Emit {
        if self.batch.is_empty() {
            return Emit::Continue;
        }
        let messages = std::mem::replace(&mut self.batch, Vec::with_capacity(STREAM_BATCH_SIZE));
        self.batch_bytes = 0;
        let data = serde_json::json!({
            "messages": messages,
            "progress": self.sent,
            "total": self.total.load(Ordering::Relaxed),
        });
        let mut evt = StreamEvent::new("batch", data);
        // 背压：channel 满时等待并响应取消
        loop {
            if self.cancel.is_cancelled() {
                return Emit::Stop;
            }
            match self.tx.try_send(evt) {
                Ok(()) => return Emit::Continue,
                Err(mpsc::error::TrySendError::Full(e)) => {
                    evt = e;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(mpsc::error::TrySendError::Closed(_)) => return Emit::Stop,
            }
        }
    }
}

/// 流式消息获取：单 consumer 全分区读取 + K 路归并 + 实时推送
/// 返回实际发送的消息条数（由调用方写入 complete 事件的 actual_total）
///
/// start 事件在引擎解析完分区后发送（带准确的分区数/total_target）：分区解析已合并进
/// 查询 consumer（省一条 TCP 连接 + metadata RTT），此处不再预知分区数
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
    partitions_hint: Option<Vec<i32>>,
    schema_info: Option<(String, String)>,
    sse_tx: mpsc::Sender<StreamEvent>,
    cancel_token: CancellationToken,
) -> Result<usize> {
    let query_start = std::time::Instant::now();
    let is_desc = sort == Some("desc") || (sort.is_none() && fetch_mode != Some("oldest"));
    let has_filter = search.is_some() || start_time.is_some() || end_time.is_some();

    // 分区列表：优先前端透传；否则留空由查询 consumer 自己的 metadata 解析
    let partitions: Vec<i32> = match partition {
        Some(p) => vec![p],
        None => match partitions_hint {
            Some(ref hint) if !hint.is_empty() => hint.clone(),
            _ => Vec::new(),
        },
    };

    // 目标总数：hint 已知则立即准确；否则等引擎解析后由回调填入（批事件随带）
    let total_shared = Arc::new(AtomicUsize::new(max_messages * partitions.len()));
    tracing::info!(
        "[Stream] topic={}, {} partitions (hint), {} msgs/partition, desc={}, has_filter={}",
        topic, partitions.len(), max_messages, is_desc, has_filter
    );

    // start 事件推迟到引擎解析完分区后发送（has_filter 提示前端进度按不确定模式展示）
    let start_tx = sse_tx.clone();
    let total_for_cb = total_shared.clone();
    let on_partitions_resolved = Box::new(move |partition_count: usize| {
        let total_target = max_messages * partition_count;
        total_for_cb.store(total_target, Ordering::Relaxed);
        let _ = start_tx.try_send(StreamEvent::new("start", serde_json::json!({
            "partitions": partition_count,
            "total_target": total_target,
            "has_filter": has_filter,
        })));
    });

    let params = QueryParams {
        brokers: brokers.to_string(),
        topic: topic.to_string(),
        partitions,
        // 用户指定 offset 仅在单分区查询时生效（与原行为一致）
        offset: if partition.is_some() { offset } else { None },
        max_messages,
        start_time,
        end_time,
        search,
        search_in,
        fetch_mode: fetch_mode.map(|s| s.to_string()),
        is_desc,
        truncate_value: Some(MAX_INLINE_VALUE_BYTES),
        on_partitions_resolved: Some(on_partitions_resolved),
    };

    let cancel = cancel_token.clone();
    let sent = tokio::task::spawn_blocking(move || {
        let mut batcher = StreamBatcher {
            tx: sse_tx,
            cancel: cancel.clone(),
            batch: Vec::with_capacity(STREAM_BATCH_SIZE),
            batch_bytes: 0,
            sent: 0,
            total: total_shared,
            schema: schema_info,
        };
        let result = run_message_query(params, cancel, |msg| batcher.emit(msg));
        let _ = batcher.flush();
        result
    })
    .await
    .map_err(|e| AppError::Internal(format!("Query task join error: {}", e)))??;

    tracing::info!("[Stream] completed: sent {} in {:?}", sent, query_start.elapsed());
    Ok(sent)
}

/// 使用临时 consumer 获取消息（支持过滤）：与流式路径共用同一查询引擎，仅 sink 不同
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
    let query_start = std::time::Instant::now();
    let is_desc = sort == Some("desc") || (sort.is_none() && fetch_mode != Some("oldest"));

    // 单分区查询直接指定；全分区留空由查询 consumer 自己的 metadata 解析
    // （原实现为此单独建一个 consumer：多一次 TCP 连接 + metadata RTT）
    let partitions: Vec<i32> = partition.map_or_else(Vec::new, |p| vec![p]);

    let params = QueryParams {
        brokers: brokers.to_string(),
        topic: topic.to_string(),
        partitions,
        offset: if partition.is_some() { offset } else { None },
        max_messages,
        start_time,
        end_time,
        search,
        search_in,
        fetch_mode: fetch_mode.map(|s| s.to_string()),
        is_desc,
        truncate_value: None, // 导出/非流式保留完整内容
        on_partitions_resolved: None,
    };

    let messages = tokio::task::spawn_blocking(
        move || -> Result<Vec<crate::kafka::consumer::KafkaMessage>> {
            let mut out: Vec<crate::kafka::consumer::KafkaMessage> =
                Vec::with_capacity(max_messages.min(1024));
            run_message_query(params, CancellationToken::new(), |msg| {
                out.push(msg);
                Emit::Continue
            })?;
            Ok(out)
        },
    )
    .await
    .map_err(|e| AppError::Internal(format!("Query task join error: {}", e)))??;

    tracing::info!(
        "[Query] non-streaming: fetched {} messages in {:?}",
        messages.len(), query_start.elapsed()
    );
    Ok(messages)
}

/// 按 partition+offset 精确获取单条完整消息（用于查看列表中被截断的大消息）
async fn handle_message_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = get_string_param(&body, "cluster_id")?;
    let topic = get_string_param(&body, "topic")?;
    let partition = get_optional_i32_param(&body, "partition").unwrap_or(0);
    let offset = get_optional_i64_param(&body, "offset")
        .ok_or_else(|| AppError::BadRequest("Missing or invalid parameter: offset".to_string()))?;

    let config = ensure_cluster_client(&state, &cluster_id).await?;
    let brokers = config.brokers.clone();
    let topic_clone = topic.clone();

    // Schema Registry 配置：与列表路径一致解码 Avro/Protobuf（否则大消息查看显示 base64 原文）
    let schema_info = {
        let pool = state.get_pool();
        let cfg = SchemaRegistryStore::get_config(&pool, &cluster_id).await.ok().flatten();
        if cfg.is_some() {
            SchemaStore::get_latest_schema(&pool, &cluster_id, &topic).await.ok().flatten()
                .map(|s| (s.schema_type, s.schema_json))
        } else {
            None
        }
    };

    tokio::task::spawn_blocking(move || -> Result<Value> {
        use rdkafka::consumer::{BaseConsumer, Consumer, DefaultConsumerContext};
        use rdkafka::{Message, TopicPartitionList};
        use std::time::{Duration, Instant};

        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let group_id = format!("kafka-mgr-get-{}-{}", std::process::id(), unique_suffix);
        let cfg = build_query_consumer_config(&brokers, &group_id, false);
        let consumer: BaseConsumer<DefaultConsumerContext> = cfg.create()?;
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(&topic_clone, partition, rdkafka::Offset::Offset(offset))?;
        consumer.assign(&tpl)?;

        // 必须超过 socket.timeout.ms=60s 内一次 Fetch 的最坏响应时间的一半以上，
        // 慢 broker 首次 Fetch（含连接+metadata+拉取）可能远超原 15s，会误报 not found
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            if Instant::now() >= deadline {
                return Err(AppError::NotFound(format!(
                    "message at {}[{}]@{} not found (timeout)", topic_clone, partition, offset
                )));
            }
            match consumer.poll(Duration::from_millis(500)) {
                Some(Ok(msg)) => {
                    if msg.offset() == offset {
                        let (key, _) = convert_payload(msg.key(), None);
                        let (mut value, _) = convert_payload(msg.payload(), None);
                        if let (Some((ref ty, ref js)), Some(v)) = (&schema_info, &value) {
                            if let Some(decoded) = try_decode_schema_value(ty, js, v) {
                                value = Some(decoded);
                            }
                        }
                        return Ok(serde_json::json!({
                            "partition": msg.partition(),
                            "offset": msg.offset(),
                            "key": key,
                            "value": value,
                            "timestamp": msg.timestamp().to_millis(),
                        }));
                    }
                    if msg.offset() > offset {
                        return Err(AppError::NotFound(format!(
                            "message at {}[{}]@{} not found", topic_clone, partition, offset
                        )));
                    }
                }
                Some(Err(rdkafka::error::KafkaError::PartitionEOF(_))) => {
                    return Err(AppError::NotFound(format!(
                        "message at {}[{}]@{} not found (end of partition)", topic_clone, partition, offset
                    )));
                }
                Some(Err(e)) => return Err(AppError::Kafka(e)),
                None => {}
            }
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("Join error: {}", e)))?
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

/// 用查询 consumer 自身的 metadata 解析分区列表（带重试）
/// 原实现为此单独建一个 consumer：多一次 TCP 连接 + metadata RTT，且配置不统一。
/// 慢集群下 5s 超时曾导致退化为只查 partition 0，静默丢失其他分区的数据
fn resolve_partitions(consumer: &rdkafka::consumer::BaseConsumer, topic: &str) -> Result<Vec<i32>> {
    use rdkafka::consumer::Consumer;
    use std::time::Duration;

    let mut last_err: Option<String> = None;
    for attempt in 1..=3 {
        match consumer.fetch_metadata(Some(topic), Duration::from_secs(10)) {
            Ok(metadata) => {
                let partitions: Vec<i32> = metadata.topics().first()
                    .map(|t| t.partitions().iter().map(|p| p.id()).collect())
                    .unwrap_or_default();
                if !partitions.is_empty() {
                    return Ok(partitions);
                }
                last_err = Some(format!("topic {} not found in metadata", topic));
                tracing::warn!("[resolve_partitions] attempt {}/3: topic {} not found in metadata", attempt, topic);
            }
            Err(e) => {
                tracing::warn!("[resolve_partitions] attempt {}/3 failed for topic {}: {}", attempt, topic, e);
                last_err = Some(e.to_string());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    Err(AppError::Internal(format!(
        "Failed to fetch metadata for topic {}: {}",
        topic,
        last_err.unwrap_or_else(|| "unknown error".to_string())
    )))
}

/// 批量获取所有分区的 (low, high) watermark：两次 RPC 覆盖全部分区
///
/// 原理：librdkafka 的 offsets_for_times 把 offset 字段原样写入 ListOffsets 请求的
/// Time 字段（按 leader 分组并行发送），协议上 -1(End)=log end offset=high watermark，
/// -2(Beginning)=log start offset=low watermark。响应 offset 均 >= 0，不与哨兵值冲突。
/// 原实现每分区一次串行 fetch_watermarks RPC（10s 超时×3 重试），
/// 慢链路（200ms RTT）30 分区 ≈ 6s 纯 setup；批量后 ≈ 2×RTT。
/// 批量未拿到的分区回退串行 fetch_watermarks_with_retry（通常为零或个别分区）
fn fetch_watermarks_batch(
    consumer: &rdkafka::consumer::BaseConsumer,
    topic: &str,
    partitions: &[i32],
) -> Result<HashMap<i32, (i64, i64)>> {
    use rdkafka::consumer::Consumer;
    use rdkafka::TopicPartitionList;
    use std::time::Duration;

    // 批量查询一类 watermark；返回成功拿到 (无分区级错误 且 offset >= 0) 的分区
    let query_batch = |offset: rdkafka::Offset, label: &str| -> HashMap<i32, i64> {
        let mut tpl = TopicPartitionList::new();
        for &p in partitions {
            if let Err(e) = tpl.add_partition_offset(topic, p, offset) {
                tracing::warn!("[watermarks] add {}[{}] to batch failed: {}", topic, p, e);
            }
        }
        let mut map = HashMap::with_capacity(partitions.len());
        // 30s 超时 + 重试：慢 broker 需容忍；只读幂等，整批重试即可
        for attempt in 1..=2 {
            match consumer.offsets_for_times(tpl.clone(), Duration::from_secs(30)) {
                Ok(r) => {
                    for elem in r.elements_for_topic(topic) {
                        if let (Ok(()), Some(raw)) = (elem.error(), elem.offset().to_raw()) {
                            if raw >= 0 {
                                map.insert(elem.partition(), raw);
                            }
                        }
                    }
                    if map.len() == partitions.len() {
                        break;
                    }
                    tracing::warn!(
                        "[watermarks] batch {} attempt {}/2: got {}/{} partitions",
                        label, attempt, map.len(), partitions.len()
                    );
                }
                Err(e) => {
                    tracing::warn!("[watermarks] batch {} attempt {}/2 failed: {}", label, attempt, e);
                }
            }
        }
        map
    };

    let mut highs = query_batch(rdkafka::Offset::End, "high");
    let mut lows = query_batch(rdkafka::Offset::Beginning, "low");

    // 批量缺失的分区回退串行查询（与原实现同严格度：单个分区彻底失败则整个查询报错，
    // 宁可报错也不静默跳过分区返回不完整数据）
    let mut result = HashMap::with_capacity(partitions.len());
    for &p in partitions {
        let (low, high) = match (lows.remove(&p), highs.remove(&p)) {
            (Some(low), Some(high)) => (low, high),
            _ => fetch_watermarks_with_retry(consumer, topic, p).map_err(|e| {
                AppError::Internal(format!("fetch_watermarks failed for {}[{}]: {}", topic, p, e))
            })?,
        };
        result.insert(p, (low, high));
    }
    Ok(result)
}

/// 获取分区 watermark（带重试）
/// 慢集群下单次 5s 超时会被误判为 (0,0) 空分区，导致整个分区被跳过
fn fetch_watermarks_with_retry(
    consumer: &rdkafka::consumer::BaseConsumer,
    topic: &str,
    partition: i32,
) -> std::result::Result<(i64, i64), rdkafka::error::KafkaError> {
    use rdkafka::consumer::Consumer;
    use std::time::Duration;

    let mut last_err = None;
    for attempt in 1..=3 {
        match consumer.fetch_watermarks(topic, partition, Duration::from_secs(10)) {
            Ok(wm) => return Ok(wm),
            Err(e) => {
                tracing::warn!("[fetch_watermarks] attempt {}/3 failed for {}[{}]: {}", attempt, topic, partition, e);
                last_err = Some(e);
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(last_err.expect("retry loop ran at least once"))
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

#[cfg(test)]
mod query_engine_tests {
    use super::*;

    // ---------- 大小写不敏感搜索 ----------

    #[test]
    fn test_bytes_contain_ci_ascii() {
        let term = prepare_search_term("Hello");
        assert!(bytes_contain_ci(b"xxHeLLoWorldxx", &term));
        assert!(bytes_contain_ci(b"hello", &term));
        assert!(!bytes_contain_ci(b"helo", &term));
        assert!(!bytes_contain_ci(b"", &term));
        assert!(!bytes_contain_ci(b"hi", &term)); // haystack 短于 needle
    }

    #[test]
    fn test_bytes_contain_ci_unicode() {
        let term = prepare_search_term("错误");
        assert!(bytes_contain_ci("发生错误了".as_bytes(), &term));
        assert!(!bytes_contain_ci("一切正常".as_bytes(), &term));
    }

    #[test]
    fn test_message_matches_search_scope() {
        let term = prepare_search_term("foo");
        // all：key 或 value 命中即可
        assert!(message_matches_search(Some(b"foo-key"), Some(b"bar"), &term, None));
        assert!(message_matches_search(Some(b"bar"), Some(b"FOO"), &term, Some("all")));
        // key / value 限定范围
        assert!(message_matches_search(Some(b"foo"), None, &term, Some("key")));
        assert!(!message_matches_search(None, Some(b"foo"), &term, Some("key")));
        assert!(message_matches_search(None, Some(b"foo"), &term, Some("value")));
        assert!(!message_matches_search(Some(b"foo"), None, &term, Some("value")));
        // 空数据不命中
        assert!(!message_matches_search(None, None, &term, None));
    }

    // ---------- payload 截断 ----------

    #[test]
    fn test_convert_payload_no_truncation() {
        let (v, t) = convert_payload(Some(b"hello"), Some(100));
        assert_eq!(v.as_deref(), Some("hello"));
        assert!(!t);
        let (v, t) = convert_payload(None, Some(100));
        assert_eq!(v, None);
        assert!(!t);
        // 非法 UTF-8 → None
        let (v, _) = convert_payload(Some(&[0xff, 0xfe]), None);
        assert_eq!(v, None);
    }

    #[test]
    fn test_convert_payload_truncation_utf8_boundary() {
        // "你好" 6 字节，limit=4 应截断到字符边界 3（"你"）
        let (v, t) = convert_payload(Some("你好".as_bytes()), Some(4));
        assert_eq!(v.as_deref(), Some("你"));
        assert!(t);
        // 不截断的边界：len == limit
        let (v, t) = convert_payload(Some("abc".as_bytes()), Some(3));
        assert_eq!(v.as_deref(), Some("abc"));
        assert!(!t);
        // None limit 永不截断
        let long = vec![b'a'; 200 * 1024];
        let (v, t) = convert_payload(Some(&long), None);
        assert_eq!(v.as_ref().map(|s| s.len()), Some(200 * 1024));
        assert!(!t);
    }

    // ---------- 归并堆排序 ----------

    fn heap_entry(desc: bool, ts: Option<i64>, offset: i64) -> HeapEntry {
        HeapEntry {
            desc,
            timestamp: ts,
            offset,
            part: 0,
            msg: crate::kafka::consumer::KafkaMessage::default(),
        }
    }

    #[test]
    fn test_heap_asc_order() {
        let mut heap = BinaryHeap::new();
        heap.push(heap_entry(false, Some(3), 0));
        heap.push(heap_entry(false, Some(1), 5));
        heap.push(heap_entry(false, Some(1), 2)); // 同时间戳按 offset 升序
        heap.push(heap_entry(false, None, 0));    // None 排最后
        assert_eq!(heap.pop().unwrap().offset, 2);
        assert_eq!(heap.pop().unwrap().offset, 5);
        assert_eq!(heap.pop().unwrap().timestamp, Some(3));
        assert_eq!(heap.pop().unwrap().timestamp, None);
    }

    #[test]
    fn test_heap_desc_order() {
        // desc 输出 = asc 输出的精确反转（与旧版"升序推送 + 前端 reverse"行为一致），
        // 因此 None 时间戳在降序时排最前
        let mut heap = BinaryHeap::new();
        heap.push(heap_entry(true, Some(3), 0));
        heap.push(heap_entry(true, Some(1), 5));
        heap.push(heap_entry(true, Some(1), 2)); // 同时间戳按 offset 降序
        heap.push(heap_entry(true, None, 9));    // None 反转后排最前
        assert_eq!(heap.pop().unwrap().timestamp, None);
        assert_eq!(heap.pop().unwrap().timestamp, Some(3));
        assert_eq!(heap.pop().unwrap().offset, 5);
        assert_eq!(heap.pop().unwrap().offset, 2);
    }

    #[test]
    fn test_heap_desc_is_exact_reverse_of_asc() {
        // 降序输出必须等于升序输出的反转（保证与旧 reverse 行为一致）
        let inputs = [(Some(5), 1), (Some(1), 9), (None, 3), (Some(1), 2), (Some(3), 0)];
        let asc: Vec<(Option<i64>, i64)> = {
            let mut h = BinaryHeap::new();
            for &(ts, o) in &inputs {
                h.push(heap_entry(false, ts, o));
            }
            let mut v = Vec::new();
            while let Some(e) = h.pop() {
                v.push((e.timestamp, e.offset));
            }
            v
        };
        let desc: Vec<(Option<i64>, i64)> = {
            let mut h = BinaryHeap::new();
            for &(ts, o) in &inputs {
                h.push(heap_entry(true, ts, o));
            }
            let mut v = Vec::new();
            while let Some(e) = h.pop() {
                v.push((e.timestamp, e.offset));
            }
            v
        };
        let mut asc_rev = asc.clone();
        asc_rev.reverse();
        assert_eq!(asc_rev, desc);
    }
}
