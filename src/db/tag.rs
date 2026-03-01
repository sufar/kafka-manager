use sqlx::SqlitePool;

/// 标签记录
#[derive(Debug, Clone)]
pub struct TagRecord {
    pub id: i64,
    pub cluster_id: String,
    pub resource_type: String,  // "topic" 或 "consumer_group"
    pub resource_name: String,
    pub tag_key: String,
    pub tag_value: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 标签存储
pub struct TagStore {
    pool: SqlitePool,
}

impl TagStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 为资源添加标签
    pub async fn add_tag(&self, cluster_id: &str, resource_type: &str, resource_name: &str, key: &str, value: &str) -> Result<i64, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO resource_tags (
                cluster_id, resource_type, resource_name, tag_key, tag_value,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(cluster_id, resource_type, resource_name, tag_key)
            DO UPDATE SET tag_value = excluded.tag_value, updated_at = excluded.updated_at
            "#,
        )
        .bind(cluster_id)
        .bind(resource_type)
        .bind(resource_name)
        .bind(key)
        .bind(value)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 删除标签
    pub async fn delete_tag(&self, cluster_id: &str, resource_type: &str, resource_name: &str, key: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM resource_tags WHERE cluster_id = ? AND resource_type = ? AND resource_name = ? AND tag_key = ?",
        )
        .bind(cluster_id)
        .bind(resource_type)
        .bind(resource_name)
        .bind(key)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// 删除资源的所有标签
    pub async fn delete_all_tags(&self, cluster_id: &str, resource_type: &str, resource_name: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM resource_tags WHERE cluster_id = ? AND resource_type = ? AND resource_name = ?",
        )
        .bind(cluster_id)
        .bind(resource_type)
        .bind(resource_name)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as i64)
    }

    /// 获取资源的标签列表
    pub async fn get_tags(&self, cluster_id: &str, resource_type: &str, resource_name: &str) -> Result<Vec<TagRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT * FROM resource_tags WHERE cluster_id = ? AND resource_type = ? AND resource_name = ? ORDER BY tag_key",
        )
        .bind(cluster_id)
        .bind(resource_type)
        .bind(resource_name)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|row| self.row_to_record(row).ok()).collect())
    }

    /// 按标签过滤资源
    pub async fn filter_by_tag(&self, cluster_id: &str, resource_type: &str, tag_key: &str, tag_value: Option<&str>) -> Result<Vec<String>, sqlx::Error> {
        let query = if tag_value.is_some() {
            "SELECT DISTINCT resource_name FROM resource_tags WHERE cluster_id = ? AND resource_type = ? AND tag_key = ? AND tag_value = ?"
        } else {
            "SELECT DISTINCT resource_name FROM resource_tags WHERE cluster_id = ? AND resource_type = ? AND tag_key = ?"
        };

        let mut db_query = sqlx::query_scalar::<_, String>(query)
            .bind(cluster_id)
            .bind(resource_type)
            .bind(tag_key);

        if let Some(value) = tag_value {
            db_query = db_query.bind(value);
        }

        db_query.fetch_all(&self.pool).await
    }

    /// 获取所有唯一的标签键
    pub async fn get_all_tag_keys(&self, cluster_id: &str, resource_type: Option<&str>) -> Result<Vec<String>, sqlx::Error> {
        let query = if resource_type.is_some() {
            "SELECT DISTINCT tag_key FROM resource_tags WHERE cluster_id = ? AND resource_type = ? ORDER BY tag_key"
        } else {
            "SELECT DISTINCT tag_key FROM resource_tags WHERE cluster_id = ? ORDER BY tag_key"
        };

        let mut db_query = sqlx::query_scalar::<_, String>(query).bind(cluster_id);

        if let Some(rtype) = resource_type {
            db_query = db_query.bind(rtype);
        }

        db_query.fetch_all(&self.pool).await
    }

    /// 获取标签键对应的所有值
    pub async fn get_tag_values(&self, cluster_id: &str, resource_type: &str, tag_key: &str) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT tag_value FROM resource_tags WHERE cluster_id = ? AND resource_type = ? AND tag_key = ? ORDER BY tag_value",
        )
        .bind(cluster_id)
        .bind(resource_type)
        .bind(tag_key)
        .fetch_all(&self.pool)
        .await
    }

    /// 批量更新资源标签
    pub async fn batch_update_tags(&self, cluster_id: &str, resource_type: &str, resource_name: &str, tags: &std::collections::HashMap<String, String>) -> Result<(), sqlx::Error> {
        for (key, value) in tags {
            self.add_tag(cluster_id, resource_type, resource_name, key, value).await?;
        }
        Ok(())
    }

    fn row_to_record(&self, row: sqlx::sqlite::SqliteRow) -> Result<TagRecord, sqlx::Error> {
        use sqlx::Row;
        Ok(TagRecord {
            id: row.try_get("id")?,
            cluster_id: row.try_get("cluster_id")?,
            resource_type: row.try_get("resource_type")?,
            resource_name: row.try_get("resource_name")?,
            tag_key: row.try_get("tag_key")?,
            tag_value: row.try_get("tag_value")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// 创建/更新标签请求
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TagRequest {
    pub key: String,
    pub value: String,
}

/// 批量标签请求
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BatchTagsRequest {
    pub tags: std::collections::HashMap<String, String>,
}

/// 标签响应
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TagResponse {
    pub id: i64,
    pub cluster_id: String,
    pub resource_type: String,
    pub resource_name: String,
    pub tag_key: String,
    pub tag_value: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TagRecord> for TagResponse {
    fn from(record: TagRecord) -> Self {
        Self {
            id: record.id,
            cluster_id: record.cluster_id,
            resource_type: record.resource_type,
            resource_name: record.resource_name,
            tag_key: record.tag_key,
            tag_value: record.tag_value,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}
