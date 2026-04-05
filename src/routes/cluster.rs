use crate::db::cluster::{ClusterStore, CreateClusterRequest, UpdateClusterRequest};
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

    // 删除该集群下的所有关联数据（按依赖顺序）
    // 1. 删除 Topic 收藏（favorite_items 表中 cluster_id 匹配的记录）
    delete_favorites_by_cluster(&state.db, &cluster_name).await?;

    // 2. 删除 Topic 浏览历史
    delete_history_by_cluster(&state.db, &cluster_name).await?;

    // 3. 删除 Topic 发送历史
    delete_sent_messages_by_cluster(&state.db, &cluster_name).await?;

    // 4. 删除 Consumer Group 元数据和 Offset
    delete_consumer_groups_by_cluster(&state.db, &cluster_name).await?;

    // 5. 删除 Schema Registry 配置和 Schema 缓存
    delete_schema_registry_by_cluster(&state.db, &cluster_name).await?;

    // 6. 删除资源标签
    delete_tags_by_cluster(&state.db, &cluster_name).await?;

    // 删除集群
    ClusterStore::delete(state.db.inner(), id).await?;

    // 重新加载 Kafka 客户端（移除已删除的集群）
    reload_clients(&state).await?;

    // 删除本地缓存的 Topic 元数据
    delete_topic_metadata_by_cluster(state.db.inner(), &cluster_name).await?;

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

/// 删除指定集群的所有 Topic 收藏
async fn delete_favorites_by_cluster(pool: &crate::db::DbPool, cluster_id: &str) -> Result<()> {
    // 直接删除该集群下的所有收藏
    let result = sqlx::query("DELETE FROM favorite_items WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool.inner())
        .await?;

    tracing::info!("Deleted {} favorites for cluster '{}'", result.rows_affected(), cluster_id);
    Ok(())
}

/// 删除指定集群的所有 Topic 浏览历史
async fn delete_history_by_cluster(pool: &crate::db::DbPool, cluster_id: &str) -> Result<()> {
    // 直接删除该集群下的所有历史记录
    let result = sqlx::query("DELETE FROM topic_history WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool.inner())
        .await?;

    tracing::info!("Deleted {} history records for cluster '{}'", result.rows_affected(), cluster_id);
    Ok(())
}

/// 删除指定集群的所有 Topic 发送历史
async fn delete_sent_messages_by_cluster(pool: &crate::db::DbPool, cluster_id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM sent_messages WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool.inner())
        .await?;

    tracing::info!(
        "Deleted {} sent messages for cluster '{}'",
        result.rows_affected(),
        cluster_id
    );
    Ok(())
}

/// 删除指定集群的所有 Consumer Group 数据
async fn delete_consumer_groups_by_cluster(pool: &crate::db::DbPool, cluster_id: &str) -> Result<()> {
    // 先删除 offsets，再删除 metadata（外键依赖）
    let result = sqlx::query("DELETE FROM consumer_group_offsets WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool.inner())
        .await?;
    tracing::info!(
        "Deleted {} consumer group offsets for cluster '{}'",
        result.rows_affected(),
        cluster_id
    );

    // 获取所有 group 名称用于日志
    let groups: Vec<String> = sqlx::query_scalar(
        "SELECT group_name FROM consumer_group_metadata WHERE cluster_id = ?"
    )
    .bind(cluster_id)
    .fetch_all(pool.inner())
    .await?;

    // 删除 metadata
    let result = sqlx::query("DELETE FROM consumer_group_metadata WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool.inner())
        .await?;
    tracing::info!(
        "Deleted {} consumer groups for cluster '{}': {:?}",
        result.rows_affected(),
        cluster_id,
        groups
    );
    Ok(())
}

/// 删除指定集群的所有 Schema Registry 数据
async fn delete_schema_registry_by_cluster(pool: &crate::db::DbPool, cluster_id: &str) -> Result<()> {
    use crate::db::schema_registry::SchemaRegistryStore;

    // 删除配置
    let _ = SchemaRegistryStore::delete_config(pool.inner(), cluster_id).await;
    tracing::info!("Deleted schema registry config for cluster '{}'", cluster_id);

    // 删除 Schema 缓存
    let result = sqlx::query("DELETE FROM schemas WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool.inner())
        .await?;
    tracing::info!(
        "Deleted {} schemas for cluster '{}'",
        result.rows_affected(),
        cluster_id
    );
    Ok(())
}

/// 删除指定集群的 Topic 元数据（本地缓存）
async fn delete_topic_metadata_by_cluster(pool: &sqlx::SqlitePool, cluster_id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM topic_metadata WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool)
        .await?;
    tracing::info!(
        "Deleted {} topic metadata for cluster '{}'",
        result.rows_affected(),
        cluster_id
    );
    Ok(())
}

/// 删除指定集群的所有资源标签
async fn delete_tags_by_cluster(pool: &crate::db::DbPool, cluster_id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM resource_tags WHERE cluster_id = ?")
        .bind(cluster_id)
        .execute(pool.inner())
        .await?;
    tracing::info!(
        "Deleted {} resource tags for cluster '{}'",
        result.rows_affected(),
        cluster_id
    );
    Ok(())
}
