//! 后端引导：初始化 AppState（数据库、Kafka 客户端、遥测）
//!
//! 与原 Tauri 壳相同的模式：在独立的 tokio runtime 上构建 AppState，
//! DB 连接池、Kafka 客户端与遥测任务都驻留在该 runtime 上。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;

use kafka_manager_api::{
    telemetry, AppState, ClusterPools, Config, DbPool, ImportExportLock, KafkaClients,
    RefreshState,
};

/// 数据库文件路径（系统数据目录）
pub fn db_path() -> String {
    let db_filename = "kafka_manager.db";

    let data_dir = if cfg!(target_os = "windows") {
        dirs::data_local_dir().map(|d| d.join("Kafka Manager"))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir().map(|d| d.join("Library/Application Support/Kafka Manager"))
    } else {
        dirs::data_local_dir().map(|d| d.join("kafka-manager"))
    };

    if let Some(dir) = data_dir {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!("Failed to create data dir {:?}: {}", dir, e);
        }
        dir.join(db_filename).to_string_lossy().to_string()
    } else {
        db_filename.to_string()
    }
}

/// 加载配置（可执行文件旁的 config.toml，可选）
fn load_config() -> Config {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let config_path = exe_dir.join("config.toml");
    if config_path.exists() {
        Config::load(&config_path).unwrap_or_else(|e| {
            tracing::warn!("Config load error: {}, using default", e);
            Config::default()
        })
    } else if cfg!(debug_assertions) {
        Config::load("config.toml").unwrap_or_default()
    } else {
        Config::default()
    }
}

/// 构建 AppState 并启动后台任务（Kafka 连接、遥测）
pub async fn build_app_state() -> Result<AppState> {
    let config = load_config();
    let db_path = db_path();
    tracing::info!("Using database: {}", db_path);

    let pool = DbPool::new(&db_path)
        .await
        .context("Failed to create database pool")?;
    pool.init().await.context("Failed to init database")?;

    let clients = Arc::new(ArcSwap::new(Arc::new(KafkaClients::default())));
    let kafka_pools = ClusterPools::new();

    let state = AppState {
        db: pool.clone(),
        clients: clients.clone(),
        config: config.clone(),
        pools: kafka_pools.clone(),
        refresh_state: Arc::new(Mutex::new(RefreshState::default())),
        import_export_lock: Arc::new(Mutex::new(ImportExportLock::default())),
    };

    // 后台异步建立 Kafka 连接
    let cluster_count = config.clusters.len();
    if cluster_count > 0 {
        let clients_arc = clients.clone();
        let pools_arc = kafka_pools.clone();
        let clients_config = config.clusters.clone();
        let pool_config = config.pool.clone();

        tokio::spawn(async move {
            tracing::info!("Background: Creating Kafka clients for {} cluster(s)...", cluster_count);

            let clients_config_clone = clients_config.clone();
            let clients_result =
                tokio::task::spawn_blocking(move || KafkaClients::new(&clients_config_clone)).await;

            match clients_result {
                Ok(Ok(new_clients)) => {
                    clients_arc.store(Arc::new(new_clients));
                }
                Ok(Err(e)) => {
                    tracing::warn!("Background: Failed to create Kafka clients: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Background: Kafka client creation panicked: {}", e);
                }
            }

            if let Err(e) = pools_arc.init(&clients_config, &pool_config).await {
                tracing::warn!("Background: Failed to init Kafka pools: {}", e);
            }
        });
    }

    // 遥测后台任务：启动时检查并上报，每小时一次
    let pool_for_telemetry = pool.clone();
    tokio::spawn(async move {
        if !telemetry::check_mysql_connection().await {
            return;
        }
        let mysql_pool = match telemetry::connect_mysql().await {
            Ok(p) => p,
            Err(_) => return,
        };
        let _ = telemetry::do_telemetry_report(pool_for_telemetry.inner(), &mysql_pool).await;

        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if !telemetry::check_mysql_connection().await {
                continue;
            }
            let _ = telemetry::do_telemetry_report(pool_for_telemetry.inner(), &mysql_pool).await;
        }
    });

    Ok(state)
}
