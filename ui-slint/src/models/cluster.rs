// 集群数据模型

use slint::Model;
use serde::{Deserialize, Serialize};

/// 集群数据结构（用于 Slint UI）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClusterData {
    pub id: i32,
    pub name: String,
    pub bootstrap_servers: String,
    pub status: String,  // "connected" / "disconnected" / "error"
    pub topics_count: i32,
    pub partitions_count: i32,
    pub group_id: Option<i32>,
}

// 从核心模型转换
impl From<kafka_manager_api::db::cluster::KafkaCluster> for ClusterData {
    fn from(cluster: kafka_manager_api::db::cluster::KafkaCluster) -> Self {
        Self {
            id: cluster.id as i32,
            name: cluster.name,
            bootstrap_servers: cluster.brokers,
            status: "disconnected".to_string(), // 初始状态，需要连接后更新
            topics_count: 0,
            partitions_count: 0,
            group_id: cluster.group_id.map(|g| g as i32),
        }
    }
}

// 从 ClusterData 转换为 Slint 的 ClusterInfo（自动生成）
impl From<ClusterData> for crate::ClusterInfo {
    fn from(data: ClusterData) -> Self {
        Self {
            id: data.id,
            name: slint::SharedString::from(data.name),
            bootstrap_servers: slint::SharedString::from(data.bootstrap_servers),
            status: slint::SharedString::from(data.status),
            topics_count: data.topics_count,
            partitions_count: data.partitions_count,
        }
    }
}

/// 创建集群请求（用于 UI）
#[derive(Clone, Debug, Default)]
pub struct CreateClusterRequest {
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i32,
    pub operation_timeout_ms: i32,
    pub group_id: Option<i32>,
}

impl From<CreateClusterRequest> for kafka_manager_api::db::cluster::CreateClusterRequest {
    fn from(req: CreateClusterRequest) -> Self {
        Self {
            name: req.name,
            brokers: req.brokers,
            request_timeout_ms: req.request_timeout_ms as i64,
            operation_timeout_ms: req.operation_timeout_ms as i64,
            group_id: req.group_id.map(|g| g as i64),
        }
    }
}