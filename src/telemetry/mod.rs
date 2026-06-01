/// 遥测模块 - 处理系统信息上报和意见反馈

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};
use std::time::Duration;
use tokio::net::TcpStream;

/// MySQL 连接配置
pub const MYSQL_HOST: &str = "erp-uat-m201-mysql-m.kyeapi.com";
pub const MYSQL_PORT: u16 = 3306;
pub const MYSQL_DATABASE: &str = "crm_marketing";
pub const MYSQL_USER: &str = "app_crm_marketing_rw";
pub const MYSQL_PASSWORD: &str = "afjslad#HFG83272";

/// 遥测记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryRecord {
    pub hostname: String,
    pub username: String,
    pub local_ip: String,
    pub app_version: String,
    pub platform: String,
    pub install_method: String,
    pub report_date: String,
    pub reported_at: String,
}

/// 反馈记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub hostname: String,
    pub username: String,
    pub local_ip: String,
    pub app_version: String,
    pub platform: String,
    pub install_method: String,
    pub feedback_content: String,
    pub submitted_at: String,
}

/// MySQL 连接状态
#[derive(Debug, Clone)]
pub struct MySqlState {
    pub pool: Option<Pool<MySql>>,
    pub is_connected: bool,
}

impl MySqlState {
    pub fn new() -> Self {
        Self {
            pool: None,
            is_connected: false,
        }
    }
}

impl Default for MySqlState {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取主机名
pub fn get_hostname() -> String {
    whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string())
}

/// 获取用户名
pub fn get_username() -> String {
    whoami::username()
}

/// 获取本地 IP 地址
pub fn get_local_ip() -> String {
    match local_ip_address::local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => "unknown".to_string(),
    }
}

/// 获取应用版本号
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 获取操作系统平台
pub fn get_platform() -> String {
    #[cfg(target_os = "windows")]
    { "windows".to_string() }

    #[cfg(target_os = "macos")]
    { "macos".to_string() }

    #[cfg(target_os = "linux")]
    { "linux".to_string() }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    { "unknown".to_string() }
}

/// 获取安装方式
/// 通过检查运行环境判断是安装版还是便携版
pub fn get_install_method() -> String {
    // 在 Windows 上，检查是否在 Program Files 目录下运行
    #[cfg(target_os = "windows")]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            let path_str = exe_path.to_string_lossy().to_lowercase();
            if path_str.contains("program files") || path_str.contains("programfiles") {
                return "installed".to_string();
            }
        }
        return "portable".to_string();
    }

    // 在 macOS 上，检查是否在 Applications 目录下
    #[cfg(target_os = "macos")]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            let path_str = exe_path.to_string_lossy();
            if path_str.contains("/Applications/") {
                return "installed".to_string();
            }
        }
        return "portable".to_string();
    }

    // Linux 默认为 portable
    #[cfg(target_os = "linux")]
    {
        return "portable".to_string();
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    { "unknown".to_string() }
}

/// 检测 MySQL TCP 连接是否可用
pub async fn check_mysql_connection() -> bool {
    let addr = format!("{}:{}", MYSQL_HOST, MYSQL_PORT);
    let timeout = Duration::from_secs(5);

    match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => {
            tracing::info!("[Telemetry] MySQL TCP connection check succeeded: {}", addr);
            true
        }
        Ok(Err(e)) => {
            tracing::warn!("[Telemetry] MySQL TCP connection check failed: {}", e);
            false
        }
        Err(_) => {
            tracing::warn!("[Telemetry] MySQL TCP connection check timeout");
            false
        }
    }
}

/// 建立 MySQL 连接池
pub async fn connect_mysql() -> Result<Pool<MySql>, sqlx::Error> {
    let connection_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        MYSQL_USER, MYSQL_PASSWORD, MYSQL_HOST, MYSQL_PORT, MYSQL_DATABASE
    );

    tracing::info!("[Telemetry] Connecting to MySQL: {}@{}:{}/{}", MYSQL_USER, MYSQL_HOST, MYSQL_PORT, MYSQL_DATABASE);

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&connection_url)
        .await?;

    tracing::info!("[Telemetry] MySQL connection pool created successfully");

    Ok(pool)
}

/// 确保 MySQL 表存在
pub async fn ensure_mysql_tables(pool: &Pool<MySql>) -> Result<(), sqlx::Error> {
    // 创建遥测表（包含版本号、平台、安装方式）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kafka_manager_telemetry (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            hostname VARCHAR(255) NOT NULL,
            username VARCHAR(255) NOT NULL,
            local_ip VARCHAR(50) NOT NULL,
            app_version VARCHAR(50) NOT NULL,
            platform VARCHAR(20) NOT NULL,
            install_method VARCHAR(20) NOT NULL,
            report_date DATE NOT NULL,
            reported_at DATETIME NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE KEY unique_hostname_user_date (hostname, username, report_date)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
        "#,
    )
    .execute(pool)
    .await?;

    // 创建反馈表（包含版本号、平台、安装方式）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kafka_manager_feedback (
            id BIGINT PRIMARY KEY AUTO_INCREMENT,
            hostname VARCHAR(255) NOT NULL,
            username VARCHAR(255) NOT NULL,
            local_ip VARCHAR(50) NOT NULL,
            app_version VARCHAR(50) NOT NULL,
            platform VARCHAR(20) NOT NULL,
            install_method VARCHAR(20) NOT NULL,
            feedback_content TEXT NOT NULL,
            submitted_at DATETIME NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("[Telemetry] MySQL tables ensured");

    Ok(())
}

