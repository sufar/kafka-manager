/// Schema Registry 管理模块

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use reqwest::Client;

/// Schema 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub subject: String,
    pub version: i32,
    pub id: i64,
    pub schema_type: String,
    pub schema: String,
    pub references: Vec<SchemaReference>,
}

/// Schema 引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaReference {
    pub name: String,
    pub subject: String,
    pub version: i32,
}

/// Schema 版本列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersions {
    pub subject: String,
    pub versions: Vec<i32>,
}

/// 兼容性检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityCheck {
    pub is_compatible: bool,
    pub messages: Vec<String>,
}

/// Schema Registry 客户端
pub struct SchemaRegistryClient {
    base_url: String,
    client: Client,
}

impl SchemaRegistryClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    /// 获取所有主题
    pub async fn get_subjects(&self) -> Result<Vec<String>> {
        let url = format!("{}/subjects", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("Schema Registry error: {}", response.status())));
        }

        Ok(response.json().await?)
    }

    /// 获取主题的所有版本
    pub async fn get_versions(&self, subject: &str) -> Result<Vec<i32>> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(AppError::NotFound(format!("Subject '{}' not found", subject)));
        }

        Ok(response.json().await?)
    }

    /// 获取指定版本的 Schema
    pub async fn get_schema(&self, subject: &str, version: &str) -> Result<SchemaInfo> {
        let url = format!("{}/subjects/{}/versions/{}", self.base_url, subject, version);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(AppError::NotFound(format!("Schema {} version {} not found", subject, version)));
        }

        Ok(response.json().await?)
    }

    /// 注册新 Schema
    pub async fn register_schema(
        &self,
        subject: &str,
        schema: &str,
        schema_type: &str,
    ) -> Result<i64> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);

        let body = serde_json::json!({
            "schema": schema,
            "schemaType": schema_type,
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("Failed to register schema: {}", response.status())));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["id"].as_i64().unwrap_or(0))
    }

    /// 检查兼容性
    pub async fn check_compatibility(
        &self,
        subject: &str,
        version: &str,
        schema: &str,
    ) -> Result<CompatibilityCheck> {
        let url = format!("{}/compatibility/subjects/{}/versions/{}", self.base_url, subject, version);

        let body = serde_json::json!({
            "schema": schema,
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("Compatibility check failed: {}", response.status())));
        }

        let result: serde_json::Value = response.json().await?;

        Ok(CompatibilityCheck {
            is_compatible: result["is_compatible"].as_bool().unwrap_or(false),
            messages: result["messages"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
    }

    /// 删除 Schema 版本
    pub async fn delete_schema_version(&self, subject: &str, version: &str) -> Result<i32> {
        let url = format!("{}/subjects/{}/versions/{}", self.base_url, subject, version);
        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("Failed to delete schema: {}", response.status())));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["id"].as_i64().unwrap_or(0) as i32)
    }

    /// 删除主题
    pub async fn delete_subject(&self, subject: &str) -> Result<Vec<i32>> {
        let url = format!("{}/subjects/{}", self.base_url, subject);
        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("Failed to delete subject: {}", response.status())));
        }

        Ok(response.json().await?)
    }

    /// 获取全局兼容性级别
    pub async fn get_compatibility_level(&self) -> Result<String> {
        let url = format!("{}/config", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("Failed to get compatibility level: {}", response.status())));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["compatibilityLevel"].as_str().unwrap_or("BACKWARD").to_string())
    }

    /// 更新全局兼容性级别
    pub async fn update_compatibility_level(&self, level: &str) -> Result<()> {
        let url = format!("{}/config", self.base_url);

        let body = serde_json::json!({
            "compatibility": level,
        });

        let response = self.client.put(&url).json(&body).send().await?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!("Failed to update compatibility level: {}", response.status())));
        }

        Ok(())
    }
}
