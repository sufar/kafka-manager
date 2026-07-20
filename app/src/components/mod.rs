//! 共享 UI 组件

use gpui::App;
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::WindowExt;

pub mod navigator;
pub mod option_select;
pub mod tree_navigator;

/// 向所有窗口推送通知（异步回调中使用）
pub fn notify(cx: &mut App, note_type: NotificationType, text: String) {
    for w in cx.windows() {
        let text = text.clone();
        let _ = w.update(cx, |_, window, cx| {
            window.push_notification(
                Notification::new().message(text).with_type(note_type),
                cx,
            );
        });
    }
}