/// 上报遥测数据到 MySQL
pub async fn report_telemetry_to_mysql(
    pool: &Pool<MySql>,
    hostname: &str,
    username: &str,
    local_ip: &str,
    app_version: &str,
    platform: &str,
    install_method: &str,
    report_date: &str,
    reported_at: &str,
) -> Result<bool, sqlx::Error> {
    // 检查今天是否已上报（MySQL 端也检查）
    let existing: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM kafka_manager_telemetry
        WHERE hostname = ? AND username = ? AND report_date = ?
        "#,
    )
    .bind(hostname)
    .bind(username)
    .bind(report_date)
    .fetch_one(pool)
    .await?;

    if existing.unwrap_or(0) > 0 {
        tracing::info!("[Telemetry] Already reported today for {}@{}", username, hostname);
        return Ok(false);
    }

    // 插入新记录（包含版本号、平台、安装方式）
    sqlx::query(
        r#"
        INSERT INTO kafka_manager_telemetry (hostname, username, local_ip, app_version, platform, install_method, report_date, reported_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(hostname)
    .bind(username)
    .bind(local_ip)
    .bind(app_version)
    .bind(platform)
    .bind(install_method)
    .bind(report_date)
    .bind(reported_at)
    .execute(pool)
    .await?;

    tracing::info!("[Telemetry] Telemetry reported successfully: {}@{} on {} (v{} on {})",
        username, hostname, report_date, app_version, platform);

    Ok(true)
}

/// 提交意见反馈到 MySQL
pub async fn submit_feedback_to_mysql(
    pool: &Pool<MySql>,
    hostname: &str,
    username: &str,
    local_ip: &str,
    app_version: &str,
    platform: &str,
    install_method: &str,
    feedback_content: &str,
    submitted_at: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO kafka_manager_feedback (hostname, username, local_ip, app_version, platform, install_method, feedback_content, submitted_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(hostname)
    .bind(username)
    .bind(local_ip)
    .bind(app_version)
    .bind(platform)
    .bind(install_method)
    .bind(feedback_content)
    .bind(submitted_at)
    .execute(pool)
    .await?;

    let id = result.last_insert_id() as i64;
    tracing::info!("[Telemetry] Feedback submitted successfully: id={} (v{} on {})", id, app_version, platform);

    Ok(id)
}

/// 检查本地 SQLite 今天是否已上报
pub async fn check_local_reported(
    sqlite_pool: &sqlx::SqlitePool,
    hostname: &str,
    username: &str,
    report_date: &str,
) -> Result<bool, sqlx::Error> {
    let existing: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM telemetry_records
        WHERE hostname = ? AND username = ? AND report_date = ?
        "#,
    )
    .bind(hostname)
    .bind(username)
    .bind(report_date)
    .fetch_one(sqlite_pool)
    .await?;

    Ok(existing.unwrap_or(0) > 0)
}

/// 记录本地遥测记录
pub async fn record_local_telemetry(
    sqlite_pool: &sqlx::SqlitePool,
    hostname: &str,
    username: &str,
    local_ip: &str,
    app_version: &str,
    platform: &str,
    install_method: &str,
    report_date: &str,
    reported_at: &str,
) -> Result<bool, sqlx::Error> {
    // 先检查是否已存在
    if check_local_reported(sqlite_pool, hostname, username, report_date).await? {
        tracing::info!("[Telemetry] Local record already exists for today");
        return Ok(false);
    }

    // 插入新记录
    sqlx::query(
        r#"
        INSERT INTO telemetry_records (hostname, username, local_ip, app_version, platform, install_method, report_date, reported_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(hostname)
    .bind(username)
    .bind(local_ip)
    .bind(app_version)
    .bind(platform)
    .bind(install_method)
    .bind(report_date)
    .bind(reported_at)
    .execute(sqlite_pool)
    .await?;

    tracing::info!("[Telemetry] Local telemetry record saved: {}@{} on {}", username, hostname, report_date);

    Ok(true)
}

/// 执行遥测上报（完整的上报流程）
pub async fn do_telemetry_report(
    sqlite_pool: &sqlx::SqlitePool,
    mysql_pool: &Pool<MySql>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let hostname = get_hostname();
    let username = get_username();
    let local_ip = get_local_ip();
    let app_version = get_app_version();
    let platform = get_platform();
    let install_method = get_install_method();
    let now = Local::now();
    let report_date = now.format("%Y-%m-%d").to_string();
    let reported_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // 先检查本地是否已上报
    if check_local_reported(sqlite_pool, &hostname, &username, &report_date).await? {
        tracing::info!("[Telemetry] Already reported today (local check)");
        return Ok(false);
    }

    // 上报到 MySQL
    let reported = report_telemetry_to_mysql(
        mysql_pool,
        &hostname,
        &username,
        &local_ip,
        &app_version,
        &platform,
        &install_method,
        &report_date,
        &reported_at,
    ).await?;

    if reported {
        // 记录到本地 SQLite
        record_local_telemetry(
            sqlite_pool,
            &hostname,
            &username,
            &local_ip,
            &app_version,
            &platform,
            &install_method,
            &report_date,
            &reported_at,
        ).await?;

        tracing::info!("[Telemetry] Telemetry report completed successfully");
        return Ok(true);
    }

    Ok(false)
}