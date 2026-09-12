//! 通用 UI 构件：按钮、徽章、开关、加载圈、分隔线、空态等纯函数 + Select 下拉实体。

use gpui::prelude::*;
use gpui::{
    anchored, deferred, div, px, Animation, AnimationExt, AnyElement, App, Context, ElementId,
    Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, SharedString, Stateful, Styled, Transformation, Window,
};

use crate::icons::icon;
use crate::theme;

// ==================== 按钮 ====================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BtnKind {
    Primary,
    Secondary,
    Outline,
    Ghost,
    Error,
    ErrorOutline,
    WarningOutline,
    Neutral,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BtnSize {
    Xs,
    Sm,
    Md,
}

/// 主题化按钮。调用方链式 `.child(...)`、`.on_click(...)`、`.when(disabled, ...)`。
pub fn btn(id: impl Into<ElementId>, kind: BtnKind, size: BtnSize) -> Stateful<gpui::Div> {
    let mut b = div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(4.))
        .rounded(px(6.))
        .cursor_pointer()
        .whitespace_nowrap()
        .font_weight(gpui::FontWeight::MEDIUM);

    b = match size {
        BtnSize::Xs => b.h(px(24.)).px(px(8.)).text_size(px(11.)),
        BtnSize::Sm => b.h(px(28.)).px(px(10.)).text_size(px(12.)),
        BtnSize::Md => b.h(px(34.)).px(px(14.)).text_size(px(13.)),
    };

    match kind {
        BtnKind::Primary => b
            .bg(theme::gradient_1())
            .text_color(gpui::white())
            .hover(|s| s.bg(theme::btn_primary_hover_1())),
        BtnKind::Secondary => b
            .bg(theme::btn_secondary_1())
            .text_color(gpui::white())
            .hover(|s| s.bg(theme::btn_secondary_2())),
        BtnKind::Outline => b
            .bg(theme::btn_ghost_hover())
            .text_color(theme::text_primary())
            .hover(|s| s.bg(theme::base_content_alpha(0.15))),
        BtnKind::Ghost => b
            .text_color(theme::text_primary())
            .hover(|s| s.bg(theme::btn_ghost_hover())),
        BtnKind::Error => b
            .bg(theme::error())
            .text_color(gpui::white())
            .hover(|s| s.bg(theme::error())),
        BtnKind::ErrorOutline => b
            .border_1()
            .border_color(theme::error())
            .text_color(theme::error())
            .hover(|s| s.bg(theme::error())),
        BtnKind::WarningOutline => b
            .border_1()
            .border_color(theme::warning())
            .text_color(theme::warning())
            .hover(|s| s.bg(theme::warning())),
        BtnKind::Neutral => b
            .bg(theme::base_300())
            .text_color(theme::text_primary())
            .hover(|s| s.bg(theme::base_content_alpha(0.15))),
    }
}

/// 图标按钮（ghost 风格）
pub fn icon_btn(id: impl Into<ElementId>, icon_name: &'static str, size: BtnSize) -> Stateful<gpui::Div> {
    let (box_size, icon_size) = match size {
        BtnSize::Xs => (24., 13.),
        BtnSize::Sm => (28., 15.),
        BtnSize::Md => (34., 17.),
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(box_size))
        .rounded(px(6.))
        .cursor_pointer()
        .text_color(theme::text_secondary())
        .hover(|s| s.bg(theme::btn_ghost_hover()).text_color(theme::text_primary()))
        .child(icon(icon_name).size(px(icon_size)))
}

/// 禁用态包装（透明度 + 不响应）
pub fn disabled_style<E: Styled>(el: E, disabled: bool) -> E {
    if disabled {
        el.opacity(0.5)
    } else {
        el
    }
}

// ==================== 徽章 ====================

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BadgeKind {
    Primary,
    Secondary,
    Success,
    Warning,
    Error,
    Info,
    Ghost,
    Accent,
}

pub fn badge(label: impl Into<SharedString>, kind: BadgeKind) -> gpui::Div {
    let (bg, fg) = match kind {
        BadgeKind::Primary => (theme::badge_primary_bg(), theme::badge_primary_text()),
        BadgeKind::Secondary => (theme::badge_secondary_bg(), theme::badge_secondary_text()),
        BadgeKind::Success => (theme::success(), theme::base_100()),
        BadgeKind::Warning => (theme::warning(), theme::warning_content()),
        BadgeKind::Error => (theme::error(), gpui::white()),
        BadgeKind::Info => (theme::info(), theme::info_content()),
        BadgeKind::Accent => (theme::accent(), theme::accent_content()),
        BadgeKind::Ghost => (theme::base_content_alpha(0.1), theme::text_secondary()),
    };
    div()
        .flex_none()
        .px(px(6.))
        .h(px(18.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .text_size(px(10.))
        .bg(bg)
        .text_color(fg)
        .child(label.into())
}

// ==================== 开关 ====================

pub fn toggle(id: impl Into<ElementId>, on: bool) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .w(px(34.))
        .h(px(19.))
        .rounded(px(10.))
        .cursor_pointer()
        .bg(if on { theme::primary() } else { theme::base_content_alpha(0.25) })
        .p(px(2.))
        .child(
            div()
                .size(px(15.))
                .rounded(px(8.))
                .bg(gpui::white())
                .when(on, |d| d.ml(px(15.))),
        )
}

// ==================== 加载圈 ====================

/// 旋转 spinner（动画旋转 refresh 图标）
pub fn spinner(size: f32) -> AnyElement {
    icon("refresh")
        .size(px(size))
        .text_color(theme::primary_alpha(0.9))
        .with_animation(
            ("km-spinner", size as usize),
            Animation::new(std::time::Duration::from_millis(900))
                .repeat()
                .with_easing(gpui::linear),
            move |el, t| el.with_transformation(Transformation::rotate(gpui::percentage(t))),
        )
        .into_any_element()
}

/// 居中 spinner + 文案
pub fn loading_block(label: impl Into<SharedString>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .size_full()
        .child(spinner(22.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::text_secondary())
                .child(label.into()),
        )
}

// ==================== 空态 ====================

pub fn empty_block(icon_name: &'static str, title: impl Into<SharedString>, desc: impl Into<SharedString>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .size_full()
        .child(
            icon(icon_name)
                .size(px(48.))
                .text_color(theme::base_content_alpha(0.25)),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme::text_secondary())
                .child(title.into()),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::base_content_alpha(0.45))
                .child(desc.into()),
        )
}

