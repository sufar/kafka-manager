use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    Extension,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// API Key 认证中间件
#[derive(Clone)]
pub struct AuthMiddleware {
    /// API Keys 集合
    pub api_keys: Arc<RwLock<Vec<String>>>,
    /// 是否启用认证
    pub enabled: bool,
}

impl AuthMiddleware {
    pub fn new(api_keys: Vec<String>, enabled: bool) -> Self {
        Self {
            api_keys: Arc::new(RwLock::new(api_keys)),
            enabled,
        }
    }

    /// 添加 API Key
    pub async fn add_api_key(&self, key: String) {
        let mut keys = self.api_keys.write().await;
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    /// 移除 API Key
    pub async fn remove_api_key(&self, key: &str) {
        let mut keys = self.api_keys.write().await;
        keys.retain(|k| k != key);
    }

    /// 验证 API Key
    pub async fn validate_key(&self, key: &str) -> bool {
        let keys = self.api_keys.read().await;
        keys.iter().any(|k| k == key)
    }
}

/// 默认状态，用于未启用认证时
impl Default for AuthMiddleware {
    fn default() -> Self {
        Self {
            api_keys: Arc::new(RwLock::new(vec![])),
            enabled: false,
        }
    }
}

/// 认证中间件处理函数
pub async fn auth_middleware(
    Extension(auth): Extension<AuthMiddleware>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 如果未启用认证，直接通过
    if !auth.enabled {
        return Ok(next.run(request).await);
    }

    // 从 Header 中获取 API Key
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    // 克隆 key 用于验证，避免借用冲突
    let key_clone = api_key.map(String::from);

    match key_clone {
        Some(key) => {
            if auth.validate_key(&key).await {
                // 认证成功，将用户信息注入到 request extensions
                let mut request = request;
                request.extensions_mut().insert(ApiKey(key));
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

/// API Key 扩展，用于在请求中传递认证信息
#[derive(Clone, Debug)]
pub struct ApiKey(pub String);

/// 跳过认证的路径列表
pub const SKIP_AUTH_PATHS: &[&str] = &[
    "/api/health",
    "/api/ready",
];

/// 检查路径是否需要认证
pub fn should_skip_auth(path: &str) -> bool {
    SKIP_AUTH_PATHS.contains(&path) || path.starts_with("/api/ready")
}
