//! Schema Registry 页：与旧版 SchemaRegistryView 一致
//!
//! 头部：标题 + 描述 + [集群选择按钮]
//! 未配置：空态卡片（打开配置表单）
//! 已配置：配置卡片（URL/用户名/状态点 + 编辑/删除）+ Subjects 表格
//! （Name | Version | Type | Compatibility | View/Delete）+ 注册 Schema 按钮
//! 对话框：集群选择（radio）、配置表单（测试连接）、Schema 详情、注册（兼容性测试）

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::radio::Radio;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::back_button::back_button;
use crate::components::notify;
use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::state::{Backend, TokioRuntime};

#[derive(Clone, Debug)]
struct SchemaSummaryItem {
    subject: String,
    latest_version: i64,
    schema_type: String,
    compatibility_level: Option<String>,
}

#[derive(Clone, Debug)]
struct RegistryConfig {
    url: String,
    username: Option<String>,
    connected: Option<bool>,
}

/// 配置表单
struct ConfigForm {
    url: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    testing: bool,
}

/// Schema 详情弹窗状态（支持多版本浏览）
#[derive(Clone, Debug, Default)]
struct SchemaDetail {
    subject: String,
    versions: Vec<i64>,
    current_version: i64,
    schema_type: String,
    compatibility: Option<String>,
    pretty: String,
    loading: bool,
}

/// 注册表单
struct RegisterForm {
    name: Entity<InputState>,
    schema_type: Entity<SelectState<SearchableVec<StringOption>>>,
    content: Entity<InputState>,
}

impl gpui::EventEmitter<crate::components::navigator::NavEvent> for SchemaRegistryPage {}

pub struct SchemaRegistryPage {
    clusters: Vec<String>,
    cluster: Option<String>,
    config: Option<RegistryConfig>,
    has_password: bool,
    schemas: Vec<SchemaSummaryItem>,
    loading: bool,
    error: Option<String>,
    config_form: Option<ConfigForm>,
    register_form: Option<RegisterForm>,
    schema_detail: Option<SchemaDetail>,
    compat_select: Option<Entity<SelectState<SearchableVec<StringOption>>>>,
    _compat_sub: Option<Subscription>,
    window_handle: AnyWindowHandle,
}

