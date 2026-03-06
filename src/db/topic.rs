/// Topic 元数据存储模块

use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Topic 元数据记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TopicMetadata {
    pub id: i64,
    pub cluster_id: String,
    pub topic_name: String,
    pub partition_count: i32,
    pub replication_factor: i32,
    pub config_json: String,
    pub fetched_at: String,
}

/// Topic 分区详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicPartitionDetail {
    pub partition: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
}

/// Topic 详情响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicDetail {
    pub topic_name: String,
    pub partition_count: i32,
    pub replication_factor: i32,
    pub partitions: Vec<TopicPartitionDetail>,
    pub config: std::collections::HashMap<String, String>,
    pub fetched_at: String,
}

/// Topic 存储
pub struct TopicStore;

impl TopicStore {
    /// 保存或更新 Topic 元数据
    pub async fn upsert(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic_name: &str,
        partition_count: i32,
        replication_factor: i32,
        config: &std::collections::HashMap<String, String>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let config_json = serde_json::to_string(config).unwrap_or_else(|_| "{}".to_string());

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO topic_metadata (cluster_id, topic_name, partition_count, replication_factor, config_json, fetched_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(cluster_id, topic_name) DO UPDATE SET
                partition_count = excluded.partition_count,
                replication_factor = excluded.replication_factor,
                config_json = excluded.config_json,
                fetched_at = excluded.fetched_at
            RETURNING id
            "#,
        )
        .bind(cluster_id)
        .bind(topic_name)
        .bind(partition_count)
        .bind(replication_factor)
        .bind(&config_json)
        .bind(&now)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    /// 获取集群的所有 Topic 元数据
    pub async fn list_by_cluster(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
    ) -> Result<Vec<TopicMetadata>> {
        let topics = sqlx::query_as(
            "SELECT * FROM topic_metadata WHERE cluster_id = ? ORDER BY topic_name",
        )
        .bind(cluster_id)
        .fetch_all(pool)
        .await?;

        Ok(topics)
    }

    /// 获取指定 Topic 元数据
    pub async fn get_by_name(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic_name: &str,
    ) -> Result<Option<TopicMetadata>> {
        let topic = sqlx::query_as(
            "SELECT * FROM topic_metadata WHERE cluster_id = ? AND topic_name = ?",
        )
        .bind(cluster_id)
        .bind(topic_name)
        .fetch_optional(pool)
        .await?;

        Ok(topic)
    }

    /// 删除 Topic 元数据
    pub async fn delete(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic_name: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM topic_metadata WHERE cluster_id = ? AND topic_name = ?",
        )
        .bind(cluster_id)
        .bind(topic_name)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取所有 Topic 元数据（用于搜索）
    pub async fn list_all(
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<TopicMetadata>> {
        let topics = sqlx::query_as(
            "SELECT * FROM topic_metadata ORDER BY cluster_id, topic_name",
        )
        .fetch_all(pool)
        .await?;

        Ok(topics)
    }

    /// 获取所有 Topic 元数据（限制数量，用于搜索）
    pub async fn list_all_limit(
        pool: &sqlx::SqlitePool,
        limit: u32,
    ) -> Result<Vec<TopicMetadata>> {
        let topics = sqlx::query_as(
            "SELECT * FROM topic_metadata ORDER BY cluster_id, topic_name LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(topics)
    }

    /// 搜索 Topic（按名称模糊匹配，返回最多 10 条）
    pub async fn search_topics(
        pool: &sqlx::SqlitePool,
        keyword: &str,
    ) -> Result<Vec<TopicMetadata>> {
        let start = std::time::Instant::now();
        tracing::info!("[TopicStore::search_topics] keyword: {}", keyword);

        // 使用前缀匹配 + 后缀通配符，让 SQLite 能使用索引（如果有的话）
        // 对于小数据量，SQLite 的 LIKE 应该很快
        let pattern = format!("%{}%", keyword);

        // 使用 query_as 的 fetch_all，避免不必要的分配
        let topics: Vec<TopicMetadata> = sqlx::query_as::<_, TopicMetadata>(
            "SELECT id, cluster_id, topic_name, partition_count, replication_factor, config_json, fetched_at
             FROM topic_metadata
             WHERE topic_name LIKE ?1
             ORDER BY cluster_id, topic_name
             LIMIT 10",
        )
        .bind(&pattern)
        .fetch_all(pool)
        .await?;

        tracing::info!("[TopicStore::search_topics] found {} topics in {:?}", topics.len(), start.elapsed());

        Ok(topics)
    }

    /// 返回新增和删除的 topic 名称列表
    pub async fn sync_topics(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        current_topics: &[String],
    ) -> Result<SyncResult> {
        // 获取数据库中已存在的 Topic
        let existing_topics = Self::list_by_cluster(pool, cluster_id).await?;
        let existing_names: std::collections::HashSet<_> = existing_topics
            .iter()
            .map(|t| t.topic_name.as_str())
            .collect();

        let current_names: std::collections::HashSet<_> = current_topics
            .iter()
            .map(|s| s.as_str())
            .collect();

        // 新增的 Topic
        let added: Vec<String> = current_topics
            .iter()
            .filter(|t| !existing_names.contains(t.as_str()))
            .cloned()
            .collect();

        // 已删除的 Topic
        let removed: Vec<String> = existing_topics
            .iter()
            .filter(|t| !current_names.contains(t.topic_name.as_str()))
            .map(|t| t.topic_name.clone())
            .collect();

        // 删除已不存在的 Topic 记录
        for topic_name in &removed {
            Self::delete(pool, cluster_id, topic_name).await?;
        }

        // 保存新增的 Topic 记录（仅保存名称，其他字段在后续获取）
        for topic_name in &added {
            let config = std::collections::HashMap::new();
            let _ = Self::upsert(pool, cluster_id, topic_name, 1, 1, &config).await;
        }

        Ok(SyncResult { added, removed })
    }

    /// 保存 Topic 列表到数据库（用于批量保存）
    pub async fn save_topics(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topics: &[(String, i32, i32)], // (topic_name, partition_count, replication_factor)
    ) -> Result<()> {
        for (topic_name, partition_count, replication_factor) in topics {
            let config = std::collections::HashMap::new();
            let _ = Self::upsert(pool, cluster_id, topic_name, *partition_count, *replication_factor, &config).await;
        }
        Ok(())
    }
}

/// 同步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}
