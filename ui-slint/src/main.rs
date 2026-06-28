#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::ComponentHandle;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;

// 包含 Slint 生成的模块（从 lib.rs 导入）
use kafka_manager_ui::*;

// 声明模块
mod handlers;
mod models;
mod i18n;

// 引用核心业务逻辑
use kafka_manager_api::{AppState, Config, DbPool, ClusterPools, RefreshState, ImportExportLock};
use kafka_manager_api::kafka::KafkaClients;
use arc_swap::ArcSwap;

// 引用 i18n模块
use i18n::{Language, apply_i18n};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("Kafka Manager (Slint) starting...");

    // 初始化应用状态
    let state = initialize_app_state().await?;

    // 创建 Slint 应用
    let app = App::new()?;

    // 设置系统托盘（后续实现）
    // setup_system_tray(&app);

    // 设置回调函数
    setup_handlers(&app, state.clone());

    // 设置初始窗口标题
    app.set_window_title("Kafka Manager (Slint)".into());

    println!("Application initialized successfully");

    // 自动加载集群列表（启动时）
    {
        let app_weak = app.as_weak();
        let state_clone = state.clone();

        tokio::spawn(async move {
            // 延迟一下，确保 UI 已完全初始化
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            use handlers::cluster::load_clusters;
            load_clusters(state_clone, app_weak).await;
        });
    }

    // 运行应用
    app.run()?;

    Ok(())
}

/// 初始化应用状态（复用现有逻辑）
async fn initialize_app_state() -> Result<Arc<RwLock<AppState>>, Box<dyn std::error::Error>> {
    // 获取数据库路径
    let db_path = get_db_path();

    // 创建数据目录（如果不存在）
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!("Database path: {:?}", db_path);

    // 连接数据库
    let db_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());
    let db = DbPool::new(&db_url).await?;

    // 加载配置（使用默认配置）
    let config = Config::default();

    // 初始化 Kafka 客户端（空）
    let clients = Arc::new(ArcSwap::new(Arc::new(KafkaClients::default())));

    // 初始化连接池
    let pools = ClusterPools::new();

    // 创建应用状态
    let state = AppState {
        db,
        clients,
        config,
        pools,
        refresh_state: Arc::new(std::sync::Mutex::new(RefreshState::default())),
        import_export_lock: Arc::new(std::sync::Mutex::new(ImportExportLock::default())),
    };

    Ok(Arc::new(RwLock::new(state)))
}

/// 获取数据库路径（遵循平台规范）
fn get_db_path() -> PathBuf {
    let db_filename = "kafka_manager.db";

    if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .map(|d| d.join("Kafka Manager"))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(db_filename)
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()
            .map(|d| d.join("Library/Application Support/Kafka Manager"))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(db_filename)
    } else {
        dirs::data_local_dir()
            .map(|d| d.join("kafka-manager"))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(db_filename)
    }
}