impl SchemaRegistryPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = Self {
            clusters: Vec::new(),
            cluster: None,
            config: None,
            has_password: false,
            schemas: Vec::new(),
            loading: true,
            error: None,
            config_form: None,
            register_form: None,
            schema_detail: None,
            compat_select: None,
            _compat_sub: None,
            window_handle: window.window_handle(),
        };
        this.load_clusters(cx);
        this
    }

    fn load_clusters(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "cluster.list", json!({})).await;
            this.update(cx, |this, cx| {
                this.clusters = result
                    .ok()
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|c| c.get("name")?.as_str().map(|s| s.to_string()))
                    .collect();
                // 默认选第一个集群
                if this.cluster.is_none() {
                    this.cluster = this.clusters.first().cloned();
                }
                this.loading = false;
                cx.notify();
                this.load_config_and_schemas(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 加载配置 + 连接状态 + schema 列表
    fn load_config_and_schemas(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let config = crate::service::call(
                &rt,
                state.clone(),
                "schema_registry.config.get",
                json!({ "cluster_id": cluster }),
            )
            .await;

            let config_info = config.ok().and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some((
                        v.get("registry_url")?.as_str()?.to_string(),
                        v.get("username").and_then(|u| u.as_str()).map(|s| s.to_string()),
                        v.get("has_password").and_then(|h| h.as_bool()).unwrap_or(false),
                    ))
                }
            });

            // 测试连接状态
            let mut connected = None;
            if let Some((url, username, _)) = &config_info {
                let test = crate::service::call(
                    &rt,
                    state.clone(),
                    "schema_registry.config.test",
                    json!({ "registry_url": url, "username": username }),
                )
                .await;
                connected = Some(
                    test.ok()
                        .and_then(|v| v.get("success").and_then(|s| s.as_bool()))
                        .unwrap_or(false),
                );
            }

            // schema 列表
            let schemas = if config_info.is_some() {
                crate::service::call(
                    &rt,
                    state,
                    "schema_registry.list",
                    json!({ "cluster_id": cluster }),
                )
                .await
                .ok()
                .and_then(|v| v.get("schemas").and_then(|s| s.as_array()).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|s| {
                    Some(SchemaSummaryItem {
                        subject: s.get("subject")?.as_str()?.to_string(),
                        latest_version: s.get("latest_version")?.as_i64()?,
                        schema_type: s
                            .get("schema_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        compatibility_level: s
                            .get("compatibility_level")
                            .and_then(|v| v.as_str())
                            .map(|x| x.to_string()),
                    })
                })
                .collect()
            } else {
                Vec::new()
            };

            let has_pw_flag = config_info.as_ref().map(|(_, _, p)| *p).unwrap_or(false);
            this.update(cx, |this, cx| {
                this.config = config_info.map(|(url, username, _has_pw)| RegistryConfig {
                    url,
                    username,
                    connected,
                });
                this.has_password = has_pw_flag;
                this.schemas = schemas;
                this.loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 集群选择对话框（radio 列表）
    fn open_cluster_selector(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let title = t(cx, "schemaRegistry.selectCluster");
        let clusters = self.clusters.clone();
        let current = self.cluster.clone();

        let border = cx.theme().border;
        window.open_dialog(cx, move |dialog, _window, _cx| {
                let entity = entity.clone();
                let rows: Vec<AnyElement> = clusters
                    .iter()
                    .map(|name| {
                        let entity = entity.clone();
                        let name = name.clone();
                        let checked = current.as_ref() == Some(&name);
                        h_flex()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .border_b_1()
                            .border_color(border)
                            .cursor_pointer()
                            .child(Radio::new(SharedString::from(format!("sr-{}", name))).checked(checked))
                            .child(div().text_sm().child(name.clone()))
                            .id(SharedString::from(format!("sr-row-{}", name)))
                            .on_click(move |_, window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.cluster = Some(name.clone());
                                    this.config = None;
                                    this.schemas.clear();
                                    this.loading = true;
                                    cx.notify();
                                    this.load_config_and_schemas(cx);
                                });
                                window.close_dialog(cx);
                            })
                            .into_any_element()
                    })
                    .collect();
                dialog
                    .title(title.clone())
                    .w(px(420.0))
                    .child(
                        div()
                            .id("sr-cluster-scroll")
                            .max_h(px(320.0))
                            .overflow_y_scroll()
                            .child(v_flex().children(rows)),
                    )
                    .alert()
        });
    }

    /// 打开配置表单
    fn open_config_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url_value = self.config.as_ref().map(|c| c.url.clone());
        let username_value = self.config.as_ref().and_then(|c| c.username.clone());

        let url_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t(cx, "schemaRegistry.registryUrlPlaceholder"));
            if let Some(u) = url_value {
                state.set_value(u, window, cx);
            }
            state
        });
        let username_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "schemaRegistry.username"));
            if let Some(u) = username_value {
                state.set_value(u, window, cx);
            }
            state
        });
        let password_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "schemaRegistry.password"))
        });

        self.config_form = Some(ConfigForm {
            url: url_state.clone(),
            username: username_state.clone(),
            password: password_state.clone(),
            testing: false,
        });

        let entity = cx.entity();
        let title = t(cx, "schemaRegistry.configTitle");
        let url_label = t(cx, "schemaRegistry.registryUrl");
        let user_label = t(cx, "schemaRegistry.username");
        let pass_label = t(cx, "schemaRegistry.password");
        let test_label = t(cx, "schemaRegistry.testConnection");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let entity_test = entity.clone();
            dialog
                .title(title.clone())
                .w(px(480.0))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(field_row(&url_label, Input::new(&url_state).into_any_element()))
                            .child(
                                div()
                                    .text_sm()
                                    .child(t(_cx, "schemaRegistry.authentication")),
                            )
                            .child(field_row(&user_label, Input::new(&username_state).into_any_element()))
                            .child(field_row(&pass_label, Input::new(&password_state).into_any_element()))
                            .child(
                                h_flex().justify_end().child(
                                    Button::new("test-conn")
                                        .ghost()
                                        .xsmall()
                                        .label(test_label.clone())
                                        .on_click(move |_, _window, cx| {
                                            entity_test.update(cx, |this, cx| this.test_form_config(cx));
                                        }),
                                ),
                            ),
                    )
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                    .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.submit_config(cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn test_form_config(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.config_form.as_mut() else { return };
        if form.testing {
            return;
        }
        form.testing = true;
        cx.notify();

        let url = form.url.read(cx).value().to_string();
        let username = form.username.read(cx).value().to_string();
        let password = form.password.read(cx).value().to_string();
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.config.test",
                json!({ "registry_url": url, "username": username, "password": password }),
            )
            .await;
            this.update(cx, |this, cx| {
                if let Some(form) = this.config_form.as_mut() {
                    form.testing = false;
                }
                cx.notify();
            })
            .ok();
            cx.update(|cx| {
                match result {
                    Ok(v) => {
                        if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                            notify(cx, NotificationType::Success, t(cx, "clusters.connectionSuccess"));
                        } else {
                            let msg = v.get("error").and_then(|e| e.as_str()).unwrap_or("").to_string();
                            notify(cx, NotificationType::Error, msg);
                        }
                    }
                    Err(e) => notify(cx, NotificationType::Error, e),
                }
            })
            .ok();
        })
        .detach();
    }

    fn submit_config(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.config_form.take() else { return };
        let Some(cluster) = self.cluster.clone() else { return };
        let url = form.url.read(cx).value().to_string();
        let username = form.username.read(cx).value().to_string();
        let password = form.password.read(cx).value().to_string();
        if url.trim().is_empty() {
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.config.save",
                json!({
                    "cluster_id": cluster,
                    "registry_url": url.trim(),
                    "username": if username.trim().is_empty() { serde_json::Value::Null } else { json!(username.trim()) },
                    "password": if password.is_empty() { serde_json::Value::Null } else { json!(password) },
                }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "common.success")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| {
                this.loading = true;
                cx.notify();
                this.load_config_and_schemas(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 删除配置（确认）
    fn confirm_delete_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let entity = cx.entity();
        let title = t(cx, "common.delete");
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let cluster = cluster.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(t(_cx, "schemaRegistry.configTitle"))
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |_this, cx| {
                            let rt = TokioRuntime::handle(cx);
                            let Some(state) = Backend::state(cx) else { return };
                            let cluster = cluster.clone();
                            cx.spawn(async move |this, cx| {
                                let result = crate::service::call(
                                    &rt,
                                    state,
                                    "schema_registry.config.delete",
                                    json!({ "cluster_id": cluster }),
                                )
                                .await;
                                cx.update(|cx| match result {
                                    Ok(_) => notify(cx, NotificationType::Success, t(cx, "common.success")),
                                    Err(e) => notify(cx, NotificationType::Error, e),
                                })
                                .ok();
                                this.update(cx, |this, cx| {
                                    this.config = None;
                                    this.schemas.clear();
                                    cx.notify();
                                })
                                .ok();
                            })
                            .detach();
                        });
                        true
                    })
        });
    }

    /// 打开注册 Schema 对话框
    fn open_register(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder("my-subject")
        });
        let type_state = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![
                    StringOption::new("AVRO", "AVRO"),
                    StringOption::new("PROTOBUF", "PROTOBUF"),
                    StringOption::new("JSON", "JSON"),
                ]),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let content_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .auto_grow(8, 16)
                .placeholder("{}")
        });

        self.register_form = Some(RegisterForm {
            name: name_state.clone(),
            schema_type: type_state.clone(),
            content: content_state.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "schemaRegistry.registerSchema");
        let name_label = t(cx, "clusters.clusterName");
        let type_label = t(cx, "schemaRegistry.schemaType");
        let content_label = "Schema";
        let test_compat_label = t(cx, "schemaRegistry.testCompatibility");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let entity_test = entity.clone();
            dialog
                .title(title.clone())
                .w(px(640.0))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(field_row(&name_label, Input::new(&name_state).into_any_element()))
                            .child(field_row(&type_label, Select::new(&type_state).into_any_element()))
                            .child(field_row(content_label, Input::new(&content_state).into_any_element()))
                            .child(
                                h_flex().justify_end().child(
                                    Button::new("test-compat")
                                        .ghost()
                                        .xsmall()
                                        .label(test_compat_label.clone())
                                        .on_click(move |_, _window, cx| {
                                            entity_test.update(cx, |this, cx| this.test_compatibility(cx));
                                        }),
                                ),
                            ),
                    )
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                    .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.submit_register(cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn test_compatibility(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(form) = self.register_form.as_ref() else { return };
        let subject = form.name.read(cx).value().to_string();
        let schema_json = form.content.read(cx).value().to_string();
        if subject.trim().is_empty() || schema_json.trim().is_empty() {
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |_this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.compatibility.test",
                json!({ "cluster_id": cluster, "subject": subject, "schema_json": schema_json }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(v) => {
                    let compatible = v
                        .get("compatible")
                        .and_then(|c| c.as_bool())
                        .unwrap_or(false);
                    let msg = v
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_string();
                    if compatible {
                        notify(cx, NotificationType::Success, t(cx, "schemaRegistry.compatible"));
                    } else {
                        notify(
                            cx,
                            NotificationType::Error,
                            format!("{}: {}", t(cx, "schemaRegistry.incompatible"), msg),
                        );
                    }
                }
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
        })
        .detach();
    }

    fn submit_register(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.register_form.take() else { return };
        let Some(cluster) = self.cluster.clone() else { return };
        let subject = form.name.read(cx).value().to_string();
        let schema_type = form
            .schema_type
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "AVRO".to_string());
        let schema_json = form.content.read(cx).value().to_string();
        if subject.trim().is_empty() || schema_json.trim().is_empty() {
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.register",
                json!({
                    "cluster_id": cluster,
                    "subject": subject.trim(),
                    "schema_json": schema_json,
                    "schema_type": schema_type,
                }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "common.success")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.load_config_and_schemas(cx)).ok();
        })
        .detach();
    }

    /// 查看 Schema 详情（多版本浏览 + 兼容性级别设置）
    fn open_schema(&mut self, subject: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        // 兼容性级别下拉（BACKWARD/FORWARD/FULL/NONE + 传递变体）
        let compat_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![
                    StringOption::new("BACKWARD", "BACKWARD"),
                    StringOption::new("BACKWARD_TRANSITIVE", "BACKWARD_TRANSITIVE"),
                    StringOption::new("FORWARD", "FORWARD"),
                    StringOption::new("FORWARD_TRANSITIVE", "FORWARD_TRANSITIVE"),
                    StringOption::new("FULL", "FULL"),
                    StringOption::new("FULL_TRANSITIVE", "FULL_TRANSITIVE"),
                    StringOption::new("NONE", "NONE"),
                ]),
                None,
                window,
                cx,
            )
        });
        let sub = cx.subscribe(&compat_select, |this, _, event: &SelectEvent<SearchableVec<StringOption>>, cx| {
            if let SelectEvent::Confirm(Some(level)) = event {
                this.set_compatibility(level.to_string(), cx);
            }
        });
        self.compat_select = Some(compat_select);
        self._compat_sub = Some(sub);

        self.schema_detail = Some(SchemaDetail {
            subject: subject.clone(),
            versions: Vec::new(),
            current_version: 0,
            schema_type: String::new(),
            compatibility: None,
            pretty: String::new(),
            loading: true,
        });
        cx.notify();

        // 加载版本列表，再加载最新版本内容
        cx.spawn(async move |this, cx| {
            let versions = crate::service::call(
                &rt,
                state.clone(),
                "schema_registry.version.list",
                json!({ "cluster_id": cluster, "subject": subject }),
            )
            .await
            .ok()
            .and_then(|v| v.get("versions").and_then(|x| x.as_array()).cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.as_i64())
            .collect::<Vec<i64>>();
            this.update(cx, |this, cx| {
                if let Some(detail) = this.schema_detail.as_mut() {
                    detail.versions = versions;
                }
                let latest = this
                    .schema_detail
                    .as_ref()
                    .and_then(|d| d.versions.iter().max().copied());
                if let Some(v) = latest {
                    this.load_schema_version(v, cx);
                } else {
                    // 无版本列表：回退到 latest 接口
                    this.load_schema_latest(cx);
                }
            })
            .ok();
        })
        .detach();

        // 详情弹窗（内容随 schema_detail 状态更新）
        let entity = cx.entity();
        let title = t(cx, "schemaRegistry.schemaDetails");
        let loading_label = t(cx, "common.loading");
        let compat_label = t(cx, "schemaRegistry.compatibilityLevel");
        window.open_dialog(cx, move |dialog, _window, cx| {
            let entity = entity.clone();
            let (subject, versions, current, schema_type, loading, select_view) = {
                let page = entity.read(cx);
                let d = page.schema_detail.clone().unwrap_or_default();
                (
                    d.subject,
                    d.versions,
                    d.current_version,
                    d.schema_type,
                    d.loading,
                    page.compat_select.clone(),
                )
            };
            let primary = cx.theme().primary;
            let muted = cx.theme().muted_foreground;

            let max_v = versions.iter().max().copied().unwrap_or(0);
            let entity_prev = entity.clone();
            let entity_next = entity.clone();
            let subject_prev = subject.clone();
            let subject_next = subject.clone();

            dialog
                .title(format!("{} — {}", title, subject))
                .w(px(720.0))
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Button::new("schema-prev")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ChevronLeft)
                                        .disabled(loading || current <= 1)
                                        .on_click(move |_, _, cx| {
                                            let prev = current - 1;
                                            entity_prev.update(cx, |this, cx| {
                                                let _ = &subject_prev;
                                                this.load_schema_version(prev, cx);
                                            });
                                        }),
                                )
                                .child(
                                    div().text_xs().text_color(muted).child(format!(
                                        "v{} / v{}",
                                        current, max_v
                                    )),
                                )
                                .child(
                                    Button::new("schema-next")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ChevronRight)
                                        .disabled(loading || current >= max_v)
                                        .on_click(move |_, _, cx| {
                                            let next = current + 1;
                                            entity_next.update(cx, |this, cx| {
                                                let _ = &subject_next;
                                                this.load_schema_version(next, cx);
                                            });
                                        }),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .px_2()
                                        .rounded_md()
                                        .bg(primary.opacity(0.15))
                                        .text_color(primary)
                                        .child(schema_type.clone()),
                                )
                                .child(div().flex_1())
                                .when_some(select_view, |el, select| {
                                    el.child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted)
                                                    .child(compat_label.clone()),
                                            )
                                            .child(
                                                div().w(px(200.0)).child(
                                                    Select::new(&select).small(),
                                                ),
                                            ),
                                    )
                                }),
                        )
                        .child({
                            let page = entity.read(cx);
                            let pretty = page
                                .schema_detail
                                .as_ref()
                                .map(|d| d.pretty.clone())
                                .unwrap_or_default();
                            div()
                                .max_h(px(480.0))
                                .when(loading, |el| {
                                    el.child(
                                        div()
                                            .p_4()
                                            .text_xs()
                                            .text_color(muted)
                                            .child(loading_label.clone()),
                                    )
                                })
                                .when(!loading, |el| {
                                    el.child(
                                        div()
                                            .text_xs()
                                            .font_family("monospace")
                                            .child(pretty),
                                    )
                                })
                                .id("schema-scroll")
                                .overflow_y_scroll()
                        }),
                )
                .alert()
        });
    }

    /// 加载指定版本的 Schema（schema_registry.get）
    fn load_schema_version(&mut self, version: i64, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(subject) = self.schema_detail.as_ref().map(|d| d.subject.clone()) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        if let Some(detail) = self.schema_detail.as_mut() {
            detail.loading = true;
        }
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.get",
                json!({ "cluster_id": cluster, "subject": subject, "version": version }),
            )
            .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(value) => {
                        let schema_json = value
                            .get("schema_json")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let pretty = serde_json::from_str::<serde_json::Value>(&schema_json)
                            .ok()
                            .and_then(|v| serde_json::to_string_pretty(&v).ok())
                            .unwrap_or(schema_json);
                        let compat = value
                            .get("compatibility_level")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        if let Some(detail) = this.schema_detail.as_mut() {
                            detail.current_version = value
                                .get("version")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(version);
                            detail.schema_type = value
                                .get("schema_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            detail.compatibility = compat.clone();
                            detail.pretty = pretty;
                            detail.loading = false;
                        }
                        this.sync_compat_select(compat, cx);
                    }
                    Err(e) => {
                        if let Some(detail) = this.schema_detail.as_mut() {
                            detail.loading = false;
                        }
                        notify(cx, NotificationType::Error, e);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 回退：加载最新版本（schema_registry.get_latest）
    fn load_schema_latest(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(subject) = self.schema_detail.as_ref().map(|d| d.subject.clone()) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.get_latest",
                json!({ "cluster_id": cluster, "subject": subject }),
            )
            .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(value) => {
                        let schema_json = value
                            .get("schema_json")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let pretty = serde_json::from_str::<serde_json::Value>(&schema_json)
                            .ok()
                            .and_then(|v| serde_json::to_string_pretty(&v).ok())
                            .unwrap_or(schema_json);
                        let compat = value
                            .get("compatibility_level")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let version = value.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
                        if let Some(detail) = this.schema_detail.as_mut() {
                            detail.current_version = version;
                            detail.versions = vec![version];
                            detail.schema_type = value
                                .get("schema_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            detail.compatibility = compat.clone();
                            detail.pretty = pretty;
                            detail.loading = false;
                        }
                        this.sync_compat_select(compat, cx);
                    }
                    Err(e) => {
                        if let Some(detail) = this.schema_detail.as_mut() {
                            detail.loading = false;
                        }
                        notify(cx, NotificationType::Error, e);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 同步兼容性下拉的选中值（SelectState 需要 window，经 window_handle 更新）
    fn sync_compat_select(&self, compat: Option<String>, cx: &mut Context<Self>) {
        let Some(select) = self.compat_select.clone() else { return };
        let Some(level) = compat else { return };
        let _ = self.window_handle.update(cx, |_, window, cx| {
            select.update(cx, |state, cx| {
                state.set_selected_value(&SharedString::from(level), window, cx);
            });
        });
    }

    /// 设置兼容性级别（schema_registry.compatibility.set）
    fn set_compatibility(&mut self, level: String, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(subject) = self.schema_detail.as_ref().map(|d| d.subject.clone()) else { return };
        // 无变化时跳过
        if self.schema_detail.as_ref().and_then(|d| d.compatibility.clone()) == Some(level.clone()) {
            return;
        }
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.compatibility.set",
                json!({ "cluster_id": cluster, "subject": subject, "compatibility_level": level }),
            )
            .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        if let Some(detail) = this.schema_detail.as_mut() {
                            detail.compatibility = Some(level);
                        }
                        let msg = t(cx, "common.success");
                        notify(cx, NotificationType::Success, msg);
                        // 刷新 Subjects 表格的兼容性列
                        this.load_config_and_schemas(cx);
                    }
                    Err(e) => notify(cx, NotificationType::Error, e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 删除 Subject（确认）
    fn confirm_delete_subject(&mut self, subject: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let entity = cx.entity();
        let title = t(cx, "common.delete");
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let cluster = cluster.clone();
            let subject = subject.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(subject.clone())
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |_this, cx| {
                            let rt = TokioRuntime::handle(cx);
                            let Some(state) = Backend::state(cx) else { return };
                            let cluster = cluster.clone();
                            let subject = subject.clone();
                            cx.spawn(async move |this, cx| {
                                let result = crate::service::call(
                                    &rt,
                                    state,
                                    "schema_registry.delete",
                                    json!({ "cluster_id": cluster, "subject": subject }),
                                )
                                .await;
                                cx.update(|cx| match result {
                                    Ok(_) => notify(cx, NotificationType::Success, t(cx, "common.success")),
                                    Err(e) => notify(cx, NotificationType::Error, e),
                                })
                                .ok();
                                this.update(cx, |this, cx| this.load_config_and_schemas(cx)).ok();
                            })
                            .detach();
                        });
                        true
                    })
        });
    }
}

