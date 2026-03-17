use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

use crate::{
    db::{
        favorite::{
            create_favorite, create_group, delete_favorite, delete_favorite_by_topic,
            delete_group, get_all_favorites_with_groups, get_all_groups, get_favorite_by_id,
            get_group_by_id, is_topic_favorite, update_favorite, update_group,
            CreateFavoriteRequest, CreateGroupRequest, GroupWithFavorites, UpdateFavoriteRequest,
            UpdateGroupRequest,
        },
        DbPool,
    },
    error::{AppError, Result},
};

/// 创建路由
pub fn routes() -> Router<Arc<crate::AppState>> {
    Router::new()
        // 分组管理
        .route("/groups", get(list_groups).post(create_group_handler))
        .route(
            "/groups/:id",
            get(get_group).put(update_group_handler).delete(delete_group_handler),
        )
        // 收藏管理
        .route("/favorites", get(list_favorites).post(create_favorite_handler))
        .route(
            "/favorites/:id",
            get(get_favorite).put(update_favorite_handler).delete(delete_favorite_handler),
        )
        .route("/favorites/check", post(check_favorite))
        .route("/favorites/topic", delete(delete_favorite_by_topic_handler))
}

/// 响应结构
#[derive(serde::Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    fn message(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            data: None,
            message: Some(msg.into()),
        }
    }
}

/// 列表所有分组
async fn list_groups(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<ApiResponse<Vec<crate::db::favorite::FavoriteGroup>>>> {
    let groups = get_all_groups(&state.db_pool).await.map_err(AppError::from)?;
    Ok(Json(ApiResponse::success(groups)))
}

/// 创建分组
async fn create_group_handler(
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<(StatusCode, Json<ApiResponse<crate::db::favorite::FavoriteGroup>>)> {
    let group = create_group(&state.db_pool, &req).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(group))))
}

/// 获取单个分组
async fn get_group(
    State(state): State<Arc<crate::AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<crate::db::favorite::FavoriteGroup>>> {
    let group = get_group_by_id(&state.db_pool, id)
        .await
        .map_err(AppError::from)?;

    match group {
        Some(g) => Ok(Json(ApiResponse::success(g))),
        None => Err(AppError::NotFound(format!("Group {} not found", id))),
    }
}

/// 更新分组
async fn update_group_handler(
    State(state): State<Arc<crate::AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<ApiResponse<crate::db::favorite::FavoriteGroup>>> {
    let group = update_group(&state.db_pool, id, &req)
        .await
        .map_err(AppError::from)?;

    match group {
        Some(g) => Ok(Json(ApiResponse::success(g))),
        None => Err(AppError::NotFound(format!("Group {} not found", id))),
    }
}

/// 删除分组
async fn delete_group_handler(
    State(state): State<Arc<crate::AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>> {
    let deleted = delete_group(&state.db_pool, id).await.map_err(AppError::from)?;

    if deleted {
        Ok(Json(ApiResponse::message("Group deleted successfully")))
    } else {
        Err(AppError::NotFound(format!("Group {} not found", id)))
    }
}

/// 获取所有收藏（按分组）
async fn list_favorites(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<ApiResponse<Vec<GroupWithFavorites>>>> {
    let favorites = get_all_favorites_with_groups(&state.db_pool)
        .await
        .map_err(AppError::from)?;
    Ok(Json(ApiResponse::success(favorites)))
}

/// 创建收藏
async fn create_favorite_handler(
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<CreateFavoriteRequest>,
) -> Result<(StatusCode, Json<ApiResponse<crate::db::favorite::FavoriteItem>>)> {
    let item = create_favorite(&state.db_pool, &req).await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(item))))
}

/// 获取单个收藏
async fn get_favorite(
    State(state): State<Arc<crate::AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<crate::db::favorite::FavoriteItem>>> {
    let item = get_favorite_by_id(&state.db_pool, id)
        .await
        .map_err(AppError::from)?;

    match item {
        Some(i) => Ok(Json(ApiResponse::success(i))),
        None => Err(AppError::NotFound(format!("Favorite {} not found", id))),
    }
}

/// 更新收藏
async fn update_favorite_handler(
    State(state): State<Arc<crate::AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateFavoriteRequest>,
) -> Result<Json<ApiResponse<crate::db::favorite::FavoriteItem>>> {
    let item = update_favorite(&state.db_pool, id, &req)
        .await
        .map_err(AppError::from)?;

    match item {
        Some(i) => Ok(Json(ApiResponse::success(i))),
        None => Err(AppError::NotFound(format!("Favorite {} not found", id))),
    }
}

/// 删除收藏
async fn delete_favorite_handler(
    State(state): State<Arc<crate::AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>> {
    let deleted = delete_favorite(&state.db_pool, id).await.map_err(AppError::from)?;

    if deleted {
        Ok(Json(ApiResponse::message("Favorite deleted successfully")))
    } else {
        Err(AppError::NotFound(format!("Favorite {} not found", id)))
    }
}

/// 检查收藏请求
#[derive(serde::Deserialize)]
struct CheckFavoriteRequest {
    cluster_id: String,
    topic_name: String,
}

/// 检查 topic 是否已收藏
async fn check_favorite(
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<CheckFavoriteRequest>,
) -> Result<Json<ApiResponse<bool>>> {
    let is_fav = is_topic_favorite(&state.db_pool, &req.cluster_id, &req.topic_name)
        .await
        .map_err(AppError::from)?;

    Ok(Json(ApiResponse::success(is_fav)))
}

/// 删除收藏请求
#[derive(serde::Deserialize)]
struct DeleteByTopicRequest {
    cluster_id: String,
    topic_name: String,
}

/// 根据 topic 删除收藏
async fn delete_favorite_by_topic_handler(
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<DeleteByTopicRequest>,
) -> Result<Json<ApiResponse<()>>> {
    let deleted = delete_favorite_by_topic(&state.db_pool, &req.cluster_id, &req.topic_name)
        .await
        .map_err(AppError::from)?;

    if deleted {
        Ok(Json(ApiResponse::message("Favorite deleted successfully")))
    } else {
        Err(AppError::NotFound("Favorite not found".to_string()))
    }
}
