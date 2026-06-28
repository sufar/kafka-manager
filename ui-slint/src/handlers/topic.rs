// Topic 事件处理器

use kafka_manager_api::{AppState};
use slint::{Weak, Model};
use tokio::sync::RwLock;
use std::sync::Arc;

/// 加载 Topic 列表（从数据库 - 仅用于元数据）
pub async fn load_topics_from_db(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
) {
    println!("Loading topics from database for cluster: {}", cluster_id);

    // 注意：这个函数主要用于加载 Topic 的元数据（如收藏状态）
    // 实际的 Topic 列表应该从 Kafka 加载（使用 load_topics_from_kafka）

    // 暂时返回空列表
    println!("Database topic loading not implemented - use load_topics_from_kafka");

    let _ = app.upgrade_in_event_loop(move |app| {
        use slint::{ModelRc, VecModel};
        app.set_topics(ModelRc::new(VecModel::default()));
    });
}

/// 从 Kafka 加载实时 Topic 列表（核心实现）
pub async fn load_topics_from_kafka(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
) {
    println!("Loading topics from Kafka for cluster: {}", cluster_id);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Admin 客户端
    let clients = state_guard.get_clients();
    let admin = clients.get_admin(&cluster_id);

    if admin.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        // 更新 UI 显示空列表
        let _ = app.upgrade_in_event_loop(move |app| {
            use slint::{ModelRc, VecModel};
            app.set_topics(ModelRc::new(VecModel::default()));
        });
        return;
    }

    let admin = admin.unwrap();

    // 使用 Kafka AdminClient 获取 Topic 列表（包含分区数）
    let topics_with_partitions = match admin.list_topics_with_partitions() {
        Ok(topics) => topics,
        Err(e) => {
            eprintln!("Failed to load topics from Kafka: {}", e);

            // 更新 UI 显示空列表
            let _ = app.upgrade_in_event_loop(move |app| {
                use slint::{ModelRc, VecModel};
                app.set_topics(ModelRc::new(VecModel::default()));
            });
            return;
        }
    };

    println!("Loaded {} topics from Kafka", topics_with_partitions.len());

    // 转换为 Slint 数据模型
    let topic_data: Vec<crate::TopicInfo> = topics_with_partitions
        .into_iter()
        .map(|(name, partitions)| crate::TopicInfo {
            name: slint::SharedString::from(name),
            cluster_id: slint::SharedString::from(cluster_id.clone()),
            partitions_count: partitions as i32,
            replication_factor: 1,  // AdminClient 不提供副本数，默认 1
            is_internal: false,     // list_topics_with_partitions 已过滤内部 topic
        })
        .collect();

    // 更新 UI（回到主线程）
    let _ = app.upgrade_in_event_loop(move |app| {
        use slint::{ModelRc, VecModel, Model};

        let model = VecModel::from(topic_data);
        let model_rc = ModelRc::new(model);
        let count = model_rc.row_count();

        app.set_topics(model_rc);

        println!("UI updated with {} topics", count);
    });
}