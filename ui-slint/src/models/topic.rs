// Topic 数据模型

use serde::{Deserialize, Serialize};

/// Topic 数据结构（用于 Slint UI）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TopicData {
    pub name: String,
    pub cluster_id: String,
    pub partitions_count: i32,
    pub replication_factor: i32,
    pub is_internal: bool,
}

/// 分区 Offset 信息
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartitionOffset {
    pub partition: i32,
    pub begin_offset: i64,
    pub end_offset: i64,
}

/// Topic 详情（包含分区信息）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TopicDetailData {
    pub topic_name: String,
    pub partition_count: i32,
    pub replication_factor: i32,
    pub partitions: Vec<TopicPartitionData>,
}

/// Topic 分区详情
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TopicPartitionData {
    pub partition: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
}

// 从核心模型转换
impl From<kafka_manager_api::db::topic::TopicMetadata> for TopicData {
    fn from(topic: kafka_manager_api::db::topic::TopicMetadata) -> Self {
        Self {
            name: topic.topic_name,
            cluster_id: topic.cluster_id,
            partitions_count: topic.partition_count,
            replication_factor: topic.replication_factor,
            is_internal: false, // 需要从 Kafka metadata 获取
        }
    }
}

impl From<kafka_manager_api::db::topic::TopicPartitionDetail> for TopicPartitionData {
    fn from(partition: kafka_manager_api::db::topic::TopicPartitionDetail) -> Self {
        Self {
            partition: partition.partition,
            leader: partition.leader,
            replicas: partition.replicas,
            isr: partition.isr,
        }
    }
}

impl From<kafka_manager_api::db::topic::TopicDetail> for TopicDetailData {
    fn from(detail: kafka_manager_api::db::topic::TopicDetail) -> Self {
        Self {
            topic_name: detail.topic_name,
            partition_count: detail.partition_count,
            replication_factor: detail.replication_factor,
            partitions: detail.partitions.into_iter().map(TopicPartitionData::from).collect(),
        }
    }
}