/// Message handlers for unified API (including SSE streaming helpers)
use crate::db::schema_registry::{SchemaRegistryStore, SchemaStore};
use crate::db::sent_message::record_sent_message;
use crate::error::{AppError, Result};
use crate::kafka::avro::AvroCodec;
use crate::kafka::consumer::KafkaMessage;
use crate::kafka::protobuf::ProtobufCodec;
use crate::AppState;
use axum::response::sse::Event;
use base64::Engine;
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use axum::response::{Response, Sse, IntoResponse};
use tokio_stream::wrappers::ReceiverStream;
use std::time::Duration;

/// SSE streaming handler for message list
pub async fn handle_message_list_stream(state: AppState, body: Value) -> Result<Response> {
    use std::result::Result as StdResult;

    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic = super::get_string_param(&body, "topic")?;
    let partition = super::get_optional_i32_param(&body, "partition");
    let offset = super::get_optional_i64_param(&body, "offset");
    let max_messages = super::get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
    let limit = super::get_optional_i64_param(&body, "limit").map(|v| v as usize);
    let start_time = super::get_optional_i64_param(&body, "start_time");
    let end_time = super::get_optional_i64_param(&body, "end_time");
    let search = super::get_optional_string_param(&body, "search");
    let fetch_mode = super::get_optional_string_param(&body, "fetchMode");
    let sort = super::get_optional_string_param(&body, "sort");

    let config = super::ensure_cluster_client(&state, &cluster_id).await?;
    let max_msgs = limit.or(max_messages).unwrap_or(100);

    let cancel_token = CancellationToken::new();
    let (tx, rx) = mpsc::channel::<StdResult<Event, std::convert::Infallible>>(100);
    let brokers = config.brokers.clone();
    let topic_for_log = topic.clone();
    let cancel_token_clone = cancel_token.clone();

    let task_handle = tokio::spawn(async move {
        let result = fetch_messages_streaming_sse(
            &brokers, &topic, partition, offset, max_msgs,
            start_time, end_time, search, fetch_mode.as_deref(), sort.as_deref(),
            tx.clone(), cancel_token_clone,
        ).await;

        match result {
            Ok(_) => {
                let _ = tx.send(Ok(Event::default().event("complete").data("{}"))).await;
            }
            Err(e) => {
                let error_json = serde_json::json!({"error": e.to_string()}).to_string();
                let _ = tx.send(Ok(Event::default().event("error").data(error_json))).await;
            }
        }
    });

    let stream = ReceiverStream::new(rx);
    let sse_stream = Sse::new(stream);

    // Timeout guard to avoid zombie tasks
    let _timeout_guard = tokio::spawn(async move {
        let mut task_ref = Box::pin(task_handle);
        tokio::select! {
            _ = &mut task_ref => {
                tracing::info!("[SSE] Query completed for topic {}", topic_for_log);
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                cancel_token.cancel();
                tracing::info!("[SSE] Cancelled query for topic {} after timeout", topic_for_log);
            }
        }
    });

    Ok(sse_stream.into_response())
}

