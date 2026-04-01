use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::db::DbPool;

/// 发送消息历史记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SentMessage {
    pub id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub partition: i32,
    pub message_key: Option<String>,
    pub message_value: String,
    pub headers: Option<String>,
    pub offset: Option<i64>,
    pub sent_at: String,
}

/// 发送消息请求
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub cluster_id: String,
    pub topic_name: String,
    pub partition: i32,
    pub message_key: Option<String>,
    pub message_value: String,
    pub headers: Option<String>,
    pub offset: Option<i64>,
}

/// 初始化发送消息历史表
pub async fn init_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sent_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cluster_id TEXT NOT NULL,
            topic_name TEXT NOT NULL,
            partition INTEGER NOT NULL,
            message_key TEXT,
            message_value TEXT NOT NULL,
            headers TEXT,
            offset INTEGER,
            sent_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sent_messages_sent_at ON sent_messages(sent_at DESC)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sent_messages_cluster_topic ON sent_messages(cluster_id, topic_name, sent_at DESC)"
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// 记录发送消息历史
pub async fn record_sent_message(
    pool: &DbPool,
    cluster_id: &str,
    topic_name: &str,
    partition: i32,
    message_key: Option<&str>,
    message_value: &str,
    headers: Option<&str>,
    offset: Option<i64>,
) -> Result<SentMessage, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    let message = sqlx::query_as::<_, SentMessage>(
        r#"
        INSERT INTO sent_messages (cluster_id, topic_name, partition, message_key, message_value, headers, offset, sent_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(cluster_id)
    .bind(topic_name)
    .bind(partition)
    .bind(message_key)
    .bind(message_value)
    .bind(headers)
    .bind(offset)
    .bind(&now)
    .fetch_one(pool.inner())
    .await?;

    Ok(message)
}

/// 获取发送消息历史列表（按时间倒序）
pub async fn get_sent_message_list(
    pool: &DbPool,
    cluster_id: Option<&str>,
    topic_name: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<SentMessage>, sqlx::Error> {
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    let query = if cluster_id.is_some() && topic_name.is_some() {
        sqlx::query_as::<_, SentMessage>(
            r#"
            SELECT * FROM sent_messages
            WHERE cluster_id = ? AND topic_name = ?
            ORDER BY sent_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
    } else if cluster_id.is_some() {
        sqlx::query_as::<_, SentMessage>(
            r#"
            SELECT * FROM sent_messages
            WHERE cluster_id = ?
            ORDER BY sent_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
    } else if topic_name.is_some() {
        sqlx::query_as::<_, SentMessage>(
            r#"
            SELECT * FROM sent_messages
            WHERE topic_name = ?
            ORDER BY sent_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
    } else {
        sqlx::query_as::<_, SentMessage>(
            r#"
            SELECT * FROM sent_messages
            ORDER BY sent_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
    };

    let messages = if cluster_id.is_some() && topic_name.is_some() {
        query
            .bind(cluster_id.unwrap())
            .bind(topic_name.unwrap())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool.inner())
            .await?
    } else if cluster_id.is_some() {
        query
            .bind(cluster_id.unwrap())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool.inner())
            .await?
    } else if topic_name.is_some() {
        query
            .bind(topic_name.unwrap())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool.inner())
            .await?
    } else {
        query
            .bind(limit)
            .bind(offset)
            .fetch_all(pool.inner())
            .await?
    };

    Ok(messages)
}

/// 删除单条发送历史记录
pub async fn delete_sent_message(pool: &DbPool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sent_messages WHERE id = ?")
        .bind(id)
        .execute(pool.inner())
        .await?;

    Ok(result.rows_affected() > 0)
}

/// 清空所有发送历史记录
pub async fn clear_sent_message_history(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sent_messages")
        .execute(pool.inner())
        .await?;

    Ok(result.rows_affected() as i64)
}
