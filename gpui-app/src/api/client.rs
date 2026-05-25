//! API Client
//!
//! HTTP client for communicating with the Kafka Manager backend.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

/// HTTP client wrapper for API requests
#[derive(Clone, Debug)]
pub struct HttpClient {
    client: Arc<reqwest::Client>,
    base_url: String,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client: Arc::new(client),
            base_url,
        }
    }

    /// Create client with custom timeout
    pub fn with_timeout(base_url: String, timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client: Arc::new(client),
            base_url,
        }
    }

    /// Create client with default base URL
    pub fn default_client() -> Self {
        Self::new("http://localhost:8080".to_string())
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Set base URL
    pub fn set_base_url(&mut self, base_url: String) {
        self.base_url = base_url;
    }

    /// Build full URL for path
    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = self.build_url(path);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Http(status.as_u16()));
        }

        let body = response.text()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        serde_json::from_str(&body)
            .map_err(|e| ApiError::Parse(e.to_string()))
    }

    /// Make a POST request with body
    pub async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ApiError> {
        let url = self.build_url(path);

        let response = self.client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Http(status.as_u16()));
        }

        let body = response.text()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        serde_json::from_str(&body)
            .map_err(|e| ApiError::Parse(e.to_string()))
    }

    /// Make a PUT request with body
    pub async fn put<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ApiError> {
        let url = self.build_url(path);

        let response = self.client
            .put(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Http(status.as_u16()));
        }

        let body = response.text()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        serde_json::from_str(&body)
            .map_err(|e| ApiError::Parse(e.to_string()))
    }

    /// Make a DELETE request
    pub async fn delete(&self, path: &str) -> Result<(), ApiError> {
        let url = self.build_url(path);

        let response = self.client
            .delete(&url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Http(status.as_u16()));
        }

        Ok(())
    }

    /// Make a DELETE request expecting response body
    pub async fn delete_with_response<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = self.build_url(path);

        let response = self.client
            .delete(&url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Http(status.as_u16()));
        }

        let body = response.text()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        serde_json::from_str(&body)
            .map_err(|e| ApiError::Parse(e.to_string()))
    }

    /// Test connection to backend
    pub async fn test_connection(&self) -> Result<bool, ApiError> {
        let url = self.build_url("/api/health");

        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Get raw client for SSE streaming
    pub fn raw_client(&self) -> Arc<reqwest::Client> {
        self.client.clone()
    }
}

/// Legacy API client type alias (for backwards compatibility)
pub type ApiClient = HttpClient;

/// API error type
#[derive(Debug, Clone, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(u16),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Not found")]
    NotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Server error: {0}")]
    ServerError(String),
}

impl ApiError {
    /// Check if error is network related
    pub fn is_network_error(&self) -> bool {
        matches!(self, ApiError::Network(_))
    }

    /// Check if error is HTTP status related
    pub fn is_http_error(&self) -> bool {
        matches!(self, ApiError::Http(_))
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            ApiError::Http(404) => "资源未找到".to_string(),
            ApiError::Http(401) => "未授权访问".to_string(),
            ApiError::Http(403) => "禁止访问".to_string(),
            ApiError::Http(500) => "服务器内部错误".to_string(),
            ApiError::Http(code) => format!("HTTP错误: {}", code),
            ApiError::Network(msg) => format!("网络错误: {}", msg),
            ApiError::Parse(msg) => format!("数据解析错误: {}", msg),
            ApiError::NotFound => "未找到".to_string(),
            ApiError::Unauthorized => "未授权".to_string(),
            ApiError::ServerError(msg) => format!("服务器错误: {}", msg),
        }
    }
}

/// Cluster API operations
pub struct ClusterApi {
    client: HttpClient,
}

impl ClusterApi {
    /// Create new cluster API
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }

    /// List all clusters
    pub async fn list(&self) -> Result<Vec<crate::api::ClusterResponse>, ApiError> {
        self.client.get("/api/clusters").await
    }

    /// Get cluster by ID
    pub async fn get(&self, id: i32) -> Result<crate::api::ClusterResponse, ApiError> {
        self.client.get(&format!("/api/clusters/{}", id)).await
    }

    /// Create new cluster
    pub async fn create(&self, request: &crate::api::CreateClusterRequest) -> Result<crate::api::ClusterResponse, ApiError> {
        self.client.post("/api/clusters", request).await
    }

    /// Update cluster
    pub async fn update(&self, id: i32, request: &crate::api::UpdateClusterRequest) -> Result<crate::api::ClusterResponse, ApiError> {
        self.client.put(&format!("/api/clusters/{}", id), request).await
    }

    /// Delete cluster
    pub async fn delete(&self, id: i32) -> Result<(), ApiError> {
        self.client.delete(&format!("/api/clusters/{}", id)).await
    }

    /// Test cluster connection
    pub async fn test_connection(&self, id: i32) -> Result<ConnectionTestResult, ApiError> {
        self.client.post(&format!("/api/clusters/{}/test", id), &serde_json::json!({}))
            .await
    }

    /// Get cluster groups
    pub async fn list_groups(&self) -> Result<Vec<crate::api::ClusterGroupResponse>, ApiError> {
        self.client.get("/api/cluster-groups").await
    }
}