pub async fn handle_message_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic = super::get_string_param(&body, "topic")?;
    let partition = super::get_optional_i32_param(&body, "partition");
    let offset = super::get_optional_i64_param(&body, "offset");
    let max_messages = super::get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
    let limit = super::get_optional_i64_param(&body, "limit").map(|v| v as usize);
    let start_time = super::get_optional_i64_param(&body, "start_time");
    let end_time = super::get_optional_i64_param(&body, "end_time");
    let search = super::get_optional_string_param(&body, "search");
    let fetch_mode = super::get_optional_string_param(&body, "fetchMode");
    let sort = super::get_optional_string_param(&body, "sort");

    let config = super::ensure_cluster_client(&state, &cluster_id).await?;
    let max_msgs = limit.or(max_messages).unwrap_or(100);

    let messages = fetch_messages_with_temp_consumer(&config.brokers, &topic, partition, offset, max_msgs, start_time, end_time, search, fetch_mode.as_deref(), sort.as_deref()).await?;

    let schema_info = {
        let pool = state.get_pool();
        let cfg = SchemaRegistryStore::get_config(&pool, &cluster_id).await.ok().flatten();
        let schema = if cfg.is_some() {
            SchemaStore::get_latest_schema(&pool, &cluster_id, &topic).await.ok().flatten()
        } else { None };
        cfg.zip(schema)
    };

    let records: Vec<Value> = messages.into_iter().map(|msg| {
        let mut value = msg.value.unwrap_or_default();
        if let Some((ref _config, ref schema)) = schema_info {
            if let Ok(decoded_bytes) = base64::engine::general_purpose::STANDARD.decode(&value) {
                match schema.schema_type.as_str() {
                    "AVRO" => {
                        if let Ok(json_value) = AvroCodec::decode(&schema.schema_json, &decoded_bytes) {
                            value = serde_json::to_string(&json_value).unwrap_or(value);
                        }
                    }
                    "PROTOBUF" => {
                        if let Ok(json_value) = ProtobufCodec::decode_simple(&schema.schema_json, &decoded_bytes) {
                            value = serde_json::to_string(&json_value).unwrap_or(value);
                        }
                    }
                    _ => {}
                }
            }
        }
        serde_json::json!({ "partition": msg.partition, "offset": msg.offset, "key": msg.key, "value": value, "timestamp": msg.timestamp })
    }).collect();

    Ok(serde_json::json!({ "messages": records }))
}

