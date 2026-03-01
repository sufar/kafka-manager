use crate::error::{AppError, Result};
use crate::kafka::offset::KafkaOffsetManager;
use crate::models::{
    CreateTopicRequest, CreateTopicResponse, PartitionDetail, TopicDetailResponse,
    TopicListResponse, TopicPartitionDetail,
};
use crate::AppState;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use futures::future::join_all;

pub fn cluster_routes() -> Router<AppState> {
    // 在 Axum 中，路由匹配优先级基于路径段的字面量匹配
    // 字面量路径（如/_count）应该优先于动态路径（如/:name）
    // 但由于 Axum 的路由树构建方式，我们需要使用不同的策略

    // 解决方案：将操作路由放在单独的路由器中，并在 mod.rs 中先注册
    // 这里只返回 topic 资源路由
    Router::new()
        // 根路由
        .route("/", get(list_topics).post(create_topic))
        // Topic 资源路由
        .route("/:name", get(get_topic).delete(delete_topic))
        .route("/:name/config", get(get_config).post(alter_config))
        .route("/:name/offsets", get(get_topic_offsets))
        .route("/:name/partitions", post(add_partitions))
        .route("/:name/throughput", get(get_topic_throughput))
}

// 操作型路由（/_count, /_saved 等）- 在单独的路由器中定义
pub fn topic_operation_routes() -> Router<AppState> {
    Router::new()
        .route("/_count", get(get_topic_count))
        .route("/_saved", get(list_saved_topics))
        .route("/_refresh", post(refresh_topics))
        .route("/_batch", post(batch_create_topics).delete(batch_delete_topics))
}

pub fn global_routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(search_topics_all_clusters))
}

#[derive(Debug, Deserialize)]
pub struct AddPartitionsRequest {
    pub new_partitions: i32,
}

#[derive(Debug, Deserialize)]
pub struct AlterConfigRequest {
    pub config: HashMap<String, String>,
}

async fn list_topics(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<TopicListResponse>> {
    // 从 Kafka 集群实时获取 topics
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let topics = admin.list_topics()?;
    Ok(Json(TopicListResponse { topics }))
}

/// 从数据库获取已保存的 topics
async fn list_saved_topics(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<TopicListResponse>> {
    use crate::db::topic::TopicStore;

    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let topic_names: Vec<String> = topics.into_iter().map(|t| t.topic_name).collect();
    Ok(Json(TopicListResponse { topics: topic_names }))
}

/// 获取集群的 topic 数量（从数据库）
#[derive(Debug, Serialize)]
pub struct TopicCountResponse {
    pub count: usize,
}

async fn get_topic_count(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<TopicCountResponse>> {
    use crate::db::topic::TopicStore;

    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    Ok(Json(TopicCountResponse { count: topics.len() }))
}

async fn create_topic(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Json(req): Json<CreateTopicRequest>,
) -> Result<Json<CreateTopicResponse>> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    admin
        .create_topic(
            &req.name,
            req.num_partitions,
            req.replication_factor,
            req.config,
        )
        .await?;

    Ok(Json(CreateTopicResponse { name: req.name }))
}

async fn get_topic(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
) -> Result<Json<TopicDetailResponse>> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let topic_info = admin.get_topic_info(&name)?;

    Ok(Json(TopicDetailResponse {
        name: topic_info.name,
        partitions: topic_info
            .partitions
            .into_iter()
            .map(|p| PartitionDetail {
                id: p.id,
                leader: p.leader,
                replicas: p.replicas,
                isr: p.isr,
            })
            .collect(),
    }))
}

async fn delete_topic(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
) -> Result<()> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    admin.delete_topic(&name).await?;
    Ok(())
}

async fn add_partitions(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
    Json(req): Json<AddPartitionsRequest>,
) -> Result<()> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    admin.create_partitions(&name, req.new_partitions).await?;
    Ok(())
}

async fn get_config(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
) -> Result<Json<HashMap<String, String>>> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let config = admin.get_topic_config(&name).await?;
    Ok(Json(config))
}

async fn alter_config(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
    Json(req): Json<AlterConfigRequest>,
) -> Result<()> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    admin.alter_topic_config(&name, req.config).await?;
    Ok(())
}

