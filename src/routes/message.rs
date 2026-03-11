use crate::db::cluster::ClusterStore;
use crate::error::{AppError, Result};
use crate::kafka::consumer::{KafkaConsumer, KafkaMessage};
use crate::models::{MessageListResponse, MessageRecord, SendMessageRequest, SendMessageResponse};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use rdkafka::consumer::Consumer;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    pub per_partition_max: Option<bool>,  // 是否启用 per-partition 模式
    pub order_by: Option<String>,    // "timestamp" or "offset"
    pub sort: Option<String>,        // "asc" or "desc"
    pub limit: Option<usize>,        // 限制返回的消息数量
    pub search: Option<String>,      // 搜索关键词
    pub search_in: Option<String>,   // 搜索范围："key", "value", "all" (默认)
    pub format: Option<String>,      // 输出格式："raw", "json", "hex"
    pub decode: Option<String>,      // 解码方式："utf8", "base64", "hex"
    pub start_time: Option<i64>,     // 开始时间戳（毫秒）
    pub end_time: Option<i64>,       // 结束时间戳（毫秒）
    pub fetch_mode: Option<String>,  // "oldest" or "newest"
}

impl GetMessageParams {
    /// 检查消息是否匹配过滤条件（流式过滤）
    fn matches(&self, msg: &KafkaMessage) -> bool {
        // 时间范围过滤
        if let Some(start) = self.start_time {
            if let Some(ts) = msg.timestamp {
                if ts < start {
                    return false;
                }
            }
        }
        if let Some(end) = self.end_time {
            if let Some(ts) = msg.timestamp {
                if ts > end {
                    return false;
                }
            }
        }

        // 搜索过滤
        if let Some(search_term) = &self.search {
            let search_in = self.search_in.as_deref().unwrap_or("all");
            let search_lower = search_term.to_lowercase();

            let matches = match search_in {
                "key" => msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower)),
                "value" => msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower)),
                _ => {
                    let key_match = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower));
                    let value_match = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower));
                    key_match || value_match
                }
            };
            if !matches {
                return false;
            }
        }

        true
    }
}

