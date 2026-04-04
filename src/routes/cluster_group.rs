use crate::db::cluster::{ClusterStore, KafkaCluster};
use crate::db::cluster_group::{ClusterGroup, ClusterGroupStore, CreateClusterGroupRequest, UpdateClusterGroupRequest};
use crate::error::{AppError, Result};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};

/// 分组路由
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_groups).post(create_group))
        .route("/:id", get(get_group).put(update_group).delete(delete_group))
        .route("/:id/clusters", get(get_clusters_in_group))
}

#[derive(Debug, Deserialize)]
pub struct AssignClusterToGroupRequest {
    pub cluster_id: i64,
}

#[derive(Debug, Serialize)]
pub struct ClusterGroupWithClusters {
    pub group: ClusterGroup,
    pub clusters: Vec<ClusterInfo>,
}

#[derive(Debug, Serialize)]
pub struct ClusterInfo {
    pub id: i64,
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i64,
    pub operation_timeout_ms: i64,
    pub group_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct GroupListResponse {
    pub groups: Vec<ClusterGroup>,
}

#[derive(Debug, Serialize)]
pub struct GroupWithClustersListResponse {
    pub groups: Vec<ClusterGroupWithClusters>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_offset")]
    pub offset: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_offset() -> i64 {
    0
}

fn default_limit() -> i64 {
    100
}

async fn list_groups(State(state): State<AppState>) -> Result<Json<GroupListResponse>> {
    let groups = ClusterGroupStore::list(state.db.inner()).await?;

    Ok(Json(GroupListResponse { groups }))
}

async fn list_groups_with_clusters(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<GroupWithClustersListResponse>> {
    let groups = ClusterGroupStore::list(state.db.inner()).await?;

    let mut groups_with_clusters = Vec::with_capacity(groups.len());
    for group in groups {
        let clusters = ClusterGroupStore::get_clusters_in_group(state.db.inner(), group.id)
            .await?
            .into_iter()
            .skip(pagination.offset as usize)
            .take(pagination.limit as usize)
            .map(|c| ClusterInfo {
                id: c.id,
                name: c.name,
                brokers: c.brokers,
                request_timeout_ms: c.request_timeout_ms,
                operation_timeout_ms: c.operation_timeout_ms,
                group_id: Some(c.id),
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect();

        groups_with_clusters.push(ClusterGroupWithClusters {
            group,
            clusters,
        });
    }

    Ok(Json(GroupWithClustersListResponse {
        groups: groups_with_clusters,
    }))
}

async fn get_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ClusterGroup>> {
    let group = ClusterGroupStore::get(state.db.inner(), id).await?;
    Ok(Json(group))
}

async fn create_group(
    State(state): State<AppState>,
    Json(req): Json<CreateClusterGroupRequest>,
) -> Result<Json<ClusterGroup>> {
    // 检查名称是否已存在
    if let Some(_existing) = ClusterGroupStore::get_by_name(state.db.inner(), &req.name).await? {
        return Err(AppError::BadRequest(format!(
            "Group name '{}' already exists",
            req.name
        )));
    }

    let group = ClusterGroupStore::create(state.db.inner(), &req).await?;
    Ok(Json(group))
}

async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateClusterGroupRequest>,
) -> Result<Json<ClusterGroup>> {
    // 如果名称改变，检查新名称是否已存在
    if let Some(ref new_name) = req.name {
        if let Some(existing) = ClusterGroupStore::get_by_name(state.db.inner(), new_name).await? {
            if existing.id != id {
                return Err(AppError::BadRequest(format!(
                    "Group name '{}' already exists",
                    new_name
                )));
            }
        }
    }

    let group = ClusterGroupStore::update(state.db.inner(), id, &req).await?;
    Ok(Json(group))
}

async fn delete_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<()> {
    ClusterGroupStore::delete(state.db.inner(), id).await?;
    Ok(())
}

async fn get_clusters_in_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ClusterInfo>>> {
    let clusters = ClusterGroupStore::get_clusters_in_group(state.db.inner(), id).await?;

    let cluster_infos: Vec<ClusterInfo> = clusters
        .into_iter()
        .map(|c| ClusterInfo {
            id: c.id,
            name: c.name,
            brokers: c.brokers,
            request_timeout_ms: c.request_timeout_ms,
            operation_timeout_ms: c.operation_timeout_ms,
            group_id: Some(id),
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .collect();

    Ok(Json(cluster_infos))
}
