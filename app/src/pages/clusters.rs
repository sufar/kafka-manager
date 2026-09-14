//! 集群管理页：与旧版 ClustersView 一致（卡片网格布局）
//!
//! 头部：标题 + [管理分组] [添加集群]
//! 分组 pills 横向筛选 + 搜索框
//! 卡片：图标+名称+分组徽标+创建时间 / Brokers / Timeouts / 错误条
//! 底部操作：Create Topic / Test / View Topics / Refresh Topics
//! 模态：创建/编辑集群（超时字段+内嵌测试）、管理分组（增删改）、创建 Topic

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::back_button::back_button;
use crate::components::notify;
use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::pages::topics::field_row_with_error;
use crate::state::{Backend, TokioRuntime};

#[derive(Clone, Debug)]
struct ClusterInfo {
    id: i64,
    name: String,
    brokers: String,
    request_timeout_ms: i64,
    operation_timeout_ms: i64,
    group_id: Option<i64>,
    created_at: String,
    /// 连接错误（来自 connection.list）
    conn_error: Option<String>,
}

#[derive(Clone, Debug)]
struct GroupInfo {
    id: i64,
    name: String,
    description: Option<String>,
}

/// 集群创建/编辑表单
struct ClusterForm {
    id: Option<i64>,
    name: Entity<InputState>,
    brokers: Entity<InputState>,
    request_timeout: Entity<InputState>,
    operation_timeout: Entity<InputState>,
    group: Entity<SelectState<SearchableVec<GroupOption>>>,
    testing: bool,
    test_result: Option<(bool, String)>,
    name_error: Option<String>,
    brokers_error: Option<String>,
}

