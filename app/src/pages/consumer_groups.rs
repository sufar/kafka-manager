//! 消费组详情页：与旧版 ConsumerGroupsView 一致（纯详情视图，无组列表）
//!
//! 头部：组名 + 集群 + 状态徽章 + Refresh + [Reset Offset] [Delete]
//! 表格：Topic | Partition | Start | End | Committed | Lag(颜色) | Last Commit
//! Reset Offset 对话框：Topic/Partition 联动下拉 + earliest/latest/offset/timestamp 四选项

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::navigator::NavEvent;
use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::components::notify;
use crate::state::{Backend, TokioRuntime};

/// 列宽（px）
#[derive(Clone, Copy)]
struct ColWidths {
    topic: f32,
    partition: f32,
    start: f32,
    end: f32,
    committed: f32,
    lag: f32,
}

impl Default for ColWidths {
    fn default() -> Self {
        Self {
            topic: 200.0,
            partition: 80.0,
            start: 96.0,
            end: 96.0,
            committed: 112.0,
            lag: 64.0,
        }
    }
}

/// 列宽拖拽状态
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ResizeCol {
    Topic,
    Partition,
    Start,
    End,
    Committed,
    Lag,
}

#[derive(Clone, Debug)]
struct OffsetInfo {
    topic: String,
    partition: i32,
    start_offset: i64,
    end_offset: i64,
    committed_offset: i64,
    lag: i64,
    last_commit_time: Option<i64>,
}

/// 重置偏移表单
struct ResetForm {
    topic: Entity<SelectState<SearchableVec<StringOption>>>,
    partition: Entity<SelectState<SearchableVec<StringOption>>>,
    reset_to: Entity<SelectState<SearchableVec<StringOption>>>,
    offset_input: Entity<InputState>,
    timestamp_input: Entity<InputState>,
}

pub struct ConsumerGroupsPage {
    cluster: Option<String>,
    group: Option<String>,
    group_state: Option<String>,
    offsets: Vec<OffsetInfo>,
    loading: bool,
    refreshing: bool,
    resetting: bool,
    error: Option<String>,
    col_widths: ColWidths,
    resizing: Option<(ResizeCol, f32, f32)>,
    reset_form: Option<ResetForm>,
    _reset_subscriptions: Vec<Subscription>,
}

impl EventEmitter<NavEvent> for ConsumerGroupsPage {}

