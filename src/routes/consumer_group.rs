// Consumer Group 路由模块
// 注意：在 Axum 中，路由匹配优先级基于注册顺序
// 静态路径（如/_*）必须先注册，动态路径（如/:name）后注册

use crate::error::{AppError, Result};
use crate::kafka::offset::{KafkaOffsetManager, ConsumerGroupOffsetsInfo};
use crate::kafka::throughput::KafkaThroughputCalculator;
use crate::models::{
    ConsumerGroupDetailResponse, ConsumerGroupLagHistory, ConsumerGroupLagSummary, ConsumerGroupListResponse, ConsumerGroupMember,
    ConsumerGroupOffsetDetailResponse, ConsumerGroupPartitionDetail, ConsumerGroupThroughputResponse,
    LagHistoryQuery, PartitionLagDetail, ResetConsumerGroupOffsetRequest,
    TopicConsumerLagHistoryResponse, TopicConsumerLagResponse, ConsumerOffsetsListResponse,
    ConsumerGroupOffsetsSummary, TopicOffsetsSummary, PartitionOffsetDetail,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;

// 操作型路由（/_*）- 先注册
pub fn consumer_group_operation_routes() -> Router<AppState> {
    Router::new()
        .route("/_batch", delete(batch_delete_consumer_groups))
}

// Consumer Group 资源路由 - 后注册
pub fn cluster_routes() -> Router<AppState> {
    Router::new()
        // 通用路由
        .route("/", get(list_consumer_groups))
        .route("/:name", get(get_consumer_group).delete(delete_consumer_group))
        .route("/:name/_offsets", get(get_consumer_group_offsets))
        .route("/:name/_offsets/reset", post(reset_consumer_group_offset))
        .route("/:name/_throughput", get(get_consumer_group_throughput))
        // Consumer Offsets 列表路由（获取所有 Consumer Group 的 offsets）
        .route("/_consumer-offsets", get(get_all_consumer_offsets))
        // Topic consumer lag 路由
        .route("/topics/:topic/_consumer-lag", get(get_topic_consumer_lag))
        .route("/topics/:topic/_consumer-lag-history", get(get_topic_consumer_lag_history))
}

#[derive(Debug, Deserialize)]
pub struct ConsumerGroupOffsetsQuery {
    pub topic: Option<String>,
}

async fn list_consumer_groups(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ConsumerGroupListResponse>> {
    // 先尝试从缓存获取
    if let Some(cached_groups) = state.cache.get_consumer_group_list(&cluster_id).await {
        let groups = cached_groups
            .into_iter()
            .map(|name| crate::models::ConsumerGroupSummary {
                name,
                state: "Unknown".to_string(),
            })
            .collect();
        return Ok(Json(ConsumerGroupListResponse { groups }));
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

    // 写入缓存
    state.cache.set_consumer_group_list(&cluster_id, group_names.clone()).await;

    Ok(Json(ConsumerGroupListResponse {
        groups: group_names
            .into_iter()
            .zip(groups.into_iter().map(|g| g.state))
            .map(|(name, state)| crate::models::ConsumerGroupSummary { name, state })
            .collect(),
    }))
}

/// 获取所有 Consumer Group 的 offsets 信息
async fn get_all_consumer_offsets(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ConsumerOffsetsListResponse>> {
    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let offset_manager = KafkaOffsetManager::new(&config);
    let all_offsets = offset_manager.get_all_consumer_offsets(&config)?;

    // 转换为响应格式
    let consumer_groups: Vec<ConsumerGroupOffsetsSummary> = all_offsets
        .into_iter()
        .map(|group: ConsumerGroupOffsetsInfo| {
            let topics: Vec<TopicOffsetsSummary> = group.topics
                .into_iter()
                .map(|topic| {
                    let partitions: Vec<PartitionOffsetDetail> = topic.partitions
                        .into_iter()
                        .map(|p| PartitionOffsetDetail {
                            partition: p.partition,
                            start_offset: p.start_offset,
                            end_offset: p.end_offset,
                            current_offset: p.current_offset,
                            lag: p.lag,
                            state: if p.current_offset >= 0 { "Active".to_string() } else { "Empty".to_string() },
                        })
                        .collect();

                    TopicOffsetsSummary {
                        topic: topic.topic,
                        partitions,
                        total_lag: topic.total_lag,
                    }
                })
                .collect();

            ConsumerGroupOffsetsSummary {
                group_name: group.group_name,
                state: group.state,
                topics,
                total_lag: group.total_lag,
            }
        })
        .collect();

    Ok(Json(ConsumerOffsetsListResponse {
        cluster_id,
        consumer_groups,
    }))
}

async fn get_consumer_group(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
) -> Result<Json<ConsumerGroupDetailResponse>> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let group_info = admin.get_consumer_group_info(&config,  &name)?;

    Ok(Json(ConsumerGroupDetailResponse {
        name: group_info.name,
        state: group_info.state,
        protocol: Some(group_info.protocol),
        members: group_info
            .members
            .into_iter()
            .map(|m| ConsumerGroupMember {
                client_id: m.client_id,
                host: m.host,
            })
            .collect(),
        // offsets 需要额外的 API 获取，这里返回空列表
        offsets: vec![],
    }))
}

async fn delete_consumer_group(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
) -> Result<()> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    admin.delete_consumer_group(&name).await?;
    Ok(())
}

async fn get_consumer_group_offsets(
    State(state): State<AppState>,
    Path((cluster_id, group_name)): Path<(String, String)>,
    Query(query): Query<ConsumerGroupOffsetsQuery>,
) -> Result<Json<ConsumerGroupOffsetDetailResponse>> {
    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let offset_manager = KafkaOffsetManager::new(&config);
    let offsets = offset_manager.get_consumer_group_offsets(
        &config,
        &group_name,
        query.topic.as_deref(),
    )?;

    // 获取 topic 名称（从第一个 offset 或者使用查询参数）
    let topic = offsets.first().map(|o| o.topic.clone()).unwrap_or_default();
    let total_lag = offsets.iter().map(|o| o.lag).sum();

    let partitions = offsets
        .into_iter()
        .map(|o| ConsumerGroupPartitionDetail {
            partition: o.partition,
            current_offset: o.current_offset,
            log_end_offset: o.log_end_offset,
            lag: o.lag,
            state: if o.current_offset >= 0 {
                "Active".to_string()
            } else {
                "Empty".to_string()
            },
            last_commit_time: None,  // 暂不支持
            topic: o.topic,
        })
        .collect();

    Ok(Json(ConsumerGroupOffsetDetailResponse {
        group_name,
        topic,
        partitions,
        total_lag,
    }))
}

async fn reset_consumer_group_offset(
    State(state): State<AppState>,
    Path((cluster_id, group_name)): Path<(String, String)>,
    Json(req): Json<ResetConsumerGroupOffsetRequest>,
) -> Result<()> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let offset_type = match req.offset {
        crate::models::ResetOffsetType::Earliest => crate::kafka::admin::OffsetType::Earliest,
        crate::models::ResetOffsetType::Latest => crate::kafka::admin::OffsetType::Latest,
        crate::models::ResetOffsetType::Value(val) => crate::kafka::admin::OffsetType::Value(val),
        crate::models::ResetOffsetType::Timestamp(ts) => crate::kafka::admin::OffsetType::Timestamp(ts),
    };

    if let Some(partition) = req.partition {
        admin.reset_consumer_group_offset(&config, &group_name, &req.topic, partition, offset_type).await?;
    } else {
        admin.reset_consumer_group_offsets(&config, &group_name, &req.topic, offset_type).await?;
    }

    Ok(())
}

// ==================== 批量操作 ====================

use serde::Serialize;

/// 批量删除 Consumer Group 请求
#[derive(Debug, Deserialize)]
pub struct BatchDeleteConsumerGroupsRequest {
    pub group_names: Vec<String>,
    /// 是否继续执行即使有失败
    #[serde(default)]
    pub continue_on_error: bool,
}

/// 批量删除 Consumer Group 响应
#[derive(Debug, Serialize)]
pub struct BatchDeleteConsumerGroupsResponse {
    pub success: bool,
    pub deleted: Vec<String>,
    pub failed: Vec<FailedConsumerGroup>,
}

/// 失败的 Consumer Group 操作
#[derive(Debug, Serialize)]
pub struct FailedConsumerGroup {
    pub name: String,
    pub error: String,
}

/// 批量删除 Consumer Group
async fn batch_delete_consumer_groups(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Json(req): Json<BatchDeleteConsumerGroupsRequest>,
) -> Result<Json<BatchDeleteConsumerGroupsResponse>> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let mut deleted = Vec::new();
    let mut failed = Vec::new();

    for group_name in req.group_names {
        match admin.delete_consumer_group(&group_name).await {
            Ok(_) => deleted.push(group_name),
            Err(e) => {
                failed.push(FailedConsumerGroup {
                    name: group_name,
                    error: e.to_string(),
                });
                if !req.continue_on_error {
                    return Ok(Json(BatchDeleteConsumerGroupsResponse {
                        success: false,
                        deleted,
                        failed,
                    }));
                }
            }
        }
    }

    Ok(Json(BatchDeleteConsumerGroupsResponse {
        success: failed.is_empty(),
        deleted,
        failed,
    }))
}

