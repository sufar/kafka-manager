use crate::db::cluster::ClusterStore;
use crate::error::{AppError, Result};
use crate::kafka::message_query::{FetchMode, MessageQuerier, QueryParams, SearchIn, SortBy, SortOrder};
use crate::models::{MessageListResponse, SendMessageRequest, SendMessageResponse};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:topic/messages", get(get_messages).post(send_message))
        .route("/:topic/messages/_export", get(export_messages))
}

/// 获取或创建 admin 客户端
async fn get_or_create_admin_client(
    state: &AppState,
    cluster_id: &str,
) -> Result<Arc<crate::kafka::KafkaAdmin>> {
    use tokio::time::Duration;

    // 首先尝试从内存中获取
    let clients = state.get_clients();
    if let Some(admin) = clients.get_admin(cluster_id) {
        return Ok(admin);
    }

    // 从数据库获取集群配置（带重试）
    let mut cluster = None;
    let mut last_db_error = None;

    for attempt in 0..3 {
        match ClusterStore::get_by_name(state.db.inner(), cluster_id).await {
            Ok(Some(c)) => {
                cluster = Some(c);
                break;
            }
            Ok(None) => {
                last_db_error = Some(AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)));
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
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

    // 建立连接池连接
    state.pools.add_cluster(cluster_id, &config, &state.config.pool).await?;

    // 更新 Kafka 客户端
    let current_clients = state.get_clients();
    let new_clients = current_clients.with_added_cluster(cluster_id, &config)?;
    state.set_clients(new_clients.into());

    // 获取新的客户端
    let updated_clients = state.get_clients();
    updated_clients.get_admin(cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Failed to get admin client for cluster '{}'", cluster_id)))
}

/// 消息查询参数
#[derive(Debug, Deserialize)]
pub struct GetMessageParams {
    /// 指定分区（不指定则查询所有分区）
    pub partition: Option<i32>,
    /// 起始offset
    pub offset: Option<i64>,
    /// 最大返回消息数
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
    /// 每个分区最大消息数（多分区查询时有效）
    #[serde(default = "default_per_partition_max")]
    pub per_partition_max: usize,
    /// 排序字段（timestamp/offset）
    #[serde(default)]
    pub order_by: Option<String>,
    /// 排序方向（asc/desc）
    #[serde(default)]
    pub sort: Option<String>,
    /// 搜索关键词
    #[serde(default)]
    pub search: Option<String>,
    /// 搜索范围（key/value/all）
    #[serde(default)]
    pub search_in: Option<String>,
    /// 开始时间戳（毫秒）
    #[serde(default)]
    pub start_time: Option<i64>,
    /// 结束时间戳（毫秒）
    #[serde(default)]
    pub end_time: Option<i64>,
    /// 获取模式（newest/oldest）
    #[serde(default)]
    pub fetch_mode: Option<String>,
}

fn default_max_messages() -> usize { 100 }
fn default_per_partition_max() -> usize { 100 }

impl GetMessageParams {
    /// 转换为内部QueryParams
    fn to_query_params(&self) -> QueryParams {
        let fetch_mode = match self.fetch_mode.as_deref() {
            Some("oldest") => FetchMode::Oldest,
            _ => FetchMode::Newest,
        };

        let sort_by = match self.order_by.as_deref() {
            Some("offset") => SortBy::Offset,
            _ => SortBy::Timestamp,
        };

        let sort_order = match self.sort.as_deref() {
            Some("asc") => SortOrder::Asc,
            _ => SortOrder::Desc,
        };

        let search_in = SearchIn::from_str(self.search_in.as_deref().unwrap_or("all"));

        QueryParams {
            partition: self.partition,
            offset: self.offset,
            max_messages: self.max_messages,
            per_partition_max: self.per_partition_max,
            fetch_mode,
            start_time: self.start_time,
            end_time: self.end_time,
            search: self.search.clone(),
            search_in,
            sort_by,
            sort_order,
        }
    }
}

/// 获取消息列表
async fn get_messages(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Query(params): Query<GetMessageParams>,
) -> Result<Json<MessageListResponse>> {
    let start_time = std::time::Instant::now();

    // 获取消费者池
    let consumer_pool = state.pools.get_consumer_pool(&cluster_id).await
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // 获取集群配置
    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // 转换为内部查询参数
    let query_params = params.to_query_params();

    tracing::info!(
        "[get_messages] cluster={}, topic={}, partition={:?}, max_messages={}, fetch_mode={:?}",
        cluster_id, topic, query_params.partition, query_params.max_messages, query_params.fetch_mode
    );

    // 使用高性能查询器查询消息
    let messages = MessageQuerier::query(
        &consumer_pool,
        &config,
        &topic,
        query_params,
    ).await.map_err(|e| {
        tracing::error!("Message query failed: {}", e);
        AppError::Internal(format!("Failed to query messages: {}", e))
    })?;

    tracing::info!(
        "[get_messages] Query completed in {:?}, returned {} messages",
        start_time.elapsed(),
        messages.len()
    );

    Ok(Json(MessageListResponse { messages }))
}

/// 发送消息到topic
async fn send_message(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>> {
    let clients = state.get_clients();
    let producer = clients
        .get_producer(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let (partition, offset) = producer
        .send_to_partition(&topic, req.partition, req.key.as_deref(), &req.value)
        .await?;

    Ok(Json(SendMessageResponse {
        partition,
        offset,
    }))
}

/// 导出消息参数
#[derive(Debug, Deserialize)]
pub struct ExportMessageParams {
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
    #[serde(default)]
    pub start_time: Option<i64>,
    #[serde(default)]
    pub end_time: Option<i64>,
    #[serde(default)]
    pub format: Option<String>, // "json", "csv", "text"
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub fetch_mode: Option<String>,
}

/// 导出格式
#[derive(Debug, Clone, Copy)]
enum ExportFormat {
    Json,
    Csv,
    Text,
}

impl ExportFormat {
    fn from_str(s: Option<&str>) -> Self {
        match s {
            Some("csv") => ExportFormat::Csv,
            Some("text") => ExportFormat::Text,
            _ => ExportFormat::Json,
        }
    }

    fn content_type(&self) -> &'static str {
        match self {
            ExportFormat::Json => "application/json",
            ExportFormat::Csv => "text/csv",
            ExportFormat::Text => "text/plain",
        }
    }
}

use axum::response::Response;
use axum::http::HeaderMap;

/// 导出消息到文件
async fn export_messages(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Query(params): Query<ExportMessageParams>,
) -> Result<Response> {
    use chrono::{DateTime, Utc};

    // 获取消费者池
    let consumer_pool = state.pools.get_consumer_pool(&cluster_id).await
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // 获取集群配置
    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let export_format = ExportFormat::from_str(params.format.as_deref());

    // 构建查询参数
    let query_params = QueryParams {
        partition: params.partition,
        offset: params.offset,
        max_messages: params.max_messages,
        per_partition_max: params.max_messages,
        fetch_mode: match params.fetch_mode.as_deref() {
            Some("oldest") => FetchMode::Oldest,
            _ => FetchMode::Newest,
        },
        start_time: params.start_time,
        end_time: params.end_time,
        search: params.search.clone(),
        search_in: SearchIn::All,
        sort_by: SortBy::Timestamp,
        sort_order: SortOrder::Asc,
    };

    // 查询消息
    let messages = MessageQuerier::query(
        &consumer_pool,
        &config,
        &topic,
        query_params,
    ).await.map_err(|e| {
        AppError::Internal(format!("Failed to query messages: {}", e))
    })?;

    // 根据格式生成内容
    let content = match export_format {
        ExportFormat::Json => {
            serde_json::to_string_pretty(&messages).map_err(|e| {
                AppError::Internal(format!("Failed to serialize messages: {}", e))
            })?
        }
        ExportFormat::Csv => {
            let mut csv = String::from("partition,offset,timestamp,key,value\n");
            for msg in &messages {
                let timestamp_str = msg.timestamp
                    .map(|ts| DateTime::from_timestamp_millis(ts)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default())
                    .unwrap_or_default();
                let key = msg.key.as_deref().unwrap_or("").replace('"', "\"\"");
                let value = msg.value.as_deref().unwrap_or("").replace('"', "\"\"");
                csv.push_str(&format!(
                    "{},{},{},\"{}\",\"{}\"\n",
                    msg.partition, msg.offset, timestamp_str, key, value
                ));
            }
            csv
        }
        ExportFormat::Text => {
            let mut text = String::new();
            for msg in &messages {
                let timestamp_str = msg.timestamp
                    .map(|ts| DateTime::from_timestamp_millis(ts)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default())
                    .unwrap_or_default();
                text.push_str(&format!(
                    "[{}] Partition {}: Offset {} - Key: {:?}, Value: {:?}\n",
                    timestamp_str, msg.partition, msg.offset, msg.key, msg.value
                ));
            }
            text
        }
    };

    // 设置响应头
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        export_format.content_type().parse().expect("valid content type"),
    );

    let filename = params.filename.unwrap_or_else(|| {
        format!("{}_export_{}", topic, Utc::now().format("%Y%m%d_%H%M%S"))
    });

    let extension = match export_format {
        ExportFormat::Json => "json",
        ExportFormat::Csv => "csv",
        ExportFormat::Text => "txt",
    };

    headers.insert(
        "Content-Disposition",
        format!("attachment; filename=\"{}.{}\"", filename, extension).parse().expect("valid disposition"),
    );

    let mut response = Response::new(content.into());
    *response.headers_mut() = headers;

    Ok(response)
}
