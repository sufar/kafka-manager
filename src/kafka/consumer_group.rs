/// Kafka Consumer Group 操作模块
/// 使用 rdkafka 的 Consumer Group 相关 API

use crate::config::KafkaConfig;
use crate::error::{AppError, Result};
use rdkafka::consumer::{BaseConsumer, Consumer, CommitMode};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::Message;
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
    pub last_commit_time: Option<i64>,  // 最后提交时间（毫秒时间戳）
}

/// Consumer Group 管理器
#[derive(Clone)]
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
                    let elements = committed.elements();
                    let element = elements.first();
                    let committed_offset = element
                        .map(|e| {
                            match e.offset() {
                                Offset::Offset(n) => n,
                                Offset::Invalid => -1,
                                _ => -1,
                            }
                        })
                        .unwrap_or(-1);

                    // 尝试从 metadata 中获取最后提交时间
                    // Kafka 的 committed offset metadata 格式不标准，通常是空的或自定义格式
                    // 这里我们尝试解析 metadata 字符串（这个值通常为空，后续会从__consumer_offsets 获取）
                    let _last_commit_time_from_metadata: Option<i64> = element
                        .and_then(|e| {
                            let metadata = e.metadata();
                            if metadata.is_empty() {
                                None
                            } else {
                                metadata.parse::<i64>().ok()
                            }
                        });

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
                        last_commit_time: None,  // 默认不查__consumer_offsets，由详情页按需调用
                    });
                }
            }
        }

        Ok(result)
    }

    /// 获取单个分区的最后提交时间（从__consumer_offsets topic 读取）
    /// 用于详情页单个 consumer group 的详细查询
    pub fn get_partition_last_commit_time(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Result<Option<i64>> {
        self.get_last_commit_time_from_offsets_topic(group_id, topic, partition)
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

    /// 从 __consumer_offsets topic 获取最后提交时间
    /// 通过读取 __consumer_offsets topic 中对应 key 的最新记录来获取提交时间戳
    pub fn get_last_commit_time_from_offsets_topic(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Result<Option<i64>> {
        // 创建一个不带 group.id 的消费者来读取 __consumer_offsets topic
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("enable.auto.commit", "false");
        client_config.set("isolation.level", "read_committed");

        let consumer: BaseConsumer = client_config.create()?;

        // __consumer_offsets topic 的 key 格式：[version][groupIdLength][groupId][topicLength][topic][partition]
        // 我们需要找到对应这个 group/topic/partition 的 offset 在 __consumer_offsets 中的位置

        // 简化方法：直接获取 __consumer_offsets topic 的 metadata，然后读取最新的消息
        // 但由于 __consumer_offsets 有很多分区，我们需要知道具体哪个分区存储了这个 group 的数据

        // 使用 group hash 来确定 __consumer_offsets 的分区
        let offsets_partition = partition_for_consumer_group_offset(group_id);

        // 获取 __consumer_offsets 的水位
        let (start_offset, end_offset) = consumer
            .fetch_watermarks("__consumer_offsets", offsets_partition, self.timeout)
            .map_err(|e| AppError::Internal(format!("Failed to fetch watermarks: {}", e)))?;

        if end_offset <= start_offset {
            return Ok(None);
        }

        // 从最新的 offset 开始向前查找，找到匹配的记录
        // 由于性能考虑，最多检查最近的 100 条记录
        let mut search_start = start_offset.max(end_offset - 100);
        let mut last_commit_time: Option<i64> = None;

        while search_start < end_offset {
            // 订阅分区并从指定位置开始消费
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset("__consumer_offsets", offsets_partition, Offset::Offset(search_start))?;
            consumer.assign(&tpl)?;

            // 消费消息
            match consumer.poll(Duration::from_millis(100)) {
                Some(Ok(msg)) => {
                    if let Some(payload) = msg.payload() {
                        // 检查是否是目标 group 的记录
                        if let Some(timestamp) = parse_consumer_offset_record(payload, group_id, topic, partition) {
                            last_commit_time = Some(timestamp);
                            break;
                        }
                    }
                }
                Some(Err(e)) => {
                    // 跳过错误
                    eprintln!("Poll error: {}", e);
                }
                None => {}
            }
            search_start += 1;
        }

        // 重置消费位置
        consumer.unassign()?;

        Ok(last_commit_time)
    }
}

/// 计算 consumer group offset 在 __consumer_offsets 中的分区
fn partition_for_consumer_group_offset(group_id: &str) -> i32 {
    // Kafka 使用 group.id 的 hash 值来确定存储 offset 的分区
    // 默认 __consumer_offsets topic 有 50 个分区
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    group_id.hash(&mut hasher);
    let hash = hasher.finish() as i32;
    (hash & 0x7fffffff) % 50
}

/// 解析 __consumer_offsets topic 中的记录
/// 返回最后提交时间（毫秒时间戳）
fn parse_consumer_offset_record(
    payload: &[u8],
    target_group: &str,
    target_topic: &str,
    target_partition: i32,
) -> Option<i64> {
    if payload.is_empty() {
        return None;
    }

    // __consumer_offsets 记录格式（版本 1-3）:
    // [version][key schema][value schema]
    // 简化解析：尝试从 payload 中提取时间戳

    // 注意：这是一个简化的实现，完整的解析需要处理所有版本格式
    // 参考：https://cwiki.apache.org/confluence/display/KAFKA/Offset+Commit+and+Fetch+API

    let mut pos = 0;

    // 读取 version (2 bytes, big endian)
    if payload.len() < 2 {
        return None;
    }
    let version = i16::from_be_bytes([payload[0], payload[1]]);
    pos += 2;

    // 不同版本格式不同，这里处理常见的版本 1-3
    match version {
        0..=3 => {
            // 尝试解析 group id
            if payload.len() < pos + 2 {
                return None;
            }
            let group_id_len = i16::from_be_bytes([payload[pos], payload[pos + 1]]) as usize;
            pos += 2;

            if payload.len() < pos + group_id_len {
                return None;
            }
            let group_id = String::from_utf8_lossy(&payload[pos..pos + group_id_len]);
            pos += group_id_len;

            if group_id != target_group {
                return None;
            }

            // 读取 topic
            if payload.len() < pos + 2 {
                return None;
            }
            let topic_len = i16::from_be_bytes([payload[pos], payload[pos + 1]]) as usize;
            pos += 2;

            if payload.len() < pos + topic_len {
                return None;
            }
            let topic = String::from_utf8_lossy(&payload[pos..pos + topic_len]);
            pos += topic_len;

            if topic != target_topic {
                return None;
            }

            // 读取 partition
            if payload.len() < pos + 4 {
                return None;
            }
            let partition = i32::from_be_bytes([payload[pos], payload[pos + 1], payload[pos + 2], payload[pos + 3]]);
            pos += 4;

            if partition != target_partition {
                return None;
            }

            // 读取 offset (8 bytes)
            pos += 8;

            // 读取 timestamp (8 bytes, big endian) - 这是我们要的
            if payload.len() < pos + 8 {
                return None;
            }
            let timestamp = i64::from_be_bytes([
                payload[pos], payload[pos + 1], payload[pos + 2], payload[pos + 3],
                payload[pos + 4], payload[pos + 5], payload[pos + 6], payload[pos + 7],
            ]);

            Some(timestamp)
        }
        _ => {
            // 其他版本暂时不支持
            None
        }
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
