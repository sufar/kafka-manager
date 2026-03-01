/// 全局用户设置路由

use crate::db::settings::SettingStore;
use crate::error::Result;
use crate::AppState;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_settings).put(update_setting))
}

#[derive(Debug, Deserialize)]
pub struct SettingsQuery {
    keys: Option<String>, // 逗号分隔的 key 列表
}

#[derive(Debug, Serialize)]
pub struct SettingValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub settings: Vec<SettingValue>,
}

/// 获取设置
async fn get_settings(
    State(state): State<AppState>,
    query: Query<SettingsQuery>,
) -> Result<Json<SettingsResponse>> {
    let keys: Vec<&str> = query
        .keys
        .as_ref()
        .map(|s| s.split(',').map(|k| k.trim()).collect())
        .unwrap_or_default();

    let settings = if keys.is_empty() {
        // 获取所有设置
        let all: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM user_settings ORDER BY key"
        )
        .fetch_all(state.db.inner())
        .await?;
        all.into_iter()
            .map(|(k, v)| SettingValue { key: k, value: v })
            .collect()
    } else {
        // 获取指定的设置
        let mut result = Vec::new();
        for key in keys {
            if let Some(value) = SettingStore::get(state.db.inner(), key).await? {
                result.push(SettingValue {
                    key: key.to_string(),
                    value,
                });
            }
        }
        result
    };

    Ok(Json(SettingsResponse { settings }))
}

/// 更新设置
#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub key: String,
    pub value: String,
}

async fn update_setting(
    State(state): State<AppState>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<SettingValue>> {
    SettingStore::set(state.db.inner(), &req.key, &req.value).await?;

    Ok(Json(SettingValue {
        key: req.key,
        value: req.value,
    }))
}
