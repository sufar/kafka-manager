/// 消息查询模块 - 使用 Consumer Pool 进行消息查询
use crate::error::{AppError, Result};
use crate::kafka::consumer::KafkaMessage;
use crate::pool::KafkaConsumerPool;
use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
use rdkafka::Message;
use rdkafka::TopicPartitionList;
use rdkafka::ClientConfig;
use std::collections::HashMap;
use std::time::Duration;

/// 使用 Pool 获取 Consumer 进行消息查询
pub async fn fetch_messages_with_pool(
    pool: &KafkaConsumerPool,
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<KafkaMessage>> {
    let start_time_total = std::time::Instant::now();
    let topic = topic.to_string();
    let _brokers = brokers.to_string();
    let search = search.clone();
    let fetch_mode = fetch_mode.map(|s| s.to_string());
    let sort = sort.map(|s| s.to_string());

    tracing::info!(
        "[PoolQuery] Topic: {}, partition: {:?}, max_messages: {}, brokers: {}",
        topic, partition, max_messages, _brokers
    );

    // 从 pool 获取 consumer
    let consumer: deadpool::managed::Object<crate::pool::kafka_consumer::KafkaConsumerManager> =
        pool.get().await.map_err(|e| {
            AppError::Internal(format!("Failed to get consumer from pool: {}", e))
        })?;

    tracing::debug!("[PoolQuery] Got consumer from pool");

    // 获取分区列表
    let partitions: Vec<i32> = {
        if partition.is_none() {
            // 使用阻塞方式获取元数据
            let consumer_ref = &*consumer;
            match tokio::task::block_in_place(|| {
                consumer_ref
                    .client()
                    .fetch_metadata(Some(&topic), Duration::from_secs(5))
            }) {
                Ok(metadata) => metadata
                    .topics()
                    .first()
                    .map(|t| t.partitions().iter().map(|p| p.id()).collect())
                    .unwrap_or_else(|| vec![0]),
                Err(_) => vec![0],
            }
        } else {
            vec![partition.unwrap_or(0)]
        }
    };

    let partition_count = partitions.len();
    let _total_target = max_messages * partition_count;

    tracing::info!(
        "[Pool] {} partitions, {} messages per partition",
        partition_count,
        max_messages
    );

    // 使用 spawn_blocking 在同步上下文中执行消费逻辑
    let result = tokio::task::spawn_blocking(move || {
        // 优化：预获取所有分区的 watermark
        let mut partition_watermarks: HashMap<i32, (i64, i64)> = HashMap::new();
        let mut partition_reached_end: HashMap<i32, bool> =
            HashMap::with_capacity(partition_count);

        for &part_id in &partitions {
            if let Ok((low, high)) = consumer
                .client()
                .fetch_watermarks(&topic, part_id, Duration::from_millis(1000))
            {
                partition_watermarks.insert(part_id, (low, high));
                if high <= low || high == 0 {
                    partition_reached_end.insert(part_id, true);
                    tracing::info!(
                        "[Pool] Partition {} is empty (low={}, high={}), marked as finished",
                        part_id, low, high
                    );
                }
            }
        }

        // 查询 end_time 对应的 offset
        let mut time_range_end_offsets: HashMap<i32, i64> = HashMap::new();
        if let Some(end_t) = end_time {
            if end_t > 0 {
                let mut time_tpl = TopicPartitionList::new();
                for &part_id in &partitions {
                    time_tpl
                        .add_partition_offset(&topic, part_id, rdkafka::Offset::Offset(end_t))
                        .ok();
                }
                match consumer.offsets_for_times(time_tpl, Duration::from_secs(3)) {
                    Ok(r) => {
                        for elem in r.elements_for_topic(&topic) {
                            if let Some(off) = elem.offset().to_raw() {
                                time_range_end_offsets.insert(elem.partition(), off.saturating_sub(1));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[Pool] Failed to query end_time offsets: {}", e);
                    }
                }
            }
        }

        // 构建 TopicPartitionList 并 assign
        let mut tpl = TopicPartitionList::new();
        for &part_id in &partitions {
            let start_offset = calculate_partition_offset_for_stream(
                &*consumer,
                &topic,
                part_id,
                max_messages,
                offset,
                start_time,
                fetch_mode.as_deref(),
            )
            .unwrap_or(0);
            let assign_offset = if start_offset < 0 {
                rdkafka::Offset::Beginning
            } else {
                rdkafka::Offset::Offset(start_offset)
            };
            tpl.add_partition_offset(&topic, part_id, assign_offset).ok();
        }

        if let Err(e) = consumer.assign(&tpl) {
            tracing::error!("[Pool] Failed to assign partitions: {}", e);
            return Vec::new();
        }

        let search_lower = search.as_ref().map(|s| s.to_lowercase());
        let is_desc = sort.as_deref() == Some("desc")
            || (sort.is_none() && fetch_mode.as_deref() == Some("newest"));

        // 收集消息
        let mut partition_counts: HashMap<i32, usize> = HashMap::with_capacity(partition_count);
        let mut raw_msgs: Vec<KafkaMessage> =
            Vec::with_capacity(max_messages * partition_count);
        let mut total_empty = 0;
        let max_empty_rounds = 100 + max_messages / 100 * 10;
        let max_poll_time = std::time::Duration::from_secs(30);
        let poll_start = std::time::Instant::now();

        let partitions_ref = &partitions;
        let all_partitions_finished =
            |counts: &HashMap<i32, usize>, reached_end: &HashMap<i32, bool>| {
                partitions_ref.iter().all(|&p| {
                    let count = counts.get(&p).copied().unwrap_or(0);
                    let reached = reached_end.get(&p).copied().unwrap_or(false);
                    count >= max_messages || reached
                })
            };

        // StreamConsumer 需要使用异步运行时来 poll
        // 由于我们在 spawn_blocking 中，需要使用 block_on 来执行异步操作
        let rt = tokio::runtime::Handle::current();

        let mut first_poll = true;
        while raw_msgs.len() < max_messages * partition_count
            && total_empty < max_empty_rounds
            && poll_start.elapsed() < max_poll_time
            && !all_partitions_finished(&partition_counts, &partition_reached_end)
        {
            let poll_timeout = if first_poll {
                first_poll = false;
                Duration::from_millis(500)
            } else {
                Duration::from_millis(50)
            };

            // 使用 block_on 在 spawn_blocking 中执行异步 recv
            let msg_result = rt.block_on(async {
                tokio::time::timeout(poll_timeout, consumer.recv()).await
            });

            match msg_result {
                Ok(Ok(msg)) => {
                    let part = msg.partition();
                    let count = partition_counts.get(&part).copied().unwrap_or(0);
                    if count >= max_messages {
                        if let Some((_, high)) = partition_watermarks.get(&part) {
                            if msg.offset() >= high.saturating_sub(1) {
                                partition_reached_end.insert(part, true);
                            }
                        }
                        continue;
                    }
                    total_empty = 0;

                    let ts = msg.timestamp().to_millis();
                    if let Some(start) = start_time {
                        if let Some(t) = ts {
                            if t < start {
                                continue;
                            }
                        }
                    }
                    if let Some(end) = end_time {
                        if let Some(t) = ts {
                            if t > end {
                                continue;
                            }
                        }
                    }

                    let key = msg
                        .key()
                        .and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                    let value = msg
                        .payload()
                        .and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                    partition_counts.insert(part, count + 1);

                    if let Some((low, high)) = partition_watermarks.get(&part) {
                        let reached_beginning = msg.offset() <= *low;
                        let reached_end = msg.offset() >= high.saturating_sub(1);
                        let partition_full = count + 1 >= max_messages;
                        let reached_time_end = time_range_end_offsets
                            .get(&part)
                            .map(|end_off| msg.offset() >= *end_off)
                            .unwrap_or(false);

                        if reached_beginning || reached_end || partition_full || reached_time_end {
                            partition_reached_end.insert(part, true);
                        }
                    }

                    raw_msgs.push(KafkaMessage {
                        partition: part,
                        offset: msg.offset(),
                        key,
                        value,
                        timestamp: ts,
                    });
                }
                Ok(Err(e)) => {
                    tracing::warn!("[Pool] Poll error: {}", e);
                    total_empty += 1;
                }
                Err(_) => {
                    // 超时，没有消息
                    total_empty += 1;
                }
            }
        }

        tracing::info!(
            "[Pool] Collected {} raw messages in {:?}",
            raw_msgs.len(),
            poll_start.elapsed()
        );

        // 清理：unassign 分区，确保 consumer 可以安全归还 pool
        if let Err(e) = consumer.unassign() {
            tracing::warn!("[Pool] Failed to unassign consumer: {}", e);
        }

        // 第二步：搜索过滤
        let filtered = if let Some(term) = search_lower {
            raw_msgs
                .into_iter()
                .filter(|msg| {
                    let key_match = msg
                        .key
                        .as_ref()
                        .map(|k| k.to_lowercase().contains(&term))
                        .unwrap_or(false);
                    let value_match = msg
                        .value
                        .as_ref()
                        .map(|v| v.to_lowercase().contains(&term))
                        .unwrap_or(false);
                    key_match || value_match
                })
                .collect()
        } else {
            raw_msgs
        };

        // 排序
        let mut result = filtered;
        if is_desc {
            result.sort_by(|a, b| b.offset.cmp(&a.offset));
        } else {
            result.sort_by(|a, b| a.offset.cmp(&b.offset));
        }

        result
    })
    .await
    .map_err(|e| AppError::Internal(format!("Spawn blocking error: {:?}", e)))?;

    tracing::info!(
        "[PoolQuery] Fetched {} messages in {:?}",
        result.len(),
        start_time_total.elapsed()
    );
    Ok(result)
}

/// 为 StreamConsumer 计算分区 offset（适配版）
fn calculate_partition_offset_for_stream<C: Consumer>(
    consumer: &C,
    topic: &str,
    partition: i32,
    max_messages: usize,
    specific_offset: Option<i64>,
    start_time: Option<i64>,
    fetch_mode: Option<&str>,
) -> Result<i64> {
    // 如果指定了具体 offset，直接使用
    if let Some(off) = specific_offset {
        return Ok(off);
    }

    // 获取 watermark
    let (low, high): (i64, i64) = consumer
        .client()
        .fetch_watermarks(topic, partition, Duration::from_secs(5))
        .map_err(|e| AppError::Internal(format!("Failed to fetch watermarks: {}", e)))?;

    // 根据 fetch_mode 计算起始 offset
    let offset = match fetch_mode {
        Some("newest") => {
            // 从最新消息往回读
            let start: i64 = high.saturating_sub(max_messages as i64);
            start.max(low)
        }
        Some("oldest") | _ => {
            // 从最早消息开始读
            low
        }
    };

    // 如果时间戳查询，返回特殊标记
    if start_time.is_some() && specific_offset.is_none() {
        // 时间戳查询会在 assign 后通过 offsets_for_times 处理
        return Ok(low); // 先返回 low，实际 offset 会在后续查询
    }

    Ok(offset)
}

/// 使用临时 consumer 获取消息（支持过滤）- 当 Pool 不可用时作为 fallback 使用
pub async fn fetch_messages_with_temp_consumer(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<KafkaMessage>> {
    // 检测是否是本地Kafka
    let is_local = brokers.contains("localhost")
        || brokers.contains("127.0.0.1")
        || brokers.contains("::1")
        || brokers.contains("host.docker.internal");

    if is_local {
        fetch_messages_local(
            brokers, topic, partition, offset, max_messages,
            start_time, end_time, search, fetch_mode, sort,
        ).await
    } else {
        fetch_messages_remote(
            brokers, topic, partition, offset, max_messages,
            start_time, end_time, search, fetch_mode, sort,
        ).await
    }
}

/// 本地集群 - 分层优化策略
async fn fetch_messages_local(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<KafkaMessage>> {
    let start_time_total = std::time::Instant::now();
    let topic = topic.to_string();
    let brokers = brokers.to_string();
    let search = search.clone();
    let fetch_mode = fetch_mode.map(|s| s.to_string());
    let sort = sort.map(|s| s.to_string());
    let offset = offset;

    tracing::info!(
        "[fetch_messages_local] Topic: {}, partition: {:?}, max_messages: {}, fetch_mode: {:?}",
        topic, partition, max_messages, fetch_mode
    );

    // 获取分区列表
    let partitions: Vec<i32> = {
        let cfg = ClientConfig::new()
            .set("bootstrap.servers", &brokers)
            .set("broker.address.family", "v4")
            .create::<BaseConsumer>()?;
        if partition.is_none() {
            match cfg.fetch_metadata(Some(&topic), Duration::from_secs(5)) {
                Ok(metadata) => metadata
                    .topics()
                    .first()
                    .map(|t| t.partitions().iter().map(|p| p.id()).collect())
                    .unwrap_or_else(|| vec![0]),
                Err(_) => vec![0],
            }
        } else {
            vec![partition.unwrap_or(0)]
        }
    };

    let partition_count = partitions.len();
    let total_target = max_messages * partition_count;

    tracing::info!(
        "[Local] {} partitions, {} messages per partition, total {}. Mode: {}",
        partition_count,
        max_messages,
        total_target,
        if (max_messages >= 1000 || partition_count == 1) && partition_count > 5 {
            "parallel"
        } else {
            "sequential"
        }
    );

    // 本地 Kafka 单分区或少分区时使用并行模式（无 group.id，直接分配分区）
    // 优化：分区数 <= 5 时使用串行模式，避免创建多个 consumer 的连接开销
    let messages = if (max_messages >= 1000 || partition_count == 1) && partition_count > 5 {
        // === 大批量并行模式（>=1000条/分区且分区数>5）===
        let msgs_per_partition = max_messages;
        tracing::info!(
            "[Parallel Mode] {} partitions, {} messages per partition, total target {}",
            partition_count,
            msgs_per_partition,
            total_target
        );
        let partition_has_specific_offset = partition.is_some();
        let partition_offset_val = if partition_has_specific_offset {
            offset
        } else {
            None
        };

        use tokio::sync::Semaphore;
        use tokio::time::{timeout, Duration as TokioDuration};
        use std::sync::Arc;

        // 最多 10 个并发
        let semaphore = Arc::new(Semaphore::new(10));
        let mut handles = vec![];

        for &part_id in &partitions {
            let brokers = brokers.clone();
            let topic = topic.clone();
            let search = search.clone();
            let fetch_mode = fetch_mode.clone();
            let part_offset = if partition_has_specific_offset {
                partition_offset_val
            } else {
                None
            };
            let sem = semaphore.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("Semaphore acquire failed");
                // 时间戳查询可能需要更长时间，增加超时到 60 秒
                let fetch_timeout = if start_time.is_some() || end_time.is_some() {
                    TokioDuration::from_secs(60)
                } else {
                    TokioDuration::from_secs(30)
                };
                timeout(
                    fetch_timeout,
                    tokio::task::spawn_blocking(move || {
                        fetch_partition_messages_parallel(
                            brokers,
                            topic,
                            part_id,
                            msgs_per_partition,
                            part_offset,
                            start_time,
                            end_time,
                            search,
                            fetch_mode,
                        )
                    }),
                )
                .await
            });
            handles.push(handle);
        }

        let mut all_msgs: Vec<KafkaMessage> = Vec::with_capacity(total_target);
        for handle in handles {
            match handle.await {
                Ok(Ok(Ok(msgs))) => {
                    tracing::info!("[Parallel] Got {} messages from task", msgs.len());
                    all_msgs.extend(msgs);
                }
                Ok(Ok(Err(_))) => tracing::warn!("Partition fetch timeout"),
                Ok(Err(e)) => tracing::warn!("Task join error: {}", e),
                Err(e) => tracing::warn!("Spawn error: {}", e),
            }
        }
        all_msgs
    } else {
        // === 串行模式（<1000条/分区 或 分区数<=5）===
        let partition_filter = partition;
        tokio::task::spawn_blocking(move || {
            let mut cfg = ClientConfig::new();
            cfg.set("bootstrap.servers", &brokers);
            cfg.set("group.id", "kafka-mgr-seq");
            cfg.set("enable.auto.commit", "false");
            cfg.set("auto.offset.reset", "earliest");
            cfg.set("session.timeout.ms", "3000");
            cfg.set("fetch.min.bytes", "1");
            cfg.set("fetch.wait.max.ms", "10");
            cfg.set("fetch.max.bytes", "52428800");
            cfg.set("socket.nagle.disable", "true");
            cfg.set("broker.address.family", "v4");
            cfg.set("socket.connection.setup.timeout.ms", "3000");
            cfg.set("reconnect.backoff.ms", "50");
            cfg.set("reconnect.backoff.max.ms", "500");

            let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };

            let effective_offset = if partition_filter.is_some() { offset } else { None };

            // 预获取所有分区的 watermark
            let mut partition_watermarks: HashMap<i32, (i64, i64)> = HashMap::new();
            let mut partition_reached_end: HashMap<i32, bool> =
                HashMap::with_capacity(partition_count);
            for &part_id in &partitions {
                if let Ok((low, high)) = consumer.fetch_watermarks(&topic, part_id, Duration::from_millis(1000)) {
                    partition_watermarks.insert(part_id, (low, high));
                    if high <= low || high == 0 {
                        partition_reached_end.insert(part_id, true);
                        tracing::info!(
                            "[Local] Partition {} is empty (low={}, high={}), marked as finished",
                            part_id, low, high
                        );
                    }
                }
            }

            // 查询 end_time 对应的 offset
            let mut time_range_end_offsets: HashMap<i32, i64> = HashMap::new();
            if let Some(end_t) = end_time {
                if end_t > 0 {
                    let mut time_tpl = TopicPartitionList::new();
                    for &part_id in &partitions {
                        time_tpl.add_partition_offset(&topic, part_id, rdkafka::Offset::Offset(end_t)).ok();
                    }
                    match consumer.offsets_for_times(time_tpl, Duration::from_secs(3)) {
                        Ok(r) => {
                            for elem in r.elements_for_topic(&topic) {
                                if let Some(off) = elem.offset().to_raw() {
                                    time_range_end_offsets.insert(elem.partition(), off.saturating_sub(1));
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[Local] Failed to query end_time offsets: {}", e);
                        }
                    }
                }
            }

            // 构建 TopicPartitionList
            let mut tpl = TopicPartitionList::new();
            for &part_id in &partitions {
                let start_offset = calculate_partition_offset(
                    &consumer,
                    &topic,
                    part_id,
                    max_messages,
                    effective_offset,
                    start_time,
                    fetch_mode.as_deref(),
                )
                .unwrap_or(0);
                let assign_offset = if start_offset < 0 {
                    rdkafka::Offset::Beginning
                } else {
                    rdkafka::Offset::Offset(start_offset)
                };
                tpl.add_partition_offset(&topic, part_id, assign_offset).ok();
            }

            if consumer.assign(&tpl).is_err() {
                return Vec::new();
            }

            let search_lower = search.as_ref().map(|s| s.to_lowercase());
            let is_desc = sort.as_deref() == Some("desc")
                || (sort.is_none() && fetch_mode.as_deref() == Some("newest"));

            let mut partition_counts: HashMap<i32, usize> = HashMap::with_capacity(partition_count);
            let mut raw_msgs: Vec<KafkaMessage> = Vec::with_capacity(max_messages * partition_count);
            let mut total_empty = 0;
            let max_empty_rounds = 100 + max_messages / 100 * 10;
            let max_poll_time = std::time::Duration::from_secs(30);
            let poll_start = std::time::Instant::now();
            let partitions_ref = &partitions;
            let all_partitions_finished =
                |counts: &HashMap<i32, usize>, reached_end: &HashMap<i32, bool>| {
                    partitions_ref.iter().all(|&p| {
                        let count = counts.get(&p).copied().unwrap_or(0);
                        let reached = reached_end.get(&p).copied().unwrap_or(false);
                        count >= max_messages || reached
                    })
                };

            let mut first_poll = true;
            while raw_msgs.len() < max_messages * partition_count
                && total_empty < max_empty_rounds
                && poll_start.elapsed() < max_poll_time
                && !all_partitions_finished(&partition_counts, &partition_reached_end)
            {
                let poll_timeout = if first_poll {
                    first_poll = false;
                    Duration::from_millis(500)
                } else {
                    Duration::from_millis(50)
                };
                match consumer.poll(poll_timeout) {
                    Some(Ok(msg)) => {
                        let part = msg.partition();
                        let count = partition_counts.get(&part).copied().unwrap_or(0);
                        if count >= max_messages {
                            if let Some((_, high)) = partition_watermarks.get(&part) {
                                if msg.offset() >= high.saturating_sub(1) {
                                    partition_reached_end.insert(part, true);
                                }
                            }
                            continue;
                        }
                        total_empty = 0;

                        let ts = msg.timestamp().to_millis();
                        if let Some(start) = start_time {
                            if let Some(t) = ts {
                                if t < start {
                                    continue;
                                }
                            }
                        }
                        if let Some(end) = end_time {
                            if let Some(t) = ts {
                                if t > end {
                                    continue;
                                }
                            }
                        }

                        let key = msg
                            .key()
                            .and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                        let value = msg
                            .payload()
                            .and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                        partition_counts.insert(part, count + 1);

                        if let Some((low, high)) = partition_watermarks.get(&part) {
                            let reached_beginning = msg.offset() <= *low;
                            let reached_end = msg.offset() >= high.saturating_sub(1);
                            let partition_full = count + 1 >= max_messages;
                            let reached_time_end = time_range_end_offsets
                                .get(&part)
                                .map(|end_off| msg.offset() >= *end_off)
                                .unwrap_or(false);

                            if reached_beginning || reached_end || partition_full || reached_time_end {
                                partition_reached_end.insert(part, true);
                            }
                        }
                        raw_msgs.push(KafkaMessage {
                            partition: part,
                            offset: msg.offset(),
                            key,
                            value,
                            timestamp: ts,
                        });
                    }
                    _ => {
                        total_empty += 1;
                    }
                }
            }

            // 搜索过滤
            let mut all_msgs: Vec<KafkaMessage> = if let Some(ref term) = search_lower {
                raw_msgs
                    .into_iter()
                    .filter(|msg| {
                        let km = msg.key.as_ref().map_or(false, |k| k.to_lowercase().contains(term));
                        let vm = msg.value.as_ref().map_or(false, |v| v.to_lowercase().contains(term));
                        km || vm
                    })
                    .collect()
            } else {
                raw_msgs
            };

            all_msgs.sort_by(|a, b| {
                match (a.timestamp, b.timestamp) {
                    (Some(ta), Some(tb)) => {
                        if is_desc { tb.cmp(&ta) } else { ta.cmp(&tb) }
                    }
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => {
                        if is_desc { b.offset.cmp(&a.offset) } else { a.offset.cmp(&b.offset) }
                    }
                }
            });

            all_msgs
        })
        .await
        .map_err(|e| AppError::Internal(format!("Join error: {}", e)))?
    };

    tracing::info!(
        "[Local] Fetched {} messages from {} partitions in {:?}",
        messages.len(),
        partition_count,
        start_time_total.elapsed()
    );

    Ok(messages)
}

/// 远程集群优化版 - 单consumer多分区，复用连接
async fn fetch_messages_remote(
    brokers: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    max_messages: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<&str>,
    sort: Option<&str>,
) -> Result<Vec<KafkaMessage>> {
    let start_time_total = std::time::Instant::now();
    let topic_owned = topic.to_string();
    let brokers_owned = brokers.to_string();
    let search_lower = search.as_ref().map(|s| s.to_lowercase());
    let fetch_mode_owned = fetch_mode.map(|s| s.to_string());
    let sort_owned = sort.map(|s| s.to_string());
    let offset_owned = if partition.is_some() { offset } else { None };

    let messages: Vec<KafkaMessage> = tokio::task::spawn_blocking(move || -> Result<Vec<KafkaMessage>> {
        // 创建consumer
        let mut cfg = ClientConfig::new();
        cfg.set("bootstrap.servers", &brokers_owned);
        cfg.set("group.id", "kafka-mgr-remote");
        cfg.set("enable.auto.commit", "false");
        cfg.set("auto.offset.reset", "earliest");
        cfg.set("session.timeout.ms", "3000");
        cfg.set("heartbeat.interval.ms", "500");
        cfg.set("fetch.min.bytes", "1");
        cfg.set("fetch.wait.max.ms", "10");
        cfg.set("fetch.max.bytes", "10485760");
        cfg.set("max.partition.fetch.bytes", "10485760");
        cfg.set("socket.nagle.disable", "true");
        cfg.set("socket.connection.setup.timeout.ms", "5000");
        cfg.set("reconnect.backoff.ms", "50");
        cfg.set("partition.assignment.strategy", "");
        cfg.set("broker.address.family", "v4");

        let consumer: BaseConsumer<DefaultConsumerContext> = cfg
            .create()
            .map_err(|e| AppError::Internal(format!("Failed to create consumer: {}", e)))?;

        // 获取分区列表
        let partitions: Vec<i32> = if partition.is_none() {
            let metadata = consumer
                .fetch_metadata(Some(&topic_owned), Duration::from_millis(1000))
                .map_err(|e| AppError::Internal(format!("Failed to fetch metadata: {}", e)))?;
            metadata
                .topics()
                .first()
                .map(|t| t.partitions().iter().map(|p| p.id()).collect())
                .unwrap_or_else(|| vec![0])
        } else {
            vec![partition.unwrap_or(0)]
        };

        let partition_count = partitions.len();

        // 批量获取所有分区的 watermarks
        let mut watermark_cache: std::collections::HashMap<i32, (i64, i64)> =
            std::collections::HashMap::new();

        for &part_id in &partitions {
            match consumer.fetch_watermarks(&topic_owned, part_id, Duration::from_millis(5000)) {
                Ok(w) => {
                    watermark_cache.insert(part_id, w);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch watermarks for partition {}: {}", part_id, e);
                }
            }
        }

        // 查询 start_time 和 end_time 对应的 offset
        let mut time_based_offsets: std::collections::HashMap<i32, i64> =
            std::collections::HashMap::new();
        let mut time_range_end_offsets: std::collections::HashMap<i32, i64> =
            std::collections::HashMap::new();

        if let Some(start_time) = start_time {
            if start_time > 0 && !partitions.is_empty() {
                let time_query_start = std::time::Instant::now();
                let mut time_tpl = TopicPartitionList::new();
                for &part_id in &partitions {
                    time_tpl.add_partition_offset(&topic_owned, part_id, rdkafka::Offset::Offset(start_time)).ok();
                }
                match consumer.offsets_for_times(time_tpl, Duration::from_secs(5)) {
                    Ok(r) => {
                        let elapsed = time_query_start.elapsed();
                        for elem in r.elements_for_topic(&topic_owned) {
                            if let Some(offset) = elem.offset().to_raw() {
                                time_based_offsets.insert(elem.partition(), offset);
                            }
                        }
                        tracing::info!(
                            "[Remote] Batch queried start_time offsets for time {} on {} partitions in {:?}, found {} results",
                            start_time, partitions.len(), elapsed, time_based_offsets.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[Remote] Failed to batch query start_time offsets for time {}: {}",
                            start_time, e
                        );
                    }
                }
            }
        }

        if let Some(end_time) = end_time {
            if end_time > 0 && !partitions.is_empty() {
                let time_query_start = std::time::Instant::now();
                let mut time_tpl = TopicPartitionList::new();
                for &part_id in &partitions {
                    time_tpl.add_partition_offset(&topic_owned, part_id, rdkafka::Offset::Offset(end_time)).ok();
                }
                match consumer.offsets_for_times(time_tpl, Duration::from_secs(5)) {
                    Ok(r) => {
                        let elapsed = time_query_start.elapsed();
                        for elem in r.elements_for_topic(&topic_owned) {
                            if let Some(offset) = elem.offset().to_raw() {
                                time_range_end_offsets.insert(elem.partition(), offset.saturating_sub(1));
                            }
                        }
                        tracing::info!(
                            "[Remote] Batch queried end_time offsets for time {} on {} partitions in {:?}, found {} results",
                            end_time, partitions.len(), elapsed, time_range_end_offsets.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[Remote] Failed to batch query end_time offsets for time {}: {}",
                            end_time, e
                        );
                    }
                }
            }
        }

        // 构建 TopicPartitionList
        let mut tpl = TopicPartitionList::new();

        for &part_id in &partitions {
            let start_offset = time_based_offsets.get(&part_id).copied();
            if start_offset.is_some() {
                tracing::info!(
                    "[Remote] Using time-based offset: {} for partition {}",
                    start_offset.unwrap(),
                    part_id
                );
            }

            let start_offset = start_offset.unwrap_or_else(|| {
                if let Some(off) = offset_owned {
                    off
                } else if fetch_mode_owned.as_deref() == Some("newest")
                    || fetch_mode_owned.is_none()
                {
                    watermark_cache
                        .get(&part_id)
                        .map(|(low, high)| {
                            if *high > 0 {
                                let latest = high.saturating_sub(1);
                                latest.saturating_sub((max_messages.saturating_sub(1)) as i64)
                                    .max(*low)
                            } else {
                                *low
                            }
                        })
                        .unwrap_or(0)
                } else if fetch_mode_owned.as_deref() == Some("oldest") {
                    watermark_cache.get(&part_id).map(|(low, _)| *low).unwrap_or(0)
                } else {
                    0
                }
            });

            tpl.add_partition_offset(
                &topic_owned,
                part_id,
                rdkafka::Offset::Offset(start_offset),
            )
            .map_err(|e| AppError::Internal(format!("Failed to set offset: {}", e)))?;
        }

        // Assign 分区
        consumer
            .assign(&tpl)
            .map_err(|e| AppError::Internal(format!("Failed to assign partitions: {}", e)))?;

        // 批量消费消息
        let mut partition_counts: HashMap<i32, usize> = HashMap::with_capacity(partition_count);
        let mut raw_msgs: Vec<KafkaMessage> = Vec::with_capacity(max_messages * partition_count);
        let mut total_empty = 0;
        let max_empty_rounds = 100 + max_messages / 100 * 10;
        let max_poll_time = std::time::Duration::from_secs(30);
        let mut partition_reached_end: HashMap<i32, bool> = HashMap::with_capacity(partition_count);

        for &part_id in &partitions {
            if let Some(&(low, high)) = watermark_cache.get(&part_id) {
                if high <= low || high == 0 {
                    partition_reached_end.insert(part_id, true);
                    tracing::info!(
                        "[Remote] Partition {} is empty (low={}, high={}), marked as finished",
                        part_id, low, high
                    );
                }
            }
        }

        let partitions_ref = &partitions;
        let all_partitions_finished =
            |counts: &HashMap<i32, usize>, reached_end: &HashMap<i32, bool>| {
                partitions_ref.iter().all(|&p| {
                    let count = counts.get(&p).copied().unwrap_or(0);
                    let reached = reached_end.get(&p).copied().unwrap_or(false);
                    count >= max_messages || reached
                })
            };

        let target_total = max_messages * partition_count;

        let poll_start = std::time::Instant::now();
        let mut first_poll = true;
        while raw_msgs.len() < target_total
            && total_empty < max_empty_rounds
            && poll_start.elapsed() < max_poll_time
            && !all_partitions_finished(&partition_counts, &partition_reached_end)
        {
            let poll_timeout = if first_poll {
                first_poll = false;
                Duration::from_millis(500)
            } else {
                Duration::from_millis(10)
            };
            match consumer.poll(poll_timeout) {
                Some(Ok(msg)) => {
                    let part = msg.partition();

                    let count = partition_counts.get(&part).copied().unwrap_or(0);
                    if count >= max_messages {
                        if let Some((_, high)) = watermark_cache.get(&part) {
                            if msg.offset() >= high.saturating_sub(1) {
                                partition_reached_end.insert(part, true);
                            }
                        }
                        continue;
                    }

                    total_empty = 0;

                    let ts = msg.timestamp().to_millis();

                    if let Some(start) = start_time {
                        if let Some(t) = ts {
                            if t < start {
                                continue;
                            }
                        }
                    }
                    if let Some(end) = end_time {
                        if let Some(t) = ts {
                            if t > end {
                                continue;
                            }
                        }
                    }

                    let key = msg
                        .key()
                        .and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                    let value = msg
                        .payload()
                        .and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                    partition_counts.insert(part, count + 1);

                    // 检查是否到达分区边界
                    if let Some((low, high)) = watermark_cache.get(&part) {
                        let reached_beginning = msg.offset() <= *low;
                        let reached_end = msg.offset() >= high.saturating_sub(1);
                        let partition_full = count + 1 >= max_messages;
                        let reached_time_end = time_range_end_offsets
                            .get(&part)
                            .map(|end_off| msg.offset() >= *end_off)
                            .unwrap_or(false);

                        if reached_beginning || reached_end || partition_full || reached_time_end {
                            partition_reached_end.insert(part, true);
                        }
                    }

                    raw_msgs.push(KafkaMessage {
                        partition: part,
                        offset: msg.offset(),
                        key,
                        value,
                        timestamp: ts,
                    });
                }
                Some(Err(e)) => {
                    tracing::warn!("[Remote] Poll error: {}", e);
                    total_empty += 1;
                }
                None => {
                    total_empty += 1;
                }
            }
        }

        tracing::info!(
            "[Remote] Collected {} raw messages in {:?}",
            raw_msgs.len(),
            poll_start.elapsed()
        );

        // 搜索过滤
        let filtered = if let Some(term) = search_lower {
            raw_msgs
                .into_iter()
                .filter(|msg| {
                    let key_match = msg
                        .key
                        .as_ref()
                        .map(|k| k.to_lowercase().contains(&term))
                        .unwrap_or(false);
                    let value_match = msg
                        .value
                        .as_ref()
                        .map(|v| v.to_lowercase().contains(&term))
                        .unwrap_or(false);
                    key_match || value_match
                })
                .collect()
        } else {
            raw_msgs
        };

        // 排序
        let mut result = filtered;
        let is_desc = sort_owned.as_deref() == Some("desc")
            || (sort_owned.is_none() && fetch_mode_owned.as_deref() == Some("newest"));
        if is_desc {
            result.sort_by(|a, b| b.offset.cmp(&a.offset));
        } else {
            result.sort_by(|a, b| a.offset.cmp(&b.offset));
        }

        Ok(result)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Spawn blocking error: {:?}", e)))?
    .map_err(|e| e)?;

    tracing::info!(
        "[Remote] Fetched {} messages in {:?}",
        messages.len(),
        start_time_total.elapsed()
    );
    Ok(messages)
}

/// 本地Docker优化版 - 单consumer多分区
/// 并行获取单个分区的消息
fn fetch_partition_messages_parallel(
    brokers: String,
    topic: String,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    search: Option<String>,
    fetch_mode: Option<String>,
) -> Vec<KafkaMessage> {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};

    tracing::info!(
        "[Parallel] Starting fetch for partition {} of topic {} (max_messages: {})",
        partition,
        topic,
        max_messages
    );

    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers);
    cfg.set("group.id", "kafka-mgr-query");
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("session.timeout.ms", "3000");
    cfg.set("heartbeat.interval.ms", "500");
    cfg.set("fetch.min.bytes", "1");
    cfg.set("fetch.wait.max.ms", "1");
    cfg.set("fetch.max.bytes", "10485760");
    cfg.set("max.partition.fetch.bytes", "10485760");
    cfg.set("socket.nagle.disable", "true");
    cfg.set("socket.receive.buffer.bytes", "65536");
    cfg.set("enable.partition.eof", "false");
    cfg.set("connections.max.idle.ms", "540000");
    cfg.set("reconnect.backoff.ms", "50");
    cfg.set("reconnect.backoff.max.ms", "500");
    cfg.set("socket.connection.setup.timeout.ms", "3000");
    cfg.set("metadata.max.age.ms", "5000");
    cfg.set("partition.assignment.strategy", "");
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[Parallel] Failed to create consumer for partition {}: {}", partition, e);
            return Vec::new();
        }
    };

    // 计算起始 offset
    let start_offset = match calculate_partition_offset(
        &consumer,
        &topic,
        partition,
        max_messages,
        offset,
        start_time,
        fetch_mode.as_deref(),
    ) {
        Ok(o) => o,
        Err(e) => {
            tracing::error!("[Parallel] Failed to calculate offset for partition {}: {}", partition, e);
            return Vec::new();
        }
    };
    tracing::info!("[Parallel] Partition {} start_offset: {}", partition, start_offset);

    // 分配到指定分区
    let mut tpl = TopicPartitionList::new();
    let seek_offset = if start_offset < 0 {
        rdkafka::Offset::Beginning
    } else {
        rdkafka::Offset::Offset(start_offset)
    };
    if let Err(e) = tpl.add_partition_offset(&topic, partition, seek_offset) {
        tracing::error!("[Parallel] Failed to add partition {}: {}", partition, e);
        return Vec::new();
    }
    if let Err(e) = consumer.assign(&tpl) {
        tracing::error!("[Parallel] Failed to assign partition {}: {}", partition, e);
        return Vec::new();
    }

    // 预热线程
    let warmup_start = std::time::Instant::now();
    let mut warmed_up = false;
    while !warmed_up && warmup_start.elapsed() < Duration::from_millis(1000) {
        match consumer.poll(Duration::from_millis(100)) {
            Some(Ok(_)) => {
                if let Err(e) = consumer.seek(&topic, partition, seek_offset, Duration::from_secs(1)) {
                    tracing::warn!("[Parallel] Failed to seek back after warmup: {}", e);
                }
                warmed_up = true;
                tracing::info!("[Parallel] Warmup complete in {:?}, fetcher ready", warmup_start.elapsed());
            }
            Some(Err(_)) | None => {
                continue;
            }
        }
    }
    if !warmed_up {
        tracing::warn!("[Parallel] Warmup timeout after {:?}, fetcher may not be ready", warmup_start.elapsed());
    }

    // 计算结束 offset
    let end_offset = if fetch_mode.as_deref() == Some("newest") {
        match consumer.fetch_watermarks(&topic, partition, Duration::from_secs(5)) {
            Ok((_, high)) if high > 0 => Some(high),
            Err(e) => {
                tracing::warn!("[Parallel] Failed to fetch watermarks for partition {}: {}", partition, e);
                None
            }
            _ => None,
        }
    } else {
        None
    };

    let search_lower = search.map(|s| s.to_lowercase());
    let mut raw_messages = Vec::with_capacity(max_messages);
    let mut empty_count = 0;
    let mut polled_count = 0;
    let max_empty = 100 + max_messages / 100 * 10;
    let max_poll_time = std::time::Duration::from_secs(30);

    let poll_start = std::time::Instant::now();
    let mut got_first = false;
    while raw_messages.len() < max_messages
        && empty_count < max_empty
        && poll_start.elapsed() < max_poll_time
    {
        match consumer.poll(Duration::from_millis(10)) {
            Some(Ok(msg)) => {
                if !got_first {
                    got_first = true;
                    tracing::info!("[Parallel] First message received after {:?}", poll_start.elapsed());
                }
                polled_count += 1;
                empty_count = 0;

                // 时间范围过滤
                let ts = msg.timestamp().to_millis();
                if let Some(start) = start_time {
                    if let Some(t) = ts {
                        if t < start {
                            continue;
                        }
                    }
                }
                if let Some(end) = end_time {
                    if let Some(t) = ts {
                        if t > end {
                            continue;
                        }
                    }
                }

                // 检查是否到达结束 offset
                if let Some(end_off) = end_offset {
                    if msg.offset() >= end_off {
                        tracing::info!(
                            "[Parallel] Reached end offset {} for partition {}",
                            end_off, partition
                        );
                        break;
                    }
                }

                let key = msg
                    .key()
                    .and_then(|k| std::str::from_utf8(k).ok().map(String::from));
                let value = msg
                    .payload()
                    .and_then(|p| std::str::from_utf8(p).ok().map(String::from));

                raw_messages.push(KafkaMessage {
                    partition: msg.partition(),
                    offset: msg.offset(),
                    key,
                    value,
                    timestamp: ts,
                });
            }
            Some(Err(e)) => {
                tracing::warn!("[Parallel] Poll error for partition {}: {}", partition, e);
                empty_count += 1;
            }
            None => {
                empty_count += 1;
            }
        }
    }

    tracing::info!(
        "[Parallel] Partition {} finished: got {} messages in {:?} (polled: {}, empty: {})",
        partition,
        raw_messages.len(),
        poll_start.elapsed(),
        polled_count,
        empty_count
    );

    // 搜索过滤
    if let Some(term) = search_lower {
        raw_messages
            .into_iter()
            .filter(|msg| {
                let key_match = msg
                    .key
                    .as_ref()
                    .map(|k| k.to_lowercase().contains(&term))
                    .unwrap_or(false);
                let value_match = msg
                    .value
                    .as_ref()
                    .map(|v| v.to_lowercase().contains(&term))
                    .unwrap_or(false);
                key_match || value_match
            })
            .collect()
    } else {
        raw_messages
    }
}

/// 为 BaseConsumer 计算分区 offset
fn calculate_partition_offset(
    consumer: &BaseConsumer,
    topic: &str,
    partition: i32,
    max_messages: usize,
    offset: Option<i64>,
    start_time: Option<i64>,
    fetch_mode: Option<&str>,
) -> Result<i64> {
    use rdkafka::consumer::Consumer;

    // 如果用户指定了特定 offset，优先使用
    if let Some(off) = offset {
        if off >= 0 {
            tracing::info!("[calculate_partition_offset] Using user-specified offset: {}", off);
            return Ok(off);
        }
    }

    if let Some(time) = start_time {
        if time > 0 {
            let time_calc_start = std::time::Instant::now();
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(time))
                .ok();
            tracing::info!(
                "[calculate_partition_offset] Querying offset for time {} on partition {}",
                time, partition
            );
            match consumer.offsets_for_times(tpl, Duration::from_secs(3)) {
                Ok(r) => {
                    let elapsed = time_calc_start.elapsed();
                    let elements = r.elements_for_topic(topic);
                    for elem in elements {
                        if elem.partition() == partition {
                            if let Some(offset) = elem.offset().to_raw() {
                                tracing::info!(
                                    "[calculate_partition_offset] Using time-based offset: {} for time {}, took {:?}",
                                    offset, time, elapsed
                                );
                                return Ok(offset);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "[calculate_partition_offset] Failed to get offset for time {} on partition {}: {}",
                        time, partition, e
                    );
                }
            }
        }
    }

    match fetch_mode {
        Some("newest") | None => {
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, high)) if high > 0 => {
                    let latest = high.saturating_sub(1);
                    let start = latest
                        .saturating_sub((max_messages.saturating_sub(1)) as i64)
                        .max(low);
                    Ok(start)
                }
                _ => Ok(0),
            }
        }
        Some("oldest") => {
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, _)) => Ok(low),
                Err(_) => Ok(0),
            }
        }
        _ => Ok(0),
    }
}
