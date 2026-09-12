//! 集群创建/编辑对话框（对齐 ClustersView 的 cluster modal）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    Window,
};

use crate::app::{backend, root, AppEvent};
use crate::i18n::t;
use crate::icons::icon;
use crate::models::Cluster;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::text_input::{TextInput, TextInputEvent};

pub struct ClusterFormDialog {
    editing: Option<Cluster>,
    name: Entity<TextInput>,
    brokers: Entity<TextInput>,
    request_timeout: Entity<TextInput>,
    operation_timeout: Entity<TextInput>,
    group_select: Entity<Select>,
    submitting: bool,
    testing: bool,
    test_result: Option<(bool, String)>,
    focus_handle: FocusHandle,
}

impl ClusterFormDialog {
    pub fn new(editing: Option<Cluster>, cx: &mut Context<Self>) -> Self {
        let groups = root(cx).read(cx).groups.clone();

        let name = cx.new(TextInput::new);
        name.update(cx, |i, _| i.set_placeholder("my-cluster"));
        let brokers = cx.new(TextInput::new);
        brokers.update(cx, |i, _| i.set_placeholder("localhost:9092,localhost:9093"));
        let request_timeout = cx.new(TextInput::new);
        request_timeout.update(cx, |i, _| i.set_placeholder("30000"));
        let operation_timeout = cx.new(TextInput::new);
        operation_timeout.update(cx, |i, _| i.set_placeholder("30000"));

        let mut options = vec![SelectOption {
            value: "".into(),
            label: t("clusters.noGroup").into(),
        }];
        options.extend(groups.iter().map(|g| SelectOption {
            value: g.id.to_string().into(),
            label: g.name.clone().into(),
        }));
        let group_select = cx.new(|cx| Select::new(options, "", cx));

        if let Some(c) = &editing {
            name.update(cx, |i, cx| i.set_text(c.name.clone(), cx));
            brokers.update(cx, |i, cx| i.set_text(c.brokers.clone(), cx));
            if let Some(v) = c.request_timeout_ms {
                request_timeout.update(cx, |i, cx| i.set_text(v.to_string(), cx));
            }
            if let Some(v) = c.operation_timeout_ms {
                operation_timeout.update(cx, |i, cx| i.set_text(v.to_string(), cx));
            }
            if let Some(gid) = c.group_id {
                group_select.update(cx, |s, cx| s.set_value(gid.to_string(), cx));
            }
        }

        for input in [&name, &brokers, &request_timeout, &operation_timeout] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }
        cx.subscribe(&name, |this, _, event: &TextInputEvent, _cx| {
            if matches!(event, TextInputEvent::Changed | TextInputEvent::EnterPressed) {
                this.test_result = None;
            }
        })
        .detach();

