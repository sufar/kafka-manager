//! 后端桥接：在独立线程跑 tokio runtime，承载 AppState（DB/Kafka/遥测），
//! UI 侧通过 `dispatch()`（oneshot 回调）与 `stream_messages()`（async_channel 事件流）调用。
//!
//! tokio 的 oneshot/mpsc/async-channel Receiver 都是 executor 无关的 Future，
//! 可以直接在 gpui 的 `cx.spawn` 里 await。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use kafka_manager_api::api::{self, StreamEvent};
use kafka_manager_api::{
    telemetry, AppState, ClusterPools, Config, DbPool, ImportExportLock, KafkaClients, RefreshState,
};

type DispatchResult = Result<Value, String>;

enum Job {
    Dispatch {
        method: String,
        params: Value,
        respond: oneshot::Sender<DispatchResult>,
    },
    StartStream {
        request_id: String,
        params: Value,
        events: async_channel::Sender<StreamEvent>,
    },
    CancelStream {
        request_id: String,
    },
}

#[derive(Clone)]
pub struct Backend {
    tx: mpsc::UnboundedSender<Job>,
    ready: Arc<AtomicBool>,
    init_error: Arc<Mutex<Option<String>>>,
}

impl Backend {
    /// 启动后端线程（立即返回，初始化在后台进行）
    pub fn start() -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<Job>();
        let ready = Arc::new(AtomicBool::new(false));
        let init_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let ready_bg = ready.clone();
        let init_error_bg = init_error.clone();
        std::thread::Builder::new()
            .name("kafka-backend".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                rt.block_on(backend_main(rx, ready_bg, init_error_bg));
            })
            .expect("spawn backend thread");

        Self { tx, ready, init_error }
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    pub fn init_error(&self) -> Option<String> {
        self.init_error.lock().ok().and_then(|e| e.clone())
    }

    /// 统一 API 分发（等价 Tauri 的 api_request）。返回的 Future 可在任意 executor 上 await。
    pub async fn dispatch(&self, method: &str, params: Value) -> DispatchResult {
        let (respond, rx) = oneshot::channel();
        self.tx
            .send(Job::Dispatch {
                method: method.to_string(),
                params,
                respond,
            })
            .map_err(|_| "backend thread stopped".to_string())?;
        rx.await.unwrap_or_else(|_| Err("backend dropped request".to_string()))
    }

    /// 启动流式消息查询，事件（start/batch/complete/error）通过返回的 channel 推送。
    /// 返回 (request_id, receiver)。UI 用 `cancel_stream(request_id)` 取消。
    pub fn stream_messages(
        &self,
        params: Value,
    ) -> (String, async_channel::Receiver<StreamEvent>) {
        let request_id = format!(
            "msg-{}-{:08x}",
            chrono_millis(),
            rand_u32()
        );
        let (events_tx, events_rx) = async_channel::bounded(256);
        let _ = self.tx.send(Job::StartStream {
            request_id: request_id.clone(),
            params,
            events: events_tx,
        });
        (request_id, events_rx)
    }

    pub fn cancel_stream(&self, request_id: &str) {
        let _ = self.tx.send(Job::CancelStream {
            request_id: request_id.to_string(),
        });
    }
}

fn chrono_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn rand_u32() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(chrono_millis() as u64 ^ (std::process::id() as u64));
    h.finish() as u32
}

