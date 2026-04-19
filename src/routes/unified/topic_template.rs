/// Topic Template handlers for unified API
use crate::db::topic_template::{CreateTopicTemplateRequest, TopicTemplateStore, UpdateTopicTemplateRequest};
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;
use std::collections::HashMap;

pub async fn handle_template_list(state: AppState) -> Result<Value> {
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let templates = store.list().await?;
    let template_list: Vec<Value> = templates.into_iter().map(|t| {
        serde_json::json!({
            "id": t.id, "name": t.name, "description": t.description,
            "num_partitions": t.num_partitions, "replication_factor": t.replication_factor,
            "config": t.config_json, "created_at": t.created_at, "updated_at": t.updated_at,
        })
    }).collect();
    Ok(serde_json::json!({ "templates": template_list }))
}

pub async fn handle_template_get(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let template = store.get(id).await?.ok_or_else(|| AppError::NotFound(format!("Template {} not found", id)))?;
    Ok(serde_json::json!({
        "id": template.id, "name": template.name, "description": template.description,
        "num_partitions": template.num_partitions, "replication_factor": template.replication_factor,
        "config": template.config_json, "created_at": template.created_at, "updated_at": template.updated_at,
    }))
}

pub async fn handle_template_create(state: AppState, body: Value) -> Result<Value> {
    let name = super::get_string_param(&body, "name")?;
    let description = super::get_optional_string_param(&body, "description");
    let num_partitions = super::get_optional_i32_param(&body, "num_partitions").unwrap_or(3);
    let replication_factor = super::get_optional_i32_param(&body, "replication_factor").unwrap_or(1);
    let config = super::get_hashmap_param(&body, "config");

    let req = CreateTopicTemplateRequest { name, description, num_partitions, replication_factor, config };
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let id = store.create(&req).await?;
    let template = store.get(id).await?.ok_or_else(|| AppError::Internal("Failed to get created template".to_string()))?;
    Ok(serde_json::json!({
        "id": template.id, "name": template.name, "description": template.description,
        "num_partitions": template.num_partitions, "replication_factor": template.replication_factor,
        "config": template.config_json, "created_at": template.created_at, "updated_at": template.updated_at,
    }))
}

pub async fn handle_template_update(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let name = super::get_optional_string_param(&body, "name");
    let description = super::get_optional_string_param(&body, "description");
    let num_partitions = super::get_optional_i32_param(&body, "num_partitions");
    let replication_factor = super::get_optional_i32_param(&body, "replication_factor");
    let config = if body.get("config").is_some() { Some(super::get_hashmap_param(&body, "config")) } else { None };
    let req = UpdateTopicTemplateRequest { name, description, num_partitions, replication_factor, config };
    let store = TopicTemplateStore::new(state.db.inner().clone());
    store.update(id, &req).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_template_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let store = TopicTemplateStore::new(state.db.inner().clone());
    let deleted = store.delete(id).await?;
    if !deleted { return Err(AppError::NotFound(format!("Template {} not found", id))); }
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_template_presets() -> Result<Value> {
    use crate::db::topic_template::preset_templates::get_preset_templates;
    let presets = get_preset_templates();
    let preset_list: Vec<Value> = presets.into_iter().map(|p| {
        serde_json::json!({
            "name": p.name, "description": p.description,
            "num_partitions": p.num_partitions, "replication_factor": p.replication_factor,
            "config": p.config,
        })
    }).collect();
    Ok(serde_json::json!({ "presets": preset_list }))
}

pub async fn handle_template_create_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic_name = super::get_string_param(&body, "topic_name")?;
    let template_id = super::get_optional_i64_param(&body, "template_id");
    let template_name = super::get_optional_string_param(&body, "template_name");
    let override_config = if body.get("override_config").is_some() {
        Some(super::get_hashmap_param(&body, "override_config"))
    } else { None };

    let clients = state.get_clients();
    let admin = clients.get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let store = TopicTemplateStore::new(state.db.inner().clone());
    let template = if let Some(id) = template_id {
        store.get(id).await?
    } else if let Some(name) = &template_name {
        store.get_by_name(&name).await?
    } else {
        store.get_by_name("default").await?
    }.ok_or_else(|| AppError::NotFound("Template not found".to_string()))?;

    let mut final_config: HashMap<String, String> = serde_json::from_str(&template.config_json).unwrap_or_default();
    if let Some(oc) = override_config {
        for (k, v) in oc { final_config.insert(k, v); }
    }

    admin.create_topic(&topic_name, template.num_partitions, template.replication_factor, final_config).await?;
    Ok(serde_json::json!({
        "success": true, "topic": topic_name, "template": template.name,
        "num_partitions": template.num_partitions, "replication_factor": template.replication_factor,
    }))
}