/// Connection test result
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: Option<String>,
    pub broker_count: Option<i32>,
}

/// Topic API operations
pub struct TopicApi {
    client: HttpClient,
}

impl TopicApi {
    /// Create new topic API
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }

    /// List topics for cluster
    pub async fn list(&self, cluster_id: i32) -> Result<Vec<crate::api::TopicResponse>, ApiError> {
        self.client.get(&format!("/api/clusters/{}/topics", cluster_id)).await
    }

    /// Get topic details
    pub async fn get(&self, cluster_id: i32, topic_name: &str) -> Result<crate::api::TopicResponse, ApiError> {
        self.client.get(&format!("/api/clusters/{}/topics/{}", cluster_id, topic_name)).await
    }

    /// Create topic
    pub async fn create(&self, cluster_id: i32, request: &crate::api::CreateTopicRequest) -> Result<crate::api::TopicResponse, ApiError> {
        self.client.post(&format!("/api/clusters/{}/topics", cluster_id), request).await
    }

    /// Delete topic
    pub async fn delete(&self, cluster_id: i32, topic_name: &str) -> Result<(), ApiError> {
        self.client.delete(&format!("/api/clusters/{}/topics/{}", cluster_id, topic_name)).await
    }

    /// Get topic config
    pub async fn get_config(&self, cluster_id: i32, topic_name: &str) -> Result<Vec<TopicConfigEntry>, ApiError> {
        self.client.get(&format!("/api/clusters/{}/topics/{}/config", cluster_id, topic_name)).await
    }
}

/// Topic config entry
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TopicConfigEntry {
    pub key: String,
    pub value: String,
    pub is_default: bool,
    pub description: Option<String>,
}

/// Message API operations
pub struct MessageApi {
    client: HttpClient,
}

impl MessageApi {
    /// Create new message API
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }

    /// Query messages
    pub async fn query(&self, cluster_id: i32, request: &crate::api::MessageQueryRequest) -> Result<Vec<crate::api::KafkaMessage>, ApiError> {
        self.client.post(&format!("/api/clusters/{}/messages/query", cluster_id), request).await
    }

    /// Send message
    pub async fn send(&self, cluster_id: i32, request: &crate::api::SendMessageRequest) -> Result<SendMessageResult, ApiError> {
        self.client.post(&format!("/api/clusters/{}/messages/send", cluster_id), request).await
    }

    /// Get SSE stream URL
    pub fn stream_url(&self, cluster_id: i32) -> String {
        format!("{}api/clusters/{}/messages/stream", self.client.base_url(), cluster_id)
    }
}

/// Send message result
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SendMessageResult {
    pub success: bool,
    pub partition: i32,
    pub offset: i64,
    pub timestamp: i64,
}

/// Consumer Group API operations
pub struct ConsumerGroupApi {
    client: HttpClient,
}

impl ConsumerGroupApi {
    /// Create new consumer group API
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }

    /// List consumer groups for cluster
    pub async fn list(&self, cluster_id: i32) -> Result<Vec<crate::api::ConsumerGroupResponse>, ApiError> {
        self.client.get(&format!("/api/clusters/{}/consumer-groups", cluster_id)).await
    }

    /// Get consumer group details
    pub async fn get(&self, cluster_id: i32, group_id: &str) -> Result<crate::api::ConsumerGroupResponse, ApiError> {
        self.client.get(&format!("/api/clusters/{}/consumer-groups/{}", cluster_id, group_id)).await
    }

    /// Get consumer groups for a topic
    pub async fn list_for_topic(&self, cluster_id: i32, topic_name: &str) -> Result<Vec<crate::api::ConsumerGroupResponse>, ApiError> {
        self.client.get(&format!("/api/clusters/{}/topics/{}/consumer-groups", cluster_id, topic_name)).await
    }
}

/// Schema Registry API operations
pub struct SchemaRegistryApi {
    client: HttpClient,
}

impl SchemaRegistryApi {
    /// Create new schema registry API
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }

    /// List subjects
    pub async fn list_subjects(&self, cluster_id: i32) -> Result<Vec<String>, ApiError> {
        self.client.get(&format!("/api/clusters/{}/schemas/subjects", cluster_id)).await
    }

    /// Get subject versions
    pub async fn get_subject_versions(&self, cluster_id: i32, subject: &str) -> Result<Vec<crate::api::SchemaVersion>, ApiError> {
        self.client.get(&format!("/api/clusters/{}/schemas/subjects/{}/versions", cluster_id, subject)).await
    }

    /// Register schema
    pub async fn register(&self, cluster_id: i32, request: &crate::api::RegisterSchemaRequest) -> Result<SchemaRegistrationResult, ApiError> {
        self.client.post(&format!("/api/clusters/{}/schemas", cluster_id), request).await
    }
}

/// Schema registration result
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SchemaRegistrationResult {
    pub id: i32,
    pub subject: String,
    pub version: i32,
}