async fn get_topic_offsets(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
) -> Result<Json<Vec<TopicPartitionDetail>>> {
    let clients = state.clients.read().await;
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let offset_manager = KafkaOffsetManager::new(&config);

    // 首先获取分区列表
    let partition_offsets = offset_manager.get_topic_partition_offsets(&config, &name)?;

    // 并行获取每个分区的详细信息
    let tasks: Vec<_> = partition_offsets
        .into_iter()
        .map(|p| async move {
            Ok(TopicPartitionDetail {
                topic: p.topic,
                partition: p.partition,
                leader: p.leader,
                replicas: p.replicas,
                isr: p.isr,
                earliest_offset: p.earliest_offset,
                latest_offset: p.latest_offset,
                first_commit_time: p.first_commit_time,
                last_commit_time: p.last_commit_time,
            }) as Result<TopicPartitionDetail>
        })
        .collect();

    let details = join_all(tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    Ok(Json(details))
}

// ==================== 批量操作 ====================

/// 批量创建 Topic 请求
#[derive(Debug, Deserialize)]
pub struct BatchCreateTopicsRequest {
    pub topics: Vec<CreateTopicRequest>,
    /// 是否继续执行即使有失败
    #[serde(default)]
    pub continue_on_error: bool,
}

/// 批量创建 Topic 响应
#[derive(Debug, Serialize)]
pub struct BatchCreateTopicsResponse {
    pub success: bool,
    pub created: Vec<String>,
    pub failed: Vec<FailedTopic>,
}

/// 批量删除 Topic 请求
#[derive(Debug, Deserialize)]
pub struct BatchDeleteTopicsRequest {
    pub topics: Vec<String>,
    /// 是否继续执行即使有失败
    #[serde(default)]
    pub continue_on_error: bool,
}

/// 批量删除 Topic 响应
#[derive(Debug, Serialize)]
pub struct BatchDeleteTopicsResponse {
    pub success: bool,
    pub deleted: Vec<String>,
    pub failed: Vec<FailedTopic>,
}

/// 失败的 Topic 操作
#[derive(Debug, Serialize)]
pub struct FailedTopic {
    pub name: String,
    pub error: String,
}

/// 批量创建 Topic
async fn batch_create_topics(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Json(req): Json<BatchCreateTopicsRequest>,
) -> Result<Json<BatchCreateTopicsResponse>> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let mut created = Vec::new();
    let mut failed = Vec::new();

    for topic_req in req.topics {
        match admin
            .create_topic(
                &topic_req.name,
                topic_req.num_partitions,
                topic_req.replication_factor,
                topic_req.config,
            )
            .await
        {
            Ok(_) => created.push(topic_req.name),
            Err(e) => {
                failed.push(FailedTopic {
                    name: topic_req.name,
                    error: e.to_string(),
                });
                if !req.continue_on_error {
                    return Ok(Json(BatchCreateTopicsResponse {
                        success: false,
                        created,
                        failed,
                    }));
                }
            }
        }
    }

    Ok(Json(BatchCreateTopicsResponse {
        success: failed.is_empty(),
        created,
        failed,
    }))
}

/// 批量删除 Topic
async fn batch_delete_topics(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Json(req): Json<BatchDeleteTopicsRequest>,
) -> Result<Json<BatchDeleteTopicsResponse>> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let mut deleted = Vec::new();
    let mut failed = Vec::new();

    for topic_name in req.topics {
        match admin.delete_topic(&topic_name).await {
            Ok(_) => deleted.push(topic_name),
            Err(e) => {
                failed.push(FailedTopic {
                    name: topic_name,
                    error: e.to_string(),
                });
                if !req.continue_on_error {
                    return Ok(Json(BatchDeleteTopicsResponse {
                        success: false,
                        deleted,
                        failed,
                    }));
                }
            }
        }
    }

    Ok(Json(BatchDeleteTopicsResponse {
        success: failed.is_empty(),
        deleted,
        failed,
    }))
}

// ==================== Topic 吞吐量 ====================

use crate::kafka::throughput::KafkaThroughputCalculator;
use crate::models::TopicThroughputResponse;

/// 获取 Topic 吞吐量（生产速度）
async fn get_topic_throughput(
    State(state): State<AppState>,
    Path((cluster_id, name)): Path<(String, String)>,
) -> Result<Json<TopicThroughputResponse>> {
    let clients = state.clients.read().await;
    let config = clients
        .get_config(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let calculator = KafkaThroughputCalculator::new(&config);
    let throughput = calculator.calculate_topic_throughput(&config, &name)?;

    Ok(Json(throughput))
}

// ==================== Topic 同步 ====================

/// 刷新 Topic 列表响应
#[derive(Debug, Serialize)]
pub struct RefreshTopicsResponse {
    pub success: bool,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub total: usize,
}

/// 刷新集群 Topic 列表（从 Kafka 集群同步到数据库）
async fn refresh_topics(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<RefreshTopicsResponse>> {
    use crate::db::topic::TopicStore;

    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    // 从 Kafka 集群获取当前 Topic 列表
    let current_topics = admin.list_topics()?;

    // 同步到数据库（删除已不存在的 topics）
    let sync_result = TopicStore::sync_topics(state.db.inner(), &cluster_id, &current_topics).await?;

    // 保存新增的 topics 到数据库
    for topic_name in &sync_result.added {
        if let Ok(topic_info) = admin.get_topic_info(topic_name) {
            let config = std::collections::HashMap::new();
            let _ = TopicStore::upsert(
                state.db.inner(),
                &cluster_id,
                topic_name,
                topic_info.partitions.len() as i32,
                1,
                &config,
            ).await;
        }
    }

    // 获取更新后的总数
    let all_topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;

    Ok(Json(RefreshTopicsResponse {
        success: true,
        added: sync_result.added,
        removed: sync_result.removed,
        total: all_topics.len(),
    }))
}

// ==================== 全局搜索 Topic ====================

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicSearchResult {
    pub cluster: String,
    pub topic: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchTopicsResponse {
    pub results: Vec<TopicSearchResult>,
}

/// 搜索所有集群中的 Topic（从数据库中搜索）
async fn search_topics_all_clusters(
    State(state): State<AppState>,
) -> Result<Json<SearchTopicsResponse>> {
    use crate::db::topic::TopicStore;
    use crate::db::cluster::ClusterStore;

    // 获取所有集群
    let clusters = ClusterStore::list(state.db.inner()).await?;

    // 搜索所有集群的 topics
    let mut results = Vec::new();

    for cluster in &clusters {
        let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster.name).await?;
        for topic in topics {
            results.push(TopicSearchResult {
                cluster: cluster.name.clone(),
                topic: topic.topic_name,
            });
        }
    }

    Ok(Json(SearchTopicsResponse { results }))
}
