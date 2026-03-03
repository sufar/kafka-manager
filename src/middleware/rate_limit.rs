/// 请求限流中间件
///
/// 使用简单的令牌桶算法实现限流

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// 限流中间件配置
#[derive(Clone)]
pub struct RateLimitConfig {
    /// 每秒允许的请求数
    pub requests_per_second: u32,
    /// 是否启用限流
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            enabled: true,
        }
    }
}

/// 限流器状态
struct RateLimiterState {
    /// 当前令牌数
    tokens: AtomicU32,
    /// 上次刷新时间
    last_refill: Mutex<Instant>,
    /// 每秒请求数限制
    max_requests_per_second: u32,
}

/// 限流中间件状态
#[derive(Clone)]
pub struct RateLimitMiddleware {
    state: std::sync::Arc<RateLimiterState>,
    config: RateLimitConfig,
}

impl RateLimitMiddleware {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            state: std::sync::Arc::new(RateLimiterState {
                tokens: AtomicU32::new(config.requests_per_second),
                last_refill: Mutex::new(Instant::now()),
                max_requests_per_second: config.requests_per_second,
            }),
            config,
        }
    }

    /// 尝试获取一个令牌
    pub fn try_acquire(&self) -> bool {
        let state = &self.state;

        // 尝试减少令牌
        let mut current_tokens = state.tokens.load(Ordering::Relaxed);

        while current_tokens > 0 {
            match state.tokens.compare_exchange_weak(
                current_tokens,
                current_tokens - 1,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(new_value) => current_tokens = new_value,
            }
        }

        false
    }

    /// 刷新令牌
    async fn refill_tokens(&self) {
        let mut last_refill = self.state.last_refill.lock().await;
        let now = Instant::now();

        if now.duration_since(*last_refill) >= Duration::from_secs(1) {
            // 重置令牌
            self.state.tokens.store(
                self.state.max_requests_per_second,
                Ordering::Relaxed,
            );
            *last_refill = now;
        }
    }
}

/// 限流中间件函数
pub async fn rate_limit_middleware(
    State(middleware): State<RateLimitMiddleware>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if !middleware.config.enabled {
        return Ok(next.run(req).await);
    }

    // 先尝试获取令牌
    if middleware.try_acquire() {
        return Ok(next.run(req).await);
    }

    // 没有令牌时，尝试刷新后再获取
    middleware.refill_tokens().await;

    if middleware.try_acquire() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

/// 创建限流中间件配置
pub fn rate_limit_config() -> RateLimitMiddleware {
    // 从环境变量读取限流配置
    let requests_per_second = std::env::var("RATE_LIMIT_REQUESTS_PER_SECOND")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let enabled = std::env::var("RATE_LIMIT_ENABLED")
        .map(|s| s.to_lowercase() == "true")
        .unwrap_or(true);

    let config = RateLimitConfig {
        requests_per_second,
        enabled,
    };

    RateLimitMiddleware::new(config)
}
