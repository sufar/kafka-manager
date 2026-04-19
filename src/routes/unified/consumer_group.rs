/// Consumer Group handlers for unified API
use crate::db::consumer_group::ConsumerGroupStore;
use crate::error::{AppError, Result};
use crate::kafka::consumer_group::KafkaConsumerGroupManager;
use crate::{AppState, RefreshState};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// RefreshGuard - RAII guard to automatically clear refresh state when refresh completes
struct RefreshGuard {
    cluster_id: String,
    refresh_state: Arc<Mutex<RefreshState>>,
}

impl Drop for RefreshGuard {
    fn drop(&mut self) {
        let mut state = self.refresh_state.lock().expect("refresh state poisoned");
        state.refreshing_clusters.remove(&self.cluster_id);
    }
}

pub async fn handle_consumer_group_refresh(state: AppState, body: Value) -> Result<Value> {
    // cluster_id 是可选参数，未指定时刷新所有集群
    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(String::from);

    // 检查并设置刷新状态
    {
        let refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        if let Some(ref cluster) = cluster_id {
            if refresh_state.refreshing_clusters.contains(cluster) {
                return Err(AppError::BadRequest(format!(
                    "Cluster '{}' is already being refreshed, please wait",
                    cluster
                )));
            }
        }
    }

    // 立即返回，后台异步刷新
    tokio::spawn(async move {
        if let Some(cluster_id) = cluster_id {
            refresh_single_consumer_group(state, cluster_id).await;
        } else {
            refresh_all_consumer_groups(state).await;
        }
    });

    Ok(serde_json::json!({
        "success": true,
        "message": "Consumer group refresh started in background",
    }))
}

/// 刷新单个集群的 Consumer Group 列表
pub async fn refresh_single_consumer_group(state: AppState, cluster_id: String) {
    use crate::db::cluster::ClusterStore;

    // 标记为正在刷新（如果已在刷新则直接返回）
    {
        let mut refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
        if !refresh_state.refreshing_clusters.insert(cluster_id.clone()) {
            tracing::info!("Cluster '{}' consumer group refresh already in progress, skipping", cluster_id);
            return;
        }
    }

    // 确保退出时清除标记
    let _guard = RefreshGuard {
        cluster_id: cluster_id.clone(),
        refresh_state: state.refresh_state.clone(),
    };

    // 从数据库获取集群配置
    let cluster = match ClusterStore::get_by_name(state.db.inner(), &cluster_id).await {
        Ok(Some(cluster)) => cluster,
        Ok(None) => {
            tracing::error!("Cluster '{}' not found in database", cluster_id);
            return;
        }
        Err(e) => {
            tracing::error!("Failed to get cluster config: {}", e);
            return;
        }
    };

    let config = crate::config::KafkaConfig {
        brokers: cluster.brokers,
        request_timeout_ms: cluster.request_timeout_ms as u32,
        operation_timeout_ms: cluster.operation_timeout_ms as u32,
    };

    // 重连集群（创建新的客户端连接）
    let clients = state.get_clients();
    if clients.get_admin(&cluster_id).is_some() {
        match clients.reconnect_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
            }
            Err(e) => {
                tracing::error!("Failed to reconnect cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    } else {
        match clients.with_added_cluster(&cluster_id, &config) {
            Ok(new_clients) => {
                state.set_clients(new_clients.clone());
            }
            Err(e) => {
                tracing::error!("Failed to add cluster '{}': {}", cluster_id, e);
                return;
            }
        }
    };

    // 同时重连连接池
    let _ = state.pools.reconnect(&cluster_id, &config, &state.config.pool).await;

    // 创建一个复用的 consumer，整个刷新过程复用同一个连接
    // 使用随机 group.id 避免多个刷新任务冲突
    let unique_suffix = format!("{}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(), rand::random::<u32>());
    let reusable_consumer = {
        let mut client_config = crate::kafka::create_client_config(&config);
        client_config.set("group.id", &format!("kafka-manager-cg-refresh-{}", unique_suffix));
        client_config.set("enable.auto.commit", "false");
        match client_config.create::<rdkafka::consumer::BaseConsumer>() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create reusable consumer for {}: {}", cluster_id, e);
                return;
            }
        }
    };

    let cg_manager = match KafkaConsumerGroupManager::with_consumer(&config, reusable_consumer) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to create ConsumerGroupManager for {}: {}", cluster_id, e);
            return;
        }
    };

    // 步骤 1: 一次 fetch_group_list 同时获取 group 名称和 topics
    let groups_with_topics = match tokio::task::spawn_blocking({
        let cg_manager = cg_manager.clone();
        move || cg_manager.list_consumer_groups_with_topics()
    })
    .await
    {
        Ok(Ok(groups)) => groups,
        Ok(Err(e)) => {
            tracing::warn!("Failed to list consumer groups for {}: {}", cluster_id, e);
            return;
        }
        Err(e) => {
            tracing::error!("Failed to list consumer groups for {}: {}", cluster_id, e);
            return;
        }
    };

    let group_names: Vec<String> = groups_with_topics.iter().map(|(name, _)| name.clone()).collect();

    // 步骤 2: 同步 group 名称到元数据表
    if let Err(e) = ConsumerGroupStore::sync_consumer_groups(state.db.inner(), &cluster_id, &group_names).await {
        tracing::error!("Failed to sync consumer groups for {}: {}", cluster_id, e);
        return;
    }

    // 步骤 3: 清理该集群下所有旧的 group-topic 关系
    if let Err(e) = ConsumerGroupStore::cleanup_all_cluster_topic_relations(state.db.inner(), &cluster_id).await {
        tracing::warn!("Failed to cleanup topic relations for {}: {}", cluster_id, e);
    }

    // 步骤 4: 将 group-topic 关系写入多对多表（使用步骤 1 已提取的 topics）
    let group_count = groups_with_topics.len();
    for (group, topics) in groups_with_topics {
        for topic in &topics {
            let _ = ConsumerGroupStore::upsert_topic_relation(
                state.db.inner(),
                &cluster_id,
                &group,
                topic,
            ).await;
        }
    }

    tracing::info!("Refreshed {} consumer groups for cluster {}", group_count, cluster_id);
}

