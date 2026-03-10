use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use rdkafka::admin::{AdminClient, AdminOptions, NewPartitions, NewTopic, ResourceSpecifier};
use rdkafka::client::DefaultClientContext;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::ClientConfig;
use rdkafka::Message;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct KafkaAdmin {
    client: Arc<AdminClient<DefaultClientContext>>,
    timeout: Duration,
}

impl KafkaAdmin {
    pub fn new(kafka_config: &KafkaConfig) -> Result<Self> {
        let client_config = super::create_client_config(kafka_config);
        let client = Arc::new(client_config.create()?);
        let timeout = Duration::from_millis(kafka_config.operation_timeout_ms as u64);

        Ok(Self { client, timeout })
    }

    /// 列出所有 Topic
    pub fn list_topics(&self) -> Result<Vec<String>> {
        let metadata = self.client.inner().fetch_metadata(None, self.timeout)?;
        Ok(metadata
            .topics()
            .iter()
            .map(|t| t.name().to_string())
            .collect())
    }

    /// 获取 Topic 详情
    pub fn get_topic_info(&self, topic_name: &str) -> Result<TopicInfo> {
        let metadata = self.client.inner().fetch_metadata(None, self.timeout)?;

        let topic_metadata = metadata
            .topics()
            .iter()
            .find(|t| t.name() == topic_name)
            .ok_or_else(|| AppError::NotFound(format!("Topic '{}' not found", topic_name)))?;

        let partitions: Vec<PartitionInfo> = topic_metadata
            .partitions()
            .iter()
            .map(|p| PartitionInfo {
                id: p.id(),
                leader: p.leader(),
                replicas: p.replicas().to_vec(),
                isr: p.isr().to_vec(),
            })
            .collect();

        Ok(TopicInfo {
            name: topic_name.to_string(),
            partitions,
        })
    }

    /// 创建 Topic
    pub async fn create_topic(
        &self,
        name: &str,
        num_partitions: i32,
        replication_factor: i32,
        config: HashMap<String, String>,
    ) -> Result<()> {
        use rdkafka::admin::TopicReplication;

        let new_topic = NewTopic::new(
            name,
            num_partitions,
            TopicReplication::Fixed(replication_factor),
        );

        // 添加配置
        let topic_with_config = config
            .iter()
            .fold(new_topic, |topic, (key, value)| topic.set(key, value));

        let result = self
            .client
            .create_topics(
                &[topic_with_config],
                &AdminOptions::new().operation_timeout(Some(self.timeout)),
            )
            .await?;

        for res in result {
            if let Err(e) = res {
                return Err(AppError::Internal(format!("Kafka error: {:?}", e)));
            }
        }

        Ok(())
    }

    /// 删除 Topic
    pub async fn delete_topic(&self, name: &str) -> Result<()> {
        let result = self
            .client
            .delete_topics(
                &[name],
                &AdminOptions::new().operation_timeout(Some(self.timeout)),
            )
            .await?;

        for res in result {
            if let Err(e) = res {
                return Err(AppError::Internal(format!("Kafka error: {:?}", e)));
            }
        }

        Ok(())
    }

    /// 增加分区
    pub async fn create_partitions(&self, topic: &str, new_partitions: i32) -> Result<()> {
        let partitions = NewPartitions::new(topic, new_partitions.try_into().unwrap_or(1));

        let result = self
            .client
            .create_partitions(
                &[partitions],
                &AdminOptions::new().operation_timeout(Some(self.timeout)),
            )
            .await?;

        for res in result {
            if let Err(e) = res {
                return Err(AppError::Internal(format!("Kafka error: {:?}", e)));
            }
        }

        Ok(())
    }

    /// 获取 Topic 配置
    pub async fn get_topic_config(&self, topic_name: &str) -> Result<HashMap<String, String>> {
        let resources = [ResourceSpecifier::Topic(topic_name)];

        let result = self
            .client
            .describe_configs(
                &resources,
                &AdminOptions::new().request_timeout(Some(self.timeout)),
            )
            .await?;

        let config_map = result
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound(format!("Topic '{}' not found", topic_name)))?
            .map_err(|e| AppError::Internal(format!("Kafka error: {:?}", e)))?;