// ==================== Consumer Group 吞吐量 ====================

#[derive(Debug, serde::Deserialize)]
pub struct ThroughputQuery {
    pub topic: Option<String>,
}

/// 获取 Consumer Group 吞吐量（消费速度）
async fn get_consumer_group_throughput(
    State(state): State<AppState>,
    Path((cluster_id, group_name)): Path<(String, String)>,
    Query(query): Query<ThroughputQuery>,
) -> Result<Json<ConsumerGroupThroughputResponse>> {
    let clients = state.get_clients();
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let topic = query.topic.ok_or_else(|| {
        AppError::BadRequest("Topic parameter is required".to_string())
    })?;

    let calculator = KafkaThroughputCalculator::new(&config);
    let throughput = calculator.calculate_consumer_group_throughput(&config, &group_name, &topic)?;

    Ok(Json(throughput))
}

/// 获取某个 Topic 所有 Consumer Group 的积压情况
async fn get_topic_consumer_lag(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
) -> Result<Json<TopicConsumerLagResponse>> {
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

        let partitions: Vec<PartitionLagDetail> = offsets
            .into_iter()
            .map(|o| PartitionLagDetail {
                partition: o.partition,
                current_offset: o.current_offset,
                log_end_offset: o.log_end_offset,
                lag: o.lag,
                state: if o.current_offset >= 0 {
                    "Active".to_string()
                } else {
                    "Empty".to_string()
                },
            })
            .collect();

        consumer_group_lags.push(ConsumerGroupLagSummary {
            group_name: group.name.clone(),
            total_lag: group_lag,
            partitions,
        });
    }

    // 按 lag 降序排序
    consumer_group_lags.sort_by(|a, b| b.total_lag.cmp(&a.total_lag));

    Ok(Json(TopicConsumerLagResponse {
        topic,
        total_lag,
        consumer_groups: consumer_group_lags,
    }))
}