impl ConsumerGroupsPage {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            cluster: None,
            group: None,
            group_state: None,
            offsets: Vec::new(),
            loading: false,
            refreshing: false,
            resetting: false,
            error: None,
            col_widths: ColWidths::default(),
            resizing: None,
            reset_form: None,
            _reset_subscriptions: Vec::new(),
        }
    }

    /// 外部导航：预选集群 + 消费组
    pub fn select_group(&mut self, cluster: String, group: Option<String>, cx: &mut Context<Self>) {
        self.cluster = Some(cluster);
        self.group = group;
        self.group_state = None;
        self.offsets.clear();
        if self.group.is_some() {
            self.loading = true;
            self.load(cx);
        }
        cx.notify();
    }

    fn load(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(group) = self.group.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let offsets = crate::service::call(
                &rt,
                state.clone(),
                "consumer_group.offsets",
                json!({ "cluster_id": cluster, "group_name": group }),
            )
            .await;
            let info = crate::service::call(
                &rt,
                state,
                "consumer_group.get",
                json!({ "cluster_id": cluster, "group_name": group }),
            )
            .await;

            this.update(cx, |this, cx| {
                this.loading = false;
                this.refreshing = false;
                match offsets {
                    Ok(value) => {
                        this.error = None;
                        this.offsets = value
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
                                    last_commit_time: o
                                        .get("last_commit_time")
                                        .and_then(|v| v.as_i64()),
                                })
                            })
                            .collect();
                    }
                    Err(e) => this.error = Some(e),
                }
                if let Ok(value) = info {
                    this.group_state = value
                        .get("state")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(group) = self.group.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        self.refreshing = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let _ = crate::service::call(
                &rt,
                state,
                "consumer_group.refresh",
                json!({ "cluster_id": cluster, "group_name": group }),
            )
            .await;
            this.update(cx, |this, cx| this.load(cx)).ok();
        })
        .detach();
    }

    /// 打开重置偏移对话框
    fn open_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.offsets.is_empty() {
            return;
        }

        // Topic 选项（去重）
        let mut topics: Vec<String> = self.offsets.iter().map(|o| o.topic.clone()).collect();
        topics.sort();
        topics.dedup();
        let topic_options: Vec<StringOption> = topics
            .iter()
            .map(|t| StringOption::new(t.clone(), t.clone()))
            .collect();

        let created = {
            let topic_state = cx.new(|cx| {
                SelectState::new(SearchableVec::new(topic_options), Some(IndexPath::new(0)), window, cx)
            });
            let partition_state = cx.new(|cx| {
                SelectState::new(SearchableVec::new(Vec::<StringOption>::new()), Some(IndexPath::new(0)), window, cx)
            });
            let reset_to_state = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec![
                        StringOption::new("earliest", "earliest"),
                        StringOption::new("latest", "latest"),
                        StringOption::new("offset", "offset"),
                        StringOption::new("timestamp", "timestamp"),
                    ]),
                    Some(IndexPath::new(0)),
                    window,
                    cx,
                )
            });
            let offset_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("0")
            });
            let timestamp_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("YYYY-MM-DD HH:mm:ss")
            });
            (topic_state, partition_state, reset_to_state, offset_input, timestamp_input)
        };
        let (topic_state, partition_state, reset_to_state, offset_input, timestamp_input) = created;

        // 初始化第一个 topic 的分区选项
        self.rebuild_partition_options(&partition_state, topics.first().cloned(), cx);

        // Topic 变化 → 重建分区选项
        let partition_for_sub = partition_state.clone();
        let sub = cx.subscribe(
            &topic_state,
            move |this, _, event: &SelectEvent<SearchableVec<StringOption>>, cx| {
                if let SelectEvent::Confirm(Some(topic)) = event {
                    this.rebuild_partition_options(
                        &partition_for_sub,
                        Some(topic.to_string()),
                        cx,
                    );
                }
            },
        );
        self._reset_subscriptions.push(sub);

        self.reset_form = Some(ResetForm {
            topic: topic_state.clone(),
            partition: partition_state.clone(),
            reset_to: reset_to_state.clone(),
            offset_input: offset_input.clone(),
            timestamp_input: timestamp_input.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "consumerGroups.resetOffset");
        let partition_label = t(cx, "consumerGroups.partitions");
        let reset_to_label = t(cx, "consumerGroups.resetOffset");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .confirm()
                .title(title.clone())
                    .w(px(480.0))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(field_row("Topic", Select::new(&topic_state).into_any_element()))
                            .child(field_row(&partition_label, Select::new(&partition_state).into_any_element()))
                            .child(field_row(&reset_to_label, Select::new(&reset_to_state).into_any_element()))
                            .child(field_row("Offset", Input::new(&offset_input).into_any_element()))
                            .child(field_row("Timestamp", Input::new(&timestamp_input).into_any_element())),
                    )
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |this, cx| this.submit_reset(cx));
                        true
                    })
                    .on_cancel(|_, _, _| true)
        });
    }

    fn rebuild_partition_options(
        &self,
        partition_state: &Entity<SelectState<SearchableVec<StringOption>>>,
        topic: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(topic) = topic else { return };
        let partitions: Vec<StringOption> = self
            .offsets
            .iter()
            .filter(|o| o.topic == topic)
            .map(|o| {
                let s = o.partition.to_string();
                StringOption::new(s.clone(), s)
            })
            .collect();
        let partition_state = partition_state.clone();
        cx.spawn(async move |_this, cx| {
            let _ = cx.update(|cx| {
                let w = cx.windows().first().cloned();
                if let Some(w) = w {
                    let _ = w.update(cx, |_, window, cx| {
                        partition_state.update(cx, |state, cx| {
                            *state = SelectState::new(SearchableVec::new(partitions), Some(IndexPath::new(0)), window, cx);
                        });
                    });
                }
            });
        })
        .detach();
    }

    fn submit_reset(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.reset_form.take() else { return };
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(group) = self.group.clone() else { return };
        if self.resetting {
            return;
        }

        let topic = form
            .topic
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_default();
        let partition: i32 = form
            .partition
            .read(cx)
            .selected_value()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let reset_to = form
            .reset_to
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "earliest".to_string());
        let offset: Option<i64> = form.offset_input.read(cx).value().trim().parse().ok();
        let timestamp = parse_time(&form.timestamp_input.read(cx).value());

        let mut params = json!({
            "cluster_id": cluster,
            "group_name": group,
            "topic": topic,
            "partition": partition,
            "reset_to": reset_to,
        });
        if reset_to == "offset" {
            params["offset"] = json!(offset.unwrap_or(0));
        }
        if reset_to == "timestamp" {
            if let Some(ts) = timestamp {
                params["timestamp"] = json!(ts);
            }
        }

        self.resetting = true;
        cx.notify();
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "consumer_group.reset_offset", params).await;
            cx.update(|cx| match result {
                Ok(_) => notify(
                    cx,
                    NotificationType::Success,
                    t(cx, "consumerGroups.offsetResetSuccess"),
                ),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| {
                this.resetting = false;
                this.load(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 删除消费组（确认对话框）
    fn confirm_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(group) = self.group.clone() else { return };
        let entity = cx.entity();
        let title = t(cx, "common.delete");
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let cluster = cluster.clone();
            let group = group.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(group.clone())
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |_this, cx| {
                            let rt = TokioRuntime::handle(cx);
                            let Some(state) = Backend::state(cx) else { return };
                            let cluster = cluster.clone();
                            let group = group.clone();
                            let entity2 = cx.entity();
                            cx.spawn(async move |_this2, cx| {
                                let result = crate::service::call(
                                    &rt,
                                    state,
                                    "consumer_group.delete",
                                    json!({ "cluster_id": cluster, "group": group }),
                                )
                                .await;
                                match result {
                                    Ok(_) => {
                                        cx.update(|cx| {
                                            notify(cx, NotificationType::Success, t(cx, "common.success"));
                                        })
                                        .ok();
                                        entity2
                                            .update(cx, |_this, cx| {
                                                cx.emit(NavEvent::OpenTopics {
                                                    cluster: cluster.clone(),
                                                });
                                            })
                                            .ok();
                                    }
                                    Err(e) => {
                                        cx.update(|cx| notify(cx, NotificationType::Error, e)).ok();
                                    }
                                }
                            })
                            .detach();
                        });
                        true
                    })
        });
    }

    fn start_column_resize(&mut self, col: ResizeCol, x: f32, cx: &mut Context<Self>) {
        let w = self.col_widths;
        let start_w = match col {
            ResizeCol::Topic => w.topic,
            ResizeCol::Partition => w.partition,
            ResizeCol::Start => w.start,
            ResizeCol::End => w.end,
            ResizeCol::Committed => w.committed,
            ResizeCol::Lag => w.lag,
        };
        self.resizing = Some((col, x, start_w));
        cx.notify();
    }

    fn on_resize_move(&mut self, x: f32, cx: &mut Context<Self>) {
        if let Some((col, start_x, start_w)) = self.resizing {
            let new_w = (start_w + (x - start_x)).max(40.0);
            match col {
                ResizeCol::Topic => self.col_widths.topic = new_w,
                ResizeCol::Partition => self.col_widths.partition = new_w,
                ResizeCol::Start => self.col_widths.start = new_w,
                ResizeCol::End => self.col_widths.end = new_w,
                ResizeCol::Committed => self.col_widths.committed = new_w,
                ResizeCol::Lag => self.col_widths.lag = new_w,
            }
            cx.notify();
        }
    }

    fn format_time(ts: Option<i64>) -> String {
        ts.and_then(|ts| chrono::DateTime::from_timestamp_millis(ts))
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string())
    }

    fn state_color(state: &str, cx: &App) -> Hsla {
        let theme = cx.theme();
        match state.to_lowercase().as_str() {
            "stable" | "empty" => theme.success,
            "rebalance" | "rebalancing" => theme.warning,
            _ => theme.danger,
        }
    }
}

