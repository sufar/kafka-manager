// 测试新的 MainLayout（Phase 11 Week 1-2）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::ComponentHandle;
use std::sync::Arc;
use tokio::sync::RwLock;

// 包含新的 Slint 模块
slint::include_modules!();

use kafka_manager_api::{AppState, Config, DbPool, ClusterPools, RefreshState, ImportExportLock};
use kafka_manager_api::kafka::KafkaClients;
use arc_swap::ArcSwap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("Kafka Manager (New Layout Test) starting...");

    // 初始化应用状态
    let state = initialize_app_state().await?;

    // 创建新的 Slint 应用
    let app = AppNew::new()?;

    // 设置窗口标题
    app.set_window_title("Kafka Manager - New Layout Test".into());

    // 设置测试数据
    let test_clusters = vec![
        ClusterNode {
            name: "Production Kafka".into(),
            bootstrap_servers: "localhost:9092".into(),
            status: "connected".into(),
            topics_count: 10,
            consumer_groups_count: 5,
            is_expanded: false,
            is_selected: false,
        },
        ClusterNode {
            name: "Development Kafka".into(),
            bootstrap_servers: "localhost:9093".into(),
            status: "disconnected".into(),
            topics_count: 5,
            consumer_groups_count: 2,
            is_expanded: false,
            is_selected: false,
        },
        ClusterNode {
            name: "Test Kafka".into(),
            bootstrap_servers: "localhost:9094".into(),
            status: "error".into(),
            topics_count: 3,
            consumer_groups_count: 1,
            is_expanded: false,
            is_selected: false,
        },
    ];

    app.set_cluster_nodes(slint::ModelRc::new(slint::VecModel::from(test_clusters)));

    // 设置测试收藏数据
    let test_favorites = vec![
        FavoriteItem {
            id: 1,
            topic_name: "user-events".into(),
            cluster_id: "Production Kafka".into(),
            group_id: 1,
        },
        FavoriteItem {
            id: 2,
            topic_name: "order-updates".into(),
            cluster_id: "Production Kafka".into(),
            group_id: 1,
        },
    ];

    app.set_favorite_items(slint::ModelRc::new(slint::VecModel::from(test_favorites)));

    // 设置国际化字符串（中文）
    app.set_i18n_clusters("集群".into());
    app.set_i18n_topics("主题".into());
    app.set_i18n_messages("消息".into());
    app.set_i18n_consumer_groups("消费组".into());
    app.set_i18n_favorites("收藏夹".into());
    app.set_i18n_settings("设置".into());
    app.set_language("zh".into());

    println!("New layout test initialized successfully");
    println!("Test clusters: 3");
    println!("Test favorites: 2");

    // 运行应用
    app.run()?;

    Ok(())
}

/// 初始化应用状态（简化版本，仅用于测试）
async fn initialize_app_state() -> Result<Arc<RwLock<AppState>>, Box<dyn std::error::Error>> {
    let db_url = "sqlite::memory:?mode=rwc";
    let db = DbPool::new(db_url).await?;

    let config = Config::default();
    let clients = Arc::new(ArcSwap::new(Arc::new(KafkaClients::default())));
    let pools = ClusterPools::new();

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