/// 获取某个 Topic 所有 Consumer Group 的历史积压数据（用于折线图）
///
/// 注意：由于 Kafka 不存储历史 offset 数据，此接口返回当前时刻的快照数据
/// 前端可以定期调用此接口（如每秒一次）采集数据点，构建实时折线图
async fn get_topic_consumer_lag_history(
    State(state): State<AppState>,
    Path((cluster_id, topic)): Path<(String, String)>,
    Query(query): Query<LagHistoryQuery>,
) -> Result<Json<TopicConsumerLagHistoryResponse>> {
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

    // 构建单点历史数据响应
    let consumer_groups: Vec<ConsumerGroupLagHistory> = snapshot.consumer_groups
        .into_iter()
        .map(|g| {
            let total_produced: i64 = g.partitions.iter()
                .map(|p| p.log_end_offset)
                .sum();
            let total_consumed: i64 = g.partitions.iter()
                .map(|p| p.current_offset)
                .sum();

            ConsumerGroupLagHistory {
                group_name: g.group_name,
                lag_series: vec![g.total_lag],
                consumed_series: vec![total_consumed],
                produced_series: vec![total_produced],
            }
        })
        .collect();

    Ok(Json(TopicConsumerLagHistoryResponse {
        topic: snapshot.topic,
        start_time: query.start_time.unwrap_or(now),
        end_time: query.end_time.unwrap_or(now),
        data_points: 1,
        timestamps: vec![now],
        consumer_groups,
    }))
}
