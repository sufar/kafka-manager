// Kafka Manager - Slint Desktop Application
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cache;
mod config;
mod db;
mod error;
mod kafka;
mod models;
mod pool;
mod ui;
mod utils;

use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashSet;
use arc_swap::ArcSwap;
use slint::ComponentHandle;
use slint::Model;

use crate::config::Config;
use crate::db::DbPool;
use crate::kafka::KafkaClients;
use crate::pool::ClusterPools;
use crate::cache::MetadataCache;
use std::time::Instant;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub clients: Arc<ArcSwap<KafkaClients>>,
    pub config: Config,
    pub pools: ClusterPools,
    pub cache: MetadataCache,
    pub refresh_state: Arc<Mutex<RefreshState>>,
    pub connect_time: Arc<Mutex<Option<Instant>>>,
}

/// 刷新状态跟踪结构
#[derive(Debug, Default)]
pub struct RefreshState {
    pub refreshing_topics: HashSet<String>,
    pub refreshing_consumer_groups: HashSet<String>,
}

impl AppState {
    pub fn get_clients(&self) -> Arc<KafkaClients> {
        self.clients.load_full()
    }

    pub fn set_clients(&self, clients: KafkaClients) {
        self.clients.store(clients.into());
    }

    pub fn get_pool(&self) -> sqlx::Pool<sqlx::Sqlite> {
        self.db.inner().clone()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    tracing::info!("=== Kafka Manager (Slint) starting ===");

    // 加载配置
    let config = Config::load("config.toml").unwrap_or_else(|_| Config::default());
    tracing::info!("Config loaded");

    // 创建 tokio 运行时
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // 在 tokio 运行时中初始化数据库和 Kafka 客户端
    let state = runtime.block_on(async {
        init_state(&config).await
    })?;

    // 启动 Slint UI
    let ui = ui::MainWindow::new()?;

    // 从数据库加载集群列表到 UI
    let cluster_names = runtime.block_on(async {
        load_cluster_names(&state.db.inner().clone()).await
    });
    ui::set_cluster_names(&ui, cluster_names);

    // 设置 UI 回调 (需要 state)
    setup_ui_callbacks(&ui, state.clone());

    // 更新 UI 状态
    ui.set_status_message("就绪".into());

    // 启动定时器更新连接时间
    let ui_handle = ui.as_weak();
    let state_timer = state.clone();
    slint::Timer::default().start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        move || {
            if let Some(ui) = ui_handle.upgrade() {
                if ui.get_connected() {
                    if let Ok(opt_instant) = state_timer.connect_time.lock() {
                        if let Some(instant) = opt_instant.as_ref() {
                            let elapsed = instant.elapsed();
                            let hours = elapsed.as_secs() / 3600;
                            let mins = (elapsed.as_secs() % 3600) / 60;
                            let secs = elapsed.as_secs() % 60;
                            let duration_str = format!("已连接：{:02}:{:02}:{:02}", hours, mins, secs);
                            ui.set_connection_duration(duration_str.into());
                        }
                    }
                }
            }
        },
    );

    // 运行 UI 事件循环
    ui::run(ui)?;

    Ok(())
}

async fn init_state(config: &Config) -> Result<AppState, Box<dyn std::error::Error>> {
    // 初始化数据库
    let db = DbPool::new("kafka_manager.db").await?;
    db.init().await?;
    tracing::info!("Database initialized");

    // 从数据库加载集群配置
    let clusters = load_clusters_from_db(db.inner()).await?;

    // 创建 Kafka 客户端
    let clients = KafkaClients::new(&clusters)?;
    let clients: Arc<ArcSwap<KafkaClients>> = Arc::new(ArcSwap::new(clients.into()));
    tracing::info!("Kafka clients initialized");

    // 初始化连接池
    let pools = ClusterPools::new();
    pools.init(&clusters, &config.pool).await?;
    tracing::info!("Connection pools initialized");

    // 初始化缓存
    let cache = MetadataCache::new();

    // 初始化刷新状态
    let refresh_state = Arc::new(Mutex::new(RefreshState::default()));

    // 初始化连接时间
    let connect_time = Arc::new(Mutex::new(None));

    Ok(AppState {
        db,
        clients,
        config: config.clone(),
        pools,
        cache,
        refresh_state,
        connect_time,
    })
}

async fn load_clusters_from_db(
    pool: &sqlx::SqlitePool,
) -> Result<std::collections::HashMap<String, crate::config::KafkaConfig>, crate::error::AppError> {
    use crate::config::KafkaConfig;
    use crate::db::cluster::ClusterStore;

    let db_clusters = ClusterStore::list(pool).await?;
    let mut clusters = std::collections::HashMap::with_capacity(db_clusters.len());

    for cluster in db_clusters {
        clusters.insert(
            cluster.name,
            KafkaConfig {
                brokers: cluster.brokers,
                request_timeout_ms: cluster.request_timeout_ms as u32,
                operation_timeout_ms: cluster.operation_timeout_ms as u32,
            },
        );
    }

    Ok(clusters)
}

