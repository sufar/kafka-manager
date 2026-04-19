/// Cluster handlers for unified API
use crate::db::cluster::{ClusterStore, CreateClusterRequest, UpdateClusterRequest};
use crate::db::topic::TopicStore;
use crate::error::{AppError, Result};
use crate::AppState;
use serde_json::Value;

pub async fn handle_cluster_list(state: AppState) -> Result<Value> {
    let clusters = ClusterStore::list(state.db.inner()).await?;

    let cluster_infos: Vec<Value> = clusters
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "brokers": c.brokers,
                "request_timeout_ms": c.request_timeout_ms,
                "operation_timeout_ms": c.operation_timeout_ms,
                "group_id": c.group_id,
                "created_at": c.created_at,
                "updated_at": c.updated_at,
            })
        })
        .collect();

    Ok(serde_json::json!(cluster_infos))
}

pub async fn handle_cluster_get(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let cluster = ClusterStore::get(state.db.inner(), id).await?;

    Ok(serde_json::json!({
        "id": cluster.id,
        "name": cluster.name,
        "brokers": cluster.brokers,
        "request_timeout_ms": cluster.request_timeout_ms,
        "operation_timeout_ms": cluster.operation_timeout_ms,
        "group_id": cluster.group_id,
        "created_at": cluster.created_at,
        "updated_at": cluster.updated_at,
    }))
}

pub async fn handle_cluster_create(state: AppState, body: Value) -> Result<Value> {
    let name = super::get_string_param(&body, "name")?;
    let brokers = super::get_string_param(&body, "brokers")?;
    let request_timeout_ms = super::get_optional_i64_param(&body, "request_timeout_ms").unwrap_or(5000);
    let operation_timeout_ms = super::get_optional_i64_param(&body, "operation_timeout_ms").unwrap_or(5000);
    let group_id = super::get_optional_i64_param(&body, "group_id");

    if let Some(_existing) = ClusterStore::get_by_name(state.db.inner(), &name).await? {
        return Err(AppError::BadRequest(format!("Cluster name '{}' already exists", name)));
    }

    let req = CreateClusterRequest {
        name: name.clone(),
        brokers,
        request_timeout_ms,
        operation_timeout_ms,
        group_id,
    };

    let cluster = ClusterStore::create(state.db.inner(), &req).await?;
    reload_clients(&state).await?;

    Ok(serde_json::json!({
        "id": cluster.id,
        "name": cluster.name,
        "brokers": cluster.brokers,
        "request_timeout_ms": cluster.request_timeout_ms,
        "operation_timeout_ms": cluster.operation_timeout_ms,
        "group_id": cluster.group_id,
        "created_at": cluster.created_at,
        "updated_at": cluster.updated_at,
    }))
}

pub async fn handle_cluster_update(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let name = super::get_optional_string_param(&body, "name");
    let brokers = super::get_optional_string_param(&body, "brokers");
    let request_timeout_ms = super::get_optional_i64_param(&body, "request_timeout_ms");
    let operation_timeout_ms = super::get_optional_i64_param(&body, "operation_timeout_ms");
    let group_id = super::get_nullable_i64_param(&body, "group_id");

    let old_cluster = ClusterStore::get(state.db.inner(), id).await?;

    if let Some(ref new_name) = name {
        if new_name != &old_cluster.name {
            if let Some(_existing) = ClusterStore::get_by_name(state.db.inner(), new_name).await? {
                return Err(AppError::BadRequest(format!("Cluster name '{}' already exists", new_name)));
            }
        }
    }

    let req = UpdateClusterRequest {
        name,
        brokers,
        request_timeout_ms,
        operation_timeout_ms,
        group_id,
    };

    let cluster = ClusterStore::update(state.db.inner(), id, &req).await?;
    reload_clients(&state).await?;

    Ok(serde_json::json!({
        "id": cluster.id,
        "name": cluster.name,
        "brokers": cluster.brokers,
        "request_timeout_ms": cluster.request_timeout_ms,
        "operation_timeout_ms": cluster.operation_timeout_ms,
        "group_id": cluster.group_id,
        "created_at": cluster.created_at,
        "updated_at": cluster.updated_at,
    }))
}

