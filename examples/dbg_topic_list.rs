//! 调试：合成数据库验证 topic.list_with_cluster 响应
use kafka_manager_api::{api, AppState, ClusterPools, DbPool, Config, ImportExportLock, KafkaClients, RefreshState};
use arc_swap::ArcSwap;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let db_path = "/tmp/dbg_kafka_manager.db";
    let _ = std::fs::remove_file(db_path);
    let pool = DbPool::new(db_path).await.unwrap();
    pool.init().await.unwrap();

    // 插入测试集群
    kafka_manager_api::db::cluster::ClusterStore::create(pool.inner(), &kafka_manager_api::db::cluster::CreateClusterRequest {
        name: "test-cluster".to_string(),
        brokers: "localhost:9092".to_string(),
        request_timeout_ms: 5000,
        operation_timeout_ms: 5000,
        group_id: None,
    }).await.unwrap();

    // 插入 topics
    for i in 0..5 {
        kafka_manager_api::db::topic::TopicStore::upsert(
            pool.inner(),
            "test-cluster",
            &format!("topic-{}", i),
            3,
            1,
            &std::collections::HashMap::new(),
        ).await.unwrap();
    }

    let state = AppState {
        db: pool,
        clients: Arc::new(ArcSwap::new(Arc::new(KafkaClients::default()))),
        config: Config::default(),
        pools: ClusterPools::new(),
        refresh_state: Arc::new(Mutex::new(RefreshState::default())),
        import_export_lock: Arc::new(Mutex::new(ImportExportLock::default())),
    };

    // 模拟 navigator 调用：cluster_ids 为名称数组
    let result = api::dispatch_request(
        "topic.list_with_cluster",
        state.clone(),
        serde_json::json!({
            "cluster_ids": ["test-cluster"],
            "offset": 0,
            "limit": 1000,
            "search": "",
        }),
    ).await.unwrap();
    println!("=== with names cluster_ids ===");
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