// ==================== 卡片 ====================

/// glass 卡片容器
pub fn card() -> gpui::Div {
    div()
        .bg(theme::glass_bg())
        .border_1()
        .border_color(theme::glass_border())
        .rounded(px(10.))
        .shadow_lg()
}

// ==================== 表单 ====================

/// label + 控件
pub fn field(label: impl Into<SharedString>, control: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(11.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme::text_secondary())
                .child(label.into()),
        )
        .child(control)
}

/// 输入框容器（包裹 TextInput 实体）
pub fn input_frame(input: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .w_full()
        .h(px(30.))
        .px(px(8.))
        .rounded(px(6.))
        .bg(theme::input_bg())
        .border_1()
        .border_color(theme::base_content_alpha(0.15))
        .text_size(px(12.))
        .child(input)
}

// ==================== Select 下拉 ====================

#[derive(Clone, Debug)]
pub struct SelectOption {
    pub value: SharedString,
    pub label: SharedString,
}

#[derive(Clone, Debug)]
pub struct SelectEvent {
    pub value: SharedString,
}

pub struct Select {
    pub options: Vec<SelectOption>,
    pub value: SharedString,
    pub placeholder: SharedString,
    pub disabled: bool,
    focus_handle: FocusHandle,
}

impl Select {
    pub fn new(options: Vec<SelectOption>, value: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            options,
            value: value.into(),
            placeholder: "请选择".into(),
            disabled: false,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_options(&mut self, options: Vec<SelectOption>, cx: &mut Context<Self>) {
        self.options = options;
        cx.notify();
    }

    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.value = value.into();
        cx.notify();
    }

    pub fn label(&self) -> SharedString {
        self.options
            .iter()
            .find(|o| o.value == self.value)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| self.placeholder.clone())
    }

    /// 由全局 overlay 在选择后回调
    pub fn choose(&mut self, value: SharedString, cx: &mut Context<Self>) {
        self.value = value.clone();
        cx.emit(SelectEvent { value });
        cx.notify();
    }
}

impl EventEmitter<SelectEvent> for Select {}

impl Focusable for Select {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Select {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = self.label();
        let disabled = self.disabled;
        div()
            .id("km-select")
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.))
            .h(px(30.))
            .pl(px(8.))
            .pr(px(4.))
            .w_full()
            .rounded(px(6.))
            .bg(theme::input_bg())
            .border_1()
            .border_color(theme::base_content_alpha(0.15))
            .text_size(px(12.))
            .when(disabled, |d| d.opacity(0.5))
            .when(!disabled, |d| {
                d.cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_this, event: &gpui::MouseDownEvent, _window, cx| {
                        let entity = cx.entity();
                        let position = event.position;
                        crate::overlay::open_dropdown(cx, position, entity);
                    }),
                )
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(theme::text_primary())
                    .child(label),
            )
            .child(
                icon("chevron-down")
                    .size(px(13.))
                    .text_color(theme::text_secondary()),
            )
    }
}

/// 下拉弹层（由根视图渲染，deferred 置顶）
pub fn dropdown_popup(
    select: Entity<Select>,
    position: gpui::Point<gpui::Pixels>,
    cx: &mut App,
) -> AnyElement {
    let options = select.read(cx).options.clone();
    let value = select.read(cx).value.clone();
    let item_count = options.len();
    let menu_height = (item_count as f32 * 30.0 + 10.0).min(320.0);
    deferred(
        anchored()
            .anchor(gpui::Anchor::TopLeft)
            .position(position)
            .snap_to_window_with_margin(px(8.))
            .child(
                div()
                    .id("km-dropdown-popup")
                    .w(px(200.))
                    .h(px(menu_height))
                    .flex()
                    .flex_col()
                    .p(px(4.))
                    .gap(px(2.))
                    .bg(theme::context_menu_bg())
                    .border_1()
                    .border_color(theme::glass_border())
                    .rounded(px(8.))
                    .shadow_lg()
                    .overflow_y_scroll()
                    .children(options.into_iter().enumerate().map(|(ix, opt)| {
                        let selected = opt.value == value;
                        let sel = select.clone();
                        div()
                            .id(("km-dropdown-item", ix))
                            .flex()
                            .items_center()
                            .justify_between()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(5.))
                            .cursor_pointer()
                            .text_size(px(12.))
                            .text_color(if selected {
                                theme::badge_primary_text()
                            } else {
                                theme::text_primary()
                            })
                            .when(selected, |d| d.bg(theme::badge_primary_bg()))
                            .hover(|s| s.bg(theme::context_menu_item_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                move |_event, _window, cx| {
                                    sel.update(cx, |s, cx| s.choose(opt.value.clone(), cx));
                                    crate::overlay::close_dropdown(cx);
                                },
                            )
                            .child(opt.label)
                            .when(selected, |d| {
                                d.child(
                                    icon("check")
                                        .size(px(12.))
                                        .text_color(theme::badge_primary_text()),
                                )
                            })
                    })),
            ),
    )
    .with_priority(100)
    .into_any_element()
}
