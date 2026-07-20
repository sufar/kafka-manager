//! Topic 维度的消费组视图：显示消费某 Topic 的所有消费组偏移量
//! 列：Group Name | Partition | Start | End | Committed | Lag | Last Commit

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::notification::NotificationType;
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::navigator::NavEvent;
use crate::i18n::t;
use crate::components::notify;
use crate::state::{Backend, TokioRuntime};

#[derive(Clone, Debug)]
struct OffsetRow {
    group: String,
    partition: i32,
    start_offset: i64,
    end_offset: i64,
    committed_offset: i64,
    lag: i64,
    last_commit_time: Option<i64>,
}

impl EventEmitter<NavEvent> for TopicConsumerGroupsPage {}

pub struct TopicConsumerGroupsPage {
    cluster: Option<String>,
    topic: Option<String>,
    rows: Vec<OffsetRow>,
    loading: bool,
    refreshing: bool,
    error: Option<String>,
}

impl TopicConsumerGroupsPage {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            cluster: None,
            topic: None,
            rows: Vec::new(),
            loading: false,
            refreshing: false,
            error: None,
        }
    }

    /// 外部导航：预选集群 + Topic
    pub fn select_cluster_topic(&mut self, cluster: String, topic: String, cx: &mut Context<Self>) {
        self.cluster = Some(cluster);
        self.topic = Some(topic);
        self.rows.clear();
        self.loading = true;
        cx.notify();
        self.load(cx);
    }

    pub fn current_topic(&self) -> Option<(String, String)> {
        Some((self.cluster.clone()?, self.topic.clone()?))
    }

    fn load(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(topic) = self.topic.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "consumer_group.list_by_topic",
                json!({ "cluster_id": cluster, "topic": topic }),
            )
            .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                this.refreshing = false;
                match result {
                    Ok(value) => {
                        this.error = None;
                        this.rows = value
                            .get("offsets")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|o| {
                                Some(OffsetRow {
                                    group: o.get("group")?.as_str()?.to_string(),
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
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(topic) = self.topic.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        self.refreshing = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "consumer_group.refresh",
                json!({ "cluster_id": cluster, "topic_name": topic }),
            )
            .await;
            if let Err(e) = result {
                cx.update(|cx| notify(cx, NotificationType::Error, e)).ok();
            }
            this.update(cx, |this, cx| this.load(cx)).ok();
        })
        .detach();
    }

    fn format_time(ts: Option<i64>) -> String {
        ts.and_then(|ts| chrono::DateTime::from_timestamp_millis(ts))
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string())
    }

    fn lag_color(lag: i64, cx: &App) -> Hsla {
        let theme = cx.theme();
        if lag == 0 {
            theme.success
        } else if lag < 100 {
            theme.warning
        } else {
            theme.danger
        }
    }
}

impl Render for TopicConsumerGroupsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let has_topic = self.cluster.is_some() && self.topic.is_some();

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
                            .child(
                                Button::new("back")
                                    .ghost()
                                    .icon(IconName::ArrowLeft)
                                    .disabled(!has_topic)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some((cluster, topic)) = this.current_topic() {
                                            cx.emit(NavEvent::OpenMessages { cluster, topic });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_semibold()
                                    .child(t(cx, "topicConsumerGroups.title")),
                            ),
                    )
                    .when(has_topic, |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!(
                                    "{}: {}  ·  {}",
                                    t(cx, "clusters.clusters"),
                                    self.cluster.clone().unwrap_or_default(),
                                    self.topic.clone().unwrap_or_default()
                                )),
                        )
                    }),
            )
            .child(
                Button::new("refresh")
                    .outline()
                    .label(t(cx, "common.refresh"))
                    .icon(IconName::Redo2)
                    .loading(self.refreshing)
                    .disabled(!has_topic)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.refresh(cx);
                    })),
            );

        let content: AnyElement = if !has_topic {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "messages.noTopicSelected"))
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
        } else if self.rows.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "topicConsumerGroups.noData"))
                .into_any_element()
        } else {
            let header_row = h_flex()
                .px_2()
                .py_1()
                .bg(theme.table_head)
                .text_xs()
                .font_semibold()
                .child(div().w(px(220.0)).child(t(cx, "consumerGroups.groupName")))
                .child(div().w_20().child(t(cx, "consumerGroups.partitions")))
                .child(div().w_24().text_right().child(t(cx, "consumerGroups.start")))
                .child(div().w_24().text_right().child(t(cx, "consumerGroups.end")))
                .child(div().w_24().text_right().child(t(cx, "consumerGroups.offset")))
                .child(div().w_16().text_right().child(t(cx, "consumerGroups.lag")))
                .child(div().flex_1().text_right().child(t(cx, "consumerGroups.lastCommit")));

            let rows: Vec<AnyElement> = self
                .rows
                .iter()
                .map(|r| {
                    let lag_color = Self::lag_color(r.lag, cx);
                    h_flex()
                        .px_2()
                        .py_1p5()
                        .border_b_1()
                        .border_color(theme.border)
                        .text_xs()
                        .child(
                            h_flex()
                                .w(px(220.0))
                                .gap_1()
                                .items_center()
                                .child(
                                    Icon::new(IconName::CircleUser)
                                        .size_3()
                                        .text_color(theme.secondary),
                                )
                                .child(
                                    div()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(r.group.clone()),
                                ),
                        )
                        .child(div().w_20().child(r.partition.to_string()))
                        .child(div().w_24().text_right().child(r.start_offset.to_string()))
                        .child(div().w_24().text_right().child(r.end_offset.to_string()))
                        .child(div().w_24().text_right().child(r.committed_offset.to_string()))
                        .child(
                            div()
                                .w_16()
                                .text_right()
                                .text_color(lag_color)
                                .child(r.lag.to_string()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_right()
                                .text_color(theme.muted_foreground)
                                .child(Self::format_time(r.last_commit_time)),
                        )
                        .into_any_element()
                })
                .collect();

            v_flex()
                .size_full()
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .child(header_row)
                .child(
                    div()
                        .id("tcg-scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .child(v_flex().children(rows)),
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