async fn get_messages(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Query(params): Query<GetMessageParams>,
) -> Result<Json<MessageListResponse>> {
    use std::collections::BinaryHeap;
    use std::cmp::Reverse;

    // 从 Pool 获取 Consumer Pool
    let consumer_pool = state.pools.get_consumer_pool(&cluster_id).await
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let max_messages = params.max_messages.unwrap_or(100);
    let limit = params.limit.unwrap_or(max_messages);
    let per_partition = params.per_partition_max.unwrap_or(false);

    // 克隆 params 用于闭包
    let params_clone = params.clone();

    // 判断是否需要排序
    let need_sort = params.order_by.as_deref() == Some("timestamp");
    let desc = params.sort.as_deref() == Some("desc");

    // Per-partition 模式：并行获取每个 partition 的消息
    let raw_messages = if per_partition && params.partition.is_none() {
        // 获取 topic 的 partition 列表
        let admin = get_or_create_admin_client(&state, &cluster_id).await?;
        let topic_info = admin.get_topic_info(&topic)?;
        let partition_ids: Vec<i32> = topic_info.partitions.iter().map(|p| p.id).collect();

        // 并发获取每个 partition 的消息（使用 Pool）
        // 优化：限制并发度，避免超过 Pool 容量
        use tokio::sync::Semaphore;
        let semaphore = std::sync::Arc::new(Semaphore::new(50)); // 最大 50 并发，使用 Pool 默认 max_size
        let mut tasks = Vec::new();
        for pid in partition_ids {
            let pool_clone = consumer_pool.clone();
            let config_clone = config.clone();
            let topic_clone = topic.clone();
            let params_clone = params_clone.clone();
            let max_msgs = max_messages;
            let cluster_id_clone = cluster_id.clone();
            let state_clone = state.clone();
            let sem_clone = semaphore.clone();

            let task = tokio::spawn(async move {
                // 获取信号量许可，限制并发度
                let _permit = sem_clone.acquire().await
                    .map_err(|e| AppError::Internal(format!("Semaphore error: {}", e)))?;

                // 计算每个 partition 的起始 offset
                let start_offset: Option<i64> = if let Some(start_time) = params_clone.start_time {
                    // 使用临时 Consumer 查询时间戳 offset（这个场景较少，影响不大）
                    let clients = state_clone.get_clients();
                    let consumer = clients.get_consumer(&cluster_id_clone)
                        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id_clone)))?;
                    let config = clients.get_config(&cluster_id_clone)
                        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id_clone)))?;
                    match consumer.get_offset_for_time(&config, &topic_clone, pid, start_time).await {
                        Ok(Some(offset)) => Some(offset),
                        Ok(None) | Err(_) => None,
                    }
                } else if params_clone.fetch_mode.as_deref() == Some("newest") && params_clone.offset.is_none() {
                    None
                } else if params_clone.fetch_mode.as_deref() == Some("oldest") && params_clone.offset.is_none() {
                    let clients = state_clone.get_clients();
                    let consumer = clients.get_consumer(&cluster_id_clone)
                        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id_clone)))?;
                    let config = clients.get_config(&cluster_id_clone)
                        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id_clone)))?;
                    consumer.get_offset_for_time(&config, &topic_clone, pid, 0).await.ok().flatten()
                } else {
                    params_clone.offset
                };

                // 使用 Pool 获取消息
                KafkaConsumer::fetch_messages_filtered_from_pool(
                    &pool_clone,
                    &config_clone,
                    &topic_clone,
                    Some(pid),
                    start_offset,
                    max_msgs,
                    &move |msg: &KafkaMessage| -> bool {
                        params_clone.matches(msg)
                    },
                )
                .await
            });
            tasks.push(task);
        }

        // 收集所有结果
        let mut all_messages = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(msgs)) => all_messages.extend(msgs),
                Ok(Err(e)) => tracing::warn!("Partition fetch error: {}", e),
                Err(e) => tracing::warn!("Task join error: {}", e),
            }
        }
        all_messages
    } else if params.fetch_mode.as_deref() == Some("newest")
        && params.offset.is_none()
        && params.start_time.is_none()
        && params.partition.is_none()
    {
        // 快速模式：查询所有 partition 的最新消息（使用 Pool）
        let admin = get_or_create_admin_client(&state, &cluster_id).await.ok();
        if let Some(admin_ref) = admin {
            match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
                Ok(Some(_cluster)) => {
                    match admin_ref.get_topic_info(&topic) {
                        Ok(topic_info) => {
                            let partition_count = topic_info.partitions.len();
                            // 优化：限制并发度，避免超过 Pool 容量
                            // 使用 Semaphore 控制同时运行的任务数
                            use tokio::sync::Semaphore;
                            let semaphore = std::sync::Arc::new(Semaphore::new(50)); // 最大 50 并发，使用 Pool 默认 max_size
                            let mut tasks = Vec::new();
                            for part in topic_info.partitions {
                                let pool_clone = consumer_pool.clone();
                                let topic_clone = topic.clone();
                                let params_clone = params_clone.clone();
                                let max_msgs = limit / partition_count.max(1);
                                let sem_clone = semaphore.clone();
                                let task = tokio::spawn(async move {
                                    // 获取信号量许可，限制并发度
                                    let _permit = sem_clone.acquire().await
                                        .map_err(|e| AppError::Internal(format!("Semaphore error: {}", e)))?;
                                    // 使用 Pool 获取 Consumer
                                    KafkaConsumer::fetch_latest_messages_from_pool(
                                        &pool_clone,
                                        &topic_clone,
                                        part.id,
                                        max_msgs.max(10),
                                        &move |msg: &KafkaMessage| -> bool {
                                            params_clone.matches(msg)
                                        },
                                    )
                                    .await
                                });
                                tasks.push(task);
                            }
                            let mut all_messages = Vec::new();
                            for task in tasks {
                                if let Ok(Ok(msgs)) = task.await {
                                    all_messages.extend(msgs);
                                }
                            }
                            all_messages
                        }
                        Err(_) => {
                            KafkaConsumer::fetch_latest_messages_from_pool(
                                &consumer_pool,
                                &topic,
                                0,
                                limit,
                                &move |msg: &KafkaMessage| -> bool {
                                    params_clone.matches(msg)
                                },
                            )
                            .await?
                        }
                    }
                }
                _ => {
                    KafkaConsumer::fetch_latest_messages_from_pool(
                        &consumer_pool,
                        &topic,
                        0,
                        limit,
                        &move |msg: &KafkaMessage| -> bool {
                            params_clone.matches(msg)
                        },
                    )
                    .await?
                }
            }
        } else {
            KafkaConsumer::fetch_latest_messages_from_pool(
                &consumer_pool,
                &topic,
                0,
                limit,
                &move |msg: &KafkaMessage| -> bool {
                    params_clone.matches(msg)
                },
            )
            .await?
        }
    } else {
        // 其他模式：使用原有逻辑查询 watermarks
        let start_offset = if params.fetch_mode.as_deref() == Some("newest") && params.offset.is_none() && params.start_time.is_none() {
            let target_partition = params.partition.unwrap_or(0);
            let admin = get_or_create_admin_client(&state, &cluster_id).await.ok();
            if let Some(admin_ref) = admin {
                match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
                    Ok(Some(cluster)) => {
                        match admin_ref.get_partition_watermarks(&topic, target_partition, &cluster.brokers) {
                            Ok((low, high)) => {
                                let start = std::cmp::max(low, high - limit as i64);
                                Some(start)
                            }
                            Err(_) => Some(0),
                        }
                    }
                    _ => Some(0),
                }
            } else {
                Some(0)
            }
        } else if let Some(start_time) = params.start_time {
            let target_partition = params.partition.unwrap_or(0);
            // 使用临时 Consumer 查询时间戳 offset
            let clients = state.get_clients();
            let consumer = clients.get_consumer(&cluster_id)
                .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
            let config = clients.get_config(&cluster_id)
                .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
            match consumer.get_offset_for_time(&config, &topic, target_partition, start_time).await {
                Ok(Some(offset)) => Some(offset),
                Ok(None) | Err(_) => params.offset,
            }
        } else if params.fetch_mode.as_deref() == Some("oldest") && params.offset.is_none() {
            let target_partition = params.partition.unwrap_or(0);
            // 使用临时 Consumer 查询 oldest offset
            let clients = state.get_clients();
            let consumer = clients.get_consumer(&cluster_id)
                .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
            let config = clients.get_config(&cluster_id)
                .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
            consumer.get_offset_for_time(&config, &topic, target_partition, 0).await.ok().flatten()
        } else {
            params.offset
        };

        // 使用 Pool 获取消息
        KafkaConsumer::fetch_messages_filtered_from_pool(
            &consumer_pool,
            &config,
            &topic,
            params_clone.partition,
            start_offset,
            limit,
            &move |msg: &KafkaMessage| -> bool {
                params_clone.matches(msg)
            },
        )
        .await?
    };

    // 转换为 MessageRecord 并流式排序
    let messages = if need_sort {
        // 使用堆进行流式 TopK 排序
        if desc {
            // 降序：使用最小堆，维护最大的 limit 个元素
            let mut heap: BinaryHeap<Reverse<MessageRecord>> = BinaryHeap::new();
            for msg in raw_messages {
                let record = MessageRecord {
                    partition: msg.partition,
                    offset: msg.offset,
                    key: msg.key,
                    value: msg.value,
                    timestamp: msg.timestamp,
                };
                if heap.len() < limit {
                    heap.push(Reverse(record));
                } else if let Some(min) = heap.peek() {
                    if record.timestamp.unwrap_or(0) > min.0.timestamp.unwrap_or(0) {
                        heap.pop();
                        heap.push(Reverse(record));
                    }
                }
            }
            // 转换为 Vec 并按 timestamp 降序排列
            let mut result: Vec<MessageRecord> = heap.into_iter().map(|Reverse(r)| r).collect();
            result.sort_by(|a, b| {
                match (a.timestamp, b.timestamp) {
                    (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => b.offset.cmp(&a.offset),
                }
            });
            result
        } else {
            // 升序：使用最大堆，维护最小的 limit 个元素
            let mut heap: BinaryHeap<MessageRecord> = BinaryHeap::new();
            for msg in raw_messages {
                let record = MessageRecord {
                    partition: msg.partition,
                    offset: msg.offset,
                    key: msg.key,
                    value: msg.value,
                    timestamp: msg.timestamp,
                };
                if heap.len() < limit {
                    heap.push(record);
                } else if let Some(max) = heap.peek() {
                    if record.timestamp.unwrap_or(i64::MAX) < max.timestamp.unwrap_or(i64::MAX) {
                        heap.pop();
                        heap.push(record);
                    }
                }
            }
            // 转换为 Vec 并按 timestamp 升序排列
            let mut result: Vec<MessageRecord> = heap.into_iter().collect();
            result.sort_by(|a, b| {
                match (a.timestamp, b.timestamp) {
                    (Some(ts_a), Some(ts_b)) => ts_a.cmp(&ts_b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.offset.cmp(&b.offset),
                }
            });
            result
        }
    } else {
        // 不需要排序，直接取前 limit 条
        raw_messages
            .into_iter()
            .take(limit)
            .map(|msg| MessageRecord {
                partition: msg.partition,
                offset: msg.offset,
                key: msg.key,
                value: msg.value,
                timestamp: msg.timestamp,
            })
            .collect()
    };

    Ok(Json(MessageListResponse { messages }))
}

/// 增强的消息记录，包含格式化后的内容
#[derive(Debug, Serialize)]
pub struct EnhancedMessageRecord {
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: Option<String>,
    pub timestamp: Option<i64>,
    /// 原始 key (base64 编码)
    pub key_raw: Option<String>,
    /// 原始 value (base64 编码)
    pub value_raw: Option<String>,
    /// JSON 格式化后的 value
    pub value_json: Option<serde_json::Value>,
    /// Hex 格式的 value
    pub value_hex: Option<String>,
    /// 内容类型推断
    pub content_type: Option<String>,
    /// 消息大小
    pub size: MessageSize,
}

/// 消息大小信息
#[derive(Debug, Serialize)]
pub struct MessageSize {
    pub key_size: usize,
    pub value_size: usize,
    pub total_size: usize,
}

/// 消息格式选项
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageFormat {
    Raw,
    Json,
    Hex,
}

impl MessageFormat {
    pub fn from_str(s: Option<&str>) -> Self {
        match s {
            Some("json") => MessageFormat::Json,
            Some("hex") => MessageFormat::Hex,
            _ => MessageFormat::Raw,
        }
    }
}

/// 解码方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecodeMode {
    Utf8,
    Base64,
    Hex,
}

