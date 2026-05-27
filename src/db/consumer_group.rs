/// Consumer Group 元数据存储模块

use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Consumer Group 元数据记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ConsumerGroupMetadata {
    pub id: i64,
    pub cluster_id: String,
    pub group_name: String,
    pub topics: String,  // JSON 数组
    pub fetched_at: String,
}

/// Consumer Group Offset 记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ConsumerGroupOffset {
    pub id: i64,
    pub cluster_id: String,
    pub group_name: String,
    pub topic_name: String,
    pub partition_id: i32,
    pub offset_value: i64,
    pub lag: i64,
    pub last_commit_time: Option<i64>,
    pub fetched_at: String,
}

/// Consumer Group 详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroupDetail {
    pub group_name: String,
    pub cluster_id: String,
    pub topics: Vec<String>,
    pub partitions: Vec<PartitionOffsetDetail>,
    pub state: String,  // Stable, Empty, Dead, PreparingRebalancing, CompletingRebalancing
}

/// 分区偏移详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionOffsetDetail {
    pub topic: String,
    pub partition: i32,
    pub start_offset: i64,   // earliest offset
    pub end_offset: i64,     // latest offset
    pub committed_offset: i64,
    pub lag: i64,
    pub last_commit_time: Option<i64>,  // 最后提交时间（毫秒时间戳）
}

/// Consumer Group 存储
pub struct ConsumerGroupStore;