/// 表单字段行：标签 + 控件
fn field_row(label: &str, control: AnyElement) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_sm().child(label.to_string()))
        .child(control)
}

impl Render for SchemaRegistryPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        // 头部
        let header = h_flex()
            .items_center()
            .justify_between()
            .mb_4()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(back_button(cx))
                            .child(
                                div()
                                    .text_xl()
                                    .font_semibold()
                                    .child(t(cx, "schemaRegistry.title")),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "schemaRegistry.description")),
                    ),
            )
            .child(
                Button::new("cluster-select")
                    .outline()
                    .icon(IconName::Building2)
                    .label(
                        self.cluster
                            .clone()
                            .unwrap_or_else(|| t(cx, "schemaRegistry.selectCluster")),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_cluster_selector(window, cx);
                    })),
            );

        let content: AnyElement = if self.loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if let Some(err) = &self.error {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.danger)
                .child(err.clone())
                .into_any_element()
        } else if self.config.is_none() {
            // 未配置
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(theme.muted_foreground)
                .child(t(cx, "schemaRegistry.configNotSet"))
                .child(
                    Button::new("config-empty")
                        .primary()
                        .label(t(cx, "schemaRegistry.configTitle"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.open_config_form(window, cx);
                        })),
                )
                .into_any_element()
        } else {
            let config = self.config.clone().unwrap();
            let dot_color = match config.connected {
                Some(true) => theme.success,
                Some(false) => theme.danger,
                None => theme.warning,
            };
            let status_text = match config.connected {
                Some(true) => t(cx, "schemaRegistry.connectionSuccess"),
                Some(false) => t(cx, "schemaRegistry.connectionFailed"),
                None => "...".to_string(),
            };

            v_flex()
                .gap_4()
                // 配置卡片
                .child(
                    v_flex()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .px_3()
                                .py_2()
                                .border_b_1()
                                .border_color(theme.border)
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .child(t(cx, "schemaRegistry.configTitle")),
                                )
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new("edit-config")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::ALargeSmall)
                                                .tooltip(t(cx, "common.edit"))
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.open_config_form(window, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("delete-config")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Delete)
                                                .tooltip(t(cx, "common.delete"))
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.confirm_delete_config(window, cx);
                                                })),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .p_3()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w_24()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(t(cx, "schemaRegistry.registryUrl")),
                                        )
                                        .child(div().text_xs().child(config.url.clone())),
                                )
                                .children(config.username.as_ref().map(|u| {
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w_24()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(t(cx, "schemaRegistry.username")),
                                        )
                                        .child(div().text_xs().child(u.clone()))
                                        .into_any_element()
                                }))
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            div()
                                                .w_24()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child("Status"),
                                        )
                                        .child(div().size_2().rounded_full().bg(dot_color))
                                        .child(div().text_xs().child(status_text)),
                                ),
                        ),
                )
                // Subjects 卡片
                .child(
                    v_flex()
                        .flex_1()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .px_3()
                                .py_2()
                                .border_b_1()
                                .border_color(theme.border)
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .child(t(cx, "schemaRegistry.subjects")),
                                )
                                .child(
                                    Button::new("register")
                                        .primary()
                                        .xsmall()
                                        .icon(IconName::Plus)
                                        .label(t(cx, "schemaRegistry.registerSchema"))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.open_register(window, cx);
                                        })),
                                ),
                        )
                        .child(self.render_schemas_table(cx)),
                )
                .into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}

