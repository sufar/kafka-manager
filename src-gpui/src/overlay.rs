//! 全局浮层管理：dropdown / context menu / modal / confirm / toast。
//! 存放为 gpui Global，根视图 `observe_global` 后在最顶层渲染。
//! 由于本 rev 无 update_global（仅 set_global 会通知观察者），统一走 `update_overlays`。

use gpui::{AnyView, App, Entity, Global, Pixels, Point};
use std::time::Duration;

use crate::widgets::common::Select;

#[derive(Default)]
pub struct Overlays {
    pub dropdown: Option<(Point<Pixels>, Entity<Select>)>,
    pub context_menu: Option<ContextMenuState>,
    pub modal: Option<AnyView>,
    pub confirm: Option<ConfirmState>,
    pub toasts: Vec<Toast>,
}

impl Global for Overlays {}

pub fn init(cx: &mut App) {
    cx.set_global(Overlays::default());
}

pub fn update_overlays(cx: &mut App, f: impl FnOnce(&mut Overlays)) {
    let mut overlays = std::mem::take(cx.global_mut::<Overlays>());
    f(&mut overlays);
    cx.set_global(overlays);
}

// ==================== Dropdown ====================

pub fn open_dropdown(cx: &mut App, position: Point<Pixels>, select: Entity<Select>) {
    update_overlays(cx, |o| {
        o.dropdown = Some((position, select));
    });
}

pub fn close_dropdown(cx: &mut App) {
    update_overlays(cx, |o| {
        o.dropdown = None;
    });
}

// ==================== Context Menu ====================

pub struct ContextItem {
    pub label: String,
    pub icon: Option<&'static str>,
    pub danger: bool,
    pub action: Box<dyn FnOnce(&mut App)>,
}

pub struct ContextMenuState {
    pub position: Point<Pixels>,
    pub title: Option<String>,
    pub items: Vec<ContextItem>,
    /// 组分隔：在第 i 项前画分隔线
    pub separators: Vec<usize>,
}

pub fn open_context_menu(cx: &mut App, menu: ContextMenuState) {
    update_overlays(cx, |o| {
        o.context_menu = Some(menu);
    });
}

pub fn close_context_menu(cx: &mut App) {
    update_overlays(cx, |o| {
        o.context_menu = None;
    });
}

// ==================== Modal ====================

pub fn open_modal(cx: &mut App, view: AnyView) {
    update_overlays(cx, |o| {
        o.modal = Some(view);
    });
}

pub fn close_modal(cx: &mut App) {
    update_overlays(cx, |o| {
        o.modal = None;
    });
}

pub fn has_modal(cx: &App) -> bool {
    cx.try_global::<Overlays>()
        .map(|o| o.modal.is_some())
        .unwrap_or(false)
}

// ==================== Confirm ====================

pub struct ConfirmState {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub danger: bool,
    pub on_confirm: Option<Box<dyn FnOnce(&mut App)>>,
}

pub fn confirm(
    cx: &mut App,
    title: impl Into<String>,
    message: impl Into<String>,
    danger: bool,
    on_confirm: impl FnOnce(&mut App) + 'static,
) {
    update_overlays(cx, |o| {
        o.confirm = Some(ConfirmState {
            title: title.into(),
            message: message.into(),
            confirm_label: crate::i18n::t("common.confirm"),
            danger,
            on_confirm: Some(Box::new(on_confirm)),
        });
    });
}

pub fn close_confirm(cx: &mut App) {
    update_overlays(cx, |o| {
        o.confirm = None;
    });
}

// ==================== Toast ====================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Warning,
    Info,
}

pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
}

static NEXT_TOAST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn toast(cx: &mut App, kind: ToastKind, message: impl Into<String>, duration_ms: u64) {
    let id = NEXT_TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let message = message.into();
    update_overlays(cx, |o| {
        // 最多保留 3 条（与 Vue 一致）
        if o.toasts.len() >= 3 {
            o.toasts.remove(0);
        }
        o.toasts.push(Toast { id, kind, message });
    });
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(Duration::from_millis(duration_ms))
            .await;
        let _ = cx.update(|cx| {
            update_overlays(cx, |o| {
                o.toasts.retain(|t| t.id != id);
            });
        });
    })
    .detach();
}

pub fn toast_success(cx: &mut App, msg: impl Into<String>) {
    toast(cx, ToastKind::Success, msg, 3000);
}

pub fn toast_error(cx: &mut App, msg: impl Into<String>) {
    toast(cx, ToastKind::Error, msg, 3000);
}

pub fn toast_info(cx: &mut App, msg: impl Into<String>) {
    toast(cx, ToastKind::Info, msg, 3000);
}

pub fn toast_warning(cx: &mut App, msg: impl Into<String>) {
    toast(cx, ToastKind::Warning, msg, 3000);
}
