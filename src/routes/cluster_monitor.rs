use crate::error::AppError;
use crate::models::{
    BrokerDetailResponse, BrokerInfo, BrokerListResponse, ClusterInfoResponse, ClusterMetricsResponse,
};
use crate::AppState;
use axum::{
    extract::{Path, State},
    Json, Router,
};
use axum::routing::get;
use serde::Serialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:cluster_id/info", get(get_cluster_info))
        .route("/:cluster_id/metrics", get(get_cluster_metrics))
        .route("/:cluster_id/brokers", get(list_brokers))
        .route("/:cluster_id/brokers/:broker_id", get(get_broker))
        .route("/:cluster_id/brokers/:broker_id/logdirs", get(get_broker_logdirs))
        .route("/:cluster_id/brokers/:broker_id/metrics", get(get_broker_metrics))
}

/// 获取集群信息
async fn get_cluster_info(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ClusterInfoResponse>, AppError> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let info = tokio::task::spawn_blocking(move || admin.get_cluster_info())
        .await
        .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(Json(ClusterInfoResponse {
        brokers: info.brokers
            .into_iter()
            .map(|b| BrokerInfo {
                id: b.id,
                host: b.host,
                port: b.port,
            })
            .collect(),
        controller_id: info.controller_id,
        cluster_id: info.cluster_id,
        topic_count: info.topic_count,
        total_partitions: info.total_partitions,
    }))
}

/// 获取集群监控指标
async fn get_cluster_metrics(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<ClusterMetricsResponse>, AppError> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let metrics = tokio::task::spawn_blocking(move || admin.get_cluster_metrics())
        .await
        .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(Json(ClusterMetricsResponse {
        broker_count: metrics.broker_count,
        controller_id: metrics.controller_id,
        topic_count: metrics.topic_count,
        partition_count: metrics.partition_count,
        under_replicated_partitions: metrics.under_replicated_partitions,
    }))
}

/// 列出所有 Broker
async fn list_brokers(
    State(state): State<AppState>,
    Path(cluster_id): Path<String>,
) -> Result<Json<BrokerListResponse>, AppError> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let info = tokio::task::spawn_blocking(move || admin.get_cluster_info())
        .await
        .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(Json(BrokerListResponse {
        brokers: info.brokers
            .into_iter()
            .map(|b| BrokerInfo {
                id: b.id,
                host: b.host,
                port: b.port,
            })
            .collect(),
    }))
}

/// 获取 Broker 详情
async fn get_broker(
    State(state): State<AppState>,
    Path((cluster_id, broker_id)): Path<(String, i32)>,
) -> Result<Json<BrokerDetailResponse>, AppError> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let broker = tokio::task::spawn_blocking(move || admin.get_broker_info(broker_id))
        .await
        .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(Json(BrokerDetailResponse {
        id: broker.id,
        host: broker.host,
        port: broker.port,
        is_controller: broker.is_controller,
        leader_partitions: broker.leader_partitions,
        replica_partitions: broker.replica_partitions,
    }))
}

/// 获取 Broker 日志目录
#[derive(Debug, Serialize)]
pub struct BrokerLogDirsResponse {
    pub broker_id: i32,
    pub log_dirs: Vec<LogDirInfo>,
}

#[derive(Debug, Serialize)]
pub struct LogDirInfo {
    pub path: String,
    pub size_bytes: Option<i64>,
    pub topic_count: i32,
    pub partition_count: i32,
}

async fn get_broker_logdirs(
    State(state): State<AppState>,
    Path((cluster_id, broker_id)): Path<(String, i32)>,
) -> Result<Json<BrokerLogDirsResponse>, AppError> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    // 在阻塞线程中执行所有 Kafka 操作
    let admin = admin.clone();
    let log_dirs = tokio::task::spawn_blocking(move || -> Result<Vec<LogDirInfo>, AppError> {
        // 获取 broker 信息
        let _broker = admin.get_broker_info(broker_id)?;

        // 获取所有 topic 的日志目录信息
        let topics = admin.list_topics()?;
        let mut log_dir_map: std::collections::HashMap<String, (i64, i32, i32)> = std::collections::HashMap::new();

        for topic in &topics {
            let topic_info = admin.get_topic_info(topic)?;
            for partition in &topic_info.partitions {
                // 只统计指定 broker 的分区
                if partition.leader == broker_id || partition.replicas.contains(&broker_id) {
                    // 由于 rdkafka 限制，无法获取实际的日志目录信息
                    // 这里返回模拟数据
                    let entry = log_dir_map.entry("/var/kafka-logs".to_string())
                        .or_insert((0, 0, 0));
                    entry.1 += 1; // topic count
                    entry.2 += 1; // partition count
                }
            }
        }

        let log_dirs: Vec<LogDirInfo> = log_dir_map
            .into_iter()
            .map(|(path, (size, topics, partitions))| LogDirInfo {
                path,
                size_bytes: Some(size),
                topic_count: topics,
                partition_count: partitions,
            })
            .collect();

        Ok(log_dirs)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(Json(BrokerLogDirsResponse {
        broker_id,
        log_dirs,
    }))
}

/// 获取 Broker 指标
#[derive(Debug, Serialize)]
pub struct BrokerMetricsResponse {
    pub broker_id: i32,
    pub leader_partitions: i32,
    pub replica_partitions: i32,
    pub is_controller: bool,
    pub under_replicated_count: i32,
    pub offline_partitions: i32,
}

async fn get_broker_metrics(
    State(state): State<AppState>,
    Path((cluster_id, broker_id)): Path<(String, i32)>,
) -> Result<Json<BrokerMetricsResponse>, AppError> {
    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    // 在阻塞线程中执行所有 Kafka 操作
    let admin = admin.clone();
    let (broker, under_replicated_count, offline_partitions) = tokio::task::spawn_blocking(move || -> Result<(crate::kafka::admin::BrokerDetail, i32, i32), AppError> {
        let broker = admin.get_broker_info(broker_id)?;

        // 统计未完全复制的分区
        let topics = admin.list_topics()?;
        let mut under_replicated_count = 0;
        let mut offline_partitions = 0;

        for topic in &topics {
            let topic_info = admin.get_topic_info(topic)?;
            for partition in &topic_info.partitions {
                if partition.isr.len() < partition.replicas.len() {
                    under_replicated_count += 1;
                }
                if partition.isr.is_empty() {
                    offline_partitions += 1;
                }
            }
        }

        Ok((broker, under_replicated_count, offline_partitions))
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(Json(BrokerMetricsResponse {
        broker_id,
        leader_partitions: broker.leader_partitions,
        replica_partitions: broker.replica_partitions,
        is_controller: broker.is_controller,
        under_replicated_count,
        offline_partitions,
    }))
}
