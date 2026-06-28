// Topic CRUD 操作处理器（Phase 8）

use kafka_manager_api::{AppState, kafka::admin::KafkaAdmin};
use slint::{ComponentHandle, Weak, Model, ModelRc, VecModel};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

/// 创建 Topic
pub async fn create_topic(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    topic_name: String,
    num_partitions: i32,
    replication_factor: i32,
) {
    println!("Creating topic: cluster={}, name={}, partitions={}, replication={}",
        cluster_id, topic_name, num_partitions, replication_factor);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Admin 客户端
    let clients = state_guard.get_clients();
    let admin = clients.get_admin(&cluster_id);

    if admin.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        let _ = app.upgrade_in_event_loop(move |app| {
            // TODO: 显示错误提示
            println!("Failed to create topic: cluster not connected");
        });
        return;
    }

    let admin = admin.unwrap();

    // 创建 Topic（使用默认配置）
    let config = HashMap::new();  // 可以添加自定义配置参数

    let result = admin.create_topic(
        &topic_name,
        num_partitions,
        replication_factor,
        config,
    ).await;

    match result {
        Ok(_) => {
            println!("Topic {} created successfully", topic_name);

            let _ = app.upgrade_in_event_loop(move |app| {
                // TODO: 显示成功提示
                println!("Topic created: {}", topic_name);

                // 刷新 topic 列表（触发重新加载）
                // app.set_selected-cluster(cluster_id);
                // app.on_load_topics();
            });
        }
        Err(e) => {
            eprintln!("Failed to create topic: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                // TODO: 显示错误提示
                println!("Failed to create topic: {}", e);
            });
        }
    }
}

/// 删除 Topic
pub async fn delete_topic(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    topic_name: String,
) {
    println!("Deleting topic: cluster={}, name={}", cluster_id, topic_name);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Admin 客户端
    let clients = state_guard.get_clients();
    let admin = clients.get_admin(&cluster_id);

    if admin.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        let _ = app.upgrade_in_event_loop(move |app| {
            println!("Failed to delete topic: cluster not connected");
        });
        return;
    }

    let admin = admin.unwrap();

    // 删除 Topic
    let result = admin.delete_topic(&topic_name).await;

    match result {
        Ok(_) => {
            println!("Topic {} deleted successfully", topic_name);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Topic deleted: {}", topic_name);

                // 刷新 topic 列表
                // app.on_load_topics();
            });
        }
        Err(e) => {
            eprintln!("Failed to delete topic: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to delete topic: {}", e);
            });
        }
    }
}

/// 获取 Topic 配置
pub async fn get_topic_config(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    topic_name: String,
) {
    println!("Getting topic config: cluster={}, name={}", cluster_id, topic_name);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Admin 客户端
    let clients = state_guard.get_clients();
    let admin = clients.get_admin(&cluster_id);

    if admin.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        let _ = app.upgrade_in_event_loop(move |app| {
            println!("Failed to get topic config: cluster not connected");
        });
        return;
    }

    let admin = admin.unwrap();

    // 获取 Topic 配置
    let result = admin.get_topic_config(&topic_name).await;

    match result {
        Ok(config) => {
            println!("Topic config fetched successfully");

            let _ = app.upgrade_in_event_loop(move |app| {
                // TODO: 显示配置对话框
                println!("Topic config: {:?}", config);
            });
        }
        Err(e) => {
            eprintln!("Failed to get topic config: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to get topic config: {}", e);
            });
        }
    }
}

/// 修改 Topic 配置
pub async fn alter_topic_config(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    topic_name: String,
    config: HashMap<String, String>,
) {
    println!("Altering topic config: cluster={}, name={}", cluster_id, topic_name);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Admin 客户端
    let clients = state_guard.get_clients();
    let admin = clients.get_admin(&cluster_id);

    if admin.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        let _ = app.upgrade_in_event_loop(move |app| {
            println!("Failed to alter topic config: cluster not connected");
        });
        return;
    }

    let admin = admin.unwrap();

    // 修改 Topic 配置
    let result = admin.alter_topic_config(&topic_name, config).await;

    match result {
        Ok(_) => {
            println!("Topic config altered successfully");

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Topic config altered: {}", topic_name);
            });
        }
        Err(e) => {
            eprintln!("Failed to alter topic config: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to alter topic config: {}", e);
            });
        }
    }
}