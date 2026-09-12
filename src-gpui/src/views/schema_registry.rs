//! Schema Registry 页（对齐 SchemaRegistryView.vue）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    ScrollHandle, Window,
};

use crate::app::{backend, root, Page, Route};
use crate::i18n::t;
use crate::icons::icon;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::text_area::{TextArea, TextAreaEvent};
use crate::widgets::text_input::{TextInput, TextInputEvent};

#[derive(Clone, Debug)]
pub struct SrConfig {
    registry_url: String,
    username: Option<String>,
    #[allow(dead_code)]
    has_password: bool,
}

#[derive(Clone, Debug)]
struct SchemaSummary {
    subject: String,
    latest_version: i64,
    schema_type: String,
    compatibility_level: Option<String>,
    #[allow(dead_code)]
    version_count: i64,
}

pub struct SchemaRegistryView {
    cluster: Option<String>,
    config: Option<SrConfig>,
    connected: Option<bool>,
    loading: bool,
    subjects: Vec<SchemaSummary>,
    loading_subjects: bool,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
}

impl SchemaRegistryView {
    pub fn new(route: Route, cx: &mut Context<Self>) -> Self {
        let cluster = route.get("cluster").map(|s| s.to_string());
        let this = Self {
            cluster,
            config: None,
            connected: None,
            loading: false,
            subjects: vec![],
            loading_subjects: false,
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        };
        if this.cluster.is_some() {
            this.load_config(cx);
            this.load_subjects(cx);
        }
        this
    }