async fn backend_main(
    mut rx: mpsc::UnboundedReceiver<Job>,
    ready: Arc<AtomicBool>,
    init_error: Arc<Mutex<Option<String>>>,
) {
    // ---- 初始化 AppState（镜像 src-tauri 的流程）----
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let config = if cfg!(debug_assertions) {
        Config::load("config.toml").unwrap_or_default()
    } else {
        let config_path = exe_dir.join("config.toml");
        if config_path.exists() {
            Config::load(&config_path).unwrap_or_default()
        } else {
            Config::default()
        }
    };

    let db_path = {
        let db_filename = "kafka_manager.db";
        let data_dir = if cfg!(target_os = "windows") {
            dirs::data_local_dir().map(|d| d.join("Kafka Manager"))
        } else if cfg!(target_os = "macos") {
            dirs::home_dir().map(|d| d.join("Library/Application Support/Kafka Manager"))
        } else {
            dirs::data_local_dir().map(|d| d.join("kafka-manager"))
        };
        match data_dir {
            Some(dir) => {
                if std::fs::create_dir_all(&dir).is_ok() {
                    dir.join(db_filename).to_string_lossy().to_string()
                } else {
                    exe_dir.join(db_filename).to_string_lossy().to_string()
                }
            }
            None => exe_dir.join(db_filename).to_string_lossy().to_string(),
        }
    };

    let state: AppState = match DbPool::new(&db_path).await {
        Ok(pool) => {
            if let Err(e) = pool.init().await {
                if let Ok(mut g) = init_error.lock() {
                    *g = Some(format!("Failed to init database: {}", e));
                }
                return;
            }
            let pool_for_telemetry = pool.clone();
            let clients = Arc::new(arc_swap::ArcSwap::new(Arc::new(KafkaClients::default())));
            let kafka_pools = ClusterPools::new();

            let state = AppState {
                db: pool,
                clients: clients.clone(),
                config: config.clone(),
                pools: kafka_pools.clone(),
                refresh_state: Arc::new(std::sync::Mutex::new(RefreshState::default())),
                import_export_lock: Arc::new(std::sync::Mutex::new(ImportExportLock::default())),
            };

            // 后台建立 Kafka 客户端与连接池
            let cluster_count = config.clusters.len();
            if cluster_count > 0 {
                let clients_arc = clients.clone();
                let pools_arc = kafka_pools.clone();
                let clients_config = config.clusters.clone();
                let pool_config = config.pool.clone();
                tokio::spawn(async move {
                    let cc = clients_config.clone();
                    if let Ok(Ok(new_clients)) =
                        tokio::task::spawn_blocking(move || KafkaClients::new(&cc)).await
                    {
                        clients_arc.store(Arc::new(new_clients));
                    }
                    let _ = pools_arc.init(&clients_config, &pool_config).await;
                });
            }

            // 遥测后台任务
            tokio::spawn(async move {
                if !telemetry::check_mysql_connection().await {
                    return;
                }
                let mysql_pool = match telemetry::connect_mysql().await {
                    Ok(p) => p,
                    Err(_) => return,
                };
                let _ = telemetry::do_telemetry_report(pool_for_telemetry.inner(), &mysql_pool).await;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    if !telemetry::check_mysql_connection().await {
                        continue;
                    }
                    let _ =
                        telemetry::do_telemetry_report(pool_for_telemetry.inner(), &mysql_pool).await;
                }
            });

            state
        }
        Err(e) => {
            if let Ok(mut g) = init_error.lock() {
                *g = Some(format!("Failed to create database pool: {}", e));
            }
            return;
        }
    };

    ready.store(true, Ordering::SeqCst);
    tracing::info!("backend ready");

    // ---- 任务循环 ----
    let streams: Arc<Mutex<HashMap<String, CancellationToken>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Some(job) = rx.recv().await {
        match job {
            Job::Dispatch {
                method,
                params,
                respond,
            } => {
                let state = state.clone();
                tokio::spawn(async move {
                    let result = api::dispatch_request(&method, state, params)
                        .await
                        .map_err(|e| e.to_message());
                    let _ = respond.send(result);
                });
            }
            Job::StartStream {
                request_id,
                params,
                events,
            } => {
                let state = state.clone();
                let streams = streams.clone();
                tokio::spawn(async move {
                    let cancel = CancellationToken::new();
                    streams
                        .lock()
                        .unwrap()
                        .insert(request_id.clone(), cancel.clone());
                    match api::start_message_list_stream(state, params, cancel.clone()).await {
                        Ok(mut rx) => {
                            while let Some(evt) = rx.recv().await {
                                if events.send(evt).await.is_err() {
                                    cancel.cancel();
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            let _ = events
                                .send(StreamEvent {
                                    event: "error".into(),
                                    data: serde_json::json!({"error": e.to_message()}).to_string(),
                                })
                                .await;
                        }
                    }
                    streams.lock().unwrap().remove(&request_id);
                });
            }
            Job::CancelStream { request_id } => {
                if let Some(token) = streams.lock().unwrap().remove(&request_id) {
                    token.cancel();
                }
            }
        }
    }
}
