use crate::db::api_key::ApiKeyStore;
use crate::error::Result;
use crate::AppState;
use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_api_keys).post(create_api_key))
        .route("/:id", delete(delete_api_key))
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: i64,
    pub key_prefix: String,
    pub name: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyListResponse {
    pub keys: Vec<ApiKeyInfo>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: i64,
    pub key: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeleteApiKeyResponse {
    pub success: bool,
}

async fn list_api_keys(State(state): State<AppState>) -> Result<Json<ApiKeyListResponse>> {
    let keys = ApiKeyStore::list(state.db.inner()).await?;

    let key_infos: Vec<ApiKeyInfo> = keys
        .into_iter()
        .map(|k| ApiKeyInfo {
            id: k.id,
            key_prefix: k.key_prefix.clone(),
            name: k.name,
            created_at: k.created_at,
            expires_at: k.expires_at,
            is_active: k.is_active,
        })
        .collect();

    Ok(Json(ApiKeyListResponse { keys: key_infos }))
}

async fn create_api_key(
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>> {
    // 生成新的 API Key
    let key = ApiKeyStore::generate_key();
    let key_prefix = key[..7].to_string(); // 包含 "km_" 前缀
    let key_hash = ApiKeyStore::hash_key(&key);

    // 计算过期时间
    let expires_at = if let Some(days) = req.expires_in_days {
        let expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::days(days))
            .unwrap()
            .to_rfc3339();
        Some(expires)
    } else {
        None
    };

    // 存储到数据库
    let record = ApiKeyStore::create(
        state.db.inner(),
        &key_hash,
        &key_prefix,
        req.name.as_deref(),
        expires_at.as_deref(),
    )
    .await?;

    Ok(Json(CreateApiKeyResponse {
        id: record.id,
        key,
        key_prefix: record.key_prefix,
        name: record.name,
        created_at: record.created_at,
        expires_at: record.expires_at,
    }))
}

async fn delete_api_key(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeleteApiKeyResponse>> {
    let deleted = ApiKeyStore::delete(state.db.inner(), id).await?;

    Ok(Json(DeleteApiKeyResponse { success: deleted }))
}
