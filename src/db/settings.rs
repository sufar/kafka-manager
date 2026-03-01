/// 全局用户设置存储模块

use sqlx::SqlitePool;
use chrono::Utc;

/// 设置存储结构
pub struct SettingStore;

impl SettingStore {
    /// 获取设置值
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM user_settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| r.0))
    }

    /// 设置值
    pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO user_settings (key, value, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 获取整数设置值
    pub async fn get_int(pool: &SqlitePool, key: &str, default: i64) -> Result<i64, sqlx::Error> {
        match Self::get(pool, key).await? {
            Some(val) => Ok(val.parse().unwrap_or(default)),
            None => Ok(default),
        }
    }

    /// 设置整数设置值
    pub async fn set_int(pool: &SqlitePool, key: &str, value: i64) -> Result<(), sqlx::Error> {
        Self::set(pool, key, &value.to_string()).await
    }
}
