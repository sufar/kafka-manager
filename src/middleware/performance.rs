/// 慢查询日志中间件
///
/// 记录超过阈值的请求，帮助识别性能问题

use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{warn, info};

/// 慢查询阈值（毫秒）
const SLOW_QUERY_THRESHOLD_MS: u128 = 1000;

/// 极慢查询阈值（毫秒）
const VERY_SLOW_QUERY_THRESHOLD_MS: u128 = 5000;

/// 性能追踪中间件
pub async fn performance_tracker(
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;
    let duration = start.elapsed();
    let duration_ms = duration.as_millis();
    let status = response.status();

    // 记录慢查询
    if duration_ms >= VERY_SLOW_QUERY_THRESHOLD_MS {
        warn!(
            target: "slow_query",
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration_ms,
            "VERY SLOW request detected (>{}ms)",
            VERY_SLOW_QUERY_THRESHOLD_MS
        );
    } else if duration_ms >= SLOW_QUERY_THRESHOLD_MS {
        warn!(
            target: "slow_query",
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration_ms,
            "Slow request detected (>{}ms)",
            SLOW_QUERY_THRESHOLD_MS
        );
    } else {
        info!(
            target: "request_timing",
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration_ms,
            "Request completed"
        );
    }

    response
}

/// 慢查询日志配置
#[derive(Debug, Clone)]
pub struct SlowQueryConfig {
    /// 慢查询阈值（毫秒）
    pub threshold_ms: u128,
    /// 是否启用日志
    pub enabled: bool,
}

impl Default for SlowQueryConfig {
    fn default() -> Self {
        Self {
            threshold_ms: SLOW_QUERY_THRESHOLD_MS,
            enabled: true,
        }
    }
}