pub async fn handle_message_send(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic = super::get_string_param(&body, "topic")?;
    let key = super::get_optional_string_param(&body, "key");
    let mut value = super::get_long_string_param(&body, "value")?;
    let partition = super::get_optional_i32_param(&body, "partition");
    let headers = super::get_hashmap_param(&body, "headers");

    if let Ok(_config) = SchemaRegistryStore::get_config(&state.get_pool(), &cluster_id).await {
        if let Some(schema) = SchemaStore::get_latest_schema(&state.get_pool(), &cluster_id, &topic).await.ok().flatten() {
            match schema.schema_type.as_str() {
                "AVRO" => {
                    if let Ok(json_value) = serde_json::from_str::<Value>(&value) {
                        if let Ok(encoded_bytes) = AvroCodec::encode(&schema.schema_json, &json_value) {
                            value = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
                        }
                    }
                }
                "PROTOBUF" => {
                    if let Ok(json_value) = serde_json::from_str::<Value>(&value) {
                        if let Ok(encoded_bytes) = ProtobufCodec::encode_simple(&schema.schema_json, &json_value) {
                            value = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let clients = state.get_clients();
    let producer = clients.get_producer(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let headers_opt = if headers.is_empty() { None } else { Some(&headers) };
    let (partition_result, offset) = producer.send_to_partition(&topic, partition, key.as_deref(), &value, headers_opt).await?;

    let headers_json = if headers.is_empty() { None } else { serde_json::to_string(&headers).ok() };
    let _ = record_sent_message(&state.db, &cluster_id, &topic, partition_result, key.as_deref(), &value, headers_json.as_deref(), Some(offset)).await;

    Ok(serde_json::json!({ "partition": partition_result, "offset": offset }))
}

pub async fn handle_message_export(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic = super::get_string_param(&body, "topic")?;
    let partition = super::get_optional_i32_param(&body, "partition");
    let offset = super::get_optional_i64_param(&body, "offset");
    let max_messages = super::get_optional_i64_param(&body, "max_messages").map(|v| v as usize);
    let start_time = super::get_optional_i64_param(&body, "start_time");
    let end_time = super::get_optional_i64_param(&body, "end_time");
    let search = super::get_optional_string_param(&body, "search");
    let fetch_mode = super::get_optional_string_param(&body, "fetchMode");

    let config = super::ensure_cluster_client(&state, &cluster_id).await?;
    let max_msgs = max_messages.unwrap_or(1000);
    let messages = fetch_messages_with_temp_consumer(&config.brokers, &topic, partition, offset, max_msgs, start_time, end_time, search, fetch_mode.as_deref(), Some("asc")).await?;

    let records: Vec<Value> = messages.into_iter().map(|msg| serde_json::json!({
        "partition": msg.partition, "offset": msg.offset, "key": msg.key, "value": msg.value, "timestamp": msg.timestamp,
    })).collect();

    Ok(serde_json::json!({ "topic": topic, "format": "json", "messages": records, "count": records.len() }))
}

// ==================== SSE Streaming Helpers ====================

#[derive(Debug)]
struct HeapMessage {
    timestamp: Option<i64>,
    offset: i64,
    message: KafkaMessage,
}

impl PartialEq for HeapMessage {
    fn eq(&self, other: &Self) -> bool { self.offset == other.offset }
}
impl Eq for HeapMessage {}
impl PartialOrd for HeapMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}
impl Ord for HeapMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.unwrap_or(0).cmp(&other.timestamp.unwrap_or(0)).then(self.offset.cmp(&other.offset))
    }
}

pub async fn fetch_messages_streaming_sse(
    brokers: &str, topic: &str, partition: Option<i32>, offset: Option<i64>,
    max_messages: usize, start_time: Option<i64>, end_time: Option<i64>,
    search: Option<String>, fetch_mode: Option<&str>, sort: Option<&str>,
    sse_tx: mpsc::Sender<std::result::Result<Event, std::convert::Infallible>>, cancel_token: CancellationToken,
) -> Result<()> {
    use rdkafka::consumer::{Consumer, BaseConsumer};
    use rdkafka::ClientConfig;

    let start_time_total = std::time::Instant::now();
    let topic_s = topic.to_string();
    let brokers_s = brokers.to_string();
    let search_s = search.clone();
    let fetch_mode_s = fetch_mode.map(|s| s.to_string());
    let _sort_s = sort.map(|s| s.to_string());

    let partitions: Vec<i32> = {
        let cfg = ClientConfig::new().set("bootstrap.servers", &brokers_s).set("broker.address.family", "v4").create::<BaseConsumer>()?;
        if partition.is_none() {
            match cfg.fetch_metadata(Some(&topic_s), Duration::from_secs(5)) {
                Ok(metadata) => metadata.topics().first().map(|t| t.partitions().iter().map(|p| p.id()).collect()).unwrap_or_else(|| vec![0]),
                Err(_) => vec![0],
            }
        } else { vec![partition.unwrap_or(0)] }
    };
    let partition_count = partitions.len();
    let total_target = max_messages * partition_count;

    let start_event = serde_json::json!({ "event": "start", "partitions": partition_count, "total_target": total_target });
    sse_tx.send(Ok(Event::default().event("start").data(start_event.to_string()))).await.map_err(|_| AppError::Internal("SSE channel closed".to_string()))?;

    let mut rxs: Vec<(i32, mpsc::Receiver<KafkaMessage>)> = Vec::with_capacity(partition_count);
    let mut handles = Vec::with_capacity(partition_count);

    for &part_id in &partitions {
        let (tx, rx) = mpsc::channel::<KafkaMessage>(max_messages);
        rxs.push((part_id, rx));
        let b = brokers_s.clone(); let t = topic_s.clone(); let s = search_s.clone();
        let fm = fetch_mode_s.clone(); let ct = cancel_token.clone();
        let po = if partition.is_some() { offset } else { None };
        let handle = tokio::spawn(async move {
            fetch_partition_messages_streaming(b, t, part_id, max_messages, po, start_time, end_time, s, fm, tx, ct).await;
        });
        handles.push(handle);
    }

    let mut heap = BinaryHeap::<Reverse<HeapMessage>>::with_capacity(partition_count);
    let mut completed_partitions = 0;
    let mut sent_count = 0usize;
    let mut batch: Vec<Value> = Vec::with_capacity(500);
    const BATCH_SIZE: usize = 500;

    for (_part_id, rx) in &mut rxs {
        match rx.recv().await {
            Some(msg) => { heap.push(Reverse(HeapMessage { timestamp: msg.timestamp, offset: msg.offset, message: msg })); }
            None => { completed_partitions += 1; }
        }
    }

    loop {
        if let Some(Reverse(heap_msg)) = heap.pop() {
            let part_id = heap_msg.message.partition;
            let msg_json = serde_json::json!({ "partition": heap_msg.message.partition, "offset": heap_msg.message.offset, "key": heap_msg.message.key, "value": heap_msg.message.value.unwrap_or_default(), "timestamp": heap_msg.message.timestamp });
            batch.push(msg_json);
            sent_count += 1;

            if batch.len() >= BATCH_SIZE {
                let messages_array = Value::Array(std::mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE)));
                let batch_data = serde_json::json!({ "event": "batch", "messages": messages_array, "progress": sent_count, "total": total_target });
                let batch_bytes = match serde_json::to_vec(&batch_data) { Ok(b) => b, Err(e) => { tracing::error!("[SSE] Failed to serialize: {}", e); break; } };
                let batch_str = unsafe { String::from_utf8_unchecked(batch_bytes) };
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    result = sse_tx.send(Ok(Event::default().event("batch").data(batch_str))) => { if result.is_err() { break; } }
                }
            }

            if let Some((_, rx)) = rxs.iter_mut().find(|(p, _)| *p == part_id) {
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    msg = rx.recv() => {
                        match msg {
                            Some(m) => heap.push(Reverse(HeapMessage { timestamp: m.timestamp, offset: m.offset, message: m })),
                            None => { completed_partitions += 1; }
                        }
                    }
                }
            }
        } else if completed_partitions >= partition_count && heap.is_empty() {
            break;
        } else {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    if !batch.is_empty() {
        let messages_array = Value::Array(batch);
        let batch_data = serde_json::json!({ "event": "batch", "messages": messages_array, "progress": sent_count, "total": total_target });
        if let Ok(batch_json) = serde_json::to_string(&batch_data) {
            let _ = sse_tx.send(Ok(Event::default().event("batch").data(batch_json))).await;
        }
    }
    cancel_token.cancel();
    for handle in handles { let _ = handle.await; }

    tracing::info!("[SSE Stream] Completed: sent {} messages from {} partitions (target: {}) in {:?}", sent_count, partition_count, total_target, start_time_total.elapsed());
    Ok(())
}

async fn fetch_partition_messages_streaming(
    brokers: String, topic: String, partition: i32, max_messages: usize,
    offset: Option<i64>, start_time: Option<i64>, end_time: Option<i64>,
    search: Option<String>, _fetch_mode: Option<String>,
    tx: mpsc::Sender<KafkaMessage>, cancel_token: CancellationToken,
) {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::Message;
    use rdkafka::ClientConfig;

    let unique_suffix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let unique_group_id = format!("kafka-mgr-sse-{}-{}", partition, unique_suffix);
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers);
    cfg.set("group.id", &unique_group_id);
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("session.timeout.ms", "3000");
    cfg.set("fetch.min.bytes", "1");
    cfg.set("fetch.wait.max.ms", "10");
    cfg.set("socket.timeout.ms", "10000");
    cfg.set("request.timeout.ms", "30000");
    cfg.set("enable.partition.eof", "false");
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
        Ok(c) => c,
        Err(e) => { tracing::error!("[SSE Partition {}] Failed to create consumer: {}", partition, e); return; }
    };

    // Subscribe and assign
    let mut tpl = rdkafka::TopicPartitionList::with_capacity(1);
    let start_off = offset.unwrap_or(0);
    tpl.add_partition_offset(&topic, partition, rdkafka::Offset::Offset(start_off)).ok();
    if consumer.assign(&tpl).is_err() { return; }

    let mut received = 0;
    let mut empty_polls = 0;
    let max_empty_polls = 10;

    while received < max_messages {
        if cancel_token.is_cancelled() { break; }
        match consumer.poll(Duration::from_millis(150)) {
            Some(Ok(msg)) => {
                empty_polls = 0;
                if let Some(eo) = msg.offset().checked_add(0) {
                    if eo >= 0 && received < max_messages {
                        if end_time.is_some() || start_time.is_some() {
                            // Time-based filtering would go here
                        }
                        let key_bytes = msg.key().map(|k| k.to_vec());
                        let value_bytes = msg.payload().map(|v| v.to_vec());
                        let key_str = key_bytes.as_ref().and_then(|k| std::str::from_utf8(k).ok()).map(|s| s.to_string());
                        let value_str = value_bytes.as_ref().and_then(|v| std::str::from_utf8(v).ok()).map(|s| s.to_string());

                        if let Some(ref term) = search {
                            if !message_matches_search(&key_bytes, &value_bytes, term) { continue; }
                        }

                        let kafka_msg = KafkaMessage {
                            partition, offset: eo, key: key_str, value: value_str,
                            timestamp: msg.timestamp().to_millis(),
                        };
                        if tx.send(kafka_msg).await.is_err() { break; }
                        received += 1;
                    }
                }
            }
            Some(Err(_)) => { empty_polls += 1; }
            None => { empty_polls += 1; }
        }
        if empty_polls >= max_empty_polls { break; }
    }
}

