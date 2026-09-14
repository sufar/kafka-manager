//! 页面头部「返回上一页」按钮（与旧版 Vue 各视图标题行左侧的返回按钮一致）
//!
//! 从全局 [`CanGoBack`] 读取可用状态（由 Workspace 维护），点击发出
//! [`NavEvent::GoBack`]，由 Workspace 弹出返回栈。

use gpui::{App, Context, EventEmitter, Global};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Disableable, IconName, Sizable};

use crate::components::navigator::NavEvent;
use crate::i18n::t;

/// 全局「能否返回」状态（Workspace 在返回栈变化时写入）
#[derive(Clone, Copy, Default)]
pub struct CanGoBack(pub bool);

impl Global for CanGoBack {}

/// 读取当前能否返回（未初始化时视为 false）
pub fn can_go_back(cx: &App) -> bool {
    cx.try_global::<CanGoBack>().map(|g| g.0).unwrap_or(false)
}

/// 页头返回按钮：ghost 小箭头，栈空时禁用，点击 emit GoBack
///
/// 只需 `&Context`（listener 本身不可变借用），避免与 render 里的 `cx.theme()` 借用冲突
pub fn back_button<V>(cx: &Context<V>) -> Button
where
    V: EventEmitter<NavEvent>,
{
    Button::new("nav-back")
        .ghost()
        .xsmall()
        .icon(IconName::ArrowLeft)
        .tooltip(t(cx, "common.back"))
        .disabled(!can_go_back(cx))
        .on_click(cx.listener(|_, _, _, cx| {
            cx.emit(NavEvent::GoBack);
        }))
}
