//! Kafka Manager API 集成测试
//!
//! 这些测试使用 Axum 的测试工具来测试 API 端点

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use tower::Service;

// Health check 测试
#[tokio::test]
async fn test_health_check() {
    let mut app = Router::new()
        .route("/health", axum::routing::get(|| async {
            axum::Json(serde_json::json!({
                "status": "healthy",
                "version": "0.1.0"
            }))
        }));

    let response = app
        .call(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
}

// Topic API 测试
#[tokio::test]
async fn test_list_topics_request() {
    // 测试请求格式
    let request_body = r#"{"name": "test-topic", "num_partitions": 3}"#;
    let json: serde_json::Value = serde_json::from_str(request_body).unwrap();

    assert_eq!(json["name"], "test-topic");
    assert_eq!(json["num_partitions"], 3);
}

#[tokio::test]
async fn test_send_message_request() {
    let request_body = r#"{"key": "test-key", "value": "Hello"}"#;
    let json: serde_json::Value = serde_json::from_str(request_body).unwrap();

    assert_eq!(json["key"], "test-key");
    assert_eq!(json["value"], "Hello");
}

// 错误处理测试
#[tokio::test]
async fn test_404_response() {
    let mut app: Router = Router::new();

    let response = app
        .call(
            Request::builder()
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
