use axum::{
    extract::MatchedPath,
    http::Request,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn};

/// 审计日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    /// 操作时间
    pub timestamp: chrono::DateTime<Utc>,
    /// 操作类型 (GET/POST/PUT/DELETE)
    pub method: String,
    /// 请求路径
    pub path: String,
    /// 集群 ID (如果 applicable)
    pub cluster_id: Option<String>,
    /// 操作对象 (topic name, consumer group name 等)
    pub resource: Option<String>,
    /// 操作类型描述
    pub action: String,
    /// API Key (脱敏)
    pub api_key: Option<String>,
    /// 响应状态码
    pub status: u16,
    /// 请求耗时 (毫秒)
    pub duration_ms: u128,
    /// 客户端 IP
    pub client_ip: Option<String>,
}

/// 审计日志中间件处理函数
pub async fn audit_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    // 获取客户端 IP
    let client_ip = request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).to_string())
        .or_else(|| {
            request
                .headers()
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        });

    // 获取 API Key (脱敏)
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|k| mask_api_key(k));

    // 提取集群 ID 和资源信息
    let (cluster_id, resource) = extract_path_info(&path);

    // 确定操作类型
    let action = determine_action(&method, &path);

    // 执行请求
    let response = next.run(request).await;
    let duration_ms = start.elapsed().as_millis();
    let status = response.status().as_u16();

    // 记录审计日志
    let log = AuditLog {
        timestamp: Utc::now(),
        method: method.to_string(),
        path: path.clone(),
        cluster_id,
        resource,
        action,
        api_key,
        status,
        duration_ms,
        client_ip,
    };

    // 根据状态码决定日志级别
    if status >= 500 {
        warn!("Audit log: {:?}", log);
    } else {
        info!("Audit log: {} {} -> {} ({}ms)", log.method, log.path, log.status, log.duration_ms);
    }

    response
}

/// 从路径中提取集群 ID 和资源信息
fn extract_path_info(path: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // 路径格式：/api/clusters/:cluster_id/...
    let mut cluster_id = None;
    let mut resource = None;

    for (i, part) in parts.iter().enumerate() {
        if *part == "clusters" && i + 1 < parts.len() {
            // 下一个部分是集群 ID（如果不是路径关键词）
            let next = parts.get(i + 1);
            if !matches!(next, Some(&"topics" | &"consumer-groups" | &"brokers" | &"info" | &"metrics")) {
                cluster_id = next.cloned().map(|s| s.to_string());
            }
        }

        // 提取资源名称
        if *part == "topics" && i + 1 < parts.len() {
            let next = parts.get(i + 1);
            if !matches!(next, Some(&"config" | &"offsets" | &"partitions" | &"messages")) {
                resource = next.map(|s| format!("topic:{}", s));
            }
        }

        if *part == "consumer-groups" && i + 1 < parts.len() {
            let next = parts.get(i + 1);
            if !matches!(next, Some(&"offsets" | &"offsets-reset")) {
                resource = next.map(|s| format!("consumer-group:{}", s));
            }
        }
    }

    (cluster_id, resource)
}

/// 根据方法和路径确定操作类型
fn determine_action(method: &axum::http::Method, path: &str) -> String {
    let method_str = method.to_string();

    if path.contains("/test") {
        return "test_connection".to_string();
    }

    if path.contains("/offsets/reset") {
        return "reset_offset".to_string();
    }

    if path.contains("/offsets") {
        return "query_offset".to_string();
    }

    if path.contains("/config") {
        if method_str == "GET" {
            return "get_config".to_string();
        } else if method_str == "POST" {
            return "update_config".to_string();
        }
    }

    match method_str.as_str() {
        "GET" => {
            if path.contains("/brokers") {
                "list_brokers".to_string()
            } else if path.contains("/topics") {
                "list_topics".to_string()
            } else if path.contains("/consumer-groups") {
                "list_consumer_groups".to_string()
            } else if path.contains("/messages") {
                "query_messages".to_string()
            } else if path.contains("/info") {
                "get_cluster_info".to_string()
            } else if path.contains("/metrics") {
                "get_metrics".to_string()
            } else if path.contains("/clusters") {
                "get_cluster".to_string()
            } else {
                "query".to_string()
            }
        }
        "POST" => {
            if path.contains("/clusters") {
                "create_cluster".to_string()
            } else if path.contains("/topics") {
                "create_topic_or_send_message".to_string()
            } else if path.contains("/consumer-groups") {
                "consumer_group_operation".to_string()
            } else {
                "create".to_string()
            }
        }
        "PUT" => "update".to_string(),
        "DELETE" => {
            if path.contains("/clusters") {
                "delete_cluster".to_string()
            } else if path.contains("/topics") {
                "delete_topic".to_string()
            } else if path.contains("/consumer-groups") {
                "delete_consumer_group".to_string()
            } else {
                "delete".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

/// 脱敏 API Key
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "***".to_string()
    } else {
        format!("{}***{}", &key[..4], &key[key.len() - 4..])
    }
}

/// 审计日志存储 trait
pub trait AuditLogStore: Send + Sync {
    /// 保存审计日志
    fn save(&self, log: AuditLog) -> impl std::future::Future<Output = ()> + Send;
}

/// 内存审计日志存储（用于测试）
#[derive(Default)]
pub struct MemoryAuditLogStore {
    logs: std::sync::Mutex<Vec<AuditLog>>,
}

impl MemoryAuditLogStore {
    pub fn get_logs(&self) -> Vec<AuditLog> {
        self.logs.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn clear(&self) {
        let _ = self.logs.lock().map(|mut g| g.clear());
    }
}

impl AuditLogStore for MemoryAuditLogStore {
    async fn save(&self, log: AuditLog) {
        let _ = self.logs.lock().map(|mut g| g.push(log));
    }
}
