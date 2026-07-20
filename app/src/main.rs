mod backend;
mod components;
mod i18n;
mod pages;
mod service;
mod state;
mod tray;
mod updater;
mod workspace;

use gpui::*;
use gpui_component::notification::{Notification, NotificationType};
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

/// 同步读取托盘开关设置（独立线程 + 临时 runtime，避免与后端 runtime 冲突）
fn read_tray_enabled(db_path: &str) -> bool {
    let db_path = db_path.to_string();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        rt.block_on(async move {
            let url = format!("sqlite:{}?mode=ro", db_path);
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&url)
                .await
                .ok()?;
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT value FROM user_settings WHERE key = 'ui.system_tray'",
            )
            .fetch_optional(&pool)
            .await
            .ok()?;
            Some(row.map(|r| r.0 == "true").unwrap_or(false))
        })
    })
    .join()
    .ok()
    .flatten()
    .unwrap_or(false)
}

fn main() {
    // 单实例检查
    let instance = single_instance::SingleInstance::new("kafka-manager-gpui")
        .expect("single instance");
    if !instance.is_single() {
        eprintln!("Kafka Manager is already running");
        std::process::exit(0);
    }

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

    // 托盘（根据设置）
    let tray_enabled = read_tray_enabled(&backend::db_path());

    Application::new().with_assets(gpui_component_assets::Assets).run(move |cx| {
        gpui_component::init(cx);
        i18n::I18n::init(cx);
        cx.set_global(Backend(app_state));
        cx.set_global(TokioRuntime(tokio_handle));
        gpui_component::Theme::sync_system_appearance(None, cx);

        if tray_enabled {
            tray::set_tray_enabled(true);
        }

        // 托盘菜单事件轮询
        cx.spawn(async move |cx| loop {
            match tray::poll_menu_event().as_deref() {
                Some(tray::MENU_ID_QUIT) => {
                    cx.update(|cx| cx.quit()).ok();
                    break;
                }
                Some(tray::MENU_ID_SHOW) => {
                    cx.update(|cx| {
                        for w in cx.windows() {
                            let _ = w.update(cx, |_, window, _cx| {
                                window.activate_window();
                            });
                        }
                    })
                    .ok();
                }
                _ => {}
            }
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
        })
        .detach();

        // 启动后延迟自动检查更新（有新版本时通知）
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(3))
                .await;
            if let Ok(result) = updater::do_check_updates().await {
                if result.available {
                    cx.update(|cx| {
                        let is_zh = i18n::I18n::global(cx).is_zh();
                        let text = if is_zh {
                            format!("🔄 发现新版本 v{}，请前往 设置 页面进行更新", result.version)
                        } else {
                            format!("🔄 New version v{} available, go to Settings to update", result.version)
                        };
                        for w in cx.windows() {
                            let text = text.clone();
                            let _ = w.update(cx, |_, window, cx| {
                                window.push_notification(
                                    Notification::new().message(text).with_type(NotificationType::Warning),
                                    cx,
                                );
                            });
                        }
                    })
                    .ok();
                }
            }
        })
        .detach();

        cx.spawn(async move |cx| {
            cx.open_window(window_options(), |window, cx| {
                // 托盘启用时，关闭窗口改为隐藏（最小化到托盘）
                let tray_on = tray_enabled;
                window.on_window_should_close(cx, move |window, cx| {
                    if tray_on {
                        cx.hide();
                        false
                    } else {
                        let _ = window;
                        true
                    }
                });
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
