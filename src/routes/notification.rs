/// 告警通知管理路由

use crate::db::notification::{NotificationStore, CreateNotificationConfigRequest, NotificationConfig, AlertHistoryQuery};
use crate::error::Result;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        // 通知配置
        .route("/notifications", get(list_notifications).post(create_notification))
        .route("/notifications/:id", get(get_notification).delete(delete_notification))
        .route("/notifications/:id/enable", post(enable_notification))
        .route("/notifications/:id/disable", post(disable_notification))
        // 告警历史
        .route("/alerts/history", get(list_alert_history))
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationRequest {
    pub enabled: bool,
}

async fn list_notifications(
    State(state): State<AppState>,
) -> Result<Json<Vec<NotificationConfig>>> {
    let configs = NotificationStore::list(state.db.inner()).await?;
    Ok(Json(configs))
}

async fn create_notification(
    State(state): State<AppState>,
    Json(req): Json<CreateNotificationConfigRequest>,
) -> Result<Json<serde_json::Value>> {
    let id = NotificationStore::create(state.db.inner(), &req).await?;

    Ok(Json(serde_json::json!({
        "id": id,
        "success": true
    })))
}

async fn get_notification(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<NotificationConfig>> {
    let config = NotificationStore::get_by_id(state.db.inner(), id).await?
        .ok_or_else(|| crate::error::AppError::NotFound(format!("Notification config {} not found", id)))?;

    Ok(Json(config))
}

async fn delete_notification(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    NotificationStore::delete(state.db.inner(), id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn enable_notification(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    NotificationStore::update(state.db.inner(), id, Some(true)).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn disable_notification(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    NotificationStore::update(state.db.inner(), id, Some(false)).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn list_alert_history(
    State(state): State<AppState>,
    Query(query): Query<AlertHistoryQuery>,
) -> Result<Json<Vec<serde_json::Value>>> {
    let history = NotificationStore::list_history(state.db.inner(), &query).await?;

    let responses: Vec<serde_json::Value> = history.into_iter().map(|h| {
        serde_json::json!({
            "id": h.id,
            "rule_id": h.rule_id,
            "cluster_id": h.cluster_id,
            "alert_type": h.alert_type,
            "alert_message": h.alert_message,
            "alert_value": h.alert_value,
            "threshold": h.threshold,
            "severity": h.severity,
            "notified": h.notified,
            "created_at": h.created_at
        })
    }).collect();

    Ok(Json(responses))
}
