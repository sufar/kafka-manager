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
#[allow(dead_code)]
pub struct TopicPartitionDetail {
    pub partition: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
}

/// Topic 详情响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    /// 搜索 Topic（按名称模糊匹配，返回最多 50 条）
    pub async fn search_topics(
        pool: &sqlx::SqlitePool,
        keyword: &str,
    ) -> Result<Vec<TopicMetadata>> {
        let start = std::time::Instant::now();
        tracing::info!("[TopicStore::search_topics] keyword: {}", keyword);

        // 使用通配符匹配，支持任意位置匹配
        let pattern = format!("%{}%", keyword);

        let topics: Vec<TopicMetadata> = sqlx::query_as::<_, TopicMetadata>(
            "SELECT id, cluster_id, topic_name, partition_count, replication_factor, config_json, fetched_at
             FROM topic_metadata
             WHERE topic_name LIKE ?1
             ORDER BY cluster_id, topic_name
             LIMIT 50",
        )
        .bind(&pattern)
        .fetch_all(pool)
        .await?;

        tracing::info!("[TopicStore::search_topics] found {} topics in {:?}", topics.len(), start.elapsed());

        Ok(topics)
    }

    /// 搜索 Topic（支持多集群过滤和分页）
    pub async fn search_topics_with_filter(
        pool: &sqlx::SqlitePool,
        keyword: &str,
        cluster_ids: &[String],
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<TopicMetadata>, u32)> {
        let start = std::time::Instant::now();
        tracing::info!(
            "[TopicStore::search_topics_with_filter] keyword: {}, clusters: {:?}, offset: {}, limit: {}",
            keyword, cluster_ids, offset, limit
        );

        // 使用通配符匹配，支持任意位置匹配
        let pattern = format!("%{}%", keyword);

        // 构建集群过滤条件
        let (cluster_filter, cluster_count) = if cluster_ids.is_empty() {
            // 空数组表示搜索所有集群
            ("".to_string(), 0)
        } else {
            // 构建 IN 子句
            let placeholders: Vec<String> = (0..cluster_ids.len()).map(|i| format!("?{}", i + 2)).collect();
            let filter = format!("AND cluster_id IN ({})", placeholders.join(", "));
            (filter, cluster_ids.len())
        };

        // 查询总数
        let count_query = if cluster_ids.is_empty() {
            "SELECT COUNT(*) FROM topic_metadata WHERE topic_name LIKE ?1".to_string()
        } else {
            format!(
                "SELECT COUNT(*) FROM topic_metadata WHERE topic_name LIKE ?1 AND cluster_id IN ({})",
                (2..2 + cluster_count).map(|i| format!("?{}", i)).collect::<Vec<_>>().join(", ")
            )
        };

        let mut count_query_builder = sqlx::query_scalar::<_, u32>(&count_query)
            .bind(&pattern);

        // 绑定集群 ID 参数
        for cluster_id in cluster_ids {
            count_query_builder = count_query_builder.bind(cluster_id);
        }

        let total: u32 = count_query_builder
            .fetch_one(pool)
            .await?;

        // 查询分页结果
        let data_query = if cluster_ids.is_empty() {
            "SELECT id, cluster_id, topic_name, partition_count, replication_factor, config_json, fetched_at
             FROM topic_metadata
             WHERE topic_name LIKE ?1
             ORDER BY cluster_id, topic_name
             LIMIT ?2 OFFSET ?3".to_string()
        } else {
            format!(
                "SELECT id, cluster_id, topic_name, partition_count, replication_factor, config_json, fetched_at
                 FROM topic_metadata
                 WHERE topic_name LIKE ?1 {}
                 ORDER BY cluster_id, topic_name
                 LIMIT ?{} OFFSET ?{}",
                cluster_filter,
                2 + cluster_count,
                3 + cluster_count
            )
        };

        let mut query = sqlx::query_as::<_, TopicMetadata>(&data_query)
            .bind(&pattern);

        // 绑定集群 ID 参数
        for cluster_id in cluster_ids {
            query = query.bind(cluster_id);
        }

        // 绑定 limit 和 offset
        query = query.bind(limit as i64).bind(offset as i64);

        let topics: Vec<TopicMetadata> = query.fetch_all(pool).await?;

        tracing::info!(
            "[TopicStore::search_topics_with_filter] found {} topics (total: {}) in {:?}",
            topics.len(),
            total,
            start.elapsed()
        );

        Ok((topics, total))
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
            let config = std::collections::HashMap::with_capacity(0);
            let _ = Self::upsert(pool, cluster_id, topic_name, 1, 1, &config).await;
        }

        Ok(SyncResult { added, removed })
    }

    /// 保存 Topic 列表到数据库（用于批量保存）
    #[allow(dead_code)]
    pub async fn save_topics(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topics: &[(String, i32, i32)], // (topic_name, partition_count, replication_factor)
    ) -> Result<()> {
        for (topic_name, partition_count, replication_factor) in topics {
            let config = std::collections::HashMap::with_capacity(0);
            let _ = Self::upsert(pool, cluster_id, topic_name, *partition_count, *replication_factor, &config).await;
        }
        Ok(())
    }

    /// 清理孤儿 Topic（所属集群已被删除的 Topic）
    pub async fn cleanup_orphan_topics(
        pool: &sqlx::SqlitePool,
        valid_cluster_ids: &[String],
    ) -> Result<Vec<(String, String)>> {
        let valid_clusters: std::collections::HashSet<_> = valid_cluster_ids.iter().cloned().collect();

        // 获取所有 Topic
        let all_topics = Self::list_all(pool).await?;

        // 找出孤儿 Topic
        let orphan_topics: Vec<(String, String)> = all_topics
            .into_iter()
            .filter(|t| !valid_clusters.contains(&t.cluster_id))
            .map(|t| (t.cluster_id.clone(), t.topic_name.clone()))
            .collect();

        // 删除孤儿 Topic
        for (cluster_id, topic_name) in &orphan_topics {
            Self::delete(pool, cluster_id, topic_name).await?;
        }

        Ok(orphan_topics)
    }
}

/// 同步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}