        Ok(config_map
            .entries
            .iter()
            .filter_map(|e| e.value.as_ref().map(|v| (e.name.clone(), v.clone())))
            .collect())
    }

    /// 修改 Topic 配置
    pub async fn alter_topic_config(
        &self,
        topic_name: &str,
        config_changes: HashMap<String, String>,
    ) -> Result<()> {
        use rdkafka::admin::{AlterConfig, ResourceSpecifier};

        let alter_config = AlterConfig::new(ResourceSpecifier::Topic(topic_name));
        let alter_config = config_changes.iter().fold(alter_config, |config, (key, value)| {
            config.set(key, value)
        });

        let result = self
            .client
            .alter_configs(
                &[alter_config],
                &AdminOptions::new().operation_timeout(Some(self.timeout)),
            )
            .await?;

        for res in result {
            if let Err(e) = res {
                return Err(AppError::Internal(format!("Kafka error: {:?}", e)));
            }
        }

        Ok(())
    }

    /// 列出所有 Consumer Groups
    pub fn list_consumer_groups(&self, _kafka_config: &KafkaConfig) -> Result<Vec<ConsumerGroupInfo>> {
        // 使用 rdkafka 的 fetch_group_list API 获取所有 consumer groups
        // 当 group 参数为 None 时，会返回所有组
        let group_list = self.client.inner().fetch_group_list(None, self.timeout)
            .map_err(|e| AppError::Internal(format!("Failed to fetch consumer groups: {}", e)))?;

        let groups = group_list.groups()
            .iter()
            .map(|g| ConsumerGroupInfo {
                name: g.name().to_string(),
                state: g.state().to_string(),
                protocol: g.protocol().to_string(),
                protocol_type: g.protocol_type().to_string(),
            })
            .collect();

        Ok(groups)
    }

    /// 获取 Consumer Group 详情
    pub fn get_consumer_group_info(&self, kafka_config: &KafkaConfig, group_name: &str) -> Result<ConsumerGroupDetail> {
        use rdkafka::consumer::{Consumer, DefaultConsumerContext};
        use rdkafka::topic_partition_list::TopicPartitionList;
        use rdkafka::ClientConfig;

        // 创建一个临时的 consumer 来获取 group 信息
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", "kafka-manager-temp");
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer<DefaultConsumerContext> = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        // 获取 consumer group 的 committed offsets
        let mut tpl = TopicPartitionList::new();

        // 尝试获取 __consumer_offsets topic 的信息
        let _metadata = self.client.inner().fetch_metadata(
            Some("__consumer_offsets"),
            self.timeout,
        );

        // 尝试获取 topic 列表
        let all_metadata = self.client.inner().fetch_metadata(None, self.timeout)?;

        // 收集所有 topic 和分区
        for topic in all_metadata.topics() {
            for partition in topic.partitions() {
                tpl.add_partition(topic.name(), partition.id());
            }
        }

        // 获取 committed offsets
        let committed = consumer.committed_offsets(tpl.clone(), self.timeout);

        // 过滤出指定 group 的 offsets
        let mut topic_partitions = Vec::new();

        if let Ok(offsets) = committed {
            for element in offsets.elements() {
                if let Some(offset) = element.offset().to_raw() {
                    if offset >= 0 {
                        topic_partitions.push((element.topic().to_string(), element.partition()));
                    }
                }
            }
        }

        // 由于无法直接获取 group 状态和成员信息，返回简化数据
        let state = if topic_partitions.is_empty() {
            "Empty".to_string()
        } else {
            "Unknown".to_string()
        };

        Ok(ConsumerGroupDetail {
            name: group_name.to_string(),
            state,
            protocol: "Unknown".to_string(),
            protocol_type: "Unknown".to_string(),
            members: vec![],
        })
    }

    /// 删除 Consumer Group
    pub async fn delete_consumer_group(&self, group_name: &str) -> Result<()> {
        let result = self
            .client
            .delete_groups(
                &[group_name],
                &AdminOptions::new().operation_timeout(Some(self.timeout)),
            )
            .await?;

        for res in result {
            if let Err(e) = res {
                return Err(AppError::Internal(format!("Kafka error: {:?}", e)));
            }
        }

        Ok(())
    }

    /// 重置 Consumer Group 在所有分区上的 offset
    pub async fn reset_consumer_group_offsets(
        &self,
        kafka_config: &KafkaConfig,
        group_id: &str,
        topic: &str,
        offset: OffsetType,
    ) -> Result<()> {
        use rdkafka::consumer::CommitMode;

        // 创建临时 consumer
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        let offset_to_set = match offset {
            OffsetType::Earliest => Offset::Beginning,
            OffsetType::Latest => Offset::End,
            OffsetType::Value(val) => Offset::Offset(val),
            OffsetType::Timestamp(_) => {
                return Err(AppError::BadRequest("Timestamp offset reset must be done per partition".to_string()));
            }
        };

        // 获取 topic 的分区列表
        let metadata = consumer
            .client()
            .fetch_metadata(Some(topic), self.timeout)
            .map_err(|e| AppError::Internal(format!("Failed to fetch metadata: {}", e)))?;

        let topic_metadata = metadata
            .topics()
            .iter()
            .find(|t: &&rdkafka::metadata::MetadataTopic| t.name() == topic)
            .ok_or_else(|| AppError::NotFound(format!("Topic '{}' not found", topic)))?;

        let mut tpl = TopicPartitionList::new();
        for partition in topic_metadata.partitions() {
            tpl.add_partition(topic, partition.id());
        }

        // 设置所有分区的 offset
        tpl.set_all_offsets(offset_to_set)
            .map_err(|e| AppError::Internal(format!("Failed to set offset: {}", e)))?;

        consumer
            .commit(&tpl, CommitMode::Sync)
            .map_err(|e| AppError::Internal(format!("Failed to commit offset: {:?}", e)))?;

        Ok(())
    }

    /// 重置 Consumer Group 指定分区的 offset
    pub async fn reset_consumer_group_offset(
        &self,
        kafka_config: &KafkaConfig,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: OffsetType,
    ) -> Result<()> {
        use rdkafka::consumer::CommitMode;

        // 创建临时 consumer
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &kafka_config.brokers);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        let offset_to_set = match offset {
            OffsetType::Earliest => Offset::Beginning,
            OffsetType::Latest => Offset::End,
            OffsetType::Value(val) => Offset::Offset(val),
            OffsetType::Timestamp(ts) => {
                // 通过时间戳查找 offset
                let ts_offset = self.find_offset_by_timestamp(&consumer, kafka_config, topic, partition, ts).await?;
                Offset::Offset(ts_offset)
            }
        };

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition(topic, partition);
        tpl.set_all_offsets(offset_to_set)
            .map_err(|e| AppError::Internal(format!("Failed to set offset: {}", e)))?;

        consumer
            .commit(&tpl, CommitMode::Sync)
            .map_err(|e| AppError::Internal(format!("Failed to commit offset: {:?}", e)))?;

        Ok(())
    }

    /// 通过时间戳查找 offset
    async fn find_offset_by_timestamp(
        &self,
        consumer: &BaseConsumer,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: i32,
        timestamp: i64,
    ) -> Result<i64> {
        // 获取 earliest 和 latest offset
        let (low, high) = consumer
            .client()
            .fetch_watermarks(topic, partition, self.timeout)
            .map_err(|e| AppError::Internal(format!("Failed to fetch watermarks: {}", e)))?;

        // 二分查找最接近时间戳的 offset
        let mut left = low;
        let mut right = high;

        while left < right {
            let mid = left + (right - left) / 2;
            let msg = self.get_message_at_offset(consumer, kafka_config, topic, partition, mid).await;

            match msg {
                Some(m) => {
                    match m.timestamp {
                        Some(ts) if ts < timestamp => left = mid + 1,
                        Some(ts) if ts >= timestamp => {
                            if mid == low {
                                return Ok(mid);
                            }
                            right = mid;
                        }
                        _ => left = mid + 1,
                    }
                }
                None => left = mid + 1,
            }
        }

        Ok(left)
    }

    /// 获取指定 offset 的消息
    async fn get_message_at_offset(
        &self,
        _consumer: &BaseConsumer,
        kafka_config: &KafkaConfig,
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> Option<crate::kafka::consumer::KafkaMessage> {
        use rdkafka::consumer::DefaultConsumerContext;

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, Offset::Offset(offset)).ok()?;

        let temp_consumer: BaseConsumer<DefaultConsumerContext> =
            ClientConfig::new()
                .set("bootstrap.servers", &kafka_config.brokers)
                .set("enable.auto.commit", "false")
                .create()
                .ok()?;

        temp_consumer.assign(&tpl).ok()?;

        match temp_consumer.poll(std::time::Duration::from_millis(100)) {
            Some(Ok(msg)) => Some(crate::kafka::consumer::KafkaMessage {
                partition: msg.partition(),
                offset: msg.offset(),
                key: msg.key().and_then(|k| std::str::from_utf8(k).ok().map(String::from)),
                value: msg.payload().and_then(|p| std::str::from_utf8(p).ok().map(String::from)),
                timestamp: msg.timestamp().to_millis(),
            }),
            _ => None,
        }
    }

    /// 获取集群信息（Brokers、Controller 等）
    pub fn get_cluster_info(&self) -> Result<ClusterInfo> {
        let metadata = self.client.inner().fetch_metadata(None, self.timeout)?;

        let brokers: Vec<BrokerInfo> = metadata
            .brokers()
            .iter()
            .map(|b| BrokerInfo {
                id: b.id(),
                host: b.host().to_string(),
                port: b.port() as i32,
            })
            .collect();

        let controller_id = None;
        let cluster_id = None;

        let topic_count = metadata.topics().len();
        let total_partitions: i32 = metadata
            .topics()
            .iter()
            .map(|t| t.partitions().len() as i32)
            .sum();

        Ok(ClusterInfo {
            brokers,
            controller_id,
            cluster_id,
            topic_count: topic_count as i32,
            total_partitions,
        })
    }

    /// 获取 Broker 详情
    pub fn get_broker_info(&self, broker_id: i32) -> Result<BrokerDetail> {
        let metadata = self.client.inner().fetch_metadata(None, self.timeout)?;

        let broker = metadata
            .brokers()
            .iter()
            .find(|b| b.id() == broker_id)
            .ok_or_else(|| AppError::NotFound(format!("Broker {} not found", broker_id)))?;

        let is_controller = false;

        let leader_partitions: i32 = metadata
            .topics()
            .iter()
            .flat_map(|t| t.partitions())
            .filter(|p| p.leader() == broker_id)
            .count() as i32;

        let replica_partitions: i32 = metadata
            .topics()
            .iter()
            .flat_map(|t| t.partitions())
            .filter(|p| p.replicas().contains(&broker_id))
            .count() as i32;

        Ok(BrokerDetail {
            id: broker.id(),
            host: broker.host().to_string(),
            port: broker.port() as i32,
            is_controller,
            leader_partitions,
            replica_partitions,
        })
    }

    /// 获取集群监控指标
    pub fn get_cluster_metrics(&self) -> Result<ClusterMetrics> {
        let metadata = self.client.inner().fetch_metadata(None, self.timeout)?;

        let topic_count = metadata.topics().len();
        let partition_count: i32 = metadata
            .topics()
            .iter()
            .map(|t| t.partitions().len() as i32)
            .sum();

        let broker_count = metadata.brokers().len() as i32;
        let controller_id = None;

        let under_replicated_partitions: i32 = metadata
            .topics()
            .iter()
            .flat_map(|t| t.partitions())
            .filter(|p| p.isr().len() < p.replicas().len())
            .count() as i32;

        Ok(ClusterMetrics {
            broker_count,
            controller_id,
            topic_count: topic_count as i32,
            partition_count,
            under_replicated_partitions,
        })
    }

    /// 获取分区 watermarks（低水位和高水位）
    pub fn get_partition_watermarks(&self, topic: &str, partition: i32, bootstrap_servers: &str) -> Result<(i64, i64)> {
        // 创建临时 consumer 获取 watermarks
        use rdkafka::consumer::BaseConsumer;
        use rdkafka::ClientConfig;

        let consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap_servers)
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        consumer
            .client()
            .fetch_watermarks(topic, partition, self.timeout)
            .map_err(|e| AppError::Internal(format!("Failed to fetch watermarks: {}", e)))
    }
}