impl DecodeMode {
    pub fn from_str(s: Option<&str>) -> Self {
        match s {
            Some("base64") => DecodeMode::Base64,
            Some("hex") => DecodeMode::Hex,
            _ => DecodeMode::Utf8,
        }
    }
}

/// 推断内容类型
fn infer_content_type(data: &[u8]) -> Option<&'static str> {
    // 检查是否为 JSON
    if data.starts_with(b"{") || data.starts_with(b"[") {
        if serde_json::from_slice::<serde_json::Value>(data).is_ok() {
            return Some("application/json");
        }
    }
    // 检查是否为纯文本
    if std::str::from_utf8(data).is_ok() {
        // 检查是否包含常见的文本特征
        let text = String::from_utf8_lossy(data);
        if text.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || ",.!?;:'\"()[]{}".contains(c)) {
            return Some("text/plain");
        }
    }
    None
}

/// 将字节转换为 hex 字符串
fn bytes_to_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 增强的消息查看 - 返回更丰富的信息（流式过滤版本）
async fn get_messages_enhanced(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Query(params): Query<GetMessageParams>,
) -> Result<serde_json::Value> {
    let clients = state.get_clients();
    let consumer = clients
        .get_consumer(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let max_messages = params.max_messages.unwrap_or(100);
    let format = MessageFormat::from_str(params.format.as_deref());
    let decode_mode = DecodeMode::from_str(params.decode.as_deref());

    // 构建流式过滤器
    let params_clone = params.clone();
    let matcher = move |msg: &KafkaMessage| -> bool {
        // 时间范围过滤
        if let Some(start) = params_clone.start_time {
            if let Some(ts) = msg.timestamp {
                if ts < start {
                    return false;
                }
            }
        }
        if let Some(end) = params_clone.end_time {
            if let Some(ts) = msg.timestamp {
                if ts > end {
                    return false;
                }
            }
        }

        // 搜索过滤
        if let Some(search_term) = &params_clone.search {
            let search_in = params_clone.search_in.as_deref().unwrap_or("all");
            let search_lower = search_term.to_lowercase();

            let matches = match search_in {
                "key" => msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower)),
                "value" => msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower)),
                _ => {
                    let key_match = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(&search_lower));
                    let value_match = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(&search_lower));
                    key_match || value_match
                }
            };
            if !matches {
                return false;
            }
        }

        true
    };

    // 流式获取消息：在读取时就进行过滤
    let mut messages = consumer
        .fetch_messages_filtered(&config, &topic, params.partition, params.offset, max_messages, &matcher)
        .await?;

    // 按 timestamp 排序
    if let Some(order_by) = params.order_by {
        if order_by == "timestamp" {
            let desc = params.sort.as_deref() == Some("desc");
            messages.sort_by(|a, b| {
                match (a.timestamp, b.timestamp) {
                    (Some(ts_a), Some(ts_b)) => {
                        if desc {
                            ts_b.cmp(&ts_a)
                        } else {
                            ts_a.cmp(&ts_b)
                        }
                    }
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => a.offset.cmp(&b.offset),
                }
            });
        }
    }

    // 限制返回数量
    if let Some(limit) = params.limit {
        if limit < messages.len() {
            messages.truncate(limit);
        }
    }

    // 转换为增强的消息记录
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    let enhanced: Vec<EnhancedMessageRecord> = messages
        .into_iter()
        .map(|msg| {
            let key_bytes = msg.key.clone().map(|k| k.into_bytes()).unwrap_or_default();
            let value_bytes = msg.value.clone().map(|v| v.into_bytes()).unwrap_or_default();

            // JSON 格式化
            let value_json = msg.value.as_ref()
                .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok());

            // Hex 格式
            let value_hex = Some(bytes_to_hex(&value_bytes));

            // 内容类型
            let content_type = infer_content_type(&value_bytes);

            // Base64 编码的原始数据
            let key_raw = if !key_bytes.is_empty() {
                Some(BASE64.encode(&key_bytes))
            } else {
                None
            };
            let value_raw = if !value_bytes.is_empty() {
                Some(BASE64.encode(&value_bytes))
            } else {
                None
            };

            EnhancedMessageRecord {
                partition: msg.partition,
                offset: msg.offset,
                key: msg.key,
                value: msg.value,
                timestamp: msg.timestamp,
                key_raw,
                value_raw,
                value_json,
                value_hex,
                content_type: content_type.map(String::from),
                size: MessageSize {
                    key_size: key_bytes.len(),
                    value_size: value_bytes.len(),
                    total_size: key_bytes.len() + value_bytes.len(),
                },
            }
        })
        .collect();

    Ok(serde_json::json!({
        "messages": enhanced,
        "count": enhanced.len(),
        "format": match format {
            MessageFormat::Raw => "raw",
            MessageFormat::Json => "json",
            MessageFormat::Hex => "hex",
        },
        "decode_mode": match decode_mode {
            DecodeMode::Utf8 => "utf8",
            DecodeMode::Base64 => "base64",
            DecodeMode::Hex => "hex",
        },
    }))
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

