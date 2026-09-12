//! API 数据模型（与后端 JSON 响应对应）

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Cluster {
    pub id: i64,
    pub name: String,
    pub brokers: String,
    #[serde(default)]
    pub group_id: Option<i64>,
    #[serde(default)]
    pub request_timeout_ms: Option<i64>,
    #[serde(default)]
    pub operation_timeout_ms: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClusterGroup {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Clone, Debug, Default)]
pub struct ClusterStatus {
    /// None=未知，Some(true)=健康
    pub health: Option<bool>,
    pub latency_ms: Option<u64>,
    pub connections: usize,
    pub pools: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TopicWithCluster {
    pub name: String,
    pub cluster: String,
}

#[derive(Clone, Debug)]
pub struct TopicItem {
    pub name: String,
    pub cluster: String,
}

#[derive(Clone, Debug)]
pub struct ConsumerGroupItem {
    pub name: String,
    pub cluster: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FavoriteItem {
    pub id: i64,
    pub group_id: i64,
    pub cluster_id: String,
    pub cluster_name: String,
    pub topic_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FavoriteGroup {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub items: Vec<FavoriteItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TopicHistoryItem {
    pub id: i64,
    pub cluster_id: String,
    pub cluster_name: String,
    pub topic_name: String,
    #[serde(default)]
    pub last_accessed_at: Option<String>,
    #[serde(default)]
    pub access_count: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SentMessageItem {
    pub id: i64,
    pub cluster_id: String,
    pub cluster_name: String,
    pub topic_name: String,
    pub partition: i32,
    #[serde(default)]
    pub message_key: Option<String>,
    #[serde(default)]
    pub message_value: Option<String>,
    #[serde(default)]
    pub headers: Option<serde_json::Value>,
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub sent_at: Option<String>,
}

/// 消息记录（紧凑格式，对齐 Vue 的 {p,o,k,v,ts,uid,vt}）
#[derive(Clone, Debug)]
pub struct MessageRecord {
    pub p: i32,
    pub o: i64,
    pub k: String,
    pub v: String,
    pub ts: Option<i64>,
    pub uid: String,
    pub vt: bool,
}

impl MessageRecord {
    pub fn from_json(v: &serde_json::Value) -> Option<Self> {
        let p = v.get("partition")?.as_i64()? as i32;
        let o = v.get("offset")?.as_i64()?;
        Some(Self {
            p,
            o,
            k: v.get("key").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            v: v.get("value").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            ts: v.get("timestamp").and_then(|x| x.as_i64()),
            uid: format!("{}-{}", p, o),
            vt: v.get("value_truncated").and_then(|x| x.as_bool()).unwrap_or(false),
        })
    }
}
