use sqlx::{sqlite::SqlitePool, FromRow};

/// 审计日志数据库记录
#[derive(Debug, FromRow)]
pub struct AuditLogDb {
    #[allow(dead_code)]
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub cluster_id: Option<String>,
    pub resource: Option<String>,
    pub action: String,
    pub api_key: Option<String>,
    pub status: i32,
    pub duration_ms: i64,
    pub client_ip: Option<String>,
}

/// 审计日志存储操作
pub struct AuditLogStore;

impl AuditLogStore {
    /// 保存审计日志
    #[allow(dead_code)]
    pub async fn save(
        pool: &SqlitePool,
        timestamp: &str,
        method: &str,
        path: &str,
        cluster_id: Option<&str>,
        resource: Option<&str>,
        action: &str,
        api_key: Option<&str>,
        status: i32,
        duration_ms: i64,
        client_ip: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO audit_logs
            (timestamp, method, path, cluster_id, resource, action, api_key, status, duration_ms, client_ip)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(timestamp)
        .bind(method)
        .bind(path)
        .bind(cluster_id)
        .bind(resource)
        .bind(action)
        .bind(api_key)
        .bind(status)
        .bind(duration_ms)
        .bind(client_ip)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 列出审计日志（支持分页和过滤）
    pub async fn list(
        pool: &SqlitePool,
        limit: i64,
        offset: i64,
        action_filter: Option<&str>,
        cluster_id_filter: Option<&str>,
        status_filter: Option<i32>,
    ) -> Result<Vec<AuditLogDb>, sqlx::Error> {
        let mut conditions = Vec::new();
        let mut binds: Vec<String> = Vec::new();

        if let Some(action) = action_filter {
            conditions.push("action = ?");
            binds.push(action.to_string());
        }
        if let Some(cluster_id) = cluster_id_filter {
            conditions.push("cluster_id = ?");
            binds.push(cluster_id.to_string());
        }
        if let Some(status) = status_filter {
            conditions.push("status = ?");
            binds.push(status.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT id, timestamp, method, path, cluster_id, resource, action, api_key, status, duration_ms, client_ip
            FROM audit_logs
            {}
            ORDER BY timestamp DESC
            LIMIT ? OFFSET ?
            "#,
            where_clause
        );

        let mut query = sqlx::query_as::<_, AuditLogDb>(&sql);

        for bind in binds {
            query = query.bind(bind);
        }
        query = query.bind(limit).bind(offset);

        let rows = query.fetch_all(pool).await?;
        Ok(rows)
    }

    /// 获取总数
    pub async fn count(
        pool: &SqlitePool,
        action_filter: Option<&str>,
        cluster_id_filter: Option<&str>,
        status_filter: Option<i32>,
    ) -> Result<i64, sqlx::Error> {
        let mut conditions = Vec::new();
        let mut binds: Vec<String> = Vec::new();

        if let Some(action) = action_filter {
            conditions.push("action = ?");
            binds.push(action.to_string());
        }
        if let Some(cluster_id) = cluster_id_filter {
            conditions.push("cluster_id = ?");
            binds.push(cluster_id.to_string());
        }
        if let Some(status) = status_filter {
            conditions.push("status = ?");
            binds.push(status.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT COUNT(*) FROM audit_logs
            {}
            "#,
            where_clause
        );

        let mut query = sqlx::query_scalar::<_, i64>(&sql);

        for bind in binds {
            query = query.bind(bind);
        }

        query.fetch_one(pool).await
    }

    /// 删除旧日志（保留最近 N 天）
    pub async fn delete_old(pool: &SqlitePool, days: i64) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM audit_logs
            WHERE timestamp < datetime('now', ?)
            "#,
        )
        .bind(&format!("-{} days", days))
        .execute(pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
