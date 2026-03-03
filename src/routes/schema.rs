/// Schema Registry 管理路由

use crate::kafka::schema::SchemaRegistryClient;
use crate::error::{AppError, Result};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        // 操作型路由使用 _ 前缀
        .route("/_register", post(register_schema))
        .route("/_compatibility", post(check_compatibility))
        .route("/_compatibility-level", get(get_compatibility_level).put(update_compatibility_level))
        // 通用路由
        .route("/", get(list_subjects))
        .route("/:subject", get(get_subject_versions).delete(delete_subject))
        .route("/:subject/:version", get(get_schema).delete(delete_schema_version))
}

#[derive(Debug, Deserialize)]
pub struct SchemaQuery {
    cluster_id: String,
    schema_registry_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterSchemaRequest {
    cluster_id: String,
    subject: String,
    schema: String,
    schema_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CompatibilityRequest {
    cluster_id: String,
    subject: String,
    version: String,
    schema: String,
}

#[derive(Debug, Deserialize)]
pub struct CompatibilityLevelRequest {
    cluster_id: String,
    level: String,
}

#[derive(Debug, Serialize)]
pub struct SubjectResponse {
    pub cluster_id: String,
    pub subjects: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SubjectVersionsResponse {
    pub cluster_id: String,
    pub subject: String,
    pub versions: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityLevelResponse {
    pub cluster_id: String,
    pub level: String,
}

async fn list_subjects(
    _state: State<AppState>,
    Query(query): Query<SchemaQuery>,
) -> Result<Json<SubjectResponse>> {
    let registry_url = query.schema_registry_url
        .ok_or_else(|| AppError::BadRequest("schema_registry_url is required".to_string()))?;

    let client = SchemaRegistryClient::new(&registry_url);
    let subjects = client.get_subjects().await?;

    Ok(Json(SubjectResponse {
        cluster_id: query.cluster_id,
        subjects,
    }))
}

async fn get_subject_versions(
    _state: State<AppState>,
    Path(params): Path<(String, String)>,
    Query(query): Query<SchemaQuery>,
) -> Result<Json<SubjectVersionsResponse>> {
    let (_cluster_id, subject) = params;

    let registry_url = query.schema_registry_url
        .ok_or_else(|| AppError::BadRequest("schema_registry_url is required".to_string()))?;

    let client = SchemaRegistryClient::new(&registry_url);
    let versions = client.get_versions(&subject).await?;

    Ok(Json(SubjectVersionsResponse {
        cluster_id: query.cluster_id,
        subject,
        versions,
    }))
}

async fn get_schema(
    _state: State<AppState>,
    Path(params): Path<(String, String, String)>,
    Query(query): Query<SchemaQuery>,
) -> Result<Json<crate::kafka::schema::SchemaInfo>> {
    let (_cluster_id, subject, version) = params;

    let registry_url = query.schema_registry_url
        .ok_or_else(|| AppError::BadRequest("schema_registry_url is required".to_string()))?;

    let client = SchemaRegistryClient::new(&registry_url);
    let schema = client.get_schema(&subject, &version).await?;

    Ok(Json(schema))
}

async fn register_schema(
    State(state): State<AppState>,
    Json(req): Json<RegisterSchemaRequest>,
) -> Result<Json<serde_json::Value>> {
    let clients = state.get_clients();
    let config = clients.get_config(&req.cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", req.cluster_id)))?;

    // 从配置中获取 Schema Registry URL
    let registry_url = std::env::var("SCHEMA_REGISTRY_URL")
        .unwrap_or_else(|_| format!("http://{}:8081", config.brokers.split(',').next().unwrap_or("localhost")));

    let client = SchemaRegistryClient::new(&registry_url);
    let id = client.register_schema(&req.subject, &req.schema, &req.schema_type).await?;

    Ok(Json(serde_json::json!({
        "id": id,
        "subject": req.subject,
        "success": true
    })))
}

async fn delete_schema_version(
    _state: State<AppState>,
    Path(params): Path<(String, String, String)>,
    Query(query): Query<SchemaQuery>,
) -> Result<Json<serde_json::Value>> {
    let (_cluster_id, subject, version) = params;

    let registry_url = query.schema_registry_url
        .ok_or_else(|| AppError::BadRequest("schema_registry_url is required".to_string()))?;

    let client = SchemaRegistryClient::new(&registry_url);
    let deleted_version = client.delete_schema_version(&subject, &version).await?;

    Ok(Json(serde_json::json!({
        "deleted_version": deleted_version,
        "subject": subject,
        "success": true
    })))
}

async fn delete_subject(
    _state: State<AppState>,
    Path(params): Path<(String, String)>,
    Query(query): Query<SchemaQuery>,
) -> Result<Json<serde_json::Value>> {
    let (_cluster_id, subject) = params;

    let registry_url = query.schema_registry_url
        .ok_or_else(|| AppError::BadRequest("schema_registry_url is required".to_string()))?;

    let client = SchemaRegistryClient::new(&registry_url);
    let deleted_versions = client.delete_subject(&subject).await?;

    Ok(Json(serde_json::json!({
        "deleted_versions": deleted_versions,
        "subject": subject,
        "success": true
    })))
}

async fn check_compatibility(
    _state: State<AppState>,
    Json(req): Json<CompatibilityRequest>,
) -> Result<Json<crate::kafka::schema::CompatibilityCheck>> {
    let registry_url = std::env::var("SCHEMA_REGISTRY_URL")
        .unwrap_or_else(|_| format!("http://{}:8081", req.cluster_id));

    let client = SchemaRegistryClient::new(&registry_url);
    let result = client.check_compatibility(&req.subject, &req.version, &req.schema).await?;

    Ok(Json(result))
}

async fn get_compatibility_level(
    _state: State<AppState>,
    Query(query): Query<SchemaQuery>,
) -> Result<Json<CompatibilityLevelResponse>> {
    let registry_url = query.schema_registry_url
        .ok_or_else(|| AppError::BadRequest("schema_registry_url is required".to_string()))?;

    let client = SchemaRegistryClient::new(&registry_url);
    let level = client.get_compatibility_level().await?;

    Ok(Json(CompatibilityLevelResponse {
        cluster_id: query.cluster_id,
        level,
    }))
}

async fn update_compatibility_level(
    _state: State<AppState>,
    Json(req): Json<CompatibilityLevelRequest>,
) -> Result<Json<serde_json::Value>> {
    let registry_url = std::env::var("SCHEMA_REGISTRY_URL")
        .unwrap_or_else(|_| format!("http://{}:8081", req.cluster_id));

    let client = SchemaRegistryClient::new(&registry_url);
    client.update_compatibility_level(&req.level).await?;

    Ok(Json(serde_json::json!({
        "level": req.level,
        "success": true
    })))
}
