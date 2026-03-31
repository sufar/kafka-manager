/// Schema Registry HTTP 客户端
/// 与 Confluent Schema Registry API 交互

use crate::error::{AppError, Result};
use crate::models::schema_registry::{
    CompatibilityLevel, CompatibilityResult, SchemaId, SchemaInfo, SchemaType,
};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

/// Schema Registry 客户端
pub struct SchemaRegistryClient {
    base_url: String,
    client: Client,
    auth_header: Option<String>,
}

/// Schema Registry API 响应结构
#[derive(Debug, Deserialize)]
struct SubjectListResponse {
    #[serde(default)]
    subjects: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VersionListResponse {
    #[serde(default)]
    versions: Vec<i32>,
}

#[derive(Debug, Deserialize)]
struct SchemaResponse {
    subject: String,
    version: i32,
    schema: String,
    #[serde(default)]
    schemaType: Option<String>,  // AVRO, PROTOBUF, JSON
    #[serde(default)]
    references: Option<Vec<SchemaReference>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SchemaReference {
    pub name: String,
    pub subject: String,
    pub version: i32,
}

#[derive(Debug, Deserialize)]
struct RegisterSchemaResponse {
    id: i64,
    subject: String,
    version: i32,
}

#[derive(Debug, Deserialize)]
struct CompatibilityResponse {
    is_compatible: bool,
}

#[derive(Debug, Serialize)]
struct RegisterSchemaBody {
    schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schemaType: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    references: Option<Vec<SchemaReference>>,
}

#[derive(Debug, Serialize)]
struct CompatibilityBody {
    #[serde(rename = "type")]
    schema_type: String,
    schema: String,
    version: i32,
}

impl SchemaRegistryClient {
    /// 创建新的 Schema Registry 客户端
    pub fn new(
        base_url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self> {
        let client_builder = Client::builder();

        let client = client_builder
            .build()
            .map_err(|e| AppError::Http(e))?;

        // 移除末尾的斜杠
        let base_url = base_url.trim_end_matches('/').to_string();

        // 创建基本认证 header
        let auth_header = match (username, password) {
            (Some(user), Some(pass)) => {
                let credentials = format!("{}:{}", user, pass);
                let encoded = base64::encode(&credentials);
                Some(format!("Basic {}", encoded))
            }
            _ => None,
        };

        Ok(Self {
            base_url,
            client,
            auth_header,
        })
    }

    /// 创建带认证的请求
    fn request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.request(method, url);
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        req
    }

    /// 获取所有 subjects
    pub async fn get_subjects(&self) -> Result<Vec<String>> {
        let url = format!("{}/subjects", self.base_url);
        let response = self.request(reqwest::Method::GET, &url).send().await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(error));
        }

        let subjects: Vec<String> = response.json().await?;
        Ok(subjects)
    }