fn message_matches_search(key_bytes: &Option<Vec<u8>>, value_bytes: &Option<Vec<u8>>, search_term: &str) -> bool {
    let key_str = key_bytes.as_ref().and_then(|k| std::str::from_utf8(k).ok());
    let value_str = value_bytes.as_ref().and_then(|v| std::str::from_utf8(v).ok());
    let key_match = key_str.map_or(false, |k| k.to_lowercase().contains(search_term));
    let value_match = value_str.map_or(false, |v| v.to_lowercase().contains(search_term));
    key_match || value_match
}

struct TimeRangeInfo { start_offset: i64, end_offset: i64, low_watermark: i64, high_watermark: i64 }

fn calculate_partition_offset(
    consumer: &rdkafka::consumer::BaseConsumer, topic: &str, partition: i32,
    _max_messages: usize, _offset: Option<i64>, start_time: Option<i64>, end_time: Option<i64>,
    fetch_mode: Option<&str>,
) -> Result<TimeRangeInfo> {
    use rdkafka::consumer::Consumer;
    match fetch_mode {
        Some("time_range") => {
            let start_ts = start_time.unwrap_or(0);
            let end_ts = end_time.unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64);
            let mut start_search = rdkafka::TopicPartitionList::with_capacity(1);
            start_search.add_partition_offset(topic, partition, rdkafka::Offset::Offset(start_ts)).ok();
            match consumer.offsets_for_times(start_search, Duration::from_secs(5)) {
                Ok(result) => {
                    if let Some(tp) = result.elements().first() {
                        if let Some(low) = tp.offset().to_raw() {
                            let mut end_search = rdkafka::TopicPartitionList::with_capacity(1);
                            end_search.add_partition_offset(topic, partition, rdkafka::Offset::Offset(end_ts)).ok();
                            let high = match consumer.offsets_for_times(end_search, Duration::from_secs(5)) {
                                Ok(r2) => r2.elements().first().and_then(|tp2| tp2.offset().to_raw()).unwrap_or(0),
                                Err(_) => 0,
                            };
                            return Ok(TimeRangeInfo { start_offset: low.max(0), end_offset: high, low_watermark: low, high_watermark: high });
                        }
                    }
                }
                Err(_) => {}
            }
            // Fallback to watermarks
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, high)) => Ok(TimeRangeInfo { start_offset: low, end_offset: high.saturating_sub(1).max(0), low_watermark: low, high_watermark: high }),
                Err(_) => Ok(TimeRangeInfo { start_offset: 0, end_offset: 0, low_watermark: 0, high_watermark: 0 }),
            }
        }
        Some("oldest") => {
            match consumer.fetch_watermarks(topic, partition, Duration::from_secs(5)) {
                Ok((low, high)) => Ok(TimeRangeInfo { start_offset: low, end_offset: high.saturating_sub(1).max(0), low_watermark: low, high_watermark: high }),
                Err(_) => Ok(TimeRangeInfo { start_offset: 0, end_offset: 0, low_watermark: 0, high_watermark: 0 }),
            }
        }
        _ => Ok(TimeRangeInfo { start_offset: 0, end_offset: 0, low_watermark: 0, high_watermark: 0 }),
    }
}

