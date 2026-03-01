//! Error 模块单元测试

use crate::error::AppError;

#[test]
fn test_error_display() {
    // 测试 BadRequest 错误
    let err = AppError::BadRequest("invalid input".to_string());
    assert_eq!(err.to_string(), "Invalid request: invalid input");

    // 测试 NotFound 错误
    let err = AppError::NotFound("Topic not found".to_string());
    assert_eq!(err.to_string(), "Not found: Topic not found");

    // 测试 Internal 错误
    let err = AppError::Internal("something went wrong".to_string());
    assert_eq!(err.to_string(), "Internal error: something went wrong");
}

#[test]
fn test_error_from_kafka_error() {
    // 测试从 KafkaError 转换 - 使用 ClientCreation 错误
    let kafka_err = rdkafka::error::KafkaError::ClientCreation("test error".to_string());
    let err: AppError = kafka_err.into();

    match err {
        AppError::Kafka(_) => {
            // 成功转换
        }
        _ => panic!("Expected Kafka error"),
    }
}

#[test]
fn test_error_from_config_error() {
    // 测试从 ConfigError 转换
    let config_err = config::ConfigError::NotFound("config file".into());
    let err: AppError = config_err.into();

    match err {
        AppError::Config(_) => {
            // 成功转换
        }
        _ => panic!("Expected Config error"),
    }
}
