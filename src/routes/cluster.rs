use crate::db::cluster::{ClusterStore, CreateClusterRequest, UpdateClusterRequest};
use crate::db::topic::TopicStore;
use crate::error::{AppError, Result};
use crate::kafka::KafkaClients;
use crate::AppState;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_clusters).post(create_cluster))
        // 操作型路由使用 _ 前缀，避免与集群 ID 冲突
        .route("/_test/:id", post(test_cluster))
        // 集群 CRUD 路由放在最后
        .route("/:id", get(get_cluster).put(update_cluster).delete(delete_cluster))
}

#[derive(Debug, Serialize)]
pub struct ClusterInfo {
    pub id: i64,
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i64,
    pub operation_timeout_ms: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ClusterListResponse {
    pub clusters: Vec<ClusterInfo>,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
}

async fn list_clusters(State(state): State<AppState>) -> Result<Json<ClusterListResponse>> {
    let clusters: Vec<_> = ClusterStore::list(state.db.inner()).await?;

    let cluster_infos: Vec<ClusterInfo> = clusters
        .into_iter()
        .map(|c| ClusterInfo {
            id: c.id,
            name: c.name,
            brokers: c.brokers,
            request_timeout_ms: c.request_timeout_ms,
            operation_timeout_ms: c.operation_timeout_ms,
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .collect();

    Ok(Json(ClusterListResponse {
        clusters: cluster_infos,
    }))
}

async fn create_cluster(
    State(state): State<AppState>,
    Json(req): Json<CreateClusterRequest>,
) -> Result<Json<ClusterInfo>> {
    // 检查名称是否已存在
    if let Some(_existing) = ClusterStore::get_by_name(state.db.inner(), &req.name).await? {
        return Err(AppError::BadRequest(format!(
            "Cluster name '{}' already exists",
            req.name
        )));
    }

    // 创建集群
    let cluster = ClusterStore::create(state.db.inner(), &req).await?;

    // 重新加载 Kafka 客户端
    reload_clients(&state).await?;

    Ok(Json(ClusterInfo {
        id: cluster.id,
        name: cluster.name,
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms,
        operation_timeout_ms: cluster.operation_timeout_ms,
        created_at: cluster.created_at,
        updated_at: cluster.updated_at,
    }))
}

async fn get_cluster(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ClusterInfo>> {
    let cluster = ClusterStore::get(state.db.inner(), id).await?;

    Ok(Json(ClusterInfo {
        id: cluster.id,
        name: cluster.name,
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms,
        operation_timeout_ms: cluster.operation_timeout_ms,
        created_at: cluster.created_at,
        updated_at: cluster.updated_at,
    }))
}

async fn update_cluster(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateClusterRequest>,
) -> Result<Json<ClusterInfo>> {
    let old_cluster = ClusterStore::get(state.db.inner(), id).await?;

    // 如果名称改变，检查新名称是否已存在
    if let Some(ref new_name) = req.name {
        if new_name != &old_cluster.name {
            if let Some(_existing) = ClusterStore::get_by_name(state.db.inner(), new_name).await? {
                return Err(AppError::BadRequest(format!(
                    "Cluster name '{}' already exists",
                    new_name
                )));
            }
        }
    }

    // 更新集群
    let cluster = ClusterStore::update(state.db.inner(), id, &req).await?;

    // 重新加载 Kafka 客户端
    reload_clients(&state).await?;

    Ok(Json(ClusterInfo {
        id: cluster.id,
        name: cluster.name,
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms,
        operation_timeout_ms: cluster.operation_timeout_ms,
        created_at: cluster.created_at,
        updated_at: cluster.updated_at,
    }))
}

async fn delete_cluster(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<()> {
    // 获取集群信息
    let cluster = ClusterStore::get(state.db.inner(), id).await?;
    let cluster_name = cluster.name.clone();
    let brokers = cluster.brokers.clone();
    let request_timeout_ms = cluster.request_timeout_ms as u32;
    let operation_timeout_ms = cluster.operation_timeout_ms as u32;

    // 获取集群下的所有 topic
    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster.name).await?;
    let topic_names: Vec<String> = topics.iter().map(|t| t.topic_name.clone()).collect();

    // 删除集群
    ClusterStore::delete(state.db.inner(), id).await?;

    // 重新加载 Kafka 客户端（移除已删除的集群）
    reload_clients(&state).await?;

    // 异步删除 Kafka 集群中的 topic（不阻塞响应）
    if !topic_names.is_empty() {
        tokio::spawn(async move {
            use crate::config::KafkaConfig;
            use crate::kafka::KafkaAdmin;

            let kafka_config = KafkaConfig {
                brokers,
                request_timeout_ms,
                operation_timeout_ms,
            };

            match KafkaAdmin::new(&kafka_config) {
                Ok(admin) => {
                    tracing::info!("Starting async deletion of {} topics for cluster '{}'", topic_names.len(), cluster_name);

                    // 并发删除所有 topic
                    let delete_tasks: Vec<_> = topic_names.iter().map(|topic_name| {
                        let admin = admin.clone();
                        let topic_name = topic_name.clone();
                        async move {
                            match admin.delete_topic(&topic_name).await {
                                Ok(_) => {
                                    tracing::info!("Deleted topic '{}'", topic_name);
                                    Ok(())
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to delete topic '{}': {}", topic_name, e);
                                    Err(e)
                                }
                            }
                        }
                    }).collect();

                    let results = futures::future::join_all(delete_tasks).await;
                    let success_count = results.iter().filter(|r| r.is_ok()).count();
                    let failed_count = results.iter().filter(|r| r.is_err()).count();

                    tracing::info!(
                        "Completed topic deletion for cluster '{}': {} succeeded, {} failed",
                        cluster_name,
                        success_count,
                        failed_count
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to create Kafka admin client for cluster '{}': {}", cluster_name, e);
                }
            }
        });
    }

    Ok(())
}

async fn test_cluster(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<TestConnectionResponse>> {
    let success = ClusterStore::test_connection(state.db.inner(), id).await?;

    Ok(Json(TestConnectionResponse { success }))
}

/// 重新加载 Kafka 客户端
async fn reload_clients(state: &AppState) -> Result<()> {
    use crate::config::KafkaConfig;
    use crate::db::topic::TopicStore;
    use futures::future::join_all;

    // 从数据库获取所有集群
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

    // 创建新的 KafkaClients
    let new_clients = KafkaClients::new(&new_clusters)?;

    // 并行同步每个集群的 Topic 列表
    let sync_tasks: Vec<_> = clusters.iter().map(|cluster| {
        let new_clients = &new_clients;
        let db = state.db.inner();
        async move {
            if let Some(admin) = new_clients.get_admin(&cluster.name) {
                if let Ok(topics) = admin.list_topics() {
                    let _ = TopicStore::sync_topics(db, &cluster.name, &topics).await;
                    tracing::info!("Synced {} topics for cluster '{}'", topics.len(), cluster.name);
                }
            }
        }
    }).collect();

    join_all(sync_tasks).await;

    // 原子更新 Kafka 客户端
    state.set_clients(new_clients.into());

    tracing::info!("Reloaded Kafka clients");

    Ok(())
}