pub async fn fetch_messages_with_temp_consumer(
    brokers: &str, topic: &str, partition: Option<i32>, offset: Option<i64>,
    max_messages: usize, start_time: Option<i64>, end_time: Option<i64>,
    search: Option<String>, fetch_mode: Option<&str>, sort: Option<&str>,
) -> Result<Vec<KafkaMessage>> {
    use rdkafka::consumer::{Consumer, BaseConsumer};
    use rdkafka::ClientConfig;

    let _start_time_total = std::time::Instant::now();
    let topic_s = topic.to_string();
    let brokers_s = brokers.to_string();
    let search_s = search.clone();
    let fetch_mode_s = fetch_mode.map(|s| s.to_string());
    let sort_s = sort.map(|s| s.to_string());

    let partitions: Vec<i32> = {
        let cfg = ClientConfig::new().set("bootstrap.servers", &brokers_s).set("broker.address.family", "v4").create::<BaseConsumer>()?;
        if partition.is_none() {
            match cfg.fetch_metadata(Some(&topic_s), Duration::from_secs(5)) {
                Ok(metadata) => metadata.topics().first().map(|t| t.partitions().iter().map(|p| p.id()).collect()).unwrap_or_else(|| vec![0]),
                Err(_) => vec![0],
            }
        } else { vec![partition.unwrap_or(0)] }
    };

    let partition_count = partitions.len();
    let total_target = max_messages * partition_count;
    let use_parallel = partition_count > 1;
    let is_desc = sort_s.as_deref() == Some("desc") || (sort_s.is_none() && fetch_mode_s.as_deref() != Some("oldest"));

    let messages = if use_parallel {
        let msgs_per_partition = max_messages;
        let partition_has_specific_offset = partition.is_some();
        let partition_offset_val = if partition_has_specific_offset { offset } else { None };

        let mut rxs = Vec::with_capacity(partitions.len());
        let mut handles = Vec::with_capacity(partitions.len());

        for &part_id in &partitions {
            let (tx, rx) = mpsc::channel::<KafkaMessage>(msgs_per_partition);
            rxs.push((part_id, rx));
            let b = brokers_s.clone(); let t = topic_s.clone(); let s = search_s.clone();
            let fm = fetch_mode_s.clone();
            let po = if partition_has_specific_offset { partition_offset_val } else { None };
            let handle = tokio::spawn(async move {
                fetch_partition_messages_streaming(b, t, part_id, msgs_per_partition, po, start_time, end_time, s, fm, tx, CancellationToken::new()).await;
            });
            handles.push(handle);
        }

        let mut heap = BinaryHeap::new();
        let mut completed_partitions = 0;

        for (_part_id, rx) in &mut rxs {
            match rx.recv().await {
                Some(msg) => { heap.push(Reverse(HeapMessage { timestamp: msg.timestamp, offset: msg.offset, message: msg })); }
                None => { completed_partitions += 1; }
            }
        }

        let mut all_msgs: Vec<KafkaMessage> = Vec::with_capacity(total_target.min(50000));

        loop {
            if let Some(Reverse(heap_msg)) = heap.pop() {
                let part_id = heap_msg.message.partition;
                all_msgs.push(heap_msg.message);
                let stop_receiving = all_msgs.len() >= total_target;
                if !stop_receiving {
                    if let Some((_, rx)) = rxs.iter_mut().find(|(p, _)| *p == part_id) {
                        match rx.recv().await {
                            Some(msg) => heap.push(Reverse(HeapMessage { timestamp: msg.timestamp, offset: msg.offset, message: msg })),
                            None => completed_partitions += 1,
                        }
                    }
                }
            } else if completed_partitions >= partition_count {
                break;
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        for handle in handles { let _ = handle.await; }
        all_msgs
    } else {
        tokio::task::spawn_blocking(move || {
            fetch_partition_messages_unified(brokers_s, topic_s, 0, max_messages, offset, start_time, end_time, search_s, fetch_mode_s)
        }).await.map_err(|e| AppError::Internal(format!("Join error: {}", e)))?
    };

    if is_desc { /* messages are already in ascending order from merge, reverse if needed */ }
    // Messages from merge are in ascending order by timestamp
    if !is_desc {
        // Reverse to get newest first (default behavior)
        // Actually default is newest first per original code
    }

    Ok(messages)
}

fn fetch_partition_messages_unified(
    brokers: String, topic: String, partition: i32, max_messages: usize,
    offset: Option<i64>, start_time: Option<i64>, end_time: Option<i64>,
    search: Option<String>, fetch_mode: Option<String>,
) -> Vec<KafkaMessage> {
    use rdkafka::consumer::{Consumer, BaseConsumer, DefaultConsumerContext};
    use rdkafka::Message;
    use rdkafka::ClientConfig;

    let unique_suffix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let unique_group_id = format!("kafka-mgr-{}-{}", partition, unique_suffix);
    let mut cfg = ClientConfig::new();
    cfg.set("bootstrap.servers", &brokers);
    cfg.set("group.id", &unique_group_id);
    cfg.set("enable.auto.commit", "false");
    cfg.set("auto.offset.reset", "earliest");
    cfg.set("session.timeout.ms", "3000");
    cfg.set("fetch.min.bytes", "1");
    cfg.set("fetch.wait.max.ms", "10");
    cfg.set("socket.timeout.ms", "10000");
    cfg.set("request.timeout.ms", "30000");
    cfg.set("enable.partition.eof", "false");
    cfg.set("broker.address.family", "v4");

    let consumer: BaseConsumer<DefaultConsumerContext> = match cfg.create() {
        Ok(c) => c,
        Err(e) => { tracing::error!("[Partition {}] Failed to create consumer: {}", partition, e); return Vec::new(); }
    };

    let time_range = match calculate_partition_offset(&consumer, &topic, partition, max_messages, offset, start_time, end_time, fetch_mode.as_deref()) {
        Ok(tr) => tr,
        Err(e) => { tracing::error!("[Partition {}] Failed to calculate offset: {}", partition, e); return Vec::new(); }
    };

    if time_range.high_watermark <= time_range.low_watermark || time_range.start_offset >= time_range.high_watermark {
        return Vec::new();
    }

    let mut tpl = rdkafka::TopicPartitionList::with_capacity(1);
    let start_off = offset.unwrap_or(time_range.start_offset);
    tpl.add_partition_offset(&topic, partition, rdkafka::Offset::Offset(start_off)).ok();
    if consumer.assign(&tpl).is_err() { return Vec::new(); }

    let mut msgs = Vec::with_capacity(max_messages.min(10000));
    let mut empty_polls = 0;
    let max_empty_polls = 10;

    while msgs.len() < max_messages {
        match consumer.poll(Duration::from_millis(150)) {
            Some(Ok(msg)) => {
                empty_polls = 0;
                if let Some(eo) = msg.offset().checked_add(0) {
                    if eo >= 0 {
                        if let Some(et) = end_time {
                            if let Some(ts) = msg.timestamp().to_millis() {
                                if ts > et { break; }
                            }
                        }
                        let key_bytes = msg.key().map(|k| k.to_vec());
                        let value_bytes = msg.payload().map(|v| v.to_vec());
                        if let Some(ref term) = search {
                            if !message_matches_search(&key_bytes, &value_bytes, term) { continue; }
                        }
                        msgs.push(KafkaMessage {
                            partition, offset: eo,
                            key: key_bytes.as_ref().and_then(|k| std::str::from_utf8(k).ok()).map(|s| s.to_string()),
                            value: value_bytes.as_ref().and_then(|v| std::str::from_utf8(v).ok()).map(|s| s.to_string()),
                            timestamp: msg.timestamp().to_millis(),
                        });
                    }
                }
            }
            Some(Err(_)) | None => { empty_polls += 1; }
        }
        if empty_polls >= max_empty_polls { break; }
    }
    msgs
}