/// 表单字段行：标签 + 控件
fn field_row(label: &str, control: AnyElement) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_sm().child(label.to_string()))
        .child(control)
}

fn parse_time(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .ok()
        .and_then(|dt| {
            dt.and_local_timezone(chrono::Local::now().timezone())
                .single()
                .map(|t| t.timestamp_millis())
        })
}

impl Render for ConsumerGroupsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let has_group = self.group.is_some();

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
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::CircleUser).text_color(theme.primary))
                            .child(
                                div()
                                    .text_xl()
                                    .font_semibold()
                                    .child(
                                        self.group
                                            .clone()
                                            .unwrap_or_else(|| t(cx, "consumerGroups.title")),
                                    ),
                            )
                            .children(self.group_state.as_ref().map(|state| {
                                let color = Self::state_color(state, cx);
                                div()
                                    .text_xs()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(color.opacity(0.15))
                                    .text_color(color)
                                    .child(state.clone())
                                    .into_any_element()
                            })),
                    )
                    .when(has_group, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!(
                                    "{}: {}",
                                    t(cx, "clusters.clusters"),
                                    self.cluster.clone().unwrap_or_default()
                                )),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("refresh")
                            .outline()
                            .label(t(cx, "common.refresh"))
                            .icon(IconName::Redo2)
                            .loading(self.refreshing)
                            .disabled(!has_group)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh(cx);
                            })),
                    )
                    .child(
                        Button::new("reset")
                            .primary()
                            .label(t(cx, "consumerGroups.resetOffset"))
                            .disabled(!has_group || self.offsets.is_empty())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_reset(window, cx);
                            })),
                    )
                    .child(
                        Button::new("delete")
                            .danger()
                            .label(t(cx, "common.delete"))
                            .disabled(!has_group)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.confirm_delete(window, cx);
                            })),
                    ),
            );

        let content: AnyElement = if !has_group {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(theme.muted_foreground)
                .child(t(cx, "common.noData"))
                .child(div().text_sm().child(t(cx, "topicConsumerGroups.selectFromNav")))
                .into_any_element()
        } else if self.loading {
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
        } else if self.offsets.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "consumerGroups.noData"))
                .into_any_element()
        } else {
            // 表头（列宽可拖拽）
            let w = self.col_widths;
            let resizer = |col: ResizeCol| {
                div()
                    .id(SharedString::from(format!("cg-resize-{:?}", col)))
                    .w_1()
                    .h_full()
                    .cursor_col_resize()
                    .hover(|el| el.bg(theme.primary))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            this.start_column_resize(col, event.position.x.as_f32(), cx);
                        }),
                    )
            };
            let header_row = h_flex()
                .px_2()
                .py_1()
                .bg(theme.table_head)
                .text_xs()
                .font_semibold()
                .child(div().w(px(w.topic)).child("Topic"))
                .child(resizer(ResizeCol::Topic))
                .child(div().w(px(w.partition)).child(t(cx, "consumerGroups.partitions")))
                .child(resizer(ResizeCol::Partition))
                .child(div().w(px(w.start)).text_right().child(t(cx, "consumerGroups.start")))
                .child(resizer(ResizeCol::Start))
                .child(div().w(px(w.end)).text_right().child(t(cx, "consumerGroups.end")))
                .child(resizer(ResizeCol::End))
                .child(div().w(px(w.committed)).text_right().child(t(cx, "consumerGroups.offset")))
                .child(resizer(ResizeCol::Committed))
                .child(div().w(px(w.lag)).text_right().child(t(cx, "consumerGroups.lag")))
                .child(resizer(ResizeCol::Lag))
                .child(div().flex_1().text_right().child(t(cx, "consumerGroups.lastCommit")));

            let rows: Vec<AnyElement> = self
                .offsets
                .iter()
                .map(|o| {
                    let lag_color = if o.lag == 0 {
                        theme.success
                    } else if o.lag < 1000 {
                        theme.warning
                    } else {
                        theme.danger
                    };
                    let w = self.col_widths;
                    h_flex()
                        .px_2()
                        .py_1p5()
                        .border_b_1()
                        .border_color(theme.border)
                        .text_xs()
                        .child(
                            div()
                                .w(px(w.topic))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(o.topic.clone()),
                        )
                        .child(
                            div().w(px(w.partition)).child(
                                div()
                                    .px_1()
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .child(o.partition.to_string()),
                            ),
                        )
                        .child(div().w(px(w.start)).text_right().child(o.start_offset.to_string()))
                        .child(div().w(px(w.end)).text_right().child(o.end_offset.to_string()))
                        .child(div().w(px(w.committed)).text_right().child(o.committed_offset.to_string()))
                        .child(
                            div()
                                .w(px(w.lag))
                                .text_right()
                                .text_color(lag_color)
                                .child(o.lag.to_string()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_right()
                                .text_color(theme.muted_foreground)
                                .child(Self::format_time(o.last_commit_time)),
                        )
                        .into_any_element()
                })
                .collect();

            v_flex()
                .size_full()
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .px_2()
                        .py_1()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(div().text_xs().font_semibold().child("Offsets"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!("{} partitions", self.offsets.len())),
                        ),
                )
                .child(header_row)
                .child(
                    div()
                        .id("offsets-scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .child(v_flex().children(rows)),
                )
                .into_any_element()
        };

        let resize_overlay: AnyElement = if self.resizing.is_some() {
            div()
                .absolute()
                .inset_0()
                .id("cg-resize-overlay")
                .cursor_col_resize()
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                    this.on_resize_move(event.position.x.as_f32(), cx);
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.resizing = None;
                        cx.notify();
                    }),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .relative()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .p_4()
                    .child(header)
                    .child(div().flex_1().overflow_hidden().child(content)),
            )
            .child(resize_overlay)
    }
}