#[derive(Debug, Deserialize)]
pub struct ExportMessageParams {
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    pub max_messages: Option<usize>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub format: Option<String>, // "json", "csv", "text"
    pub filename: Option<String>,
    pub search: Option<String>,      // 搜索关键词
    pub fetch_mode: Option<String>,  // "oldest" or "newest"
}

/// 导出消息到文件
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
    let export_format = ExportFormat::from_str(params.format.as_deref());

    // 根据 fetch_mode 确定 offset
    // 如果是 newest 且未指定 offset，需要查询 high watermark
    let target_offset = if params.offset.is_none() && params.fetch_mode.as_deref() == Some("newest") {
        // 使用 consumer 查询 high watermark
        use rdkafka::config::ClientConfig;
        use std::time::Duration;

        let temp_config = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("group.id", &format!("kafka-manager-watermark-{}-{}-{}", std::process::id(), topic, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("SystemTime before UNIX epoch").as_millis()))
            .create::<rdkafka::consumer::StreamConsumer>()
            .ok();

        if let Some(temp_consumer) = temp_config {
            let partition_id = params.partition.unwrap_or(0);
            match temp_consumer.fetch_watermarks(&topic, partition_id, Some(Duration::from_millis(500))) {
                Ok((_, high)) if high > 0 => Some(high - 1),  // 从最后一个消息开始
                _ => None,
            }
        } else {
            None
        }
    } else {
        params.offset
    };

    // 流式过滤：时间范围过滤和搜索过滤在读取时进行
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
