//! 删除 Topic 对话框（对齐 DeleteTopicDialog.vue：需输入完整主题名确认）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    Window,
};

use crate::app::{backend, root, AppEvent};
use crate::i18n::t;
use crate::icons::icon;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::text_input::{TextInput, TextInputEvent};

pub struct DeleteTopicDialog {
    cluster: String,
    topic: String,
    confirm_input: Entity<TextInput>,
    deleting: bool,
    focus_handle: FocusHandle,
}

impl DeleteTopicDialog {
    pub fn new(cluster: String, topic: String, cx: &mut Context<Self>) -> Self {
        let confirm_input = cx.new(TextInput::new);
        confirm_input.update(cx, |i, _| i.set_placeholder(topic.clone()));
        cx.subscribe(&confirm_input, |_, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();
        Self {
            cluster,
            topic,
            confirm_input,
            deleting: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn matches(&self, cx: &App) -> bool {
        self.confirm_input.read(cx).text().trim() == self.topic
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.deleting || !self.matches(cx) {
            if !self.matches(cx) {
                overlay::toast_error(cx, "输入的主题名称不匹配");
            }
            return;
        }
        self.deleting = true;
        cx.notify();
        let b = backend(cx);
        let cluster = self.cluster.clone();
        let topic = self.topic.clone();
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "topic.delete",
                    serde_json::json!({"cluster_id": cluster, "topic": topic}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.deleting = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, "主题已删除");
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            app.publish(
                                AppEvent::TopicDeleted {
                                    cluster,
                                    topic,
                                },
                                cx,
                            );
                        });
                    }
                    Err(e) => overlay::toast_error(cx, e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn copy_name(&self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(self.topic.clone()));
        overlay::toast_success(cx, t("common.copied"));
    }
}

impl Focusable for DeleteTopicDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DeleteTopicDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let matches = self.matches(cx);

        div()
            .w(px(420.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            // 头部
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(36.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme::error())
                            .child(icon("trash").size(px(18.)).text_color(gpui::white())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme::text_primary())
                                    .child(t("topics.delete")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child(t("topics.deleteIrreversible")),
                            ),
                    )
                    .child(
                        icon_btn("close-delete-topic", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            // 信息区
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .p(px(12.))
                    .rounded(px(8.))
                    .bg(theme::base_content_alpha(0.06))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child("Cluster"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme::text_primary())
                                    .child(self.cluster.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child(t("topics.topicName")),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_size(px(12.))
                                    .font_family("monospace")
                                    .text_color(theme::text_primary())
                                    .child(self.topic.clone()),
                            )
                            .child(
                                icon_btn("copy-topic-name", "clipboard", BtnSize::Xs)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.copy_name(cx)),
                                    ),
                            ),
                    ),
            )
            // 确认输入
            .child(field(
                t("topics.typeNameToConfirm"),
                input_frame(self.confirm_input.clone()).into_any_element(),
            ))
            // 按钮
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .pt(px(4.))
                    .child(
                        btn("delete-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("delete-submit", BtnKind::Error, BtnSize::Sm)
                            .when(!matches || self.deleting, |b| b.opacity(0.5))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(cx)),
                            )
                            .child(if self.deleting {
                                spinner(13.)
                            } else {
                                icon("trash").size(px(13.)).into_any_element()
                            })
                            .child(t("common.delete")),
                    ),
            )
    }
}
