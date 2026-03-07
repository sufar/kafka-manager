/// 告警通知模块

use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 通知配置
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub id: i64,
    pub name: String,
    pub config_type: String,  // EMAIL, WEBHOOK, DINGTALK, WECHAT, SLACK
    pub webhook_url: Option<String>,
    pub email_recipients: Option<String>,  // JSON 数组
    pub dingtalk_webhook: Option<String>,
    pub dingtalk_secret: Option<String>,
    pub wechat_webhook: Option<String>,
    pub slack_webhook: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 告警历史
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AlertHistory {
    pub id: i64,
    pub rule_id: i64,
    pub cluster_id: String,
    pub alert_type: String,
    pub alert_message: String,
    pub alert_value: Option<f64>,
    pub threshold: f64,
    pub severity: String,
    pub notified: bool,
    pub notification_configs: Option<String>,  // JSON 数组，记录发送的通知配置 ID
    pub created_at: String,
}

/// 通知配置创建请求
#[derive(Debug, Deserialize)]
pub struct CreateNotificationConfigRequest {
    pub name: String,
    pub config_type: String,
    pub webhook_url: Option<String>,
    pub email_recipients: Option<Vec<String>>,
    pub dingtalk_webhook: Option<String>,
    pub dingtalk_secret: Option<String>,
    pub wechat_webhook: Option<String>,
    pub slack_webhook: Option<String>,
}

/// 告警历史查询参数
#[derive(Debug, Deserialize)]
pub struct AlertHistoryQuery {
    pub cluster_id: Option<String>,
    pub rule_id: Option<i64>,
    pub severity: Option<String>,
    pub limit: Option<i64>,
}

/// 存储操作
pub struct NotificationStore;

