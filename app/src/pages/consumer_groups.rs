//! 消费组页：列表、偏移量详情、重置偏移、删除

use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::pages::clusters::notify;
use crate::state::{Backend, TokioRuntime};

#[derive(Clone, Debug)]
struct GroupInfo {
    name: String,
    cluster: String,
    topics: Vec<String>,
}

#[derive(Clone, Debug)]
struct OffsetInfo {
    topic: String,
    partition: i32,
    start_offset: i64,
    end_offset: i64,
    committed_offset: i64,
    lag: i64,
}

pub struct ConsumerGroupsPage {
    cluster_state: Option<Entity<SelectState<SearchableVec<StringOption>>>>,
    search_input: Entity<InputState>,
    groups: Vec<GroupInfo>,
    loading: bool,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl ConsumerGroupsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "common.search"))
        });
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.load_groups(cx);
                }
            },
        ));

        let this = Self {
            cluster_state: None,
            search_input,
            groups: Vec::new(),
            loading: true,
            error: None,
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
                            this.load_groups(cx);
                        }
                    },
                );
                this._subscriptions.push(sub);
                this.cluster_state = Some(select);
                this.loading = false;
                cx.notify();
                this.load_groups(cx);
            })
            .ok();
        })
        .detach();
    }

    fn load_groups(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let search = self.search_input.read(cx).value().to_string();

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "consumer_group.list",
                json!({ "cluster_ids": [cluster], "search": search, "limit": 100000 }),
            )
            .await;

            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(value) => {
                        this.error = None;
                        this.groups = value
                            .get("groups")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|g| {
                                        Some(GroupInfo {
                                            name: g.get("group_name")?.as_str()?.to_string(),
                                            cluster: g
                                                .get("cluster_id")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string(),
                                            topics: g
                                                .get("topics")
                                                .and_then(|v| v.as_array())
                                                .map(|ts| {
                                                    ts.iter()
                                                        .filter_map(|t| {
                                                            t.as_str().map(|s| s.to_string())
                                                        })
                                                        .collect()
                                                })
                                                .unwrap_or_default(),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                    }
                    Err(e) => this.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn refresh_groups(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let _ = crate::service::call(
                &rt,
                state,
                "consumer_group.refresh",
                json!({ "cluster_id": cluster }),
            )
            .await;
            this.update(cx, |this, cx| this.load_groups(cx)).ok();
        })
        .detach();
    }

    /// 打开偏移量详情对话框
    fn open_offsets(&mut self, group: GroupInfo, _window: &mut Window, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let title = t(cx, "consumerGroups.offset");

        cx.spawn(async move |_this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "consumer_group.offsets",
                json!({ "cluster_id": group.cluster, "group_name": group.name }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(value) => {
                    let offsets: Vec<OffsetInfo> = value
                        .get("offsets")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|o| {
                            Some(OffsetInfo {
                                topic: o.get("topic")?.as_str()?.to_string(),
                                partition: o.get("partition")?.as_i64()? as i32,
                                start_offset: o.get("start_offset")?.as_i64()?,
                                end_offset: o.get("end_offset")?.as_i64()?,
                                committed_offset: o.get("committed_offset")?.as_i64()?,
                                lag: o.get("lag")?.as_i64()?,
                                            })
                        })
                        .collect();
                    let group_name = group.name.clone();
                    let cluster = group.cluster.clone();
                    for w in cx.windows() {
                        let offsets = offsets.clone();
                        let title = title.clone();
                        let group_name = group_name.clone();
                        let cluster = cluster.clone();
                        let _ = w.update(cx, |_, window, cx| {
                            let border = cx.theme().border;
                            window.open_dialog(cx, move |dialog, _window, cx| {
                                let rows: Vec<AnyElement> = offsets
                                    .iter()
                                    .map(|o| render_offset_row(o, &group_name, &cluster, border, cx))
                                    .collect();
                                dialog
                                    .title(format!("{} — {}", title, group_name))
                                    .w(px(860.0))
                                    .child(
                                        v_flex()
                                            .gap_0()
                                            .child(render_offset_header(cx))
                                            .children(rows),
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

    /// 删除消费组（确认对话框）
    fn confirm_delete(&mut self, group: GroupInfo, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let title = t(cx, "common.delete");
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let group = group.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(group.name.clone())
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.delete_group(group.clone(), cx));
                    true
                })
        });
    }

    fn delete_group(&mut self, group: GroupInfo, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "consumer_group.delete",
                json!({ "cluster_id": group.cluster, "group": group.name }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "common.success")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.load_groups(cx)).ok();
        })
        .detach();
    }
}

fn render_offset_header(cx: &App) -> AnyElement {
    let theme = cx.theme();
    h_flex()
        .gap_2()
        .py_1()
        .text_xs()
        .font_semibold()
        .text_color(theme.muted_foreground)
        .border_b_1()
        .border_color(theme.border)
        .child(div().w_40().child("Topic"))
        .child(div().w_16().child(t(cx, "consumerGroups.partitions")))
        .child(div().w_20().child(t(cx, "consumerGroups.start")))
        .child(div().w_20().child(t(cx, "consumerGroups.end")))
        .child(div().w_20().child(t(cx, "consumerGroups.offset")))
        .child(div().w_16().child(t(cx, "consumerGroups.lag")))
        .child(div().flex_1().child(t(cx, "consumerGroups.resetOffset")))
        .into_any_element()
}

fn render_offset_row(
    o: &OffsetInfo,
    group_name: &str,
    cluster: &str,
    border: Hsla,
    cx: &mut App,
) -> AnyElement {
    let cluster_e = cluster.to_string();
    let cluster_l = cluster.to_string();
    let group_e = group_name.to_string();
    let group_l = group_name.to_string();
    let topic_e = o.topic.clone();
    let topic_l = o.topic.clone();
    let partition = o.partition;

    h_flex()
        .gap_2()
        .py_1()
        .text_xs()
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .w_40()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(o.topic.clone()),
        )
        .child(div().w_16().child(o.partition.to_string()))
        .child(div().w_20().child(o.start_offset.to_string()))
        .child(div().w_20().child(o.end_offset.to_string()))
        .child(div().w_20().child(o.committed_offset.to_string()))
        .child(div().w_16().child(o.lag.to_string()))
        .child(
            h_flex()
                .flex_1()
                .gap_1()
                .child(
                    Button::new(SharedString::from(format!("earliest-{}-{}", o.topic, o.partition)))
                        .ghost()
                        .label(t(cx, "consumerGroups.resetOffsetToEarliest"))
                        .on_click(move |_, _, cx| {
                            let rt = cx.global::<TokioRuntime>().0.clone();
                            let Some(state) = Backend::state(cx) else { return };
                            let cluster = cluster_e.clone();
                            let group = group_e.clone();
                            let topic = topic_e.clone();
                            cx.spawn(async move |cx| {
                                let result = crate::service::call(
                                    &rt,
                                    state,
                                    "consumer_group.reset_offset",
                                    json!({
                                        "cluster_id": cluster,
                                        "group_name": group,
                                        "topic": topic,
                                        "partition": partition,
                                        "reset_to": "earliest",
                                    }),
                                )
                                .await;
                                cx.update(|cx| match result {
                                    Ok(_) => notify(
                                        cx,
                                        NotificationType::Success,
                                        t(cx, "consumerGroups.offsetResetSuccess"),
                                    ),
                                    Err(e) => notify(cx, NotificationType::Error, e),
                                })
                                .ok();
                            })
                            .detach();
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("latest-{}-{}", o.topic, o.partition)))
                        .ghost()
                        .label(t(cx, "consumerGroups.resetOffsetToLatest"))
                        .on_click(move |_, _, cx| {
                            let rt = cx.global::<TokioRuntime>().0.clone();
                            let Some(state) = Backend::state(cx) else { return };
                            let cluster = cluster_l.clone();
                            let group = group_l.clone();
                            let topic = topic_l.clone();
                            cx.spawn(async move |cx| {
                                let result = crate::service::call(
                                    &rt,
                                    state,
                                    "consumer_group.reset_offset",
                                    json!({
                                        "cluster_id": cluster,
                                        "group_name": group,
                                        "topic": topic,
                                        "partition": partition,
                                        "reset_to": "latest",
                                    }),
                                )
                                .await;
                                cx.update(|cx| match result {
                                    Ok(_) => notify(
                                        cx,
                                        NotificationType::Success,
                                        t(cx, "consumerGroups.offsetResetSuccess"),
                                    ),
                                    Err(e) => notify(cx, NotificationType::Error, e),
                                })
                                .ok();
                            })
                            .detach();
                        }),
                ),
        )
        .into_any_element()
}

