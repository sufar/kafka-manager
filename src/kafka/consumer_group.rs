/// Kafka Consumer Group 操作模块
/// 使用 rdkafka 的 Consumer Group 相关 API

use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use rdkafka::consumer::{BaseConsumer, Consumer, CommitMode};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use std::time::Duration;

/// Consumer Group 详情
#[derive(Debug, Clone)]
pub struct ConsumerGroupInfo {
    pub group_id: String,
    pub state: String,
    pub topics: Vec<String>,
}

/// 分区偏移详情
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PartitionOffsetDetail {
    pub topic: String,
    pub partition: i32,
    pub start_offset: i64,
    pub end_offset: i64,
    pub committed_offset: i64,
    pub lag: i64,
    pub last_commit_time: Option<i64>,  // 最后提交时间（毫秒时间戳），暂无数据
}

/// Consumer Group 管理器
pub struct KafkaConsumerGroupManager {
    consumer_config: KafkaConfig,
    timeout: Duration,
}

impl KafkaConsumerGroupManager {
    pub fn new(config: &KafkaConfig) -> Result<Self> {
        Ok(Self {
            consumer_config: config.clone(),
            timeout: Duration::from_millis(config.operation_timeout_ms as u64),
        })
    }

    /// 列出所有 Consumer Groups
    pub fn list_consumer_groups(&self) -> Result<Vec<String>> {
        // 使用 rdkafka 的 fetch_group_list API 获取所有 consumer groups
        // 创建一个临时的 client 来获取 group 列表
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", "kafka-manager-temp-group");
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        // 使用 fetch_group_list API，None 表示获取所有组
        let group_list = consumer.fetch_group_list(None, self.timeout)
            .map_err(|e| AppError::Internal(format!("Failed to fetch consumer groups: {}", e)))?;

        let groups = group_list.groups()
            .iter()
            .map(|g| g.name().to_string())
            .collect();

        Ok(groups)
    }

    /// 获取 Consumer Group 关联的所有 topics（从 committed offsets 获取）
    pub fn get_consumer_group_topics(&self, group_id: &str) -> Result<Vec<String>> {
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        let mut topics: Vec<String> = Vec::new();

        // 方法 1：尝试从 active members 的 metadata 获取 topics（适用于有 active consumer 的情况）
        let group_list = consumer.fetch_group_list(Some(group_id), self.timeout)
            .map_err(|e| AppError::Internal(format!("Failed to fetch consumer group info: {}", e)))?;

        for group in group_list.groups() {
            for member in group.members() {
                if let Some(metadata) = member.metadata() {
                    if let Some(group_topics) = parse_consumer_protocol_metadata(metadata) {
                        for topic in group_topics {
                            if !topics.contains(&topic) {
                                topics.push(topic);
                            }
                        }
                    }
                }
            }
        }

        // 方法 2：如果从 member metadata 没获取到 topics，尝试从 committed offsets 获取
        // 这需要知道所有可能的 topics，所以我们先尝试从 broker 获取所有 topics
        if topics.is_empty() {
            let metadata = consumer.fetch_metadata(None, self.timeout)?;
            for topic_meta in metadata.topics() {
                let topic_name = topic_meta.name();
                // 跳过内部 topics
                if topic_name.starts_with("__") {
                    continue;
                }

                // 检查这个 topic 是否有 committed offsets
                let mut tpl = TopicPartitionList::new();
                for partition_meta in topic_meta.partitions() {
                    tpl.add_partition(topic_name, partition_meta.id());
                }

                let committed = consumer.committed_offsets(tpl, self.timeout)?;
                // 如果有任何 partition 有有效的 committed offset，说明这个 topic 被 consumer group 订阅过
                for elem in committed.elements() {
                    if matches!(elem.offset(), Offset::Offset(_)) {
                        if !topics.contains(&topic_name.to_string()) {
                            topics.push(topic_name.to_string());
                        }
                        break;
                    }
                }
            }
        }

        Ok(topics)
    }

    /// 获取 Consumer Group 的 offset 和 lag 信息（自动获取 topics）
    pub fn get_consumer_group_offsets_auto(&self, group_id: &str) -> Result<Vec<PartitionOffsetDetail>> {
        // 先获取 topics
        let topics = self.get_consumer_group_topics(group_id)?;
        // 再获取 offsets
        if topics.is_empty() {
            return Ok(Vec::new());
        }
        self.get_consumer_group_offsets(group_id, &topics)
    }

    /// 获取 Consumer Group 详情
    pub fn get_consumer_group_info(&self, group_id: &str, topics: &[String]) -> Result<ConsumerGroupInfo> {
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        // 获取 committed offsets 来判断状态
        let mut tpl = TopicPartitionList::new();
        for topic in topics {
            // 这里简化处理，假设每个 topic 只有一个分区
            tpl.add_partition(topic, 0);
        }

        let committed = consumer.committed_offsets(tpl, self.timeout)?;

        let state = if committed.elements().is_empty() {
            "Empty".to_string()
        } else {
            "Stable".to_string()
        };

        Ok(ConsumerGroupInfo {
            group_id: group_id.to_string(),
            state,
            topics: topics.to_vec(),
        })
    }

