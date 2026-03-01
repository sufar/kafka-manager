use crate::db::topic_template::{
    TopicTemplateStore, CreateTopicTemplateRequest, UpdateTopicTemplateRequest,
    CreateTopicFromTemplateRequest, TopicTemplateResponse,
};
use crate::db::topic_template::preset_templates::get_preset_templates;
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
        // 预定义模板
        .route("/topic-templates/presets", get(list_preset_templates))
        // 自定义模板
        .route("/topic-templates", get(list_templates))
        .route("/topic-templates", post(create_template))
        .route("/topic-templates/:id", get(get_template))
        .route("/topic-templates/:id", put(update_template))
        .route("/topic-templates/:id", delete(delete_template))
        // 使用模板创建 Topic
        .route("/topics/from-template", post(create_topic_from_template))
}

/// 列出预定义模板
async fn list_preset_templates() -> Result<Json<Vec<CreateTopicTemplateRequest>>> {
    Ok(Json(get_preset_templates()))
}

/// 列出所有自定义模板
async fn list_templates(State(state): State<AppState>) -> Result<Json<Vec<TopicTemplateResponse>>> {
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let templates = store.list().await?;
    Ok(Json(templates.into_iter().map(|t| t.into()).collect()))
}

/// 获取单个模板
async fn get_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<TopicTemplateResponse>> {
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let template = store.get(id).await?
        .ok_or_else(|| AppError::NotFound(format!("Template {} not found", id)))?;
    Ok(Json(template.into()))
}

/// 创建模板
async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<CreateTopicTemplateRequest>,
) -> Result<Json<TopicTemplateResponse>> {
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let id = store.create(&req).await?;

    let template = store.get(id).await?
        .ok_or_else(|| AppError::Internal("Failed to get created template".to_string()))?;

    Ok(Json(template.into()))
}

/// 更新模板
async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTopicTemplateRequest>,
) -> Result<()> {
    let store = TopicTemplateStore::new(state.db.inner().clone());
    store.update(id, &req).await?;
    Ok(())
}

/// 删除模板
async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<()> {
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let deleted = store.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound(format!("Template {} not found", id)));
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateTopicFromTemplateQuery {
    pub cluster_id: String,
}

/// 使用模板创建 Topic
async fn create_topic_from_template(
    State(state): State<AppState>,
    Query(query): Query<CreateTopicFromTemplateQuery>,
    Json(req): Json<CreateTopicFromTemplateRequest>,
) -> Result<Json<serde_json::Value>> {
    let clients = state.clients.read().await;
    let admin = clients
        .get_admin(&query.cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", query.cluster_id)))?;

    let store = TopicTemplateStore::new(state.db.inner().clone());

    // 获取模板
    let template = if let Some(id) = req.template_id {
        store.get(id).await?
    } else if let Some(name) = &req.template_name {
        store.get_by_name(name).await?
    } else {
        // 默认使用 default 模板
        store.get_by_name("default").await?
    }
    .ok_or_else(|| AppError::NotFound("Template not found".to_string()))?;

    let template_resp: TopicTemplateResponse = template.into();

    // 合并配置
    let mut final_config = template_resp.config.clone();
    if let Some(override_config) = &req.override_config {
        for (key, value) in override_config {
            final_config.insert(key.clone(), value.clone());
        }
    }

    // 创建 Topic
    admin.create_topic(
        &req.topic_name,
        template_resp.num_partitions,
        template_resp.replication_factor,
        final_config,
    ).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "topic": req.topic_name,
        "template": template_resp.name,
        "num_partitions": template_resp.num_partitions,
        "replication_factor": template_resp.replication_factor,
    })))
}