/// 消费组列表行
fn group_row(
    entity: Entity<ConsumerGroupsPage>,
    ix: usize,
    group: GroupInfo,
    border: Hsla,
) -> AnyElement {
    let entity_offsets = entity.clone();
    let group_offsets = group.clone();
    let group_delete = group.clone();

    h_flex()
        .items_center()
        .justify_between()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(border)
        .child(
            v_flex()
                .gap_1()
                .child(div().font_semibold().child(group.name.clone()))
                .child(
                    div()
                        .text_xs()
                        .child(format!("{} topics", group.topics.len())),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .child(
                    Button::new(("offsets", ix))
                        .outline()
                        .label(t_label_offsets())
                        .on_click(move |_, window, cx| {
                            entity_offsets.update(cx, |this, cx| {
                                this.open_offsets(group_offsets.clone(), window, cx)
                            });
                        }),
                )
                .child(
                    Button::new(("delete", ix))
                        .ghost()
                        .icon(IconName::Delete)
                        .on_click(move |_, window, cx| {
                            entity.update(cx, |this, cx| {
                                this.confirm_delete(group_delete.clone(), window, cx)
                            });
                        }),
                ),
        )
        .into_any_element()
}

fn t_label_offsets() -> &'static str {
    "Offsets"
}

impl Render for ConsumerGroupsPage {
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
                    .child(div().text_xl().font_semibold().child(t(cx, "consumerGroups.title")))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "consumerGroups.description")),
                    ),
            )
            .child(
                Button::new("refresh")
                    .outline()
                    .label(t(cx, "common.refresh"))
                    .disabled(!has_cluster)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.loading = true;
                        cx.notify();
                        this.refresh_groups(cx);
                    })),
            );

        let toolbar = h_flex()
            .gap_2()
            .mb_3()
            .children(
                self.cluster_state
                    .as_ref()
                    .map(|s| div().w_48().child(Select::new(s)).into_any_element()),
            )
            .child(div().w_64().child(Input::new(&self.search_input)));

        let content: AnyElement = if self.cluster_state.is_none() || (self.loading && self.groups.is_empty()) {
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
        } else if self.groups.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "consumerGroups.noData"))
                .into_any_element()
        } else {
            let entity = cx.entity();
            let groups = std::rc::Rc::new(self.groups.clone());
            let border = theme.border;
            let count = groups.len();

            uniform_list("group-list", count, move |range, _window, _cx| {
                range
                    .map(|ix| group_row(entity.clone(), ix, groups[ix].clone(), border))
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
