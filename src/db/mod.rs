pub mod api_key;
pub mod audit_log;
pub mod cluster;
pub mod cluster_connection;
pub mod settings;
pub mod tag;
pub mod topic;
pub mod topic_template;
pub mod user;

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;
use tracing;

/// 数据库连接池
#[derive(Clone)]
pub struct DbPool {
    pool: Arc<SqlitePool>,
}

impl DbPool {
    /// 创建新的数据库连接池
    pub async fn new(database_path: &str) -> Result<Self, sqlx::Error> {
        // 确保使用绝对路径
        let db_path = std::path::Path::new(database_path);
        let abs_path = if db_path.is_absolute() {
            db_path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(db_path)
        };

        // 确保父目录存在
        if let Some(parent) = abs_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // 首先尝试打开文件，如果不存在则创建
        let _file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&abs_path)
            .map_err(|e| sqlx::Error::Protocol(format!("Cannot create database file: {}", e)))?;

        // 构建 SQLite 连接 URL
        let path_str = abs_path.to_string_lossy().to_string();

        // Windows 和 Unix 使用不同的 URL 格式
        #[cfg(target_os = "windows")]
        let conn_url = {
            // Windows: 路径使用正斜杠，sqlite:///C:/path/to/db 格式
            let normalized_path = path_str.replace('\\', "/");
            format!("sqlite:///{}", normalized_path)
        };

        #[cfg(not(target_os = "windows"))]
        let conn_url = format!("sqlite:///{}", path_str);

        tracing::info!("[KAFKA-MANAGER] Database connection URL: {}", conn_url);

        let pool = SqlitePoolOptions::new()
            .max_connections(20)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&conn_url)
            .await?;

        tracing::info!("[KAFKA-MANAGER] Database pool created successfully");

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// 获取底层连接池
    pub fn inner(&self) -> &SqlitePool {
        &self.pool
    }

    /// 初始化数据库表
    pub async fn init(&self) -> Result<(), sqlx::Error> {
        tracing::info!("[KAFKA-MANAGER] Initializing database tables...");

        // 运行迁移
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS kafka_clusters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                brokers TEXT NOT NULL,
                request_timeout_ms INTEGER NOT NULL DEFAULT 5000,
                operation_timeout_ms INTEGER NOT NULL DEFAULT 5000,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_kafka_clusters_name ON kafka_clusters(name)",
        )
        .execute(self.inner())
        .await?;

        // 创建 API Key 表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key_hash TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                name TEXT,
                created_at TEXT NOT NULL,
                expires_at TEXT,
                is_active INTEGER NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_api_keys_hash ON api_keys(key_hash)",
        )
        .execute(self.inner())
        .await?;

        // 创建审计日志表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                cluster_id TEXT,
                resource TEXT,
                action TEXT NOT NULL,
                api_key TEXT,
                status INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                client_ip TEXT
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        // 创建告警规则表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS resource_tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cluster_id TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_name TEXT NOT NULL,
                tag_key TEXT NOT NULL,
                tag_value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(cluster_id, resource_type, resource_name, tag_key)
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        // 创建 Topic 模板表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS topic_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                num_partitions INTEGER NOT NULL DEFAULT 3,
                replication_factor INTEGER NOT NULL DEFAULT 1,
                config_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        // 创建集群连接历史表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cluster_connection_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cluster_name TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                latency_ms INTEGER,
                checked_at TEXT NOT NULL
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_cluster_history_name ON cluster_connection_history(cluster_name)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_cluster_history_checked_at ON cluster_connection_history(checked_at DESC)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_cluster_history_name_checked_at ON cluster_connection_history(cluster_name, checked_at DESC)",
        )
        .execute(self.inner())
        .await?;

        // 创建用户和角色表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS roles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                permissions TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                email TEXT,
                role_id INTEGER NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (role_id) REFERENCES roles(id)
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)",
        )
        .execute(self.inner())
        .await?;

        // 创建 Topic 元数据表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS topic_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cluster_id TEXT NOT NULL,
                topic_name TEXT NOT NULL,
                partition_count INTEGER NOT NULL DEFAULT 1,
                replication_factor INTEGER NOT NULL DEFAULT 1,
                config_json TEXT NOT NULL DEFAULT '{}',
                fetched_at TEXT NOT NULL,
                UNIQUE(cluster_id, topic_name)
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_topic_metadata_cluster ON topic_metadata(cluster_id)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_topic_metadata_name ON topic_metadata(topic_name)",
        )
        .execute(self.inner())
        .await?;

        // 创建全局设置表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        // 运行额外的索引优化迁移
        self.run_indexes_migration().await?;

        // 初始化默认角色
        self.init_default_roles().await?;

        tracing::info!("[KAFKA-MANAGER] Database tables initialized successfully");

        Ok(())
    }

    /// 初始化默认角色
    async fn init_default_roles(&self) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        // 创建管理员角色（所有权限）
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO roles (name, description, permissions, created_at)
            VALUES ('admin', '系统管理员，拥有所有权限', '["*"]', ?)
            "#,
        )
        .bind(&now)
        .execute(self.inner())
        .await?;

        // 创建运维角色
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO roles (name, description, permissions, created_at)
            VALUES ('operator', '运维人员，拥有集群管理权限',
                    '["cluster:*", "topic:*", "consumer_group:*", "message:*"]', ?)
            "#,
        )
        .bind(&now)
        .execute(self.inner())
        .await?;

        // 创建只读角色
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO roles (name, description, permissions, created_at)
            VALUES ('viewer', '只读用户，仅查看权限',
                    '["cluster:read", "topic:read", "consumer_group:read", "message:read"]', ?)
            "#,
        )
        .bind(&now)
        .execute(self.inner())
        .await?;

        Ok(())
    }

    /// 运行索引优化迁移
    async fn run_indexes_migration(&self) -> Result<(), sqlx::Error> {
        // 审计日志索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_logs_cluster_id ON audit_logs(cluster_id)")
            .execute(self.inner()).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC)")
            .execute(self.inner()).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_logs_cluster_timestamp ON audit_logs(cluster_id, timestamp DESC)")
            .execute(self.inner()).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_logs_status ON audit_logs(status)")
            .execute(self.inner()).await?;

        // API Key 索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_is_active ON api_keys(is_active)")
            .execute(self.inner()).await?;

        // Topic 标签索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_cluster_id ON resource_tags(cluster_id)")
            .execute(self.inner()).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_resource ON resource_tags(resource_type, resource_name)")
            .execute(self.inner()).await?;

        Ok(())
    }
}
