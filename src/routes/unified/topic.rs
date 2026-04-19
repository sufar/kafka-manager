/// Topic handlers for unified API
use crate::db::cluster::ClusterStore;
use crate::db::topic::TopicStore;
use crate::error::{AppError, Result};
use crate::kafka::offset::KafkaOffsetManager;
use crate::kafka::throughput::KafkaThroughputCalculator;
use crate::AppState;
use serde_json::Value;

pub async fn handle_topic_list(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let cluster_id = match cluster_id {
        Some(id) => id,
        None => return handle_topic_list_all_clusters(state).await,
    };
    let db_topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await.ok();
    if let Some(topics) = db_topics {
        if !topics.is_empty() {
            let topic_names: Vec<String> = topics.into_iter().map(|t| t.topic_name).collect();
            return Ok(serde_json::json!({ "topics": topic_names }));
        }
    }
    sync_topics_from_kafka(state, &cluster_id).await
}

pub async fn handle_topic_list_with_cluster(state: AppState, body: Value) -> Result<Value> {
    #[derive(serde::Serialize)]
    struct TopicWithCluster { name: String, cluster: String }

    let cluster_ids: Option<Vec<String>> = body.get("cluster_ids").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    });
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let offset = body.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(10000) as usize;
    let search_query = body.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());

    let clusters_to_fetch: Vec<(i64, String)> = if let Some(ref ids) = cluster_ids {
        if ids.is_empty() {
            ClusterStore::list(state.db.inner()).await.ok().unwrap_or_default().into_iter().map(|c| (c.id, c.name)).collect()
        } else {
            let mut result = Vec::with_capacity(ids.len());
            for id in ids {
                if let Ok(Some(cluster)) = ClusterStore::get_by_name(state.db.inner(), id).await {
                    result.push((cluster.id, cluster.name.clone()));
                }
            }
            result
        }
    } else if let Some(ref id) = cluster_id {
        if let Ok(Some(cluster)) = ClusterStore::get_by_name(state.db.inner(), id).await {
            vec![(cluster.id, cluster.name.clone())]
        } else { vec![] }
    } else {
        ClusterStore::list(state.db.inner()).await.ok().unwrap_or_default().into_iter().map(|c| (c.id, c.name)).collect()
    };

    if let Some(query) = search_query.filter(|q| !q.is_empty()) {
        let search_cluster_ids: Vec<String> = cluster_ids.as_ref().map(|ids| {
            if ids.is_empty() { Vec::new() } else { ids.clone() }
        }).unwrap_or_default();
        let (topics, total) = TopicStore::search_topics_with_filter(
            state.db.inner(), &query, &search_cluster_ids, offset as u32, limit as u32,
        ).await?;
        let all_topics: Vec<TopicWithCluster> = topics.into_iter().map(|t| TopicWithCluster { name: t.topic_name, cluster: t.cluster_id }).collect();
        let end = (offset + limit).min(total as usize);
        return Ok(serde_json::json!({
            "topics": all_topics, "total": total, "offset": offset, "limit": limit, "has_more": end < total as usize
        }));
    }

    let mut all_topics: Vec<TopicWithCluster> = Vec::with_capacity(clusters_to_fetch.len() * 50);
    for (_cluster_id, cluster_name) in &clusters_to_fetch {
        if let Ok(topics) = TopicStore::list_by_cluster(state.db.inner(), cluster_name).await {
            for topic in topics {
                all_topics.push(TopicWithCluster { name: topic.topic_name, cluster: cluster_name.clone() });
            }
        }
    }
    all_topics.sort_by(|a, b| a.cluster.cmp(&b.cluster).then(a.name.cmp(&b.name)));
    let total = all_topics.len();
    let end = (offset + limit).min(total);
    let paginated = if offset < total { all_topics.into_iter().skip(offset).take(limit).collect() } else { Vec::new() };
    Ok(serde_json::json!({ "topics": paginated, "total": total, "offset": offset, "limit": limit, "has_more": end < total }))
}

async fn handle_topic_list_all_clusters(state: AppState) -> Result<Value> {
    let clusters = ClusterStore::list(state.db.inner()).await?;
    let mut all_topics: Vec<String> = Vec::with_capacity(clusters.len() * 50);
    for cluster in clusters {
        if let Ok(topics) = TopicStore::list_by_cluster(state.db.inner(), &cluster.name).await {
            for topic in topics { all_topics.push(topic.topic_name); }
        }
    }
    all_topics.sort();
    all_topics.dedup();
    Ok(serde_json::json!({ "topics": all_topics }))
}

