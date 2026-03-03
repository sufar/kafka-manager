use crate::error::{AppError, Result};
use crate::AppState;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

pub fn routes() -> Router<AppState> {
    Router::new().route("/stats", get(get_cluster_stats))
}

#[derive(Debug, Serialize)]
pub struct ClusterStatsResponse {
    pub cluster_id: String,
    pub broker_count: i32,
    pub controller_id: Option<i32>,
    pub topic_count: i32,
    pub partition_count: i32,
    pub under_replicated_partitions: i32,
    pub consumer_group_count: i32,
    pub total_lag: i64,
    pub broker_stats: Vec<BrokerStats>,
}

#[derive(Debug, Serialize)]
pub struct BrokerStats {
    pub id: i32,
    pub host: String,
    pub port: i32,
    pub is_controller: bool,
    pub leader_partitions: i32,
    pub replica_partitions: i32,
}

async fn get_cluster_stats(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ClusterStatsResponse>> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    // 获取集群信息
    let cluster_info = admin.get_cluster_info()?;

    // 获取所有 topic 的分区信息
    let topics = admin.list_topics()?;
    let mut partition_count = 0;
    let mut under_replicated = 0;
    let mut broker_leader_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
    let mut broker_replica_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();

    for topic in &topics {
        let topic_info = admin.get_topic_info(topic)?;
        for partition in &topic_info.partitions {
            partition_count += 1;

            // 检查是否未完全复制
            if partition.isr.len() < partition.replicas.len() {
                under_replicated += 1;
            }

            // 统计 leader 和 replica
            let leader = partition.leader;
            *broker_leader_counts.entry(leader).or_insert(0) += 1;
            for replica in &partition.replicas {
                *broker_replica_counts.entry(*replica).or_insert(0) += 1;
            }
        }
    }

    // 构建 broker 统计
    let broker_stats: Vec<BrokerStats> = cluster_info
        .brokers
        .iter()
        .map(|b| {
            let is_controller = cluster_info.controller_id == Some(b.id);
            BrokerStats {
                id: b.id,
                host: b.host.clone(),
                port: b.port,
                is_controller,
                leader_partitions: *broker_leader_counts.get(&b.id).unwrap_or(&0),
                replica_partitions: *broker_replica_counts.get(&b.id).unwrap_or(&0),
            }
        })
        .collect();

    // 注意：由于 rdkafka 限制，consumer group 数量暂时返回 0
    // 实际应用中可以通过 offsets topic 来估算
    let consumer_group_count = 0;
    let total_lag = 0;

    Ok(Json(ClusterStatsResponse {
        cluster_id,
        broker_count: cluster_info.brokers.len() as i32,
        controller_id: cluster_info.controller_id,
        topic_count: topics.len() as i32,
        partition_count,
        under_replicated_partitions: under_replicated,
        consumer_group_count,
        total_lag,
        broker_stats,
    }))
}