    fn load_config(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.loading = true;
                cx.notify();
            })
            .ok();
            let result = b
                .dispatch(
                    "schema_registry.config.get",
                    serde_json::json!({"cluster_id": cluster}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(v) => {
                        let cfg = v.get("config").cloned().unwrap_or(v.clone());
                        if cfg.is_null() {
                            this.config = None;
                        } else if cfg.get("registry_url").is_some() {
                            this.config = Some(SrConfig {
                                registry_url: cfg
                                    .get("registry_url")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                username: cfg
                                    .get("username")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                has_password: cfg
                                    .get("has_password")
                                    .and_then(|x| x.as_bool())
                                    .unwrap_or(false),
                            });
                            // 静默测试一次连接
                            this.silent_test(cx);
                        } else {
                            this.config = None;
                        }
                    }
                    Err(_) => this.config = None,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn silent_test(&self, cx: &mut Context<Self>) {
        let Some(cfg) = self.config.clone() else { return };
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "schema_registry.config.test",
                    serde_json::json!({
                        "registry_url": cfg.registry_url,
                        "username": cfg.username,
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.connected = result
                    .ok()
                    .and_then(|v| v.get("success").and_then(|x| x.as_bool()));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_subjects(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.loading_subjects = true;
                cx.notify();
            })
            .ok();
            let result = b
                .dispatch(
                    "schema_registry.list",
                    serde_json::json!({"cluster_id": cluster}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.loading_subjects = false;
                if let Ok(v) = result {
                    this.subjects = v
                        .get("schemas")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    Some(SchemaSummary {
                                        subject: item.get("subject")?.as_str()?.to_string(),
                                        latest_version: item
                                            .get("latest_version")
                                            .and_then(|x| x.as_i64())
                                            .unwrap_or(0),
                                        schema_type: item
                                            .get("schema_type")
                                            .and_then(|x| x.as_str())
                                            .unwrap_or("AVRO")
                                            .to_string(),
                                        compatibility_level: item
                                            .get("compatibility_level")
                                            .and_then(|x| x.as_str())
                                            .map(|s| s.to_string()),
                                        version_count: item
                                            .get("version_count")
                                            .and_then(|x| x.as_i64())
                                            .unwrap_or(0),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_cluster_selector(&self, cx: &mut Context<Self>) {
        let view = cx.new(SrClusterSelector::new);
        overlay::open_modal(cx, view.into());
    }

    fn open_config_dialog(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let config = self.config.clone();
        let view = cx.new(|cx| SrConfigDialog::new(cluster, config, cx));
        overlay::open_modal(cx, view.into());
    }

    fn delete_config(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        overlay::confirm(cx, t("common.confirm"), t("schemaRegistry.deleteConfirm"), true, move |cx| {
            let b = backend(cx);
            let cluster = cluster.clone();
            cx.spawn(async move |cx| {
                let result = b
                    .dispatch(
                        "schema_registry.config.delete",
                        serde_json::json!({"cluster_id": cluster}),
                    )
                    .await;
                cx.update(|cx| {
                    match result {
                        Ok(_) => {
                            overlay::toast_success(cx, "Configuration deleted");
                            root(cx).update(cx, |app, cx| {
                                if let Some(v) = app.page_schema_registry() {
                                    v.update(cx, |view, cx| {
                                        view.config = None;
                                        view.subjects = vec![];
                                        view.connected = None;
                                        cx.notify();
                                    });
                                }
                            });
                        }
                        Err(e) => overlay::toast_error(cx, e),
                    }
})
            })
            .detach();
        });
    }

    fn view_schema(&self, subject: &str, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let b = backend(cx);
        let subject = subject.to_string();
        cx.spawn(async move |_, cx| {
            let result = b
                .dispatch(
                    "schema_registry.get_latest",
                    serde_json::json!({"cluster_id": cluster, "subject": subject}),
                )
                .await;
            cx.update(|cx| {
                match result {
                    Ok(v) => {
                        let info = SchemaDetail {
                            subject: v
                                .get("subject")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            version: v.get("version").and_then(|x| x.as_i64()).unwrap_or(0),
                            schema_type: v
                                .get("schema_type")
                                .and_then(|x| x.as_str())
                                .unwrap_or("AVRO")
                                .to_string(),
                            schema_json: v
                                .get("schema_json")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            compatibility_level: v
                                .get("compatibility_level")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string()),
                        };
                        let view = cx.new(|cx| SrDetailDialog::new(info, cx));
                        overlay::open_modal(cx, view.into());
                    }
                    Err(e) => overlay::toast_error(cx, e),
                }
            })
        })
        .detach();
    }

    fn delete_schema(&self, subject: &str, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let message = format!("{}\n\n{}", t("schemaRegistry.deleteConfirm"), subject);
        let subject = subject.to_string();
        overlay::confirm(cx, t("common.confirm"), message, true, move |cx| {
            let b = backend(cx);
            let (cluster, subject) = (cluster.clone(), subject.clone());
            cx.spawn(async move |cx| {
                let result = b
                    .dispatch(
                        "schema_registry.delete",
                        serde_json::json!({"cluster_id": cluster, "subject": subject}),
                    )
                    .await;
                cx.update(|cx| {
                    match result {
                        Ok(_) => {
                            overlay::toast_success(cx, "Schema deleted successfully");
                            root(cx).update(cx, |app, cx| {
                                if let Some(v) = app.page_schema_registry() {
                                    v.update(cx, |view, cx| view.load_subjects(cx));
                                }
                            });
                        }
                        Err(e) => overlay::toast_error(cx, e),
                    }
})
            })
            .detach();
        });
    }

    fn open_register(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let view = cx.new(|cx| SrRegisterDialog::new(cluster, cx));
        overlay::open_modal(cx, view.into());
    }
}

impl Focusable for SchemaRegistryView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SchemaRegistryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);
        let cluster = self.cluster.clone();

        let mut page = div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(12.))
            .gap(px(12.))
            .overflow_hidden();

        // 页头
        page = page.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    icon_btn("sr-back", "arrow-left", BtnSize::Xs)
                        .when(!app.read(cx).can_go_back(), |b| b.opacity(0.5))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            root(cx).update(cx, |app, cx| app.go_back(cx));
                        }),
                )
                .child(
                    icon("book")
                        .size(px(20.))
                        .text_color(theme::badge_primary_text()),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme::text_primary())
                                .child("Schema Registry"),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::text_secondary())
                                .child(t("schemaRegistry.description")),
                        ),
                )
                .child(
                    btn("sr-cluster-select", BtnKind::Outline, BtnSize::Sm)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.open_cluster_selector(cx)),
                        )
                        .child(icon("server").size(px(13.)).into_any_element())
                        .child(
                            cluster
                                .clone()
                                .unwrap_or_else(|| t("schemaRegistry.selectCluster")),
                        ),
                ),
        );

        // 前置状态
        if cluster.is_none() {
            return page.child(
                div().flex_1().child(empty_block(
                    "database",
                    t("common.noData"),
                    t("schemaRegistry.configNotSet"),
                )),
            );
        }
        if self.loading {
            return page.child(div().flex_1().child(loading_block(t("common.loading"))));
        }
        if self.config.is_none() {
            return page.child(
                div().flex_1().flex().items_center().justify_center().child(
                    card()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .p(px(30.))
                        .w(px(360.))
                        .child(
                            icon("key")
                                .size(px(36.))
                                .text_color(theme::base_content_alpha(0.3)),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme::text_primary())
                                .child(t("schemaRegistry.configNotSet")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::text_secondary())
                                .child(t("schemaRegistry.description")),
                        )
                        .child(
                            btn("sr-open-config", BtnKind::Primary, BtnSize::Sm)
                                .child(t("schemaRegistry.configTitle"))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| this.open_config_dialog(cx)),
                                ),
                        ),
                ),
            );
        }

        let config = self.config.clone().unwrap();
        let connected = self.connected;

        page.child(
            div().id("views_schema_registry_rs_2")
                .flex_1()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .flex()
                .flex_col()
                .gap(px(12.))
                // 配置卡片
                .child(
                    card()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .px(px(12.))
                                .h(px(40.))
                                .border_b_1()
                                .border_color(theme::border_base_200())
                                .child(
                                    icon("cog")
                                        .size(px(15.))
                                        .text_color(theme::badge_primary_text()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_size(px(13.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme::text_primary())
                                        .child(t("schemaRegistry.configTitle")),
                                )
                                .child(
                                    icon_btn("sr-edit-config", "pencil", BtnSize::Xs)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.open_config_dialog(cx)
                                            }),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("sr-delete-config")
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size(px(24.))
                                        .rounded(px(6.))
                                        .cursor_pointer()
                                        .text_color(theme::error())
                                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.delete_config(cx)
                                            }),
                                        )
                                        .child(icon("trash").size(px(13.))),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .p(px(12.))
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .w(px(96.))
                                                .text_size(px(11.))
                                                .text_color(theme::text_secondary())
                                                .child("Registry URL"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_family("monospace")
                                                .text_color(theme::text_primary())
                                                .child(config.registry_url.clone()),
                                        ),
                                )
                                .when_some(config.username.clone(), |d, username| {
                                    d.child(
                                        div()
                                            .flex()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .w(px(96.))
                                                    .text_size(px(11.))
                                                    .text_color(theme::text_secondary())
                                                    .child(t("schemaRegistry.username")),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme::text_primary())
                                                    .child(username),
                                            ),
                                    )
                                })
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .w(px(96.))
                                                .text_size(px(11.))
                                                .text_color(theme::text_secondary())
                                                .child(t("common.status")),
                                        )
                                        .child(
                                            div()
                                                .size(px(8.))
                                                .rounded(px(4.))
                                                .bg(match connected {
                                                    Some(true) => theme::success(),
                                                    Some(false) => theme::error(),
                                                    None => theme::warning(),
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme::text_primary())
                                                .child(match connected {
                                                    Some(true) => t("common.connected"),
                                                    Some(false) => t("common.disconnected"),
                                                    None => t("common.unknown"),
                                                }),
                                        ),
                                ),
                        ),
                )
                // Subjects 卡片
                .child(
                    card()
                        .flex()
                        .flex_col()
                        .flex_none()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .px(px(12.))
                                .h(px(40.))
                                .border_b_1()
                                .border_color(theme::border_base_200())
                                .child(
                                    icon("document")
                                        .size(px(15.))
                                        .text_color(theme::badge_primary_text()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_size(px(13.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme::text_primary())
                                        .child("Subjects"),
                                )
                                .child(
                                    btn("sr-register", BtnKind::Primary, BtnSize::Xs)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.open_register(cx)
                                            }),
                                        )
                                        .child(icon("plus").size(px(12.)).into_any_element())
                                        .child(t("schemaRegistry.registerSchema")),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .when(self.loading_subjects, |d| {
                                    d.child(div().h(px(80.)).child(loading_block(t("common.loading"))))
                                })
                                .when(!self.loading_subjects && self.subjects.is_empty(), |d| {
                                    d.child(div().h(px(80.)).child(empty_block(
                                        "document",
                                        t("schemaRegistry.noSubjects"),
                                        "",
                                    )))
                                })
                                .when(!self.subjects.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            // 表头
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .h(px(30.))
                                                    .px(px(12.))
                                                    .text_size(px(11.))
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(theme::text_secondary())
                                                    .border_b_1()
                                                    .border_color(theme::border_base_200())
                                                    .child(
                                                        div().flex_1().child(t("common.name")),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(60.))
                                                            .child(t("schemaRegistry.version")),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(90.))
                                                            .child(t("schemaRegistry.schemaType")),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(120.))
                                                            .child(t("schemaRegistry.compatibilityLevel")),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(70.))
                                                            .flex()
                                                            .justify_end()
                                                            .child(t("common.actions")),
                                                    ),
                                            )
                                            .children(self.subjects.iter().enumerate().map(
                                                |(ix, s)| {
                                                    let subject = s.subject.clone();
                                                    let subject2 = s.subject.clone();
                                                    div()
                                                        .id(("subject-row", ix))
                                                        .flex()
                                                        .items_center()
                                                        .h(px(34.))
                                                        .px(px(12.))
                                                        .text_size(px(11.))
                                                        .hover(|s2| {
                                                            s2.bg(theme::table_row_hover())
                                                        })
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .font_family("monospace")
                                                                .text_color(theme::text_primary())
                                                                .overflow_hidden()
                                                                .whitespace_nowrap()
                                                                .child(s.subject.clone()),
                                                        )
                                                        .child(
                                                            div()
                                                                .w(px(60.))
                                                                .text_color(theme::text_primary())
                                                                .child(
                                                                    s.latest_version.to_string(),
                                                                ),
                                                        )
                                                        .child(
                                                            div().w(px(90.)).child(badge(
                                                                s.schema_type.clone(),
                                                                BadgeKind::Ghost,
                                                            )),
                                                        )
                                                        .child(
                                                            div()
                                                                .w(px(120.))
                                                                .text_color(theme::text_secondary())
                                                                .child(
                                                                    s.compatibility_level
                                                                        .clone()
                                                                        .unwrap_or_else(|| {
                                                                            "-".into()
                                                                        }),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .w(px(70.))
                                                                .flex()
                                                                .justify_end()
                                                                .gap(px(2.))
                                                                .child(
                                                                    icon_btn(
                                                                        ("subject-view", ix),
                                                                        "eye",
                                                                        BtnSize::Xs,
                                                                    )
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        cx.listener(
                                                                            move |this, _, _, cx| {
                                                                                this.view_schema(
                                                                                    &subject, cx,
                                                                                )
                                                                            },
                                                                        ),
                                                                    ),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id((
                                                                            "subject-del", ix,
                                                                        ))
                                                                        .flex_none()
                                                                        .flex()
                                                                        .items_center()
                                                                        .justify_center()
                                                                        .size(px(24.))
                                                                        .rounded(px(6.))
                                                                        .cursor_pointer()
                                                                        .text_color(theme::error())
                                                                        .hover(|s2| {
                                                                            s2.bg(
                                                                                theme::btn_ghost_hover(),
                                                                            )
                                                                        })
                                                                        .on_mouse_down(
                                                                            MouseButton::Left,
                                                                            cx.listener(
                                                                                move |this, _, _, cx| {
                                                                                    this.delete_schema(
                                                                                        &subject2,
                                                                                        cx,
                                                                                    )
                                                                                },
                                                                            ),
                                                                        )
                                                                        .child(
                                                                            icon("trash")
                                                                                .size(px(13.)),
                                                                        ),
                                                                ),
                                                        )
                                                },
                                            )),
                                    )
                                }),
                        ),
                ),
        )
    }
}

