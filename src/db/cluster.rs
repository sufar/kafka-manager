use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Kafka 集群数据库模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct KafkaCluster {
    pub id: i64,
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i64,
    pub operation_timeout_ms: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建集群请求
#[derive(Debug, Deserialize)]
pub struct CreateClusterRequest {
    pub name: String,
    pub brokers: String,
    #[serde(default = "default_timeout")]
    pub request_timeout_ms: i64,
    #[serde(default = "default_timeout")]
    pub operation_timeout_ms: i64,
}

/// 更新集群请求
#[derive(Debug, Deserialize)]
pub struct UpdateClusterRequest {
    pub name: Option<String>,
    pub brokers: Option<String>,
    pub request_timeout_ms: Option<i64>,
    pub operation_timeout_ms: Option<i64>,
}

fn default_timeout() -> i64 {
    5000
}

/// 集群存储操作
pub struct ClusterStore;

impl ClusterStore {
    /// 列出所有集群
    pub async fn list(pool: &sqlx::SqlitePool) -> Result<Vec<KafkaCluster>> {
        let clusters = sqlx::query_as::<_, KafkaCluster>(
            "SELECT * FROM kafka_clusters ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(clusters)
    }

    /// 获取集群详情
    pub async fn get(pool: &sqlx::SqlitePool, id: i64) -> Result<KafkaCluster> {
        let cluster = sqlx::query_as::<_, KafkaCluster>(
            "SELECT * FROM kafka_clusters WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Cluster {} not found", id)))?;

        Ok(cluster)
    }

    /// 按名称获取集群
    pub async fn get_by_name(pool: &sqlx::SqlitePool, name: &str) -> Result<Option<KafkaCluster>> {
        let cluster = sqlx::query_as::<_, KafkaCluster>(
            "SELECT * FROM kafka_clusters WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;

        Ok(cluster)
    }

    /// 创建集群
    pub async fn create(
        pool: &sqlx::SqlitePool,
        req: &CreateClusterRequest,
    ) -> Result<KafkaCluster> {
        let now = chrono::Utc::now().to_rfc3339();

        let cluster = sqlx::query_as::<_, KafkaCluster>(
            r#"
            INSERT INTO kafka_clusters (name, brokers, request_timeout_ms, operation_timeout_ms, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&req.name)
        .bind(&req.brokers)
        .bind(req.request_timeout_ms)
        .bind(req.operation_timeout_ms)
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await?;

        Ok(cluster)
    }

    /// 更新集群
    pub async fn update(
        pool: &sqlx::SqlitePool,
        id: i64,
        req: &UpdateClusterRequest,
    ) -> Result<KafkaCluster> {
        let now = chrono::Utc::now().to_rfc3339();

        // 先获取原有数据
        let existing = Self::get(pool, id).await?;

        let name = req.name.as_ref().unwrap_or(&existing.name);
        let brokers = req.brokers.as_ref().unwrap_or(&existing.brokers);
        let request_timeout_ms = req.request_timeout_ms.unwrap_or(existing.request_timeout_ms);
        let operation_timeout_ms = req.operation_timeout_ms.unwrap_or(existing.operation_timeout_ms);

        let cluster = sqlx::query_as::<_, KafkaCluster>(
            r#"
            UPDATE kafka_clusters
            SET name = ?, brokers = ?, request_timeout_ms = ?, operation_timeout_ms = ?, updated_at = ?
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(brokers)
        .bind(request_timeout_ms)
        .bind(operation_timeout_ms)
        .bind(&now)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Cluster {} not found", id)))?;

        Ok(cluster)
    }

    /// 删除集群
    pub async fn delete(pool: &sqlx::SqlitePool, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM kafka_clusters WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Cluster {} not found", id)));
        }

        Ok(())
    }

    /// 测试集群连接
    pub async fn test_connection(pool: &sqlx::SqlitePool, id: i64) -> Result<bool> {
        let cluster = Self::get(pool, id).await?;

        // 尝试创建临时客户端测试连接
        use crate::config::KafkaConfig;
        use crate::kafka::KafkaAdmin;

        let config = KafkaConfig {
            brokers: cluster.brokers,
            request_timeout_ms: cluster.request_timeout_ms as u32,
            operation_timeout_ms: cluster.operation_timeout_ms as u32,
        };

        match KafkaAdmin::new(&config) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
