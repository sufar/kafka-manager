#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use gpui::{actions, px, size, App, AppContext, Bounds, KeyBinding, Menu, MenuItem, WindowBounds, WindowOptions};
use kafka_manager_gpui::app::KafkaManagerApp;

actions!(kafka_manager, [Quit]);

fn quit(_: &Quit, cx: &mut App) {
    cx.quit();
}

/// 尽量注册 CJK 字体（Linux/Android 环境 fontconfig 可能扫不到 /system/fonts）
fn ensure_cjk_fonts(cx: &App) {
    let candidates = [
        "/system/fonts/NotoSansCJK-Regular.ttc",
        "/system/fonts/NotoSerifCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ];
    let mut fonts: Vec<std::borrow::Cow<'static, [u8]>> = Vec::new();
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.push(std::borrow::Cow::Owned(bytes));
        }
    }
    if !fonts.is_empty() {
        if let Err(e) = cx.text_system().add_fonts(fonts) {
            tracing::warn!("failed to register CJK fonts: {}", e);
        } else {
            tracing::info!("registered CJK fonts");
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("kafka_manager_gpui=info")),
        )
        .init();

    // 启动后端（tokio runtime 线程）
    let backend = kafka_manager_gpui::backend::Backend::start();

    gpui_platform::application()
        .with_assets(kafka_manager_gpui::icons::EmbeddedAssets)
        .run(move |cx: &mut App| {
            ensure_cjk_fonts(cx);
            cx.activate(true);
            cx.on_action(quit);
            cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);
            cx.set_menus([Menu::new("Kafka Manager").items([MenuItem::action(
                "Quit Kafka Manager",
                Quit,
            )])]);

            // 输入组件键绑定
            kafka_manager_gpui::widgets::text_input::register_keybindings(cx);
            kafka_manager_gpui::widgets::text_area::register_keybindings(cx);
            kafka_manager_gpui::views::navigator::register_keybindings(cx);
            kafka_manager_gpui::views::messages::register_keybindings(cx);

            // 全局浮层
            kafka_manager_gpui::overlay::init(cx);
            cx.set_global(kafka_manager_gpui::app::BackendGlobal(backend.clone()));

            let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| KafkaManagerApp::new(window, cx));
                    cx.set_global(kafka_manager_gpui::app::RootHandle(view.downgrade()));
                    view.update(cx, |app, cx| app.start(window, cx));
                    view
                },
            )
            .expect("failed to open window");
        });
}
