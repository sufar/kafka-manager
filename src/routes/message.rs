use crate::db::cluster::ClusterStore;
use crate::error::{AppError, Result};
use crate::kafka::consumer::KafkaMessage;
use crate::kafka::message_query::{MessageQueryExecutor, MessageQueryParams, FetchMode, SearchFilter, SearchScope, TimeRange, OrderBy, SortOrder};
use crate::models::{MessageListResponse, MessageRecord, SendMessageRequest, SendMessageResponse};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

/// 获取或创建 admin 客户端（用于获取 topic 元数据）
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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:topic/messages", get(get_messages).post(send_message))
        .route("/:topic/messages/_export", get(export_messages))
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetMessageParams {
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    pub max_messages: Option<usize>,
    pub per_partition_max: Option<bool>,
    pub order_by: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<usize>,
    pub search: Option<String>,
    pub search_in: Option<String>,
    pub format: Option<String>,
    pub decode: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub fetch_mode: Option<String>,
    pub scan_depth: Option<usize>,
}

/// 增强的消息查询接口，返回统计信息
#[derive(Debug, Deserialize)]
pub struct GetMessageParamsEnhanced {
    #[serde(flatten)]
    pub params: GetMessageParams,
    #[serde(default)]
    pub include_stats: bool,
}

async fn get_messages(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Query(params): Query<GetMessageParams>,
) -> Result<Json<MessageListResponse>> {
    let start_time = Instant::now();

    // 确保集群连接存在
    let _admin = get_or_create_admin_client(&state, &cluster_id).await?;

    // 从 Pool 获取 Consumer Pool
    let consumer_pool = state.pools.get_consumer_pool(&cluster_id).await
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    // 构建查询参数
    let query_params = build_query_params(&params, &topic);

    // 执行查询
    let executor = MessageQueryExecutor::new(consumer_pool);
    let (messages, _stats) = executor.execute(&query_params).await?;

    // 转换为 MessageRecord
    let records: Vec<MessageRecord> = messages
        .into_iter()
        .map(|msg| MessageRecord {
            partition: msg.partition,
            offset: msg.offset,
            key: msg.key,
            value: msg.value,
            timestamp: msg.timestamp,
        })
        .collect();

    tracing::info!(
        "[get_messages] cluster_id={}, topic={}, returned {} messages, took {}ms",
        cluster_id,
        topic,
        records.len(),
        start_time.elapsed().as_millis()
    );

    Ok(Json(MessageListResponse { messages: records }))
}

/// 构建查询参数
fn build_query_params(params: &GetMessageParams, topic: &str) -> MessageQueryParams {
    let max_messages = params.max_messages.unwrap_or(100);
    let limit = params.limit.unwrap_or(max_messages);

    // 确定 fetch_mode
    let fetch_mode = FetchMode::from_str(
        params.fetch_mode.as_deref(),
        params.offset,
        params.start_time,
    );

    // 构建时间范围
    let time_range = match (params.start_time, params.end_time) {
        (Some(start), Some(end)) => Some(TimeRange { start, end }),
        (Some(start), None) => Some(TimeRange { start, end: i64::MAX }),
        (None, Some(end)) => Some(TimeRange { start: 0, end }),
        (None, None) => None,
    };

    // 构建搜索过滤器
    let search = params.search.as_ref().map(|keyword| SearchFilter {
        keyword: keyword.clone(),
        scope: SearchScope::from_str(params.search_in.as_deref()),
    });

    // 确定排序方式
    let order_by = if params.order_by.as_deref() == Some("timestamp") {
        OrderBy::Timestamp
    } else {
        OrderBy::Offset
    };

    let sort_order = if params.sort.as_deref() == Some("asc") {
        SortOrder::Asc
    } else {
        SortOrder::Desc
    };

    // 计算扫描深度
    let scan_depth = if let Some(depth) = params.scan_depth {
        depth
    } else if search.is_some() {
        // 搜索模式：扩大扫描范围
        limit * 100
    } else {
        limit
    };

    MessageQueryParams {
        topic: topic.to_string(),
        partition: params.partition,
        fetch_mode,
        time_range,
        search,
        limit,
        per_partition_max: params.per_partition_max.unwrap_or(false),
        scan_depth: Some(scan_depth),
        order_by,
        sort_order,
    }
}

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

// ==================== 消息导出功能 ====================

use axum::response::Response;
use axum::http::HeaderMap;

#[derive(Debug, Deserialize)]
pub struct ExportMessageParams {
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    pub max_messages: Option<usize>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub format: Option<String>,
    pub filename: Option<String>,
    pub search: Option<String>,
    pub fetch_mode: Option<String>,
}

async fn export_messages(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Query(params): Query<ExportMessageParams>,
) -> Result<Response> {
    use chrono::{DateTime, Utc};

    let clients = state.get_clients();
    let consumer = clients
        .get_consumer(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let max_messages = params.max_messages.unwrap_or(1000);

    // 构建搜索过滤器
    let matcher = |msg: &KafkaMessage| -> bool {
        // 时间范围过滤
        if let Some(start_time) = params.start_time {
            if let Some(ts) = msg.timestamp {
                if ts < start_time {
                    return false;
                }
            }
        }
        if let Some(end_time) = params.end_time {
            if let Some(ts) = msg.timestamp {
                if ts > end_time {
                    return false;
                }
            }
        }
        // 搜索过滤
        if let Some(search) = &params.search {
            let search_lower = search.to_lowercase();
            let key_match = msg.key.as_ref().map(|k| k.to_lowercase().contains(&search_lower)).unwrap_or(false);
            let value_match = msg.value.as_ref().map(|v| v.to_lowercase().contains(&search_lower)).unwrap_or(false);
            if !key_match && !value_match {
                return false;
            }
        }
        true
    };

    // 根据 fetch_mode 确定 offset
    let target_offset = if params.offset.is_none() && params.fetch_mode.as_deref() == Some("newest") {
        // 查询 high watermark
        use rdkafka::config::ClientConfig;
        use rdkafka::consumer::Consumer;
        use std::time::Duration;

        let temp_config = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("group.id", &format!("kafka-manager-export-{}-{}-{}", std::process::id(), topic, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("SystemTime before UNIX epoch").as_millis()))
            .create::<rdkafka::consumer::StreamConsumer>()
            .ok();

        if let Some(temp_consumer) = temp_config {
            let partition_id = params.partition.unwrap_or(0);
            match temp_consumer.fetch_watermarks(&topic, partition_id, Some(Duration::from_millis(500))) {
                Ok((_, high)) if high > 0 => Some(high - 1),
                _ => None,
            }
        } else {
            None
        }
    } else {
        params.offset
    };

    let mut messages = consumer
        .fetch_messages_filtered(&config, &topic, params.partition, target_offset, max_messages, &matcher)
        .await?;

    // 按时间戳排序
    messages.sort_by(|a, b| {
        match (a.timestamp, b.timestamp) {
            (Some(ts_a), Some(ts_b)) => ts_a.cmp(&ts_b),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => a.offset.cmp(&b.offset),
        }
    });

    // 根据格式生成内容
    let export_format = match params.format.as_deref() {
        Some("csv") => ExportFormat::Csv,
        Some("text") => ExportFormat::Text,
        _ => ExportFormat::Json,
    };

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
