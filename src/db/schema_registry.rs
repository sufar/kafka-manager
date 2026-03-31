/// Schema Registry 配置和 Schema 元数据存储模块

use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Schema Registry 配置记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SchemaRegistryConfig {
    pub id: i64,
    pub cluster_id: String,
    pub registry_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Schema 元数据记录（缓存）
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CachedSchema {
    pub id: i64,
    pub cluster_id: String,
    pub subject: String,
    pub version: i32,
    pub schema_type: String,
    pub schema_json: String,
    pub compatibility_level: Option<String>,
    pub fetched_at: String,
}

/// Schema 摘要信息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SchemaSummary {
    pub subject: String,
    pub latest_version: i32,
    pub schema_type: String,
    pub compatibility_level: Option<String>,
    pub version_count: i32,
}

/// Schema Registry 配置存储
pub struct SchemaRegistryStore;

impl SchemaRegistryStore {
    /// 获取 Schema Registry 配置
    pub async fn get_config(pool: &sqlx::SqlitePool, cluster_id: &str) -> Result<Option<SchemaRegistryConfig>> {
        let config = sqlx::query_as(
            "SELECT * FROM schema_registry_configs WHERE cluster_id = ?",
        )
        .bind(cluster_id)
        .fetch_optional(pool)
        .await?;

        Ok(config)
    }

    /// 保存或更新 Schema Registry 配置
    pub async fn save_config(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        registry_url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        // 加密密码（如果提供）
        let encrypted_password = if let Some(pwd) = password {
            if !pwd.is_empty() {
                Some(bcrypt::hash(pwd, bcrypt::DEFAULT_COST)?)
            } else {
                None
            }
        } else {
            None
        };

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO schema_registry_configs (cluster_id, registry_url, username, password, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(cluster_id) DO UPDATE SET
                registry_url = excluded.registry_url,
                username = excluded.username,
                password = excluded.password,
                updated_at = excluded.updated_at
            RETURNING id
            "#,
        )
        .bind(cluster_id)
        .bind(registry_url)
        .bind(username)
        .bind(encrypted_password.as_deref())
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    /// 删除 Schema Registry 配置
    pub async fn delete_config(pool: &sqlx::SqlitePool, cluster_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM schema_registry_configs WHERE cluster_id = ?")
            .bind(cluster_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

/// Schema 存储
pub struct SchemaStore;

impl SchemaStore {
    /// 缓存 Schema 信息
    pub async fn cache_schema(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        subject: &str,
        version: i32,
        schema_type: &str,
        schema_json: &str,
        compatibility_level: Option<&str>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO schemas (cluster_id, subject, version, schema_type, schema_json, compatibility_level, fetched_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(cluster_id, subject, version) DO UPDATE SET
                schema_type = excluded.schema_type,
                schema_json = excluded.schema_json,
                compatibility_level = excluded.compatibility_level,
                fetched_at = excluded.fetched_at
            RETURNING id
            "#,
        )
        .bind(cluster_id)
        .bind(subject)
        .bind(version)
        .bind(schema_type)
        .bind(schema_json)
        .bind(compatibility_level)
        .bind(&now)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    /// 获取缓存的 Schema
    pub async fn get_cached_schema(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        subject: &str,
        version: i32,
    ) -> Result<Option<CachedSchema>> {
        let schema = sqlx::query_as(
            "SELECT * FROM schemas WHERE cluster_id = ? AND subject = ? AND version = ?",
        )
        .bind(cluster_id)
        .bind(subject)
        .bind(version)
        .fetch_optional(pool)
        .await?;

        Ok(schema)
    }

    /// 获取指定 subject 的最新版本 Schema
    pub async fn get_latest_schema(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        subject: &str,
    ) -> Result<Option<CachedSchema>> {
        let schema = sqlx::query_as(
            r#"
            SELECT * FROM schemas
            WHERE cluster_id = ? AND subject = ?
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .bind(cluster_id)
        .bind(subject)
        .fetch_optional(pool)
        .await?;

        Ok(schema)
    }

    /// 列出集群的所有 Schema 摘要
    pub async fn list_schemas(pool: &sqlx::SqlitePool, cluster_id: &str) -> Result<Vec<SchemaSummary>> {
        let summaries = sqlx::query_as(
            r#"
            SELECT
                subject,
                MAX(version) as latest_version,
                schema_type,
                compatibility_level,
                COUNT(*) as version_count
            FROM schemas
            WHERE cluster_id = ?
            GROUP BY cluster_id, subject
            ORDER BY subject
            "#,
        )
        .bind(cluster_id)
        .fetch_all(pool)
        .await?;

        Ok(summaries)
    }

    /// 获取 subject 的所有版本
    pub async fn list_versions(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        subject: &str,
    ) -> Result<Vec<i32>> {
        let versions = sqlx::query_scalar(
            "SELECT version FROM schemas WHERE cluster_id = ? AND subject = ? ORDER BY version",
        )
        .bind(cluster_id)
        .bind(subject)
        .fetch_all(pool)
        .await?;

        Ok(versions)
    }

    /// 删除指定 subject 的所有 Schema 缓存
    pub async fn delete_subject_schemas(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        subject: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM schemas WHERE cluster_id = ? AND subject = ?")
            .bind(cluster_id)
            .bind(subject)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 清除集群的所有 Schema 缓存
    pub async fn clear_cluster_schemas(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM schemas WHERE cluster_id = ?")
            .bind(cluster_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

/// 初始化数据库表
pub async fn init_tables(pool: &sqlx::SqlitePool) -> sqlx::Result<()> {
    // 创建 Schema Registry 配置表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_registry_configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cluster_id TEXT NOT NULL,
            registry_url TEXT NOT NULL,
            username TEXT,
            password TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(cluster_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_schema_registry_cluster ON schema_registry_configs(cluster_id)")
        .execute(pool)
        .await?;

    // 创建 Schema 缓存表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schemas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cluster_id TEXT NOT NULL,
            subject TEXT NOT NULL,
            version INTEGER NOT NULL,
            schema_type TEXT NOT NULL,
            schema_json TEXT NOT NULL,
            compatibility_level TEXT,
            fetched_at TEXT NOT NULL,
            UNIQUE(cluster_id, subject, version)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_schemas_cluster ON schemas(cluster_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_schemas_subject ON schemas(subject)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_schemas_cluster_subject ON schemas(cluster_id, subject)")
        .execute(pool)
        .await?;

    Ok(())
}
