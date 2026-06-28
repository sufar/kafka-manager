/// JSON 高亮模板路由

use crate::db::json_highlight::{JsonHighlightTemplate, TemplateStyle};
use crate::db::settings::SettingStore;
use crate::error::Result;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_templates).post(create_template))
        .route("/builtin", get(get_builtin_templates))
        .route("/custom", get(get_custom_templates))
        .route("/:id", get(get_template_by_id).put(update_template).delete(delete_template))
        .route("/preview", post(generate_preview_css))
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
    pub style_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<JsonHighlightTemplate> for TemplateResponse {
    fn from(template: JsonHighlightTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            description: template.description,
            is_builtin: template.is_builtin,
            style_json: template.style_json,
            created_at: template.created_at,
            updated_at: template.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateResponse>,
}

/// 获取所有模板
async fn get_templates(
    State(state): State<AppState>,
) -> Result<Json<TemplatesResponse>> {
    let templates = JsonHighlightTemplate::get_all_templates(state.db.inner())
        .await?
        .into_iter()
        .map(|t| t.into())
        .collect();

    Ok(Json(TemplatesResponse { templates }))
}

/// 获取内置模板
async fn get_builtin_templates(
    State(state): State<AppState>,
) -> Result<Json<TemplatesResponse>> {
    let templates = JsonHighlightTemplate::get_builtin_templates(state.db.inner())
        .await?
        .into_iter()
        .map(|t| t.into())
        .collect();

    Ok(Json(TemplatesResponse { templates }))
}

/// 获取自定义模板
async fn get_custom_templates(
    State(state): State<AppState>,
) -> Result<Json<TemplatesResponse>> {
    let templates = JsonHighlightTemplate::get_custom_templates(state.db.inner())
        .await?
        .into_iter()
        .map(|t| t.into())
        .collect();

    Ok(Json(TemplatesResponse { templates }))
}

/// 根据 ID 获取模板
async fn get_template_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<TemplateResponse>> {
    let template = JsonHighlightTemplate::get_template_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| crate::error::Error::NotFound(format!("Template {} not found", id)))?;

    Ok(Json(template.into()))
}

/// 创建自定义模板
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: String,
    pub style_json: String,
}

async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<Json<TemplateResponse>> {
    let id = JsonHighlightTemplate::save_template(
        state.db.inner(),
        &req.name,
        &req.description,
        false,
        &req.style_json,
    )
    .await?;

    let template = JsonHighlightTemplate::get_template_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| crate::error::Error::Internal("Template created but not found".to_string()))?;

    Ok(Json(template.into()))
}

/// 更新模板
#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub description: String,
    pub style_json: String,
}

async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTemplateRequest>,
) -> Result<Json<TemplateResponse>> {
    // 获取模板信息
    let template = JsonHighlightTemplate::get_template_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| crate::error::Error::NotFound(format!("Template {} not found", id)))?;

    // 不允许更新内置模板
    if template.is_builtin {
        return Err(crate::error::Error::BadRequest(
            "Cannot update built-in templates".to_string()
        ));
    }

    // 更新模板
    JsonHighlightTemplate::save_template(
        state.db.inner(),
        &template.name,
        &req.description,
        false,
        &req.style_json,
    )
    .await?;

    let updated = JsonHighlightTemplate::get_template_by_id(state.db.inner(), id)
        .await?
        .ok_or_else(|| crate::error::Error::Internal("Template updated but not found".to_string()))?;

    Ok(Json(updated.into()))
}

/// 删除模板
async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let deleted = JsonHighlightTemplate::delete_template(state.db.inner(), id).await?;

    if !deleted {
        return Err(crate::error::Error::NotFound(
            "Template not found or cannot delete built-in templates".to_string()
        ));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// 生成预览 CSS
#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub style_json: String,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub css: String,
}

async fn generate_preview_css(
    Json(req): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>> {
    // 解析样式 JSON 并生成 CSS
    let style: TemplateStyle = serde_json::from_str(&req.style_json)
        .map_err(|e| crate::error::Error::BadRequest(format!("Invalid style JSON: {}", e)))?;

    let css = format!(
        r#".json-preview .json-key {{ color: {}; {} }}
.json-preview .json-string {{ color: {}; }}
.json-preview .json-number {{ color: {}; }}
.json-preview .json-boolean {{ color: {}; {} }}
.json-preview .json-null {{ color: {}; }}
.json-preview .json-bracket {{ color: {}; }}
.json-preview .json-colon {{ color: {}; }}
.json-preview .json-comma {{ color: {}; }}"#,
        style.key.color,
        style.key.font_weight.map(|w| format!("font-weight: {};", w)).unwrap_or_default(),
        style.string.color,
        style.number.color,
        style.boolean.color,
        style.boolean.font_weight.map(|w| format!("font-weight: {};", w)).unwrap_or_default(),
        style.null.color,
        style.bracket.color,
        style.colon.color,
        style.comma.color
    );

    Ok(Json(PreviewResponse { css }))
}

/// 获取当前选中的 JSON 高亮模板名称
#[derive(Debug, Deserialize)]
pub struct CurrentTemplateResponse {
    pub name: String,
}

pub async fn get_current_template(
    State(state): State<AppState>,
) -> Result<Json<CurrentTemplateResponse>> {
    let name = SettingStore::get(state.db.inner(), "ui.json_highlight_template")
        .await?
        .unwrap_or_else(|| "default".to_string());

    Ok(Json(CurrentTemplateResponse { name }))
}

/// 更新当前选中的 JSON 高亮模板
#[derive(Debug, Deserialize)]
pub struct UpdateCurrentTemplateRequest {
    pub name: String,
}

pub async fn update_current_template(
    State(state): State<AppState>,
    Json(req): Json<UpdateCurrentTemplateRequest>,
) -> Result<Json<CurrentTemplateResponse>> {
    SettingStore::set(state.db.inner(), "ui.json_highlight_template", &req.name).await?;

    Ok(Json(CurrentTemplateResponse { name: req.name }))
}
