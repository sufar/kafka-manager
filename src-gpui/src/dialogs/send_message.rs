//! 发送消息对话框（对齐 SendMessageModal.vue）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    WeakEntity, Window,
};

use crate::app::backend;
use crate::i18n::t;
use crate::icons::icon;
use crate::json::{format_json, is_valid_json};
use crate::overlay;
use crate::theme;
use crate::views::messages::MessagesView;
use crate::widgets::common::*;
use crate::widgets::text_area::{TextArea, TextAreaEvent};
use crate::widgets::text_input::{TextInput, TextInputEvent};

struct HeaderRow {
    key: Entity<TextInput>,
    value: Entity<TextInput>,
}

pub struct SendMessageDialog {
    cluster: String,
    topic: String,
    #[allow(dead_code)]
    partitions: Vec<i32>,
    partition_select: Entity<Select>,
    key_input: Entity<TextInput>,
    value_editor: Entity<TextArea>,
    headers_open: bool,
    header_rows: Vec<HeaderRow>,
    sending: bool,
    last_offset: Option<i64>,
    keep_open: bool,
    source: WeakEntity<MessagesView>,
    focus_handle: FocusHandle,
}

impl SendMessageDialog {
    pub fn new(
        cluster: String,
        topic: String,
        #[allow(dead_code)]
    partitions: Vec<i32>,
        initial_partition: Option<i32>,
        initial_key: Option<String>,
        initial_value: Option<String>,
        source: WeakEntity<MessagesView>,
        cx: &mut Context<Self>,
    ) -> Self {
        let default_partition = initial_partition
            .or_else(|| partitions.first().copied())
            .unwrap_or(0);
        let partition_select = cx.new(|cx| {
            Select::new(
                partitions
                    .iter()
                    .map(|p| SelectOption {
                        value: p.to_string().into(),
                        label: format!("Partition {}", p).into(),
                    })
                    .collect(),
                default_partition.to_string(),
                cx,
            )
        });
        let key_input = cx.new(TextInput::new);
        key_input.update(cx, |i, _| i.set_placeholder(t("messages.optional")));
        if let Some(k) = initial_key {
            key_input.update(cx, |i, cx| i.set_text(k, cx));
        }
        let value_editor = cx.new(TextArea::new);
        value_editor.update(cx, |a, _| {
            a.set_placeholder(r#"{"key": "value"}"#);
        });
        if let Some(v) = initial_value {
            let formatted = if is_valid_json(&v) { format_json(&v) } else { v };
            value_editor.update(cx, |a, cx| a.set_text(formatted, cx));
        }

        cx.subscribe(&key_input, |_, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();
        cx.subscribe(&value_editor, |_, _, event: &TextAreaEvent, cx| {
            if matches!(event, TextAreaEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();

        Self {
            cluster,
            topic,
            partitions,
            partition_select,
            key_input,
            value_editor,
            headers_open: false,
            header_rows: vec![],
            sending: false,
            last_offset: None,
            keep_open: false,
            source,
            focus_handle: cx.focus_handle(),
        }
    }

    fn format_value(&self, cx: &mut Context<Self>) {
        self.value_editor.update(cx, |a, cx| {
            let formatted = format_json(&a.text());
            a.set_text(formatted, cx);
        });
    }

    fn add_header_row(&mut self, cx: &mut Context<Self>) {
        let key = cx.new(TextInput::new);
        key.update(cx, |i, _| i.set_placeholder("key"));
        let value = cx.new(TextInput::new);
        value.update(cx, |i, _| i.set_placeholder("value"));
        cx.subscribe(&key, |_, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();
        cx.subscribe(&value, |_, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();
        self.header_rows.push(HeaderRow { key, value });
        cx.notify();
    }

    fn submit(&mut self, keep_open: bool, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        let partition: i32 = self
            .partition_select
            .read(cx)
            .value
            .parse()
            .unwrap_or(0);
        let key = self.key_input.read(cx).text();
        let key = if key.is_empty() { None } else { Some(key) };
        let value = self.value_editor.read(cx).text();
        if value.is_empty() {
            overlay::toast_error(cx, t("messages.valueRequired"));
            return;
        }
        let mut headers = serde_json::Map::new();
        for row in &self.header_rows {
            let k = row.key.read(cx).text();
            let v = row.value.read(cx).text();
            if !k.is_empty() && !v.is_empty() {
                headers.insert(k, serde_json::json!(v));
            }
        }

        self.sending = true;
        self.keep_open = keep_open;
        cx.notify();
        let b = backend(cx);
        let (cluster, topic) = (self.cluster.clone(), self.topic.clone());
        let mut params = serde_json::json!({
            "cluster_id": cluster,
            "topic": topic,
            "partition": partition,
            "value": value,
        });
        if let Some(k) = key {
            params["key"] = serde_json::json!(k);
        }
        if !headers.is_empty() {
            params["headers"] = serde_json::Value::Object(headers);
        }
        cx.spawn(async move |this, cx| {
            let result = b.dispatch("message.send", params).await;
            this.update(cx, |this, cx| {
                this.sending = false;
                match result {
                    Ok(v) => {
                        let offset = v.get("offset").and_then(|x| x.as_i64()).unwrap_or(0);
                        this.last_offset = Some(offset);
                        overlay::toast_success(
                            cx,
                            format!("{} Offset: {}", t("messages.sendSuccess"), offset),
                        );
                        if !this.keep_open {
                            overlay::close_modal(cx);
                        }
                        // 通知消息页重新查询
                        if let Some(view) = this.source.upgrade() {
                            view.update(cx, |view, cx| view.on_message_sent(cx));
                        }
                    }
                    Err(e) => overlay::toast_error(cx, e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Focusable for SendMessageDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SendMessageDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let headers_count = self.header_rows.len();

        div().id("dialogs_send_message_rs_1")
            .w(px(720.))
            .max_h(px(640.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .overflow_y_scroll()
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
                            .bg(theme::gradient_1())
                            .child(icon("send").size(px(18.)).text_color(gpui::white())),
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
                                    .child(t("messages.sendMessage")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_family("monospace")
                                    .text_color(theme::text_secondary())
                                    .child(self.topic.clone()),
                            ),
                    )
                    .child(
                        icon_btn("close-send-message", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            // 成功提示条
            .when_some(self.last_offset, |d, offset| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .bg(theme::success())
                        .child(icon("check-circle").size(px(14.)).text_color(gpui::white()))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(gpui::white())
                                .child(format!("{} Offset: {}", t("messages.sendSuccess"), offset)),
                        ),
                )
            })
            // Partition + Key
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(
                        div().w(px(180.)).child(field(
                            "Partition",
                            self.partition_select.clone().into_any_element(),
                        )),
                    )
                    .child(
                        div().flex_1().child(field(
                            "Key",
                            input_frame(self.key_input.clone()).into_any_element(),
                        )),
                    ),
            )
            // Value
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme::text_secondary())
                                    .child(format!("Value *")),
                            )
                            .child(
                                btn("format-value", BtnKind::Ghost, BtnSize::Xs)
                                    .child(t("messages.format"))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.format_value(cx)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .h(px(240.))
                            .rounded(px(6.))
                            .bg(theme::input_bg())
                            .border_1()
                            .border_color(theme::base_content_alpha(0.15))
                            .font_family("monospace")
                            .text_size(px(12.))
                            .p(px(6.))
                            .child(self.value_editor.clone()),
                    ),
            )
            // Headers 折叠
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .id("headers-toggle")
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .h(px(28.))
                            .px(px(6.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::btn_ghost_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.headers_open = !this.headers_open;
                                    cx.notify();
                                }),
                            )
                            .child(
                                icon(if self.headers_open {
                                    "chevron-down"
                                } else {
                                    "chevron-right"
                                })
                                .size(px(12.))
                                .text_color(theme::text_secondary()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme::text_primary())
                                    .child("Headers"),
                            )
                            .when(headers_count > 0, |d| {
                                d.child(badge(headers_count.to_string(), BadgeKind::Primary))
                            }),
                    )
                    .when(self.headers_open, |d| {
                        let mut panel = div().flex().flex_col().gap(px(6.));
                        for (ix, _row) in self.header_rows.iter().enumerate() {
                            let (key, value) = {
                                let row = &self.header_rows[ix];
                                (row.key.clone(), row.value.clone())
                            };
                            panel = panel.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(div().flex_1().child(input_frame(key)))
                                    .child(div().flex_1().child(input_frame(value)))
                                    .child(
                                        icon_btn(("header-del", ix), "x-mark", BtnSize::Xs)
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.header_rows.remove(ix);
                                                    cx.notify();
                                                }),
                                            ),
                                    ),
                            );
                        }
                        d.child(panel.child(
                            btn("add-header", BtnKind::Ghost, BtnSize::Xs)
                                .child(icon("plus").size(px(12.)).into_any_element())
                                .child(t("common.add"))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| this.add_header_row(cx)),
                                ),
                        ))
                    }),
            )
            // 底部按钮
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .pt(px(4.))
                    .child(
                        btn("send-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("send-continue", BtnKind::Outline, BtnSize::Sm)
                            .when(self.sending, |b| b.opacity(0.6))
                            .child(t("messages.sendAndContinue"))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(true, cx)),
                            ),
                    )
                    .child(
                        btn("send-submit", BtnKind::Primary, BtnSize::Sm)
                            .when(self.sending, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(false, cx)),
                            )
                            .child(if self.sending {
                                spinner(13.)
                            } else {
                                icon("send").size(px(13.)).into_any_element()
                            })
                            .child(if self.sending {
                                t("messages.sending")
                            } else {
                                t("messages.send")
                            }),
                    ),
            )
    }
}
