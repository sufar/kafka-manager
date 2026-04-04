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
    /// 将错误转换为状态码和消息
    pub fn to_status_and_message(&self) -> (u16, String) {
        use tracing;

        match self {
            AppError::Kafka(e) => {
                tracing::error!("Kafka error: {}", e);
                (500, e.to_string())
            }
            AppError::Config(e) => {
                tracing::error!("Config error: {}", e);
                (500, "Configuration error".to_string())
            }
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (500, "Database error".to_string())
            }
            AppError::Http(e) => {
                tracing::error!("HTTP error: {}", e);
                (500, "HTTP request error".to_string())
            }
            AppError::Json(e) => {
                tracing::error!("JSON error: {}", e);
                (400, "JSON parse error".to_string())
            }
            AppError::Io(e) => {
                tracing::error!("IO error: {}", e);
                (500, "IO error".to_string())
            }
            AppError::Bcrypt(e) => {
                tracing::error!("Bcrypt error: {}", e);
                (500, "Encryption error".to_string())
            }
            AppError::Avro(e) => {
                tracing::error!("Avro error: {}", e);
                (400, format!("Avro error: {}", e))
            }
            AppError::BadRequest(msg) => (400, msg.clone()),
            AppError::NotFound(msg) => (404, msg.clone()),
            AppError::NotConnected(msg) => (428, msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (500, msg.clone())
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
