//! 消费者组详情页 + Topic 维度消费者组页
//! （对齐 ConsumerGroupsView.vue / TopicConsumerGroupsView.vue）

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
use crate::widgets::text_input::{TextInput, TextInputEvent};

// ==================== 数据模型 ====================

#[derive(Clone, Debug)]
pub struct OffsetItem {
    topic: String,
    partition: i32,
    start_offset: i64,
    end_offset: i64,
    committed_offset: i64,
    lag: i64,
    last_commit_time: Option<i64>,
}

impl OffsetItem {
    fn from_json(v: &serde_json::Value) -> Option<Self> {
        Some(Self {
            topic: v.get("topic")?.as_str()?.to_string(),
            partition: v.get("partition")?.as_i64()? as i32,
            start_offset: v.get("start_offset")?.as_i64()?,
            end_offset: v.get("end_offset")?.as_i64()?,
            committed_offset: v.get("committed_offset")?.as_i64()?,
            lag: v.get("lag")?.as_i64()?,
            last_commit_time: v.get("last_commit_time").and_then(|x| x.as_i64()),
        })
    }
}

fn lag_color(lag: i64, threshold: i64) -> gpui::Hsla {
    if lag == 0 {
        theme::success()
    } else if lag < threshold {
        theme::warning()
    } else {
        theme::error()
    }
}

fn state_badge_kind(state: &str) -> BadgeKind {
    match state.to_lowercase().as_str() {
        "stable" | "empty" => BadgeKind::Success,
        "preparing_rebalance" | "completing_rebalance" => BadgeKind::Warning,
        "dead" | "unknown" => BadgeKind::Error,
        _ => BadgeKind::Ghost,
    }
}

const COL_W: [f32; 6] = [60., 80., 80., 90., 70., 150.];

// ==================== 详情页 ====================

pub struct ConsumerGroupsView {
    cluster: Option<String>,
    group: Option<String>,
    group_state: String,
    offsets: Vec<OffsetItem>,
    loading: bool,
    error: Option<String>,
    refreshing: bool,
    col_widths: [f32; 6],
    #[allow(dead_code)]
    resizing_col: Option<usize>,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
}

impl ConsumerGroupsView {
    pub fn new(route: Route, cx: &mut Context<Self>) -> Self {
        let this = Self {
            cluster: route.get("cluster").map(|s| s.to_string()),
            group: route.get("group").map(|s| s.to_string()),
            group_state: "Unknown".into(),
            offsets: vec![],
            loading: false,
            error: None,
            refreshing: false,
            col_widths: COL_W,
            resizing_col: None,
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        };
        this.load_detail(cx);
        this
    }

    fn is_detail(&self) -> bool {
        self.cluster.is_some() && self.group.is_some()
    }

