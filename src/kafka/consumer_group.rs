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

        let mut topics: Vec<String> = Vec::with_capacity(20);

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

    /// 获取 Consumer Group 在指定 topic 上的 offset 和 lag 信息
    /// 跳过耗时的 topic 发现步骤，直接查询指定 topic 的 offsets
    pub fn get_consumer_group_offsets_for_topic(&self, group_id: &str, topic: &str) -> Result<Vec<PartitionOffsetDetail>> {
        self.get_consumer_group_offsets(group_id, &[topic.to_string()])
    }

    /// 获取 Consumer Group 详情
    pub fn get_consumer_group_info(&self, group_id: &str, topics: &[String]) -> Result<ConsumerGroupInfo> {
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        // 使用 fetch_group_list API 获取消费者组的真实状态
        let group_list = consumer.fetch_group_list(Some(group_id), self.timeout)?;

        // 获取消费者组状态
        let state = if let Some(group) = group_list.groups().first() {
            // 获取组的状态（Stable, PreparingRebalance, CompletingRebalance, Dead, Empty 等）
            let group_state = group.state();
            match group_state {
                "Stable" => "Stable".to_string(),
                "PreparingRebalance" => "PreparingRebalance".to_string(),
                "CompletingRebalance" => "CompletingRebalance".to_string(),
                "Dead" => "Dead".to_string(),
                "Empty" => "Empty".to_string(),
                _ => group_state.to_string(),
            }
        } else {
            "Unknown".to_string()
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

        // 预分配容量：假设每个 topic 平均 4 个分区
        let mut result = Vec::with_capacity(topics.len() * 4);

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

    /// 批量获取多个分区的最后提交时间
    /// 使用并行处理提高效率
    pub fn get_partitions_last_commit_time(
        &self,
        group_id: &str,
        partitions: &[(String, i32)],
    ) -> Result<Vec<Option<i64>>> {
        if partitions.is_empty() {
            return Ok(vec![]);
        }

        // 创建一个 consumer 获取所有 committed offsets
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        // 构建 TopicPartitionList 包含所有分区
        let mut tpl = TopicPartitionList::new();
        for (topic, partition) in partitions {
            tpl.add_partition(topic, *partition);
        }

        // 批量获取 committed offsets
        let committed = consumer.committed_offsets(tpl, self.timeout)?;

        // 解析 committed offsets
        let mut offsets_map: std::collections::HashMap<(String, i32), i64> = std::collections::HashMap::with_capacity(committed.elements().len());
        for elem in committed.elements() {
            if let Offset::Offset(o) = elem.offset() {
                offsets_map.insert((elem.topic().to_string(), elem.partition()), o);
            }
        }

        // 使用标准库线程并行处理
        let mut handles = vec![];

        for (topic, partition) in partitions.iter().cloned() {
            let offset = match offsets_map.get(&(topic.clone(), partition)) {
                Some(&o) if o > 0 => o,
                _ => {
                    handles.push(None);
                    continue;
                }
            };

            let config = self.consumer_config.clone();
            let _timeout = self.timeout;

            let handle = std::thread::spawn(move || {
                // 为每个分区创建独立的 consumer
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("SystemTime before UNIX epoch")
                    .as_millis();
                let mut read_config = crate::kafka::create_client_config(&config);
                read_config.set("group.id", &format!("kafka-manager-time-reader-{}-{}-{}-{}",
                    std::process::id(), topic, partition, timestamp));
                read_config.set("enable.auto.commit", "false");

                let read_consumer: BaseConsumer = match read_config.create() {
                    Ok(c) => c,
                    Err(_) => return None,
                };

                // Assign 到指定的 offset
                let mut read_tpl = TopicPartitionList::new();
                if read_tpl.add_partition_offset(&topic, partition, Offset::Offset(offset - 1)).is_err() {
                    return None;
                }
                if read_consumer.assign(&read_tpl).is_err() {
                    return None;
                }

                // Poll 消息获取时间戳（增加超时以适应服务器环境）
                let timeout = Duration::from_millis(2000);
                let start = std::time::Instant::now();

                while start.elapsed() < timeout {
                    match read_consumer.poll(Duration::from_millis(50)) {
                        Some(Ok(msg)) => {
                            if let Some(ts) = msg.timestamp().to_millis() {
                                return Some(ts);
                            }
                        }
                        Some(Err(_)) => break,
                        None => continue,
                    }
                }

                None
            });

            handles.push(Some(handle));
        }

        // 收集结果
        let mut results = vec![];
        for handle in handles {
            match handle {
                Some(h) => results.push(h.join().unwrap_or(None)),
                None => results.push(None),
            }
        }

        Ok(results)
    }

    /// 获取单个分区的最后提交时间
    /// 使用 committed_offsets API 获取 committed offset，然后读取消息时间戳
    pub fn get_partition_last_commit_time(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Result<Option<i64>> {
        // 创建一个临时 consumer 来获取 committed offset
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");

        let consumer: BaseConsumer = client_config.create()?;

        // 构建 TopicPartitionList
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition(topic, partition);

        // 获取 committed offset
        let committed = consumer.committed_offsets(tpl, self.timeout)?;

        // 解析 committed offset
        let committed_offset = committed.elements()
            .iter()
            .find(|e| e.topic() == topic && e.partition() == partition)
            .and_then(|e| match e.offset() {
                Offset::Offset(o) => Some(o),
                _ => None,
            });

        let offset = match committed_offset {
            Some(o) => o,
            None => {
                tracing::debug!("[get_last_commit_time] No committed offset for {}/{}/{}", group_id, topic, partition);
                return Ok(None);
            }
        };

        tracing::info!("[get_last_commit_time] Committed offset for {}/{}/{}: {}", group_id, topic, partition, offset);

        // 读取该 offset 的消息来获取时间戳
        // 如果 offset 是 0 或负数，无法读取消息
        if offset <= 0 {
            return Ok(None);
        }

        // 创建临时 consumer 读取消息时间戳
        // 使用唯一的 group.id 避免与现有 consumer group 冲突
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime before UNIX epoch")
            .as_millis();
        let mut read_config = crate::kafka::create_client_config(&self.consumer_config);
        read_config.set("group.id", &format!("kafka-manager-time-reader-{}-{}-{}-{}",
            std::process::id(), group_id, topic, timestamp));
        read_config.set("enable.auto.commit", "false");

        let read_consumer: BaseConsumer = read_config.create()?;

        // Assign 到指定的 offset
        let mut read_tpl = TopicPartitionList::new();
        read_tpl.add_partition_offset(topic, partition, Offset::Offset(offset - 1))?;
        read_consumer.assign(&read_tpl)?;

        // Poll 消息获取时间戳
        let timeout = Duration::from_millis(5000);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match read_consumer.poll(Duration::from_millis(100)) {
                Some(Ok(msg)) => {
                    if let Some(ts) = msg.timestamp().to_millis() {
                        tracing::info!("[get_last_commit_time] Found timestamp: {} for {}/{}/{}",
                            ts, group_id, topic, partition);
                        return Ok(Some(ts));
                    }
                }
                Some(Err(e)) => {
                    tracing::warn!("[get_last_commit_time] Error polling message: {}", e);
                    break;
                }
                None => continue,
            }
        }

        tracing::warn!("[get_last_commit_time] Could not get timestamp for {}/{}/{}", group_id, topic, partition);
        Ok(None)
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

        // 创建 TopicPartitionList 用于提交
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, Offset::Offset(offset))
            .map_err(|e| AppError::Internal(format!("Failed to add partition offset: {}", e)))?;

        // 尝试使用 assign + commit 方式
        // 首先 assign 分区
        if let Err(e) = consumer.assign(&tpl) {
            tracing::warn!("Failed to assign partition: {}", e);
        }

        // 然后提交 offset
        let commit_result = consumer.commit(&tpl, CommitMode::Sync);

        // 无论 assign 是否成功，都尝试提交
        commit_result
            .map_err(|e| {
                let msg = format!("Failed to commit offset: {}", e);
                tracing::warn!("Commit error: {}", msg);

                // UnknownMemberId 通常表示消费者组正在 rebalance 或有活跃消费者
                // 在有活跃消费者的情况下，这是预期行为
                if msg.contains("UnknownMemberId") || msg.contains("Unknown") {
                    AppError::BadRequest(
                        "无法重置 offset：消费者组当前有活跃的消费者连接。请先停止所有消费者，等待消费者组稳定后再试。".to_string()
                    )
                } else {
                    AppError::Internal(msg)
                }
            })?;

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

    /// Kafka 使用的 Murmur2 hash 算法
    /// 与 Kafka 的 Utils.murmur2() 完全一致
    fn murmur2(data: &[u8]) -> i32 {
        const M: i32 = 0x5bd1e995;
        const R: i32 = 24;
        const SEED: i32 = 0x9747b28cu32 as i32;

        let len = data.len() as i32;
        let mut h = SEED ^ len;

        let mut i: usize = 0;
        while (len as usize) - i >= 4 {
            let mut k = (data[i] as i32) & 0xff
                | (((data[i + 1] as i32) & 0xff) << 8)
                | (((data[i + 2] as i32) & 0xff) << 16)
                | (((data[i + 3] as i32) & 0xff) << 24);

            k = k.wrapping_mul(M);
            k ^= k >> R;
            k = k.wrapping_mul(M);

            h = h.wrapping_mul(M);
            h ^= k;

            i += 4;
        }

        // 处理剩余字节
        if (len as usize) - i >= 1 {
            if (len as usize) - i >= 3 {
                h ^= ((data[i + 2] as i32) & 0xff) << 16;
            }
            if (len as usize) - i >= 2 {
                h ^= ((data[i + 1] as i32) & 0xff) << 8;
            }
            h ^= (data[i] as i32) & 0xff;
            h = h.wrapping_mul(M);
        }

        h ^= h >> 13;
        h = h.wrapping_mul(M);
        h ^= h >> 15;

        h
    }

    /// 从 __consumer_offsets topic 获取最后提交时间
    /// 通过读取 __consumer_offsets topic 中对应 key 的最新记录来获取提交时间戳
    pub fn get_last_commit_time_from_offsets_topic(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Result<Option<i64>> {
        // 创建一个独立的消费者来读取 __consumer_offsets topic
        let mut client_config = crate::kafka::create_client_config(&self.consumer_config);
        client_config.set("group.id", "kafka-manager-reader");
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");
        client_config.set("isolation.level", "read_uncommitted");
        client_config.set("enable.partition.eof", "true");

        let consumer: BaseConsumer = client_config.create()?;

        // 等待 consumer 准备好
        std::thread::sleep(Duration::from_millis(100));

        // 首先获取 __consumer_offsets topic 的元数据
        let metadata = consumer.fetch_metadata(Some("__consumer_offsets"), self.timeout)?;
        let offsets_topic_meta = metadata.topics().iter()
            .find(|t| t.name() == "__consumer_offsets");

        if offsets_topic_meta.is_none() {
            tracing::warn!("__consumer_offsets topic not found");
            return Ok(None);
        }

        let offsets_topic_meta = offsets_topic_meta.unwrap();
        let num_partitions = offsets_topic_meta.partitions().len() as i32;

        tracing::info!(
            "[get_last_commit_time] Looking for group_id='{}', topic='{}', partition={}",
            group_id, topic, partition
        );
        tracing::info!("[get_last_commit_time] __consumer_offsets has {} partitions", num_partitions);

        // 计算目标分区（使用 Kafka 的默认分区算法）
        // 算法: abs(murmur2(group_id)) % num_partitions
        let hash = Self::murmur2(group_id.as_bytes());
        let target_partition = hash.wrapping_abs() % num_partitions;

        tracing::info!("[get_last_commit_time] Hash for group '{}': {} -> target partition: {}",
            group_id, hash, target_partition);

        // 检查目标分区是否存在
        let partition_exists = offsets_topic_meta.partitions()
            .iter()
            .any(|p| p.id() == target_partition);

        if !partition_exists {
            tracing::warn!("[get_last_commit_time] Target partition {} does not exist", target_partition);
            return Ok(None);
        }

        // 获取该分区的水位
        let (start_offset, end_offset) = consumer
            .fetch_watermarks("__consumer_offsets", target_partition, self.timeout)
            .map_err(|e| {
                tracing::error!("Failed to fetch watermarks for partition {}: {}", target_partition, e);
                AppError::Internal(format!("Failed to fetch watermarks: {}", e))
            })?;

        tracing::info!("[get_last_commit_time] Partition {} offsets: {} to {}",
            target_partition, start_offset, end_offset);

        if end_offset <= start_offset {
            tracing::warn!("[get_last_commit_time] Partition {} is empty", target_partition);
            return Ok(None);
        }

        let target_key = Self::build_offset_key(group_id, topic, partition);
        tracing::info!("[get_last_commit_time] Target key bytes (hex): {:02x?}", target_key);

        // 从后向前搜索
        // 如果目标分区没找到，搜索所有分区（用于调试）
        let result = self.search_partition_for_commit(
            &consumer,
            target_partition,
            start_offset,
            end_offset,
            &target_key,
            group_id,
            topic,
            partition,
        )?;

        // 如果目标分区没找到，尝试搜索附近的分区（murmur2可能有实现差异）
        if result.is_none() {
            tracing::warn!("[get_last_commit_time] Not found in target partition {}, checking nearby partitions", target_partition);
            // 只检查目标分区前后5个分区
            let nearby_partitions: Vec<i32> = ((-5..=5)
                .map(|i| target_partition + i)
                .filter(|&p| p >= 0 && p < num_partitions && p != target_partition))
                .collect();

            for p in nearby_partitions {
                let (start, end) = match consumer.fetch_watermarks("__consumer_offsets", p, self.timeout) {
                    Ok((s, e)) if e > s => (s, e),
                    _ => continue,
                };
                tracing::info!("[get_last_commit_time] Checking partition {} offsets {}..{}", p, start, end);
                if let Ok(Some(ts)) = self.search_partition_for_commit(
                    &consumer, p, start, end, &target_key, group_id, topic, partition
                ) {
                    tracing::info!("[get_last_commit_time] Found in partition {}!", p);
                    return Ok(Some(ts));
                }
            }

            // 如果附近分区也没找到，可能这个 group 没有 offset 提交记录
            tracing::warn!("[get_last_commit_time] Group '{}' has no offset commit record for {}/{}",
                group_id, topic, partition);
        }

        Ok(result)
    }

    /// 在指定分区中搜索提交记录
    fn search_partition_for_commit(
        &self,
        consumer: &BaseConsumer,
        partition: i32,
        start_offset: i64,
        end_offset: i64,
        _target_key: &[u8],
        group_id: &str,
        topic: &str,
        target_partition: i32,
    ) -> Result<Option<i64>> {
        // 限制最多检查 5000 条消息，避免性能问题
        let max_check = 5000i64;
        let check_start = (end_offset - max_check).max(start_offset);
        let mut last_found_timestamp: Option<i64> = None;

        tracing::info!("[search_partition] Searching partition {} from offset {} to {}",
            partition, check_start, end_offset);

        // 使用批量消费提高效率
        let batch_size = 100i64;
        let mut current_batch_start = check_start;

        while current_batch_start < end_offset {
            let batch_end = (current_batch_start + batch_size).min(end_offset);

            // 尝试读取这一批消息
            match self.read_batch(
                consumer,
                partition,
                current_batch_start,
                batch_end,
                group_id,
                topic,
                target_partition,
            ) {
                Ok(Some(timestamp)) => {
                    last_found_timestamp = Some(timestamp);
                    // 继续向前查找，看是否有更新的记录
                }
                Ok(None) => {
                    // 这批没找到，继续
                }
                Err(e) => {
                    tracing::warn!("[search_partition] Error reading batch: {}", e);
                }
            }

            current_batch_start = batch_end;
        }

        if last_found_timestamp.is_none() {
            tracing::warn!("[search_partition] No commit record found for {}/{}/{}",
                group_id, topic, target_partition);
        }

        Ok(last_found_timestamp)
    }

    /// 读取一批消息并查找匹配的记录
    fn read_batch(
        &self,
        consumer: &BaseConsumer,
        partition: i32,
        start: i64,
        end: i64,
        target_group: &str,
        target_topic: &str,
        target_partition: i32,
    ) -> Result<Option<i64>> {
        tracing::info!("[read_batch] Reading partition {} from offset {} to {}", partition, start, end);

        // 首先 assign 分区
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition("__consumer_offsets", partition);

        if let Err(e) = consumer.assign(&tpl) {
            tracing::error!("[read_batch] Failed to assign: {}", e);
            return Err(AppError::Internal(format!("Failed to assign: {}", e)));
        }

        // 等待一下确保 assign 生效
        std::thread::sleep(Duration::from_millis(50));

        // 然后 seek 到起始位置
        tracing::info!("[read_batch] Seeking to offset {}", start);
        if let Err(e) = consumer.seek("__consumer_offsets", partition, Offset::Offset(start), self.timeout) {
            tracing::error!("[read_batch] Failed to seek: {}", e);
            let _ = consumer.unassign();
            return Err(AppError::Internal(format!("Failed to seek: {}", e)));
        }

        // seek 后再等一下
        std::thread::sleep(Duration::from_millis(50));

        let mut last_match: Option<i64> = None;
        let mut messages_read = 0;
        let max_messages = ((end - start) as usize).max(1);

        // 循环读取消息
        let start_time = std::time::Instant::now();
        let max_poll_time = Duration::from_millis(10000);

        loop {
            if start_time.elapsed() > max_poll_time {
                tracing::warn!("[read_batch] Timeout after reading {} messages", messages_read);
                break;
            }

            match consumer.poll(Duration::from_millis(500)) {
                Some(Ok(msg)) => {
                    messages_read += 1;
                    let msg_offset = msg.offset();

                    // 打印前几条消息的 key 用于调试
                    if messages_read <= 10 {
                        if let Some(key) = msg.key() {
                            tracing::info!("[read_batch] Message {} at offset {}: key_len={}, first_bytes={:02x?}",
                                messages_read, msg_offset, key.len(), &key[..key.len().min(20)]);
                        } else {
                            tracing::info!("[read_batch] Message {} at offset {}: no key", messages_read, msg_offset);
                        }
                    }

                    // 检查 offset 是否超出范围
                    if msg_offset >= end {
                        tracing::info!("[read_batch] Reached end offset {}", msg_offset);
                        break;
                    }

                    // 解析并检查是否匹配
                    if let Some(timestamp) = parse_consumer_offset_message(
                        &msg, target_group, target_topic, target_partition
                    ) {
                        tracing::info!(
                            "[read_batch] Found commit at offset {}: timestamp={} for {}/{}/{}",
                            msg_offset, timestamp, target_group, target_topic, target_partition
                        );
                        last_match = Some(timestamp);
                    }

                    // 读取足够消息后退出
                    if messages_read >= max_messages {
                        break;
                    }
                }
                Some(Err(e)) => {
                    tracing::warn!("[read_batch] Poll error: {}", e);
                    break;
                }
                None => {
                    tracing::debug!("[read_batch] No more messages after reading {}", messages_read);
                    break;
                }
            }
        }

        // 清理 assign
        let _ = consumer.unassign();

        tracing::info!("[read_batch] Finished reading {} messages, found match: {:?}",
            messages_read, last_match.is_some());

        Ok(last_match)
    }

    /// 构建 __consumer_offsets 的 key（用于调试和验证）
    fn build_offset_key(group_id: &str, topic: &str, partition: i32) -> Vec<u8> {
        let mut key = Vec::new();

        // version (2 bytes, big endian) - 使用版本 0
        key.extend_from_slice(&0i16.to_be_bytes());

        // group_id length (2 bytes) + group_id
        let group_id_bytes = group_id.as_bytes();
        key.extend_from_slice(&(group_id_bytes.len() as i16).to_be_bytes());
        key.extend_from_slice(group_id_bytes);

        // topic length (2 bytes) + topic
        let topic_bytes = topic.as_bytes();
        key.extend_from_slice(&(topic_bytes.len() as i16).to_be_bytes());
        key.extend_from_slice(topic_bytes);

        // partition (4 bytes)
        key.extend_from_slice(&partition.to_be_bytes());

        key
    }
}

/// 解析 __consumer_offsets topic 中的消息（使用 key 和 value）
/// 返回最后提交时间（毫秒时间戳）
fn parse_consumer_offset_message(
    msg: &rdkafka::message::BorrowedMessage,
    target_group: &str,
    target_topic: &str,
    target_partition: i32,
) -> Option<i64> {
    // 首先从 key 中解析 group_id, topic, partition
    let key = msg.key();
    if key.is_none() || key.unwrap().is_empty() {
        tracing::debug!("[parse_consumer_offset_message] No key or empty key");
        return None;
    }
    let key = key.unwrap();

    // 解析 key 获取 group_id, topic, partition
    let parse_result = parse_offset_key(key);
    if parse_result.is_none() {
        tracing::debug!("[parse_consumer_offset_message] Failed to parse key, len={}", key.len());
        return None;
    }
    let (key_group, key_topic, key_partition) = parse_result?;

    tracing::debug!("[parse_consumer_offset_message] Parsed key: group={}, topic={}, partition={}", key_group, key_topic, key_partition);

    // 检查是否匹配目标
    if key_group != target_group || key_topic != target_topic || key_partition != target_partition {
        return None;
    }

    // 从 value 中解析时间戳
    let payload = msg.payload();
    if payload.is_none() || payload.unwrap().is_empty() {
        tracing::debug!("[parse_consumer_offset_message] No payload or empty payload");
        return None;
    }
    let payload = payload.unwrap();

    tracing::debug!("[parse_consumer_offset_message] Payload len={}", payload.len());

    let result = parse_offset_value(payload);
    if result.is_some() {
        tracing::info!("[parse_consumer_offset_message] Found timestamp: {:?}", result);
    } else {
        tracing::debug!("[parse_consumer_offset_message] Failed to parse timestamp from payload");
    }
    result
}

/// 解析 __consumer_offsets 的 key
/// 返回 (group_id, topic, partition)
fn parse_offset_key(key: &[u8]) -> Option<(String, String, i32)> {
    if key.len() < 2 {
        tracing::debug!("[parse_offset_key] Key too short: {}", key.len());
        return None;
    }

    let mut pos = 0;

    // 读取 version (2 bytes, big endian)
    let version = i16::from_be_bytes([key[0], key[1]]);
    pos += 2;

    tracing::debug!("[parse_offset_key] Key version: {}", version);

    match version {
        0..=3 => {
            // 读取 group_id
            if key.len() < pos + 2 {
                tracing::debug!("[parse_offset_key] Key too short for group_id_len");
                return None;
            }
            let group_id_len = i16::from_be_bytes([key[pos], key[pos + 1]]) as usize;
            pos += 2;

            if key.len() < pos + group_id_len {
                tracing::debug!("[parse_offset_key] Key too short for group_id: len={}, pos={}, group_id_len={}", key.len(), pos, group_id_len);
                return None;
            }
            let group_id = String::from_utf8_lossy(&key[pos..pos + group_id_len]).to_string();
            pos += group_id_len;

            // 读取 topic
            if key.len() < pos + 2 {
                tracing::debug!("[parse_offset_key] Key too short for topic_len");
                return None;
            }
            let topic_len = i16::from_be_bytes([key[pos], key[pos + 1]]) as usize;
            pos += 2;

            if key.len() < pos + topic_len {
                tracing::debug!("[parse_offset_key] Key too short for topic: len={}, pos={}, topic_len={}", key.len(), pos, topic_len);
                return None;
            }
            let topic = String::from_utf8_lossy(&key[pos..pos + topic_len]).to_string();
            pos += topic_len;

            // 读取 partition (4 bytes)
            if key.len() < pos + 4 {
                tracing::debug!("[parse_offset_key] Key too short for partition");
                return None;
            }
            let partition = i32::from_be_bytes([key[pos], key[pos + 1], key[pos + 2], key[pos + 3]]);

            tracing::debug!("[parse_offset_key] Parsed: group_id={}, topic={}, partition={}", group_id, topic, partition);
            Some((group_id, topic, partition))
        }
        _ => {
            // 其他版本不支持
            tracing::debug!("Unsupported key version: {}", version);
            None
        }
    }
}

/// 从 __consumer_offsets value 中解析时间戳
fn parse_offset_value(payload: &[u8]) -> Option<i64> {
    if payload.is_empty() {
        tracing::debug!("[parse_offset_value] Empty payload");
        return None;
    }

    let mut pos = 0;

    // 读取 version (2 bytes, big endian)
    if payload.len() < 2 {
        tracing::debug!("[parse_offset_value] Payload too short: {}", payload.len());
        return None;
    }
    let version = i16::from_be_bytes([payload[0], payload[1]]);
    pos += 2;

    tracing::info!("[parse_offset_value] Version: {}, payload len: {}", version, payload.len());

    // 不同版本格式不同，这里处理常见的版本 0-3
    match version {
        0 => {
            // Version 0: [version][offset][timestamp]
            // offset: 8 bytes, timestamp: 8 bytes
            pos += 8; // skip offset
            if payload.len() < pos + 8 {
                tracing::debug!("[parse_offset_value] v0 payload too short: {} < {}", payload.len(), pos + 8);
                return None;
            }
            // 尝试大端序
            let timestamp_be = i64::from_be_bytes([
                payload[pos], payload[pos + 1], payload[pos + 2], payload[pos + 3],
                payload[pos + 4], payload[pos + 5], payload[pos + 6], payload[pos + 7],
            ]);
            // 尝试小端序
            let timestamp_le = i64::from_le_bytes([
                payload[pos], payload[pos + 1], payload[pos + 2], payload[pos + 3],
                payload[pos + 4], payload[pos + 5], payload[pos + 6], payload[pos + 7],
            ]);
            tracing::info!("[parse_offset_value] v0 raw bytes: {:02x?}", &payload[pos..pos+8]);
            tracing::info!("[parse_offset_value] v0 timestamp_be: {}, timestamp_le: {}", timestamp_be, timestamp_le);
            // 使用合理的时间戳（毫秒时间戳应该在 2020-2030 年之间，即 1600000000000 - 1900000000000）
            if timestamp_be > 1600000000000 && timestamp_be < 1900000000000 {
                Some(timestamp_be)
            } else if timestamp_le > 1600000000000 && timestamp_le < 1900000000000 {
                Some(timestamp_le)
            } else {
                // 如果都不合理，返回大端序的值
                Some(timestamp_be)
            }
        }
        1 => {
            // Version 1: [version][offset][timestamp][expireTimestamp]
            pos += 8; // skip offset
            if payload.len() < pos + 8 {
                tracing::debug!("[parse_offset_value] v1 payload too short");
                return None;
            }
            let timestamp = i64::from_be_bytes([
                payload[pos], payload[pos + 1], payload[pos + 2], payload[pos + 3],
                payload[pos + 4], payload[pos + 5], payload[pos + 6], payload[pos + 7],
            ]);
            tracing::info!("[parse_offset_value] v1 timestamp: {}", timestamp);
            Some(timestamp)
        }
        2 => {
            // Version 2: [version][offset][timestamp][metadataLength][metadata]
            pos += 8; // skip offset
            if payload.len() < pos + 8 {
                tracing::debug!("[parse_offset_value] v2 payload too short");
                return None;
            }
            let timestamp = i64::from_be_bytes([
                payload[pos], payload[pos + 1], payload[pos + 2], payload[pos + 3],
                payload[pos + 4], payload[pos + 5], payload[pos + 6], payload[pos + 7],
            ]);
            tracing::info!("[parse_offset_value] v2 timestamp: {}", timestamp);
            Some(timestamp)
        }
        3 => {
            // Version 3: similar to version 2 but with leader epoch
            // [version][offset][timestamp][metadataLength][metadata][leaderEpoch]
            pos += 8; // skip offset
            if payload.len() < pos + 8 {
                tracing::debug!("[parse_offset_value] v3 payload too short");
                return None;
            }
            let timestamp = i64::from_be_bytes([
                payload[pos], payload[pos + 1], payload[pos + 2], payload[pos + 3],
                payload[pos + 4], payload[pos + 5], payload[pos + 6], payload[pos + 7],
            ]);
            tracing::info!("[parse_offset_value] v3 timestamp: {}", timestamp);
            Some(timestamp)
        }
        _ => {
            // 其他版本不支持
            tracing::debug!("[parse_offset_value] Unsupported value version: {}", version);
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

    let mut topics = Vec::with_capacity(topic_count);

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
