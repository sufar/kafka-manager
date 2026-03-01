/// 集群连接历史存储模块

use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

/// 集群连接历史状态
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ClusterConnectionHistory {
    pub id: i64,
    pub cluster_name: String,
    pub status: String,
    pub error_message: Option<String>,
    pub latency_ms: Option<i64>,
    pub checked_at: String,
}

/// 连接状态统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub cluster_name: String,
    pub total_checks: i64,
    pub successful_checks: i64,
    pub failed_checks: i64,
    pub avg_latency_ms: Option<f64>,
    pub last_status: String,
    pub last_checked_at: Option<String>,
}

/// 存储操作
pub struct ClusterConnectionStore;

impl ClusterConnectionStore {
    /// 记录连接状态
    pub async fn record(
        pool: &sqlx::SqlitePool,
        cluster_name: &str,
        status: &str,
        error_message: Option<&str>,
        latency_ms: Option<i64>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO cluster_connection_history
            (cluster_name, status, error_message, latency_ms, checked_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(cluster_name)
        .bind(status)
        .bind(error_message)
        .bind(latency_ms)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 获取集群的历史记录
    pub async fn get_history(
        pool: &sqlx::SqlitePool,
        cluster_name: &str,
        limit: i64,
    ) -> Result<Vec<ClusterConnectionHistory>> {
        let histories = sqlx::query_as::<_, ClusterConnectionHistory>(
            r#"
            SELECT * FROM cluster_connection_history
            WHERE cluster_name = ?
            ORDER BY checked_at DESC
            LIMIT ?
            "#,
        )
        .bind(cluster_name)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(histories)
    }

    /// 获取集群连接统计
    pub async fn get_stats(
        pool: &sqlx::SqlitePool,
        cluster_name: &str,
    ) -> Result<Option<ConnectionStats>> {
        let row = sqlx::query(
            r#"
            SELECT
                cluster_name,
                COUNT(*) as total_checks,
                SUM(CASE WHEN status = 'connected' THEN 1 ELSE 0 END) as successful_checks,
                SUM(CASE WHEN status != 'connected' THEN 1 ELSE 0 END) as failed_checks,
                AVG(latency_ms) as avg_latency_ms,
                (SELECT status FROM cluster_connection_history
                 WHERE cluster_name = ? ORDER BY checked_at DESC LIMIT 1) as last_status,
                (SELECT checked_at FROM cluster_connection_history
                 WHERE cluster_name = ? ORDER BY checked_at DESC LIMIT 1) as last_checked_at
            FROM cluster_connection_history
            WHERE cluster_name = ?
            "#,
        )
        .bind(cluster_name)
        .bind(cluster_name)
        .bind(cluster_name)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => {
                let total_checks: i64 = row.get("total_checks");
                let successful_checks: i64 = row.get("successful_checks");
                let failed_checks: i64 = row.get("failed_checks");
                let avg_latency_ms: Option<f64> = row.get("avg_latency_ms");
                let last_status: String = row.get("last_status");
                let last_checked_at: Option<String> = row.get("last_checked_at");

                Ok(Some(ConnectionStats {
                    cluster_name: cluster_name.to_string(),
                    total_checks,
                    successful_checks,
                    failed_checks,
                    avg_latency_ms,
                    last_status,
                    last_checked_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// 获取所有集群的统计
    pub async fn get_all_stats(
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<ConnectionStats>> {
        let rows = sqlx::query(
            r#"
            SELECT
                cluster_name,
                COUNT(*) as total_checks,
                SUM(CASE WHEN status = 'connected' THEN 1 ELSE 0 END) as successful_checks,
                SUM(CASE WHEN status != 'connected' THEN 1 ELSE 0 END) as failed_checks,
                AVG(latency_ms) as avg_latency_ms,
                (SELECT status FROM cluster_connection_history
                 WHERE cluster_name = c.cluster_name ORDER BY checked_at DESC LIMIT 1) as last_status,
                (SELECT checked_at FROM cluster_connection_history
                 WHERE cluster_name = c.cluster_name ORDER BY checked_at DESC LIMIT 1) as last_checked_at
            FROM cluster_connection_history c
            GROUP BY cluster_name
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut stats = Vec::new();
        for row in rows {
            let cluster_name: String = row.get("cluster_name");
            let total_checks: i64 = row.get("total_checks");
            let successful_checks: i64 = row.get("successful_checks");
            let failed_checks: i64 = row.get("failed_checks");
            let avg_latency_ms: Option<f64> = row.get("avg_latency_ms");
            let last_status: String = row.get("last_status");
            let last_checked_at: Option<String> = row.get("last_checked_at");

            stats.push(ConnectionStats {
                cluster_name,
                total_checks,
                successful_checks,
                failed_checks,
                avg_latency_ms,
                last_status,
                last_checked_at,
            });
        }

        Ok(stats)
    }

    /// 清理旧的历史记录
    pub async fn cleanup_old(
        pool: &sqlx::SqlitePool,
        hours: i64,
    ) -> Result<i64> {
        let cutoff = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::hours(hours))
            .unwrap()
            .to_rfc3339();

        let result = sqlx::query(
            r#"
            DELETE FROM cluster_connection_history
            WHERE checked_at < ?
            "#,
        )
        .bind(&cutoff)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