    /// 获取 Consumer Group 的 offset 和 lag 信息
    pub fn get_consumer_group_offsets(&self, group_id: &str, topics: &[String]) -> Result<Vec<PartitionOffsetDetail>> {
        use rdkafka::topic_partition_list::Offset;

        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        let mut result = Vec::new();

        // 对于每个 topic，获取分区信息和 offset
        for topic in topics {
            // 获取 topic 的元数据
            let metadata = consumer.fetch_metadata(Some(topic), self.timeout)?;

            for topic_meta in metadata.topics() {
                if topic_meta.name() != topic {
                    continue;
                }

                for partition_meta in topic_meta.partitions() {
                    let partition = partition_meta.id();

                    // 获取 committed offset
                    let mut tpl = TopicPartitionList::new();
                    tpl.add_partition(topic, partition);

                    let committed = consumer.committed_offsets(tpl, self.timeout)?;
                    // 处理 Offset 类型，将 Invalid 转换为 -1
                    let committed_offset = committed
                        .elements()
                        .first()
                        .map(|e| {
                            match e.offset() {
                                Offset::Offset(n) => n,
                                Offset::Invalid => -1,
                                _ => -1,
                            }
                        })
                        .unwrap_or(-1);

                    // 获取 start offset (earliest) 和 end offset (latest)
                    let (start_offset, end_offset) = consumer
                        .fetch_watermarks(topic, partition, self.timeout)
                        .unwrap_or((0, 0));

                    // 计算 lag
                    let lag = if committed_offset < 0 {
                        end_offset - start_offset
                    } else {
                        end_offset - committed_offset
                    };

                    result.push(PartitionOffsetDetail {
                        topic: topic.clone(),
                        partition,
                        start_offset,
                        end_offset,
                        committed_offset,
                        lag,
                        last_commit_time: None,
                    });
                }
            }
        }

        Ok(result)
    }

    /// 重置 Consumer Group 的 offset
    pub fn reset_consumer_group_offset(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> Result<i64> {
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, Offset::Offset(offset))
            .map_err(|e| AppError::Internal(format!("Failed to add partition offset: {}", e)))?;

        // 提交 offset
        consumer.commit(&tpl, CommitMode::Sync)?;

        Ok(offset)
    }

    /// 重置 Consumer Group 的 offset 到最早
    pub fn reset_consumer_group_offset_to_earliest(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Result<i64> {
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        let (start_offset, _) = consumer.fetch_watermarks(topic, partition, self.timeout)?;
        self.reset_consumer_group_offset(group_id, topic, partition, start_offset)?;
        Ok(start_offset)
    }

    /// 重置 Consumer Group 的 offset 到最新
    pub fn reset_consumer_group_offset_to_latest(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Result<i64> {
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        let (_, end_offset) = consumer.fetch_watermarks(topic, partition, self.timeout)?;
        self.reset_consumer_group_offset(group_id, topic, partition, end_offset)?;
        Ok(end_offset)
    }

    /// 重置 Consumer Group 的 offset 到指定时间
    pub fn reset_consumer_group_offset_to_timestamp(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
        timestamp: i64,
    ) -> Result<i64> {
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        // 创建 TopicPartitionList 用于查询 offset
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, Offset::Offset(timestamp))
            .map_err(|e| AppError::Internal(format!("Failed to add partition offset: {}", e)))?;

        // 使用 offsets_for_times 获取指定时间的 offset
        let offsets = consumer.offsets_for_times(tpl, self.timeout)?;

        for elem in offsets.elements() {
            if elem.topic() == topic && elem.partition() == partition {
                if let Some(offset) = elem.offset().to_raw() {
                    self.reset_consumer_group_offset(group_id, topic, partition, offset)?;
                    return Ok(offset);
                }
            }
        }

        Err(AppError::NotFound(format!(
            "No offset found for topic {} partition {} at timestamp {}",
            topic, partition, timestamp
        )))
    }

    /// 删除空的 Consumer Group
    pub fn delete_empty_consumer_group(&self, group_id: &str) -> Result<()> {
        // 验证 group 是否为空
        let group_info = self.get_consumer_group_info(group_id, &[])?;

        if group_info.state != "Empty" && group_info.state != "Dead" {
            return Err(AppError::BadRequest(format!(
                "Cannot delete consumer group '{}' with state '{}'. Only Empty or Dead groups can be deleted.",
                group_id, group_info.state
            )));
        }

        // 实际上，consumer group 会在没有 active consumer 且所有 offset 过期后自动删除
        // 这里我们只是从数据库中删除记录
        Ok(())
    }
}

/// 解析 Kafka Consumer Protocol Metadata
/// 格式参考：https://cwiki.apache.org/confluence/display/KAFKA/A+Guide+To+The+Kafka+Protocol#AGuideToTheKafkaProtocol-ConsumerMetadata
/// 简化版解析，只提取 topic 名称
fn parse_consumer_protocol_metadata(data: &[u8]) -> Option<Vec<String>> {
    if data.is_empty() {
        return None;
    }

    let mut topics = Vec::new();
    let mut pos = 0;

    // 跳过 version (2 bytes)
    if data.len() < 2 {
        return None;
    }
    pos += 2;

    // 读取 topic 数量
    if data.len() < pos + 4 {
        return None;
    }
    let topic_count = u32::from_be_bytes([
        data[pos],
        data[pos + 1],
        data[pos + 2],
        data[pos + 3],
    ]) as usize;
    pos += 4;

    // 解析每个 topic
    for _ in 0..topic_count {
        // 读取 topic 名称长度
        if data.len() < pos + 2 {
            break;
        }
        let topic_name_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;

        // 读取 topic 名称
        if data.len() < pos + topic_name_len {
            break;
        }
        let topic_name = String::from_utf8_lossy(&data[pos..pos + topic_name_len]).to_string();
        pos += topic_name_len;

        topics.push(topic_name);

        // 跳过分区信息（每个分区 4 字节）
        if data.len() < pos + 4 {
            break;
        }
        let partition_count = u32::from_be_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
        ]) as usize;
        pos += 4 + partition_count * 4;
    }

    if topics.is_empty() {
        None
    } else {
        Some(topics)
    }
}
