/// Schema Registry API 路由处理器

use crate::db::schema_registry::{SchemaRegistryStore, SchemaStore, init_tables as init_schema_tables};
use crate::error::{AppError, Result};
use crate::kafka::schema_registry_client::SchemaRegistryClient;
use crate::models::schema_registry::{
    CompatibilityLevel, SchemaRegistryConfigInfo,
};
use crate::AppState;
use serde_json::Value;

/// 获取 Schema Registry 配置
pub async fn handle_config_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            // 不返回密码，只返回是否有密码
            let config_info = SchemaRegistryConfigInfo {
                id: cfg.id,
                cluster_id: cfg.cluster_id,
                registry_url: cfg.registry_url,
                username: cfg.username,
                has_password: cfg.password.is_some(),
                created_at: cfg.created_at,
                updated_at: cfg.updated_at,
            };
            Ok(serde_json::to_value(config_info)?)
        }
        None => Ok(Value::Null),
    }
}

/// 保存 Schema Registry 配置
pub async fn handle_config_save(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let registry_url = body
        .get("registry_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing registry_url parameter".to_string()))?;

    let username = body.get("username").and_then(|v| v.as_str()).map(|s| s.to_string());

    // 密码处理：如果是空字符串，保持原有密码
    let password = body.get("password")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let id = SchemaRegistryStore::save_config(
        state.db.inner(),
        &cluster_id,
        &registry_url,
        username.as_deref(),
        password.as_deref(),
    )
    .await?;

    // 初始化 Schema 表
    init_schema_tables(state.db.inner()).await?;

    Ok(serde_json::json!({ "id": id, "success": true }))
}

/// 删除 Schema Registry 配置
pub async fn handle_config_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    SchemaRegistryStore::delete_config(state.db.inner(), &cluster_id).await?;

    Ok(serde_json::json!({ "success": true }))
}

/// 测试 Schema Registry 连接
pub async fn handle_config_test(_state: AppState, body: Value) -> Result<Value> {
    let registry_url = body
        .get("registry_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing registry_url parameter".to_string()))?;

    let username = body.get("username").and_then(|v| v.as_str()).map(|s| s.to_string());
    let password = body.get("password").and_then(|v| v.as_str()).map(|s| s.to_string());

    let client = SchemaRegistryClient::new_with_global_proxy(&registry_url, username.as_deref(), password.as_deref())?;

    let connected = client.test_connection().await?;

    Ok(serde_json::json!({
        "success": connected,
        "message": if connected { "Connection successful" } else { "Connection failed" }
    }))
}

