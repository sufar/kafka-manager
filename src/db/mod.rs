pub mod cluster;
pub mod cluster_group;
pub mod consumer_group;
pub mod favorite;
pub mod settings;
pub mod topic;
pub mod topic_template;
pub mod topic_history;
pub mod sent_message;
pub mod json_highlight;
pub mod schema_registry;

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
            // 每个新连接建立时设置 PRAGMA，确保所有连接都使用统一的配置
            .after_connect(|conn, _| {
                Box::pin(async move {
                    // 启用 WAL 模式提升并发性能
                    sqlx::query("PRAGMA journal_mode = WAL")
                        .execute(&mut *conn)
                        .await?;
                    // 限制 SQLite 缓存为 2MB，避免大量查询后缓存页不释放
                    // -2000 表示 2000 KB，SQLite 会在空闲时自动释放超出限制的缓存
                    sqlx::query("PRAGMA cache_size = -2000")
                        .execute(&mut *conn)
                        .await?;
                    // 定期将 WAL 检查点写入主数据库文件（每 1000 页）
                    sqlx::query("PRAGMA wal_autocheckpoint = 1000")
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
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

        // resource_tags 索引：优化按 cluster_id 和资源名称查询
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_resource_tags_cluster ON resource_tags(cluster_id)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_resource_tags_resource ON resource_tags(resource_type, resource_name)",
        )
        .execute(self.inner())
        .await?;

        // resource_tags 标签键值索引：优化按标签查询和标签键去重
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_resource_tags_key ON resource_tags(tag_key)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_resource_tags_key_value ON resource_tags(tag_key, tag_value)",
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

        // 复合索引：优化搜索 Topic 时的 cluster + name 组合查询
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_topic_metadata_cluster_name ON topic_metadata(cluster_id, topic_name)",
        )
        .execute(self.inner())
        .await?;

        // 创建 Consumer Group 元数据表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS consumer_group_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cluster_id TEXT NOT NULL,
                group_name TEXT NOT NULL,
                topics TEXT NOT NULL DEFAULT '[]',
                fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(cluster_id, group_name)
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_consumer_group_metadata_cluster ON consumer_group_metadata(cluster_id)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_consumer_group_metadata_name ON consumer_group_metadata(group_name)",
        )
        .execute(self.inner())
        .await?;

        // 创建 Consumer Group 与 Topic 多对多关系表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS consumer_group_topics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cluster_id TEXT NOT NULL,
                group_name TEXT NOT NULL,
                topic_name TEXT NOT NULL,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(cluster_id, group_name, topic_name)
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_consumer_group_topics_cluster_topic ON consumer_group_topics(cluster_id, topic_name)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_consumer_group_topics_cluster_group ON consumer_group_topics(cluster_id, group_name)",
        )
        .execute(self.inner())
        .await?;

        // 创建 Consumer Group Offset 缓存表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS consumer_group_offsets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cluster_id TEXT NOT NULL,
                group_name TEXT NOT NULL,
                topic_name TEXT NOT NULL,
                partition_id INTEGER NOT NULL,
                offset_value INTEGER NOT NULL,
                lag INTEGER NOT NULL,
                last_commit_time INTEGER,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(cluster_id, group_name, topic_name, partition_id)
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_consumer_group_offsets_cluster_group ON consumer_group_offsets(cluster_id, group_name)",
        )
        .execute(self.inner())
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_consumer_group_offsets_topic ON consumer_group_offsets(cluster_id, topic_name)",
        )
        .execute(self.inner())
        .await?;

        // 复合索引：优化按 cluster + group 查询 consumer group offsets
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_consumer_group_offsets_cluster_group ON consumer_group_offsets(cluster_id, group_name, topic_name)",
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

        // 创建 JSON 高亮模板表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS json_highlight_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                is_builtin INTEGER NOT NULL DEFAULT 0,
                style_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_json_highlight_name ON json_highlight_templates(name)")
            .execute(self.inner())
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_json_highlight_builtin ON json_highlight_templates(is_builtin)")
            .execute(self.inner())
            .await?;

        // 创建收藏表
        favorite::init_tables(self.inner()).await?;

        // 创建 Topic 历史表
        topic_history::init_tables(self.inner()).await?;

        // 创建发送消息历史表
        sent_message::init_tables(self.inner()).await?;

        // 创建集群分组表
        self.init_cluster_groups_table().await?;

        // 运行额外的索引优化迁移
        self.run_indexes_migration().await?;

        // 初始化内置 JSON 高亮模板
        json_highlight::JsonHighlightTemplate::init_builtin_templates(self.inner()).await?;

        // 初始化 Schema Registry 表
        schema_registry::init_tables(self.inner()).await?;

        // 初始化遥测记录表
        self.init_telemetry_table().await?;

        tracing::info!("[KAFKA-MANAGER] Database tables initialized successfully");

        Ok(())
    }

    /// 运行索引优化迁移
    async fn run_indexes_migration(&self) -> Result<(), sqlx::Error> {
        // Topic 标签索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_cluster_id ON resource_tags(cluster_id)")
            .execute(self.inner()).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_resource ON resource_tags(resource_type, resource_name)")
            .execute(self.inner()).await?;

        Ok(())
    }

    /// 初始化集群分组表
    async fn init_cluster_groups_table(&self) -> Result<(), sqlx::Error> {
        // 创建集群分组表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cluster_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        // 为 kafka_clusters 表添加 group_id 字段（如果不存在）
        // SQLite 不支持直接添加 IF NOT EXISTS，需要检查字段是否存在
        let column_exists: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('kafka_clusters') WHERE name = 'group_id'",
        )
        .fetch_one(self.inner())
        .await?;

        if column_exists.unwrap_or(0) == 0 {
            sqlx::query("ALTER TABLE kafka_clusters ADD COLUMN group_id INTEGER REFERENCES cluster_groups(id)")
                .execute(self.inner())
                .await?;
        }

        // 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_cluster_groups_name ON cluster_groups(name)")
            .execute(self.inner())
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_cluster_groups_sort_order ON cluster_groups(sort_order)")
            .execute(self.inner())
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_kafka_clusters_group_id ON kafka_clusters(group_id)")
            .execute(self.inner())
            .await?;

        // 创建默认分组（如果不存在）
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO cluster_groups (name, description, sort_order, created_at, updated_at)
            VALUES ('default', '默认分组，包含所有未分组的集群', 0, datetime('now'), datetime('now'))
            "#,
        )
        .execute(self.inner())
        .await?;

        // 将现有集群分配到默认分组（如果 group_id 为空）
        sqlx::query(
            r#"
            UPDATE kafka_clusters
            SET group_id = (SELECT id FROM cluster_groups WHERE name = 'default')
            WHERE group_id IS NULL
            "#,
        )
        .execute(self.inner())
        .await?;

        Ok(())
    }

    /// 初始化遥测记录表
    async fn init_telemetry_table(&self) -> Result<(), sqlx::Error> {
        // 创建遥测记录表（包含版本号、平台、安装方式）
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS telemetry_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hostname TEXT NOT NULL,
                username TEXT NOT NULL,
                local_ip TEXT NOT NULL,
                app_version TEXT NOT NULL,
                platform TEXT NOT NULL,
                install_method TEXT NOT NULL,
                report_date TEXT NOT NULL,
                reported_at TEXT NOT NULL,
                UNIQUE(hostname, username, report_date)
            )
            "#,
        )
        .execute(self.inner())
        .await?;

        // 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_telemetry_records_date ON telemetry_records(report_date)")
            .execute(self.inner())
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_telemetry_records_hostname_user ON telemetry_records(hostname, username)")
            .execute(self.inner())
            .await?;

        tracing::info!("[KAFKA-MANAGER] Telemetry records table initialized");

        Ok(())
    }
}
