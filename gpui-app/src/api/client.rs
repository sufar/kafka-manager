//! API Client
//!
//! HTTP client for communicating with the Kafka Manager backend.

use serde::de::DeserializeOwned;

/// API client configuration
#[derive(Clone, Debug)]
pub struct ApiClient {
    base_url: String,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Create client with default base URL
    pub fn default_client() -> Self {
        Self::new("http://localhost:8080".to_string())
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Make a GET request (placeholder - returns mock data)
    #[allow(dead_code)]
    pub async fn get<T: DeserializeOwned>(&self, _path: &str) -> Result<T, ApiError> {
        // Placeholder implementation - in real app would use reqwest
        Err(ApiError::Http(404))
    }

    /// Make a POST request (placeholder)
    #[allow(dead_code)]
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(&self, _path: &str, _body: &B) -> Result<T, ApiError> {
        Err(ApiError::Http(404))
    }

    /// Make a PUT request (placeholder)
    #[allow(dead_code)]
    pub async fn put<T: DeserializeOwned, B: serde::Serialize>(&self, _path: &str, _body: &B) -> Result<T, ApiError> {
        Err(ApiError::Http(404))
    }

    /// Make a DELETE request (placeholder)
    #[allow(dead_code)]
    pub async fn delete(&self, _path: &str) -> Result<(), ApiError> {
        Err(ApiError::Http(404))
    }
}

/// API error type
#[derive(Debug, Clone, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(u16),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parse error: {0}")]
    Parse(String),
}