    fn load_detail(&self, cx: &mut Context<Self>) {
        if !self.is_detail() {
            return;
        }
        let (cluster, group) = (self.cluster.clone().unwrap(), self.group.clone().unwrap());
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.loading = true;
                this.error = None;
                cx.notify();
            })
            .ok();
            let info = b
                .dispatch(
                    "consumer_group.get",
                    serde_json::json!({"cluster_id": cluster, "group_name": group}),
                )
                .await;
            let offsets = b
                .dispatch(
                    "consumer_group.offsets",
                    serde_json::json!({"cluster_id": cluster, "group_name": group}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                this.refreshing = false;
                match (&info, &offsets) {
                    (Ok(i), Ok(o)) => {
                        this.group_state = i
                            .get("state")
                            .and_then(|x| x.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        this.offsets = o
                            .get("offsets")
                            .and_then(|x| x.as_array())
                            .map(|arr| arr.iter().filter_map(OffsetItem::from_json).collect())
                            .unwrap_or_default();
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        this.error = Some(e.clone());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn refresh_offsets(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        cx.notify();
        let b = backend(cx);
        let (cluster, group) = (self.cluster.clone().unwrap(), self.group.clone().unwrap());
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "consumer_group.offsets",
                    serde_json::json!({"cluster_id": cluster, "group_name": group}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.refreshing = false;
                match result {
                    Ok(o) => {
                        this.offsets = o
                            .get("offsets")
                            .and_then(|x| x.as_array())
                            .map(|arr| arr.iter().filter_map(OffsetItem::from_json).collect())
                            .unwrap_or_default();
                        overlay::toast_success(cx, t("consumerGroups.offsetsRefreshed"));
                    }
                    Err(e) => overlay::toast_error(cx, format!("Refresh failed: {}", e)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_actions_menu(&self, position: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        let (cluster, group) = (
            self.cluster.clone().unwrap_or_default(),
            self.group.clone().unwrap_or_default(),
        );
        let entity = cx.entity();
        overlay::open_context_menu(cx, overlay::ContextMenuState {
            position,
            title: None,
            separators: vec![],
            items: vec![
                overlay::ContextItem {
                    label: t("consumerGroups.resetOffset"),
                    icon: Some("refresh"),
                    danger: false,
                    action: Box::new(move |cx| {
                        entity.update(cx, |view, cx| view.open_reset_dialog(cx));
                    }),
                },
                overlay::ContextItem {
                    label: t("consumerGroups.deleteGroup"),
                    icon: Some("trash"),
                    danger: true,
                    action: Box::new(move |cx| {
                        let title = t("common.confirm");
                        let message = format!(
                            "Are you sure you want to delete consumer group \"{}\"?",
                            group
                        );
                        overlay::confirm(cx, title, message, true, move |cx| {
                            let b = backend(cx);
                            let (cluster, group) = (cluster.clone(), group.clone());
                            cx.spawn(async move |cx| {
                                let result = b
                                    .dispatch(
                                        "consumer_group.delete",
                                        serde_json::json!({"cluster_id": cluster, "group": group}),
                                    )
                                    .await;
                                cx.update(|cx| {
                                    match result {
                                        Ok(_) => {
                                            overlay::toast_success(
                                                cx,
                                                t("consumerGroups.deleted"),
                                            );
                                            root(cx).update(cx, |app, cx| {
                                                let route = Route::new(Page::Topics)
                                                    .with("cluster", &cluster);
                                                app.navigate(route, true, cx);
                                            });
                                        }
                                        Err(e) => overlay::toast_error(
                                            cx,
                                            format!("Delete failed: {}", e),
                                        ),
                                    }
})
                            })
                            .detach();
                        });
                    }),
                },
            ],
        });
    }

    fn open_reset_dialog(&self, cx: &mut Context<Self>) {
        let (cluster, group) = (
            self.cluster.clone().unwrap_or_default(),
            self.group.clone().unwrap_or_default(),
        );
        let offsets = self.offsets.clone();
        let view = cx.new(|cx| ResetOffsetDialog::new(cluster, group, offsets, cx));
        overlay::open_modal(cx, view.into());
    }

    fn render_offsets_table(&self, _cx: &mut Context<Self>, lag_threshold: i64) -> impl IntoElement {
        let headers = [
            t("consumerGroups.partition"),
            t("consumerGroups.startOffset"),
            t("consumerGroups.endOffset"),
            t("consumerGroups.committedOffset"),
            t("consumerGroups.lag"),
            t("consumerGroups.lastCommitTime"),
        ];
        let col_widths = self.col_widths;

        card()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            // 卡片头
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .h(px(40.))
                    .flex_none()
                    .border_b_1()
                    .border_color(theme::border_base_200())
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("consumerGroups.offsets")),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::text_secondary())
                            .child(format!("{} {}", self.offsets.len(), t("consumerGroups.partitions"))),
                    ),
            )
            // 表头
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(30.))
                    .flex_none()
                    .text_size(px(11.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme::text_secondary())
                    .border_b_1()
                    .border_color(theme::border_base_200())
                    .child(div().flex_1().px(px(8.)).child(t("consumerGroups.topic")))
                    .children(headers.iter().enumerate().map(|(ix, h)| {
                        div()
                            .w(px(col_widths[ix]))
                            .flex_none()
                            .px(px(4.))
                            .flex()
                            .justify_end()
                            .child(h.clone())
                    })),
            )
            // 表体
            .child(
                div().id("views_consumer_groups_rs_2")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(self.offsets.iter().enumerate().map(|(ix, o)| {
                        div()
                            .id(("offset-row", ix))
                            .flex()
                            .items_center()
                            .h(px(40.))
                            .text_size(px(11.))
                            .hover(|s| s.bg(theme::table_row_hover()))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(8.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme::text_primary())
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(o.topic.clone()),
                            )
                            .child(
                                div()
                                    .w(px(col_widths[0]))
                                    .flex_none()
                                    .flex()
                                    .justify_end()
                                    .px(px(4.))
                                    .child(badge(o.partition.to_string(), BadgeKind::Ghost)),
                            )
                            .child(offset_cell(o.start_offset, col_widths[1], theme::text_primary()))
                            .child(offset_cell(o.end_offset, col_widths[2], theme::text_primary()))
                            .child(offset_cell(o.committed_offset, col_widths[3], theme::text_primary()))
                            .child(offset_cell(o.lag, col_widths[4], lag_color(o.lag, lag_threshold)))
                            .child(
                                div()
                                    .w(px(col_widths[5]))
                                    .flex_none()
                                    .px(px(4.))
                                    .flex()
                                    .justify_end()
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child(
                                        o.last_commit_time
                                            .map(|ts| {
                                                crate::views::navigator::format_datetime(ts, false)
                                            })
                                            .unwrap_or_else(|| "-".into()),
                                    ),
                            )
                    })),
            )
    }
}

fn offset_cell(value: i64, width: f32, color: gpui::Hsla) -> gpui::Div {
    div()
        .w(px(width))
        .flex_none()
        .px(px(4.))
        .flex()
        .justify_end()
        .font_family("monospace")
        .text_size(px(11.))
        .text_color(color)
        .child(value.to_string())
}

impl Focusable for ConsumerGroupsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ConsumerGroupsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);

        if !self.is_detail() {
            return div()
                .size_full()
                .p(px(12.))
                .child(empty_block(
                    "users",
                    t("common.noData"),
                    t("consumerGroups.selectFromNav"),
                ));
        }

        let cluster = self.cluster.clone().unwrap();
        let group = self.group.clone().unwrap();
        let loading = self.loading;
        let error = self.error.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(12.))
            .gap(px(12.))
            .overflow_hidden()
            // 头部卡片
            .child(
                card()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .p(px(14.))
                    .child(
                        icon_btn("cg-back", "arrow-left", BtnSize::Sm)
                            .when(!app.read(cx).can_go_back(), |b| b.opacity(0.5))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                root(cx).update(cx, |app, cx| app.go_back(cx));
                            }),
                    )
                    .child(
                        icon("users")
                            .size(px(20.))
                            .text_color(theme::badge_primary_text()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme::text_primary())
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(format!("{}: {}", t("consumerGroups.title"), group)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme::text_secondary())
                                            .child(format!("{}: {}", t("topics.cluster"), cluster)),
                                    )
                                    .child(badge(
                                        self.group_state.clone(),
                                        state_badge_kind(&self.group_state),
                                    )),
                            ),
                    )
                    .child(
                        btn("cg-refresh", BtnKind::Outline, BtnSize::Sm)
                            .when(self.refreshing, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.refresh_offsets(cx)),
                            )
                            .child(if self.refreshing {
                                spinner(13.)
                            } else {
                                icon("refresh").size(px(13.)).into_any_element()
                            })
                            .child(t("common.refresh")),
                    )
                    .child(
                        btn("cg-actions", BtnKind::Primary, BtnSize::Sm)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                                    let pos = event.position;
                                    this.open_actions_menu(pos, cx);
                                }),
                            )
                            .child(t("common.actions"))
                            .child(icon("chevron-down").size(px(12.)).into_any_element()),
                    ),
            )
            // 内容
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .when(loading, |d| {
                        d.child(loading_block(t("common.loading")))
                    })
                    .when(!loading && error.is_some(), |d| {
                        let err_text = error.clone().unwrap_or_default();
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_full()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .px(px(14.))
                                        .py(px(10.))
                                        .rounded(px(8.))
                                        .bg(theme::error())
                                        .child(
                                            icon("warn").size(px(16.)).text_color(gpui::white()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(gpui::white())
                                                .child(err_text),
                                        ),
                                ),
                        )
                    })
                    .when(!loading && error.is_none() && self.offsets.is_empty(), |d| {
                        d.child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap(px(8.))
                                .child(
                                    icon("info")
                                        .size(px(40.))
                                        .text_color(theme::base_content_alpha(0.25)),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme::text_secondary())
                                        .child(t("common.noData")),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme::base_content_alpha(0.45))
                                        .child(t("consumerGroups.noOffsets")),
                                )
                                .child(
                                    btn("cg-reload", BtnKind::Outline, BtnSize::Sm)
                                        .child(t("common.refresh"))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.refresh_offsets(cx)
                                            }),
                                        ),
                                ),
                        )
                    })
                    .when(!loading && error.is_none() && !self.offsets.is_empty(), |d| {
                        d.child(self.render_offsets_table(cx, 1000))
                    }),
            )
    }
}

