use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};

/// Schema Registry 客户端
pub struct SchemaRegistryClient {
    base_url: String,
    timeout: std::time::Duration,
}

impl SchemaRegistryClient {
    pub fn new(base_url: &str, timeout_ms: u64) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout: std::time::Duration::from_millis(timeout_ms),
        }
    }

    /// 获取所有 subject
    pub async fn list_subjects(&self) -> Result<Vec<String>> {
        let url = format!("{}/subjects", self.base_url);
        self.get(&url).await
    }

    /// 获取 subject 的所有版本
    pub async fn list_versions(&self, subject: &str) -> Result<Vec<i32>> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);
        self.get(&url).await
    }

    /// 获取指定 subject 和版本的 schema
    pub async fn get_schema(
        &self,
        subject: &str,
        version: &str,
    ) -> Result<SchemaResponse> {
        let url = format!("{}/subjects/{}/versions/{}", self.base_url, subject, version);
        self.get(&url).await
    }

    /// 获取最新版本的 schema
    pub async fn get_latest_schema(&self, subject: &str) -> Result<SchemaResponse> {
        self.get_schema(subject, "latest").await
    }

    /// 注册新 schema
    pub async fn register_schema(
        &self,
        subject: &str,
        schema: &str,
        schema_type: &str,
    ) -> Result<RegisterSchemaResponse> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);

        let body = RegisterRequest {
            schema: Some(schema.to_string()),
            schema_type: Some(schema_type.to_string()),
            references: None,
        };

        self.post(&url, &body).await
    }

    /// 删除 subject
    pub async fn delete_subject(&self, subject: &str) -> Result<Vec<i32>> {
        let url = format!("{}/subjects/{}", self.base_url, subject);
        self.delete(&url).await
    }

    /// 删除指定版本
    pub async fn delete_version(&self, subject: &str, version: &str) -> Result<i32> {
        let url = format!("{}/subjects/{}/versions/{}", self.base_url, subject, version);
        self.delete(&url).await
    }

    /// 检查 schema 兼容性
    pub async fn check_compatibility(
        &self,
        subject: &str,
        version: &str,
    ) -> Result<CompatibilityResponse> {
        let url = format!(
            "{}/compatibility/subjects/{}/versions/{}",
            self.base_url, subject, version
        );
        self.get(&url).await
    }

    /// 获取全局兼容性级别
    pub async fn get_compatibility_level(&self) -> Result<CompatibilityLevelResponse> {
        let url = format!("{}/config", self.base_url);
        self.get(&url).await
    }

    /// 设置 subject 的兼容性级别
    pub async fn set_compatibility_level(
        &self,
        subject: &str,
        level: &str,
    ) -> Result<CompatibilityLevelResponse> {
        let url = format!("{}/config/{}", self.base_url, subject);
        let body = serde_json::json!({ "compatibility": level });
        self.put(&url, &body).await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!("Schema Registry error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse response: {}", e)))
    }

    async fn post<T: serde::de::DeserializeOwned, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!("Schema Registry error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse response: {}", e)))
    }

    async fn put<T: serde::de::DeserializeOwned, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .put(url)
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!("Schema Registry error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse response: {}", e)))
    }

    async fn delete<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .delete(url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(format!("Schema Registry error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse response: {}", e)))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaResponse {
    pub subject: String,
    pub version: i32,
    pub id: i32,
    pub schema: String,
    #[serde(default)]
    pub schema_type: String,
    #[serde(default)]
    pub references: Vec<SchemaReference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaReference {
    pub name: String,
    pub subject: String,
    pub version: i32,
}

#[derive(Debug, Serialize)]
struct RegisterRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<SchemaReference>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterSchemaResponse {
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompatibilityResponse {
    pub is_compatible: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompatibilityLevelResponse {
    pub compatibility_level: String,
}
