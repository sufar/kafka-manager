/// Cluster Group handlers for unified API
use crate::db::cluster_group::{ClusterGroupStore, CreateClusterGroupRequest, UpdateClusterGroupRequest};
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;

pub async fn handle_cluster_group_list(state: AppState) -> Result<Value> {
    let groups = ClusterGroupStore::list(state.db.inner()).await?;
    let groups_json: Vec<Value> = groups.into_iter().map(|g| {
        serde_json::json!({
            "id": g.id, "name": g.name, "description": g.description,
            "sort_order": g.sort_order, "created_at": g.created_at, "updated_at": g.updated_at,
        })
    }).collect();
    Ok(serde_json::json!(groups_json))
}

pub async fn handle_cluster_group_get(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let group = ClusterGroupStore::get(state.db.inner(), id).await?;
    Ok(serde_json::json!({
        "id": group.id, "name": group.name, "description": group.description,
        "sort_order": group.sort_order, "created_at": group.created_at, "updated_at": group.updated_at,
    }))
}

pub async fn handle_cluster_group_create(state: AppState, body: Value) -> Result<Value> {
    let name = super::get_string_param(&body, "name")?;
    let description = super::get_optional_string_param(&body, "description");
    let sort_order = super::get_optional_i64_param(&body, "sort_order").unwrap_or(0);
    if let Some(_existing) = ClusterGroupStore::get_by_name(state.db.inner(), &name).await? {
        return Err(AppError::BadRequest(format!("Group name '{}' already exists", name)));
    }
    let req = CreateClusterGroupRequest { name: name.clone(), description, sort_order };
    let group = ClusterGroupStore::create(state.db.inner(), &req).await?;
    Ok(serde_json::json!({
        "id": group.id, "name": group.name, "description": group.description,
        "sort_order": group.sort_order, "created_at": group.created_at, "updated_at": group.updated_at,
    }))
}

pub async fn handle_cluster_group_update(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let name = super::get_optional_string_param(&body, "name");
    let description = super::get_optional_string_param(&body, "description");
    let sort_order = super::get_optional_i64_param(&body, "sort_order");
    if let Some(ref new_name) = name {
        if let Some(existing) = ClusterGroupStore::get_by_name(state.db.inner(), new_name).await? {
            if existing.id != id {
                return Err(AppError::BadRequest(format!("Group name '{}' already exists", new_name)));
            }
        }
    }
    let req = UpdateClusterGroupRequest { name, description, sort_order };
    let group = ClusterGroupStore::update(state.db.inner(), id, &req).await?;
    Ok(serde_json::json!({
        "id": group.id, "name": group.name, "description": group.description,
        "sort_order": group.sort_order, "created_at": group.created_at, "updated_at": group.updated_at,
    }))
}

pub async fn handle_cluster_group_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    ClusterGroupStore::delete(state.db.inner(), id).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_cluster_group_clusters(state: AppState, body: Value) -> Result<Value> {
    let group_id = super::get_i64_param(&body, "group_id")?;
    let clusters = ClusterGroupStore::get_clusters_in_group(state.db.inner(), group_id).await?;
    let clusters_json: Vec<Value> = clusters.into_iter().map(|c| {
        serde_json::json!({
            "id": c.id, "name": c.name, "brokers": c.brokers,
            "request_timeout_ms": c.request_timeout_ms, "operation_timeout_ms": c.operation_timeout_ms,
            "group_id": c.group_id, "created_at": c.created_at, "updated_at": c.updated_at,
        })
    }).collect();
    Ok(serde_json::json!(clusters_json))
}

pub async fn handle_cluster_group_assign_cluster(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_i64_param(&body, "cluster_id")?;
    let group_id = super::get_i64_param(&body, "group_id")?;
    ClusterGroupStore::assign_cluster_to_group(state.db.inner(), cluster_id, group_id).await?;
    Ok(serde_json::json!({ "success": true }))
}
