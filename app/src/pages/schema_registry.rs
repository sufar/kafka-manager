//! Schema Registry 页：配置管理、Subject 列表、Schema 详情

use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::pages::clusters::notify;
use crate::state::{Backend, TokioRuntime};

/// Registry 配置表单
struct ConfigForm {
    url: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
}

pub struct SchemaRegistryPage {
    cluster_state: Option<Entity<SelectState<SearchableVec<StringOption>>>>,
    configured: bool,
    registry_url: Option<String>,
    subjects: Vec<String>,
    loading: bool,
    error: Option<String>,
    config_form: Option<ConfigForm>,
    _subscriptions: Vec<Subscription>,
}

impl SchemaRegistryPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = Self {
            cluster_state: None,
            configured: false,
            registry_url: None,
            subjects: Vec::new(),
            loading: true,
            error: None,
            config_form: None,
            _subscriptions: Vec::new(),
        };
        this.load_clusters(window, cx);
        this
    }

    fn selected_cluster(&self, cx: &App) -> Option<String> {
        self.cluster_state
            .as_ref()?
            .read(cx)
            .selected_value()
            .map(|s| s.to_string())
    }

    fn load_clusters(&self, window: &mut Window, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn_in(window, async move |this, cx| {
            let result = crate::service::call(&rt, state, "cluster.list", json!({})).await;
            let arr = result
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            let options: Vec<StringOption> = arr
                .iter()
                .filter_map(|c| {
                    let name = c.get("name")?.as_str()?.to_string();
                    Some(StringOption::new(name.clone(), name))
                })
                .collect();

            this.update_in(cx, |this, window, cx| {
                let has_options = !options.is_empty();
                let select = cx.new(|cx| {
                    SelectState::new(
                        SearchableVec::new(options),
                        if has_options { Some(IndexPath::new(0)) } else { None },
                        window,
                        cx,
                    )
                });
                let sub = cx.subscribe(
                    &select,
                    |this, _, event: &SelectEvent<SearchableVec<StringOption>>, cx| {
                        if matches!(event, SelectEvent::Confirm(Some(_))) {
                            this.load_config_and_subjects(cx);
                        }
                    },
                );
                this._subscriptions.push(sub);
                this.cluster_state = Some(select);
                this.loading = false;
                cx.notify();
                this.load_config_and_subjects(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 加载配置；已配置则加载 subjects
    fn load_config_and_subjects(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
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

            let configured = config
                .as_ref()
                .ok()
                .map(|v| !v.is_null())
                .unwrap_or(false);
            let registry_url = config
                .as_ref()
                .ok()
                .and_then(|v| v.get("registry_url").and_then(|u| u.as_str()))
                .map(|s| s.to_string());

            let subjects = if configured {
                crate::service::call(
                    &rt,
                    state,
                    "schema_registry.subject.list",
                    json!({ "cluster_id": cluster }),
                )
                .await
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|s| s.as_str().map(|x| x.to_string()))
                .collect()
            } else {
                Vec::new()
            };

            this.update(cx, |this, cx| {
                this.configured = configured;
                this.registry_url = registry_url;
                this.subjects = subjects;
                this.loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 打开配置对话框
    fn open_config_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "schemaRegistry.registryUrlPlaceholder"))
        });
        if let Some(url) = &self.registry_url {
            url_state.update(cx, |state, cx| {
                state.set_value(url.clone(), window, cx);
            });
        }
        let username_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "schemaRegistry.username"))
        });
        let password_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "schemaRegistry.password"))
        });

        self.config_form = Some(ConfigForm {
            url: url_state.clone(),
            username: username_state.clone(),
            password: password_state.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "schemaRegistry.configTitle");
        let url_label = t(cx, "schemaRegistry.registryUrl");
        let user_label = t(cx, "schemaRegistry.username");
        let pass_label = t(cx, "schemaRegistry.password");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title(title.clone())
                .w(px(480.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(field_row(&url_label, Input::new(&url_state).into_any_element()))
                        .child(field_row(&user_label, Input::new(&username_state).into_any_element()))
                        .child(field_row(&pass_label, Input::new(&password_state).into_any_element())),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, window, cx| {
                    entity.update(cx, |this, cx| this.submit_config(window, cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn submit_config(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.config_form.take() else { return };
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let url = form.url.read(cx).value().to_string();
        let username = form.username.read(cx).value().to_string();
        let password = form.password.read(cx).value().to_string();

        if url.trim().is_empty() {
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            // 先测试连接
            let mut test_params = json!({ "registry_url": url.trim() });
            if !username.trim().is_empty() {
                test_params["username"] = json!(username.trim());
            }
            if !password.is_empty() {
                test_params["password"] = json!(password);
            }
            let test = crate::service::call(
                &rt,
                state.clone(),
                "schema_registry.config.test",
                test_params.clone(),
            )
            .await;

            let test_ok = test
                .as_ref()
                .ok()
                .and_then(|v| v.get("success").and_then(|s| s.as_bool()))
                .unwrap_or(false);
            if !test_ok {
                let msg = test
                    .ok()
                    .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
                    .unwrap_or_else(|| "Connection failed".to_string());
                cx.update(|cx| notify(cx, NotificationType::Error, msg)).ok();
                return;
            }

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
            this.update(cx, |this, cx| this.load_config_and_subjects(cx)).ok();
        })
        .detach();
    }

    /// 查看 Subject 最新 Schema
    fn open_schema(&mut self, subject: String, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
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
                    let subject_name = value
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let version = value.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
                    let schema_type = value
                        .get("schema_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let schema_json = value
                        .get("schema_json")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let pretty = serde_json::from_str::<serde_json::Value>(&schema_json)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or(schema_json);

                    for w in cx.windows() {
                        let subject_name = subject_name.clone();
                        let schema_type = schema_type.clone();
                        let pretty = pretty.clone();
                        let _ = w.update(cx, |_, window, cx| {
                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                dialog
                                    .title(format!("{} (v{}) — {}", subject_name, version, schema_type))
                                    .w(px(720.0))
                                    .child(
                                        div()
                                            .id("schema-scroll")
                                            .max_h(px(480.0))
                                            .overflow_y_scroll()
                                            .child(div().text_xs().child(pretty.clone())),
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
        let has_cluster = self.selected_cluster(cx).is_some();

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
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("refresh")
                            .outline()
                            .label(t(cx, "common.refresh"))
                            .disabled(!has_cluster)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.loading = true;
                                cx.notify();
                                this.load_config_and_subjects(cx);
                            })),
                    )
                    .child(
                        Button::new("config")
                            .primary()
                            .label(t(cx, "schemaRegistry.configTitle"))
                            .disabled(!has_cluster)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_config_form(window, cx);
                            })),
                    ),
            );

        let toolbar = h_flex().gap_2().mb_3().children(
            self.cluster_state
                .as_ref()
                .map(|s| div().w_48().child(Select::new(s)).into_any_element()),
        );

        let content: AnyElement = if self.cluster_state.is_none() || self.loading {
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
        } else if !self.configured {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(theme.muted_foreground)
                .child(t(cx, "schemaRegistry.noSubjects"))
                .child(
                    Button::new("config-empty")
                        .primary()
                        .label(t(cx, "schemaRegistry.configTitle"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.open_config_form(window, cx);
                        })),
                )
                .into_any_element()
        } else if self.subjects.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "schemaRegistry.noSubjects"))
                .into_any_element()
        } else {
            let entity = cx.entity();
            let subjects = std::rc::Rc::new(self.subjects.clone());
            let border = theme.border;
            let count = subjects.len();

            uniform_list("subject-list", count, move |range, _window, _cx| {
                range
                    .map(|ix| {
                        let subject = subjects[ix].clone();
                        let entity = entity.clone();
                        h_flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(border)
                            .child(div().child(subject.clone()))
                            .child(
                                Button::new(("view", ix))
                                    .outline()
                                    .label("View")
                                    .on_click(move |_, window, cx| {
                                        entity.update(cx, |this, cx| {
                                            this.open_schema(subject.clone(), window, cx)
                                        });
                                    }),
                            )
                            .into_any_element()
                    })
                    .collect()
            })
            .into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(toolbar)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
