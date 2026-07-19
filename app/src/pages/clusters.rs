//! 集群管理页：列表、创建/编辑、删除、连接测试、分组

use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::select::{SearchableVec, Select, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::i18n::t;
use crate::state::{Backend, TokioRuntime};

#[derive(Clone, Debug)]
struct ClusterInfo {
    id: i64,
    name: String,
    brokers: String,
    group_id: Option<i64>,
}

#[derive(Clone, Debug)]
struct GroupInfo {
    id: i64,
    name: String,
}

/// 分组下拉选项
#[derive(Clone)]
struct GroupOption {
    id: i64, // 0 表示未分组
    name: SharedString,
}

impl gpui_component::select::SelectItem for GroupOption {
    type Value = i64;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// 集群创建/编辑表单
struct ClusterForm {
    id: Option<i64>,
    name: Entity<InputState>,
    brokers: Entity<InputState>,
    group: Entity<SelectState<SearchableVec<GroupOption>>>,
}

pub struct ClustersPage {
    clusters: Vec<ClusterInfo>,
    groups: Vec<GroupInfo>,
    loading: bool,
    error: Option<String>,
    testing_id: Option<i64>,
    form: Option<ClusterForm>,
}

/// 向所有窗口推送通知（异步回调中使用）
pub fn notify(cx: &mut App, note_type: NotificationType, text: String) {
    for w in cx.windows() {
        let text = text.clone();
        let _ = w.update(cx, |_, window, cx| {
            window.push_notification(
                Notification::new().message(text).with_type(note_type),
                cx,
            );
        });
    }
}

impl ClustersPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = Self {
            clusters: Vec::new(),
            groups: Vec::new(),
            loading: true,
            error: None,
            testing_id: None,
            form: None,
        };
        this.reload(cx);
        this
    }

    /// 重新加载集群与分组
    fn reload(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else {
            return;
        };

        cx.spawn(async move |this, cx| {
            let clusters = crate::service::call(&rt, state.clone(), "cluster.list", json!({})).await;
            let groups = crate::service::call(&rt, state, "cluster_group.list", json!({})).await;

            this.update(cx, |this, cx| {
                this.loading = false;
                match clusters {
                    Ok(value) => {
                        this.error = None;
                        this.clusters = value
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|c| {
                                        Some(ClusterInfo {
                                            id: c.get("id")?.as_i64()?,
                                            name: c.get("name")?.as_str()?.to_string(),
                                            brokers: c.get("brokers")?.as_str()?.to_string(),
                                            group_id: c.get("group_id").and_then(|v| v.as_i64()),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                    }
                    Err(e) => this.error = Some(e),
                }
                if let Ok(value) = groups {
                    this.groups = value
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|g| {
                                    Some(GroupInfo {
                                        id: g.get("id")?.as_i64()?,
                                        name: g.get("name")?.as_str()?.to_string(),
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

    /// 测试集群连接
    fn test_connection(&mut self, id: i64, cx: &mut Context<Self>) {
        if self.testing_id.is_some() {
            return;
        }
        self.testing_id = Some(id);
        cx.notify();

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "cluster.test", json!({ "id": id })).await;
            this.update(cx, |this, cx| {
                this.testing_id = None;
                cx.notify();
            })
            .ok();
            cx.update(|cx| {
                match result {
                    Ok(v) => {
                        if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                            notify(
                                cx,
                                NotificationType::Success,
                                t(cx, "clusters.connectionSuccess"),
                            );
                        } else {
                            let msg = v
                                .get("error")
                                .and_then(|e| e.as_str())
                                .unwrap_or("")
                                .to_string();
                            notify(
                                cx,
                                NotificationType::Error,
                                format!("{}: {}", t(cx, "clusters.connectionFailed"), msg),
                            );
                        }
                    }
                    Err(e) => notify(cx, NotificationType::Error, e),
                }
            })
            .ok();
        })
        .detach();
    }

    /// 删除集群（确认对话框）
    fn confirm_delete(
        &mut self,
        id: i64,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entity = cx.entity();
        let title = t(cx, "common.delete");
        let message = format!("{}「{}」？", t(cx, "clusters.confirmDelete"), name);
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(message.clone())
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                .on_ok(move |_, window, cx| {
                    entity.update(cx, |this, cx| this.delete_cluster(id, window, cx));
                    true
                })
        });
    }

    fn delete_cluster(&mut self, id: i64, _window: &mut Window, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "cluster.delete", json!({ "id": id })).await;
            cx.update(|cx| match result {
                Ok(_) => notify(
                    cx,
                    NotificationType::Success,
                    t(cx, "clusters.clusterDeletedToast"),
                ),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    /// 打开创建/编辑对话框（edit 为 None 时创建）
    fn open_form(&mut self, edit: Option<&ClusterInfo>, window: &mut Window, cx: &mut Context<Self>) {
        let name_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "clusters.clusterName"))
        });
        let brokers_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "clusters.brokers"))
        });

        // 分组选项（0 = 未分组）
        let mut options: Vec<GroupOption> = vec![GroupOption {
            id: 0,
            name: t(cx, "clusters.noGroup").into(),
        }];
        options.extend(self.groups.iter().map(|g| GroupOption {
            id: g.id,
            name: g.name.clone().into(),
        }));
        let selected_ix = edit
            .and_then(|c| c.group_id)
            .and_then(|gid| options.iter().position(|o| o.id == gid))
            .unwrap_or(0);
        let group_state = cx.new(|cx| {
            let mut state = SelectState::new(SearchableVec::new(options), None, window, cx);
            state.set_selected_index(Some(IndexPath::new(selected_ix)), window, cx);
            state
        });

        if let Some(c) = edit {
            name_state.update(cx, |state, cx| {
                state.set_value(c.name.clone(), window, cx);
            });
            brokers_state.update(cx, |state, cx| {
                state.set_value(c.brokers.clone(), window, cx);
            });
        }

        self.form = Some(ClusterForm {
            id: edit.map(|c| c.id),
            name: name_state.clone(),
            brokers: brokers_state.clone(),
            group: group_state.clone(),
        });

        let entity = cx.entity();
        let title = if edit.is_some() {
            t(cx, "clusters.editClusterTitle")
        } else {
            t(cx, "clusters.addCluster")
        };
        let name_label = t(cx, "clusters.clusterName");
        let brokers_label = t(cx, "clusters.brokers");
        let group_label = t(cx, "clusters.group");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title(title.clone())
                .w(px(480.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(field_row(&name_label, Input::new(&name_state).into_any_element()))
                        .child(field_row(
                            &brokers_label,
                            Input::new(&brokers_state).into_any_element(),
                        ))
                        .child(field_row(&group_label, Select::new(&group_state).into_any_element())),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, window, cx| {
                    entity.update(cx, |this, cx| this.submit_form(window, cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    /// 提交创建/编辑表单
    fn submit_form(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form.take() else { return };
        let name = form.name.read(cx).value().to_string();
        let brokers = form.brokers.read(cx).value().to_string();
        let group_id = form
            .group
            .read(cx)
            .selected_value()
            .copied()
            .filter(|id| *id != 0);

        if name.trim().is_empty() || brokers.trim().is_empty() {
            let msg = t(cx, "clusters.validationNameRequired");
            notify(cx, NotificationType::Error, msg);
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let is_create = form.id.is_none();
        let method = if is_create { "cluster.create" } else { "cluster.update" };
        let mut params = json!({
            "name": name.trim(),
            "brokers": brokers.trim(),
        });
        if let Some(id) = form.id {
            params["id"] = json!(id);
        }
        params["group_id"] = json!(group_id);

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, method, params).await;
            cx.update(|cx| match result {
                Ok(_) => notify(
                    cx,
                    NotificationType::Success,
                    t(cx, if is_create { "clusters.created" } else { "clusters.updated" }),
                ),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    fn render_cluster_row(&self, c: &ClusterInfo, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let id = c.id;
        let name_for_delete = c.name.clone();
        let testing = self.testing_id == Some(id);
        let group_name = c
            .group_id
            .and_then(|gid| self.groups.iter().find(|g| g.id == gid).map(|g| g.name.clone()));

        h_flex()
            .items_center()
            .justify_between()
            .p_3()
            .rounded_md()
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().font_semibold().child(c.name.clone()))
                            .children(group_name.map(|g| {
                                div()
                                    .text_xs()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(theme.muted)
                                    .text_color(theme.foreground)
                                    .child(g)
                            })),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(c.brokers.clone()),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new(("test", id as usize))
                            .outline()
                            .label(t(cx, "clusters.test"))
                            .loading(testing)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.test_connection(id, cx);
                            })),
                    )
                    .child(
                        Button::new(("edit", id as usize))
                            .ghost()
                            .label(t(cx, "common.edit"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let info = this.clusters.iter().find(|c| c.id == id).cloned();
                                if let Some(info) = info {
                                    this.open_form(Some(&info), window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new(("delete", id as usize))
                            .ghost()
                            .label(t(cx, "common.delete"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.confirm_delete(id, name_for_delete.clone(), window, cx);
                            })),
                    ),
            )
    }
}

/// 表单字段行：标签 + 控件
fn field_row(label: &str, control: AnyElement) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_sm().child(label.to_string()))
        .child(control)
}

fn render_section(title: String, rows: Vec<AnyElement>) -> AnyElement {
    v_flex()
        .mb_4()
        .child(div().text_sm().font_semibold().mb_2().child(title))
        .child(v_flex().gap_2().children(rows))
        .into_any_element()
}

impl Render for ClustersPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let header = h_flex()
            .items_center()
            .justify_between()
            .mb_4()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().font_semibold().child(t(cx, "clusters.title")))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "clusters.description")),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("refresh")
                            .outline()
                            .label(t(cx, "common.refresh"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.loading = true;
                                cx.notify();
                                this.reload(cx);
                            })),
                    )
                    .child(
                        Button::new("create")
                            .primary()
                            .label(t(cx, "clusters.addCluster"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_form(None, window, cx);
                            })),
                    ),
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
        } else if self.clusters.is_empty() {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(theme.muted_foreground)
                .child(t(cx, "common.noData"))
                .into_any_element()
        } else {
            let mut sections: Vec<AnyElement> = Vec::new();

            for group in &self.groups {
                let rows: Vec<AnyElement> = self
                    .clusters
                    .iter()
                    .filter(|c| c.group_id == Some(group.id))
                    .map(|c| self.render_cluster_row(c, cx).into_any_element())
                    .collect();
                if !rows.is_empty() {
                    sections.push(render_section(group.name.clone(), rows));
                }
            }
            let ungrouped: Vec<AnyElement> = self
                .clusters
                .iter()
                .filter(|c| {
                    c.group_id.is_none() || !self.groups.iter().any(|g| Some(g.id) == c.group_id)
                })
                .map(|c| self.render_cluster_row(c, cx).into_any_element())
                .collect();
            if !ungrouped.is_empty() {
                sections.push(render_section(t(cx, "clusters.noGroup"), ungrouped));
            }

            v_flex().children(sections).into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
