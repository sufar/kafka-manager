use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::db::DbPool;

/// Topic 浏览历史记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TopicHistory {
    pub id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub viewed_at: String,
}

/// 记录浏览历史请求
#[derive(Debug, Deserialize)]
pub struct RecordHistoryRequest {
    pub cluster_id: String,
    pub topic_name: String,
}

/// 初始化 Topic 历史表
pub async fn init_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS topic_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cluster_id TEXT NOT NULL,
            topic_name TEXT NOT NULL,
            viewed_at TEXT NOT NULL,
            UNIQUE(cluster_id, topic_name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_topic_history_viewed_at ON topic_history(viewed_at DESC)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_topic_history_cluster ON topic_history(cluster_id, viewed_at DESC)"
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// 记录浏览历史（如果存在则更新 viewed_at，否则插入）
pub async fn record_history(
    pool: &DbPool,
    cluster_id: &str,
    topic_name: &str,
) -> Result<TopicHistory, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    let history = sqlx::query_as::<_, TopicHistory>(
        r#"
        INSERT INTO topic_history (cluster_id, topic_name, viewed_at)
        VALUES (?, ?, ?)
        ON CONFLICT(cluster_id, topic_name) DO UPDATE SET viewed_at = ?
        RETURNING *
        "#,
    )
    .bind(cluster_id)
    .bind(topic_name)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool.inner())
    .await?;

    Ok(history)
}

/// 获取浏览历史列表（按时间倒序）
pub async fn get_history_list(
    pool: &DbPool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<TopicHistory>, sqlx::Error> {
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    let histories = sqlx::query_as::<_, TopicHistory>(
        r#"
        SELECT * FROM topic_history
        ORDER BY viewed_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool.inner())
    .await?;

    Ok(histories)
}

/// 获取单个历史记录
pub async fn get_history_by_id(pool: &DbPool, id: i64) -> Result<Option<TopicHistory>, sqlx::Error> {
    let history = sqlx::query_as::<_, TopicHistory>(
        "SELECT * FROM topic_history WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool.inner())
    .await?;

    Ok(history)
}

/// 删除单条历史记录
pub async fn delete_history(pool: &DbPool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM topic_history WHERE id = ?")
        .bind(id)
        .execute(pool.inner())
        .await?;

    Ok(result.rows_affected() > 0)
}

/// 删除指定 cluster 和 topic 的历史
pub async fn delete_history_by_topic(
    pool: &DbPool,
    cluster_id: &str,
    topic_name: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM topic_history WHERE cluster_id = ? AND topic_name = ?")
        .bind(cluster_id)
        .bind(topic_name)
        .execute(pool.inner())
        .await?;

    Ok(result.rows_affected() > 0)
}

/// 清空所有历史记录
pub async fn clear_history(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM topic_history")
        .execute(pool.inner())
        .await?;

    Ok(result.rows_affected() as i64)
}

/// 检查是否存在某条历史记录
pub async fn is_history_exists(
    pool: &DbPool,
    cluster_id: &str,
    topic_name: &str,
) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM topic_history WHERE cluster_id = ? AND topic_name = ?"
    )
    .bind(cluster_id)
    .bind(topic_name)
    .fetch_one(pool.inner())
    .await?;

    Ok(count > 0)
}

/// 插入或更新历史记录，保留原始 viewed_at 时间（用于导入）
pub async fn import_history(
    pool: &DbPool,
    cluster_id: &str,
    topic_name: &str,
    viewed_at: &str,
) -> Result<TopicHistory, sqlx::Error> {
    let history = sqlx::query_as::<_, TopicHistory>(
        r#"
        INSERT INTO topic_history (cluster_id, topic_name, viewed_at)
        VALUES (?, ?, ?)
        ON CONFLICT(cluster_id, topic_name) DO UPDATE SET viewed_at = ?
        RETURNING *
        "#,
    )
    .bind(cluster_id)
    .bind(topic_name)
    .bind(viewed_at)
    .bind(viewed_at)
    .fetch_one(pool.inner())
    .await?;

    Ok(history)
}
