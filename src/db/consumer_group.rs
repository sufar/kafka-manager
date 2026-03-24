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

    /// 返回新增和删除的 group 名称列表
    pub async fn sync_consumer_groups(
        pool: &sqlx::SqlitePool,
        cluster_id: &str,
        current_groups: &[String],
    ) -> Result<SyncResult> {
        let existing_groups = Self::list_by_cluster(pool, cluster_id).await?;
        let existing_names: std::collections::HashSet<_> = existing_groups
            .iter()
            .map(|g| g.group_name.as_str())
            .collect();

        let current_names: std::collections::HashSet<_> = current_groups
            .iter()
            .map(|s| s.as_str())
            .collect();

        let added: Vec<String> = current_groups
            .iter()
            .filter(|g| !existing_names.contains(g.as_str()))
            .cloned()
            .collect();

        let removed: Vec<String> = existing_groups
            .iter()
            .filter(|g| !current_names.contains(g.group_name.as_str()))
            .map(|g| g.group_name.clone())
            .collect();

        for group_name in &removed {
            Self::delete(pool, cluster_id, group_name).await?;
        }

        for group_name in &added {
            let topics = vec![];
            let _ = Self::upsert(pool, cluster_id, group_name, &topics).await;
        }

        Ok(SyncResult { added, removed })
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
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO consumer_group_offsets (cluster_id, group_name, topic_name, partition_id, offset_value, lag, fetched_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(cluster_id, group_name, topic_name, partition_id) DO UPDATE SET
                offset_value = excluded.offset_value,
                lag = excluded.lag,
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
}

/// 同步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}