/// 刷新所有集群的 Consumer Group 列表（在单个任务中，各集群并行刷新）
pub async fn refresh_all_consumer_groups(state: AppState) {
    use crate::db::cluster::ClusterStore;

    // 获取所有集群
    let clusters = match ClusterStore::list(state.db.inner()).await {
        Ok(clusters) => clusters,
        Err(e) => {
            tracing::error!("Failed to list clusters: {}", e);
            return;
        }
    };

    tracing::info!("Refreshing all {} clusters consumer groups in parallel", clusters.len());

    // 并行刷新所有集群（如果某个集群正在刷新则静默跳过）
    let mut tasks = Vec::with_capacity(clusters.len());
    for cluster in &clusters {
        let cluster_id = cluster.name.clone();
        // 检查是否正在刷新该集群，如果是则跳过
        let refreshing = {
            let refresh_state = state.refresh_state.lock().expect("refresh state poisoned");
            refresh_state.refreshing_clusters.contains(&cluster_id)
        };
        if refreshing {
            tracing::debug!("Skipping consumer group refresh for cluster {} (already refreshing)", cluster_id);
            continue;
        }
        let state = state.clone();
        tasks.push(tokio::spawn(async move {
            refresh_single_consumer_group(state, cluster_id).await;
        }));
    }

    // 等待所有任务完成
    for task in tasks {
        let _ = task.await;
    }

    tracing::info!("Completed refreshing all clusters consumer groups");
}

pub async fn handle_consumer_group_saved(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_cluster_id_param(&body)?;

    let groups = ConsumerGroupStore::list_by_cluster(state.db.inner(), &cluster_id).await?;
    let group_names: Vec<String> = groups.into_iter().map(|g| g.group_name).collect();

    Ok(serde_json::json!({ "groups": group_names }))
}

