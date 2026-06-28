// Consumer Group 事件处理器

use kafka_manager_api::{AppState};
use slint::{ComponentHandle, Weak, Model};
use tokio::sync::RwLock;
use std::sync::Arc;

/// 从 Kafka 加载 Consumer Group 列表
pub async fn load_consumer_groups_from_kafka(
    state: Arc<RwLock<AppState>>,
    app: Weak<crate::App>,
    cluster_id: String,
) {
    println!("Loading consumer groups from Kafka for cluster: {}", cluster_id);

    // 获取应用状态（读锁）
    let state_guard = state.read().await;

    // 获取 Kafka Admin 客户端和配置
    let clients = state_guard.get_clients();
    let admin = clients.get_admin(&cluster_id);
    let config = clients.get_config(&cluster_id);

    if admin.is_none() || config.is_none() {
        eprintln!("Cluster {} not connected", cluster_id);

        // 更新 UI 显示空列表
        let _ = app.upgrade_in_event_loop(move |app| {
            use slint::{ModelRc, VecModel};
            app.set_consumer_groups(ModelRc::new(VecModel::default()));
        });
        return;
    }

    let admin = admin.unwrap();
    let config = config.unwrap();

    // 使用 Kafka AdminClient 获取 Consumer Group 列表
    let group_list = match admin.list_consumer_groups(&config.as_ref()) {
        Ok(groups) => groups,
        Err(e) => {
            eprintln!("Failed to load consumer groups from Kafka: {}", e);

            // 更新 UI 显示空列表
            let _ = app.upgrade_in_event_loop(move |app| {
                use slint::{ModelRc, VecModel};
                app.set_consumer_groups(ModelRc::new(VecModel::default()));
            });
            return;
        }
    };

    println!("Loaded {} consumer groups from Kafka", group_list.len());

    // 转换为 Slint 数据模型（基础信息，缺少成员数和 lag）
    let group_data: Vec<crate::ConsumerGroupInfo> = group_list
        .into_iter()
        .map(|g| crate::ConsumerGroupInfo {
            name: slint::SharedString::from(g.name),
            state: slint::SharedString::from(g.state),
            members_count: 0,  // list_consumer_groups 不返回成员数，后续可扩展
            lag: 0,            // 需要额外查询 offset，后续可扩展
        })
        .collect();

    // 更新 UI（回到主线程）
    let _ = app.upgrade_in_event_loop(move |app| {
        use slint::{ModelRc, VecModel, Model};

        let model = VecModel::from(group_data);
        let model_rc = ModelRc::new(model);
        let count = model_rc.row_count();

        app.set_consumer_groups(model_rc);

        println!("UI updated with {} consumer groups", count);
    });
}