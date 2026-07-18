mod backend;
mod i18n;
mod pages;
mod service;
mod state;
mod workspace;

use gpui::*;
use gpui_component::*;

use crate::state::{Backend, TokioRuntime};
use crate::workspace::Workspace;

fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    kafka_manager_api::utils::ensure_log_dir();
    let log_path = kafka_manager_api::utils::app_log_path();
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("open log file");
    let (non_blocking, guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%d %H:%M:%S".to_string(),
        ))
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rdkafka=warn".into()),
        )
        .init();
    guard
}

fn main() {
    let _log_guard = init_logging();
    tracing::info!("Kafka Manager (GPUI) starting...");

    // 后端线程：独立的 tokio runtime（DB 连接池、Kafka 客户端、遥测都驻留在此）
    let (rt_tx, rt_rx) = std::sync::mpsc::channel::<tokio::runtime::Handle>();
    let (state_tx, state_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
        let _ = rt_tx.send(rt.handle().clone());
        rt.block_on(async move {
            let state = backend::build_app_state()
                .await
                .map_err(|e| tracing::error!("Backend init failed: {:#}", e))
                .ok();
            let _ = state_tx.send(state);
            // 保持 runtime 存活
            std::future::pending::<()>().await;
        });
    });

    let tokio_handle = rt_rx.recv().expect("receive tokio runtime handle");
    // 等待后端初始化（最多 30 秒，与原 Tauri 行为一致）
    let app_state = match state_rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(state) => state,
        Err(_) => {
            tracing::error!("Backend init timed out");
            None
        }
    };

    Application::new().run(move |cx| {
        gpui_component::init(cx);
        i18n::I18n::init(cx);
        cx.set_global(Backend(app_state));
        cx.set_global(TokioRuntime(tokio_handle));
        gpui_component::Theme::sync_system_appearance(None, cx);

        cx.spawn(async move |cx| {
            cx.open_window(window_options(), |window, cx| {
                let view = cx.new(|cx| Workspace::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

fn window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(100.0), px(100.0)),
            size: size(px(1280.0), px(800.0)),
        })),
        app_id: Some("kafka-manager".to_string()),
        ..Default::default()
    }
}