/// 分组下拉选项
#[derive(Clone)]
struct GroupOption {
    id: i64, // 0 = 未分组
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

/// 分组管理表单
struct GroupForm {
    id: Option<i64>,
    name: Entity<InputState>,
    description: Entity<InputState>,
}

/// 创建 Topic 表单
struct TopicForm {
    cluster: String,
    name: Entity<InputState>,
    partitions: Entity<InputState>,
    replication: Entity<InputState>,
    advanced_open: bool,
    cleanup_policy: Entity<SelectState<SearchableVec<StringOption>>>,
    retention_ms: Entity<InputState>,
    retention_bytes: Entity<InputState>,
    segment_bytes: Entity<InputState>,
    retention_ms_error: Option<String>,
    retention_bytes_error: Option<String>,
    segment_bytes_error: Option<String>,
}

pub struct ClustersPage {
    clusters: Vec<ClusterInfo>,
    groups: Vec<GroupInfo>,
    selected_group: Option<i64>,
    search_input: Entity<InputState>,
    loading: bool,
    error: Option<String>,
    testing_id: Option<i64>,
    form: Option<ClusterForm>,
    group_form: Option<GroupForm>,
    topic_form: Option<TopicForm>,
    _subscriptions: Vec<Subscription>,
}

impl ClustersPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "clusters.searchPlaceholder"))
        });
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reload(cx);
                }
            },
        ));

        let this = Self {
            clusters: Vec::new(),
            groups: Vec::new(),
            selected_group: None,
            search_input,
            loading: true,
            error: None,
            testing_id: None,
            form: None,
            group_form: None,
            topic_form: None,
            _subscriptions: subscriptions,
        };
        this.reload(cx);
        this
    }

    /// 重新加载集群、分组、连接状态
    fn reload(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let search = self.search_input.read(cx).value().to_string();

        cx.spawn(async move |this, cx| {
            let clusters = crate::service::call(
                &rt,
                state.clone(),
                "cluster.list",
                json!({ "search": search }),
            )
            .await;
            let groups = crate::service::call(&rt, state.clone(), "cluster_group.list", json!({})).await;
            let connections = crate::service::call(&rt, state, "connection.list", json!({})).await;

            let conn_errors: std::collections::HashMap<String, String> = connections
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|c| {
                    let id = c.get("cluster_id")?.as_str()?.to_string();
                    let err = c.get("error_message")?.as_str()?.to_string();
                    if err.is_empty() { None } else { Some((id, err)) }
                })
                .collect();

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
                                        let name = c.get("name")?.as_str()?.to_string();
                                        Some(ClusterInfo {
                                            id: c.get("id")?.as_i64()?,
                                            brokers: c.get("brokers")?.as_str()?.to_string(),
                                            request_timeout_ms: c
                                                .get("request_timeout_ms")
                                                .and_then(|v| v.as_i64())
                                                .unwrap_or(5000),
                                            operation_timeout_ms: c
                                                .get("operation_timeout_ms")
                                                .and_then(|v| v.as_i64())
                                                .unwrap_or(5000),
                                            group_id: c.get("group_id").and_then(|v| v.as_i64()),
                                            created_at: c
                                                .get("created_at")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            conn_error: conn_errors.get(&name).cloned(),
                                            name,
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
                                        description: g
                                            .get("description")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
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
                            notify(cx, NotificationType::Success, t(cx, "clusters.connectionSuccess"));
                        } else {
                            let msg = v.get("error").and_then(|e| e.as_str()).unwrap_or("").to_string();
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

    /// 刷新 Topics（fire-and-forget + toast）
    fn refresh_topics(&self, cluster: String, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let msg = t(cx, "clusters.refreshingBg");
        notify(cx, NotificationType::Success, msg);
        cx.spawn(async move |_this, _cx| {
            let _ = crate::service::call(&rt, state, "topic.refresh", json!({ "cluster_id": cluster })).await;
        })
        .detach();
    }

    /// 右键菜单路由：对指定集群执行动作（创建 Topic / 编辑 / 删除）
    pub fn open_action(
        &mut self,
        cluster: String,
        action: crate::components::navigator::ClusterAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::components::navigator::ClusterAction;
        match action {
            ClusterAction::CreateTopic => self.open_create_topic(cluster, window, cx),
            ClusterAction::EditCluster => {
                let info = self.clusters.iter().find(|c| c.name == cluster).cloned();
                if let Some(info) = info {
                    self.open_form(Some(info), window, cx);
                } else {
                    let msg = t(cx, "toast.clusterNotFound");
                    notify(cx, NotificationType::Warning, msg);
                }
            }
            ClusterAction::DeleteCluster => {
                let found = self
                    .clusters
                    .iter()
                    .find(|c| c.name == cluster)
                    .map(|c| (c.id, c.name.clone()));
                if let Some((id, name)) = found {
                    self.confirm_delete(id, name, window, cx);
                } else {
                    let msg = t(cx, "toast.clusterNotFound");
                    notify(cx, NotificationType::Warning, msg);
                }
            }
        }
    }

    /// 删除集群（确认对话框）
    fn confirm_delete(&mut self, id: i64, name: String, window: &mut Window, cx: &mut Context<Self>) {
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
                .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.delete_cluster(id, cx));
                    true
                })
        });
    }

    fn delete_cluster(&mut self, id: i64, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "cluster.delete", json!({ "id": id })).await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "clusters.clusterDeletedToast")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    /// 打开创建/编辑集群对话框
    fn open_form(&mut self, edit: Option<ClusterInfo>, window: &mut Window, cx: &mut Context<Self>) {
        let is_edit = edit.is_some();
        let Some(form) = self.build_form(edit, window, cx) else { return };
        let name_state = form.name.clone();
        let brokers_state = form.brokers.clone();
        let req_state = form.request_timeout.clone();
        let op_state = form.operation_timeout.clone();
        let group_state = form.group.clone();
        self.form = Some(form);

        let entity = cx.entity();
        let title = if is_edit {
            t(cx, "clusters.editClusterTitle")
        } else {
            t(cx, "clusters.addCluster")
        };
        let name_label = t(cx, "clusters.clusterName");
        let brokers_label = t(cx, "clusters.brokers");
        let req_label = t(cx, "clusters.requestTimeout");
        let op_label = t(cx, "clusters.operationTimeout");
        let group_label = t(cx, "clusters.group");
        let test_label = t(cx, "clusters.testConnection");

        window.open_dialog(cx, move |dialog, _window, cx| {
            let entity = entity.clone();
            let entity_test = entity.clone();
            let (name_error, brokers_error) = {
                let page = entity.read(cx);
                page.form
                    .as_ref()
                    .map(|f| (f.name_error.clone(), f.brokers_error.clone()))
                    .unwrap_or_default()
            };
            let danger = cx.theme().danger;
            dialog
                .title(title.clone())
                    .w(px(520.0))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(field_row_with_error(&name_label, Input::new(&name_state).into_any_element(), name_error, danger))
                            .child(field_row_with_error(&brokers_label, Input::new(&brokers_state).into_any_element(), brokers_error, danger))
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(field_row(&req_label, Input::new(&req_state).into_any_element()))
                                    .child(field_row(&op_label, Input::new(&op_state).into_any_element())),
                            )
                            .child(field_row(&group_label, Select::new(&group_state).into_any_element()))
                            .child(
                                div().child(
                                    Button::new("test-inline")
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
                        entity.update(cx, |this, cx| this.submit_form(cx))
                    })
                    .on_cancel(|_, _, _| true)
        });
    }

    fn build_form(&mut self, edit: Option<ClusterInfo>, window: &mut Window, cx: &mut Context<Self>) -> Option<ClusterForm> {
        let is_edit_name = edit.as_ref().map(|c| c.name.clone());
        let is_edit_brokers = edit.as_ref().map(|c| c.brokers.clone());
        let req_ms = edit.as_ref().map(|c| c.request_timeout_ms).unwrap_or(5000);
        let op_ms = edit.as_ref().map(|c| c.operation_timeout_ms).unwrap_or(5000);
        let edit_group_id = edit.as_ref().and_then(|c| c.group_id);
        let edit_id = edit.as_ref().map(|c| c.id);

        let mut options: Vec<GroupOption> = vec![GroupOption {
            id: 0,
            name: t(cx, "clusters.noGroup").into(),
        }];
        options.extend(self.groups.iter().map(|g| GroupOption {
            id: g.id,
            name: g.name.clone().into(),
        }));
        let selected_ix = edit_group_id
            .and_then(|gid| options.iter().position(|o| o.id == gid))
            .unwrap_or(0);

        let name_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "clusters.clusterName"));
            if let Some(n) = is_edit_name {
                state.set_value(n, window, cx);
            }
            state
        });
        let brokers_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("broker1:9092,broker2:9092");
            if let Some(b) = is_edit_brokers {
                state.set_value(b, window, cx);
            }
            state
        });
        let req_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(req_ms.to_string(), window, cx);
            state
        });
        let op_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(op_ms.to_string(), window, cx);
            state
        });
        let group_state = cx.new(|cx| {
            let mut state = SelectState::new(SearchableVec::new(options), None, window, cx);
            state.set_selected_index(Some(IndexPath::new(selected_ix)), window, cx);
            state
        });

        Some(ClusterForm {
            id: edit_id,
            name: name_state,
            brokers: brokers_state,
            request_timeout: req_state,
            operation_timeout: op_state,
            group: group_state,
            testing: false,
            test_result: None,
            name_error: None,
            brokers_error: None,
        })
    }

    /// 表单内嵌测试连接
    fn test_form_config(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.form.as_mut() else { return };
        if form.testing {
            return;
        }
        form.testing = true;
        cx.notify();

        let brokers = form.brokers.read(cx).value().to_string();
        let name = form.name.read(cx).value().to_string();
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "cluster.test_config",
                json!({ "name": name, "brokers": brokers }),
            )
            .await;
            this.update(cx, |this, cx| {
                if let Some(form) = this.form.as_mut() {
                    form.testing = false;
                    form.test_result = result.ok().map(|v| {
                        (
                            v.get("success").and_then(|s| s.as_bool()).unwrap_or(false),
                            v.get("error").and_then(|e| e.as_str()).unwrap_or("").to_string(),
                        )
                    });
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 提交集群表单；返回是否关闭对话框（校验失败时保留表单并在字段下方显示错误）
    fn submit_form(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(form) = self.form.as_ref() else { return true };
        let name = form.name.read(cx).value().trim().to_string();
        let brokers = form.brokers.read(cx).value().to_string().replace(';', ",");
        let request_timeout_ms: i64 = form.request_timeout.read(cx).value().trim().parse().unwrap_or(5000);
        let operation_timeout_ms: i64 = form.operation_timeout.read(cx).value().trim().parse().unwrap_or(5000);
        let group_id = form.group.read(cx).selected_value().copied().filter(|id| *id != 0);

        // 名称：必填 / ≤15 字符 / 仅字母、数字、中文、连字符、下划线
        let name_error = if name.is_empty() {
            Some(t(cx, "clusters.validationNameRequired"))
        } else if name.chars().count() > 15 {
            Some(t(cx, "clusters.validationNameTooLong"))
        } else if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            Some(t(cx, "clusters.validationNameInvalid"))
        } else {
            None
        };

        // Brokers：必填 / 不允许空格和引号（分号已统一替换为逗号）
        let brokers_error = if brokers.trim().is_empty() {
            Some(t(cx, "clusters.validationBrokersRequired"))
        } else if brokers.chars().any(|c| c.is_whitespace() || c == '\'' || c == '"') {
            Some(t(cx, "clusters.validationBrokersInvalid"))
        } else {
            None
        };

        let invalid = name_error.is_some() || brokers_error.is_some();
        if let Some(form) = self.form.as_mut() {
            form.name_error = name_error;
            form.brokers_error = brokers_error;
        }
        if invalid {
            cx.notify();
            return false;
        }

        let form = self.form.take().expect("form checked above");
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return true };
        let is_create = form.id.is_none();
        let method = if is_create { "cluster.create" } else { "cluster.update" };
        let mut params = json!({
            "name": name.trim(),
            "brokers": brokers.trim(),
            "request_timeout_ms": request_timeout_ms,
            "operation_timeout_ms": operation_timeout_ms,
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
        true
    }

    /// 打开创建 Topic 对话框（含高级选项折叠区）
    fn open_create_topic(&mut self, cluster: String, window: &mut Window, cx: &mut Context<Self>) {
        let name_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "topics.topicNamePlaceholder"))
        });
        let partitions_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value("1".to_string(), window, cx);
            state
        });
        let replication_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value("1".to_string(), window, cx);
            state
        });
        // 高级选项
        let cleanup_state = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![
                    StringOption::new("-", ""),
                    StringOption::new("delete", "delete"),
                    StringOption::new("compact", "compact"),
                    StringOption::new("delete,compact", "delete,compact"),
                ]),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let retention_ms_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "topics.retentionMsPlaceholder"))
        });
        let retention_bytes_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "topics.retentionBytesPlaceholder"))
        });
        let segment_bytes_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "topics.segmentBytesPlaceholder"))
        });

        self.topic_form = Some(TopicForm {
            cluster: cluster.clone(),
            name: name_state.clone(),
            partitions: partitions_state.clone(),
            replication: replication_state.clone(),
            advanced_open: false,
            cleanup_policy: cleanup_state.clone(),
            retention_ms: retention_ms_state.clone(),
            retention_bytes: retention_bytes_state.clone(),
            segment_bytes: segment_bytes_state.clone(),
            retention_ms_error: None,
            retention_bytes_error: None,
            segment_bytes_error: None,
        });

        let entity = cx.entity();
        let title = t(cx, "topics.createTopic");
        let name_label = t(cx, "topics.topicName");
        let partitions_label = t(cx, "topics.numPartitions");
        let replication_label = t(cx, "topics.replicationFactor");
        let advanced_label = t(cx, "topics.advancedOptions");
        let cleanup_label = t(cx, "topics.cleanupPolicy");
        let retention_ms_label = t(cx, "topics.retentionMs");
        let retention_bytes_label = t(cx, "topics.retentionBytes");
        let segment_bytes_label = t(cx, "topics.segmentBytes");

        window.open_dialog(cx, move |dialog, _window, cx| {
            let entity = entity.clone();
            let entity_toggle = entity.clone();
            let (advanced_open, retention_ms_error, retention_bytes_error, segment_bytes_error) = {
                let page = entity.read(cx);
                page.topic_form
                    .as_ref()
                    .map(|f| {
                        (
                            f.advanced_open,
                            f.retention_ms_error.clone(),
                            f.retention_bytes_error.clone(),
                            f.segment_bytes_error.clone(),
                        )
                    })
                    .unwrap_or_default()
            };
            let danger = cx.theme().danger;
            let muted = cx.theme().muted_foreground;
            dialog
                .title(format!("{} — {}", title, cluster))
                    .w(px(480.0))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(field_row(&name_label, Input::new(&name_state).into_any_element()))
                            .child(field_row(&partitions_label, Input::new(&partitions_state).into_any_element()))
                            .child(field_row(&replication_label, Input::new(&replication_state).into_any_element()))
                            .child(
                                v_flex()
                                    .gap_3()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .id("advanced-toggle")
                                            .cursor_pointer()
                                            .child(
                                                Icon::new(if advanced_open {
                                                    IconName::ChevronDown
                                                } else {
                                                    IconName::ChevronRight
                                                })
                                                .size_4()
                                                .text_color(muted),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(muted)
                                                    .child(advanced_label.clone()),
                                            )
                                            .on_click(move |_, _, cx| {
                                                entity_toggle.update(cx, |this, cx| {
                                                    if let Some(form) = this.topic_form.as_mut() {
                                                        form.advanced_open = !form.advanced_open;
                                                    }
                                                    cx.notify();
                                                });
                                            }),
                                    )
                                    .when(advanced_open, |this| {
                                        this.child(
                                            field_row(&cleanup_label, Select::new(&cleanup_state).into_any_element()),
                                        )
                                        .child(field_row_with_error(
                                            &retention_ms_label,
                                            Input::new(&retention_ms_state).into_any_element(),
                                            retention_ms_error,
                                            danger,
                                        ))
                                        .child(field_row_with_error(
                                            &retention_bytes_label,
                                            Input::new(&retention_bytes_state).into_any_element(),
                                            retention_bytes_error,
                                            danger,
                                        ))
                                        .child(field_row_with_error(
                                            &segment_bytes_label,
                                            Input::new(&segment_bytes_state).into_any_element(),
                                            segment_bytes_error,
                                            danger,
                                        ))
                                    }),
                            ),
                    )
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |this, cx| this.submit_create_topic(cx))
                    })
                    .on_cancel(|_, _, _| true)
        });
    }

    /// 提交创建 Topic；返回是否关闭对话框（校验失败时保留表单并显示错误）
    fn submit_create_topic(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(form) = self.topic_form.as_ref() else { return true };
        let name = form.name.read(cx).value().to_string();
        let num_partitions: i32 = form.partitions.read(cx).value().trim().parse().unwrap_or(1);
        let replication_factor: i32 = form.replication.read(cx).value().trim().parse().unwrap_or(1);
        let cleanup_policy = form
            .cleanup_policy
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_default();
        let retention_ms = form.retention_ms.read(cx).value().trim().to_string();
        let retention_bytes = form.retention_bytes.read(cx).value().trim().to_string();
        let segment_bytes = form.segment_bytes.read(cx).value().trim().to_string();

        if name.trim().is_empty() {
            let msg = t(cx, "topics.validationTopicNameRequired");
            notify(cx, NotificationType::Error, msg);
            return false;
        }

        // 高级选项校验（仅非空时校验）
        let retention_ms_error = if !retention_ms.is_empty()
            && !matches!(retention_ms.parse::<i64>(), Ok(v) if v > 0)
        {
            Some(t(cx, "topics.validationRetentionMs"))
        } else {
            None
        };
        let retention_bytes_error =
            if !retention_bytes.is_empty() && retention_bytes.parse::<i64>().is_err() {
                Some(t(cx, "topics.validationRetentionBytes"))
            } else {
                None
            };
        let segment_bytes_error = if !segment_bytes.is_empty()
            && !matches!(segment_bytes.parse::<i64>(), Ok(v) if v > 0)
        {
            Some(t(cx, "topics.validationSegmentBytes"))
        } else {
            None
        };

        let invalid = retention_ms_error.is_some()
            || retention_bytes_error.is_some()
            || segment_bytes_error.is_some();
        if let Some(form) = self.topic_form.as_mut() {
            form.retention_ms_error = retention_ms_error;
            form.retention_bytes_error = retention_bytes_error;
            form.segment_bytes_error = segment_bytes_error;
        }
        if invalid {
            cx.notify();
            return false;
        }

        // 组装可选 config
        let mut config = serde_json::Map::new();
        if !cleanup_policy.is_empty() {
            config.insert("cleanup.policy".to_string(), json!(cleanup_policy));
        }
        if !retention_ms.is_empty() {
            config.insert("retention.ms".to_string(), json!(retention_ms));
        }
        if !retention_bytes.is_empty() {
            config.insert("retention.bytes".to_string(), json!(retention_bytes));
        }
        if !segment_bytes.is_empty() {
            config.insert("segment.bytes".to_string(), json!(segment_bytes));
        }

        let form = self.topic_form.take().expect("form checked above");
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return true };

        cx.spawn(async move |_this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic.create",
                json!({
                    "cluster_id": form.cluster,
                    "name": name.trim(),
                    "num_partitions": num_partitions,
                    "replication_factor": replication_factor,
                    "config": config,
                }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "topics.createdSuccess")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
        })
        .detach();
        true
    }

    /// 打开管理分组对话框
    fn open_manage_groups(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let title = t(cx, "clusters.manageGroups");
        let groups = self.groups.clone();
        let clusters = self.clusters.clone();

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        window.open_dialog(cx, move |dialog, _window, _cx| {
                let entity = entity.clone();
                let entity_add = entity.clone();
                let rows: Vec<AnyElement> = if groups.is_empty() {
                    vec![div()
                        .py_3()
                        .text_sm()
                        .text_color(muted)
                        .child(t(_cx, "favorites.noGroups"))
                        .into_any_element()]
                } else {
                    groups
                        .iter()
                        .map(|g| {
                            let count = clusters
                                .iter()
                                .filter(|c| c.group_id == Some(g.id))
                                .count();
                            let entity_del = entity.clone();
                            let entity_edit = entity.clone();
                            let gid = g.id;
                            let gname = g.name.clone();
                            let gname_edit = g.name.clone();
                            let gdesc_edit = g.description.clone();
                            h_flex()
                                .items_center()
                                .justify_between()
                                .py_2()
                                .border_b_1()
                                .border_color(border)
                                .child(
                                    v_flex()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_semibold()
                                                .child(format!("{} ({} clusters)", g.name, count)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(muted)
                                                .child(
                                                    g.description
                                                        .clone()
                                                        .unwrap_or_else(|| t(_cx, "clusters.noDescription")),
                                                ),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new(("edit-group", gid as usize))
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::ALargeSmall)
                                                .on_click(move |_, window, cx| {
                                                    entity_edit.update(cx, |this, cx| {
                                                        window.close_dialog(cx);
                                                        this.open_group_form(
                                                            Some(crate::pages::clusters::GroupInfo {
                                                                id: gid,
                                                                name: gname_edit.clone(),
                                                                description: gdesc_edit.clone(),
                                                            }),
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }),
                                        )
                                        .child(
                                            Button::new(("del-group", gid as usize))
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Delete)
                                                .on_click(move |_, window, cx| {
                                                    entity_del.update(cx, |this, cx| {
                                                        this.delete_group(gid, gname.clone(), window, cx)
                                                    });
                                                }),
                                        ),
                                )
                                .into_any_element()
                        })
                        .collect()
                };
                dialog
                    .title(title.clone())
                    .w(px(520.0))
                    .child(
                        v_flex()
                            .gap_1()
                            .children(rows)
                            .child(
                                div().pt_2().child(
                                    Button::new("add-group-inline")
                                        .primary()
                                        .xsmall()
                                        .icon(IconName::Plus)
                                        .label(t(_cx, "clusters.addGroup"))
                                        .on_click(move |_, window, cx| {
                                            entity_add.update(cx, |this, cx| {
                                                window.close_dialog(cx);
                                                this.open_group_form(None, window, cx);
                                            });
                                        }),
                                ),
                            ),
                    )
                    .alert()
        });
    }

    /// 打开分组创建/编辑对话框
    fn open_group_form(&mut self, edit: Option<GroupInfo>, window: &mut Window, cx: &mut Context<Self>) {
        let edit_name = edit.as_ref().map(|g| g.name.clone());
        let edit_desc = edit.as_ref().and_then(|g| g.description.clone());
        let edit_id = edit.as_ref().map(|g| g.id);

        let name_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "clusters.groupNamePlaceholder"));
            if let Some(n) = edit_name {
                state.set_value(n, window, cx);
            }
            state
        });
        let desc_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "clusters.groupDescPlaceholder"));
            if let Some(d) = edit_desc {
                state.set_value(d, window, cx);
            }
            state
        });

        self.group_form = Some(GroupForm {
            id: edit_id,
            name: name_state.clone(),
            description: desc_state.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "clusters.addGroup");
        let name_label = t(cx, "clusters.groupName");
        let desc_label = t(cx, "clusters.groupDescription");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title(title.clone())
                .w(px(480.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(field_row(&name_label, Input::new(&name_state).into_any_element()))
                        .child(field_row(&desc_label, Input::new(&desc_state).into_any_element())),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.submit_group_form(cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn submit_group_form(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.group_form.take() else { return };
        let name = form.name.read(cx).value().to_string();
        let description = form.description.read(cx).value().to_string();
        if name.trim().is_empty() {
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let is_create = form.id.is_none();
        let method = if is_create { "cluster_group.create" } else { "cluster_group.update" };
        let mut params = json!({ "name": name.trim(), "description": description.trim() });
        if let Some(id) = form.id {
            params["id"] = json!(id);
        }

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, method, params).await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "clusters.groupCreated")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    fn delete_group(&mut self, id: i64, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let title = t(cx, "clusters.deleteGroupTitle");
        window.open_dialog(cx, move |dialog, _window, _cx| {
                let entity = entity.clone();
                dialog
                    .confirm()
                    .title(title.clone())
                    .child(name.clone())
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |_this, cx| {
                            let rt = TokioRuntime::handle(cx);
                            let Some(state) = Backend::state(cx) else { return };
                            cx.spawn(async move |this, cx| {
                                let result = crate::service::call(
                                    &rt,
                                    state,
                                    "cluster_group.delete",
                                    json!({ "id": id }),
                                )
                                .await;
                                cx.update(|cx| match result {
                                    Ok(_) => notify(cx, NotificationType::Success, t(cx, "clusters.groupDeleted")),
                                    Err(e) => notify(cx, NotificationType::Error, e),
                                })
                                .ok();
                                this.update(cx, |this, cx| this.reload(cx)).ok();
                            })
                            .detach();
                        });
                        true
                    })
        });
    }

    /// 查看 Topics（导航到 Topics 页）
    fn view_topics(&self, cluster: String, cx: &mut Context<Self>) {
        cx.emit(crate::components::navigator::NavEvent::OpenTopics { cluster });
    }

    fn group_name(&self, group_id: Option<i64>) -> Option<String> {
        group_id.and_then(|gid| {
            self.groups
                .iter()
                .find(|g| g.id == gid)
                .map(|g| g.name.clone())
        })
    }

    fn render_card(&self, c: &ClusterInfo, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let id = c.id;
        let name_del = c.name.clone();
        let cluster_topic = c.name.clone();
        let cluster_view = c.name.clone();
        let cluster_refresh = c.name.clone();
        let testing = self.testing_id == Some(id);
        let group_name = self.group_name(c.group_id);
        let created = c.created_at.split('T').next().unwrap_or("").to_string();
        let cluster_edit = c.clone();

        v_flex()
            .w(px(360.0))
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .bg(theme.secondary)
            .child(
                // 头部：图标 + 名称 + 徽标 + 编辑/删除
                h_flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .child(
                        div()
                            .size_8()
                            .rounded_md()
                            .bg(theme.primary.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::Building2).text_color(theme.primary)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .font_semibold()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(c.name.clone()),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .children(group_name.map(|g| {
                                        div()
                                            .text_xs()
                                            .px_1()
                                            .rounded_md()
                                            .bg(theme.muted)
                                            .child(g)
                                            .into_any_element()
                                    }))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child(format!("{} {}", t(cx, "clusters.createdDate"), created)),
                                    ),
                            ),
                    )
                    .child(
                        Button::new(("edit", id as usize))
                            .ghost()
                            .xsmall()
                            .icon(IconName::ALargeSmall)
                            .tooltip(t(cx, "common.edit"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_form(Some(cluster_edit.clone()), window, cx);
                            })),
                    )
                    .child(
                        Button::new(("delete", id as usize))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Delete)
                            .tooltip(t(cx, "common.delete"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.confirm_delete(id, name_del.clone(), window, cx);
                            })),
                    ),
            )
            .child(
                // 正文：Brokers / Timeouts
                v_flex()
                    .gap_1()
                    .px_3()
                    .pb_2()
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(t(cx, "clusters.brokers")),
                            )
                            .child(div().text_xs().child(c.brokers.clone())),
                    )
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(t(cx, "clusters.timeoutsLabel")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .child(format!(
                                        "Request: {}ms  Operation: {}ms",
                                        c.request_timeout_ms, c.operation_timeout_ms
                                    )),
                            ),
                    )
                    .children(c.conn_error.as_ref().map(|err| {
                        let err_msg = err.clone();
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .text_color(theme.danger)
                                    .overflow_hidden()
                                    .child(err_msg),
                            )
                            .child(
                                Button::new(("retry-conn", id as usize))
                                    .ghost()
                                    .xsmall()
                                    .label(t(cx, "clusters.retry"))
                                    .loading(testing)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.test_connection(id, cx);
                                    })),
                            )
                            .into_any_element()
                    })),
            )
            .child(
                // 底部操作按钮
                h_flex()
                    .gap_1()
                    .p_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        Button::new(("create-topic", id as usize))
                            .primary()
                            .xsmall()
                            .icon(IconName::Plus)
                            .label(t(cx, "clusters.createTopic"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_create_topic(cluster_topic.clone(), window, cx);
                            })),
                    )
                    .child(
                        Button::new(("test", id as usize))
                            .outline()
                            .xsmall()
                            .label(t(cx, "clusters.test"))
                            .loading(testing)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.test_connection(id, cx);
                            })),
                    )
                    .child(
                        Button::new(("view-topics", id as usize))
                            .outline()
                            .xsmall()
                            .label(t(cx, "clusters.viewTopics"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.view_topics(cluster_view.clone(), cx);
                            })),
                    )
                    .child(
                        Button::new(("refresh-topics", id as usize))
                            .outline()
                            .xsmall()
                            .icon(IconName::Redo2)
                            .tooltip(t(cx, "clusters.refreshTopics"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.refresh_topics(cluster_refresh.clone(), cx);
                            })),
                    ),
            )
    }
}