/// 获取所有 subjects
pub async fn handle_subject_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    // 获取 Schema Registry 配置
    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let subjects = client.get_subjects().await?;

            // 缓存到数据库
            for subject in &subjects {
                if let Ok(latest) = client.get_latest_schema(subject).await {
                    let _ = SchemaStore::cache_schema(
                        state.db.inner(),
                        &cluster_id,
                        &latest.subject,
                        latest.version,
                        &latest.schema_type.to_string(),
                        &latest.schema_json,
                        latest.compatibility_level.as_ref().map(|l| l.to_string()).as_deref(),
                    ).await;
                }
            }

            Ok(serde_json::json!({ "subjects": subjects }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 获取 subject 的所有版本
pub async fn handle_version_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let versions = client.get_versions(&subject).await?;
            Ok(serde_json::json!({ "versions": versions }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 获取指定版本的 schema
pub async fn handle_schema_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let version = body
        .get("version")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| AppError::BadRequest("Missing version parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let schema = client.get_schema(&subject, version).await?;

            // 获取兼容性级别
            let compatibility_level = client.get_compatibility_level(&subject).await.ok();

            // 缓存 schema
            let _ = SchemaStore::cache_schema(
                state.db.inner(),
                &cluster_id,
                &schema.subject,
                schema.version,
                &schema.schema_type.to_string(),
                &schema.schema_json,
                compatibility_level.as_ref().map(|l| l.to_string()).as_deref(),
            ).await;

            Ok(serde_json::json!({
                "subject": schema.subject,
                "version": schema.version,
                "schema_type": schema.schema_type.to_string(),
                "schema_json": schema.schema_json,
                "compatibility_level": compatibility_level.map(|l| l.to_string()),
            }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 获取最新版本的 schema
pub async fn handle_schema_get_latest(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let schema = client.get_latest_schema(&subject).await?;
            let compatibility_level = client.get_compatibility_level(&subject).await.ok();

            Ok(serde_json::json!({
                "subject": schema.subject,
                "version": schema.version,
                "schema_type": schema.schema_type.to_string(),
                "schema_json": schema.schema_json,
                "compatibility_level": compatibility_level.map(|l| l.to_string()),
            }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 注册新的 schema
pub async fn handle_schema_register(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let schema_json = body
        .get("schema_json")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing schema_json parameter".to_string()))?;

    let schema_type_str = body
        .get("schema_type")
        .and_then(|v| v.as_str())
        .unwrap_or("AVRO");

    let schema_type = crate::models::schema_registry::SchemaType::from_str(schema_type_str)
        .ok_or_else(|| AppError::BadRequest("Invalid schema_type parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let result = client.register_schema(&subject, &schema_json, schema_type.clone()).await?;

            // 缓存 schema
            let _ = SchemaStore::cache_schema(
                state.db.inner(),
                &cluster_id,
                &result.subject,
                result.version,
                &schema_type.to_string(),
                &schema_json,
                None,
            ).await;

            Ok(serde_json::json!({
                "id": result.id,
                "subject": result.subject,
                "version": result.version,
                "success": true,
            }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 测试 schema 兼容性
pub async fn handle_compatibility_test(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let schema_json = body
        .get("schema_json")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing schema_json parameter".to_string()))?;

    let version = body
        .get("version")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1) as i32;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let result = client.test_compatibility(&subject, &schema_json, version).await?;

            Ok(serde_json::json!({
                "compatible": result.compatible,
                "errors": result.errors,
                "messages": result.messages,
            }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 获取兼容性级别
pub async fn handle_compatibility_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let level = client.get_compatibility_level(&subject).await?;

            Ok(serde_json::json!({
                "compatibility_level": level.to_string(),
            }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 设置兼容性级别
pub async fn handle_compatibility_set(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let level_str = body
        .get("compatibility_level")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing compatibility_level parameter".to_string()))?;

    let level = CompatibilityLevel::from_str(level_str)
        .ok_or_else(|| AppError::BadRequest("Invalid compatibility_level parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            client.set_compatibility_level(&subject, level.clone()).await?;

            Ok(serde_json::json!({
                "success": true,
                "compatibility_level": level.to_string(),
            }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}

/// 获取 Schema 列表
pub async fn handle_schema_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    // 先从缓存获取
    let summaries = SchemaStore::list_schemas(state.db.inner(), &cluster_id).await?;

    let result: Vec<Value> = summaries
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "subject": s.subject,
                "latest_version": s.latest_version,
                "schema_type": s.schema_type,
                "compatibility_level": s.compatibility_level,
                "version_count": s.version_count,
            })
        })
        .collect();

    Ok(serde_json::json!({ "schemas": result }))
}

/// 删除 Schema subject
pub async fn handle_schema_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body
        .get("cluster_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing cluster_id parameter".to_string()))?;

    let subject = body
        .get("subject")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing subject parameter".to_string()))?;

    let config = SchemaRegistryStore::get_config(state.db.inner(), &cluster_id).await?;

    match config {
        Some(cfg) => {
            let client = SchemaRegistryClient::new(
                &cfg.registry_url,
                cfg.username.as_deref(),
                cfg.password.as_deref(),
            )?;

            let versions = client.delete_subject(&subject).await?;

            // 清除缓存
            let _ = SchemaStore::delete_subject_schemas(state.db.inner(), &cluster_id, &subject).await;

            Ok(serde_json::json!({
                "success": true,
                "deleted_versions": versions,
            }))
        }
        None => Err(AppError::NotFound("Schema Registry not configured for this cluster".to_string())),
    }
}
