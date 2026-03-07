use crate::db::tag::{TagStore, BatchTagsRequest, TagRequest};
use crate::error::{AppError, Result};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        // 标签管理
        .route("/tags", get(list_all_tags))
        .route("/tags/keys", get(list_tag_keys))
        .route("/tags/:key/values", get(list_tag_values))
        .route("/tags/filter", get(filter_by_tag))
        // 资源标签管理
        .route("/:resource_type/:name/tags", get(get_resource_tags))
        .route("/:resource_type/:name/tags", post(add_resource_tag))
        .route("/:resource_type/:name/tags/:key", put(update_resource_tag))
        .route("/:resource_type/:name/tags/:key", delete(delete_resource_tag))
        .route("/:resource_type/:name/tags/batch", put(batch_update_tags))
        .route("/:resource_type/:name/tags/all", delete(delete_all_resource_tags))
}

#[derive(Debug, Deserialize)]
pub struct TagListQuery {
    pub resource_type: Option<String>,
    pub resource_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FilterByTagQuery {
    pub resource_type: String,
    pub tag_key: String,
    pub tag_value: Option<String>,
}

/// 列出所有标签（可过滤）
async fn list_all_tags(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Query(query): Query<TagListQuery>,
) -> Result<Json<Vec<crate::db::tag::TagResponse>>> {
    let store = TagStore::new(state.db.inner().clone());

    // 如果没有过滤条件，返回空列表
    let Some(ref resource_type) = query.resource_type else {
        return Ok(Json(vec![]));
    };
    let Some(ref resource_name) = query.resource_name else {
        return Ok(Json(vec![]));
    };

    let tags = store.get_tags(
        &cluster_id,
        resource_type,
        resource_name,
    ).await?;

    Ok(Json(tags.into_iter().map(|t| t.into()).collect()))
}

/// 列出所有标签键
async fn list_tag_keys(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Query(query): Query<TagListQuery>,
) -> Result<Json<Vec<String>>> {
    let store = TagStore::new(state.db.inner().clone());

    let keys = store.get_all_tag_keys(&cluster_id, query.resource_type.as_deref()).await?;

    Ok(Json(keys))
}

/// 列出标签键对应的所有值
async fn list_tag_values(
    State(state): State<AppState>,
    Path((cluster_id, key)): Path<(String, String)>,
    Query(query): Query<TagListQuery>,
) -> Result<Json<Vec<String>>> {
    let store = TagStore::new(state.db.inner().clone());

    let resource_type = query.resource_type.unwrap_or_else(|| "topic".to_string());

    let values = store.get_tag_values(&cluster_id, &resource_type, &key).await?;

    Ok(Json(values))
}

/// 按标签过滤资源
async fn filter_by_tag(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
    Query(query): Query<FilterByTagQuery>,
) -> Result<Json<Vec<String>>> {
    let store = TagStore::new(state.db.inner().clone());

    let resources = store.filter_by_tag(
        &cluster_id,
        &query.resource_type,
        &query.tag_key,
        query.tag_value.as_deref(),
    ).await?;

    Ok(Json(resources))
}

/// 获取资源的标签列表
async fn get_resource_tags(
    State(state): State<AppState>,
    Path((cluster_id, resource_type, name)): Path<(String, String, String)>,
) -> Result<Json<Vec<crate::db::tag::TagResponse>>> {
    let store = TagStore::new(state.db.inner().clone());

    let tags = store.get_tags(&cluster_id, &resource_type, &name).await?;

    Ok(Json(tags.into_iter().map(|t| t.into()).collect()))
}

/// 添加/更新资源标签
async fn add_resource_tag(
    State(state): State<AppState>,
    Path((cluster_id, resource_type, name)): Path<(String, String, String)>,
    Json(req): Json<TagRequest>,
) -> Result<Json<crate::db::tag::TagResponse>> {
    let store = TagStore::new(state.db.inner().clone());

    store.add_tag(&cluster_id, &resource_type, &name, &req.key, &req.value).await?;

    let tags = store.get_tags(&cluster_id, &resource_type, &name).await?;
    let tag = tags.into_iter()
        .find(|t| t.tag_key == req.key)
        .ok_or_else(|| AppError::Internal("Failed to get created tag".to_string()))?;

    Ok(Json(tag.into()))
}

/// 更新资源标签
async fn update_resource_tag(
    State(state): State<AppState>,
    Path((cluster_id, resource_type, name, key)): Path<(String, String, String, String)>,
    Json(req): Json<TagRequest>,
) -> Result<()> {
    let store = TagStore::new(state.db.inner().clone());

    // 如果 key 不同，先删除旧的再添加新的
    if key != req.key {
        store.delete_tag(&cluster_id, &resource_type, &name, &key).await?;
    }

    store.add_tag(&cluster_id, &resource_type, &name, &req.key, &req.value).await?;

    Ok(())
}

/// 删除资源标签
async fn delete_resource_tag(
    State(state): State<AppState>,
    Path((cluster_id, resource_type, name, key)): Path<(String, String, String, String)>,
) -> Result<()> {
    let store = TagStore::new(state.db.inner().clone());

    let deleted = store.delete_tag(&cluster_id, &resource_type, &name, &key).await?;

    if !deleted {
        return Err(AppError::NotFound(format!("Tag '{}' not found", key)));
    }

    Ok(())
}

/// 批量更新资源标签
async fn batch_update_tags(
    State(state): State<AppState>,
    Path((cluster_id, resource_type, name)): Path<(String, String, String)>,
    Json(req): Json<BatchTagsRequest>,
) -> Result<()> {
    let store = TagStore::new(state.db.inner().clone());

    store.batch_update_tags(&cluster_id, &resource_type, &name, &req.tags).await?;

    Ok(())
}

/// 删除资源的所有标签
async fn delete_all_resource_tags(
    State(state): State<AppState>,
    Path((cluster_id, resource_type, name)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>> {
    let store = TagStore::new(state.db.inner().clone());

    let count = store.delete_all_tags(&cluster_id, &resource_type, &name).await?;

    Ok(Json(serde_json::json!({ "deleted": count })))
}