impl gpui::EventEmitter<crate::components::navigator::NavEvent> for ClustersPage {}

/// 表单字段行：标签 + 控件
fn field_row(label: &str, control: AnyElement) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_sm().child(label.to_string()))
        .child(control)
}

impl Render for ClustersPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        // 头部
        let header = h_flex()
            .items_center()
            .justify_between()
            .mb_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(back_button(cx))
                            .child(div().text_xl().font_semibold().child(t(cx, "clusters.title"))),
                    )
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
                        Button::new("manage-groups")
                            .outline()
                            .label(t(cx, "clusters.manageGroups"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_manage_groups(window, cx);
                            })),
                    )
                    .child(
                        Button::new("create")
                            .primary()
                            .icon(IconName::Plus)
                            .label(t(cx, "clusters.addCluster"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_form(None, window, cx);
                            })),
                    ),
            );

        // 分组 pills
        let group_pills: AnyElement = if self.groups.is_empty() {
            div().into_any_element()
        } else {
            h_flex()
                .gap_1()
                .items_center()
                .mb_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{}:", t(cx, "clusters.group"))),
                )
                .child(
                    Button::new("pill-all")
                        .ghost()
                        .xsmall()
                        .label(t(cx, "common.all"))
                        .when(self.selected_group.is_none(), |b| b.outline())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.selected_group = None;
                            cx.notify();
                        })),
                )
                .children(self.groups.iter().map(|g| {
                    let gid = g.id;
                    let active = self.selected_group == Some(gid);
                    Button::new(("pill", gid as usize))
                        .ghost()
                        .xsmall()
                        .label(g.name.clone())
                        .when(active, |b| b.outline())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected_group = Some(gid);
                            cx.notify();
                        }))
                }))
                .into_any_element()
        };

        // 搜索框
        let search_bar = div().mb_3().child(Input::new(&self.search_input));

        // 内容
        let content: AnyElement = if self.loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if let Some(err) = &self.error {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .child(div().text_color(theme.danger).child(t(cx, "clusters.connectionError")))
                .child(div().text_sm().child(err.clone()))
                .child(
                    Button::new("retry")
                        .primary()
                        .label(t(cx, "clusters.retry"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.loading = true;
                            cx.notify();
                            this.reload(cx);
                        })),
                )
                .into_any_element()
        } else {
            let visible: Vec<&ClusterInfo> = self
                .clusters
                .iter()
                .filter(|c| self.selected_group.is_none() || c.group_id == self.selected_group)
                .collect();
            if visible.is_empty() {
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .text_color(theme.muted_foreground)
                    .child(t(cx, "common.noData"))
                    .child(
                        Button::new("create-empty")
                            .primary()
                            .label(t(cx, "clusters.addCluster"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_form(None, window, cx);
                            })),
                    )
                    .into_any_element()
            } else {
                div()
                    .id("clusters-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .child(
                        h_flex()
                            .flex_wrap()
                            .gap_3()
                            .children(visible.iter().map(|c| {
                                self.render_card(c, cx).into_any_element()
                            })),
                    )
                    .into_any_element()
            }
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(group_pills)
            .child(search_bar)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