pub async fn handle_consumer_group_list(state: AppState, body: Value) -> Result<Value> {
    // Get pagination parameters (default: offset=0, limit=10000)
    let offset = body.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(10000) as usize;

    // Get cluster_ids array for multi-cluster selection
    let cluster_ids: Option<Vec<String>> = body.get("cluster_ids").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    });

    let cluster_id = body.get("cluster_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Determine which clusters to fetch consumer groups from
    let clusters_to_fetch: Vec<String> = if let Some(ref ids) = cluster_ids {
        if ids.is_empty() {
            // Empty array means "all clusters"
            crate::db::cluster::ClusterStore::list(state.db.inner()).await.ok().unwrap_or_default()
                .into_iter().map(|c| c.name).collect()
        } else {
            ids.clone()
        }
    } else if let Some(ref id) = cluster_id {
        vec![id.clone()]
    } else {
        // Default to all clusters
        crate::db::cluster::ClusterStore::list(state.db.inner()).await.ok().unwrap_or_default()
            .into_iter().map(|c| c.name).collect()
    };

    // Fetch all consumer groups from specified clusters
    let mut all_groups: Vec<serde_json::Value> = Vec::with_capacity(clusters_to_fetch.len() * 20);

    for cluster_name in &clusters_to_fetch {
        if let Ok(groups) = ConsumerGroupStore::list_by_cluster(
            state.db.inner(),
            cluster_name,
        ).await {
            for group in groups {
                let topics: Vec<String> = serde_json::from_str(&group.topics).unwrap_or_default();
                all_groups.push(serde_json::json!({
                    "id": group.id,
                    "cluster_id": group.cluster_id,
                    "group_name": group.group_name,
                    "topics": topics,
                    "fetched_at": group.fetched_at,
                }));
            }
        }
    }

    // Sort by cluster then by group name
    all_groups.sort_by(|a, b| {
        let cluster_cmp = a["cluster_id"].as_str().cmp(&b["cluster_id"].as_str());
        if cluster_cmp == std::cmp::Ordering::Equal {
            a["group_name"].as_str().cmp(&b["group_name"].as_str())
        } else {
            cluster_cmp
        }
    });

    // Apply pagination
    let total = all_groups.len();
    let end = (offset + limit).min(total);
    let paginated_groups = if offset < total {
        all_groups.into_iter().skip(offset).take(limit).collect()
    } else {
        Vec::new()
    };

    Ok(serde_json::json!({
        "groups": paginated_groups,
        "total": total,
        "offset": offset,
        "limit": limit,
        "has_more": end < total
    }))
}

