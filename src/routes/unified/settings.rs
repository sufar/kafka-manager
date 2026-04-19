/// Settings handlers for unified API
use crate::db::settings::SettingStore;
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;
use std::sync::Arc;

pub async fn handle_settings_get(state: AppState, body: Value) -> Result<Value> {
    let keys = super::get_string_array_param(&body, "keys");

    let settings = if keys.is_empty() {
        let all: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM user_settings ORDER BY key")
            .fetch_all(state.db.inner()).await?;
        all.into_iter().map(|(k, v)| serde_json::json!({ "key": k, "value": v })).collect()
    } else {
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(value) = SettingStore::get(state.db.inner(), &key).await? {
                result.push(serde_json::json!({ "key": key, "value": value }));
            }
        }
        result
    };

    Ok(serde_json::json!({ "settings": settings }))
}

pub async fn handle_settings_update(state: AppState, body: Value) -> Result<Value> {
    let key = super::get_string_param(&body, "key")?;
    let value = body.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
    SettingStore::set(state.db.inner(), &key, &value).await?;
    Ok(serde_json::json!({ "key": key, "value": value }))
}

pub async fn handle_settings_export(state: AppState) -> Result<Value> {
    let state_arc = Arc::new(state);
    let result = crate::routes::settings::export_data(axum::extract::State(state_arc)).await?;
    Ok(serde_json::to_value(result.0).map_err(|e| AppError::Internal(format!("Failed to serialize export data: {}", e)))?)
}

pub async fn handle_settings_import(state: AppState, body: Value) -> Result<Value> {
    let req: crate::routes::settings::ImportDataRequest = serde_json::from_value(body)
        .map_err(|e| AppError::BadRequest(format!("Invalid import data: {}", e)))?;
    let state_arc = Arc::new(state);
    let result = crate::routes::settings::import_data(
        axum::extract::State(state_arc),
        axum::Json(req),
    ).await?;
    Ok(serde_json::to_value(result.0).map_err(|e| AppError::Internal(format!("Failed to serialize import result: {}", e)))?)
}