/// 从数据库加载集群名称列表
async fn load_cluster_names(
    pool: &sqlx::SqlitePool,
) -> Vec<String> {
    use crate::db::cluster::ClusterStore;

    match ClusterStore::list(pool).await {
        Ok(clusters) => clusters.iter().map(|c| c.name.clone()).collect(),
        Err(_) => Vec::new(),
    }
}

/// 设置 UI 回调
fn setup_ui_callbacks(ui: &ui::MainWindow, state: AppState) {
    // 连接集群回调
    let ui_handle = ui.as_weak();
    let state_connect = state.clone();
    ui.on_connect_cluster(move || {
        tracing::info!("Connect button clicked");
        if let Some(ui) = ui_handle.upgrade() {
            // 设置加载状态
            ui::set_loading(&ui, true, "连接中...");

            let state = state_connect.clone();
            slint::spawn_local(async move {
                // 获取选中的集群
                let cluster_idx: i32 = ui.get_selected_cluster_index();
                let cluster_names = ui.get_cluster_names();
                if let Some(cluster_name) = cluster_names.row_data(cluster_idx as usize) {
                    let cluster_name_str = cluster_name.to_string();
                    tracing::info!("Connecting to cluster: {}", cluster_name_str);

                    // 从数据库加载集群配置
                    use crate::db::cluster::ClusterStore;
                    let result = ClusterStore::get_by_name(&state.db.inner(), &cluster_name_str).await;

                    match result {
                        Ok(Some(cluster)) => {
                            let config = crate::config::KafkaConfig {
                                brokers: cluster.brokers.clone(),
                                request_timeout_ms: cluster.request_timeout_ms as u32,
                                operation_timeout_ms: cluster.operation_timeout_ms as u32,
                            };

                            // 创建新客户端并更新
                            match KafkaClients::new(&std::collections::HashMap::from([(cluster_name_str.clone(), config)])) {
                                Ok(new_clients) => {
                                    state.set_clients(new_clients);

                                    // 设置连接时间
                                    if let Ok(mut time) = state.connect_time.lock() {
                                        *time = Some(Instant::now());
                                    }

                                    ui.set_connected(true);
                                    ui.set_current_cluster(cluster_name_str.clone().into());

                                    // 刷新 Topic 列表 (list_topics 是同步方法)
                                    if let Some(admin) = state.get_clients().get_admin(&cluster_name_str) {
                                        if let Ok(topics) = admin.list_topics() {
                                            ui::set_topic_names(&ui, topics);
                                        }
                                    }

                                    // 刷新 Consumer Group 列表
                                    if let Some(consumer) = state.get_clients().get_consumer(&cluster_name_str) {
                                        if let Some(config) = state.get_clients().get_config(&cluster_name_str) {
                                            if let Ok(groups) = consumer.list_consumer_groups(&config) {
                                                ui::set_consumer_group_names(&ui, groups);
                                            }
                                        }
                                    }

                                    ui::set_loading(&ui, false, "");
                                    ui.set_status_message(format!("已连接到 {}", cluster_name_str).into());
                                }
                                Err(e) => {
                                    tracing::error!("Failed to create Kafka clients: {}", e);
                                    ui::set_loading(&ui, false, "");
                                    ui.set_status_message(format!("创建客户端失败：{}", e).into());
                                }
                            }
                        }
                        Ok(None) => {
                            ui::set_loading(&ui, false, "");
                            ui.set_status_message(format!("集群不存在：{}", cluster_name_str).into());
                        }
                        Err(e) => {
                            tracing::error!("Failed to load cluster config: {}", e);
                            ui::set_loading(&ui, false, "");
                            ui.set_status_message(format!("加载集群配置失败：{}", e).into());
                        }
                    }
                } else {
                    ui::set_loading(&ui, false, "");
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 断开集群回调
    let ui_handle = ui.as_weak();
    let state_disconnect = state.clone();
    ui.on_disconnect_cluster(move || {
        tracing::info!("Disconnect button clicked");
        if let Some(ui) = ui_handle.upgrade() {
            let current_cluster = ui.get_current_cluster().to_string();

            // 清空客户端
            let empty_clients = KafkaClients::new(&std::collections::HashMap::new()).unwrap();
            state_disconnect.set_clients(empty_clients);

            // 清空连接时间
            if let Ok(mut time) = state_disconnect.connect_time.lock() {
                *time = None;
            }

            ui.set_connected(false);
            ui.set_current_cluster("".into());
            ui.set_status_message("已断开".into());
            ui::set_topic_names(&ui, vec![]);
            ui::set_consumer_group_names(&ui, vec![]);

            tracing::info!("Disconnected from cluster: {}", current_cluster);
        }
    });

    // 刷新数据回调
    let ui_handle = ui.as_weak();
    let state_refresh = state.clone();
    ui.on_refresh_data(move || {
        tracing::info!("Refresh button clicked");
        if let Some(ui) = ui_handle.upgrade() {
            let current_cluster = ui.get_current_cluster().to_string();
            if current_cluster.is_empty() {
                ui.set_status_message("未连接集群".into());
                return;
            }

            // 设置加载状态
            ui::set_loading(&ui, true, "刷新中...");

            let state = state_refresh.clone();
            slint::spawn_local(async move {
                let clients = state.get_clients();
                let selected_tab: i32 = ui.get_selected_tab();

                let result = match selected_tab {
                    1 => {
                        // Consumer Groups Tab
                        if let Some(consumer) = clients.get_consumer(&current_cluster) {
                            if let Some(config) = clients.get_config(&current_cluster) {
                                match consumer.list_consumer_groups(&config) {
                                    Ok(groups) => {
                                        ui::set_consumer_group_names(&ui, groups);
                                        Ok("Consumer Group 刷新成功".to_string())
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to list consumer groups: {}", e);
                                        Err(format!("刷新失败：{}", e))
                                    }
                                }
                            } else {
                                Err("未找到集群配置".to_string())
                            }
                        } else {
                            Err("未找到 Consumer 客户端".to_string())
                        }
                    }
                    _ => {
                        // Topics Tab (default)
                        if let Some(admin) = clients.get_admin(&current_cluster) {
                            match admin.list_topics() {
                                Ok(topics) => {
                                    ui::set_topic_names(&ui, topics);
                                    Ok("刷新成功".to_string())
                                }
                                Err(e) => {
                                    tracing::error!("Failed to refresh topics: {}", e);
                                    Err(format!("刷新失败：{}", e))
                                }
                            }
                        } else {
                            Err("未找到 Admin 客户端".to_string())
                        }
                    }
                };

                // 清除加载状态
                ui::set_loading(&ui, false, "");
                if let Ok(msg) = result {
                    ui.set_status_message(msg.into());
                } else if let Err(msg) = result {
                    ui.set_status_message(msg.into());
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 集群变更回调
    let ui_handle = ui.as_weak();
    ui.on_on_cluster_changed(move |index: i32| {
        tracing::info!("Cluster changed to index: {}", index);
        if let Some(ui) = ui_handle.upgrade() {
            let cluster_names = ui.get_cluster_names();
            if let Some(name) = cluster_names.row_data(index as usize) {
                ui.set_current_cluster(name.clone());
            }
        }
    });

    // Topic 选择回调
    let ui_handle = ui.as_weak();
    let state_topic = state.clone();
    ui.on_on_topic_selected(move |topic_name: slint::SharedString| {
        tracing::info!("Topic selected: {}", topic_name);
        if let Some(ui) = ui_handle.upgrade() {
            let state = state_topic.clone();
            let topic_name_str = topic_name.to_string();
            let current_cluster = ui.get_current_cluster().to_string();

            // 设置查看消息的 Topic
            ui.set_view_messages_topic(topic_name_str.clone().into());
            ui.set_view_messages_partition("0".into());
            ui.set_view_messages_count("0".into());
            ui::set_view_messages(&ui, vec![]);

            slint::spawn_local(async move {
                if let Some(admin) = state.get_clients().get_admin(&current_cluster) {
                    if let Some(config) = state.get_clients().get_config(&current_cluster) {
                        match admin.get_topic_info(&topic_name_str) {
                            Ok(info) => {
                                let num_partitions = info.partitions.len();
                                let replication = if num_partitions > 0 {
                                    info.partitions[0].replicas.len()
                                } else {
                                    0
                                };

                                // 格式化分区详情
                                let partition_details: Vec<String> = info.partitions.iter().map(|p| {
                                    let replicas_str = p.replicas.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",");
                                    let isr_str = p.isr.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",");
                                    format!("{}      {}      {}      {}", p.id, p.leader, replicas_str, isr_str)
                                }).collect();

                                // 获取统计信息
                                let consumer = crate::kafka::consumer::KafkaConsumer::new(&config);
                                let (total_messages, total_size) = match &consumer {
                                    Ok(c) => {
                                        match c.get_topic_partition_info(&config, &topic_name_str) {
                                            Ok(partitions) => {
                                                let total: u64 = partitions.iter().map(|(_, _, count)| *count as u64).sum();
                                                // 估算大小（假设平均每条消息 1KB）
                                                let estimated_size = total * 1024;
                                                (total, estimated_size)
                                            }
                                            Err(_) => (0, 0)
                                        }
                                    }
                                    Err(_) => (0, 0)
                                };

                                ui::set_topic_detail(&ui, &topic_name_str, num_partitions, replication, "正常", partition_details, total_messages, total_size);
                                ui.set_show_topic_detail_dialog(true);
                                ui.set_delete_topic_name(topic_name_str.clone().into());
                            }
                            Err(e) => {
                                tracing::error!("Failed to get topic info: {}", e);
                                ui.set_status_message(format!("获取 Topic 详情失败：{}", e).into());
                            }
                        }
                    } else {
                        ui.set_status_message("未找到集群配置".into());
                    }
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 关闭 Topic 详情弹窗回调
    let ui_handle = ui.as_weak();
    ui.on_close_topic_detail_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_topic_detail_dialog(false);
        }
    });

    // 刷新 Topic 详情回调
    let ui_handle = ui.as_weak();
    let state_topic_detail = state.clone();
    ui.on_refresh_topic_detail(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let state = state_topic_detail.clone();
            let topic_name = ui.get_topic_detail_name().to_string();
            let current_cluster = ui.get_current_cluster().to_string();

            slint::spawn_local(async move {
                if let Some(admin) = state.get_clients().get_admin(&current_cluster) {
                    if let Some(config) = state.get_clients().get_config(&current_cluster) {
                        match admin.get_topic_info(&topic_name) {
                            Ok(info) => {
                                let num_partitions = info.partitions.len();
                                let replication = if num_partitions > 0 {
                                    info.partitions[0].replicas.len()
                                } else {
                                    0
                                };

                                // 格式化分区详情
                                let partition_details: Vec<String> = info.partitions.iter().map(|p| {
                                    let replicas_str = p.replicas.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",");
                                    let isr_str = p.isr.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",");
                                    format!("{}      {}      {}      {}", p.id, p.leader, replicas_str, isr_str)
                                }).collect();

                                // 获取统计信息
                                let consumer = crate::kafka::consumer::KafkaConsumer::new(&config);
                                let (total_messages, total_size) = match &consumer {
                                    Ok(c) => {
                                        match c.get_topic_partition_info(&config, &topic_name) {
                                            Ok(partitions) => {
                                                let total: u64 = partitions.iter().map(|(_, _, count)| *count as u64).sum();
                                                // 估算大小（假设平均每条消息 1KB）
                                                let estimated_size = total * 1024;
                                                (total, estimated_size)
                                            }
                                            Err(_) => (0, 0)
                                        }
                                    }
                                    Err(_) => (0, 0)
                                };

                                ui::set_topic_detail(&ui, &topic_name, num_partitions, replication, "正常", partition_details, total_messages, total_size);
                            }
                            Err(e) => {
                                ui.set_status_message(format!("刷新失败：{}", e).into());
                            }
                        }
                    } else {
                        ui.set_status_message("未找到集群配置".into());
                    }
                }
            }).expect("Failed to spawn async task");
        }
    });

    // Consumer Group 选择回调
    let ui_handle = ui.as_weak();
    let state_consumer_group = state.clone();
    ui.on_on_consumer_group_selected(move |group_name: slint::SharedString| {
        tracing::info!("Consumer Group selected: {}", group_name);
        if let Some(ui) = ui_handle.upgrade() {
            let state = state_consumer_group.clone();
            let group_name_str = group_name.to_string();
            let current_cluster = ui.get_current_cluster().to_string();

            slint::spawn_local(async move {
                // 获取 Consumer Group 详情
                if let Some(admin) = state.get_clients().get_admin(&current_cluster) {
                    if let Some(config) = state.get_clients().get_config(&current_cluster) {
                        match admin.get_consumer_group_info(&config, &group_name_str) {
                            Ok(info) => {
                                // 获取成员数量
                                let member_count = info.members.len();

                                // 获取订阅的 topics（从 committed offsets）
                                use crate::kafka::consumer_group::KafkaConsumerGroupManager;
                                let topics = if let Ok(group_mgr) = KafkaConsumerGroupManager::new(&config) {
                                    match group_mgr.get_consumer_group_topics(&group_name_str) {
                                        Ok(t) => t.join(", "),
                                        Err(_) => "Unknown".to_string(),
                                    }
                                } else {
                                    "Unknown".to_string()
                                };

                                // 获取 lag 统计和分区详情
                                let (total_lag, partition_details) = if let Ok(group_mgr) = KafkaConsumerGroupManager::new(&config) {
                                    let topic_vec = if let Ok(group_mgr) = KafkaConsumerGroupManager::new(&config) {
                                        group_mgr.get_consumer_group_topics(&group_name_str).unwrap_or_default()
                                    } else {
                                        Vec::new()
                                    };
                                    match group_mgr.get_consumer_group_offsets(&group_name_str, &topic_vec) {
                                        Ok(offsets) => {
                                            let total_lag: u64 = offsets.iter().map(|o| o.lag.max(0) as u64).sum();
                                            let partition_details: Vec<String> = offsets.iter().take(10).map(|o| {
                                                format!("{}      {}      {}      {}      {}",
                                                    o.topic, o.partition, o.committed_offset, o.end_offset, o.lag)
                                            }).collect();
                                            (total_lag, partition_details)
                                        }
                                        Err(_) => (0, vec!["暂无分区详情".to_string()])
                                    }
                                } else {
                                    (0, vec!["暂无分区详情".to_string()])
                                };

                                ui::set_consumer_group_detail(
                                    &ui,
                                    &group_name_str,
                                    &info.state,
                                    member_count,
                                    &topics,
                                    total_lag,
                                    partition_details,
                                );
                                ui.set_show_consumer_group_detail_dialog(true);

                                // 设置重置 offset 相关属性
                                ui.set_reset_offset_group(group_name_str.clone().into());
                                ui.set_reset_offset_topic("".into());
                                ui.set_reset_offset_partition("0".into());
                                ui.set_reset_offset_mode(0);
                                ui.set_reset_offset_specific("0".into());
                            }
                            Err(e) => {
                                tracing::error!("Failed to get consumer group info: {}", e);
                                ui.set_status_message(format!("获取 Consumer Group 详情失败：{}", e).into());
                            }
                        }
                    } else {
                        ui.set_status_message("未找到集群配置".into());
                    }
                } else {
                    ui.set_status_message("未找到 Admin 客户端".into());
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 关闭 Consumer Group 详情弹窗回调
    let ui_handle = ui.as_weak();
    ui.on_close_consumer_group_detail_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_consumer_group_detail_dialog(false);
        }
    });

    // 刷新 Consumer Group 详情回调
    let ui_handle = ui.as_weak();
    let state_refresh_cg_detail = state.clone();
    ui.on_refresh_consumer_group_detail(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let group = ui.get_consumer_group_detail_name().to_string();
            let current_cluster = ui.get_current_cluster().to_string();

            ui::set_loading(&ui, true, "刷新中...");

            let state = state_refresh_cg_detail.clone();
            slint::spawn_local(async move {
                if let Some(admin) = state.get_clients().get_admin(&current_cluster) {
                    if let Some(config) = state.get_clients().get_config(&current_cluster) {
                        match admin.get_consumer_group_info(&config, &group) {
                            Ok(info) => {
                                let member_count = info.members.len();

                                use crate::kafka::consumer_group::KafkaConsumerGroupManager;
                                let topics = if let Ok(group_mgr) = KafkaConsumerGroupManager::new(&config) {
                                    match group_mgr.get_consumer_group_topics(&group) {
                                        Ok(t) => t.join(", "),
                                        Err(_) => "Unknown".to_string(),
                                    }
                                } else {
                                    "Unknown".to_string()
                                };

                                // 获取 lag 统计和分区详情
                                let (total_lag, partition_details) = if let Ok(group_mgr) = KafkaConsumerGroupManager::new(&config) {
                                    let topic_vec = group_mgr.get_consumer_group_topics(&group).unwrap_or_default();
                                    match group_mgr.get_consumer_group_offsets(&group, &topic_vec) {
                                        Ok(offsets) => {
                                            let total_lag: u64 = offsets.iter().map(|o| o.lag.max(0) as u64).sum();
                                            let partition_details: Vec<String> = offsets.iter().take(10).map(|o| {
                                                format!("{}      {}      {}      {}      {}",
                                                    o.topic, o.partition, o.committed_offset, o.end_offset, o.lag)
                                            }).collect();
                                            (total_lag, partition_details)
                                        }
                                        Err(_) => (0, vec!["暂无分区详情".to_string()])
                                    }
                                } else {
                                    (0, vec!["暂无分区详情".to_string()])
                                };

                                ui::set_consumer_group_detail(&ui, &group, &info.state, member_count, &topics, total_lag, partition_details);
                                ui::set_loading(&ui, false, "");
                            }
                            Err(e) => {
                                ui::set_loading(&ui, false, "");
                                ui.set_status_message(format!("刷新失败：{}", e).into());
                            }
                        }
                    } else {
                        ui::set_loading(&ui, false, "");
                        ui.set_status_message("未找到集群配置".into());
                    }
                } else {
                    ui::set_loading(&ui, false, "");
                    ui.set_status_message("未找到 Admin 客户端".into());
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 关闭查看消息弹窗回调
    let ui_handle = ui.as_weak();
    ui.on_close_view_messages_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_view_messages_dialog(false);
        }
    });

    // 关闭重置 offset 弹窗回调
    let ui_handle = ui.as_weak();
    ui.on_close_reset_offset_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_reset_offset_dialog(false);
        }
    });

    // 重置模式变化回调
    ui.on_on_reset_offset_mode_changed(move |mode: i32| {
        tracing::info!("Reset offset mode changed to: {}", mode);
    });

    // 确认重置 offset 回调
    let ui_handle = ui.as_weak();
    let state_reset_offset = state.clone();
    ui.on_confirm_reset_offset(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let group = ui.get_reset_offset_group().to_string();
            let topic = ui.get_reset_offset_topic().to_string();
            let partition: i32 = ui.get_reset_offset_partition().to_string().parse().unwrap_or(0);
            let mode: i32 = ui.get_reset_offset_mode();
            let specific_offset: i64 = ui.get_reset_offset_specific().to_string().parse().unwrap_or(0);
            let current_cluster = ui.get_current_cluster().to_string();

            ui::set_loading(&ui, true, "重置 Offset 中...");

            let state = state_reset_offset.clone();
            slint::spawn_local(async move {
                use crate::kafka::consumer_group::KafkaConsumerGroupManager;
                use crate::error::AppError;

                if let Some(config) = state.get_clients().get_config(&current_cluster) {
                    match KafkaConsumerGroupManager::new(&config) {
                        Ok(mgr) => {
                            let result = match mode {
                                0 => mgr.reset_consumer_group_offset_to_earliest(&group, &topic, partition),
                                1 => mgr.reset_consumer_group_offset_to_latest(&group, &topic, partition),
                                2 => mgr.reset_consumer_group_offset(&group, &topic, partition, specific_offset),
                                _ => Err(AppError::Internal("Invalid mode".to_string())),
                            };

                            ui::set_loading(&ui, false, "");
                            match result {
                                Ok(offset) => {
                                    ui.set_show_reset_offset_dialog(false);
                                    ui.set_status_message(format!("Offset 已重置为：{}", offset).into());
                                }
                                Err(e) => {
                                    ui.set_status_message(format!("重置 Offset 失败：{}", e).into());
                                }
                            }
                        }
                        Err(e) => {
                            ui::set_loading(&ui, false, "");
                            ui.set_status_message(format!("创建 Consumer Group 管理器失败：{}", e).into());
                        }
                    }
                } else {
                    ui::set_loading(&ui, false, "");
                    ui.set_status_message("未找到集群配置".into());
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 分区输入变化回调
    let ui_handle = ui.as_weak();
    ui.on_on_view_messages_partition_changed(move |text: slint::SharedString| {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_view_messages_partition(text);
        }
    });

    // 确认查看消息回调
    let ui_handle = ui.as_weak();
    let state_view_messages = state.clone();
    ui.on_confirm_view_messages(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let topic = ui.get_view_messages_topic().to_string();
            let partition: i32 = ui.get_view_messages_partition().to_string().parse().unwrap_or(0);
            let filter_key = ui.get_view_messages_filter_key().to_string();
            let filter_value = ui.get_view_messages_filter_value().to_string();
            let current_cluster = ui.get_current_cluster().to_string();

            ui::set_loading(&ui, true, "加载消息中...");

            let state = state_view_messages.clone();
            slint::spawn_local(async move {
                if let Some(consumer) = state.get_clients().get_consumer(&current_cluster) {
                    if let Some(config) = state.get_clients().get_config(&current_cluster) {
                        match consumer.fetch_latest_messages_sync(&config, &topic, partition, 10) {
                            Ok(messages) => {
                                // 应用过滤器
                                let has_key_filter = !filter_key.is_empty();
                                let has_value_filter = !filter_value.is_empty();

                                let filtered_messages: Vec<_> = messages.into_iter().filter(|m| {
                                    let key_match = if has_key_filter {
                                        m.key.as_ref().map(|k| k.contains(&filter_key)).unwrap_or(false)
                                    } else {
                                        true
                                    };
                                    let value_match = if has_value_filter {
                                        m.value.as_ref().map(|v| v.contains(&filter_value)).unwrap_or(false)
                                    } else {
                                        true
                                    };
                                    key_match && value_match
                                }).collect();

                                let count = filtered_messages.len();
                                let message_strs: Vec<slint::SharedString> = filtered_messages.iter().map(|m| {
                                    let key_str = m.key.as_deref().unwrap_or("null");
                                    let value_str = m.value.as_deref().unwrap_or("null");
                                    let ts_str = m.timestamp.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string());

                                    // 格式化 JSON 值
                                    let formatted_value = crate::utils::json_formatter::format_json(value_str);

                                    format!("{} | {} | {} | {}", m.offset, ts_str, key_str, formatted_value).into()
                                }).collect();

                                ui.set_view_messages_count(count.to_string().into());
                                ui::set_view_messages(&ui, message_strs.iter().map(|s| s.to_string()).collect());
                                ui::set_loading(&ui, false, "");
                            }
                            Err(e) => {
                                tracing::error!("Failed to fetch messages: {}", e);
                                ui::set_loading(&ui, false, "");
                                ui.set_status_message(format!("加载消息失败：{}", e).into());
                            }
                        }
                    } else {
                        ui::set_loading(&ui, false, "");
                        ui.set_status_message("未找到集群配置".into());
                    }
                } else {
                    ui::set_loading(&ui, false, "");
                    ui.set_status_message("未找到 Consumer 客户端".into());
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 关闭创建 Topic 弹窗回调
    let ui_handle = ui.as_weak();
    ui.on_close_create_topic_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_create_topic_dialog(false);
            ui.set_create_topic_name("".into());
            ui.set_create_topic_partitions(3);
            ui.set_create_topic_partitions_str("3".into());
            ui.set_create_topic_replication(1);
            ui.set_create_topic_replication_str("1".into());
        }
    });

    // 分区数输入变化回调
    let ui_handle = ui.as_weak();
    ui.on_on_create_topic_partitions_changed(move |text: slint::SharedString| {
        if let Some(ui) = ui_handle.upgrade() {
            let val = text.to_string().parse::<i32>().unwrap_or(3);
            ui.set_create_topic_partitions(val);
        }
    });

    // 副本数输入变化回调
    let ui_handle = ui.as_weak();
    ui.on_on_create_topic_replication_changed(move |text: slint::SharedString| {
        if let Some(ui) = ui_handle.upgrade() {
            let val = text.to_string().parse::<i32>().unwrap_or(1);
            ui.set_create_topic_replication(val);
        }
    });

    // 确认创建 Topic 回调
    let ui_handle = ui.as_weak();
    let state_create = state.clone();
    ui.on_confirm_create_topic(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let topic_name = ui.get_create_topic_name().to_string();
            let partitions = ui.get_create_topic_partitions();
            let replication = ui.get_create_topic_replication();
            let current_cluster = ui.get_current_cluster().to_string();

            if topic_name.is_empty() {
                ui.set_status_message("Topic 名称不能为空".into());
                return;
            }

            let state = state_create.clone();
            slint::spawn_local(async move {
                if let Some(admin) = state.get_clients().get_admin(&current_cluster) {
                    match admin.create_topic(&topic_name, partitions, replication, std::collections::HashMap::new()).await {
                        Ok(_) => {
                            ui.set_status_message(format!("Topic '{}' 创建成功", topic_name).into());
                            ui.set_show_create_topic_dialog(false);
                            ui.set_create_topic_name("".into());
                            ui.set_create_topic_partitions(3);
                            ui.set_create_topic_replication(1);

                            // 刷新 Topic 列表
                            if let Ok(topics) = admin.list_topics() {
                                ui::set_topic_names(&ui, topics);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to create topic: {}", e);
                            ui.set_status_message(format!("创建 Topic 失败：{}", e).into());
                        }
                    }
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 关闭删除确认弹窗回调
    let ui_handle = ui.as_weak();
    ui.on_close_delete_confirm_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_delete_confirm_dialog(false);
        }
    });

    // 确认删除 Topic 回调
    let ui_handle = ui.as_weak();
    let state_delete = state.clone();
    ui.on_confirm_delete_topic(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let topic_name = ui.get_delete_topic_name().to_string();
            let current_cluster = ui.get_current_cluster().to_string();

            ui.set_show_delete_confirm_dialog(false);

            let state = state_delete.clone();
            slint::spawn_local(async move {
                if let Some(admin) = state.get_clients().get_admin(&current_cluster) {
                    match admin.delete_topic(&topic_name).await {
                        Ok(_) => {
                            ui.set_status_message(format!("Topic '{}' 删除成功", topic_name).into());
                            ui.set_show_topic_detail_dialog(false);

                            // 刷新 Topic 列表
                            if let Ok(topics) = admin.list_topics() {
                                ui::set_topic_names(&ui, topics);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to delete topic: {}", e);
                            ui.set_status_message(format!("删除 Topic 失败：{}", e).into());
                        }
                    }
                }
            }).expect("Failed to spawn async task");
        }
    });

    // Tab 切换回调
    let ui_handle = ui.as_weak();
    ui.on_on_tab_changed(move |tab_index: i32| {
        tracing::info!("Tab changed to: {}", tab_index);
        if let Some(ui) = ui_handle.upgrade() {
            let tab_name = if tab_index == 1 { "Consumer Groups" } else { "Topics" };
            ui.set_status_message(format!("切换到 {} 标签页", tab_name).into());
        }
    });

    // 显示集群配置对话框
    let ui_handle = ui.as_weak();
    ui.on_show_cluster_config(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_cluster_config_name("".into());
            ui.set_cluster_config_brokers("".into());
            ui.set_cluster_config_sasl_username("".into());
            ui.set_cluster_config_sasl_password("".into());
            ui.set_show_cluster_config_dialog(true);
        }
    });

    // 关闭集群配置对话框
    let ui_handle = ui.as_weak();
    ui.on_close_cluster_config_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_cluster_config_dialog(false);
        }
    });

    // 确认保存集群配置
    let ui_handle = ui.as_weak();
    let state_cluster_config = state.clone();
    ui.on_confirm_save_cluster_config(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let name = ui.get_cluster_config_name().to_string();
            let brokers = ui.get_cluster_config_brokers().to_string();
            let _sasl_username = ui.get_cluster_config_sasl_username().to_string();
            let _sasl_password = ui.get_cluster_config_sasl_password().to_string();

            if name.is_empty() || brokers.is_empty() {
                ui.set_status_message("集群名称和 Brokers 地址不能为空".into());
                return;
            }

            let state = state_cluster_config.clone();
            slint::spawn_local(async move {
                use crate::db::cluster::KafkaCluster;
                let pool = state.get_pool();

                // 检查是否已存在同名集群
                let existing: Option<KafkaCluster> = sqlx::query_as(
                    "SELECT * FROM kafka_clusters WHERE name = ?"
                )
                .bind(&name)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten();

                if let Some(_) = existing {
                    ui.set_status_message(format!("集群 '{}' 已存在", name).into());
                    return;
                }

                // 创建新集群配置
                let now = chrono::Utc::now().to_rfc3339();
                match sqlx::query(
                    "INSERT INTO kafka_clusters (name, brokers, request_timeout_ms, operation_timeout_ms, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(&name)
                .bind(&brokers)
                .bind(5000)
                .bind(5000)
                .bind(&now)
                .bind(&now)
                .execute(&pool)
                .await
                {
                    Ok(_) => {
                        ui.set_status_message(format!("集群 '{}' 创建成功", name).into());
                        ui.set_show_cluster_config_dialog(false);

                        // 更新 UI 集群列表
                        let clusters: Vec<KafkaCluster> = sqlx::query_as("SELECT * FROM kafka_clusters ORDER BY name")
                            .fetch_all(&pool)
                            .await
                            .unwrap_or_default();

                        let cluster_names: Vec<slint::SharedString> = clusters.iter().map(|c| c.name.clone().into()).collect();
                        let model = slint::VecModel::from(cluster_names);
                        ui.set_cluster_names(slint::ModelRc::new(model));

                        if !clusters.is_empty() {
                            ui.set_current_cluster(clusters[0].name.clone().into());
                        }
                    }
                    Err(e) => {
                        ui.set_status_message(format!("创建集群失败：{}", e).into());
                    }
                }
            }).expect("Failed to spawn async task");
        }
    });

    // 下一个集群回调
    let ui_handle = ui.as_weak();
    ui.on_next_cluster(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let current: i32 = ui.get_selected_cluster_index();
            let cluster_names = ui.get_cluster_names();
            let len: usize = cluster_names.row_count();
            if len > 0 {
                let next = ((current as usize + 1) % len) as i32;
                ui.set_selected_cluster_index(next);
                if let Some(name) = cluster_names.row_data(next as usize) {
                    ui.set_current_cluster(name.clone());
                }
            }
        }
    });

    // 关闭发送消息对话框
    let ui_handle = ui.as_weak();
    ui.on_close_send_message_dialog(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_show_send_message_dialog(false);
        }
    });

    // 分区输入变化回调
    let ui_handle = ui.as_weak();
    ui.on_on_send_message_partition_changed(move |text: slint::SharedString| {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_send_message_partition(text);
        }
    });

    // 确认发送消息回调
    let ui_handle = ui.as_weak();
    let state_send_message = state.clone();
    ui.on_confirm_send_message(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let topic = ui.get_send_message_topic().to_string();
            let partition: i32 = ui.get_send_message_partition().to_string().parse().unwrap_or(0);
            let key = ui.get_send_message_key().to_string();
            let value = ui.get_send_message_value().to_string();
            let current_cluster = ui.get_current_cluster().to_string();

            if value.is_empty() {
                ui.set_status_message("消息内容不能为空".into());
                return;
            }

            ui::set_loading(&ui, true, "发送消息中...");

            let state = state_send_message.clone();
            slint::spawn_local(async move {
                if let Some(producer) = state.get_clients().get_producer(&current_cluster) {
                    match producer.send_to_partition(&topic, Some(partition), if key.is_empty() { None } else { Some(&key) }, &value, None).await {
                        Ok((partition, offset)) => {
                            let result = format!("消息发送成功！分区：{}, Offset: {}", partition, offset);
                            ui.set_send_message_result(result.clone().into());
                            ui.set_status_message(result.into());
                            ui::set_loading(&ui, false, "");
                        }
                        Err(e) => {
                            tracing::error!("Failed to send message: {}", e);
                            ui.set_send_message_result(format!("发送失败：{}", e).into());
                            ui::set_loading(&ui, false, "");
                            ui.set_status_message(format!("发送消息失败：{}", e).into());
                        }
                    }
                } else {
                    ui::set_loading(&ui, false, "");
                    ui.set_status_message("未找到 Producer 客户端".into());
                }
            }).expect("Failed to spawn async task");
        }
    });
}
