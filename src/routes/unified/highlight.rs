/// JSON Highlight Template handlers for unified API
use crate::db::json_highlight::JsonHighlightTemplate;
use crate::db::settings::SettingStore;
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;

pub async fn handle_json_highlight_list(state: AppState) -> Result<Value> {
    let templates: Vec<Value> = JsonHighlightTemplate::get_all_templates(state.db.inner())
        .await?.into_iter().map(|t| {
            serde_json::json!({
                "id": t.id, "name": t.name, "description": t.description,
                "is_builtin": t.is_builtin, "style_json": t.style_json,
                "created_at": t.created_at, "updated_at": t.updated_at,
            })
        }).collect();
    Ok(serde_json::json!({ "templates": templates }))
}

pub async fn handle_json_highlight_get_current(state: AppState) -> Result<Value> {
    let name = SettingStore::get(state.db.inner(), "ui.json_highlight_template")
        .await?.unwrap_or_else(|| "default".to_string());
    Ok(serde_json::json!({ "name": name }))
}

pub async fn handle_json_highlight_set_current(state: AppState, body: Value) -> Result<Value> {
    let name = super::get_string_param(&body, "name")?;
    SettingStore::set(state.db.inner(), "ui.json_highlight_template", &name).await?;
    Ok(serde_json::json!({ "name": name }))
}

pub async fn handle_json_highlight_create(state: AppState, body: Value) -> Result<Value> {
    let name = super::get_string_param(&body, "name")?;
    let description = super::get_string_param(&body, "description")?;
    let style_json = super::get_long_string_param(&body, "style_json")?;

    if let Err(e) = JsonHighlightTemplate::validate_style_json(&style_json) {
        return Err(AppError::BadRequest(format!("模板样式验证失败：{}", e)));
    }

    let templates = JsonHighlightTemplate::get_all_templates(state.db.inner()).await?;
    if templates.iter().any(|t| t.name == name) {
        return Err(AppError::BadRequest(format!("模板名称 '{}' 已存在，请使用其他名称", name)));
    }

    let id = JsonHighlightTemplate::save_template(state.db.inner(), &name, &description, false, &style_json).await?;
    Ok(serde_json::json!({ "id": id }))
}

pub async fn handle_json_highlight_update(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let description = super::get_string_param(&body, "description")?;
    let style_json = super::get_long_string_param(&body, "style_json")?;

    let template = JsonHighlightTemplate::get_template_by_id(state.db.inner(), id)
        .await?.ok_or_else(|| AppError::NotFound(format!("Template {} not found", id)))?;

    if template.is_builtin {
        return Err(AppError::BadRequest("Cannot update built-in templates".to_string()));
    }

    if let Err(e) = JsonHighlightTemplate::validate_style_json(&style_json) {
        return Err(AppError::BadRequest(format!("模板样式验证失败：{}", e)));
    }

    JsonHighlightTemplate::save_template(state.db.inner(), &template.name, &description, false, &style_json).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_json_highlight_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let deleted = JsonHighlightTemplate::delete_template(state.db.inner(), id).await?;
    if !deleted {
        return Err(AppError::NotFound("Template not found or cannot delete built-in templates".to_string()));
    }
    Ok(serde_json::json!({ "success": true }))
}
