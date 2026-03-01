pub mod audit_log;
pub mod auth;
pub mod cluster;
pub mod cluster_connection;
pub mod cluster_monitor;
pub mod cluster_stats;
pub mod consumer_group;
pub mod health;
pub mod message;
pub mod notification;
pub mod schema;
pub mod settings;
pub mod tag;
pub mod topic;
pub mod topic_template;
pub mod user;

use crate::AppState;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    // 集群级别的路由（需要 cluster_id 参数）
    // 注意：在 Axum 中，路由匹配优先级基于注册顺序
    // 静态路径（如/_*）必须先注册，动态路径（如/:name）后注册
    let cluster_routes = Router::new()
        // 1. 先注册操作型路由（静态路径 /_*）
        .nest(
            "/:cluster_id/topics",
            topic::topic_operation_routes().with_state(state.clone()),
        )
        .nest(
            "/:cluster_id/consumer-groups",
            consumer_group::consumer_group_operation_routes().with_state(state.clone()),
        )
        // 2. 再注册资源路由（动态路径 /:name）
        .nest(
            "/:cluster_id/topics",
            topic::cluster_routes().with_state(state.clone()),
        )
        .nest(
            "/:cluster_id/consumer-groups",
            consumer_group::cluster_routes().with_state(state.clone()),
        )
        .nest(
            "/:cluster_id",
            message::routes().with_state(state.clone()),
        )
        .nest(
            "/:cluster_id",
            cluster_monitor::routes().with_state(state.clone()),
        )
        .nest(
            "/:cluster_id",
            cluster_stats::routes().with_state(state.clone()),
        );

    // 顶层路由（集群管理、健康检查、认证等）
    Router::new()
        .nest("/api/clusters", cluster_routes)
        .nest(
            "/api/clusters",
            cluster::routes().with_state(state.clone()),
        )
        .nest("/api/cluster-connections", cluster_connection::routes().with_state(state.clone()))
        .nest("/api/auth/keys", auth::routes().with_state(state.clone()))
        .nest("/api/audit-logs", audit_log::routes().with_state(state.clone()))
        .nest("/api/clusters/:cluster_id", tag::routes().with_state(state.clone()))
        .nest("/api", topic_template::routes().with_state(state.clone()))
        .nest("/api/tasks", crate::task::routes().with_state(state.clone()))
        // 用户和权限管理
        .nest("/api/users", user::user_routes().with_state(state.clone()))
        .nest("/api/roles", user::role_routes().with_state(state.clone()))
        // 通知管理
        .nest("/api/notifications", notification::routes().with_state(state.clone()))
        // Schema Registry 管理
        .nest("/api/schema-registry", schema::routes().with_state(state.clone()))
        // Topic 全局搜索（不需要 cluster_id）
        .nest("/api/topics", topic::global_routes().with_state(state.clone()))
        // 全局设置
        .nest("/api/settings", settings::routes().with_state(state.clone()))
        .nest("/api", health::routes())
}
