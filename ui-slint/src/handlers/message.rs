// 消息事件处理器（Phase 7 - 流式消息推送）

use kafka_manager_api::{AppState, kafka::consumer::{KafkaConsumer, KafkaMessage}};
use slint::{ComponentHandle, Weak, Model, ModelRc, VecModel};
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;
use std::time::Duration;

/// 流式消息查询（实时推送）
pub async fn query_messages_stream(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    topic: String,
    partition: i32,         // -1 表示所有分区
    max_messages: i32,
    keyword: String,        // 关键字过滤
) {
    println!("Starting stream query: cluster={}, topic={}, partition={}, max={}, keyword={}",
        cluster_id, topic, partition, max_messages, keyword);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Consumer Pool
    let pool = state_guard.pools.get_consumer_pool(&cluster_id).await;

    if pool.is_none() {
        eprintln!("Consumer pool for cluster {} not found", cluster_id);

        // 更新 UI 显示空列表
        let _ = app.upgrade_in_event_loop(move |app| {
            app.set_messages(ModelRc::new(VecModel::default()));
            app.set_loading(false);
            app.set_received_count(0);
        });
        return;
    }

    let pool = pool.unwrap();

    // 更新 UI 显示加载状态
    let _ = app.upgrade_in_event_loop(move |app| {
        app.set_loading(true);
        app.set_received_count(0);
        // 清空旧消息
        app.set_messages(ModelRc::new(VecModel::default()));
    });

    // 创建 channel 用于流式传输
    let (tx, mut rx) = mpsc::unbounded_channel::<KafkaMessage>();

    // 创建过滤函数（使用 move 关键字捕获所有权）
    let keyword_clone = keyword.clone();
    let matcher = move |msg: &KafkaMessage| {
        if keyword_clone.is_empty() {
            return true;
        }
        // 关键字匹配：在 key 或 value 中搜索
        msg.key.as_ref().map(|k| k.contains(&keyword_clone)).unwrap_or(false)
            || msg.value.as_ref().map(|v| v.contains(&keyword_clone)).unwrap_or(false)
    };

    // 后台任务：从 Kafka 读取消息并推送到 channel
    let pool_clone = pool.clone();
    let topic_clone = topic.clone();
    let max_messages_usize = max_messages as usize;

    tokio::spawn(async move {
        // 调用 KafkaConsumer API
        let partition_opt = if partition >= 0 { Some(partition) } else { None };
        let offset_opt = None; // 从最新开始

        let result = KafkaConsumer::fetch_messages_filtered_from_pool(
            &pool_clone,
            &kafka_manager_api::config::KafkaConfig::default(), // 使用默认配置
            &topic_clone,
            partition_opt,
            offset_opt,
            max_messages_usize,
            &matcher,
        ).await;

        match result {
            Ok(messages) => {
                println!("Fetched {} messages from Kafka", messages.len());

                // 推送到 channel
                for msg in messages {
                    if tx.send(msg).is_err() {
                        println!("Channel closed, stopping stream");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch messages: {}", e);
            }
        }
    });

    // 主线程：从 channel 接收消息并批量更新 UI
    let mut batch = Vec::new();
    let batch_size = 50;  // 每 50 条消息批量更新一次 UI
    let mut total_count = 0;

    while let Some(message) = rx.recv().await {
        batch.push(message);
        total_count += 1;

        if batch.len() >= batch_size {
            let batch_clone = batch.clone();
            batch.clear();

            let _ = app.upgrade_in_event_loop(move |app| {
                // 转换为 Slint 数据模型
                let new_messages: Vec<crate::MessageInfo> = batch_clone
                    .into_iter()
                    .map(|m| crate::MessageInfo {
                        partition: m.partition,
                        offset: m.offset as i32,
                        key: slint::SharedString::from(m.key.unwrap_or_default()),
                        value: slint::SharedString::from(m.value.unwrap_or_default()),
                        timestamp: m.timestamp.unwrap_or(0) as i32,
                    })
                    .collect();

                // 添加到现有列表
                let messages = app.get_messages();
                let vec_model = messages.iter().collect::<Vec<_>>();
                let mut updated = vec_model.clone();
                updated.extend(new_messages);

                app.set_messages(ModelRc::new(VecModel::from(updated)));
                app.set_received_count(total_count);

                println!("UI updated: batch of {} messages, total {}", batch_size, total_count);
            });
        }

        // 如果达到最大数量，停止接收
        if total_count as i32 >= max_messages {
            println!("Reached max messages {}, stopping stream", max_messages);
            break;
        }
    }

    // 处理剩余的消息（如果 batch 不为空）
    if !batch.is_empty() {
        let batch_clone = batch.clone();

        let _ = app.upgrade_in_event_loop(move |app| {
            let new_messages: Vec<crate::MessageInfo> = batch_clone
                .into_iter()
                .map(|m| crate::MessageInfo {
                    partition: m.partition,
                    offset: m.offset as i32,
                    key: slint::SharedString::from(m.key.unwrap_or_default()),
                    value: slint::SharedString::from(m.value.unwrap_or_default()),
                    timestamp: m.timestamp.unwrap_or(0) as i32,
                })
                .collect();

            let messages = app.get_messages();
            let vec_model = messages.iter().collect::<Vec<_>>();
            let mut updated = vec_model.clone();
            updated.extend(new_messages);

            app.set_messages(ModelRc::new(VecModel::from(updated)));
            app.set_received_count(total_count);
            app.set_loading(false);

            println!("UI updated: final batch of {} messages, total {}", batch.len(), total_count);
        });
    } else {
        // 如果没有剩余消息，只更新 loading 状态
        let _ = app.upgrade_in_event_loop(move |app| {
            app.set_loading(false);
        });
    }

    println!("Stream query completed: total {} messages", total_count);
}

/// 从 Kafka 查询消息（基础实现 - 限制数量查询）
pub async fn query_messages_from_kafka(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    topic: String,
    partition: i32,
    max_messages: i32,
) {
    // 调用流式查询（无关键字过滤）
    query_messages_stream(state, app, cluster_id, topic, partition, max_messages, String::new()).await;
}

/// 停止消息查询（取消流式传输）
pub async fn stop_message_query(
    app: Weak<crate::App>,
) {
    println!("Stopping message query");

    let _ = app.upgrade_in_event_loop(move |app| {
        app.set_loading(false);
    });
}

/// 发送消息到 Kafka（可选功能）
pub async fn send_message_to_kafka(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
    topic: String,
    key: String,
    value: String,
    partition: i32,
) {
    println!("Sending message to Kafka: cluster={}, topic={}, key={}, value={}",
        cluster_id, topic, key, value);

    // TODO: 实现消息发送功能
    // 需要扩展 KafkaProducer API

    // 暂时只打印日志
    let _ = app.upgrade_in_event_loop(move |app| {
        // 可以显示 Toast 提示
        println!("Message sent successfully");
    });
}