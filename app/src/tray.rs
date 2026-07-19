//! 系统托盘：图标 + 显示/退出菜单

use tray_icon::menu::{Menu, MenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

static ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

pub const MENU_ID_SHOW: &str = "show";
pub const MENU_ID_QUIT: &str = "quit";

fn load_icon() -> Option<tray_icon::Icon> {
    let img = image::load_from_memory(ICON_PNG).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).ok()
}

/// 创建系统托盘图标（返回 TrayIcon 句柄，调用方需保持存活）
pub fn create_tray() -> Option<TrayIcon> {
    let icon = load_icon()?;

    let show_item = MenuItem::with_id(MENU_ID_SHOW, "显示", true, None);
    let quit_item = MenuItem::with_id(MENU_ID_QUIT, "退出", true, None);
    let menu = Menu::with_items(&[&show_item, &quit_item]).ok()?;

    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Kafka Manager")
        .with_icon(icon)
        .build()
        .map_err(|e| {
            tracing::warn!("Failed to create tray icon: {}", e);
            e
        })
        .ok()
}

thread_local! {
    /// 全局托盘句柄（仅主线程访问；TrayIcon 非 Send，必须驻留在创建它的线程上）
    static TRAY: std::cell::RefCell<Option<TrayIcon>> = const { std::cell::RefCell::new(None) };
}

/// 启用/禁用托盘（必须在主线程调用）
pub fn set_tray_enabled(enabled: bool) {
    TRAY.with(|tray| {
        let mut guard = tray.borrow_mut();
        if enabled && guard.is_none() {
            *guard = create_tray();
        } else if !enabled {
            *guard = None;
        }
    });
}

/// 轮询托盘菜单事件，返回动作（"show" / "quit"）
pub fn poll_menu_event() -> Option<String> {
    tray_icon::menu::MenuEvent::receiver()
        .try_recv()
        .ok()
        .map(|event| event.id.0)
}
