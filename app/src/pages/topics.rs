//! Topics 页：与旧版 TopicsView 一致
//!
//! 头部：标题 + 集群名 + [刷新] [创建]（无集群下拉，集群由导航器预选）
//! 卡片：搜索头（图标+输入+计数）+ 两列表格（名称+操作）虚拟滚动
//! 行：收藏星 + 名称 + 删除；删除需输入 Topic 名称确认

use std::collections::HashSet;
use std::rc::Rc;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::notify;
use crate::i18n::t;
use crate::state::{Backend, TokioRuntime};

/// 创建 Topic 表单
struct CreateTopicForm {
    name: Entity<InputState>,
    partitions: Entity<InputState>,
    replication: Entity<InputState>,
}

impl EventEmitter<crate::components::navigator::NavEvent> for TopicsPage {}

pub struct TopicsPage {
    cluster: Option<String>,
    search_input: Entity<InputState>,
    confirm_input: Option<Entity<InputState>>,
    topics: Vec<String>,
    favorites: HashSet<String>,
    loading: bool,
    refreshing: bool,
    error: Option<String>,
    create_form: Option<CreateTopicForm>,
    _subscriptions: Vec<Subscription>,
}

impl TopicsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "common.search"))
        });
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.load_topics(cx);
                }
            },
        ));

        Self {
            cluster: None,
            search_input,
            confirm_input: None,
            topics: Vec::new(),
            favorites: HashSet::new(),
            loading: false,
            refreshing: false,
            error: None,
            create_form: None,
            _subscriptions: subscriptions,
        }
    }

    /// 外部导航：预选集群（由工作区调用）
    pub fn select_cluster(&mut self, cluster: String, cx: &mut Context<Self>) {
        self.cluster = Some(cluster);
        self.loading = true;
        cx.notify();
        self.load_topics(cx);
    }

    /// 加载 Topic 列表（含收藏状态）
    fn load_topics(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let search = self.search_input.read(cx).value().to_string();

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

    fn refresh_topics(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        self.refreshing = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let _ = crate::service::call(&rt, state, "topic.refresh", json!({ "cluster_id": cluster }))
                .await;
            this.update(cx, |this, cx| {
                this.refreshing = false;
                cx.notify();
                this.load_topics(cx);
            })
            .ok();
        })
        .detach();
    }

    fn toggle_favorite(&mut self, topic: String, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let key = format!("{}\u{1}{}", cluster, topic);
        let is_fav = self.favorites.contains(&key);
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
                this.update(cx, |this, cx| this.load_topics(cx)).ok();
            }
        })
        .detach();
    }

    /// 打开删除确认对话框（输入 Topic 名称确认，与旧版 DeleteTopicDialog 一致）
    fn open_delete_confirm(&mut self, topic: String, window: &mut Window, cx: &mut Context<Self>) {
        let confirm_state = cx.new(|cx| InputState::new(window, cx));
        self.confirm_input = Some(confirm_state.clone());

        let entity = cx.entity();
        let title = t(cx, "topics.confirmDeleteTitle");
        let hint = t(cx, "topics.confirmDeleteHint");
        let mismatch = t(cx, "topics.confirmDeleteMatchError");

        window.open_dialog(cx, move |dialog, _window, cx| {
            let entity = entity.clone();
            let topic = topic.clone();
            let mismatch = mismatch.clone();
            let confirm_state = confirm_state.clone();
            dialog
                .confirm()
                .title(title.clone())
                .w(px(480.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(div().text_sm().child(hint.clone()))
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .child(topic.clone()),
                        )
                        .child(Input::new(&confirm_state))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().danger)
                                .child(mismatch.clone()),
                        ),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                .on_ok(move |_, _window, cx| {
                    let typed = confirm_state.read(cx).value().to_string();
                    if typed.trim() == topic {
                        entity.update(cx, |this, cx| {
                            this.delete_topic(topic.clone(), cx);
                        });
                        true
                    } else {
                        false // 名称不匹配不关闭
                    }
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn delete_topic(&mut self, topic: String, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
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

    /// 打开创建 Topic 对话框
    fn open_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                        .child(field_row(&partitions_label, Input::new(&partitions_state).into_any_element()))
                        .child(field_row(&replication_label, Input::new(&replication_state).into_any_element())),
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
        let Some(cluster) = self.cluster.clone() else { return };
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
    warning: Hsla,
) -> AnyElement {
    let topic_fav = topic.clone();
    let topic_del = topic.clone();
    let entity_fav = entity.clone();

    h_flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_1p5()
        .border_b_1()
        .border_color(border_color)
        .child(
            Button::new(("fav", ix))
                .ghost()
                .xsmall()
                .icon(if is_fav { IconName::Star } else { IconName::StarOff })
                .when(is_fav, |b| b.text_color(warning))
                .on_click(move |_, _, cx| {
                    entity_fav.update(cx, |this, cx| {
                        this.toggle_favorite(topic_fav.clone(), cx)
                    });
                }),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_xs()
                .cursor_pointer()
                .child(topic.clone())
                .id(("topic-name", ix))
                .on_click({
                    let topic_open = topic.clone();
                    let entity = entity.clone();
                    move |event: &ClickEvent, _, cx| {
                        // 双击打开消息页（与旧版行双击联动一致）
                        if event.click_count() == 2 {
                            let cluster = entity.read(cx).cluster.clone().unwrap_or_default();
                            entity.update(cx, |_this, cx| {
                                cx.emit(crate::components::navigator::NavEvent::OpenMessages {
                                    cluster,
                                    topic: topic_open.clone(),
                                });
                            });
                        }
                    }
                }),
        )
        .child(
            Button::new(("del", ix))
                .ghost()
                .xsmall()
                .icon(IconName::Delete)
                .on_click(move |_, window, cx| {
                    entity.update(cx, |this, cx| {
                        this.open_delete_confirm(topic_del.clone(), window, cx)
                    });
                }),
        )
        .into_any_element()
}

impl Render for TopicsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let has_cluster = self.cluster.is_some();

        // 头部
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
                            .child(if let Some(cluster) = &self.cluster {
                                format!("{}: {}", t(cx, "clusters.clusters"), cluster)
                            } else {
                                t(cx, "topics.description")
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("refresh")
                            .outline()
                            .icon(IconName::Redo2)
                            .label(t(cx, "common.refresh"))
                            .loading(self.refreshing)
                            .disabled(!has_cluster)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_topics(cx);
                            })),
                    )
                    .child(
                        Button::new("create")
                            .primary()
                            .icon(IconName::Plus)
                            .label(t(cx, "common.create"))
                            .disabled(!has_cluster)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_create(window, cx);
                            })),
                    ),
            );

        let content: AnyElement = if !has_cluster {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(theme.muted_foreground)
                .child(t(cx, "common.noData"))
                .child(div().text_sm().child(t(cx, "topics.description")))
                .into_any_element()
        } else if self.loading && self.topics.is_empty() {
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
        } else {
            // 卡片：搜索头 + 表格
            let card_header = h_flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(theme.border)
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(IconName::Search)
                                .size_4()
                                .text_color(theme.muted_foreground),
                        )
                        .child(div().w_64().child(Input::new(&self.search_input).small())),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{} topics", self.topics.len())),
                );

            let table_header = h_flex()
                .px_3()
                .py_1()
                .bg(theme.table_head)
                .text_xs()
                .font_semibold()
                .child(div().flex_1().child(t(cx, "topics.topicName")))
                .child(div().w_16().text_right().child(t(cx, "messages.actions")));

            let body: AnyElement = if self.topics.is_empty() {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t(cx, "common.noData"))
                    .into_any_element()
            } else {
                let entity = cx.entity();
                let cluster = self.cluster.clone().unwrap_or_default();
                let favorites = Rc::new(self.favorites.clone());
                let topics = Rc::new(self.topics.clone());
                let border = theme.border;
                let warning = theme.warning;
                let count = topics.len();

                uniform_list("topics-list", count, move |range, _window, _cx| {
                    range
                        .map(|ix| {
                            let topic = topics[ix].clone();
                            let is_fav = favorites.contains(&format!("{}\u{1}{}", cluster, topic));
                            topic_row(entity.clone(), ix, topic, is_fav, border, warning)
                        })
                        .collect()
                })
                .into_any_element()
            };

            v_flex()
                .size_full()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .child(card_header)
                .child(table_header)
                .child(div().flex_1().overflow_hidden().child(body))
                .into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
