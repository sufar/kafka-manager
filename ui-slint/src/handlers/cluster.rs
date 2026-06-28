// 集群事件处理器

use crate::models::ClusterData;
use kafka_manager_api::{AppState, db::cluster::ClusterStore};
use slint::{ComponentHandle, Weak, Model};
use tokio::sync::RwLock;
use std::sync::Arc;

/// 加载集群列表
pub async fn load_clusters(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
) {
    println!("Loading clusters...");

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 从数据库加载集群列表
    let clusters = match ClusterStore::list(state_guard.db.inner()).await {
        Ok(clusters) => clusters,
        Err(e) => {
            eprintln!("Failed to load clusters: {}", e);
            // 更新 UI 显示错误
            let _ = app.upgrade_in_event_loop(move |app| {
                app.set_window_title(format!("Error: {}", e).into());
            });
            return;
        }
    };

    println!("Loaded {} clusters from database", clusters.len());

    // 转换为 Slint 数据模型
    let cluster_data: Vec<crate::ClusterInfo> = clusters
        .into_iter()
        .map(|c| crate::ClusterInfo::from(ClusterData::from(c)))
        .collect();

    // 更新 UI（回到主线程）
    let _ = app.upgrade_in_event_loop(move |app| {
        use slint::{ModelRc, VecModel, Model};

        // 创建模型
        let model = VecModel::from(cluster_data);
        let model_rc = ModelRc::new(model);
        let count = model_rc.row_count();

        // 设置集群列表
        app.set_clusters(model_rc);

        println!("UI updated with {} clusters", count);
    });
}

/// 创建新集群
pub async fn create_cluster(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    name: String,
    brokers: String,
    request_timeout_ms: i32,
    operation_timeout_ms: i32,
    group_id: Option<i32>,
) {
    println!("Creating cluster: {}", name);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 创建请求
    let req = kafka_manager_api::db::cluster::CreateClusterRequest {
        name,
        brokers,
        request_timeout_ms: request_timeout_ms as i64,
        operation_timeout_ms: operation_timeout_ms as i64,
        group_id: group_id.map(|g| g as i64),
    };

    // 调用数据库创建
    let cluster = match ClusterStore::create(state_guard.db.inner(), &req).await {
        Ok(cluster) => cluster,
        Err(e) => {
            eprintln!("Failed to create cluster: {}", e);
            return;
        }
    };

    println!("Cluster created: {}", cluster.name);

    // 转换为 Slint 数据模型
    let cluster_data = ClusterData::from(cluster);

    // 更新 UI
    let _ = app.upgrade_in_event_loop(move |app| {
        // 注意：需要在 app.slint 中实现添加集群的逻辑
        // let clusters = app.get_clusters();
        // clusters.push(cluster_data);

        println!("UI updated with new cluster");
    });
}

/// 删除集群
pub async fn delete_cluster(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: i32,
) {
    println!("Deleting cluster: {}", cluster_id);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 调用数据库删除
    match ClusterStore::delete(state_guard.db.inner(), cluster_id as i64).await {
        Ok(_) => println!("Cluster deleted"),
        Err(e) => {
            eprintln!("Failed to delete cluster: {}", e);
            return;
        }
    }

    // 更新 UI
    let _ = app.upgrade_in_event_loop(move |app| {
        // 注意：需要在 app.slint 中实现删除集群的逻辑
        // let clusters = app.get_clusters();
        // 找到并删除对应的集群

        println!("UI updated after cluster deletion");
    });
}