/// 全局用户设置路由

use crate::db::settings::SettingStore;
use crate::error::Result;
use crate::AppState;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_settings).put(update_setting))
        .route("/export", post(export_data))
        .route("/import", post(import_data))
}

#[derive(Debug, Deserialize)]
pub struct SettingsQuery {
    keys: Option<String>, // 逗号分隔的 key 列表
}

#[derive(Debug, Serialize)]
pub struct SettingValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub settings: Vec<SettingValue>,
}

/// 获取设置
async fn get_settings(
    State(state): State<Arc<AppState>>,
    query: Query<SettingsQuery>,
) -> Result<Json<SettingsResponse>> {
    let keys: Vec<&str> = query
        .keys
        .as_ref()
        .map(|s| s.split(',').map(|k| k.trim()).collect())
        .unwrap_or_default();

    let settings = if keys.is_empty() {
        let all: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM user_settings ORDER BY key"
        )
        .fetch_all(state.db.inner())
        .await?;
        all.into_iter()
            .map(|(k, v)| SettingValue { key: k, value: v })
            .collect()
    } else {
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(value) = SettingStore::get(state.db.inner(), key).await? {
                result.push(SettingValue {
                    key: key.to_string(),
                    value,
                });
            }
        }
        result
    };

    Ok(Json(SettingsResponse { settings }))
}

/// 更新设置
#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub key: String,
    pub value: String,
}

async fn update_setting(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<SettingValue>> {
    SettingStore::set(state.db.inner(), &req.key, &req.value).await?;

    Ok(Json(SettingValue {
        key: req.key,
        value: req.value,
    }))
}

// ==================== 导入导出功能 ====================

/// 导出数据响应
#[derive(Debug, Serialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub cluster_groups: Vec<ExportClusterGroup>,
    pub clusters: Vec<ExportCluster>,
    pub topics: Vec<ExportTopicMetadata>,
    pub favorites: Vec<ExportFavoriteGroup>,
    pub history: Vec<ExportTopicHistory>,
}