        Self {
            editing,
            name,
            brokers,
            request_timeout,
            operation_timeout,
            group_select,
            submitting: false,
            testing: false,
            test_result: None,
            focus_handle: cx.focus_handle(),
        }
    }

    fn collect(&self, cx: &App) -> Option<(String, String, Option<i64>, Option<i64>, Option<i64>)> {
        let name = self.name.read(cx).text().trim().to_string();
        if name.is_empty() || name.len() > 15 {
            return None;
        }
        // 分号统一替换为逗号（Vue blur 行为）
        let brokers = self
            .brokers
            .read(cx)
            .text()
            .trim()
            .replace(';', ",")
            .to_string();
        if brokers.is_empty() {
            return None;
        }
        let req = self
            .request_timeout
            .read(cx)
            .text()
            .trim()
            .parse::<i64>()
            .ok();
        let op = self
            .operation_timeout
            .read(cx)
            .text()
            .trim()
            .parse::<i64>()
            .ok();
        let gid = self
            .group_select
            .read(cx)
            .value
            .to_string()
            .parse::<i64>()
            .ok();
        Some((name, brokers, req, op, gid))
    }

    fn test_connection(&mut self, cx: &mut Context<Self>) {
        let Some((_, brokers, req, op, _)) = self.collect(cx) else {
            return;
        };
        self.testing = true;
        self.test_result = None;
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "cluster.test_config",
                    serde_json::json!({
                        "brokers": brokers,
                        "request_timeout_ms": req,
                        "operation_timeout_ms": op,
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.testing = false;
                this.test_result = Some(match result {
                    Ok(v) => {
                        let success = v.get("success").and_then(|x| x.as_bool()).unwrap_or(false);
                        (success, String::new())
                    }
                    Err(e) => (false, e),
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.submitting {
            return;
        }
        let Some((name, brokers, req, op, gid)) = self.collect(cx) else {
            overlay::toast_error(cx, t("clusters.invalidForm"));
            return;
        };
        self.submitting = true;
        cx.notify();
        let b = backend(cx);
        let editing = self.editing.clone();
        cx.spawn(async move |this, cx| {
            let result = if let Some(c) = &editing {
                b.dispatch(
                    "cluster.update",
                    serde_json::json!({
                        "cluster_id": c.id,
                        "name": name,
                        "brokers": brokers,
                        "request_timeout_ms": req,
                        "operation_timeout_ms": op,
                        "group_id": gid,
                    }),
                )
                .await
            } else {
                b.dispatch(
                    "cluster.create",
                    serde_json::json!({
                        "name": name,
                        "brokers": brokers,
                        "request_timeout_ms": req,
                        "operation_timeout_ms": op,
                        "group_id": gid,
                    }),
                )
                .await
            };
            this.update(cx, |this, cx| {
                this.submitting = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(
                            cx,
                            if editing.is_some() {
                                t("clusters.updateSuccess")
                            } else {
                                t("clusters.createSuccess")
                            },
                        );
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            app.reload_clusters(cx);
                            app.publish(AppEvent::ClustersChanged, cx);
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
}

impl Focusable for ClusterFormDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ClusterFormDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = if self.editing.is_some() {
            t("clusters.editCluster")
        } else {
            t("clusters.addCluster")
        };
        let name_display = {
            let n = self.name.read(cx).text();
            if n.is_empty() {
                t("clusters.newCluster")
            } else {
                n
            }
        };
        let has_groups = !root(cx).read(cx).groups.is_empty();

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
                            .child(icon("server").size(px(18.)).text_color(gpui::white())),
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
                                    .child(title),
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
                        icon_btn("close-cluster-form", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(
                        div().flex_1().child(field(
                            format!("{} *", t("clusters.clusterName")),
                            input_frame(self.name.clone()).into_any_element(),
                        )),
                    )
                    .child(
                        div().flex_1().child(field(
                            format!("{} *", t("clusters.brokers")),
                            input_frame(self.brokers.clone()).into_any_element(),
                        )),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme::text_secondary())
                    .child(t("clusters.brokersHelp")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(
                        div().flex_1().child(field(
                            t("clusters.requestTimeout"),
                            input_frame(self.request_timeout.clone()).into_any_element(),
                        )),
                    )
                    .child(
                        div().flex_1().child(field(
                            t("clusters.operationTimeout"),
                            input_frame(self.operation_timeout.clone()).into_any_element(),
                        )),
                    ),
            )
            .when(has_groups, |d| {
                d.child(field(
                    t("clusters.group"),
                    self.group_select.clone().into_any_element(),
                ))
            })
            // 测试连接
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(theme::base_content_alpha(0.1))
                    .child(
                        btn("test-cluster-config", BtnKind::Ghost, BtnSize::Sm)
                            .when(self.testing, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.test_connection(cx)),
                            )
                            .child(if self.testing {
                                spinner(13.)
                            } else {
                                icon("check-circle")
                                    .size(px(14.))
                                    .text_color(theme::text_secondary())
                                    .into_any_element()
                            })
                            .child(if self.testing {
                                t("clusters.testingConnection")
                            } else {
                                t("clusters.testConnection")
                            }),
                    )
                    .when_some(self.test_result.clone(), |d, (success, err)| {
                        d.child(
                            div()
                                .text_size(px(11.))
                                .text_color(if success {
                                    theme::success()
                                } else {
                                    theme::error()
                                })
                                .child(if success {
                                    t("clusters.connectionSuccess")
                                } else {
                                    format!("{}：{}", t("clusters.connectionFailed"), err)
                                }),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .pt(px(4.))
                    .child(
                        btn("cluster-form-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("cluster-form-submit", BtnKind::Primary, BtnSize::Sm)
                            .when(self.submitting, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(cx)),
                            )
                            .child(if self.submitting {
                                spinner(13.)
                            } else {
                                icon("check").size(px(13.)).into_any_element()
                            })
                            .child(if self.editing.is_some() {
                                t("common.edit")
                            } else {
                                t("common.create")
                            }),
                    ),
            )
    }
}