pub async fn handle_cluster_delete(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;

    let cluster = ClusterStore::get(state.db.inner(), id).await?;
    let cluster_name = cluster.name.clone();

    // Delete all related data in dependency order
    sqlx::query("DELETE FROM favorite_items WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;
    sqlx::query("DELETE FROM topic_history WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;
    sqlx::query("DELETE FROM sent_messages WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;
    sqlx::query("DELETE FROM consumer_group_offsets WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;
    sqlx::query("DELETE FROM consumer_group_metadata WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;
    sqlx::query("DELETE FROM schema_registry_configs WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;
    sqlx::query("DELETE FROM schemas WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;
    sqlx::query("DELETE FROM resource_tags WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;

    ClusterStore::delete(state.db.inner(), id).await?;
    reload_clients(&state).await?;
    sqlx::query("DELETE FROM topic_metadata WHERE cluster_id = ?")
        .bind(&cluster_name).execute(state.db.inner()).await?;

    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_cluster_test(state: AppState, body: Value) -> Result<Value> {
    let id = super::get_i64_param(&body, "id")?;
    let success = ClusterStore::test_connection(state.db.inner(), id).await?;
    Ok(serde_json::json!({ "success": success }))
}

pub async fn handle_cluster_test_with_config(_state: AppState, body: Value) -> Result<Value> {
    let brokers = super::get_string_param(&body, "brokers")?;
    let request_timeout_ms = super::get_optional_i64_param(&body, "request_timeout_ms").unwrap_or(5000);
    let operation_timeout_ms = super::get_optional_i64_param(&body, "operation_timeout_ms").unwrap_or(5000);

    use crate::config::KafkaConfig;
    use crate::kafka::KafkaAdmin;

    let config = KafkaConfig {
        brokers,
        request_timeout_ms: request_timeout_ms as u32,
        operation_timeout_ms: operation_timeout_ms as u32,
    };

    match KafkaAdmin::new(&config) {
        Ok(_) => Ok(serde_json::json!({ "success": true })),
        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

pub async fn handle_cluster_stats(state: AppState, body: Value) -> Result<Value> {
    let cluster_id = super::get_string_param(&body, "cluster_id")?;

    let clients = state.get_clients();
    let admin = clients
        .get_admin(&cluster_id)
        .ok_or_else(|| AppError::NotConnected(format!("Cluster '{}' is not connected", cluster_id)))?;

    let timeout_duration = std::time::Duration::from_secs(5);
    let admin = admin.clone();
    let result = tokio::time::timeout(timeout_duration, tokio::task::spawn_blocking(move || -> Result<(crate::kafka::admin::ClusterInfo, Vec<String>, i32, i32, std::collections::HashMap<i32, i32>, std::collections::HashMap<i32, i32>)> {
        let cluster_info = admin.get_cluster_info()?;
        let topics = admin.list_topics()?;
        let mut partition_count = 0;
        let mut under_replicated = 0;
        let estimated_brokers = 4;
        let mut broker_leader_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::with_capacity(estimated_brokers);
        let mut broker_replica_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::with_capacity(estimated_brokers);

        for topic in &topics {
            let topic_info = admin.get_topic_info(topic)?;
            for partition in &topic_info.partitions {
                partition_count += 1;
                if partition.isr.len() < partition.replicas.len() {
                    under_replicated += 1;
                }
                let leader = partition.leader;
                *broker_leader_counts.entry(leader).or_insert(0) += 1;
                for replica in &partition.replicas {
                    *broker_replica_counts.entry(*replica).or_insert(0) += 1;
                }
            }
        }

        Ok((cluster_info, topics, partition_count, under_replicated, broker_leader_counts, broker_replica_counts))
    })).await;

    let (cluster_info, topics, partition_count, under_replicated, broker_leader_counts, broker_replica_counts) = match result {
        Ok(Ok(Ok(data))) => data,
        Ok(Ok(Err(e))) => {
            tracing::warn!("Failed to get cluster stats for '{}': {}", cluster_id, e);
            return Ok(serde_json::json!({
                "cluster_id": cluster_id, "broker_count": 0, "controller_id": null,
                "topic_count": 0, "partition_count": 0, "under_replicated_partitions": 0,
                "consumer_group_count": 0, "total_lag": 0, "broker_stats": [],
                "error": format!("Failed to connect: {}", e),
            }));
        }
        Ok(Err(e)) => {
            tracing::warn!("Task failed for cluster '{}': {}", cluster_id, e);
            return Ok(serde_json::json!({
                "cluster_id": cluster_id, "broker_count": 0, "controller_id": null,
                "topic_count": 0, "partition_count": 0, "under_replicated_partitions": 0,
                "consumer_group_count": 0, "total_lag": 0, "broker_stats": [],
                "error": format!("Task failed: {}", e),
            }));
        }
        Err(_) => {
            tracing::warn!("Timeout getting cluster stats for '{}'", cluster_id);
            return Ok(serde_json::json!({
                "cluster_id": cluster_id, "broker_count": 0, "controller_id": null,
                "topic_count": 0, "partition_count": 0, "under_replicated_partitions": 0,
                "consumer_group_count": 0, "total_lag": 0, "broker_stats": [],
                "error": "Timeout connecting to cluster",
            }));
        }
    };

    let broker_stats: Vec<Value> = cluster_info.brokers.iter().map(|b| {
        serde_json::json!({
            "id": b.id, "host": b.host.clone(), "port": b.port,
            "is_controller": cluster_info.controller_id == Some(b.id),
            "leader_partitions": *broker_leader_counts.get(&b.id).unwrap_or(&0),
            "replica_partitions": *broker_replica_counts.get(&b.id).unwrap_or(&0),
        })
    }).collect();

    Ok(serde_json::json!({
        "cluster_id": cluster_id,
        "broker_count": cluster_info.brokers.len(),
        "controller_id": cluster_info.controller_id,
        "topic_count": topics.len(),
        "partition_count": partition_count,
        "under_replicated_partitions": under_replicated,
        "consumer_group_count": 0,
        "total_lag": 0,
        "broker_stats": broker_stats,
    }))
}

pub async fn reload_clients(state: &AppState) -> Result<()> {
    use crate::config::KafkaConfig;

    let clusters = ClusterStore::list(state.db.inner()).await?;

    let mut new_clusters = std::collections::HashMap::with_capacity(clusters.len());
    for cluster in &clusters {
        new_clusters.insert(
            cluster.name.clone(),
            KafkaConfig {
                brokers: cluster.brokers.clone(),
                request_timeout_ms: cluster.request_timeout_ms as u32,
                operation_timeout_ms: cluster.operation_timeout_ms as u32,
            },
        );
    }

    let new_clients = crate::kafka::KafkaClients::new(&new_clusters)?;
    let clients_for_sync = new_clients.clone();
    state.set_clients(new_clients);

    let db_pool = state.db.clone();
    let cluster_names: Vec<String> = clusters.iter().map(|c| c.name.clone()).collect();

    tokio::spawn(async move {
        for cluster_name in cluster_names {
            let db = db_pool.inner();
            if let Some(admin) = clients_for_sync.get_admin(&cluster_name) {
                let admin_clone = admin.clone();
                match tokio::task::spawn_blocking(move || admin_clone.list_topics()).await {
                    Ok(Ok(topics)) => {
                        let _ = TopicStore::sync_topics(db, &cluster_name, &topics).await;
                        tracing::info!("Synced {} topics for cluster '{}'", topics.len(), cluster_name);
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("Failed to list topics for cluster '{}': {}", cluster_name, e);
                    }
                    Err(e) => {
                        tracing::warn!("Topic list task panicked for cluster '{}': {}", cluster_name, e);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    tracing::info!("Reloaded Kafka clients (topic sync in background)");
    Ok(())
}