    /// 获取指定 subject 的所有版本
    pub async fn get_versions(&self, subject: &str) -> Result<Vec<i32>> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);
        let response = self.request(reqwest::Method::GET, &url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let versions: Vec<i32> = response.json().await?;
                Ok(versions)
            }
            StatusCode::NOT_FOUND => {
                Err(AppError::NotFound(format!("Subject '{}' not found", subject)))
            }
            _ => {
                let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(AppError::Internal(error))
            }
        }
    }

    /// 获取指定版本的 schema
    pub async fn get_schema(&self, subject: &str, version: i32) -> Result<SchemaInfo> {
        let url = format!("{}/subjects/{}/versions/{}", self.base_url, subject, version);
        let response = self.request(reqwest::Method::GET, &url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let schema_resp: SchemaResponse = response.json().await?;

                // 解析 schema 类型
                let schema_type = schema_resp
                    .schemaType
                    .as_deref()
                    .and_then(SchemaType::from_str)
                    .unwrap_or(SchemaType::Avro);

                Ok(SchemaInfo {
                    subject: schema_resp.subject,
                    version: schema_resp.version,
                    schema_type,
                    schema_json: schema_resp.schema,
                    compatibility_level: None,  // 需要单独获取
                    id: None,
                })
            }
            StatusCode::NOT_FOUND => {
                Err(AppError::NotFound(format!(
                    "Schema '{}' version {} not found",
                    subject, version
                )))
            }
            _ => {
                let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(AppError::Internal(error))
            }
        }
    }

    /// 获取最新版本的 schema
    pub async fn get_latest_schema(&self, subject: &str) -> Result<SchemaInfo> {
        self.get_schema(subject, -1).await  // -1 表示最新版本
    }

    /// 注册新的 schema
    pub async fn register_schema(
        &self,
        subject: &str,
        schema_json: &str,
        schema_type: SchemaType,
    ) -> Result<SchemaId> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);

        let body = RegisterSchemaBody {
            schema: schema_json.to_string(),
            schemaType: Some(schema_type.to_string()),
            references: None,
        };

        let response = self.request(reqwest::Method::POST, &url).json(&body).send().await?;

        match response.status() {
            StatusCode::OK => {
                let result: RegisterSchemaResponse = response.json().await?;
                Ok(SchemaId {
                    id: result.id,
                    subject: result.subject,
                    version: result.version,
                })
            }
            StatusCode::CONFLICT => {
                // Schema 已存在，返回现有版本
                let result: RegisterSchemaResponse = response.json().await?;
                Ok(SchemaId {
                    id: result.id,
                    subject: result.subject,
                    version: result.version,
                })
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                let error = response.text().await.unwrap_or_else(|_| "Invalid schema".to_string());
                Err(AppError::BadRequest(format!("Invalid schema: {}", error)))
            }
            _ => {
                let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(AppError::Internal(error))
            }
        }
    }

    /// 测试 schema 兼容性
    pub async fn test_compatibility(
        &self,
        subject: &str,
        schema_json: &str,
        version: i32,
    ) -> Result<CompatibilityResult> {
        let url = format!(
            "{}/compatibility/subjects/{}/versions/{}",
            self.base_url, subject, version
        );

        let body = CompatibilityBody {
            schema_type: "AVRO".to_string(),
            schema: schema_json.to_string(),
            version,
        };

        let response = self.request(reqwest::Method::POST, &url).json(&body).send().await?;

        match response.status() {
            StatusCode::OK => {
                let result: CompatibilityResponse = response.json().await?;
                Ok(CompatibilityResult {
                    compatible: result.is_compatible,
                    errors: vec![],
                    messages: if result.is_compatible {
                        vec!["Schema is compatible".to_string()]
                    } else {
                        vec!["Schema is not compatible".to_string()]
                    },
                })
            }
            _ => {
                // 返回不兼容的结果
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Ok(CompatibilityResult {
                    compatible: false,
                    errors: vec![error_text],
                    messages: vec![],
                })
            }
        }
    }

    /// 获取兼容性级别
    pub async fn get_compatibility_level(&self, subject: &str) -> Result<CompatibilityLevel> {
        // 先尝试获取 subject 级别的配置
        let url = format!("{}/config/{}", self.base_url, subject);
        let response = self.request(reqwest::Method::GET, &url).send().await?;

        if response.status().is_success() {
            #[derive(Debug, Deserialize)]
            struct ConfigResponse {
                compatibilityLevel: String,
            }
            let result: ConfigResponse = response.json().await?;
            return CompatibilityLevel::from_str(&result.compatibilityLevel)
                .ok_or_else(|| AppError::Internal("Invalid compatibility level".to_string()));
        }

        // 如果 subject 级别没有配置，获取全局配置
        let url = format!("{}/config", self.base_url);
        let response = self.request(reqwest::Method::GET, &url).send().await?;

        match response.status() {
            StatusCode::OK => {
                #[derive(Debug, Deserialize)]
                struct GlobalConfigResponse {
                    compatibilityLevel: String,
                }
                let result: GlobalConfigResponse = response.json().await?;
                CompatibilityLevel::from_str(&result.compatibilityLevel)
                    .ok_or_else(|| AppError::Internal("Invalid compatibility level".to_string()))
            }
            _ => Ok(CompatibilityLevel::Backward),  // 默认兼容性级别
        }
    }

    /// 设置兼容性级别
    pub async fn set_compatibility_level(
        &self,
        subject: &str,
        level: CompatibilityLevel,
    ) -> Result<()> {
        let url = format!("{}/config/{}", self.base_url, subject);

        #[derive(Debug, Serialize)]
        struct SetConfigBody {
            compatibility: String,
        }

        let body = SetConfigBody {
            compatibility: level.to_string(),
        };

        let response = self.request(reqwest::Method::PUT, &url).json(&body).send().await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::Internal(error));
        }

        Ok(())
    }

    /// 删除 subject 的所有版本
    pub async fn delete_subject(&self, subject: &str) -> Result<Vec<i32>> {
        let url = format!("{}/subjects/{}", self.base_url, subject);
        let response = self.request(reqwest::Method::DELETE, &url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let versions: Vec<i32> = response.json().await?;
                Ok(versions)
            }
            StatusCode::NOT_FOUND => {
                Err(AppError::NotFound(format!("Subject '{}' not found", subject)))
            }
            _ => {
                let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(AppError::Internal(error))
            }
        }
    }

    /// 删除指定版本
    pub async fn delete_version(&self, subject: &str, version: i32) -> Result<i32> {
        let url = format!("{}/subjects/{}/versions/{}", self.base_url, subject, version);
        let response = self.request(reqwest::Method::DELETE, &url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let deleted_version: i32 = response.json().await?;
                Ok(deleted_version)
            }
            StatusCode::NOT_FOUND => {
                Err(AppError::NotFound(format!(
                    "Schema '{}' version {} not found",
                    subject, version
                )))
            }
            _ => {
                let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(AppError::Internal(error))
            }
        }
    }

    /// 测试连接
    pub async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/subjects", self.base_url);
        match self.request(reqwest::Method::GET, &url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