/// 设置回调函数
fn setup_handlers(app: &App, state: Arc<RwLock<AppState>>) {
    use handlers::{
        cluster::load_clusters,
        topic::load_topics_from_kafka,
        topic_crud::{create_topic, delete_topic, get_topic_config},
        consumer_group::load_consumer_groups_from_kafka,
        consumer_group_detail::{get_consumer_group_members, get_consumer_group_lag, get_consumer_group_total_lag},
        message::{query_messages_from_kafka, query_messages_stream, stop_message_query},
        settings::{load_settings, save_all_settings, save_tray_enabled, save_language, save_theme},
    };

    // Clone state for each handler (Arc is cheap to clone)
    let state_clusters = state.clone();
    let state_topics = state.clone();
    let state_groups = state.clone();
    let state_messages = state.clone();
    let state_messages_stream = state.clone();
    let state_messages_stop = state.clone();
    let state_settings_load = state.clone();
    let state_settings_save = state.clone();
    let state_tray = state.clone();
    let state_lang = state.clone();
    let state_theme = state.clone();

    let app_weak = app.as_weak();

    // 设置加载集群回调
    app.on_load_clusters(move || {
        let app_weak = app_weak.clone();
        let state = state_clusters.clone();

        tokio::spawn(async move {
            load_clusters(state, app_weak).await;
        });
    });

    // 设置加载 Topics 回调
    let app_weak_topics = app.as_weak();

    app.on_load_topics(move || {
        let app_weak = app_weak_topics.clone();
        let state = state_topics.clone();

        // 获取当前选中的集群 ID
        let app_strong = app_weak.upgrade().unwrap();
        let selected_cluster = app_strong.get_selected_cluster();
        let cluster_id = selected_cluster.to_string();
        // app_strong 会自动 drop

        tokio::spawn(async move {
            load_topics_from_kafka(state, app_weak, cluster_id).await;
        });
    });

    // 设置加载 Consumer Groups 回调
    let app_weak_groups = app.as_weak();

    app.on_load_consumer_groups(move || {
        let app_weak = app_weak_groups.clone();
        let state = state_groups.clone();

        // 获取当前选中的集群 ID
        let app_strong = app_weak.upgrade().unwrap();
        let selected_cluster = app_strong.get_selected_cluster();
        let cluster_id = selected_cluster.to_string();
        // app_strong 会自动 drop

        tokio::spawn(async move {
            load_consumer_groups_from_kafka(state, app_weak, cluster_id).await;
        });
    });

    // 设置查询消息回调（Phase 7 - 流式查询）
    let app_weak_messages = app.as_weak();

    app.on_query_messages(move || {
        let app_weak = app_weak_messages.clone();
        let state = state_messages.clone();

        // 获取当前选中的集群和 topic
        let app_strong = app_weak.upgrade().unwrap();
        let selected_cluster = app_strong.get_selected_cluster();
        let selected_topic = app_strong.get_selected_topic();
        let selected_partition = app_strong.get_selected_partition();
        let max_messages = app_strong.get_max_messages();
        let search_keyword = app_strong.get_search_keyword();
        // app_strong 会自动 drop

        println!("Query messages: cluster={}, topic={}, partition={}, max={}, keyword={}",
            selected_cluster, selected_topic, selected_partition, max_messages, search_keyword);

        tokio::spawn(async move {
            query_messages_stream(
                state,
                app_weak,
                selected_cluster.to_string(),
                selected_topic.to_string(),
                selected_partition,
                max_messages,
                search_keyword.to_string(),
            ).await;
        });
    });

    // 设置停止查询回调（Phase 7）
    let app_weak_stop = app.as_weak();

    app.on_stop_query(move || {
        let app_weak = app_weak_stop.clone();

        tokio::spawn(async move {
            stop_message_query(app_weak).await;
        });
    });

    // 设置发送消息回调（可选功能）
    // TODO: 实现消息发送对话框

    // 其他消息参数回调（Phase 7）
    app.on_partition_changed(move |_partition| {
        // 分区变更，不需要立即查询
    });

    app.on_fetch_mode_changed(move |_mode| {
        // 查询模式变更，不需要立即查询
    });

    app.on_max_messages_changed(move |_count| {
        // 最大消息数变更，不需要立即查询
    });

    app.on_search_keyword_changed(move |_keyword| {
        // 搜索关键字变更，不需要立即查询
    });

    // 设置保存 Settings 回调
    let app_weak_settings_save = app.as_weak();

    app.on_save_settings(move || {
        let app_weak = app_weak_settings_save.clone();
        let state = state_settings_save.clone();

        tokio::spawn(async move {
            save_all_settings(state, app_weak).await;
        });
    });

    // 设置托盘开关回调
    app.on_tray_enabled_changed(move |enabled| {
        let state = state_tray.clone();
        tokio::spawn(async move {
            save_tray_enabled(state, enabled).await;
        });
    });

    // 设置语言切换回调
    let app_weak_lang = app.as_weak();

    app.on_language_changed(move |lang| {
        let state = state_lang.clone();
        let lang_str = lang.to_string();
        let app_for_i18n = app_weak_lang.clone();

        tokio::spawn(async move {
            save_language(state, lang_str.clone()).await;

            // 应用国际化（在主线程）
            let lang_enum = Language::from_str(&lang_str);
            let _ = app_for_i18n.upgrade_in_event_loop(move |app| {
                apply_i18n(&app, lang_enum);
            });
        });
    });

    // 设置主题切换回调（Phase 9 - 支持深色主题）
    let app_weak_theme_ui = app.as_weak();

    app.on_theme_changed(move |theme| {
        let state = state_theme.clone();
        let theme_str = theme.to_string();
        let app_for_ui = app_weak_theme_ui.clone();

        // 保存主题设置
        tokio::spawn(async move {
            save_theme(state, theme_str.clone()).await;

            // 更新 UI（在保存完成后）
            let theme_for_ui = theme_str.clone();
            let _ = app_for_ui.upgrade_in_event_loop(move |app| {
                app.set_theme(slint::SharedString::from(theme_for_ui));
            });

            println!("Theme changed to: {}", theme_str);
        });
    });

    // Phase 8: Topic CRUD 回调
    // 创建 Topic
    let state_create_topic = state.clone();
    let app_weak_create_topic = app.as_weak();

    app.on_create_topic(move |cluster_id, topic_name, num_partitions, replication_factor| {
        let app_weak = app_weak_create_topic.clone();
        let state = state_create_topic.clone();

        tokio::spawn(async move {
            create_topic(
                state,
                app_weak,
                cluster_id.to_string(),
                topic_name.to_string(),
                num_partitions,
                replication_factor,
            ).await;
        });
    });

    // 删除 Topic
    let state_delete_topic = state.clone();
    let app_weak_delete_topic = app.as_weak();

    app.on_delete_topic(move |cluster_id, topic_name| {
        let app_weak = app_weak_delete_topic.clone();
        let state = state_delete_topic.clone();

        tokio::spawn(async move {
            delete_topic(
                state,
                app_weak,
                cluster_id.to_string(),
                topic_name.to_string(),
            ).await;
        });
    });

    // 获取 Topic 配置
    let state_get_topic_config = state.clone();
    let app_weak_get_topic_config = app.as_weak();

    app.on_get_topic_config(move |cluster_id, topic_name| {
        let app_weak = app_weak_get_topic_config.clone();
        let state = state_get_topic_config.clone();

        tokio::spawn(async move {
            get_topic_config(
                state,
                app_weak,
                cluster_id.to_string(),
                topic_name.to_string(),
            ).await;
        });
    });

    // Phase 8: Consumer Group 详情回调
    // 获取 Consumer Group 详情
    let state_cg_detail = state.clone();
    let app_weak_cg_detail = app.as_weak();

    app.on_get_consumer_group_detail(move |cluster_id, group_id| {
        let app_weak = app_weak_cg_detail.clone();
        let state = state_cg_detail.clone();

        tokio::spawn(async move {
            get_consumer_group_members(
                state,
                app_weak,
                cluster_id.to_string(),
                group_id.to_string(),
            ).await;
        });
    });

    // 获取 Consumer Group Lag
    let state_cg_lag = state.clone();
    let app_weak_cg_lag = app.as_weak();

    app.on_get_consumer_group_lag(move |cluster_id, group_id, topic| {
        let app_weak = app_weak_cg_lag.clone();
        let state = state_cg_lag.clone();

        tokio::spawn(async move {
            get_consumer_group_lag(
                state,
                app_weak,
                cluster_id.to_string(),
                group_id.to_string(),
                topic.to_string(),
            ).await;
        });
    });

    // 获取 Consumer Group 总 Lag
    let state_cg_total_lag = state.clone();
    let app_weak_cg_total_lag = app.as_weak();

    app.on_get_consumer_group_total_lag(move |cluster_id, group_id| {
        let app_weak = app_weak_cg_total_lag.clone();
        let state = state_cg_total_lag.clone();

        tokio::spawn(async move {
            get_consumer_group_total_lag(
                state,
                app_weak,
                cluster_id.to_string(),
                group_id.to_string(),
            ).await;
        });
    });

    // 启动时加载 Settings
    let app_weak_settings_load = app.as_weak();
    let state = state_settings_load.clone();
    tokio::spawn(async move {
        load_settings(state, app_weak_settings_load).await;
    });
}

// 系统托盘设置（后续实现）
// fn setup_system_tray(app: &App) {
//     // TODO: 实现系统托盘逻辑
// }