pub async fn handle_consumer_group_list_by_topic(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_cluster_id_param(&body)?;
    let topic = super::get_string_param(&body, "topic")?;

    // 步骤 1: 从多对多关系表查询订阅了该 topic 的 consumer group names
    let group_names = match ConsumerGroupStore::list_group_names_by_topic(
        state.db.inner(),
        &cluster_id,
        &topic,
    )
    .await
    {
        Ok(groups) => groups,
        Err(e) => {
            tracing::error!("Failed to query consumer groups by topic: {}", e);
            Vec::new()
        }
    };

    tracing::info!("[handle_consumer_group_list_by_topic] found {} groups from DB for topic '{}'", group_names.len(), topic);

    // 步骤 2: 确保集群客户端已创建，并复用同一个 consumer 连接
    let config = super::ensure_cluster_client(&state, &cluster_id).await?;

    // 使用随机 group.id 避免冲突
    let unique_suffix = format!("{}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(), rand::random::<u32>());
    let reusable_consumer = {
        let mut client_config = crate::kafka::create_client_config(&config);
        client_config.set("group.id", &format!("kafka-manager-cg-list-by-topic-{}", unique_suffix));
        client_config.set("enable.auto.commit", "false");
        match client_config.create::<rdkafka::consumer::BaseConsumer>() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create reusable consumer for {}: {}", cluster_id, e);
                return Err(e.into());
            }
        }
    };

    let cg_manager = KafkaConsumerGroupManager::with_consumer(&config, reusable_consumer)?;

    // 步骤 3: 顺序获取每个 group 的指定 topic offset 信息（复用同一个 Kafka 连接）
    let mut all_offsets: Vec<(String, crate::kafka::consumer_group::PartitionOffsetDetail)> = Vec::new();

    for group_name in &group_names {
        match cg_manager.get_consumer_group_offsets_for_topic(group_name, &topic) {
            Ok(offsets) => {
                for offset in offsets {
                    all_offsets.push((group_name.clone(), offset));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get offsets for group '{}': {}", group_name, e);
            }
        }
    }

    // 步骤 4: 顺序获取所有分区的最后提交时间（复用同一个 Kafka 连接）
    let mut last_commit_times: std::collections::HashMap<(String, String, i32), Option<i64>> = std::collections::HashMap::new();

    for group_name in &group_names {
        let group_parts: Vec<(String, i32)> = all_offsets
            .iter()
            .filter(|(g, _)| g.as_str() == group_name.as_str())
            .map(|(_, o)| (o.topic.clone(), o.partition))
            .collect();

        if group_parts.is_empty() {
            continue;
        }

        match cg_manager.get_partitions_last_commit_time(group_name, &group_parts) {
            Ok(times) => {
                for (i, time) in times.iter().enumerate() {
                    if i < group_parts.len() {
                        let key = (group_name.clone(), group_parts[i].0.clone(), group_parts[i].1);
                        last_commit_times.insert(key, *time);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get last commit time for group '{}': {}", group_name, e);
            }
        }
    }

    // 步骤 5: 构建返回结果
    let mut topic_offsets: Vec<serde_json::Value> = Vec::new();

    for (group_name, offset) in all_offsets {
        let key = (group_name.clone(), offset.topic.clone(), offset.partition);
        let last_commit_time = last_commit_times.get(&key).copied().flatten();

        topic_offsets.push(serde_json::json!({
            "group": group_name,
            "topic": offset.topic,
            "partition": offset.partition,
            "start_offset": offset.start_offset,
            "end_offset": offset.end_offset,
            "committed_offset": offset.committed_offset,
            "lag": offset.lag,
            "last_commit_time": last_commit_time,
        }));
    }

    // 按 group name，然后 partition 排序
    topic_offsets.sort_by(|a, b| {
        let group_cmp = a["group"].as_str().cmp(&b["group"].as_str());
        if group_cmp != std::cmp::Ordering::Equal {
            return group_cmp;
        }
        a["partition"].as_i64().cmp(&b["partition"].as_i64())
    });

    tracing::info!("[handle_consumer_group_list_by_topic] returning {} offsets", topic_offsets.len());

    Ok(serde_json::json!({
        "offsets": topic_offsets,
    }))
}

pub async fn handle_consumer_group_get(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_cluster_id_param(&body)?;
    let group_name = super::get_string_param(&body, "group_name")?;

    // 从数据库获取 group 信息
    let _group = ConsumerGroupStore::get_by_name(state.db.inner(), &cluster_id, &group_name)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Consumer group '{}' not found", group_name)))?;

    // 从多对多关系表获取 topics
    let topics = ConsumerGroupStore::list_topics_by_group(state.db.inner(), &cluster_id, &group_name)
        .await
        .unwrap_or_default();

    // 确保集群客户端已创建
    let config = super::ensure_cluster_client(&state, &cluster_id).await?;

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;

    // 在阻塞线程中执行 Kafka 操作
    let group_info = tokio::task::spawn_blocking(move || {
        cg_manager.get_consumer_group_info(&group_name, &topics)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    Ok(serde_json::json!({
        "group_id": group_info.group_id,
        "cluster_id": cluster_id,
        "state": group_info.state,
        "topics": group_info.topics,
    }))
}

pub async fn handle_consumer_group_offsets(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_cluster_id_param(&body)?;
    let group_name = super::get_string_param(&body, "group_name")?;

    tracing::info!("[handle_consumer_group_offsets] cluster_id={}, group_name={}", cluster_id, group_name);

    // 确保集群客户端已创建
    let config = super::ensure_cluster_client(&state, &cluster_id).await?;

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;
    let group_name_clone = group_name.clone();
    let cg_manager_clone = cg_manager.clone();

    // 首先尝试从数据库获取保存的 topics
    let db_topics = ConsumerGroupStore::get_by_name(state.db.inner(), &cluster_id, &group_name)
        .await
        .ok()
        .flatten()
        .and_then(|g| serde_json::from_str::<Vec<String>>(&g.topics).ok())
        .unwrap_or_default();

    tracing::info!("[handle_consumer_group_offsets] DB topics: {:?}", db_topics);

    // 在阻塞线程中执行 Kafka 操作 - 使用数据库中的 topics
    let mut offsets = tokio::task::spawn_blocking(move || {
        if db_topics.is_empty() {
            // 如果数据库中没有 topics，尝试从 Kafka 自动获取
            cg_manager_clone.get_consumer_group_offsets_auto(&group_name_clone)
        } else {
            // 使用数据库中的 topics 获取 offsets
            cg_manager_clone.get_consumer_group_offsets(&group_name_clone, &db_topics)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    tracing::info!("[handle_consumer_group_offsets] Got {} offsets", offsets.len());

    // 批量获取所有分区的最后提交时间
    let partitions: Vec<(String, i32)> = offsets
        .iter()
        .map(|o| (o.topic.clone(), o.partition))
        .collect();

    if !partitions.is_empty() {
        match cg_manager.get_partitions_last_commit_time(&group_name, &partitions) {
            Ok(times) => {
                for (i, offset) in offsets.iter_mut().enumerate() {
                    if i < times.len() {
                        offset.last_commit_time = times[i];
                        tracing::info!("[handle_consumer_group_offsets] Got last_commit_time for {}/{}: {:?}",
                            offset.topic, offset.partition, times[i]);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[handle_consumer_group_offsets] Failed to get last_commit_time: {}", e);
            }
        }
    }

    let offsets_json: Vec<serde_json::Value> = offsets
        .into_iter()
        .map(|o| {
            serde_json::json!({
                "topic": o.topic,
                "partition": o.partition,
                "start_offset": o.start_offset,
                "end_offset": o.end_offset,
                "committed_offset": o.committed_offset,
                "lag": o.lag,
                "last_commit_time": o.last_commit_time,
            })
        })
        .collect();

    Ok(serde_json::json!({ "offsets": offsets_json }))
}

pub async fn handle_consumer_group_reset_offset(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_cluster_id_param(&body)?;
    let group_name = super::get_string_param(&body, "group_name")?;
    let topic = super::get_string_param(&body, "topic")?;
    let partition = super::get_i32_param(&body, "partition")?;
    let reset_to = super::get_string_param(&body, "reset_to")?;  // "earliest", "latest", "offset", or "timestamp"
    let timestamp = super::get_optional_i64_param(&body, "timestamp");
    let offset = super::get_optional_i64_param(&body, "offset");

    tracing::info!("Reset offset request: cluster={}, group={}, topic={}, partition={}, reset_to={}",
                   cluster_id, group_name, topic, partition, reset_to);

    // 确保集群客户端已创建
    let config = super::ensure_cluster_client(&state, &cluster_id).await?;
    tracing::info!("Got config for cluster: {}", cluster_id);

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;
    tracing::info!("Created ConsumerGroupManager");

    // 在阻塞线程中执行 Kafka 操作
    let new_offset = tokio::task::spawn_blocking(move || -> Result<i64> {
        tracing::info!("Spawn blocking task for reset offset");
        let result = match reset_to.as_str() {
            "earliest" => cg_manager.reset_consumer_group_offset_to_earliest(&group_name, &topic, partition),
            "latest" => cg_manager.reset_consumer_group_offset_to_latest(&group_name, &topic, partition),
            "offset" => {
                let off = offset.ok_or_else(|| AppError::BadRequest("offset is required when reset_to is 'offset'".to_string()))?;
                cg_manager.reset_consumer_group_offset(&group_name, &topic, partition, off)
            }
            "timestamp" => {
                let ts = timestamp.ok_or_else(|| AppError::BadRequest("timestamp is required when reset_to is 'timestamp'".to_string()))?;
                cg_manager.reset_consumer_group_offset_to_timestamp(&group_name, &topic, partition, ts)
            }
            _ => Err(AppError::BadRequest(format!("Invalid reset_to value: {}. Must be 'earliest', 'latest', 'offset', or 'timestamp'", reset_to))),
        };
        tracing::info!("Spawn blocking task completed: {:?}", result.as_ref().map(|o| o.to_string()).unwrap_or_else(|e| format!("Error: {}", e)));
        result
    })
    .await
    .map_err(|e| {
        tracing::error!("Task join error: {}", e);
        AppError::Internal(format!("Task failed: {}", e))
    })??;

    tracing::info!("Reset offset successful: new_offset={}", new_offset);

    Ok(serde_json::json!({
        "success": true,
        "new_offset": new_offset,
    }))
}

pub async fn handle_consumer_group_delete(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_cluster_id_param(&body)?;
    let group_name = super::get_string_param(&body, "group_name")?;

    // 确保集群客户端已创建
    let config = super::ensure_cluster_client(&state, &cluster_id).await?;

    let cg_manager = KafkaConsumerGroupManager::new(&config)?;

    // 克隆 group_name 用于闭包和后续使用
    let group_name_clone = group_name.clone();

    // 在阻塞线程中执行 Kafka 操作
    tokio::task::spawn_blocking(move || {
        cg_manager.delete_empty_consumer_group(&group_name_clone)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task failed: {}", e)))??;

    // 从数据库中删除
    ConsumerGroupStore::delete(state.db.inner(), &cluster_id, &group_name).await?;
    ConsumerGroupStore::delete_offsets(state.db.inner(), &cluster_id, &group_name).await?;

    Ok(serde_json::json!({ "success": true }))
}