/// Offset 类型
#[derive(Debug, Clone)]
pub enum OffsetType {
    Earliest,
    Latest,
    Value(i64),
    Timestamp(i64),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TopicInfo {
    pub name: String,
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PartitionInfo {
    pub id: i32,
    pub leader: i32,
    pub replicas: Vec<i32>,
    pub isr: Vec<i32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConsumerGroupInfo {
    pub name: String,
    pub state: String,
    pub protocol: String,
    pub protocol_type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConsumerGroupDetail {
    pub name: String,
    pub state: String,
    pub protocol: String,
    pub protocol_type: String,
    pub members: Vec<GroupMemberInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GroupMemberInfo {
    pub client_id: String,
    pub host: String,
    pub topic_partitions: Vec<(String, i32)>,
}

/// 集群信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClusterInfo {
    pub brokers: Vec<BrokerInfo>,
    pub controller_id: Option<i32>,
    pub cluster_id: Option<String>,
    pub topic_count: i32,
    pub total_partitions: i32,
}

/// Broker 信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrokerInfo {
    pub id: i32,
    pub host: String,
    pub port: i32,
}

/// Broker 详情
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrokerDetail {
    pub id: i32,
    pub host: String,
    pub port: i32,
    pub is_controller: bool,
    pub leader_partitions: i32,
    pub replica_partitions: i32,
}

/// 集群监控指标
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClusterMetrics {
    pub broker_count: i32,
    pub controller_id: Option<i32>,
    pub topic_count: i32,
    pub partition_count: i32,
    pub under_replicated_partitions: i32,
}
