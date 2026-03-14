pub mod audit_log;
pub mod auth;
pub mod cluster;
pub mod cluster_connection;
pub mod cluster_monitor;
pub mod consumer_group;
pub mod health;
pub mod notification;
pub mod schema;
pub mod settings;
pub mod tag;
pub mod topic;
pub mod topic_template;
pub mod unified;
pub mod user;

use crate::AppState;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    // 使用统一路由处理所有 API 请求
    // 所有请求都通过 POST /api 发送，method 放在 header 中，参数放在 body 中
    Router::new()
        .nest("/api", unified::routes().with_state(state))
}