async fn sync_topics_from_kafka(state: AppState, cluster_id: &str) -> Result<Value> {
    let admin = super::get_or_create_admin_client(&state, cluster_id).await?;
    let mut last_error = None;
    for attempt in 0..3 {
        let admin_clone = admin.clone();
        let result = tokio::task::spawn_blocking(move || admin_clone.list_topics()).await
            .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?;
        match result {
            Ok(topics) => {
                let _ = TopicStore::sync_topics(state.db.inner(), cluster_id, &topics).await;
                return Ok(serde_json::json!({ "topics": topics }));
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < 2 {
                    let wait_ms = 200 * (attempt + 1) as u64;
                    tracing::warn!("list_topics failed (attempt {}), retrying in {}ms: {}", attempt + 1, wait_ms, last_error.as_ref().map(|e| format!("{}", e)).unwrap_or_default());
                    tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
                }
            }
        }
    }
    last_error.map_or_else(|| Err(AppError::Internal("Unknown error occurred".to_string())), Err)
}

pub async fn handle_topic_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "name")?;
    let admin = super::get_or_create_admin_client(&state, &cluster_id).await?;
    let topic_info = tokio::task::spawn_blocking({
        let admin = admin.clone();
        let name = name.clone();
        move || admin.get_topic_info(&name)
    }).await.map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;
    Ok(serde_json::json!({
        "name": topic_info.name,
        "partitions": topic_info.partitions.into_iter().map(|p| serde_json::json!({
            "id": p.id, "leader": p.leader, "replicas": p.replicas, "isr": p.isr,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn handle_topic_create(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "name")?;
    let num_partitions = super::get_optional_i32_param(&body, "num_partitions").unwrap_or(1);
    let replication_factor = super::get_optional_i32_param(&body, "replication_factor").unwrap_or(1);
    let config = super::get_hashmap_param(&body, "config");
    let admin = super::get_or_create_admin_client(&state, &cluster_id).await?;
    admin.create_topic(&name, num_partitions, replication_factor, config).await?;
    Ok(serde_json::json!({ "name": name }))
}

pub async fn handle_topic_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "topic").or_else(|_| super::get_string_param(&body, "name"))?;
    let admin = super::get_or_create_admin_client(&state, &cluster_id).await?;
    admin.delete_topic(&name).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_topic_batch_create(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topics = body.get("topics").and_then(|v| v.as_array()).ok_or_else(|| AppError::BadRequest("Missing topics array".to_string()))?;
    let continue_on_error = super::get_optional_bool_param(&body, "continue_on_error").unwrap_or(false);
    let admin = super::get_or_create_admin_client(&state, &cluster_id).await?;
    let mut created = Vec::with_capacity(topics.len());
    let mut failed = Vec::with_capacity(topics.len());
    for topic_req in topics {
        let name = topic_req.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let num_partitions = topic_req.get("num_partitions").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(1);
        let replication_factor = topic_req.get("replication_factor").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(1);
        let config = topic_req.get("config").and_then(|v| v.as_object()).map(|obj| obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()).unwrap_or_default();
        match admin.create_topic(&name, num_partitions, replication_factor, config).await {
            Ok(_) => created.push(name),
            Err(e) => {
                failed.push(serde_json::json!({ "name": name, "error": e.to_string() }));
                if !continue_on_error { return Ok(serde_json::json!({ "success": false, "created": created, "failed": failed })); }
            }
        }
    }
    Ok(serde_json::json!({ "success": failed.is_empty(), "created": created, "failed": failed }))
}

pub async fn handle_topic_batch_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topics = super::get_string_array_param(&body, "topics");
    let continue_on_error = super::get_optional_bool_param(&body, "continue_on_error").unwrap_or(false);
    let admin = super::get_or_create_admin_client(&state, &cluster_id).await?;
    let mut deleted = Vec::with_capacity(topics.len());
    let mut failed = Vec::with_capacity(topics.len());
    for topic_name in topics {
        match admin.delete_topic(&topic_name).await {
            Ok(_) => deleted.push(topic_name),
            Err(e) => {
                failed.push(serde_json::json!({ "name": topic_name, "error": e.to_string() }));
                if !continue_on_error { return Ok(serde_json::json!({ "success": false, "deleted": deleted, "failed": failed })); }
            }
        }
    }
    Ok(serde_json::json!({ "success": failed.is_empty(), "deleted": deleted, "failed": failed }))
}

pub async fn handle_topic_delete_all(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let topic_names: Vec<String> = topics.iter().map(|t| t.topic_name.clone()).collect();
    if topic_names.is_empty() {
        return Ok(serde_json::json!({ "success": true, "deleted": Vec::<String>::new(), "failed": Vec::<serde_json::Value>::new(), "total_deleted": 0, "total_failed": 0 }));
    }
    for topic_name in &topic_names { let _ = TopicStore::delete(state.db.inner(), &cluster_id, topic_name).await; }
    Ok(serde_json::json!({ "success": true, "deleted": topic_names, "failed": Vec::<serde_json::Value>::new(), "total_deleted": topic_names.len(), "total_failed": 0 }))
}

pub async fn handle_topic_offsets(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "name")?;
    let clients = state.get_clients();
    let config = clients.get_config(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let offset_manager = KafkaOffsetManager::new(&config);
    let partition_offsets = offset_manager.get_topic_partition_offsets(&config, &name)?;
    let details: Vec<Value> = partition_offsets.into_iter().map(|p| serde_json::json!({
        "topic": p.topic, "partition": p.partition, "leader": p.leader, "replicas": p.replicas, "isr": p.isr,
        "earliest_offset": p.earliest_offset, "latest_offset": p.latest_offset,
        "first_commit_time": p.first_commit_time, "last_commit_time": p.last_commit_time,
    })).collect();
    Ok(serde_json::json!({ "offsets": details }))
}

pub async fn handle_topic_partition_watermarks(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topic = super::get_string_param(&body, "topic")?;
    let partition = super::get_i32_param(&body, "partition")?;
    let clients = state.get_clients();
    let config = clients.get_config(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let admin = clients.get_admin(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let watermarks = admin.get_partition_watermarks(&topic, partition, &config.brokers)?;
    Ok(serde_json::json!({ "low_offset": watermarks.0, "high_offset": watermarks.1 }))
}

pub async fn handle_topic_config_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "name")?;
    let clients = state.get_clients();
    let admin = clients.get_admin(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let config = admin.get_topic_config(&name).await?;
    Ok(serde_json::json!(config))
}

pub async fn handle_topic_config_alter(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "name")?;
    let config = super::get_hashmap_param(&body, "config");
    let clients = state.get_clients();
    let admin = clients.get_admin(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    admin.alter_topic_config(&name, config).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_topic_partitions_add(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "name")?;
    let new_partitions = super::get_i32_param(&body, "new_partitions")?;
    let clients = state.get_clients();
    let admin = clients.get_admin(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    admin.create_partitions(&name, new_partitions).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_topic_throughput(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let name = super::get_string_param(&body, "name")?;
    let clients = state.get_clients();
    let config = clients.get_config(&cluster_id).ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;
    let calculator = KafkaThroughputCalculator::new(&config);
    let throughput = calculator.calculate_topic_throughput(&config, &name)?;
    Ok(serde_json::json!({
        "topic": throughput.topic,
        "produce_throughput": { "messages_per_second": throughput.produce_throughput.messages_per_second, "bytes_per_second": throughput.produce_throughput.bytes_per_second, "window_seconds": throughput.produce_throughput.window_seconds },
        "total_messages": throughput.total_messages,
        "partitions": throughput.partitions.iter().map(|p| serde_json::json!({
            "partition": p.partition, "earliest_offset": p.earliest_offset, "latest_offset": p.latest_offset,
            "message_count": p.message_count, "produce_rate": p.produce_rate,
            "first_message_time": p.first_message_time, "last_message_time": p.last_message_time,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn handle_topic_refresh(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(String::from);
    {
        let refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        if let Some(ref cluster) = cluster_id {
            if refresh_state.refreshing_clusters.contains(cluster) {
                return Err(AppError::BadRequest(format!("Cluster '{}' is already being refreshed, please wait", cluster)));
            }
        }
    }
    tokio::spawn(async move {
        if let Some(cluster_id) = cluster_id { refresh_single_cluster(state, cluster_id).await; }
        else { refresh_all_clusters(state).await; }
    });
    Ok(serde_json::json!({ "success": true, "message": "Topic refresh started in background" }))
}

pub async fn refresh_single_cluster(state: AppState, cluster_id: String) {
    let cluster = match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
        Ok(Some(cluster)) => cluster,
        Ok(None) => { tracing::error!("Cluster '{}' not found in database", cluster_id); return; }
        Err(e) => { tracing::error!("Failed to get cluster config: {}", e); return; }
    };
    let config = crate::config::KafkaConfig { brokers: cluster.brokers, request_timeout_ms: cluster.request_timeout_ms as u32, operation_timeout_ms: cluster.operation_timeout_ms as u32 };
    let clients = state.get_clients();
    let clients = if clients.get_admin(&cluster_id).is_some() {
        match clients.reconnect_cluster(&cluster_id, &config) {
            Ok(nc) => { state.set_clients(nc.clone()); nc }
            Err(e) => { tracing::error!("Failed to reconnect cluster '{}': {}", cluster_id, e); return; }
        }
    } else {
        match clients.with_added_cluster(&cluster_id, &config) {
            Ok(nc) => { state.set_clients(nc.clone()); nc }
            Err(e) => { tracing::error!("Failed to add cluster '{}': {}", cluster_id, e); return; }
        }
    };
    let admin = match clients.get_admin(&cluster_id) {
        Some(a) => a,
        None => { tracing::error!("Failed to get admin client for cluster '{}'", cluster_id); return; }
    };
    let _ = state.pools.reconnect(&cluster_id, &config, &state.config.pool).await;
    let current_topics = {
        let admin = admin.clone();
        match tokio::task::spawn_blocking(move || admin.list_topics()).await {
            Ok(Ok(topics)) => topics,
            Ok(Err(e)) => { tracing::error!("Failed to list topics: {}", e); return; }
            Err(e) => { tracing::error!("Task join error: {}", e); return; }
        }
    };
    match TopicStore::sync_topics(state.db.inner(), &cluster_id, &current_topics).await {
        Ok(sync_result) => {
            tracing::info!("Refreshed topics for cluster '{}': +{} -{}", cluster_id, sync_result.added.len(), sync_result.removed.len());
            for topic_name in &sync_result.added {
                let topic_name = topic_name.clone();
                let admin = admin.clone();
                let cluster_id = cluster_id.clone();
                let db = state.db.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(topic_info) = admin.get_topic_info(&topic_name) {
                        let config = std::collections::HashMap::with_capacity(0);
                        let _ = TopicStore::upsert(db.inner(), &cluster_id, &topic_name, topic_info.partitions.len() as i32, 1, &config);
                    }
                }).await;
            }
        }
        Err(e) => { tracing::error!("Failed to sync topics: {}", e); }
    }
}

async fn refresh_all_clusters(state: AppState) {
    let clusters = match ClusterStore::list(state.db.inner()).await {
        Ok(clusters) => clusters,
        Err(e) => { tracing::error!("Failed to list clusters: {}", e); return; }
    };
    tracing::info!("Refreshing all {} clusters in parallel", clusters.len());
    let mut tasks = Vec::with_capacity(clusters.len());
    for cluster in clusters {
        let state = state.clone();
        let cluster_id = cluster.name;
        tasks.push(tokio::spawn(async move { refresh_single_cluster(state, cluster_id).await; }));
    }
    for task in tasks { let _ = task.await; }
    tracing::info!("Completed refreshed all clusters");
}

pub async fn handle_topic_search(state: AppState, body: Value) -> Result<Value> {
    let keyword = body.get("keyword").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
    let start = std::time::Instant::now();
    tracing::info!("[search] keyword: {:?}", keyword);
    let topics = if let Some(kw) = keyword {
        TopicStore::search_topics(state.db.inner(), kw).await?
    } else {
        TopicStore::list_all_limit(state.db.inner(), 100).await?
    };
    tracing::info!("[search] found {} topics in {:?}", topics.len(), start.elapsed());
    let results: Vec<Value> = topics.into_iter().map(|topic| serde_json::json!({ "cluster": topic.cluster_id, "topic": topic.topic_name })).collect();
    Ok(serde_json::json!({ "results": results }))
}

pub async fn handle_topic_count(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    Ok(serde_json::json!({ "count": topics.len() }))
}

pub async fn handle_topic_cleanup_orphans(state: AppState, _body: Value) -> Result<Value> {
    let clusters = ClusterStore::list(state.db.inner()).await?;
    let valid_cluster_ids: Vec<String> = clusters.into_iter().map(|c| c.name).collect();
    let removed = TopicStore::cleanup_orphan_topics(state.db.inner(), &valid_cluster_ids).await?;
    Ok(serde_json::json!({ "success": true, "removed": removed, "count": removed.len() }))
}

pub async fn handle_topic_saved(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;
    let topics = TopicStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let topic_names: Vec<String> = topics.into_iter().map(|t| t.topic_name).collect();
    Ok(serde_json::json!({ "topics": topic_names }))
}
