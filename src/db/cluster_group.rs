use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 集群分组数据库模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ClusterGroup {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建分组请求
#[derive(Debug, Deserialize, Clone)]
pub struct CreateClusterGroupRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

/// 更新分组请求
#[derive(Debug, Deserialize, Clone)]
pub struct UpdateClusterGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i64>,
}

/// 集群分组存储操作
pub struct ClusterGroupStore;

impl ClusterGroupStore {
    /// 列出所有分组
    pub async fn list(pool: &sqlx::SqlitePool) -> Result<Vec<ClusterGroup>> {
        let groups = sqlx::query_as::<_, ClusterGroup>(
            "SELECT * FROM cluster_groups ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(pool)
        .await?;

        Ok(groups)
    }

    /// 获取分组详情
    pub async fn get(pool: &sqlx::SqlitePool, id: i64) -> Result<ClusterGroup> {
        let group = sqlx::query_as::<_, ClusterGroup>(
            "SELECT * FROM cluster_groups WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Cluster group {} not found", id)))?;

        Ok(group)
    }

    /// 按名称获取分组
    pub async fn get_by_name(pool: &sqlx::SqlitePool, name: &str) -> Result<Option<ClusterGroup>> {
        let group = sqlx::query_as::<_, ClusterGroup>(
            "SELECT * FROM cluster_groups WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;

        Ok(group)
    }

    /// 创建分组
    pub async fn create(
        pool: &sqlx::SqlitePool,
        req: &CreateClusterGroupRequest,
    ) -> Result<ClusterGroup> {
        let now = chrono::Utc::now().to_rfc3339();

        let group = sqlx::query_as::<_, ClusterGroup>(
            r#"
            INSERT INTO cluster_groups (name, description, sort_order, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.sort_order)
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await?;

        Ok(group)
    }

    /// 更新分组
    pub async fn update(
        pool: &sqlx::SqlitePool,
        id: i64,
        req: &UpdateClusterGroupRequest,
    ) -> Result<ClusterGroup> {
        let now = chrono::Utc::now().to_rfc3339();

        // 先获取原有数据
        let existing = Self::get(pool, id).await?;

        let name = req.name.as_ref().unwrap_or(&existing.name);
        let description = req.description.as_ref().or(existing.description.as_ref());
        let sort_order = req.sort_order.unwrap_or(existing.sort_order);

        let group = sqlx::query_as::<_, ClusterGroup>(
            r#"
            UPDATE cluster_groups
            SET name = ?, description = ?, sort_order = ?, updated_at = ?
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(sort_order)
        .bind(&now)
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(group)
    }

    /// 删除分组
    pub async fn delete(pool: &sqlx::SqlitePool, id: i64) -> Result<()> {
        // 检查是否有集群使用该分组
        let clusters_in_group = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kafka_clusters WHERE group_id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        if clusters_in_group > 0 {
            return Err(AppError::BadRequest(format!(
                "Cannot delete group: {} clusters are assigned to this group",
                clusters_in_group
            )));
        }

        let result = sqlx::query("DELETE FROM cluster_groups WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Group {} not found", id)));
        }

        Ok(())
    }

    /// 将集群分配到分组
    pub async fn assign_cluster_to_group(
        pool: &sqlx::SqlitePool,
        cluster_id: i64,
        group_id: i64,
    ) -> Result<()> {
        // 检查分组是否存在
        let _group = Self::get(pool, group_id).await?;

        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE kafka_clusters
            SET group_id = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(group_id)
        .bind(&now)
        .bind(cluster_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 获取分组下的所有集群
    pub async fn get_clusters_in_group(
        pool: &sqlx::SqlitePool,
        group_id: i64,
    ) -> Result<Vec<super::cluster::KafkaCluster>> {
        use crate::db::cluster::KafkaCluster;

        let clusters = sqlx::query_as::<_, KafkaCluster>(
            r#"
            SELECT * FROM kafka_clusters
            WHERE group_id = ?
            ORDER BY name ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(pool)
        .await?;

        Ok(clusters)
    }
}