// ==================== 重置 Offset 对话框 ====================

pub struct ResetOffsetDialog {
    cluster: String,
    group: String,
    offsets: Vec<OffsetItem>,
    topic_select: Entity<Select>,
    partition_select: Entity<Select>,
    reset_to_select: Entity<Select>,
    offset_value: Entity<TextInput>,
    timestamp_value: Entity<TextInput>,
    resetting: bool,
    focus_handle: FocusHandle,
}

impl ResetOffsetDialog {
    pub fn new(
        cluster: String,
        group: String,
        offsets: Vec<OffsetItem>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut topics: Vec<String> = offsets.iter().map(|o| o.topic.clone()).collect();
        topics.sort();
        topics.dedup();
        let first_topic = topics.first().cloned().unwrap_or_default();

        let topic_select = cx.new(|cx| {
            Select::new(
                topics
                    .iter()
                    .map(|t2| SelectOption {
                        value: t2.clone().into(),
                        label: t2.clone().into(),
                    })
                    .collect(),
                first_topic.clone(),
                cx,
            )
        });

        let partitions: Vec<i32> = {
            let mut ps: Vec<i32> = offsets
                .iter()
                .filter(|o| o.topic == first_topic)
                .map(|o| o.partition)
                .collect();
            ps.sort();
            ps.dedup();
            ps
        };
        let partition_select = cx.new(|cx| {
            Select::new(
                partitions
                    .iter()
                    .map(|p| SelectOption {
                        value: p.to_string().into(),
                        label: format!("Partition {}", p).into(),
                    })
                    .collect(),
                partitions.first().map(|p| p.to_string()).unwrap_or_default(),
                cx,
            )
        });

        let reset_to_select = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption {
                        value: "earliest".into(),
                        label: "最早 (earliest)".into(),
                    },
                    SelectOption {
                        value: "latest".into(),
                        label: "最新 (latest)".into(),
                    },
                    SelectOption {
                        value: "offset".into(),
                        label: "指定偏移 (offset)".into(),
                    },
                    SelectOption {
                        value: "timestamp".into(),
                        label: "时间戳".into(),
                    },
                ],
                "earliest",
                cx,
            )
        });

        let offset_value = cx.new(TextInput::new);
        offset_value.update(cx, |i, cx| i.set_text("0", cx));
        let timestamp_value = cx.new(TextInput::new);
        timestamp_value.update(cx, |i, _| i.set_placeholder("YYYY-MM-DD HH:mm:ss"));

        for input in [&offset_value, &timestamp_value] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }

        let this = Self {
            cluster,
            group,
            offsets,
            topic_select: topic_select.clone(),
            partition_select,
            reset_to_select,
            offset_value,
            timestamp_value,
            resetting: false,
            focus_handle: cx.focus_handle(),
        };

        // topic 变化 → 重建 partition 选项
        cx.subscribe(&topic_select, |this, offsets_entity, event: &SelectEvent, cx| {
            let _ = offsets_entity;
            let topic = event.value.to_string();
            let mut ps: Vec<i32> = this
                .offsets
                .iter()
                .filter(|o| o.topic == topic)
                .map(|o| o.partition)
                .collect();
            ps.sort();
            ps.dedup();
            this.partition_select.update(cx, |s, cx| {
                s.set_options(
                    ps.iter()
                        .map(|p| SelectOption {
                            value: p.to_string().into(),
                            label: format!("Partition {}", p).into(),
                        })
                        .collect(),
                    cx,
                );
                if let Some(first) = ps.first() {
                    s.set_value(first.to_string(), cx);
                }
            });
        })
        .detach();

        this
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.resetting {
            return;
        }
        let topic = self.topic_select.read(cx).value.to_string();
        let partition: i32 = self
            .partition_select
            .read(cx)
            .value
            .parse()
            .unwrap_or(-1);
        let reset_to = self.reset_to_select.read(cx).value.to_string();
        if partition < 0 {
            overlay::toast_error(cx, "Please select a partition");
            return;
        }
        let mut params = serde_json::json!({
            "cluster_id": self.cluster,
            "group_name": self.group,
            "topic": topic,
            "partition": partition,
            "reset_to": reset_to,
        });
        if reset_to == "offset" {
            let value: i64 = self
                .offset_value
                .read(cx)
                .text()
                .trim()
                .parse()
                .unwrap_or(-1);
            if value < 0 {
                overlay::toast_error(cx, "Invalid offset value");
                return;
            }
            params["offset"] = serde_json::json!(value);
        }
        if reset_to == "timestamp" {
            let ts_text = self.timestamp_value.read(cx).text().trim().to_string();
            let Some(ts) = crate::views::navigator::parse_iso8601(&ts_text) else {
                overlay::toast_error(cx, "Please select a timestamp");
                return;
            };
            params["timestamp"] = serde_json::json!(ts);
        }

        self.resetting = true;
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b.dispatch("consumer_group.reset_offset", params).await;
            this.update(cx, |this, cx| {
                this.resetting = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, t("consumerGroups.offsetResetSuccess"));
                        overlay::close_modal(cx);
                        // 刷新详情页 offsets
                        if let Some(v) = root(cx).read(cx).page_consumer_groups() {
                            v.update(cx, |view, cx| view.refresh_offsets(cx));
                        }
                    }
                    Err(e) => {
                        if e.contains("UnknownMemberId") || e.contains("Unknown member") {
                            overlay::toast_error(
                                cx,
                                "重置失败：当前消费者组没有活跃成员。请确保有消费者连接到该组后再尝试重置偏移量。",
                            );
                        } else if e.contains("group_name") {
                            overlay::toast_error(cx, "重置失败：消费者组名称无效或不存在。");
                        } else {
                            overlay::toast_error(cx, format!("Reset failed: {}", e));
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Focusable for ResetOffsetDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ResetOffsetDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let reset_to = self.reset_to_select.read(cx).value.to_string();

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
                            .child(t("consumerGroups.resetOffset")),
                    )
                    .child(
                        icon_btn("close-reset-offset", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(field(
                t("consumerGroups.selectTopic"),
                self.topic_select.clone().into_any_element(),
            ))
            .child(field(
                t("consumerGroups.partition"),
                self.partition_select.clone().into_any_element(),
            ))
            .child(field(
                t("consumerGroups.resetTo"),
                self.reset_to_select.clone().into_any_element(),
            ))
            .when(reset_to == "offset", |d| {
                d.child(field(
                    t("consumerGroups.offsetValue"),
                    input_frame(self.offset_value.clone()).into_any_element(),
                ))
            })
            .when(reset_to == "timestamp", |d| {
                d.child(field(
                    t("consumerGroups.timestampValue"),
                    input_frame(self.timestamp_value.clone()).into_any_element(),
                ))
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .pt(px(4.))
                    .child(
                        btn("reset-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("reset-submit", BtnKind::Primary, BtnSize::Sm)
                            .when(self.resetting, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(cx)),
                            )
                            .child(if self.resetting {
                                spinner(13.)
                            } else {
                                icon("check").size(px(13.)).into_any_element()
                            })
                            .child(t("common.confirm")),
                    ),
            )
    }
}


// ==================== Topic 维度页 ====================

pub struct TopicConsumerGroupsView {
    cluster: Option<String>,
    topic: Option<String>,
    offsets: Vec<(String, OffsetItem)>, // (group, offset)
    loading: bool,
    error: Option<String>,
    refreshing: bool,
    col_widths: [f32; 6],
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
}

impl TopicConsumerGroupsView {
    pub fn new(route: Route, cx: &mut Context<Self>) -> Self {
        let this = Self {
            cluster: route.get("cluster").map(|s| s.to_string()),
            topic: route.get("topic").map(|s| s.to_string()),
            offsets: vec![],
            loading: false,
            error: None,
            refreshing: false,
            col_widths: COL_W,
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        };
        this.load(cx);
        this
    }

    pub fn load(&self, cx: &mut Context<Self>) {
        let (Some(cluster), Some(topic)) = (self.cluster.clone(), self.topic.clone()) else {
            return;
        };
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.loading = true;
                this.error = None;
                cx.notify();
            })
            .ok();
            let result = b
                .dispatch(
                    "consumer_group.list_by_topic",
                    serde_json::json!({"cluster_id": cluster, "topic": topic}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                this.refreshing = false;
                match result {
                    Ok(v) => {
                        this.offsets = v
                            .get("offsets")
                            .and_then(|x| x.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|item| {
                                        let group = item.get("group")?.as_str()?.to_string();
                                        let offset = OffsetItem::from_json(item)?;
                                        Some((group, offset))
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

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        cx.notify();
        self.load(cx);
        // load 完成后 toast（简单起见立即提示）
        overlay::toast_success(cx, t("topicConsumerGroups.refreshed"));
    }
}

impl Focusable for TopicConsumerGroupsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TopicConsumerGroupsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);
        let cluster = self.cluster.clone().unwrap_or_default();
        let topic = self.topic.clone().unwrap_or_default();
        let loading = self.loading;
        let error = self.error.clone();
        let col_widths = self.col_widths;

        let headers = [
            t("consumerGroups.partition"),
            t("consumerGroups.startOffset"),
            t("consumerGroups.endOffset"),
            t("consumerGroups.committedOffset"),
            t("consumerGroups.lag"),
            t("consumerGroups.lastCommitTime"),
        ];

        div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(12.))
            .gap(px(12.))
            .overflow_hidden()
            // 头部
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .flex_none()
                    .child(
                        icon_btn("tcg-back", "arrow-left", BtnSize::Xs)
                            .when(!app.read(cx).can_go_back(), |b| b.opacity(0.5))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                root(cx).update(cx, |app, cx| app.go_back(cx));
                            }),
                    )
                    .child(
                        icon("users")
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
                                    .child(t("topicConsumerGroups.title")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child(format!(
                                        "{}: {} • Topic: {}",
                                        t("topics.cluster"),
                                        cluster,
                                        topic
                                    ))
                                    .child(
                                        icon("info")
                                            .size(px(12.))
                                            .text_color(theme::text_secondary()),
                                    ),
                            ),
                    )
                    .child(
                        btn("tcg-refresh", BtnKind::Outline, BtnSize::Xs)
                            .when(self.refreshing, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.refresh(cx)),
                            )
                            .child(if self.refreshing {
                                spinner(12.)
                            } else {
                                icon("refresh").size(px(12.)).into_any_element()
                            })
                            .child(t("common.refresh")),
                    ),
            )
            // 内容
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .when(loading, |d| d.child(loading_block(t("common.loading"))))
                    .when(!loading && error.is_some(), |d| {
                        let err_text = error.clone().unwrap_or_default();
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_full()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .px(px(14.))
                                        .py(px(10.))
                                        .rounded(px(8.))
                                        .bg(theme::error())
                                        .child(
                                            icon("warn").size(px(16.)).text_color(gpui::white()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(gpui::white())
                                                .child(err_text),
                                        ),
                                ),
                        )
                    })
                    .when(!loading && error.is_none() && self.offsets.is_empty(), |d| {
                        d.child(empty_block(
                            "users",
                            t("common.noData"),
                            t("topicConsumerGroups.noData"),
                        ))
                    })
                    .when(!loading && error.is_none() && !self.offsets.is_empty(), |d| {
                        d.child(
                            card()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .overflow_hidden()
                                // 表头
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .h(px(30.))
                                        .flex_none()
                                        .text_size(px(11.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme::text_secondary())
                                        .border_b_1()
                                        .border_color(theme::border_base_200())
                                        .child(
                                            div()
                                                .flex_1()
                                                .px(px(8.))
                                                .child("Consumer Group"),
                                        )
                                        .children(headers.iter().enumerate().map(|(ix, h)| {
                                            div()
                                                .w(px(col_widths[ix]))
                                                .flex_none()
                                                .px(px(4.))
                                                .flex()
                                                .justify_end()
                                                .child(h.clone())
                                        })),
                                )
                                .child(
                                    div().id("views_consumer_groups_rs_4")
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .track_scroll(&self.scroll)
                                        .children(self.offsets.iter().enumerate().map(
                                            |(ix, (group, o))| {
                                                div()
                                                    .id(("tcg-row", ix))
                                                    .flex()
                                                    .items_center()
                                                    .h(px(36.))
                                                    .text_size(px(11.))
                                                    .hover(|s| s.bg(theme::table_row_hover()))
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.))
                                                            .px(px(8.))
                                                            .overflow_hidden()
                                                            .child(
                                                                icon("users")
                                                                    .size(px(13.))
                                                                    .text_color(
                                                                        theme::badge_primary_text(
                                                                        ),
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .overflow_hidden()
                                                                    .whitespace_nowrap()
                                                                    .text_color(
                                                                        theme::text_primary(),
                                                                    )
                                                                    .child(group.clone()),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(col_widths[0]))
                                                            .flex_none()
                                                            .flex()
                                                            .justify_end()
                                                            .px(px(4.))
                                                            .child(badge(
                                                                o.partition.to_string(),
                                                                BadgeKind::Ghost,
                                                            )),
                                                    )
                                                    .child(offset_cell(
                                                        o.start_offset,
                                                        col_widths[1],
                                                        theme::text_primary(),
                                                    ))
                                                    .child(offset_cell(
                                                        o.end_offset,
                                                        col_widths[2],
                                                        theme::text_primary(),
                                                    ))
                                                    .child(offset_cell(
                                                        o.committed_offset,
                                                        col_widths[3],
                                                        theme::text_primary(),
                                                    ))
                                                    .child(offset_cell(
                                                        o.lag,
                                                        col_widths[4],
                                                        lag_color(o.lag, 100),
                                                    ))
                                                    .child(
                                                        div()
                                                            .w(px(col_widths[5]))
                                                            .flex_none()
                                                            .px(px(4.))
                                                            .flex()
                                                            .justify_end()
                                                            .text_size(px(11.))
                                                            .text_color(theme::text_secondary())
                                                            .child(
                                                                o.last_commit_time
                                                                    .map(|ts| {
                                                                        crate::views::navigator::format_datetime(ts, false)
                                                                    })
                                                                    .unwrap_or_else(|| "-".into()),
                                                            ),
                                                    )
                                            },
                                        )),
                                ),
                        )
                    }),
            )
    }
}
