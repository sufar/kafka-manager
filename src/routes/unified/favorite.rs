/// Favorite handlers for unified API
use crate::db::favorite::{
    create_favorite, create_group, delete_favorite, delete_favorite_by_topic,
    delete_group, get_all_favorites_with_groups, get_all_groups_with_count, get_favorite_by_id,
    get_group_by_id, is_topic_favorite, update_favorite, update_group,
    CreateFavoriteRequest, CreateGroupRequest, UpdateFavoriteRequest, UpdateGroupRequest,
};
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;

pub async fn handle_favorite_group_list(state: AppState) -> Result<Value> {
    let groups = get_all_groups_with_count(&state.db).await?;
    Ok(serde_json::json!(groups))
}

pub async fn handle_favorite_group_create(state: AppState, body: Value) -> Result<Value> {
    let name = super::get_string_param(&body, "name")?;
    let description = super::get_optional_string_param(&body, "description");
    let sort_order = super::get_optional_i64_param(&body, "sort_order").map(|v| v as i32);
    let req = CreateGroupRequest { name, description, sort_order };
    let group = create_group(&state.db, &req).await?;
    Ok(serde_json::json!(group))
}

pub async fn handle_favorite_group_get(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let group = get_group_by_id(&state.db, id).await?;
    match group {
        Some(g) => Ok(serde_json::json!(g)),
        None => Err(AppError::NotFound(format!("Group {} not found", id))),
    }
}

pub async fn handle_favorite_group_update(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let name = super::get_optional_string_param(&body, "name");
    let description = super::get_optional_string_param(&body, "description");
    let sort_order = super::get_optional_i64_param(&body, "sort_order").map(|v| v as i32);
    let req = UpdateGroupRequest { name, description, sort_order };
    let group = update_group(&state.db, id, &req).await?;
    match group {
        Some(g) => Ok(serde_json::json!(g)),
        None => Err(AppError::NotFound(format!("Group {} not found", id))),
    }
}

pub async fn handle_favorite_group_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let deleted = delete_group(&state.db, id).await?;
    if deleted { Ok(serde_json::json!({ "message": "Group deleted successfully" })) }
    else { Err(AppError::NotFound(format!("Group {} not found", id))) }
}

pub async fn handle_favorite_list(state: AppState) -> Result<Value> {
    let favorites = get_all_favorites_with_groups(&state.db).await?;
    Ok(serde_json::json!(favorites))
}

pub async fn handle_favorite_create(state: AppState, body: Value) -> Result<Value> {
    let group_id = super::get_i64_param(&body, "group_id")?;
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic_name = super::get_string_param(&body, "topic_name")?;
    let description = super::get_optional_string_param(&body, "description");
    let sort_order = super::get_optional_i64_param(&body, "sort_order").map(|v| v as i32);
    let req = CreateFavoriteRequest { group_id, cluster_id, topic_name, description, sort_order };
    let item = create_favorite(&state.db, &req).await?;
    Ok(serde_json::json!(item))
}

pub async fn handle_favorite_get(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let item = get_favorite_by_id(&state.db, id).await?;
    match item {
        Some(i) => Ok(serde_json::json!(i)),
        None => Err(AppError::NotFound(format!("Favorite {} not found", id))),
    }
}

pub async fn handle_favorite_update(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let group_id = super::get_optional_i64_param(&body, "group_id");
    let description = super::get_optional_string_param(&body, "description");
    let sort_order = super::get_optional_i64_param(&body, "sort_order").map(|v| v as i32);
    let req = UpdateFavoriteRequest { group_id, description, sort_order };
    let item = update_favorite(&state.db, id, &req).await?;
    match item {
        Some(i) => Ok(serde_json::json!(i)),
        None => Err(AppError::NotFound(format!("Favorite {} not found", id))),
    }
}

pub async fn handle_favorite_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let deleted = delete_favorite(&state.db, id).await?;
    if deleted { Ok(serde_json::json!({ "message": "Favorite deleted successfully" })) }
    else { Err(AppError::NotFound(format!("Favorite {} not found", id))) }
}

pub async fn handle_favorite_check(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic_name = super::get_string_param(&body, "topic_name")?;
    let is_fav = is_topic_favorite(&state.db, &cluster_id, &topic_name).await?;
    Ok(serde_json::json!({ "is_favorite": is_fav }))
}

pub async fn handle_favorite_delete_by_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic_name = super::get_string_param(&body, "topic_name")?;
    let deleted = delete_favorite_by_topic(&state.db, &cluster_id, &topic_name).await?;
    if deleted { Ok(serde_json::json!({ "message": "Favorite deleted successfully" })) }
    else { Err(AppError::NotFound("Favorite not found".to_string())) }
}
