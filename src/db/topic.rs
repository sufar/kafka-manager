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

    /// 获取多个集群的 Topic（数据库级别排序和分页，避免加载全部到内存）
    pub async fn list_by_clusters_with_pagination(
        pool: &sqlx::SqlitePool,
        cluster_ids: &[String],
        offset: u32,
        limit: u32,
    ) -> Result<Vec<(String, String)>> {
        if cluster_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = (0..cluster_ids.len()).map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT topic_name, cluster_id FROM topic_metadata WHERE cluster_id IN ({}) ORDER BY cluster_id, topic_name LIMIT ? OFFSET ?",
            placeholders.join(", ")
        );

        let mut query = sqlx::query_as::<_, (String, String)>(&sql);
        for id in cluster_ids {
            query = query.bind(id);
        }
        query = query.bind(limit as i64).bind(offset as i64);

        let rows = query.fetch_all(pool).await?;
        Ok(rows)
    }

    /// 获取多个集群的 Topic 总数
    pub async fn count_by_clusters(
        pool: &sqlx::SqlitePool,
        cluster_ids: &[String],
    ) -> Result<u64> {
        if cluster_ids.is_empty() {
            return Ok(0);
        }

        let placeholders: Vec<String> = (0..cluster_ids.len()).map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT COUNT(*) FROM topic_metadata WHERE cluster_id IN ({})",
            placeholders.join(", ")
        );

        let mut query = sqlx::query_scalar::<_, i64>(&sql);
        for id in cluster_ids {
            query = query.bind(id);
        }

        let count = query.fetch_one(pool).await?;
        Ok(count as u64)
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

    /// 返回新增和删除的 topic 名称列表（批量 SQL 优化）
    pub async fn sync_topics(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        current_topics: &[String],
    ) -> Result<SyncResult> {
        // 获取数据库中已存在的 Topic（只查名称，减少数据传输）
        let existing_names: std::collections::HashSet<String> = sqlx::query_scalar::<_, String>(
            "SELECT topic_name FROM topic_metadata WHERE cluster_id = ?",
        )
        .bind(cluster_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();

        let current_names: std::collections::HashSet<_> = current_topics.iter().cloned().collect();

        // 新增的 Topic
        let added: Vec<String> = current_topics
            .iter()
            .filter(|t| !existing_names.contains(*t))
            .cloned()
            .collect();

        // 已删除的 Topic
        let removed: Vec<String> = existing_names
            .into_iter()
            .filter(|t| !current_names.contains(t))
            .collect();

        // 批量删除不存在的 Topic
        if !removed.is_empty() {
            Self::batch_delete(pool, cluster_id, &removed).await?;
        }

        // 批量保存新增的 Topic
        if !added.is_empty() {
            Self::batch_upsert_names(pool, cluster_id, &added).await?;
        }

        Ok(SyncResult { added, removed })
    }

    /// 批量 upsert topic 名称（仅名称 + 默认值，1 条 SQL）
    /// SQLite 限制每条查询最多 32766 个参数，6 个参数/行 → 最多 5000 行/批
    pub async fn batch_upsert_names(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic_names: &[String],
    ) -> Result<()> {
        if topic_names.is_empty() {
            return Ok(());
        }

        const MAX_BATCH: usize = 5000;
        let now = chrono::Utc::now().to_rfc3339();
        let empty_config = "{}";

        for chunk in topic_names.chunks(MAX_BATCH) {
            let placeholders: Vec<String> = chunk.iter()
                .map(|_| "(?, ?, ?, ?, ?, ?)".to_string())
                .collect();

            let sql = format!(
                r#"
                INSERT INTO topic_metadata (cluster_id, topic_name, partition_count, replication_factor, config_json, fetched_at)
                VALUES {}
                ON CONFLICT(cluster_id, topic_name) DO UPDATE SET
                    partition_count = excluded.partition_count,
                    replication_factor = excluded.replication_factor,
                    config_json = excluded.config_json,
                    fetched_at = excluded.fetched_at
                "#,
                placeholders.join(", ")
            );

            let mut query = sqlx::query(&sql);
            for name in chunk {
                query = query
                    .bind(cluster_id)
                    .bind(name)
                    .bind(1)
                    .bind(1)
                    .bind(empty_config)
                    .bind(&now);
            }

            query.execute(pool).await?;
        }
        Ok(())
    }

    /// 批量 upsert topic 详情（名称 + 分区数，1 条 SQL）
    /// SQLite 限制每条查询最多 32766 个参数，6 个参数/行 → 最多 5000 行/批
    pub async fn batch_upsert_details(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic_details: &[(String, i32)],
    ) -> Result<()> {
        if topic_details.is_empty() {
            return Ok(());
        }

        const MAX_BATCH: usize = 5000;
        let now = chrono::Utc::now().to_rfc3339();
        let empty_config = "{}";

        for chunk in topic_details.chunks(MAX_BATCH) {
            let placeholders: Vec<String> = chunk.iter()
                .map(|_| "(?, ?, ?, ?, ?, ?)".to_string())
                .collect();

            let sql = format!(
                r#"
                INSERT INTO topic_metadata (cluster_id, topic_name, partition_count, replication_factor, config_json, fetched_at)
                VALUES {}
                ON CONFLICT(cluster_id, topic_name) DO UPDATE SET
                    partition_count = excluded.partition_count,
                    replication_factor = excluded.replication_factor,
                    config_json = excluded.config_json,
                    fetched_at = excluded.fetched_at
                "#,
                placeholders.join(", ")
            );

            let mut query = sqlx::query(&sql);
            for (name, partition_count) in chunk {
                query = query
                    .bind(cluster_id)
                    .bind(name)
                    .bind(partition_count)
                    .bind(1)
                    .bind(empty_config)
                    .bind(&now);
            }

            query.execute(pool).await?;
        }
        Ok(())
    }

    /// 批量删除 topic（1 条 SQL）
    /// SQLite 限制每条查询最多 32766 个参数，1 + N 参数 → 最多 32000 行/批
    pub async fn batch_delete(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic_names: &[String],
    ) -> Result<()> {
        if topic_names.is_empty() {
            return Ok(());
        }

        const MAX_BATCH: usize = 30_000;
        for chunk in topic_names.chunks(MAX_BATCH) {
            let placeholders: String = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "DELETE FROM topic_metadata WHERE cluster_id = ? AND topic_name IN ({})",
                placeholders
            );

            let mut query = sqlx::query(&sql).bind(cluster_id);
            for name in chunk {
                query = query.bind(name);
            }

            query.execute(pool).await?;
        }
        Ok(())
    }

    /// 按集群统计 topic 数量（用 COUNT 替代全量查询）
    pub async fn count_by_cluster(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
    ) -> Result<u32> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM topic_metadata WHERE cluster_id = ?",
        )
        .bind(cluster_id)
        .fetch_one(pool)
        .await?;

        Ok(count.0 as u32)
    }

    /// 保存 Topic 列表到数据库（用于批量保存）— 已废弃，使用 batch_upsert_names
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
