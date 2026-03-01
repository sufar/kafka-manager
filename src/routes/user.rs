/// 用户和角色管理路由

use crate::db::user::{UserStore, RoleStore, CreateUserRequest, CreateRoleRequest, UserResponse, RoleResponse};
use crate::error::{AppError, Result};
use crate::AppState;
use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/:id", get(get_user).put(update_user))
        .route("/:id/password", put(update_password))
}

pub fn role_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/:id", get(get_role).put(update_role))
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub role_id: Option<i64>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

// 用户管理

async fn list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserResponse>>> {
    let users = UserStore::list_with_roles(state.db.inner()).await?;
    Ok(Json(users))
}

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>> {
    // 密码哈希
    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;

    let id = UserStore::create(
        state.db.inner(),
        &req.username,
        &password_hash,
        req.email.as_deref(),
        req.role_id,
    ).await?;

    Ok(Json(serde_json::json!({
        "id": id,
        "username": req.username
    })))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<UserResponse>> {
    let user = UserStore::get_with_role(state.db.inner(), id).await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

    Ok(Json(user))
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>> {
    UserStore::update(
        state.db.inner(),
        id,
        req.email.as_deref(),
        req.role_id,
        req.is_active,
    ).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn update_password(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePasswordRequest>,
) -> Result<Json<serde_json::Value>> {
    // 验证旧密码
    let user = UserStore::get_by_id(state.db.inner(), id).await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

    let valid = bcrypt::verify(&req.old_password, &user.password_hash)
        .map_err(|e| AppError::Internal(format!("Failed to verify password: {}", e)))?;

    if !valid {
        return Err(AppError::Unauthorized("Invalid old password".to_string()));
    }

    // 更新密码
    let new_hash = bcrypt::hash(&req.new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;

    UserStore::update_password(state.db.inner(), id, &new_hash).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// 角色管理

async fn list_roles(
    State(state): State<AppState>,
) -> Result<Json<Vec<RoleResponse>>> {
    let roles = RoleStore::list(state.db.inner()).await?;

    let responses: Vec<RoleResponse> = roles.into_iter().map(|r| {
        let permissions: Vec<String> = serde_json::from_str(&r.permissions).unwrap_or_default();
        RoleResponse {
            id: r.id,
            name: r.name,
            description: r.description,
            permissions,
            created_at: r.created_at,
        }
    }).collect();

    Ok(Json(responses))
}

async fn create_role(
    State(state): State<AppState>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<serde_json::Value>> {
    let id = RoleStore::create(
        state.db.inner(),
        &req.name,
        req.description.as_deref(),
        &req.permissions,
    ).await?;

    Ok(Json(serde_json::json!({
        "id": id,
        "name": req.name
    })))
}

async fn get_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<RoleResponse>> {
    let role = RoleStore::get_by_id(state.db.inner(), id).await?
        .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))?;

    let permissions: Vec<String> = serde_json::from_str(&role.permissions).unwrap_or_default();

    Ok(Json(RoleResponse {
        id: role.id,
        name: role.name,
        description: role.description,
        permissions,
        created_at: role.created_at,
    }))
}

async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<serde_json::Value>> {
    RoleStore::update(
        state.db.inner(),
        id,
        req.name.as_deref(),
        req.description.as_deref(),
        req.permissions.as_deref(),
    ).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