#[derive(Debug, Serialize)]
pub struct ExportClusterGroup {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Serialize)]
pub struct ExportCluster {
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i64,
    pub operation_timeout_ms: i64,
    pub group_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportTopicMetadata {
    pub cluster_name: String,
    pub topic_name: String,
    pub partition_count: i32,
    pub replication_factor: i32,
    pub config: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ExportFavoriteGroup {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub items: Vec<ExportFavoriteItem>,
}

#[derive(Debug, Serialize)]
pub struct ExportFavoriteItem {
    pub cluster_id: String,
    pub topic_name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Serialize)]
pub struct ExportTopicHistory {
    pub cluster_id: String,
    pub topic_name: String,
    pub viewed_at: String,
}

/// 导入数据请求
#[derive(Debug, Deserialize)]
pub struct ImportDataRequest {
    pub data: ImportData,
    /// 导入策略：skip(跳过已存在) 或 overwrite(覆盖已存在)
    #[serde(default = "default_import_strategy")]
    pub strategy: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportData {
    pub cluster_groups: Vec<ImportClusterGroup>,
    pub clusters: Vec<ImportCluster>,
    pub topics: Vec<ImportTopicMetadata>,
    pub favorites: Vec<ImportFavoriteGroup>,
    pub history: Vec<ImportTopicHistory>,
}

#[derive(Debug, Deserialize)]
pub struct ImportClusterGroup {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Deserialize)]
pub struct ImportCluster {
    pub name: String,
    pub brokers: String,
    pub request_timeout_ms: i64,
    pub operation_timeout_ms: i64,
    pub group_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportTopicMetadata {
    pub cluster_name: String,
    pub topic_name: String,
    pub partition_count: i32,
    pub replication_factor: i32,
    pub config: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportFavoriteGroup {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub items: Vec<ImportFavoriteItem>,
}

#[derive(Debug, Deserialize)]
pub struct ImportFavoriteItem {
    pub cluster_id: String,
    pub topic_name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct ImportTopicHistory {
    pub cluster_id: String,
    pub topic_name: String,
    pub viewed_at: String,
}

fn default_import_strategy() -> String {
    "skip".to_string()
}

/// 导入响应
#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub cluster_groups_imported: i32,
    pub cluster_groups_skipped: i32,
    pub clusters_imported: i32,
    pub clusters_skipped: i32,
    pub topics_imported: i32,
    pub topics_skipped: i32,
    pub favorites_imported: i32,
    pub favorites_skipped: i32,
    pub history_imported: i32,
    pub history_skipped: i32,
}

/// 导出数据
pub(crate) async fn export_data(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ExportData>> {
    use crate::db::cluster::ClusterStore;
    use crate::db::cluster_group::ClusterGroupStore;
    use crate::db::favorite::get_all_favorites_with_groups;
    use crate::db::topic::TopicStore;
    use crate::db::topic_history::get_history_list;

    let cluster_groups_db = ClusterGroupStore::list(state.db.inner()).await?;
    let cluster_groups: Vec<ExportClusterGroup> = cluster_groups_db
        .iter()
        .map(|g| ExportClusterGroup {
            name: g.name.clone(),
            description: g.description.clone(),
            sort_order: g.sort_order,
        })
        .collect();

    let clusters_db = ClusterStore::list(state.db.inner()).await?;
    let clusters: Vec<ExportCluster> = clusters_db
        .into_iter()
        .map(|c| {
            let group_name = c.group_id.and_then(|gid| {
                cluster_groups_db.iter().find(|g| g.id == gid).map(|g| g.name.clone())
            });
            ExportCluster {
                name: c.name,
                brokers: c.brokers,
                request_timeout_ms: c.request_timeout_ms,
                operation_timeout_ms: c.operation_timeout_ms,
                group_name,
            }
        })
        .collect();

    let topics_db = TopicStore::list_all(state.db.inner()).await?;
    let topics: Vec<ExportTopicMetadata> = topics_db
        .into_iter()
        .map(|t| {
            let config: std::collections::HashMap<String, String> =
                serde_json::from_str(&t.config_json).unwrap_or_default();
            ExportTopicMetadata {
                cluster_name: t.cluster_id,
                topic_name: t.topic_name,
                partition_count: t.partition_count,
                replication_factor: t.replication_factor,
                config,
            }
        })
        .collect();

    let favorites_with_groups = get_all_favorites_with_groups(&state.db).await?;
    let favorites: Vec<ExportFavoriteGroup> = favorites_with_groups
        .into_iter()
        .map(|g| ExportFavoriteGroup {
            name: g.name,
            description: g.description,
            sort_order: g.sort_order,
            items: g.items
                .into_iter()
                .map(|item| ExportFavoriteItem {
                    cluster_id: item.cluster_id,
                    topic_name: item.topic_name,
                    description: item.description,
                    sort_order: item.sort_order,
                })
                .collect(),
        })
        .collect();

    let histories = get_history_list(&state.db, None, None).await?;
    let history: Vec<ExportTopicHistory> = histories
        .into_iter()
        .map(|h| ExportTopicHistory {
            cluster_id: h.cluster_id,
            topic_name: h.topic_name,
            viewed_at: h.viewed_at,
        })
        .collect();

    let export_data = ExportData {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        cluster_groups,
        clusters,
        topics,
        favorites,
        history,
    };

    Ok(Json(export_data))
}

/// 核心导入逻辑（被 HTTP handler 和 Tauri command 共用）
pub async fn import_data_from_request(
    pool: &crate::db::DbPool,
    req: &ImportDataRequest,
) -> Result<ImportResult> {
    use crate::db::cluster::{ClusterStore, CreateClusterRequest};
    use crate::db::cluster_group::{ClusterGroupStore, CreateClusterGroupRequest};
    use crate::db::favorite::{
        create_group, create_favorite, get_all_groups,
        CreateGroupRequest, CreateFavoriteRequest,
    };
    use crate::db::topic::TopicStore;
    use crate::db::topic_history::import_history;

    let mut cluster_groups_imported = 0;
    let mut cluster_groups_skipped = 0;
    let mut clusters_imported = 0;
    let mut clusters_skipped = 0;
    let mut topics_imported = 0;
    let mut topics_skipped = 0;
    let mut favorites_imported = 0;
    let mut favorites_skipped = 0;
    let mut history_imported = 0;
    let mut history_skipped = 0;

    // 1. 导入集群分组
    for group in &req.data.cluster_groups {
        match ClusterGroupStore::get_by_name(pool.inner(), &group.name).await {
            Ok(Some(_)) => {
                cluster_groups_skipped += 1;
            }
            Ok(None) | Err(_) => {
                let create_req = CreateClusterGroupRequest {
                    name: group.name.clone(),
                    description: group.description.clone(),
                    sort_order: group.sort_order,
                };
                if ClusterGroupStore::create(pool.inner(), &create_req).await.is_ok() {
                    cluster_groups_imported += 1;
                } else {
                    cluster_groups_skipped += 1;
                }
            }
        }
    }

    // 2. 导入集群
    let all_groups = ClusterGroupStore::list(pool.inner()).await?;
    let group_name_to_id: std::collections::HashMap<String, i64> = all_groups
        .iter()
        .map(|g| (g.name.clone(), g.id))
        .collect();

    for cluster in &req.data.clusters {
        match ClusterStore::get_by_name(pool.inner(), &cluster.name).await {
            Ok(Some(_)) => {
                clusters_skipped += 1;
            }
            Ok(None) | Err(_) => {
                let group_id = cluster.group_name.as_ref().and_then(|name| group_name_to_id.get(name).copied());
                let create_req = CreateClusterRequest {
                    name: cluster.name.clone(),
                    brokers: cluster.brokers.clone(),
                    request_timeout_ms: cluster.request_timeout_ms,
                    operation_timeout_ms: cluster.operation_timeout_ms,
                    group_id,
                };
                if ClusterStore::create(pool.inner(), &create_req).await.is_ok() {
                    clusters_imported += 1;
                } else {
                    clusters_skipped += 1;
                }
            }
        }
    }

    // 3. 导入 Topic 元数据
    for topic in &req.data.topics {
        match TopicStore::upsert(
            pool.inner(),
            &topic.cluster_name,
            &topic.topic_name,
            topic.partition_count,
            topic.replication_factor,
            &topic.config,
        ).await {
            Ok(_) => topics_imported += 1,
            Err(_) => topics_skipped += 1,
        }
    }

    // 4. 导入收藏分组和收藏项
    let existing_groups = get_all_groups(pool).await?;
    let mut group_name_to_id: std::collections::HashMap<String, i64> = existing_groups
        .iter()
        .map(|g| (g.name.clone(), g.id))
        .collect();
    let mut max_sort_order: i32 = existing_groups.iter().map(|g| g.sort_order).max().unwrap_or(0);

    for fav_group in &req.data.favorites {
        let group_id = match group_name_to_id.get(&fav_group.name) {
            Some(&id) => {
                favorites_skipped += fav_group.items.len() as i32;
                id
            }
            None => {
                max_sort_order += 1;
                let create_group_req = CreateGroupRequest {
                    name: fav_group.name.clone(),
                    description: fav_group.description.clone(),
                    sort_order: Some(max_sort_order),
                };
                match create_group(pool, &create_group_req).await {
                    Ok(group) => {
                        group_name_to_id.insert(fav_group.name.clone(), group.id);
                        group.id
                    }
                    Err(_) => {
                        favorites_skipped += fav_group.items.len() as i32;
                        continue;
                    }
                }
            }
        };

        for item in &fav_group.items {
            let create_item_req = CreateFavoriteRequest {
                group_id,
                cluster_id: item.cluster_id.clone(),
                topic_name: item.topic_name.clone(),
                description: item.description.clone(),
                sort_order: Some(item.sort_order),
            };
            if create_favorite(pool, &create_item_req).await.is_ok() {
                favorites_imported += 1;
            } else {
                favorites_skipped += 1;
            }
        }
    }

    // 5. 导入历史记录
    for history in &req.data.history {
        if req.strategy == "overwrite" {
            if import_history(pool, &history.cluster_id, &history.topic_name, &history.viewed_at).await.is_ok() {
                history_imported += 1;
            } else {
                history_skipped += 1;
            }
        } else {
            use crate::db::topic_history::is_history_exists;
            match is_history_exists(pool, &history.cluster_id, &history.topic_name).await {
                Ok(true) => {
                    history_skipped += 1;
                }
                Ok(false) | Err(_) => {
                    if import_history(pool, &history.cluster_id, &history.topic_name, &history.viewed_at).await.is_ok() {
                        history_imported += 1;
                    } else {
                        history_skipped += 1;
                    }
                }
            }
        }
    }

    Ok(ImportResult {
        cluster_groups_imported,
        cluster_groups_skipped,
        clusters_imported,
        clusters_skipped,
        topics_imported,
        topics_skipped,
        favorites_imported,
        favorites_skipped,
        history_imported,
        history_skipped,
    })
}

/// HTTP handler：导入数据
pub(crate) async fn import_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportDataRequest>,
) -> Result<Json<ImportResult>> {
    let result = import_data_from_request(&state.db, &req).await?;
    Ok(Json(result))
}
