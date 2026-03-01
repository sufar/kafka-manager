use sqlx::{sqlite::SqlitePool, Row};

/// API Key 记录
#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub is_active: bool,
}

/// 创建 API Key 请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
    pub expires_in_days: Option<i64>,
}

/// API Key 存储操作
pub struct ApiKeyStore;

impl ApiKeyStore {
    /// 列出所有 API Key（不返回完整的 key_hash，只返回前缀）
    pub async fn list(pool: &SqlitePool) -> Result<Vec<ApiKeyRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, key_hash, key_prefix, name, created_at, expires_at, is_active
            FROM api_keys
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let keys = rows
            .into_iter()
            .map(|row| ApiKeyRecord {
                id: row.get(0),
                key_hash: row.get(1),
                key_prefix: row.get(2),
                name: row.get(3),
                created_at: row.get(4),
                expires_at: row.get(5),
                is_active: row.get(6),
            })
            .collect();

        Ok(keys)
    }

    /// 根据 ID 获取 API Key
    pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<ApiKeyRecord>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, key_hash, key_prefix, name, created_at, expires_at, is_active
            FROM api_keys
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|row| ApiKeyRecord {
            id: row.get(0),
            key_hash: row.get(1),
            key_prefix: row.get(2),
            name: row.get(3),
            created_at: row.get(4),
            expires_at: row.get(5),
            is_active: row.get(6),
        }))
    }

    /// 根据 key 前缀获取 API Key（用于验证）
    pub async fn get_by_prefix(pool: &SqlitePool, prefix: &str) -> Result<Vec<ApiKeyRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, key_hash, key_prefix, name, created_at, expires_at, is_active
            FROM api_keys
            WHERE key_prefix = ? AND is_active = 1
            "#,
        )
        .bind(prefix)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ApiKeyRecord {
                id: row.get(0),
                key_hash: row.get(1),
                key_prefix: row.get(2),
                name: row.get(3),
                created_at: row.get(4),
                expires_at: row.get(5),
                is_active: row.get(6),
            })
            .collect())
    }

    /// 创建新的 API Key
    pub async fn create(
        pool: &SqlitePool,
        key_hash: &str,
        key_prefix: &str,
        name: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<ApiKeyRecord, sqlx::Error> {
        let created_at = chrono::Utc::now().to_rfc3339();

        let row = sqlx::query(
            r#"
            INSERT INTO api_keys (key_hash, key_prefix, name, created_at, expires_at, is_active)
            VALUES (?, ?, ?, ?, ?, 1)
            RETURNING id, key_hash, key_prefix, name, created_at, expires_at, is_active
            "#,
        )
        .bind(key_hash)
        .bind(key_prefix)
        .bind(name)
        .bind(&created_at)
        .bind(expires_at)
        .fetch_one(pool)
        .await?;

        Ok(ApiKeyRecord {
            id: row.get(0),
            key_hash: row.get(1),
            key_prefix: row.get(2),
            name: row.get(3),
            created_at: row.get(4),
            expires_at: row.get(5),
            is_active: row.get(6),
        })
    }

    /// 删除 API Key
    pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 禁用 API Key
    pub async fn deactivate(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE api_keys SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 验证 API Key
    pub async fn validate(pool: &SqlitePool, key: &str) -> Result<bool, sqlx::Error> {
        // 计算 key 的 hash
        use sha2::{Digest, Sha256};
        let hash = hex::encode(Sha256::digest(key.as_bytes()));

        let row = sqlx::query(
            r#"
            SELECT id FROM api_keys
            WHERE key_hash = ? AND is_active = 1
            AND (expires_at IS NULL OR expires_at > datetime('now'))
            "#,
        )
        .bind(&hash)
        .fetch_optional(pool)
        .await?;

        Ok(row.is_some())
    }

    /// 生成随机 API Key
    pub fn generate_key() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
        const KEY_LENGTH: usize = 32;

        let mut rng = rand::thread_rng();
        let key: String = (0..KEY_LENGTH)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        format!("km_{}", key)
    }

    /// 计算 API Key 的 hash
    pub fn hash_key(key: &str) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(key.as_bytes()))
    }
}