impl ConsumerGroupStore {
    /// 保存或更新 Consumer Group 元数据
    pub async fn upsert(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
        topics: &[String],
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let topics_json = serde_json::to_string(topics).unwrap_or_else(|_| "[]".to_string());

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO consumer_group_metadata (cluster_id, group_name, topics, fetched_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(cluster_id, group_name) DO UPDATE SET
                topics = excluded.topics,
                fetched_at = excluded.fetched_at
            RETURNING id
            "#,
        )
        .bind(cluster_id)
        .bind(group_name)
        .bind(&topics_json)
        .bind(&now)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    /// 获取集群的所有 Consumer Group 元数据
    pub async fn list_by_cluster(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
    ) -> Result<Vec<ConsumerGroupMetadata>> {
        let groups = sqlx::query_as(
            "SELECT * FROM consumer_group_metadata WHERE cluster_id = ? ORDER BY group_name",
        )
        .bind(cluster_id)
        .fetch_all(pool)
        .await?;

        Ok(groups)
    }

    /// 获取指定 Consumer Group 元数据
    pub async fn get_by_name(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
    ) -> Result<Option<ConsumerGroupMetadata>> {
        let group = sqlx::query_as(
            "SELECT * FROM consumer_group_metadata WHERE cluster_id = ? AND group_name = ?",
        )
        .bind(cluster_id)
        .bind(group_name)
        .fetch_optional(pool)
        .await?;

        Ok(group)
    }

    /// 按 topic 查询 consumer groups（topics 字段为 JSON 数组，用 LIKE 匹配）
    pub async fn list_by_topic(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic: &str,
    ) -> Result<Vec<ConsumerGroupMetadata>> {
        // topics 格式: ["topic-a","topic-b"]
        // 使用 '%"topic_name"%' 精确匹配 JSON 数组中的元素
        let pattern = format!("%\"{}\"%", topic);
        tracing::info!("[ConsumerGroupStore::list_by_topic] cluster_id={}, topic={}, pattern={}", cluster_id, topic, pattern);

        let groups: Vec<ConsumerGroupMetadata> = sqlx::query_as(
            "SELECT * FROM consumer_group_metadata WHERE cluster_id = ? AND topics LIKE ?",
        )
        .bind(cluster_id)
        .bind(&pattern)
        .fetch_all(pool)
        .await?;

        tracing::info!("[ConsumerGroupStore::list_by_topic] found {} groups", groups.len());
        for g in &groups {
            tracing::info!("[ConsumerGroupStore::list_by_topic] group={}, topics={}", g.group_name, g.topics);
        }

        Ok(groups)
    }

    /// 删除 Consumer Group 元数据
    pub async fn delete(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM consumer_group_metadata WHERE cluster_id = ? AND group_name = ?",
        )
        .bind(cluster_id)
        .bind(group_name)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 搜索 Consumer Group（按名称模糊匹配，返回最多 50 条）
    pub async fn search_groups(
        pool: &sqlx::SqlitePool,
        keyword: &str,
    ) -> Result<Vec<ConsumerGroupMetadata>> {
        let start = std::time::Instant::now();
        tracing::info!("[ConsumerGroupStore::search_groups] keyword: {}", keyword);

        let pattern = format!("%{}%", keyword);

        let groups: Vec<ConsumerGroupMetadata> = sqlx::query_as::<_, ConsumerGroupMetadata>(
            "SELECT id, cluster_id, group_name, topics, fetched_at
             FROM consumer_group_metadata
             WHERE group_name LIKE ?1
             ORDER BY cluster_id, group_name
             LIMIT 50",
        )
        .bind(&pattern)
        .fetch_all(pool)
        .await?;

        tracing::info!("[ConsumerGroupStore::search_groups] found {} groups in {:?}", groups.len(), start.elapsed());

        Ok(groups)
    }

    /// 返回新增和删除的 group 名称列表（批量 SQL 优化）
    pub async fn sync_consumer_groups(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        current_groups: &[String],
    ) -> Result<SyncResult> {
        // 只查名称，减少数据传输
        let existing_names: std::collections::HashSet<String> = sqlx::query_scalar::<_, String>(
            "SELECT group_name FROM consumer_group_metadata WHERE cluster_id = ?",
        )
        .bind(cluster_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();

        let current_names: std::collections::HashSet<_> = current_groups.iter().cloned().collect();

        let added: Vec<String> = current_groups
            .iter()
            .filter(|g| !existing_names.contains(*g))
            .cloned()
            .collect();

        let removed: Vec<String> = existing_names
            .into_iter()
            .filter(|g| !current_names.contains(g))
            .collect();

        // 批量删除
        if !removed.is_empty() {
            Self::batch_delete_groups(pool, cluster_id, &removed).await?;
        }

        // 批量新增
        if !added.is_empty() {
            Self::batch_upsert_groups(pool, cluster_id, &added).await?;
        }

        Ok(SyncResult { added, removed })
    }

    /// 批量 upsert group 名称（1 条 SQL）
    /// SQLite 限制每条查询最多 32766 个参数，4 个参数/行 → 最多 8000 行/批
    pub async fn batch_upsert_groups(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_names: &[String],
    ) -> Result<()> {
        if group_names.is_empty() {
            return Ok(());
        }

        const MAX_BATCH: usize = 8000;
        let now = chrono::Utc::now().to_rfc3339();
        let empty_topics = "[]";

        for chunk in group_names.chunks(MAX_BATCH) {
            let placeholders: String = chunk.iter().map(|_| "(?, ?, ?, ?)").collect::<Vec<_>>().join(", ");

            let sql = format!(
                r#"
                INSERT INTO consumer_group_metadata (cluster_id, group_name, topics, fetched_at)
                VALUES {}
                ON CONFLICT(cluster_id, group_name) DO UPDATE SET
                    topics = excluded.topics,
                    fetched_at = excluded.fetched_at
                "#,
                placeholders
            );

            let mut query = sqlx::query(&sql);
            for name in chunk {
                query = query.bind(cluster_id).bind(name).bind(empty_topics).bind(&now);
            }

            query.execute(pool).await?;
        }
        Ok(())
    }

    /// 批量删除 group（1 条 SQL）
    /// SQLite 限制每条查询最多 32766 个参数，1 + N 参数 → 最多 32000 行/批
    pub async fn batch_delete_groups(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_names: &[String],
    ) -> Result<()> {
        if group_names.is_empty() {
            return Ok(());
        }

        const MAX_BATCH: usize = 30_000;
        for chunk in group_names.chunks(MAX_BATCH) {
            let placeholders: String = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "DELETE FROM consumer_group_metadata WHERE cluster_id = ? AND group_name IN ({})",
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

    // ==================== Consumer Group Offsets ====================

    /// 保存或更新 Consumer Group Offset
    pub async fn upsert_offset(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
        topic_name: &str,
        partition_id: i32,
        offset_value: i64,
        lag: i64,
        last_commit_time: Option<i64>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO consumer_group_offsets (cluster_id, group_name, topic_name, partition_id, offset_value, lag, last_commit_time, fetched_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(cluster_id, group_name, topic_name, partition_id) DO UPDATE SET
                offset_value = excluded.offset_value,
                lag = excluded.lag,
                last_commit_time = excluded.last_commit_time,
                fetched_at = excluded.fetched_at
            RETURNING id
            "#,
        )
        .bind(cluster_id)
        .bind(group_name)
        .bind(topic_name)
        .bind(partition_id)
        .bind(offset_value)
        .bind(lag)
        .bind(last_commit_time)
        .bind(&now)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    /// 获取 Consumer Group 的所有 Offset 记录
    pub async fn list_offsets_by_group(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
    ) -> Result<Vec<ConsumerGroupOffset>> {
        let offsets = sqlx::query_as(
            "SELECT * FROM consumer_group_offsets WHERE cluster_id = ? AND group_name = ? ORDER BY topic_name, partition_id",
        )
        .bind(cluster_id)
        .bind(group_name)
        .fetch_all(pool)
        .await?;

        Ok(offsets)
    }

    /// 删除 Consumer Group 的所有 Offset 记录
    pub async fn delete_offsets(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM consumer_group_offsets WHERE cluster_id = ? AND group_name = ?",
        )
        .bind(cluster_id)
        .bind(group_name)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 清理孤儿 Consumer Group（所属集群已被删除的 Group）
    pub async fn cleanup_orphan_groups(
        pool: &sqlx::SqlitePool,
        valid_cluster_ids: &[String],
    ) -> Result<Vec<(String, String)>> {
        let valid_clusters: std::collections::HashSet<_> = valid_cluster_ids.iter().cloned().collect();

        let all_groups = Self::list_all(pool).await?;

        let orphan_groups: Vec<(String, String)> = all_groups
            .into_iter()
            .filter(|g| !valid_clusters.contains(&g.cluster_id))
            .map(|g| (g.cluster_id.clone(), g.group_name.clone()))
            .collect();

        for (cluster_id, group_name) in &orphan_groups {
            Self::delete(pool, cluster_id, group_name).await?;
        }

        Ok(orphan_groups)
    }

    /// 获取所有 Consumer Group 元数据
    pub async fn list_all(
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<ConsumerGroupMetadata>> {
        let groups = sqlx::query_as(
            "SELECT * FROM consumer_group_metadata ORDER BY cluster_id, group_name",
        )
        .fetch_all(pool)
        .await?;

        Ok(groups)
    }

    // ==================== Consumer Group <-> Topic 多对多关系 ====================

    /// 插入或忽略 consumer group 与 topic 的关系（INSERT OR IGNORE）
    pub async fn upsert_topic_relation(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
        topic_name: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO consumer_group_topics (cluster_id, group_name, topic_name, fetched_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(cluster_id)
        .bind(group_name)
        .bind(topic_name)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 批量插入 group-topic 关系（4 个参数/行，最多 8000 行/批）
    pub async fn batch_upsert_topic_relations(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        relations: &[(String, String)], // (group_name, topic_name)
    ) -> Result<()> {
        if relations.is_empty() {
            return Ok(());
        }

        const MAX_BATCH: usize = 8000;
        let now = chrono::Utc::now().to_rfc3339();

        for chunk in relations.chunks(MAX_BATCH) {
            let placeholders: String = chunk.iter().map(|_| "(?, ?, ?, ?)").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "INSERT OR IGNORE INTO consumer_group_topics (cluster_id, group_name, topic_name, fetched_at) VALUES {}",
                placeholders
            );

            let mut query = sqlx::query(&sql);
            for (group, topic) in chunk {
                query = query.bind(cluster_id).bind(group).bind(topic).bind(&now);
            }

            query.execute(pool).await?;
        }
        Ok(())
    }

    /// 清理指定 groups 的旧 topic 关系
    /// SQLite 限制每条查询最多 32766 个参数，1 + N 参数 → 最多 32000 行/批
    pub async fn cleanup_topic_relations(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_names: &[&str],
    ) -> Result<()> {
        if group_names.is_empty() {
            return Ok(());
        }
        const MAX_BATCH: usize = 30_000;
        for chunk in group_names.chunks(MAX_BATCH) {
            let placeholders: Vec<&str> = chunk.iter().map(|_| "?").collect();
            let sql = format!(
                "DELETE FROM consumer_group_topics WHERE cluster_id = ? AND group_name IN ({})",
                placeholders.join(", ")
            );
            let mut query = sqlx::query(&sql).bind(cluster_id);
            for name in chunk {
                query = query.bind(name);
            }
            query.execute(pool).await?;
        }
        Ok(())
    }

    /// 清理某个集群下所有 group-topic 关系
    pub async fn cleanup_all_cluster_topic_relations(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM consumer_group_topics WHERE cluster_id = ?")
            .bind(cluster_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 清理指定 group 的 topic 关系
    pub async fn cleanup_group(pool: &sqlx::SqlitePool, cluster_id: &str, group_name: &str) -> Result<()> {
        sqlx::query("DELETE FROM consumer_group_topics WHERE cluster_id = ? AND group_name = ?")
            .bind(cluster_id)
            .bind(group_name)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 按 topic 查询 consumer group names
    pub async fn list_group_names_by_topic(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        topic: &str,
    ) -> Result<Vec<String>> {
        let groups: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT group_name FROM consumer_group_topics WHERE cluster_id = ? AND topic_name = ?",
        )
        .bind(cluster_id)
        .bind(topic)
        .fetch_all(pool)
        .await?;

        Ok(groups.into_iter().map(|(name,)| name).collect())
    }

    /// 按 group 查询其关联的 topics
    pub async fn list_topics_by_group(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        group_name: &str,
    ) -> Result<Vec<String>> {
        let topics: Vec<(String,)> = sqlx::query_as(
            "SELECT topic_name FROM consumer_group_topics WHERE cluster_id = ? AND group_name = ? ORDER BY topic_name",
        )
        .bind(cluster_id)
        .bind(group_name)
        .fetch_all(pool)
        .await?;

        Ok(topics.into_iter().map(|(t,)| t).collect())
    }
}

/// 同步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}
