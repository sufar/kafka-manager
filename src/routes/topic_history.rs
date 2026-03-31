use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use crate::{
    db::{
        topic_history::{
            clear_history, delete_history, delete_history_by_topic, get_history_list,
            record_history, RecordHistoryRequest,
        },
        DbPool,
    },
    error::{AppError, Result},
};

/// 创建路由
pub fn routes() -> Router<Arc<crate::AppState>> {
    Router::new()
        // 历史记录的增删改查
        .route("/list", get(list_history))
        .route("/record", post(record_history_handler))
        .route("/:id", delete(delete_history_handler))
        .route("/topic", delete(delete_history_by_topic_handler))
        .route("/clear", post(clear_history_handler))
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

/// 查询参数
#[derive(serde::Deserialize)]
struct HistoryListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// 列表所有浏览历史
async fn list_history(
    State(state): State<Arc<crate::AppState>>,
    Query(params): Query<HistoryListQuery>,
) -> Result<Json<ApiResponse<Vec<crate::db::topic_history::TopicHistory>>>> {
    let histories = get_history_list(&state.db_pool, params.limit, params.offset)
        .await
        .map_err(AppError::from)?;
    Ok(Json(ApiResponse::success(histories)))
}

/// 记录浏览历史
async fn record_history_handler(
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<RecordHistoryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<crate::db::topic_history::TopicHistory>>)> {
    let history = record_history(&state.db_pool, &req.cluster_id, &req.topic_name)
        .await
        .map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(history))))
}

/// 删除单条历史记录
async fn delete_history_handler(
    State(state): State<Arc<crate::AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>> {
    let deleted = delete_history(&state.db_pool, id)
        .await
        .map_err(AppError::from)?;

    if deleted {
        Ok(Json(ApiResponse::message("History deleted successfully")))
    } else {
        Err(AppError::NotFound(format!("History {} not found", id)))
    }
}

/// 删除指定 topic 的历史
#[derive(serde::Deserialize)]
struct DeleteByTopicRequest {
    cluster_id: String,
    topic_name: String,
}

async fn delete_history_by_topic_handler(
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<DeleteByTopicRequest>,
) -> Result<Json<ApiResponse<()>>> {
    let deleted = delete_history_by_topic(&state.db_pool, &req.cluster_id, &req.topic_name)
        .await
        .map_err(AppError::from)?;

    if deleted {
        Ok(Json(ApiResponse::message("History deleted successfully")))
    } else {
        Err(AppError::NotFound("History not found".to_string()))
    }
}

/// 清空所有历史记录
async fn clear_history_handler(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<ApiResponse<i64>>> {
    let count = clear_history(&state.db_pool)
        .await
        .map_err(AppError::from)?;
    Ok(Json(ApiResponse::success(count)))
}
