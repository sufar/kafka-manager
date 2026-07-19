//! Topic 管理页：集群选择、搜索、列表、创建、删除、详情、收藏

use std::collections::HashSet;

use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::i18n::t;
use crate::pages::clusters::notify;
use crate::state::{Backend, TokioRuntime};

/// 集群下拉选项（value 为集群名，Topic API 以集群名作为 cluster_id）
#[derive(Clone)]
struct ClusterOption {
    name: SharedString,
}

impl gpui_component::select::SelectItem for ClusterOption {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

type ClusterSelect = SelectState<SearchableVec<ClusterOption>>;

/// Topic 详情
#[derive(Clone, Debug)]
struct PartitionInfo {
    id: i32,
    leader: i32,
    replicas: Vec<i32>,
    isr: Vec<i32>,
}

/// 创建 Topic 表单
struct CreateTopicForm {
    name: Entity<InputState>,
    partitions: Entity<InputState>,
    replication: Entity<InputState>,
}

pub struct TopicsPage {
    cluster_state: Option<Entity<ClusterSelect>>,
    search_state: Entity<InputState>,
    topics: Vec<String>,
    favorites: HashSet<String>,
    loading: bool,
    error: Option<String>,
    create_form: Option<CreateTopicForm>,
    _subscriptions: Vec<Subscription>,
}

impl TopicsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "common.search"))
        });

        let mut subscriptions = Vec::new();
        // 搜索输入 → 重新加载（后端搜索）
        subscriptions.push(cx.subscribe(
            &search_state,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.load_topics(cx);
                }
            },
        ));

        let this = Self {
            cluster_state: None,
            search_state,
            topics: Vec::new(),
            favorites: HashSet::new(),
            loading: true,
            error: None,
            create_form: None,
            _subscriptions: subscriptions,
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

    /// 加载集群列表并创建集群选择器
    fn load_clusters(&self, window: &mut Window, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn_in(window, async move |this, cx| {
            let result = crate::service::call(&rt, state, "cluster.list", json!({})).await;
            let options: Vec<ClusterOption> = result
                .ok()
                .and_then(|value| {
                    Some(
                        value
                            .as_array()?
                            .iter()
                            .filter_map(|c| {
                                Some(ClusterOption {
                                    name: c.get("name")?.as_str()?.to_string().into(),
                                })
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default();

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
                // 集群切换 → 重新加载 Topic
                let sub = cx.subscribe(
                    &select,
                    |this, _, event: &SelectEvent<SearchableVec<ClusterOption>>, cx| {
                        if matches!(event, SelectEvent::Confirm(Some(_))) {
                            this.load_topics(cx);
                        }
                    },
                );
                this._subscriptions.push(sub);
                this.cluster_state = Some(select);
                this.loading = false;
                cx.notify();
                this.load_topics(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 加载 Topic 列表（含收藏状态）
    fn load_topics(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let search = self.search_state.read(cx).value().to_string();

        cx.spawn(async move |this, cx| {
            let topics = crate::service::call(
                &rt,
                state.clone(),
                "topic.list_with_cluster",
                json!({ "cluster_id": cluster, "search": search, "limit": 100000 }),
            )
            .await;
            let favorites = crate::service::call(&rt, state, "favorite.list", json!({})).await;

            this.update(cx, |this, cx| {
                this.loading = false;
                match topics {
                    Ok(value) => {
                        this.error = None;
                        this.topics = value
                            .get("topics")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|t| t.get("name")?.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                    }
                    Err(e) => this.error = Some(e),
                }
                // 收藏集合（cluster_id\u{1}topic_name）
                if let Ok(value) = favorites {
                    this.favorites = value
                        .as_array()
                        .map(|groups| {
                            groups
                                .iter()
                                .flat_map(|g| {
                                    g.get("items")
                                        .and_then(|i| i.as_array())
                                        .cloned()
                                        .unwrap_or_default()
                                })
                                .filter_map(|item| {
                                    let cluster = item.get("cluster_id")?.as_str()?.to_string();
                                    let topic = item.get("topic_name")?.as_str()?.to_string();
                                    Some(format!("{}\u{1}{}", cluster, topic))
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

    /// 刷新（从集群重新拉取元数据）
    fn refresh_topics(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let _ = crate::service::call(&rt, state, "topic.refresh", json!({ "cluster_id": cluster }))
                .await;
            this.update(cx, |this, cx| this.load_topics(cx)).ok();
        })
        .detach();
    }

    /// 切换收藏
    fn toggle_favorite(&mut self, topic: String, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let key = format!("{}\u{1}{}", cluster, topic);
        let is_fav = self.favorites.contains(&key);
        // 乐观更新
        if is_fav {
            self.favorites.remove(&key);
        } else {
            self.favorites.insert(key);
        }
        cx.notify();

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = if is_fav {
                crate::service::call(
                    &rt,
                    state.clone(),
                    "favorite.delete_by_topic",
                    json!({ "cluster_id": cluster, "topic_name": topic }),
                )
                .await
            } else {
                // 确保有收藏分组：取第一个分组，没有则创建
                let group_id = match crate::service::call(&rt, state.clone(), "favorite.group.list", json!({})).await {
                    Ok(value) => {
                        let first = value
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|g| g.get("id"))
                            .and_then(|id| id.as_i64());
                        match first {
                            Some(id) => Some(id),
                            None => crate::service::call(
                                &rt,
                                state.clone(),
                                "favorite.group.create",
                                json!({ "name": "默认分组" }),
                            )
                            .await
                            .ok()
                            .and_then(|v| v.get("id").and_then(|id| id.as_i64())),
                        }
                    }
                    Err(_) => None,
                };
                match group_id {
                    Some(gid) => {
                        crate::service::call(
                            &rt,
                            state.clone(),
                            "favorite.create",
                            json!({ "group_id": gid, "cluster_id": cluster, "topic_name": topic }),
                        )
                        .await
                    }
                    None => Err("No favorite group available".to_string()),
                }
            };
            if let Err(e) = result {
                cx.update(|cx| notify(cx, NotificationType::Error, e)).ok();
                // 回滚乐观更新
                this.update(cx, |this, cx| this.load_topics(cx)).ok();
            }
        })
        .detach();
    }

    /// 删除 Topic（确认对话框）
    fn confirm_delete(&mut self, topic: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let entity = cx.entity();
        let title = t(cx, "topics.confirmDeleteTitle");
        let message = topic.clone();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let cluster = cluster.clone();
            let topic = message.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(topic.clone())
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                .on_ok(move |_, window, cx| {
                    entity.update(cx, |this, cx| {
                        this.delete_topic(cluster.clone(), topic.clone(), window, cx)
                    });
                    true
                })
        });
    }

    fn delete_topic(
        &mut self,
        cluster: String,
        topic: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic.delete",
                json!({ "cluster_id": cluster, "topic": topic }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "topics.deletedSuccess")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.load_topics(cx)).ok();
        })
        .detach();
    }

    /// 查看 Topic 详情（分区信息）
    fn open_detail(&mut self, topic: String, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let title = t(cx, "topics.topicDetails");

        cx.spawn(async move |_this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic.get",
                json!({ "cluster_id": cluster, "name": topic }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(value) => {
                    let name = value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let partitions: Vec<PartitionInfo> = value
                        .get("partitions")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|p| {
                                    Some(PartitionInfo {
                                        id: p.get("id")?.as_i64()? as i32,
                                        leader: p.get("leader")?.as_i64()? as i32,
                                        replicas: p
                                            .get("replicas")?
                                            .as_array()?
                                            .iter()
                                            .filter_map(|r| r.as_i64().map(|v| v as i32))
                                            .collect(),
                                        isr: p
                                            .get("isr")?
                                            .as_array()?
                                            .iter()
                                            .filter_map(|r| r.as_i64().map(|v| v as i32))
                                            .collect(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    for w in cx.windows() {
                        let title = title.clone();
                        let name = name.clone();
                        let partitions = partitions.clone();
                        let _ = w.update(cx, |_, window, cx| {
                            let border = cx.theme().border;
                            let partitions = partitions.clone();
                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                let rows: Vec<AnyElement> = partitions
                                    .iter()
                                    .map(|p| {
                                        h_flex()
                                            .gap_4()
                                            .py_1()
                                            .border_b_1()
                                            .border_color(border)
                                            .child(div().w_20().child(format!("Partition {}", p.id)))
                                            .child(div().w_24().child(format!("Leader: {}", p.leader)))
                                            .child(
                                                div()
                                                    .w_32()
                                                    .child(format!("Replicas: {:?}", p.replicas)),
                                            )
                                            .child(div().child(format!("ISR: {:?}", p.isr)))
                                            .into_any_element()
                                    })
                                    .collect();
                                dialog
                                    .title(format!("{} — {}", title, name))
                                    .w(px(640.0))
                                    .child(v_flex().children(rows))
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

    /// 打开创建 Topic 对话框
    fn open_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "topics.topicNamePlaceholder"))
        });
        let partitions_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("1");
            state.set_value("1".to_string(), window, cx);
            state
        });
        let replication_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("1");
            state.set_value("1".to_string(), window, cx);
            state
        });

        self.create_form = Some(CreateTopicForm {
            name: name_state.clone(),
            partitions: partitions_state.clone(),
            replication: replication_state.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "topics.createTopic");
        let name_label = t(cx, "topics.topicName");
        let partitions_label = t(cx, "topics.numPartitions");
        let replication_label = t(cx, "topics.replicationFactor");

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
                            &partitions_label,
                            Input::new(&partitions_state).into_any_element(),
                        ))
                        .child(field_row(
                            &replication_label,
                            Input::new(&replication_state).into_any_element(),
                        )),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, window, cx| {
                    entity.update(cx, |this, cx| this.submit_create(window, cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn submit_create(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.create_form.take() else { return };
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let name = form.name.read(cx).value().to_string();
        let num_partitions: i32 = form.partitions.read(cx).value().parse().unwrap_or(1);
        let replication_factor: i32 = form.replication.read(cx).value().parse().unwrap_or(1);

        if name.trim().is_empty() {
            let msg = t(cx, "topics.validationTopicNameRequired");
            notify(cx, NotificationType::Error, msg);
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic.create",
                json!({
                    "cluster_id": cluster,
                    "name": name.trim(),
                    "num_partitions": num_partitions,
                    "replication_factor": replication_factor,
                }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "topics.createdSuccess")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.load_topics(cx)).ok();
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

/// 单个 Topic 行
fn topic_row(
    entity: Entity<TopicsPage>,
    ix: usize,
    topic: String,
    is_fav: bool,
    border_color: Hsla,
) -> AnyElement {
    let entity_fav = entity.clone();
    let entity_detail = entity.clone();
    let topic_for_fav = topic.clone();
    let topic_for_detail = topic.clone();
    let topic_for_delete = topic.clone();

    h_flex()
        .items_center()
        .justify_between()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(border_color)
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new(("fav", ix))
                        .ghost()
                        .icon(if is_fav { IconName::Star } else { IconName::StarOff })
                        .on_click(move |_, _, cx| {
                            entity_fav.update(cx, |this, cx| {
                                this.toggle_favorite(topic_for_fav.clone(), cx)
                            });
                        }),
                )
                .child(
                    Button::new(("name", ix))
                        .ghost()
                        .label(topic.clone())
                        .on_click(move |_, window, cx| {
                            entity_detail.update(cx, |this, cx| {
                                this.open_detail(topic_for_detail.clone(), window, cx)
                            });
                        }),
                ),
        )
        .child(
            Button::new(("del", ix))
                .ghost()
                .icon(IconName::Delete)
                .on_click(move |_, window, cx| {
                    entity.update(cx, |this, cx| {
                        this.confirm_delete(topic_for_delete.clone(), window, cx)
                    });
                }),
        )
        .into_any_element()
}

impl Render for TopicsPage {
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
                    .child(div().text_xl().font_semibold().child(t(cx, "topics.title")))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "topics.description")),
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
                                this.refresh_topics(cx);
                            })),
                    )
                    .child(
                        Button::new("create")
                            .primary()
                            .label(t(cx, "common.create"))
                            .disabled(!has_cluster)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_create(window, cx);
                            })),
                    ),
            );

        let toolbar = h_flex()
            .gap_2()
            .mb_3()
            .children(
                self.cluster_state
                    .as_ref()
                    .map(|state| div().w_48().child(Select::new(state)).into_any_element()),
            )
            .child(div().w_64().child(Input::new(&self.search_state)));

        let content: AnyElement = if self.cluster_state.is_none() || (self.loading && self.topics.is_empty()) {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if !has_cluster {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "topics.validationSelectCluster"))
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
        } else if self.topics.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "common.noData"))
                .into_any_element()
        } else {
            let entity = cx.entity();
            let topics = std::rc::Rc::new(self.topics.clone());
            let favorites = std::rc::Rc::new(self.favorites.clone());
            let cluster = self.selected_cluster(cx).unwrap_or_default();
            let border = theme.border;
            let count = topics.len();

            uniform_list("topic-list", count, move |range, _window, _cx| {
                range
                    .map(|ix| {
                        let topic = topics[ix].clone();
                        let is_fav = favorites.contains(&format!("{}\u{1}{}", cluster, topic));
                        topic_row(entity.clone(), ix, topic, is_fav, border)
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