impl SchemaRegistryPage {
    fn render_schemas_table(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        if self.schemas.is_empty() {
            return div()
                .p_4()
                .text_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(t(cx, "schemaRegistry.noSubjects"))
                .into_any_element();
        }

        let header_row = h_flex()
            .px_3()
            .py_1()
            .bg(theme.table_head)
            .text_xs()
            .font_semibold()
            .child(div().flex_1().child("Name"))
            .child(div().w_16().child("Version"))
            .child(div().w_24().child(t(cx, "schemaRegistry.schemaType")))
            .child(div().w(px(112.0)).child(t(cx, "schemaRegistry.compatibilityLevel")))
            .child(div().w_20().child(t(cx, "messages.actions")));

        let rows: Vec<AnyElement> = self
            .schemas
            .iter()
            .enumerate()
            .map(|(ix, s)| {
                let subject_view = s.subject.clone();
                let subject_del = s.subject.clone();
                h_flex()
                    .items_center()
                    .px_3()
                    .py_1p5()
                    .border_b_1()
                    .border_color(theme.border)
                    .text_xs()
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(s.subject.clone()),
                    )
                    .child(div().w_16().child(format!("v{}", s.latest_version)))
                    .child(
                        div().w_24().child(
                            div()
                                .px_1()
                                .rounded_md()
                                .bg(theme.secondary)
                                .child(s.schema_type.clone()),
                        ),
                    )
                    .child(
                        div()
                            .w(px(112.0))
                            .child(s.compatibility_level.clone().unwrap_or_else(|| "-".to_string())),
                    )
                    .child(
                        h_flex()
                            .w_20()
                            .gap_1()
                            .child(
                                Button::new(("view", ix))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Eye)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_schema(subject_view.clone(), window, cx);
                                    })),
                            )
                            .child(
                                Button::new(("del", ix))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Delete)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.confirm_delete_subject(subject_del.clone(), window, cx);
                                    })),
                            ),
                    )
                    .into_any_element()
            })
            .collect();

        v_flex()
            .child(header_row)
            .child(
                div()
                    .id("schemas-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(v_flex().children(rows)),
            )
            .into_any_element()
    }
}
