use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),

    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Bcrypt error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),

    #[error("Avro error: {0}")]
    Avro(#[from] apache_avro::Error),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Not connected: {0}")]
    NotConnected(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// 转换为前端展示用的错误消息（与原统一 API 错误响应格式保持一致）
    pub fn to_message(&self) -> String {
        match self {
            AppError::BadRequest(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::NotConnected(msg) => msg.clone(),
            AppError::Kafka(err) => format!("Kafka error: {}", err),
            AppError::Internal(msg) => msg.clone(),
            _ => self.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
