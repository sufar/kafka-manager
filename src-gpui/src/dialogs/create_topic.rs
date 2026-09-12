//! 创建 Topic 对话框（对齐 CreateTopicDialog.vue：字段、校验、高级选项）

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

pub struct CreateTopicDialog {
    cluster: String,
    name: Entity<TextInput>,
    partitions: Entity<TextInput>,
    replication: Entity<TextInput>,
    cleanup_policy: Entity<Select>,
    retention_ms: Entity<TextInput>,
    retention_bytes: Entity<TextInput>,
    segment_bytes: Entity<TextInput>,
    advanced_open: bool,
    submitting: bool,
    focus_handle: FocusHandle,
}

impl CreateTopicDialog {
    pub fn new(cluster: String, cx: &mut Context<Self>) -> Self {
        let name = cx.new(TextInput::new);
        name.update(cx, |i, _| i.set_placeholder(t("topics.namePlaceholder")));
        let partitions = cx.new(TextInput::new);
        partitions.update(cx, |i, cx| i.set_text("3", cx));
        let replication = cx.new(TextInput::new);
        replication.update(cx, |i, cx| i.set_text("1", cx));
        let cleanup_policy = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption { value: "delete".into(), label: "delete".into() },
                    SelectOption { value: "compact".into(), label: "compact".into() },
                    SelectOption { value: "delete,compact".into(), label: "delete,compact".into() },
                ],
                "delete",
                cx,
            )
        });
        let retention_ms = cx.new(TextInput::new);
        retention_ms.update(cx, |i, _| i.set_placeholder("604800000 (7 天)"));
        let retention_bytes = cx.new(TextInput::new);
        retention_bytes.update(cx, |i, _| i.set_placeholder("-1 (无限制)"));
        let segment_bytes = cx.new(TextInput::new);
        segment_bytes.update(cx, |i, _| i.set_placeholder("1073741824 (1GB)"));

        // 输入框 Escape 关闭对话框
        for input in [&name, &partitions, &replication, &retention_ms, &retention_bytes, &segment_bytes] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }
        // 回车提交
        cx.subscribe(&name, |this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EnterPressed) {
                this.submit(cx);
            }
        })
        .detach();

        Self {
            cluster,
            name,
            partitions,
            replication,
            cleanup_policy,
            retention_ms,
            retention_bytes,
            segment_bytes,
            advanced_open: false,
            submitting: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.submitting {
            return;
        }
        // ---- 校验（顺序与 Vue 一致）----
        if self.cluster.is_empty() {
            overlay::toast_error(cx, "集群 ID 不能为空");
            return;
        }
        let name = self.name.read(cx).text().trim().to_string();
        if name.is_empty() {
            overlay::toast_error(cx, "主题名称不能为空");
            return;
        }
        if name.len() > 256 {
            overlay::toast_error(cx, "主题名称不能超过 256 个字符");
            return;
        }
        if name.contains([' ', '"', '\'', ',']) {
            overlay::toast_error(cx, "主题名称不能包含空格、引号或逗号");
            return;
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
        {
            overlay::toast_error(cx, "主题名称只能包含字母、数字、点号、下划线和短横线");
            return;
        }
        let num_partitions: i64 = self
            .partitions
            .read(cx)
            .text()
            .trim()
            .parse()
            .unwrap_or(3)
            .clamp(1, 100);
        let replication_factor: i64 = self
            .replication
            .read(cx)
            .text()
            .trim()
            .parse()
            .unwrap_or(1)
            .clamp(1, 10);

        let mut config = serde_json::Map::new();
        if self.advanced_open {
            config.insert(
                "cleanup.policy".into(),
                serde_json::json!(self.cleanup_policy.read(cx).value.to_string()),
            );
            let rms = self.retention_ms.read(cx).text().trim().to_string();
            if !rms.is_empty() {
                match rms.parse::<i64>() {
                    Ok(v) if v >= 0 => {
                        config.insert("retention.ms".into(), serde_json::json!(v.to_string()));
                    }
                    _ => {
                        overlay::toast_error(cx, "retention.ms 必须是正数");
                        return;
                    }
                }
            }
            let rbytes = self.retention_bytes.read(cx).text().trim().to_string();
            if !rbytes.is_empty() {
                match rbytes.parse::<i64>() {
                    Ok(_) => {
                        config.insert("retention.bytes".into(), serde_json::json!(rbytes));
                    }
                    _ => {
                        overlay::toast_error(cx, "retention.bytes 必须是数字（使用 -1 表示无限制）");
                        return;
                    }
                }
            }
            let sbytes = self.segment_bytes.read(cx).text().trim().to_string();
            if !sbytes.is_empty() {
                match sbytes.parse::<i64>() {
                    Ok(v) if v >= 0 => {
                        config.insert("segment.bytes".into(), serde_json::json!(v.to_string()));
                    }
                    _ => {
                        overlay::toast_error(cx, "segment.bytes 必须是正数");
                        return;
                    }
                }
            }
        }

        self.submitting = true;
        cx.notify();
        let b = backend(cx);
        let cluster = self.cluster.clone();
        let cluster_for_event = cluster.clone();
        let name_for_toast = name.clone();
        let mut params = serde_json::json!({
            "cluster_id": cluster,
            "name": name,
            "num_partitions": num_partitions,
            "replication_factor": replication_factor,
        });
        if !config.is_empty() {
            params["config"] = serde_json::Value::Object(config);
        }
        cx.spawn(async move |this, cx| {
            let result = b.dispatch("topic.create", params).await;
            this.update(cx, |this, cx| {
                this.submitting = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(
                            cx,
                            format!("主题 \"{}\" 创建成功", name_for_toast),
                        );
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            app.publish(
                                AppEvent::TopicCreated {
                                    cluster: cluster_for_event,
                                },
                                cx,
                            );
                        });
                    }
                    Err(e) => {
                        overlay::toast_error(cx, format!("创建主题失败: {}", e));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Focusable for CreateTopicDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CreateTopicDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let name_display = {
            let n = self.name.read(cx).text();
            if n.is_empty() {
                t("topics.namePlaceholder")
            } else {
                n
            }
        };

        div()
            .w(px(560.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .on_mouse_down(MouseButton::Left, |_, _, _| {})
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
                            .child(
                                icon("database")
                                    .size(px(18.))
                                    .text_color(gpui::white()),
                            ),
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
                                    .child(t("topics.create")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_family("monospace")
                                    .text_color(theme::text_secondary())
                                    .child(name_display),
                            ),
                    )
                    .child(
                        icon_btn("close-create-topic", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            // 名称
            .child(field(
                format!("{} *", t("topics.topicName")),
                input_frame(self.name.clone()).into_any_element(),
            ))
            // 分区数 + 副本因子
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(
                        div().flex_1().child(field(
                            format!("{} (1-100)", t("topics.numPartitions")),
                            input_frame(self.partitions.clone()).into_any_element(),
                        )),
                    )
                    .child(
                        div().flex_1().child(field(
                            format!("{} (1-10)", t("topics.replicationFactor")),
                            input_frame(self.replication.clone()).into_any_element(),
                        )),
                    ),
            )
            // 高级选项折叠
            .child(
                div()
                    .id("advanced-toggle")
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(32.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme::btn_ghost_hover()))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.advanced_open = !this.advanced_open;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(theme::text_primary())
                            .child(
                                icon(if self.advanced_open {
                                    "chevron-down"
                                } else {
                                    "chevron-right"
                                })
                                .size(px(12.))
                                .text_color(theme::text_secondary()),
                            )
                            .child(t("topics.advancedOptions")),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::text_secondary())
                            .child(if self.advanced_open {
                                t("common.collapse")
                            } else {
                                t("common.expand")
                            }),
                    ),
            )
            .when(self.advanced_open, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .p(px(10.))
                        .rounded(px(8.))
                        .bg(theme::base_content_alpha(0.05))
                        .child(field(
                            "cleanup.policy",
                            self.cleanup_policy.clone().into_any_element(),
                        ))
                        .child(field(
                            "retention.ms",
                            input_frame(self.retention_ms.clone()).into_any_element(),
                        ))
                        .child(field(
                            "retention.bytes",
                            input_frame(self.retention_bytes.clone()).into_any_element(),
                        ))
                        .child(field(
                            "segment.bytes",
                            input_frame(self.segment_bytes.clone()).into_any_element(),
                        )),
                )
            })
            // 底部按钮
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .pt(px(4.))
                    .child(
                        btn("create-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("create-submit", BtnKind::Primary, BtnSize::Sm)
                            .when(self.submitting, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(cx)),
                            )
                            .child(if self.submitting {
                                spinner(13.)
                            } else {
                                icon("plus")
                                    .size(px(13.))
                                    .text_color(gpui::white())
                                    .into_any_element()
                            })
                            .child(t("common.create")),
                    ),
            )
    }
}
