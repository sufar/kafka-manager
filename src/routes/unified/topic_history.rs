/// Topic History handlers for unified API
use crate::db::sent_message::{clear_sent_message_history, delete_sent_message, get_sent_message_list, record_sent_message};
use crate::db::topic_history::{clear_history, delete_history, delete_history_by_topic, get_history_list, record_history};
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;

pub async fn handle_topic_history_list(state: AppState, body: Value) -> Result<Value> {
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
    let offset = body.get("offset").and_then(|v| v.as_i64());
    let history = get_history_list(&state.db, Some(limit), offset).await?;
    Ok(serde_json::json!({ "history": history }))
}

pub async fn handle_topic_history_record(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic_name = super::get_string_param(&body, "topic_name")?;
    record_history(&state.db, &cluster_id, &topic_name).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_topic_history_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let deleted = delete_history(&state.db, id).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "History deleted successfully" }))
    } else {
        Err(AppError::NotFound("History not found".to_string()))
    }
}

pub async fn handle_topic_history_delete_by_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic_name = super::get_string_param(&body, "topic_name")?;
    let deleted = delete_history_by_topic(&state.db, &cluster_id, &topic_name).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "Topic history deleted successfully" }))
    } else {
        Err(AppError::NotFound("Topic history not found".to_string()))
    }
}

pub async fn handle_topic_history_clear(state: AppState) -> Result<Value> {
    clear_history(&state.db).await?;
    Ok(serde_json::json!({ "success": true }))
}

// Sent Message History handlers
pub async fn handle_sent_message_list(state: AppState, body: Value) -> Result<Value> {
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(String::from);
    let topic_name = body.get("topic_name").and_then(|v| v.as_str()).map(String::from);
    let offset = body.get("offset").and_then(|v| v.as_i64());
    let history = get_sent_message_list(&state.db, cluster_id.as_deref(), topic_name.as_deref(), Some(limit), offset).await?;
    Ok(serde_json::json!({ "sent_messages": history }))
}

pub async fn handle_sent_message_record(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic_name = super::get_string_param(&body, "topic_name")?;
    let partition = super::get_optional_i32_param(&body, "partition").unwrap_or(0);
    let key = super::get_optional_string_param(&body, "key");
    let value = super::get_optional_string_param(&body, "value").unwrap_or_default();
    let headers: Option<String> = body.get("headers").and_then(|v| v.as_str()).map(String::from);
    let offset = super::get_optional_i64_param(&body, "offset");
    record_sent_message(&state.db, &cluster_id, &topic_name, partition, key.as_deref(), &value, headers.as_deref(), offset).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_sent_message_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let deleted = delete_sent_message(&state.db, id).await?;
    if deleted {
        Ok(serde_json::json!({ "message": "Sent message deleted successfully" }))
    } else {
        Err(AppError::NotFound("Sent message not found".to_string()))
    }
}

pub async fn handle_sent_message_clear(state: AppState) -> Result<Value> {
    clear_sent_message_history(&state.db).await?;
    Ok(serde_json::json!({ "success": true }))
}
