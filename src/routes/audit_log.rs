use crate::db::audit_log::AuditLogStore;
use crate::error::Result;
use crate::AppState;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_audit_logs))
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    /// 每页数量
    limit: Option<i64>,
    /// 偏移量
    offset: Option<i64>,
    /// 按 action 过滤
    action: Option<String>,
    /// 按集群 ID 过滤
    cluster_id: Option<String>,
    /// 按状态码过滤
    status: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogInfo {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub cluster_id: Option<String>,
    pub resource: Option<String>,
    pub action: String,
    pub api_key: Option<String>,
    pub status: u16,
    pub duration_ms: u128,
    pub client_ip: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub logs: Vec<AuditLogInfo>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct ClearOldLogsRequest {
    /// 保留最近 N 天的日志
    pub days: i64,
}

#[derive(Debug, Serialize)]
pub struct ClearOldLogsResponse {
    pub deleted: i64,
}

async fn list_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> Result<Json<AuditLogListResponse>> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let total = AuditLogStore::count(
        state.db.inner(),
        params.action.as_deref(),
        params.cluster_id.as_deref(),
        params.status,
    )
    .await?;

    let logs = AuditLogStore::list(
        state.db.inner(),
        limit,
        offset,
        params.action.as_deref(),
        params.cluster_id.as_deref(),
        params.status,
    )
    .await?;

    let log_infos: Vec<AuditLogInfo> = logs
        .into_iter()
        .map(|log| {
            // 解析时间戳
            let timestamp = DateTime::parse_from_rfc3339(&log.timestamp)
                .unwrap_or_else(|_| DateTime::from_timestamp(0, 0).unwrap().into())
                .with_timezone(&Utc);

            AuditLogInfo {
                id: log.id,
                timestamp,
                method: log.method,
                path: log.path,
                cluster_id: log.cluster_id,
                resource: log.resource,
                action: log.action,
                api_key: log.api_key,
                status: log.status as u16,
                duration_ms: log.duration_ms as u128,
                client_ip: log.client_ip,
            }
        })
        .collect();

    Ok(Json(AuditLogListResponse {
        logs: log_infos,
        total,
        limit,
        offset,
    }))
}

async fn clear_old_logs(
    State(state): State<AppState>,
    Json(req): Json<ClearOldLogsRequest>,
) -> Result<Json<ClearOldLogsResponse>> {
    let deleted = AuditLogStore::delete_old(state.db.inner(), req.days).await?;

    Ok(Json(ClearOldLogsResponse { deleted }))
}