impl NotificationStore {
    /// 创建通知配置
    pub async fn create(
        pool: &sqlx::SqlitePool,
        req: &CreateNotificationConfigRequest,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let email_json = req.email_recipients.as_ref()
            .map(|r| serde_json::to_string(r))
            .transpose()
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to serialize email recipients: {}", e)))?;

        let result = sqlx::query(
            r#"
            INSERT INTO notification_configs
            (name, config_type, webhook_url, email_recipients, dingtalk_webhook, dingtalk_secret,
             wechat_webhook, slack_webhook, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&req.name)
        .bind(&req.config_type)
        .bind(&req.webhook_url)
        .bind(&email_json)
        .bind(&req.dingtalk_webhook)
        .bind(&req.dingtalk_secret)
        .bind(&req.wechat_webhook)
        .bind(&req.slack_webhook)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 更新通知配置
    pub async fn update(
        pool: &sqlx::SqlitePool,
        id: i64,
        enabled: Option<bool>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE notification_configs SET enabled = COALESCE(?, enabled), updated_at = ? WHERE id = ?"
        )
        .bind(enabled)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 删除通知配置
    pub async fn delete(pool: &sqlx::SqlitePool, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM notification_configs WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 获取通知配置
    pub async fn get_by_id(pool: &sqlx::SqlitePool, id: i64) -> Result<Option<NotificationConfig>> {
        let config = sqlx::query_as::<_, NotificationConfig>(
            "SELECT * FROM notification_configs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(config)
    }

    /// 获取所有启用的通知配置
    pub async fn list_enabled(pool: &sqlx::SqlitePool) -> Result<Vec<NotificationConfig>> {
        let configs = sqlx::query_as::<_, NotificationConfig>(
            "SELECT * FROM notification_configs WHERE enabled = 1 ORDER BY id DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(configs)
    }

    /// 获取所有通知配置
    pub async fn list(pool: &sqlx::SqlitePool) -> Result<Vec<NotificationConfig>> {
        let configs = sqlx::query_as::<_, NotificationConfig>(
            "SELECT * FROM notification_configs ORDER BY id DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(configs)
    }

    /// 记录告警历史
    pub async fn record_alert(
        pool: &sqlx::SqlitePool,
        rule_id: i64,
        cluster_id: &str,
        alert_type: &str,
        alert_message: &str,
        alert_value: Option<f64>,
        threshold: f64,
        severity: &str,
        notified: bool,
        notification_configs: Option<&[i64]>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let configs_json = notification_configs.map(|c| serde_json::to_string(&c))
            .transpose()
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to serialize notification configs: {}", e)))?;

        let result = sqlx::query(
            r#"
            INSERT INTO alert_history
            (rule_id, cluster_id, alert_type, alert_message, alert_value, threshold, severity,
             notified, notification_configs, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(rule_id)
        .bind(cluster_id)
        .bind(alert_type)
        .bind(alert_message)
        .bind(alert_value)
        .bind(threshold)
        .bind(severity)
        .bind(notified)
        .bind(&configs_json)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 查询告警历史
    pub async fn list_history(
        pool: &sqlx::SqlitePool,
        query: &AlertHistoryQuery,
    ) -> Result<Vec<AlertHistory>> {
        let mut sql = String::from("SELECT * FROM alert_history WHERE 1=1");
        let limit = query.limit.unwrap_or(100);

        if query.cluster_id.is_some() {
            sql.push_str(" AND cluster_id = ?");
        }
        if query.rule_id.is_some() {
            sql.push_str(" AND rule_id = ?");
        }
        if query.severity.is_some() {
            sql.push_str(" AND severity = ?");
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut builder = sqlx::query_as::<_, AlertHistory>(&sql);

        if let Some(ref cluster_id) = query.cluster_id {
            builder = builder.bind(cluster_id);
        }
        if let Some(rule_id) = query.rule_id {
            builder = builder.bind(rule_id);
        }
        if let Some(ref severity) = query.severity {
            builder = builder.bind(severity);
        }

        let history = builder.bind(limit).fetch_all(pool).await?;

        Ok(history)
    }
}

/// 通知发送器
pub struct NotificationSender;

impl NotificationSender {
    /// 发送通知
    pub async fn send(
        config: &NotificationConfig,
        title: &str,
        message: &str,
    ) -> Result<()> {
        match config.config_type.as_str() {
            "WEBHOOK" => Self::send_webhook(&config.webhook_url, title, message).await,
            "DINGTALK" => Self::send_dingtalk(&config.dingtalk_webhook, &config.dingtalk_secret, title, message).await,
            "WECHAT" => Self::send_wechat(&config.wechat_webhook, title, message).await,
            "SLACK" => Self::send_slack(&config.slack_webhook, title, message).await,
            "EMAIL" => Self::send_email(&config.email_recipients, title, message).await,
            _ => Ok(()),
        }
    }

    async fn send_webhook(url: &Option<String>, title: &str, message: &str) -> Result<()> {
        let url = match url {
            Some(u) => u,
            None => return Ok(()),
        };

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "title": title,
            "message": message
        });

        client.post(url).json(&body).send().await?;

        Ok(())
    }

    async fn send_dingtalk(webhook: &Option<String>, secret: &Option<String>, title: &str, message: &str) -> Result<()> {
        let webhook = match webhook {
            Some(w) => w,
            None => return Ok(()),
        };

        let mut url = webhook.clone();

        // 添加签名（如果有 secret）
        if let Some(s) = secret {
            let timestamp = chrono::Utc::now().timestamp_millis().to_string();
            let string_to_sign = format!("{}\n{}", timestamp, s);

            // 使用简单的哈希计算签名
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            string_to_sign.hash(&mut hasher);
            let signature = hasher.finish();

            url = format!("{}&timestamp={}&sign={}", url, timestamp, signature);
        }

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "title": title,
                "text": format!("**{}**\n\n{}", title, message)
            }
        });

        client.post(&url).json(&body).send().await?;

        Ok(())
    }

    async fn send_wechat(webhook: &Option<String>, title: &str, message: &str) -> Result<()> {
        let webhook = match webhook {
            Some(w) => w,
            None => return Ok(()),
        };

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "content": format!("**{}**\n\n> {}", title, message)
            }
        });

        client.post(webhook).json(&body).send().await?;

        Ok(())
    }

    async fn send_slack(webhook: &Option<String>, title: &str, message: &str) -> Result<()> {
        let webhook = match webhook {
            Some(w) => w,
            None => return Ok(()),
        };

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "text": title,
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": title
                    }
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": message
                    }
                }
            ]
        });

        client.post(webhook).json(&body).send().await?;

        Ok(())
    }

    async fn send_email(_recipients: &Option<String>, _title: &str, _message: &str) -> Result<()> {
        // TODO: 实现邮件发送
        // 需要配置 SMTP 服务器
        Ok(())
    }
}
