use sqlx::SqlitePool;
use std::collections::HashMap;

/// Topic 模板记录
#[derive(Debug, Clone)]
pub struct TopicTemplateRecord {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub num_partitions: i32,
    pub replication_factor: i32,
    pub config_json: String,  // JSON 格式的配置
    pub created_at: String,
    pub updated_at: String,
}

/// Topic 模板存储
pub struct TopicTemplateStore {
    pool: SqlitePool,
}

impl TopicTemplateStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建模板
    pub async fn create(&self, template: &CreateTopicTemplateRequest) -> Result<i64, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let config_json = serde_json::to_string(&template.config).unwrap_or("{}".to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO topic_templates (
                name, description, num_partitions, replication_factor,
                config_json, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&template.name)
        .bind(&template.description)
        .bind(template.num_partitions)
        .bind(template.replication_factor)
        .bind(&config_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 获取模板列表
    pub async fn list(&self) -> Result<Vec<TopicTemplateRecord>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM topic_templates ORDER BY name")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().filter_map(|row| self.row_to_record(row).ok()).collect())
    }

    /// 获取单个模板
    pub async fn get(&self, id: i64) -> Result<Option<TopicTemplateRecord>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM topic_templates WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(self.row_to_record(row).ok()),
            None => Ok(None),
        }
    }

    /// 按名称获取模板
    pub async fn get_by_name(&self, name: &str) -> Result<Option<TopicTemplateRecord>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM topic_templates WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(self.row_to_record(row).ok()),
            None => Ok(None),
        }
    }

    /// 更新模板
    pub async fn update(&self, id: i64, template: &UpdateTopicTemplateRequest) -> Result<bool, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut updated = false;

        if let Some(name) = &template.name {
            sqlx::query("UPDATE topic_templates SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        if let Some(description) = &template.description {
            sqlx::query("UPDATE topic_templates SET description = ?, updated_at = ? WHERE id = ?")
                .bind(description)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        if let Some(num_partitions) = template.num_partitions {
            sqlx::query("UPDATE topic_templates SET num_partitions = ?, updated_at = ? WHERE id = ?")
                .bind(num_partitions)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        if let Some(replication_factor) = template.replication_factor {
            sqlx::query("UPDATE topic_templates SET replication_factor = ?, updated_at = ? WHERE id = ?")
                .bind(replication_factor)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        if let Some(config) = &template.config {
            let config_json = serde_json::to_string(config).unwrap_or("{}".to_string());
            sqlx::query("UPDATE topic_templates SET config_json = ?, updated_at = ? WHERE id = ?")
                .bind(&config_json)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            updated = true;
        }

        Ok(updated)
    }

    /// 删除模板
    pub async fn delete(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM topic_templates WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    fn row_to_record(&self, row: sqlx::sqlite::SqliteRow) -> Result<TopicTemplateRecord, sqlx::Error> {
        use sqlx::Row;
        Ok(TopicTemplateRecord {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            num_partitions: row.try_get("num_partitions")?,
            replication_factor: row.try_get("replication_factor")?,
            config_json: row.try_get("config_json")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// 创建 Topic 模板请求
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateTopicTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_partitions")]
    pub num_partitions: i32,
    #[serde(default = "default_replication")]
    pub replication_factor: i32,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

fn default_partitions() -> i32 { 3 }
fn default_replication() -> i32 { 1 }

/// 更新 Topic 模板请求
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct UpdateTopicTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub num_partitions: Option<i32>,
    pub replication_factor: Option<i32>,
    pub config: Option<HashMap<String, String>>,
}

/// Topic 模板响应
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TopicTemplateResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub num_partitions: i32,
    pub replication_factor: i32,
    pub config: HashMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TopicTemplateRecord> for TopicTemplateResponse {
    fn from(record: TopicTemplateRecord) -> Self {
        let config: HashMap<String, String> = serde_json::from_str(&record.config_json)
            .unwrap_or_else(|_| HashMap::new());
        Self {
            id: record.id,
            name: record.name,
            description: record.description,
            num_partitions: record.num_partitions,
            replication_factor: record.replication_factor,
            config,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// 使用模板创建 Topic 请求
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateTopicFromTemplateRequest {
    pub topic_name: String,
    pub template_id: Option<i64>,
    pub template_name: Option<String>,
    /// 可选的覆盖配置
    pub override_config: Option<HashMap<String, String>>,
}

/// 预定义模板
pub mod preset_templates {
    use super::*;

    pub fn get_preset_templates() -> Vec<CreateTopicTemplateRequest> {
        let mut templates = Vec::new();

        // default 模板
        let mut default_config = HashMap::new();
        default_config.insert("retention.ms".to_string(), "604800000".to_string());  // 7 天
        default_config.insert("segment.bytes".to_string(), "1073741824".to_string()); // 1GB
        templates.push(CreateTopicTemplateRequest {
            name: "default".to_string(),
            description: Some("默认模板：3 分区，1 副本，保留 7 天".to_string()),
            num_partitions: 3,
            replication_factor: 1,
            config: default_config,
        });

        // high-throughput 模板
        let mut high_throughput_config = HashMap::new();
        high_throughput_config.insert("retention.ms".to_string(), "259200000".to_string());  // 3 天
        high_throughput_config.insert("segment.bytes".to_string(), "536870912".to_string()); // 512MB
        high_throughput_config.insert("compression.type".to_string(), "lz4".to_string());
        templates.push(CreateTopicTemplateRequest {
            name: "high-throughput".to_string(),
            description: Some("高吞吐模板：12 分区，3 副本，保留 3 天".to_string()),
            num_partitions: 12,
            replication_factor: 3,
            config: high_throughput_config,
        });

        // event-sourcing 模板
        let mut event_sourcing_config = HashMap::new();
        event_sourcing_config.insert("retention.ms".to_string(), "-1".to_string());  // 永久
        event_sourcing_config.insert("cleanup.policy".to_string(), "compact".to_string());
        event_sourcing_config.insert("min.compaction.lag.ms".to_string(), "60000".to_string());
        templates.push(CreateTopicTemplateRequest {
            name: "event-sourcing".to_string(),
            description: Some("事件溯源模板：6 分区，3 副本，永久保留".to_string()),
            num_partitions: 6,
            replication_factor: 3,
            config: event_sourcing_config,
        });

        // logging 模板
        let mut logging_config = HashMap::new();
        logging_config.insert("retention.ms".to_string(), "86400000".to_string());   // 1 天
        logging_config.insert("retention.bytes".to_string(), "10737418240".to_string()); // 10GB
        logging_config.insert("compression.type".to_string(), "gzip".to_string());
        templates.push(CreateTopicTemplateRequest {
            name: "logging".to_string(),
            description: Some("日志模板：6 分区，2 副本，保留 1 天，压缩".to_string()),
            num_partitions: 6,
            replication_factor: 2,
            config: logging_config,
        });

        // dead-letter 模板
        let mut dlt_config = HashMap::new();
        dlt_config.insert("retention.ms".to_string(), "2592000000".to_string());  // 30 天
        dlt_config.insert("max.message.bytes".to_string(), "10485760".to_string()); // 10MB
        templates.push(CreateTopicTemplateRequest {
            name: "dead-letter".to_string(),
            description: Some("死信队列模板：3 分区，3 副本，保留 30 天".to_string()),
            num_partitions: 3,
            replication_factor: 3,
            config: dlt_config,
        });

        templates
    }
}
