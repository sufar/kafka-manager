//! Schema Registry 页：与旧版 SchemaRegistryView 一致
//!
//! 头部：标题 + 描述 + [集群选择按钮]
//! 未配置：空态卡片（打开配置表单）
//! 已配置：配置卡片（URL/用户名/状态点 + 编辑/删除）+ Subjects 表格
//! （Name | Version | Type | Compatibility | View/Delete）+ 注册 Schema 按钮
//! 对话框：集群选择（radio）、配置表单（测试连接）、Schema 详情、注册（兼容性测试）

use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::radio::Radio;
use gpui_component::select::{SearchableVec, Select, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

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

/// 注册表单
struct RegisterForm {
    name: Entity<InputState>,
    schema_type: Entity<SelectState<SearchableVec<StringOption>>>,
    content: Entity<InputState>,
}

pub struct SchemaRegistryPage {
    window_handle: AnyWindowHandle,
    clusters: Vec<String>,
    cluster: Option<String>,
    config: Option<RegistryConfig>,
    has_password: bool,
    schemas: Vec<SchemaSummaryItem>,
    loading: bool,
    error: Option<String>,
    config_form: Option<ConfigForm>,
    register_form: Option<RegisterForm>,
}

impl SchemaRegistryPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = Self {
            window_handle: window.window_handle(),
            clusters: Vec::new(),
            cluster: None,
            config: None,
            has_password: false,
            schemas: Vec::new(),
            loading: true,
            error: None,
            config_form: None,
            register_form: None,
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
    fn open_cluster_selector(&mut self, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let title = t(cx, "schemaRegistry.selectCluster");
        let clusters = self.clusters.clone();
        let current = self.cluster.clone();

        let _ = self.window_handle.update(cx, |_, window, cx| {
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
        });
    }

    /// 打开配置表单
    fn open_config_form(&mut self, cx: &mut Context<Self>) {
        let url_value = self.config.as_ref().map(|c| c.url.clone());
        let username_value = self.config.as_ref().and_then(|c| c.username.clone());

        let created = self.window_handle.update(cx, |_, window, cx| {
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
            (url_state, username_state, password_state)
        });
        let Ok((url_state, username_state, password_state)) = created else { return };

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

        let _ = self.window_handle.update(cx, |_, window, cx| {
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
    fn confirm_delete_config(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let entity = cx.entity();
        let title = t(cx, "common.delete");
        let _ = self.window_handle.update(cx, |_, window, cx| {
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
        });
    }

    /// 打开注册 Schema 对话框
    fn open_register(&mut self, cx: &mut Context<Self>) {
        let created = self.window_handle.update(cx, |_, window, cx| {
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
            (name_state, type_state, content_state)
        });
        let Ok((name_state, type_state, content_state)) = created else { return };

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

        let _ = self.window_handle.update(cx, |_, window, cx| {
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

    /// 查看 Schema 详情
    fn open_schema(&mut self, subject: String, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |_this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "schema_registry.get_latest",
                json!({ "cluster_id": cluster, "subject": subject }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(value) => {
                    let subject_name = value.get("subject").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let version = value.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
                    let schema_type = value.get("schema_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let compat = value
                        .get("compatibility_level")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let schema_json = value.get("schema_json").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let pretty = serde_json::from_str::<serde_json::Value>(&schema_json)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or(schema_json);

                    for w in cx.windows() {
                        let subject_name = subject_name.clone();
                        let schema_type = schema_type.clone();
                        let compat = compat.clone();
                        let pretty = pretty.clone();
                        let _ = w.update(cx, |_, window, cx| {
                            let primary = cx.theme().primary;
                            let secondary = cx.theme().secondary;
                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                dialog
                                    .title(format!("{} (v{})", subject_name, version))
                                    .w(px(720.0))
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .px_2()
                                                            .rounded_md()
                                                            .bg(primary.opacity(0.15))
                                                            .text_color(primary)
                                                            .child(schema_type.clone()),
                                                    )
                                                    .children(compat.as_ref().map(|c| {
                                                        div()
                                                            .text_xs()
                                                            .px_2()
                                                            .rounded_md()
                                                            .bg(secondary)
                                                            .child(c.clone())
                                                            .into_any_element()
                                                    })),
                                            )
                                            .child(
                                                div()
                                                    .id("schema-scroll")
                                                    .max_h(px(480.0))
                                                    .overflow_y_scroll()
                                                    .child(div().text_xs().child(pretty.clone())),
                                            ),
                                    )
                                    .alert()
                            });
                        });
                    }
                }
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
        })
        .detach();
    }

    /// 删除 Subject（确认）
    fn confirm_delete_subject(&mut self, subject: String, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let entity = cx.entity();
        let title = t(cx, "common.delete");
        let _ = self.window_handle.update(cx, |_, window, cx| {
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
                    .child(div().text_xl().font_semibold().child(t(cx, "schemaRegistry.title")))
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
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_cluster_selector(cx);
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
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_config_form(cx);
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
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.open_config_form(cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("delete-config")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Delete)
                                                .tooltip(t(cx, "common.delete"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.confirm_delete_config(cx);
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
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_register(cx);
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
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_schema(subject_view.clone(), cx);
                                    })),
                            )
                            .child(
                                Button::new(("del", ix))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Delete)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.confirm_delete_subject(subject_del.clone(), cx);
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
