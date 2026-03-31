/// Schema Registry 数据模型

use serde::{Deserialize, Serialize};

/// Schema 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SchemaType {
    Avro,
    Protobuf,
    Json,
}

impl std::fmt::Display for SchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaType::Avro => write!(f, "AVRO"),
            SchemaType::Protobuf => write!(f, "PROTOBUF"),
            SchemaType::Json => write!(f, "JSON"),
        }
    }
}

impl SchemaType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "AVRO" => Some(SchemaType::Avro),
            "PROTOBUF" => Some(SchemaType::Protobuf),
            "JSON" => Some(SchemaType::Json),
            _ => None,
        }
    }
}

/// 兼容性级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityLevel {
    Backward,
    BackwardTransitive,
    Forward,
    ForwardTransitive,
    Full,
    FullTransitive,
    None,
}

impl std::fmt::Display for CompatibilityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompatibilityLevel::Backward => write!(f, "BACKWARD"),
            CompatibilityLevel::BackwardTransitive => write!(f, "BACKWARD_TRANSITIVE"),
            CompatibilityLevel::Forward => write!(f, "FORWARD"),
            CompatibilityLevel::ForwardTransitive => write!(f, "FORWARD_TRANSITIVE"),
            CompatibilityLevel::Full => write!(f, "FULL"),
            CompatibilityLevel::FullTransitive => write!(f, "FULL_TRANSITIVE"),
            CompatibilityLevel::None => write!(f, "NONE"),
        }
    }
}

impl CompatibilityLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "BACKWARD" => Some(CompatibilityLevel::Backward),
            "BACKWARD_TRANSITIVE" => Some(CompatibilityLevel::BackwardTransitive),
            "FORWARD" => Some(CompatibilityLevel::Forward),
            "FORWARD_TRANSITIVE" => Some(CompatibilityLevel::ForwardTransitive),
            "FULL" => Some(CompatibilityLevel::Full),
            "FULL_TRANSITIVE" => Some(CompatibilityLevel::FullTransitive),
            "NONE" => Some(CompatibilityLevel::None),
            _ => None,
        }
    }
}

/// Schema 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub subject: String,
    pub version: i32,
    pub schema_type: SchemaType,
    pub schema_json: String,
    pub compatibility_level: Option<CompatibilityLevel>,
    pub id: Option<i64>,  // Schema Registry 返回的全局 ID
}

/// Schema ID（注册后返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaId {
    pub id: i64,
    pub subject: String,
    pub version: i32,
}

/// 兼容性测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResult {
    pub compatible: bool,
    pub errors: Vec<String>,
    pub messages: Vec<String>,
}

/// Schema Registry 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryConfigInfo {
    pub id: i64,
    pub cluster_id: String,
    pub registry_url: String,
    pub username: Option<String>,
    pub has_password: bool,  // 只返回是否有密码，不返回密码本身
    pub created_at: String,
    pub updated_at: String,
}

/// Schema 摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSummary {
    pub subject: String,
    pub latest_version: i32,
    pub schema_type: SchemaType,
    pub compatibility_level: Option<CompatibilityLevel>,
    pub version_count: i32,
}

/// 注册 Schema 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSchemaRequest {
    pub subject: String,
    pub schema_json: String,
    pub schema_type: SchemaType,
    pub compatibility_level: Option<CompatibilityLevel>,
}

/// 测试兼容性请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCompatibilityRequest {
    pub subject: String,
    pub schema_json: String,
    pub version: i32,  // 使用 0 表示最新版本
}
