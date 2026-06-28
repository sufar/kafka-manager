// Consumer Group 详情处理器（Phase 8）

use kafka_manager_api::{AppState, kafka::consumer_group::{KafkaConsumerGroupManager, PartitionOffsetDetail}};
use slint::{ComponentHandle, Weak, Model, ModelRc, VecModel};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Consumer Group 成员信息（Slint 数据结构）
#[derive(Clone, Debug, Default)]
pub struct ConsumerGroupMemberInfo {
    pub client_id: String,
    pub host: String,
    pub topic_partitions: Vec<(String, i32)>,
}

/// Consumer Group Lag 详情（Slint 数据结构）
#[derive(Clone, Debug, Default)]
pub struct ConsumerGroupLagInfo {
    pub topic: String,
    pub partition: i32,
    pub start_offset: i64,
    pub end_offset: i64,
    pub committed_offset: i64,
    pub lag: i64,
}

/// 获取 Consumer Group 成员详情
pub async fn get_consumer_group_members(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    group_id: String,
) {
    println!("Getting consumer group members: cluster={}, group={}", cluster_id, group_id);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Consumer Group Manager
    let clients = state_guard.get_clients();
    let config = clients.get_config(&cluster_id);

    if config.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        let _ = app.upgrade_in_event_loop(move |app| {
            println!("Failed to get consumer group members: cluster not connected");
        });
        return;
    }

    let config = config.unwrap();
    let manager = match KafkaConsumerGroupManager::new(&config) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to create ConsumerGroupManager: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to create ConsumerGroupManager");
            });
            return;
        }
    };

    // 获取 Consumer Group 详情（包含成员信息）
    let result = manager.get_single_consumer_group(&group_id);

    match result {
        Ok((state_str, topics)) => {
            println!("Consumer group state: {}, topics: {:?}", state_str, topics);

            // TODO: 获取成员详情（需要扩展 KafkaConsumerGroupManager API）
            // 目前返回简化信息

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Consumer group details fetched");
                // TODO: 显示详情UI
            });
        }
        Err(e) => {
            eprintln!("Failed to get consumer group members: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to get consumer group members: {}", e);
            });
        }
    }
}

/// 获取 Consumer Group Lag 详情
pub async fn get_consumer_group_lag(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    group_id: String,
    topic: String,
) {
    println!("Getting consumer group lag: cluster={}, group={}, topic={}", cluster_id, group_id, topic);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Consumer Group Manager
    let clients = state_guard.get_clients();
    let config = clients.get_config(&cluster_id);

    if config.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        let _ = app.upgrade_in_event_loop(move |app| {
            println!("Failed to get consumer group lag: cluster not connected");
        });
        return;
    }

    let config = config.unwrap();
    let manager = match KafkaConsumerGroupManager::new(&config) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to create ConsumerGroupManager: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to create ConsumerGroupManager");
            });
            return;
        }
    };

    // 获取 Consumer Group 在指定 topic 上的 Lag 详情
    let result = if topic.is_empty() {
        // 自动获取所有 topic 的 lag
        manager.get_consumer_group_offsets_auto(&group_id)
    } else {
        // 获取指定 topic 的 lag
        manager.get_consumer_group_offsets_for_topic(&group_id, &topic)
    };

    match result {
        Ok(lag_details) => {
            println!("Consumer group lag details fetched: {} partitions", lag_details.len());

            // 转换为 Slint 数据结构
            let lag_data: Vec<ConsumerGroupLagInfo> = lag_details
                .into_iter()
                .map(|d| ConsumerGroupLagInfo {
                    topic: d.topic,
                    partition: d.partition,
                    start_offset: d.start_offset,
                    end_offset: d.end_offset,
                    committed_offset: d.committed_offset,
                    lag: d.lag,
                })
                .collect();

            let _ = app.upgrade_in_event_loop(move |app| {
                // TODO: 显示 Lag 详情 UI
                println!("Lag details: {:?}", lag_data);
            });
        }
        Err(e) => {
            eprintln!("Failed to get consumer group lag: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to get consumer group lag: {}", e);
            });
        }
    }
}

/// 获取 Consumer Group 总 Lag 统计
pub async fn get_consumer_group_total_lag(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    group_id: String,
) {
    println!("Getting consumer group total lag: cluster={}, group={}", cluster_id, group_id);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Consumer Group Manager
    let clients = state_guard.get_clients();
    let config = clients.get_config(&cluster_id);

    if config.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        let _ = app.upgrade_in_event_loop(move |app| {
            println!("Failed to get consumer group total lag: cluster not connected");
        });
        return;
    }

    let config = config.unwrap();
    let manager = match KafkaConsumerGroupManager::new(&config) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to create ConsumerGroupManager: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to create ConsumerGroupManager");
            });
            return;
        }
    };

    // 获取 Consumer Group 的所有 Lag 详情
    let result = manager.get_consumer_group_offsets_auto(&group_id);

    match result {
        Ok(lag_details) => {
            // 计算总 Lag
            let total_lag: i64 = lag_details.iter().map(|d| d.lag).sum();

            println!("Consumer group total lag: {}", total_lag);

            let _ = app.upgrade_in_event_loop(move |app| {
                // TODO: 更新 UI 显示总 Lag
                println!("Total lag: {}", total_lag);
            });
        }
        Err(e) => {
            eprintln!("Failed to get consumer group total lag: {}", e);

            let _ = app.upgrade_in_event_loop(move |app| {
                println!("Failed to get consumer group total lag: {}", e);
            });
        }
    }
}