// ==================== 集群选择对话框 ====================

pub struct SrClusterSelector {
    focus_handle: FocusHandle,
}

impl SrClusterSelector {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for SrClusterSelector {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SrClusterSelector {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(_cx);
        let clusters = app.read(_cx).clusters.clone();
        let current = app.read(_cx).route.get("cluster").map(|s| s.to_string());

        div()
            .w(px(440.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(16.))
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(32.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme::gradient_1())
                            .child(icon("server").size(px(16.)).text_color(gpui::white())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme::text_primary())
                                    .child(t("schemaRegistry.selectCluster")),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme::text_secondary())
                                    .child(t("schemaRegistry.selectClusterDesc")),
                            ),
                    )
                    .child(
                        icon_btn("close-sr-selector", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(
                div().id("views_schema_registry_rs_3")
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .max_h(px(320.))
                    .overflow_y_scroll()
                    .children(clusters.into_iter().enumerate().map(|(ix, c)| {
                        let selected = current.as_ref() == Some(&c.name);
                        let name = c.name.clone();
                        div()
                            .id(("sr-cluster", ix))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .px(px(10.))
                            .py(px(8.))
                            .rounded(px(8.))
                            .cursor_pointer()
                            .when(selected, |d| d.bg(theme::badge_primary_bg()))
                            .hover(|s| s.bg(theme::context_menu_item_hover()))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                root(cx).update(cx, |app, cx| {
                                    let route =
                                        Route::new(Page::SchemaRegistry).with("cluster", &name);
                                    app.navigate(route, true, cx);
                                });
                                overlay::close_modal(cx);
                            })
                            .child(
                                div()
                                    .size(px(14.))
                                    .rounded(px(7.))
                                    .border_1()
                                    .border_color(if selected {
                                        theme::primary()
                                    } else {
                                        theme::base_content_alpha(0.3)
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(selected, |d| {
                                        d.child(
                                            div()
                                                .size(px(7.))
                                                .rounded(px(4.))
                                                .bg(theme::primary()),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_family("monospace")
                                    .text_color(theme::text_primary())
                                    .child(c.name),
                            )
                    })),
            )
    }
}

// ==================== 配置对话框 ====================

pub struct SrConfigDialog {
    cluster: String,
    url: Entity<TextInput>,
    username: Entity<TextInput>,
    password: Entity<TextInput>,
    testing: bool,
    saving: bool,
    connected: Option<bool>,
    focus_handle: FocusHandle,
}

impl SrConfigDialog {
    pub fn new(cluster: String, config: Option<SrConfig>, cx: &mut Context<Self>) -> Self {
        let url = cx.new(TextInput::new);
        url.update(cx, |i, _| i.set_placeholder("http://localhost:8081"));
        let username = cx.new(TextInput::new);
        username.update(cx, |i, _| i.set_placeholder(t("schemaRegistry.username")));
        let password = cx.new(TextInput::new);
        password.update(cx, |i, _| {
            i.set_placeholder(t("schemaRegistry.password"));
            i.set_masked(true);
        });
        if let Some(cfg) = &config {
            url.update(cx, |i, cx| i.set_text(cfg.registry_url.clone(), cx));
            if let Some(u) = &cfg.username {
                username.update(cx, |i, cx| i.set_text(u.clone(), cx));
            }
        }
        for input in [&url, &username, &password] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }
        Self {
            cluster,
            url,
            username,
            password,
            testing: false,
            saving: false,
            connected: None,
            focus_handle: cx.focus_handle(),
        }
    }

    fn test(&mut self, cx: &mut Context<Self>) {
        let url = self.url.read(cx).text().trim().to_string();
        if url.is_empty() {
            return;
        }
        self.testing = true;
        cx.notify();
        let username = self.username.read(cx).text().trim().to_string();
        let password = self.password.read(cx).text();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "schema_registry.config.test",
                    serde_json::json!({
                        "registry_url": url,
                        "username": if username.is_empty() { None } else { Some(username) },
                        "password": if password.is_empty() { None } else { Some(password) },
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.testing = false;
                let success = result
                    .as_ref()
                    .ok()
                    .and_then(|v| v.get("success"))
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                this.connected = Some(success);
                if success {
                    overlay::toast_success(cx, t("schemaRegistry.connectionSuccess"));
                } else {
                    overlay::toast_error(cx, t("schemaRegistry.connectionFailed"));
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let url = self.url.read(cx).text().trim().to_string();
        if url.is_empty() {
            overlay::toast_error(cx, "Registry URL is required");
            return;
        }
        if self.saving {
            return;
        }
        self.saving = true;
        cx.notify();
        let username = self.username.read(cx).text().trim().to_string();
        let password = self.password.read(cx).text();
        let b = backend(cx);
        let cluster = self.cluster.clone();
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "schema_registry.config.save",
                    serde_json::json!({
                        "cluster_id": cluster,
                        "registry_url": url,
                        "username": if username.is_empty() { None } else { Some(username) },
                        "password": if password.is_empty() { None } else { Some(password) },
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, "Configuration saved");
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            if let Some(v) = app.page_schema_registry() {
                                v.update(cx, |view, cx| {
                                    view.load_config(cx);
                                    view.load_subjects(cx);
                                });
                            }
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

impl Focusable for SrConfigDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SrConfigDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("schemaRegistry.configTitle")),
                    )
                    .child(
                        icon_btn("close-sr-config", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(field(
                "Registry URL *",
                input_frame(self.url.clone()).into_any_element(),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme::text_secondary())
                            .child(t("schemaRegistry.authentication")),
                    )
                    .child(input_frame(self.username.clone()))
                    .child(input_frame(self.password.clone())),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div().flex().items_center().gap(px(6.)).when_some(
                            self.connected,
                            |d, connected| {
                                d.child(
                                    div()
                                        .size(px(8.))
                                        .rounded(px(4.))
                                        .bg(if connected {
                                            theme::success()
                                        } else {
                                            theme::error()
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme::text_secondary())
                                        .child(if connected {
                                            t("common.connected")
                                        } else {
                                            t("common.disconnected")
                                        }),
                                )
                            },
                        ),
                    )
                    .child(
                        btn("sr-test-config", BtnKind::Ghost, BtnSize::Sm)
                            .when(self.testing, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.test(cx)),
                            )
                            .child(if self.testing {
                                spinner(13.)
                            } else {
                                icon("lightning").size(px(13.)).into_any_element()
                            })
                            .child(t("clusters.testConnection")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        btn("sr-config-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("sr-config-save", BtnKind::Primary, BtnSize::Sm)
                            .when(self.saving, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.save(cx)),
                            )
                            .child(if self.saving {
                                spinner(13.)
                            } else {
                                icon("check").size(px(13.)).into_any_element()
                            })
                            .child(t("schemaRegistry.saveConfig")),
                    ),
            )
    }
}

// ==================== Schema 详情对话框 ====================

pub struct SchemaDetail {
    subject: String,
    version: i64,
    schema_type: String,
    schema_json: String,
    compatibility_level: Option<String>,
}

pub struct SrDetailDialog {
    info: SchemaDetail,
    focus_handle: FocusHandle,
}

impl SrDetailDialog {
    pub fn new(info: SchemaDetail, cx: &mut Context<Self>) -> Self {
        Self {
            info,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for SrDetailDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SrDetailDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(640.))
            .max_h(px(560.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .font_family("monospace")
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(self.info.subject.clone()),
                    )
                    .child(badge(
                        format!("v{}", self.info.version),
                        BadgeKind::Primary,
                    ))
                    .child(
                        icon_btn("close-sr-detail", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .child(badge(self.info.schema_type.clone(), BadgeKind::Primary))
                    .when_some(self.info.compatibility_level.clone(), |d, level| {
                        d.child(badge(level, BadgeKind::Ghost))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme::text_secondary())
                            .child(t("schemaRegistry.schemaContent")),
                    )
                    .child(
                        div().id("views_schema_registry_rs_4")
                            .flex_1()
                            .overflow_y_scroll()
                            .rounded(px(6.))
                            .bg(theme::base_content_alpha(0.05))
                            .p(px(10.))
                            .font_family("monospace")
                            .text_size(px(12.))
                            .text_color(theme::text_primary())
                            .child(self.info.schema_json.clone()),
                    ),
            )
    }
}

// ==================== 注册 Schema 对话框 ====================

pub struct SrRegisterDialog {
    cluster: String,
    subject: Entity<TextInput>,
    schema_type: Entity<Select>,
    schema_json: Entity<TextArea>,
    registering: bool,
    testing: bool,
    focus_handle: FocusHandle,
}

impl SrRegisterDialog {
    pub fn new(cluster: String, cx: &mut Context<Self>) -> Self {
        let subject = cx.new(TextInput::new);
        subject.update(cx, |i, _| i.set_placeholder("Subject name"));
        let schema_type = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption {
                        value: "AVRO".into(),
                        label: "AVRO".into(),
                    },
                    SelectOption {
                        value: "PROTOBUF".into(),
                        label: "PROTOBUF".into(),
                    },
                    SelectOption {
                        value: "JSON".into(),
                        label: "JSON".into(),
                    },
                ],
                "AVRO",
                cx,
            )
        });
        let schema_json = cx.new(TextArea::new);
        schema_json.update(cx, |a, _| {
            a.set_placeholder(t("schemaRegistry.schemaPlaceholder"))
        });

        cx.subscribe(&subject, |_, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();
        cx.subscribe(&schema_json, |_, _, event: &TextAreaEvent, cx| {
            if matches!(event, TextAreaEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();

        Self {
            cluster,
            subject,
            schema_type,
            schema_json,
            registering: false,
            testing: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn test_compatibility(&mut self, cx: &mut Context<Self>) {
        let subject = self.subject.read(cx).text().trim().to_string();
        let schema = self.schema_json.read(cx).text();
        if subject.is_empty() || schema.is_empty() {
            overlay::toast_error(cx, t("schemaRegistry.fillRequired"));
            return;
        }
        self.testing = true;
        cx.notify();
        let b = backend(cx);
        let cluster = self.cluster.clone();
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "schema_registry.compatibility.test",
                    serde_json::json!({
                        "cluster_id": cluster,
                        "subject": subject,
                        "schema_json": schema,
                        "version": -1,
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.testing = false;
                match result {
                    Ok(v) => {
                        let compatible = v
                            .get("compatible")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false);
                        if compatible {
                            overlay::toast_success(cx, t("schemaRegistry.compatible"));
                        } else {
                            overlay::toast_warning(cx, t("schemaRegistry.incompatible"));
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

    fn register(&mut self, cx: &mut Context<Self>) {
        let subject = self.subject.read(cx).text().trim().to_string();
        let schema = self.schema_json.read(cx).text();
        if subject.is_empty() || schema.is_empty() {
            overlay::toast_error(cx, t("schemaRegistry.fillRequired"));
            return;
        }
        if self.registering {
            return;
        }
        self.registering = true;
        cx.notify();
        let schema_type = self.schema_type.read(cx).value.to_string();
        let b = backend(cx);
        let cluster = self.cluster.clone();
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "schema_registry.register",
                    serde_json::json!({
                        "cluster_id": cluster,
                        "subject": subject,
                        "schema_json": schema,
                        "schema_type": schema_type,
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.registering = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, "Schema registered successfully");
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            if let Some(v) = app.page_schema_registry() {
                                v.update(cx, |view, cx| view.load_subjects(cx));
                            }
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

impl Focusable for SrRegisterDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SrRegisterDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(640.))
            .max_h(px(600.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("schemaRegistry.registerSchema")),
                    )
                    .child(
                        icon_btn("close-sr-register", "x-mark", BtnSize::Sm).on_mouse_down(
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
                            format!("{} *", t("common.name")),
                            input_frame(self.subject.clone()).into_any_element(),
                        )),
                    )
                    .child(
                        div().w(px(150.)).child(field(
                            t("schemaRegistry.schemaType"),
                            self.schema_type.clone().into_any_element(),
                        )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme::text_secondary())
                            .child(format!("{} *", t("schemaRegistry.schemaContent"))),
                    )
                    .child(
                        div()
                            .h(px(220.))
                            .rounded(px(6.))
                            .bg(theme::input_bg())
                            .border_1()
                            .border_color(theme::base_content_alpha(0.15))
                            .font_family("monospace")
                            .text_size(px(12.))
                            .p(px(6.))
                            .child(self.schema_json.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        btn("sr-test-compat", BtnKind::Ghost, BtnSize::Sm)
                            .when(self.testing, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.test_compatibility(cx)),
                            )
                            .child(if self.testing {
                                spinner(13.)
                            } else {
                                icon("check-circle").size(px(13.)).into_any_element()
                            })
                            .child(t("schemaRegistry.testCompatibility")),
                    )
                    .child(
                        btn("sr-register-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("sr-register-submit", BtnKind::Primary, BtnSize::Sm)
                            .when(self.registering, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.register(cx)),
                            )
                            .child(if self.registering {
                                spinner(13.)
                            } else {
                                icon("plus").size(px(13.)).into_any_element()
                            })
                            .child(t("schemaRegistry.registerSchema")),
                    ),
            )
    }
}
