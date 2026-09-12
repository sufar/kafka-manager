//! 消息查询页（对齐 MessageQueryTool.vue：流式查询、虚拟列表、排序、详情面板、发送历史）
//! 这是全应用最复杂的视图。

use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    actions, div, px, App, Context, Entity, FocusHandle, Focusable, HighlightStyle, IntoElement,
    KeyBinding, MouseButton, Render, ScrollHandle, ScrollStrategy, StyledText,
    UniformListScrollHandle, Window,
};

actions!(km_messages, [MsgSelectUp, MsgSelectDown]);

pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", MsgSelectUp, Some("KmMessages")),
        KeyBinding::new("down", MsgSelectDown, Some("KmMessages")),
    ]);
}

use crate::app::{backend, root, Page, PageView, Route};
use crate::i18n::t;
use crate::icons::icon;
use crate::json::{format_json, styles_for_current_theme, to_hex_dump, token_color, tokenize, TokenKind};
use crate::models::*;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::favorite_button::FavoriteButton;
use crate::widgets::text_input::{TextInput, TextInputEvent};

// ==================== 常量 ====================

const COL_PARTITION: usize = 0;
const COL_OFFSET: usize = 1;
const COL_TIMESTAMP: usize = 2;
const COL_KEY: usize = 3;
const COL_VALUE: usize = 4;
const COL_ACTIONS: usize = 5;

fn default_col_widths() -> [f32; 6] {
    if crate::i18n::language() == crate::i18n::Language::Zh {
        [48., 64., 112., 80., 200., 40.]
    } else {
        [70., 62., 104., 60., 200., 50.]
    }
}

// ==================== 视图 ====================

pub struct MessagesView {
    route: Route,
    cluster: String,
    topic: String,
    partitions: Vec<i32>,

    // 工具栏
    partition_select: Entity<Select>,
    fetch_mode_select: Entity<Select>,
    max_messages_input: Entity<TextInput>,
    search_in_select: Entity<Select>,
    search_input: Entity<TextInput>,
    show_time_filters: bool,
    start_time_input: Entity<TextInput>,
    end_time_input: Entity<TextInput>,

    // 数据
    messages: Arc<Vec<MessageRecord>>,
    sorted: Arc<Vec<MessageRecord>>,
    pending: Vec<MessageRecord>,
    loading: bool,
    error: Option<String>,
    has_new_data: bool,

    // 流式状态
    request_seq: u64,
    current_request_id: Option<String>,
    is_aborted: bool,
    is_streaming: bool,
    stream_received: usize,
    stream_total: usize,
    stream_filtered: bool,
    last_query_time_ms: Option<u64>,
    query_started: Option<std::time::Instant>,

    // 排序
    partition_sort: Option<bool>,
    offset_sort: Option<bool>,
    key_sort: Option<bool>,
    timestamp_sort: Option<bool>,

    // 列宽
    col_widths: [f32; 6],
    resizing_col: Option<usize>,

    // 详情面板
    selected: Option<(i32, i64)>,
    panel_height: f32,
    resizing_panel: bool,
    detail_format: Entity<Select>,
    detail_search_open: bool,
    detail_search_input: Entity<TextInput>,
    detail_match_ix: usize,
    detail_scroll: ScrollHandle,

    favorite_btn: Option<Entity<FavoriteButton>>,

    // 发送历史
    show_history: bool,
    history_items: Vec<SentMessageItem>,
    history_loading: bool,
    history_search_input: Entity<TextInput>,

    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    merge_scheduled: bool,
    settings_loaded: bool,
}

impl MessagesView {
    pub fn new(route: Route, cx: &mut Context<Self>) -> Self {
        let cluster = route.get("cluster").unwrap_or("").to_string();
        let topic = route.get("topic").unwrap_or("").to_string();

        let partition_select = cx.new(|cx| {
            Select::new(
                vec![SelectOption {
                    value: "all".into(),
                    label: t("messages.allPartitions").into(),
                }],
                "all",
                cx,
            )
        });
        let fetch_mode_select = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption {
                        value: "newest".into(),
                        label: t("messages.newest").into(),
                    },
                    SelectOption {
                        value: "oldest".into(),
                        label: t("messages.oldest").into(),
                    },
                ],
                "newest",
                cx,
            )
        });
        let max_messages_input = cx.new(TextInput::new);
        max_messages_input.update(cx, |i, cx| i.set_text("100", cx));
        let search_in_select = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption {
                        value: "all".into(),
                        label: t("messages.searchAll").into(),
                    },
                    SelectOption {
                        value: "key".into(),
                        label: "Key".into(),
                    },
                    SelectOption {
                        value: "value".into(),
                        label: "Value".into(),
                    },
                ],
                "all",
                cx,
            )
        });
        let search_input = cx.new(TextInput::new);
        search_input.update(cx, |i, _| i.set_placeholder(t("messages.searchPlaceholder")));
        let start_time_input = cx.new(TextInput::new);
        start_time_input.update(cx, |i, _| i.set_placeholder("YYYY-MM-DD HH:mm:ss"));
        let end_time_input = cx.new(TextInput::new);
        end_time_input.update(cx, |i, _| i.set_placeholder("YYYY-MM-DD HH:mm:ss"));
        let detail_format = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption {
                        value: "json".into(),
                        label: "JSON".into(),
                    },
                    SelectOption {
                        value: "raw".into(),
                        label: "Raw".into(),
                    },
                    SelectOption {
                        value: "hex".into(),
                        label: "Hex".into(),
                    },
                ],
                "json",
                cx,
            )
        });
        let detail_search_input = cx.new(TextInput::new);
        detail_search_input.update(cx, |i, _| i.set_placeholder(t("common.search")));
        let history_search_input = cx.new(TextInput::new);
        history_search_input.update(cx, |i, _| i.set_placeholder(t("common.search")));

        let this = Self {
            route: route.clone(),
            cluster,
            topic,
            partitions: vec![],
            partition_select,
            fetch_mode_select,
            max_messages_input,
            search_in_select,
            search_input: search_input.clone(),
            show_time_filters: false,
            start_time_input,
            end_time_input,
            messages: Arc::new(vec![]),
            sorted: Arc::new(vec![]),
            pending: vec![],
            loading: false,
            error: None,
            has_new_data: false,
            request_seq: 0,
            current_request_id: None,
            is_aborted: false,
            is_streaming: false,
            stream_received: 0,
            stream_total: 0,
            stream_filtered: false,
            last_query_time_ms: None,
            query_started: None,
            partition_sort: None,
            offset_sort: None,
            key_sort: None,
            timestamp_sort: Some(false),
            col_widths: default_col_widths(),
            resizing_col: None,
            selected: None,
            panel_height: 380.,
            resizing_panel: false,
            detail_format,
            detail_search_open: false,
            detail_search_input,
            detail_match_ix: 0,
            detail_scroll: ScrollHandle::new(),
            favorite_btn: None,
            show_history: false,
            history_items: vec![],
            history_loading: false,
            history_search_input,
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            merge_scheduled: false,
            settings_loaded: false,
        };

        cx.subscribe(&search_input, |this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EnterPressed) {
                this.query_messages(cx);
            }
        })
        .detach();

        // 初始化：设置 → 分区 → 历史 → 自动查询
        let mut this = this;
        this.initialize(cx);
        this
    }

    fn initialize(&mut self, cx: &mut Context<Self>) {
        // 读取 max_messages 设置
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            if let Ok(v) = b
                .dispatch(
                    "settings.get",
                    serde_json::json!({"keys": ["messages.max_messages"]}),
                )
                .await
            {
                if let Some(val) = v
                    .get("settings")
                    .and_then(|s| s.get("messages.max_messages"))
                    .and_then(|x| x.as_str())
                {
                    if let Ok(n) = val.parse::<i64>() {
                        if (1..=10000).contains(&n) {
                            this.update(cx, |this, cx| {
                                this.max_messages_input.update(cx, |i, cx| {
                                    i.set_text(n.to_string(), cx)
                                });
                                this.settings_loaded = true;
                            })
                            .ok();
                        }
                    }
                }
            }
        })
        .detach();

        // URL 中的 partition 参数
        if let Some(p) = self.route.get("partition") {
            if p.parse::<i32>().is_ok() {
                self.partition_select.update(cx, |s, cx| s.set_value(p, cx));
            }
        }
        // action=send：打开消息页后自动打开发送弹窗
        let open_send = self.route.get("action") == Some("send");

        self.load_partitions(cx);
        self.record_history(cx);
        self.query_messages(cx);
        if open_send {
            self.open_send_modal(cx);
        }
        // 收藏按钮
        if !self.cluster.is_empty() && !self.topic.is_empty() {
            let btn = cx.new(|cx| {
                let mut b = FavoriteButton::new(self.cluster.clone(), self.topic.clone(), false, cx);
                b.check(cx);
                b
            });
            self.favorite_btn = Some(btn);
        }
    }

    /// 路由变化（组件复用）：stop → 重新加载 → 自动查询
    pub fn on_route_changed(&mut self, route: Route, cx: &mut Context<Self>) {
        self.stop_query(cx);
        self.route = route.clone();
        self.cluster = route.get("cluster").unwrap_or("").to_string();
        self.topic = route.get("topic").unwrap_or("").to_string();
        self.messages = Arc::new(vec![]);
        self.sorted = Arc::new(vec![]);
        self.selected = None;
        self.error = None;
        if let Some(p) = route.get("partition") {
            if p.parse::<i32>().is_ok() {
                self.partition_select.update(cx, |s, cx| s.set_value(p, cx));
            }
        }
        let open_send = route.get("action") == Some("send");
        self.favorite_btn = if !self.cluster.is_empty() && !self.topic.is_empty() {
            Some(cx.new(|cx| {
                let mut b =
                    FavoriteButton::new(self.cluster.clone(), self.topic.clone(), false, cx);
                b.check(cx);
                b
            }))
        } else {
            None
        };
        self.load_partitions(cx);
        self.record_history(cx);
        self.query_messages(cx);
        if open_send {
            self.open_send_modal(cx);
        }
        cx.notify();
    }

    pub fn on_topic_deleted(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        if cluster == self.cluster && topic == self.topic {
            self.stop_query(cx);
            self.messages = Arc::new(vec![]);
            self.sorted = Arc::new(vec![]);
            self.selected = None;
            cx.notify();
        }
    }

    pub fn on_highlight_changed(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn record_history(&self, cx: &mut Context<Self>) {
        if self.cluster.is_empty() || self.topic.is_empty() {
            return;
        }
        let b = backend(cx);
        let (c, t2) = (self.cluster.clone(), self.topic.clone());
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch(
                    "topic_history.record",
                    serde_json::json!({"cluster_id": c, "topic_name": t2}),
                )
                .await;
        })
        .detach();
    }

    fn load_partitions(&mut self, cx: &mut Context<Self>) {
        if self.cluster.is_empty() || self.topic.is_empty() {
            return;
        }
        let b = backend(cx);
        let (c, t2) = (self.cluster.clone(), self.topic.clone());
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "topic.get",
                    serde_json::json!({"cluster_id": c, "name": t2}),
                )
                .await;
            this.update(cx, |this, cx| {
                if let Ok(v) = result {
                    this.partitions = v
                        .get("partitions")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|p| p.get("id").and_then(|x| x.as_i64()).map(|n| n as i32))
                                .collect()
                        })
                        .unwrap_or_default();
                    // 更新分区下拉
                    let mut options = vec![SelectOption {
                        value: "all".into(),
                        label: t("messages.allPartitions").into(),
                    }];
                    options.extend(this.partitions.iter().map(|p| SelectOption {
                        value: p.to_string().into(),
                        label: format!("Partition {}", p).into(),
                    }));
                    let current = this.partition_select.read(cx).value.clone();
                    this.partition_select.update(cx, |s, cx| {
                        s.set_options(options, cx);
                        s.set_value(current, cx);
                    });
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ==================== 查询 ====================

    fn parse_time(&self, input: &Entity<TextInput>, cx: &App) -> Option<i64> {
        let s = input.read(cx).text().trim().to_string();
        if s.is_empty() {
            return None;
        }
        crate::views::navigator::parse_iso8601(&s).or_else(|| {
            // 尝试纯日期
            crate::views::navigator::parse_iso8601(&format!("{} 00:00:00", s))
        })
    }

    fn query_messages(&mut self, cx: &mut Context<Self>) {
        if self.cluster.is_empty() || self.topic.is_empty() || self.loading {
            return;
        }
        // 时间校验
        let start_time = self.parse_time(&self.start_time_input, cx);
        let end_time = self.parse_time(&self.end_time_input, cx);
        let start_raw = self.start_time_input.read(cx).text().trim().to_string();
        let end_raw = self.end_time_input.read(cx).text().trim().to_string();
        if !start_raw.is_empty() && start_time.is_none() {
            overlay::toast_error(cx, t("messages.invalidStartTime"));
            return;
        }
        if !end_raw.is_empty() && end_time.is_none() {
            overlay::toast_error(cx, t("messages.invalidEndTime"));
            return;
        }
        if let (Some(s), Some(e)) = (start_time, end_time) {
            if s > e {
                overlay::toast_error(cx, t("messages.startAfterEnd"));
                return;
            }
        }

        self.stop_query(cx);
        self.is_aborted = false;
        self.request_seq += 1;
        let seq = self.request_seq;

        self.loading = true;
        self.error = None;
        self.stream_received = 0;
        self.stream_total = 0;
        self.stream_filtered = false;
        self.is_streaming = false;
        self.has_new_data = false;
        self.query_started = Some(std::time::Instant::now());
        cx.notify();

        // 持久化 max_messages
        let max_messages: i64 = self
            .max_messages_input
            .read(cx)
            .text()
            .trim()
            .parse()
            .unwrap_or(100)
            .clamp(1, 10000);
        {
            let b = backend(cx);
            cx.spawn(async move |_, _| {
                let _ = b
                    .dispatch(
                        "settings.update",
                        serde_json::json!({"key": "messages.max_messages", "value": max_messages.to_string()}),
                    )
                    .await;
            })
            .detach();
        }

        let fetch_mode = self.fetch_mode_select.read(cx).value.to_string();
        let sort = if fetch_mode == "newest" { "desc" } else { "asc" };
        let partition = self.partition_select.read(cx).value.to_string();
        let search = self.search_input.read(cx).text().trim().to_string();
        let search_in = self.search_in_select.read(cx).value.to_string();

        let mut params = serde_json::json!({
            "cluster_id": self.cluster,
            "topic": self.topic,
            "max_messages": max_messages,
            "fetchMode": fetch_mode,
            "sort": sort,
        });
        if partition != "all" {
            if let Ok(p) = partition.parse::<i32>() {
                params["partition"] = serde_json::json!(p);
            }
        } else if !self.partitions.is_empty() {
            params["partitions"] = serde_json::json!(self.partitions);
        }
        if !search.is_empty() {
            params["search"] = serde_json::json!(search);
            params["search_in"] = serde_json::json!(search_in);
        }
        if let Some(s) = start_time {
            params["start_time"] = serde_json::json!(s);
        }
        if let Some(e) = end_time {
            params["end_time"] = serde_json::json!(e);
        }

        let b = backend(cx);
        let (request_id, rx) = b.stream_messages(params);
        self.current_request_id = Some(request_id.clone());

        // 90 秒超时保护
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(90))
                .await;
            this.update(cx, |this, cx| {
                if this.request_seq == seq && this.loading {
                    this.stop_query(cx);
                    this.error = Some(t("messages.queryTimeout"));
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();

        // 事件消费循环
        cx.spawn(async move |this, cx| {
            while let Ok(evt) = rx.recv().await {
                let alive = this
                    .update(cx, |this, cx| {
                        if this.request_seq != seq || this.is_aborted {
                            return false;
                        }
                        this.handle_stream_event(&evt.event, &evt.data, cx);
                        true
                    })
                    .unwrap_or(false);
                if !alive {
                    break;
                }
            }
        })
        .detach();
    }

    fn handle_stream_event(&mut self, event: &str, data: &str, cx: &mut Context<Self>) {
        match event {
            "start" => {
                if self.is_streaming {
                    return;
                }
                let v: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
                self.stream_total = v
                    .get("total_target")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as usize;
                self.stream_received = 0;
                self.stream_filtered = v
                    .get("has_filter")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                self.is_streaming = true;
                cx.notify();
            }
            "batch" => {
                let v: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
                self.stream_received = v
                    .get("progress")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as usize;
                if let Some(t) = v.get("total").and_then(|x| x.as_u64()) {
                    self.stream_total = t as usize;
                }
                if let Some(started) = self.query_started {
                    self.last_query_time_ms = Some(started.elapsed().as_millis() as u64);
                }
                if let Some(arr) = v.get("messages").and_then(|x| x.as_array()) {
                    for item in arr {
                        if let Some(rec) = MessageRecord::from_json(item) {
                            self.pending.push(rec);
                        }
                    }
                }
                self.schedule_merge(cx);
            }
            "complete" => {
                let v: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
                if let Some(t) = v.get("actual_total").and_then(|x| x.as_u64()) {
                    self.stream_total = t as usize;
                }
                self.merge_pending(true, cx);
                if !self.has_new_data {
                    self.messages = Arc::new(vec![]);
                    self.apply_sort(cx);
                }
                self.loading = false;
                self.is_streaming = false;
                self.current_request_id = None;
                cx.notify();
            }
            "error" => {
                let v: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
                self.error = v
                    .get("error")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some(t("messages.queryFailed")));
                self.loading = false;
                self.is_streaming = false;
                cx.notify();
            }
            _ => {}
        }
    }

    /// 节流合并（50~300ms 动态间隔 + 摊平阈值）
    fn schedule_merge(&mut self, cx: &mut Context<Self>) {
        if self.merge_scheduled {
            return;
        }
        self.merge_scheduled = true;
        let interval = (50 + (self.pending.len() / 100) as u64 * 25).min(300);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(interval))
                .await;
            this.update(cx, |this, cx| {
                this.merge_scheduled = false;
                this.merge_pending(false, cx);
            })
            .ok();
        })
        .detach();
    }

    fn merge_pending(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.pending.is_empty() {
            if force && !self.has_new_data {
                self.messages = Arc::new(vec![]);
                self.apply_sort(cx);
            }
            return;
        }
        let threshold = 500.max((self.messages.len() as f32 * 0.25) as usize);
        if !force && self.pending.len() < threshold {
            // 未达阈值且非强制：重新调度
            self.schedule_merge(cx);
            return;
        }
        let mut merged: Vec<MessageRecord> = if self.has_new_data {
            self.messages.as_ref().clone()
        } else {
            // 第一批新数据到达时才清空旧列表
            self.has_new_data = true;
            vec![]
        };
        merged.append(&mut self.pending);
        self.messages = Arc::new(merged);
        self.apply_sort(cx);
        cx.notify();
    }

    pub fn stop_query(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.current_request_id.take() {
            backend(cx).cancel_stream(&id);
        }
        self.request_seq += 1;
        self.loading = false;
        self.is_streaming = false;
        self.pending.clear();
        self.merge_scheduled = false;
        self.is_aborted = true;
        cx.notify();
    }

    // ==================== 排序 ====================

    fn toggle_sort(&mut self, col: usize, cx: &mut Context<Self>) {
        let cycle = |cur: Option<bool>| -> Option<bool> {
            match cur {
                None => Some(true),
                Some(true) => Some(false),
                Some(false) => None,
            }
        };
        match col {
            COL_PARTITION => {
                self.partition_sort = cycle(self.partition_sort);
                self.key_sort = None;
                self.offset_sort = None;
            }
            COL_OFFSET => {
                self.offset_sort = cycle(self.offset_sort);
                self.key_sort = None;
                self.partition_sort = None;
            }
            COL_KEY => {
                self.key_sort = cycle(self.key_sort);
                self.offset_sort = None;
                self.partition_sort = None;
            }
            COL_TIMESTAMP => {
                self.timestamp_sort = cycle(self.timestamp_sort);
                self.partition_sort = None;
                self.offset_sort = None;
                self.key_sort = None;
            }
            _ => {}
        }
        self.apply_sort(cx);
        cx.notify();
    }

    fn apply_sort(&mut self, _cx: &mut Context<Self>) {
        let mut sorted = self.messages.as_ref().clone();
        let ts_sort = self.timestamp_sort;
        sorted.sort_by(|a, b| {
            let cmp = if let Some(asc) = self.partition_sort {
                let c = a.p.cmp(&b.p);
                if asc { c } else { c.reverse() }
            } else if let Some(asc) = self.offset_sort {
                let c = a.o.cmp(&b.o);
                if asc { c } else { c.reverse() }
            } else if let Some(asc) = self.key_sort {
                // 先按字符串长度再按字典序
                let c = a.k.len().cmp(&b.k.len()).then_with(|| a.k.cmp(&b.k));
                if asc { c } else { c.reverse() }
            } else if let Some(asc) = ts_sort {
                let c = a.ts.cmp(&b.ts);
                if asc { c } else { c.reverse() }
            } else {
                std::cmp::Ordering::Equal
            };
            cmp
        });
        self.sorted = Arc::new(sorted);
    }

    // ==================== 详情面板 ====================

    fn selected_message(&self) -> Option<MessageRecord> {
        self.selected.and_then(|(p, o)| {
            self.messages
                .iter()
                .find(|m| m.p == p && m.o == o)
                .cloned()
        })
    }

    fn select_message(&mut self, p: i32, o: i64, cx: &mut Context<Self>) {
        self.selected = Some((p, o));
        self.detail_search_open = false;
        self.detail_match_ix = 0;
        cx.notify();
    }

    fn detail_value_text(&self, msg: &MessageRecord, cx: &App) -> String {
        match self.detail_format.read(cx).value.as_ref() {
            "json" => format_json(&msg.v),
            "hex" => to_hex_dump(&msg.v),
            _ => msg.v.clone(),
        }
    }

    fn detail_matches(&self, text: &str, cx: &App) -> Vec<(usize, usize)> {
        let query = self.detail_search_input.read(cx).text().to_lowercase();
        if query.is_empty() || !self.detail_search_open {
            return vec![];
        }
        let lower = text.to_lowercase();
        let mut matches = Vec::new();
        let mut start = 0;
        while let Some(pos) = lower[start..].find(&query) {
            let abs = start + pos;
            matches.push((abs, abs + query.len()));
            start = abs + query.len().max(1);
        }
        matches
    }

    fn copy_value(&self, formatted: bool, cx: &mut Context<Self>) {
        if let Some(msg) = self.selected_message() {
            let text = if formatted {
                format_json(&msg.v)
            } else {
                msg.v.clone()
            };
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            overlay::toast_success(cx, t("common.copied"));
        }
    }

    fn copy_key(&self, cx: &mut Context<Self>) {
        if let Some(msg) = self.selected_message() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(msg.k.clone()));
            overlay::toast_success(cx, t("common.copied"));
        }
    }

    fn load_full_message(&mut self, cx: &mut Context<Self>) {
        let Some((p, o)) = self.selected else { return };
        let b = backend(cx);
        let (c, t2) = (self.cluster.clone(), self.topic.clone());
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "message.get",
                    serde_json::json!({
                        "cluster_id": c,
                        "topic": t2,
                        "partition": p,
                        "offset": o,
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(v) => {
                        if let Some(rec) = MessageRecord::from_json(&v) {
                            let mut messages = this.messages.as_ref().clone();
                            if let Some(existing) =
                                messages.iter_mut().find(|m| m.uid == rec.uid)
                            {
                                *existing = rec;
                            }
                            this.messages = Arc::new(messages);
                            this.apply_sort(cx);
                        }
                    }
                    Err(e) => overlay::toast_error(cx, e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ==================== 发送 ====================

    pub fn open_send_modal(&mut self, cx: &mut Context<Self>) {
        let selected = self.selected_message();
        let (init_p, init_k, init_v) = match &selected {
            Some(m) => (Some(m.p), Some(m.k.clone()), Some(m.v.clone())),
            None => (None, None, None),
        };
        let source = cx.entity().downgrade();
        let view = cx.new(|cx| {
            crate::dialogs::send_message::SendMessageDialog::new(
                self.cluster.clone(),
                self.topic.clone(),
                self.partitions.clone(),
                init_p,
                init_k,
                init_v,
                source,
                cx,
            )
        });
        overlay::open_modal(cx, view.into());
    }

    /// 发送成功后自动重新查询
    pub fn on_message_sent(&mut self, cx: &mut Context<Self>) {
        self.loading = false;
        self.query_messages(cx);
    }

    // ==================== 发送历史 ====================

    fn toggle_history(&mut self, cx: &mut Context<Self>) {
        self.show_history = !self.show_history;
        if self.show_history {
            self.load_sent_history(cx);
        }
        cx.notify();
    }

    fn load_sent_history(&mut self, cx: &mut Context<Self>) {
        self.history_loading = true;
        cx.notify();
        let b = backend(cx);
        let (c, t2) = (self.cluster.clone(), self.topic.clone());
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "sent_message.list",
                    serde_json::json!({
                        "limit": 100,
                        "offset": 0,
                        "cluster_id": c,
                        "topic_name": t2,
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.history_loading = false;
                if let Ok(v) = result {
                    this.history_items = v
                        .get("messages")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    Some(SentMessageItem {
                                        id: item.get("id")?.as_i64()?,
                                        cluster_id: item
                                            .get("cluster_id")?
                                            .as_str()?
                                            .to_string(),
                                        cluster_name: item
                                            .get("cluster_name")
                                            .and_then(|x| x.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        topic_name: item
                                            .get("topic_name")?
                                            .as_str()?
                                            .to_string(),
                                        partition: item
                                            .get("partition")
                                            .and_then(|x| x.as_i64())
                                            .unwrap_or(0)
                                            as i32,
                                        message_key: item
                                            .get("message_key")
                                            .and_then(|x| x.as_str())
                                            .map(|s| s.to_string()),
                                        message_value: item
                                            .get("message_value")
                                            .and_then(|x| x.as_str())
                                            .map(|s| s.to_string()),
                                        headers: item.get("headers").cloned(),
                                        offset: item.get("offset").and_then(|x| x.as_i64()),
                                        sent_at: item
                                            .get("sent_at")
                                            .and_then(|x| x.as_str())
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

    fn clear_sent_history(&mut self, cx: &mut Context<Self>) {
        overlay::confirm(cx, t("sentMessageHistory.clearTitle"), t("sentMessageHistory.clearConfirm"), true, |cx| {
            let b = backend(cx);
            cx.spawn(async move |cx| {
                let _ = b
                    .dispatch("sent_message.clear", serde_json::json!({}))
                    .await;
                cx.update(|cx| {
                    if let Some(v) = root(cx).read(cx).page_messages() {
                        v.update(cx, |view, cx| {
                            view.history_items = vec![];
                            cx.notify();
                        });
                    }
})
            })
            .detach();
        });
    }

    fn delete_sent_item(&mut self, id: i64, cx: &mut Context<Self>) {
        self.history_items.retain(|i| i.id != id);
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch("sent_message.delete", serde_json::json!({"id": id}))
                .await;
        })
        .detach();
    }

    /// 双击历史记录 → 回填发送表单
    fn resend_from_history(&mut self, item: &SentMessageItem, cx: &mut Context<Self>) {
        self.show_history = false;
        let source = cx.entity().downgrade();
        let view = cx.new(|cx| {
            crate::dialogs::send_message::SendMessageDialog::new(
                item.cluster_id.clone(),
                item.topic_name.clone(),
                self.partitions.clone(),
                Some(item.partition),
                item.message_key.clone(),
                item.message_value.clone(),
                source,
                cx,
            )
        });
        overlay::open_modal(cx, view.into());
        cx.notify();
    }

    // ==================== 导出 ====================

    fn export_messages(&self, cx: &mut Context<Self>) {
        if self.messages.is_empty() {
            return;
        }
        let data: Vec<serde_json::Value> = self
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "partition": m.p,
                    "offset": m.o,
                    "key": m.k,
                    "value": m.v,
                    "timestamp": m.ts,
                    "uid": m.uid,
                })
            })
            .collect();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let filename = format!("{}_messages_{}.json", self.topic, timestamp);
        let picker = cx.new(|cx| {
            crate::dialogs::file_picker::FilePickerDialog::new(
                crate::dialogs::file_picker::PickerMode::Save { filename },
                cx,
            )
        });
        let json = serde_json::to_string_pretty(&data).unwrap_or_default();
        cx.subscribe(&picker, move |_, _, event: &crate::dialogs::file_picker::PickerEvent, cx| {
            let crate::dialogs::file_picker::PickerEvent::Selected(path) = event;
            let ok = std::fs::write(path, &json).is_ok();
            if ok {
                overlay::toast_success(cx, t("messages.exportSuccess"));
            } else {
                overlay::toast_error(cx, t("messages.exportFailed"));
            }
        })
        .detach();
        overlay::open_modal(cx, picker.into());
    }

    // ==================== 键盘导航 ====================

    fn msg_select_up(&mut self, _: &MsgSelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(-1, cx);
    }

    fn msg_select_down(&mut self, _: &MsgSelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(1, cx);
    }

    /// ↑/↓ 循环移动选中（到底回卷），并滚动到可见
    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.sorted.len();
        if len == 0 {
            return;
        }
        let current = self.selected.and_then(|(p, o)| {
            self.sorted.iter().position(|m| m.p == p && m.o == o)
        });
        let next = match current {
            Some(ix) => (ix as isize + delta).rem_euclid(len as isize) as usize,
            None => {
                if delta > 0 {
                    0
                } else {
                    len - 1
                }
            }
        };
        if let Some(msg) = self.sorted.get(next) {
            self.selected = Some((msg.p, msg.o));
            self.scroll.scroll_to_item(next, ScrollStrategy::Center);
            cx.notify();
        }
    }

    // ==================== 时间预设 ====================

    fn set_preset(&mut self, minutes: i64, cx: &mut Context<Self>) {
        self.show_time_filters = true;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let end = now - minutes * 60_000;
        let start = now - 2 * minutes * 60_000;
        let fmt = |ms: i64| {
            crate::views::navigator::format_datetime(ms, false).replace('/', "-")
        };
        self.start_time_input.update(cx, |i, cx| i.set_text(fmt(start), cx));
        self.end_time_input.update(cx, |i, cx| i.set_text(fmt(end), cx));
        cx.notify();
    }

    // ==================== 渲染 ====================

    fn render_toolbar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_query = !self.cluster.is_empty() && !self.topic.is_empty() && !self.loading;
        let search_empty = self.search_input.read(cx).is_empty();

        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(8.))
            .py(px(6.))
            .flex_none()
            // 返回
            .child(
                icon_btn("msg-back", "arrow-left", BtnSize::Sm)
                    .when(!root(cx).read(cx).can_go_back(), |b| b.opacity(0.5))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        root(cx).update(cx, |app, cx| app.go_back(cx));
                    }),
            )
            // 分区
            .child(div().w(px(112.)).child(self.partition_select.clone()))
            // 查询模式
            .child(div().w(px(96.)).child(self.fetch_mode_select.clone()))
            // 数量
            .child(
                div().w(px(64.)).child(input_frame(self.max_messages_input.clone())),
            )
            // 时间范围开关
            .child(
                btn("time-filter-toggle", if self.show_time_filters { BtnKind::Primary } else { BtnKind::Ghost }, BtnSize::Sm)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.show_time_filters = !this.show_time_filters;
                            cx.notify();
                        }),
                    )
                    .child(icon("clock").size(px(14.)).into_any_element())
                    .child(t("messages.timeRange")),
            )
            // 搜索组
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(0.))
                    .child(div().w(px(84.)).child(self.search_in_select.clone()))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .child(div().flex_1().child(input_frame(self.search_input.clone())))
                            .when(!search_empty, |d| {
                                d.child(
                                    icon_btn("clear-search", "x-mark", BtnSize::Sm)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.search_input.update(cx, |i, cx| i.reset(cx));
                                                this.query_messages(cx);
                                            }),
                                        ),
                                )
                            }),
                    ),
            )
            // 查询按钮
            .child(
                btn("query-btn", BtnKind::Primary, BtnSize::Sm)
                    .when(!can_query, |b| b.opacity(0.5))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.query_messages(cx)),
                    )
                    .child(if self.loading {
                        spinner(14.)
                    } else {
                        icon("search").size(px(14.)).into_any_element()
                    }),
            )
            // 停止按钮（仅 loading）
            .when(self.loading, |d| {
                d.child(
                    icon_btn("stop-query", "x-mark", BtnSize::Sm)
                        .text_color(theme::error())
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.stop_query(cx)),
                        ),
                )
            })
            // 发送消息
            .child(
                icon_btn("open-send", "send", BtnSize::Sm).on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        if !this.cluster.is_empty() && !this.topic.is_empty() {
                            this.open_send_modal(cx);
                        }
                    }),
                ),
            )
    }

    fn render_time_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_time = !self.start_time_input.read(cx).is_empty()
            || !self.end_time_input.read(cx).is_empty();
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(8.))
            .py(px(6.))
            .flex_none()
            .bg(theme::base_content_alpha(0.04))
            .border_b_1()
            .border_color(theme::border_base_200())
            .child(input_frame(self.start_time_input.clone()).into_any_element())
            .child(div().text_color(theme::text_secondary()).child("-"))
            .child(input_frame(self.end_time_input.clone()).into_any_element())
            .child(div().w(px(8.)))
            .children([5, 15, 30, 60, 1440].into_iter().map(|mins| {
                let label = match mins {
                    5 => t("messages.preset5m"),
                    15 => t("messages.preset15m"),
                    30 => t("messages.preset30m"),
                    60 => t("messages.preset1h"),
                    _ => t("messages.preset1d"),
                };
                btn(("preset", mins as usize), BtnKind::Ghost, BtnSize::Xs)
                    .child(label)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| this.set_preset(mins, cx)),
                    )
                    .into_any_element()
            }))
            .child(div().flex_1())
            .child(
                btn("clear-time", BtnKind::Ghost, BtnSize::Xs)
                    .when(!has_time, |b| b.opacity(0.4))
                    .child(t("common.clear"))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.start_time_input.update(cx, |i, cx| i.reset(cx));
                            this.end_time_input.update(cx, |i, cx| i.reset(cx));
                        }),
                    ),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let progress_pct = if self.stream_total > 0 {
            (self.stream_received as f32 / self.stream_total as f32 * 100.0).min(100.0)
        } else {
            0.0
        };

        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(8.))
            .h(px(28.))
            .flex_none()
            .text_size(px(11.))
            .text_color(theme::text_secondary())
            // Topic + 收藏
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(t("messages.topic"))
                    .child(
                        div()
                            .font_family("monospace")
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::badge_primary_text())
                            .child(self.topic.clone()),
                    )
                    .when_some(self.favorite_btn.clone(), |d, btn| d.child(btn)),
            )
            // 流式进度
            .when(self.is_streaming, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(spinner(11.))
                        .child(if !self.stream_filtered && self.stream_total > 0 {
                            format!(
                                "{} {} / {}",
                                t("messages.receiving"),
                                self.stream_received,
                                format_number(self.stream_total)
                            )
                        } else {
                            format!("{} {}", t("messages.receiving"), self.stream_received)
                        }),
                )
            })
            // 耗时
            .when(!self.is_streaming && self.last_query_time_ms.is_some(), |d| {
                d.child(format!(
                    "{}: {}ms",
                    t("messages.elapsed"),
                    self.last_query_time_ms.unwrap_or(0)
                ))
            })
            // 消息数
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(t("messages.total"))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme::success())
                            .child(self.messages.len().to_string()),
                    )
                    .child(t("messages.count")),
            )
            // 工具按钮组
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(0.))
                    .child(icon_btn("export-info", "info", BtnSize::Xs))
                    .child(
                        div()
                            .id("export-messages")
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .when(self.messages.is_empty(), |d| d.opacity(0.4))
                            .text_color(theme::text_secondary())
                            .hover(|s| s.bg(theme::btn_ghost_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.export_messages(cx)),
                            )
                            .child(icon("download").size(px(13.))),
                    )
                    .child(
                        icon_btn("sent-history", "clock", BtnSize::Xs)
                            .when(self.cluster.is_empty() || self.topic.is_empty(), |b| {
                                b.opacity(0.4)
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.toggle_history(cx)),
                            ),
                    )
                    .child(
                        icon_btn("goto-consumer-groups", "users", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                let (c, t2) = (this.cluster.clone(), this.topic.clone());
                                root(cx).update(cx, |app, cx| {
                                    let route = Route::new(Page::TopicConsumerGroups)
                                        .with("cluster", &c)
                                        .with("topic", &t2);
                                    app.navigate(route, true, cx);
                                });
                            }),
                        ),
                    )
                    .child(
                        div()
                            .id("delete-current-topic")
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .text_color(theme::error())
                            .hover(|s| s.bg(theme::btn_ghost_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    let (c, t2) = (this.cluster.clone(), this.topic.clone());
                                    let view = cx.new(|cx| {
                                        crate::dialogs::delete_topic::DeleteTopicDialog::new(
                                            c, t2, cx,
                                        )
                                    });
                                    overlay::open_modal(cx, view.into());
                                }),
                            )
                            .child(icon("trash").size(px(13.))),
                    ),
            )
            // 错误
            .when_some(self.error.clone(), |d, err| {
                d.child(
                    div()
                        .text_color(theme::error())
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(err),
                )
            })
            .child(div().flex_1())
            // 进度条
            .when(self.is_streaming && !self.stream_filtered && self.stream_total > 0, |d| {
                d.child(
                    div()
                        .w(px(120.))
                        .h(px(4.))
                        .rounded(px(2.))
                        .bg(theme::base_content_alpha(0.1))
                        .child(
                            div()
                                .h_full()
                                .rounded(px(2.))
                                .bg(theme::info())
                                .w(gpui::relative(progress_pct / 100.0)),
                        ),
                )
            })
    }

    fn render_header_cell(&self, col: usize, label: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let sort_state = match col {
            COL_PARTITION => self.partition_sort,
            COL_OFFSET => self.offset_sort,
            COL_KEY => self.key_sort,
            COL_TIMESTAMP => self.timestamp_sort,
            _ => None,
        };
        let width = self.col_widths[col];
        let sortable = col != COL_VALUE && col != COL_ACTIONS;

        div()
            .id(("header-cell", col))
            .flex()
            .items_center()
            .gap(px(2.))
            .h_full()
            .w(px(width))
            .flex_none()
            .px(px(4.))
            .when(sortable, |d| {
                d.cursor_pointer()
                    .hover(|s| s.bg(theme::table_row_hover()))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| this.toggle_sort(col, cx)),
                    )
            })
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme::text_secondary())
                    .child(label.to_string()),
            )
            .when(sortable, |d| {
                d.child(
                    icon(match sort_state {
                        Some(true) => "arrow-up",
                        Some(false) => "arrow-down",
                        None => "sort-both",
                    })
                    .size(px(10.))
                    .text_color(if sort_state.is_some() {
                        theme::badge_primary_text()
                    } else {
                        theme::base_content_alpha(0.35)
                    }),
                )
            })
    }

    fn render_col_resizer(&self, col: usize, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(("col-resizer", col))
            .w(px(4.))
            .flex_none()
            .h_full()
            .cursor_col_resize()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.resizing_col = Some(col);
                    cx.notify();
                }),
            )
    }

    fn render_detail_panel(&mut self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let msg = self.selected_message()?;
        let value_text = self.detail_value_text(&msg, cx);
        let matches = self.detail_matches(&value_text, cx);
        let match_count = matches.len();
        let current_match = self.detail_match_ix.min(match_count.saturating_sub(1));

        // JSON 语法高亮 + 搜索高亮
        let format = self.detail_format.read(cx).value.to_string();
        let app = root(cx);
        let template = app.read(cx).current_highlight_style();
        let styles = styles_for_current_theme(&template);

        let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
        if format == "json" {
            let tokens = tokenize(&value_text);
            let mut byte_offset = 0;
            for token in tokens {
                let token_start = byte_offset;
                let token_end = byte_offset + token.text.len();
                byte_offset = token_end;
                if token.kind == TokenKind::Whitespace {
                    continue;
                }
                let (color, weight) = token_color(styles, token.kind);
                highlights.push((
                    token_start..token_end,
                    HighlightStyle {
                        color: Some(color),
                        font_weight: Some(weight),
                        ..Default::default()
                    },
                ));
            }
        }
        // 搜索匹配高亮
        for (ix, (start, end)) in matches.iter().enumerate() {
            let is_current = ix == current_match;
            highlights.push((
                *start..*end,
                HighlightStyle {
                    background_color: Some(if is_current {
                        theme::search_highlight_current()
                    } else {
                        theme::search_highlight_bg()
                    }),
                    color: if is_current {
                        Some(gpui::white())
                    } else {
                        None
                    },
                    ..Default::default()
                },
            ));
        }

        let panel_height = px(self.panel_height.clamp(150., 600.));

        Some(
            div()
                .flex_none()
                .h(panel_height)
                .flex()
                .flex_col()
                .border_t_1()
                .border_color(theme::border_base_200())
                .bg(theme::base_100())
                // 拖拽手柄
                .child(
                    div()
                        .id("panel-resizer")
                        .h(px(5.))
                        .flex_none()
                        .cursor_ns_resize()
                        .hover(|s| s.bg(theme::primary_alpha(0.3)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.resizing_panel = true;
                                cx.notify();
                            }),
                        ),
                )
                // 头部
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .px(px(10.))
                        .h(px(32.))
                        .flex_none()
                        .border_b_1()
                        .border_color(theme::border_base_200())
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme::text_primary())
                                .child(t("messages.detail")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::text_secondary())
                                .child(format!("Partition: {}", msg.p)),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::text_secondary())
                                .child(format!("Offset: {}", msg.o)),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::text_secondary())
                                .child(
                                    msg.ts
                                        .map(|ts| {
                                            crate::views::navigator::format_datetime(ts, false)
                                        })
                                        .unwrap_or_default(),
                                ),
                        )
                        .child(div().flex_1())
                        // 面板搜索
                        .when(self.detail_search_open, |d| {
                            d.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .child(
                                        div().w(px(140.)).child(
                                            div()
                                                .h(px(24.))
                                                .flex()
                                                .items_center()
                                                .px(px(4.))
                                                .rounded(px(5.))
                                                .bg(theme::input_bg())
                                                .border_1()
                                                .border_color(theme::base_content_alpha(0.15))
                                                .text_size(px(11.))
                                                .child(self.detail_search_input.clone()),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(theme::text_secondary())
                                            .child(if match_count > 0 {
                                                format!("{}/{}", current_match + 1, match_count)
                                            } else {
                                                format!("0 {}", t("messages.matches"))
                                            }),
                                    )
                                    .child(
                                        icon_btn("match-prev", "chevron-up", BtnSize::Xs)
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    let count = this
                                                        .selected_message()
                                                        .map(|m| {
                                                            let text =
                                                                this.detail_value_text(&m, cx);
                                                            this.detail_matches(&text, cx).len()
                                                        })
                                                        .unwrap_or(0);
                                                    if count > 0 {
                                                        this.detail_match_ix = (this.detail_match_ix
                                                            + count
                                                            - 1)
                                                            % count;
                                                        cx.notify();
                                                    }
                                                }),
                                            ),
                                    )
                                    .child(
                                        icon_btn("match-next", "chevron-down", BtnSize::Xs)
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    let count = this
                                                        .selected_message()
                                                        .map(|m| {
                                                            let text =
                                                                this.detail_value_text(&m, cx);
                                                            this.detail_matches(&text, cx).len()
                                                        })
                                                        .unwrap_or(0);
                                                    if count > 0 {
                                                        this.detail_match_ix =
                                                            (this.detail_match_ix + 1) % count;
                                                        cx.notify();
                                                    }
                                                }),
                                            ),
                                    ),
                            )
                        })
                        .child(
                            icon_btn("detail-close", "x-mark", BtnSize::Xs).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.selected = None;
                                    cx.notify();
                                }),
                            ),
                        ),
                )
                // 内容区
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .p(px(10.))
                        .overflow_hidden()
                        // Key 区块
                        .when(!msg.k.is_empty(), |d| {
                            d.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .flex_none()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(theme::text_secondary())
                                                    .child("Key"),
                                            )
                                            .child(
                                                btn("copy-key", BtnKind::Ghost, BtnSize::Xs)
                                                    .child(t("common.copy"))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.copy_key(cx)
                                                        }),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .px(px(8.))
                                            .py(px(5.))
                                            .rounded(px(5.))
                                            .bg(theme::base_content_alpha(0.05))
                                            .font_family("monospace")
                                            .text_size(px(11.))
                                            .text_color(theme::text_primary())
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .child(msg.k.clone()),
                                    ),
                            )
                        })
                        // Value 区块
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .overflow_hidden()
                                .child(
                                    div()
                                        .flex()
                                        .flex_none()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .text_color(theme::text_secondary())
                                                .child("Value"),
                                        )
                                        .child(
                                            div().w(px(90.)).child(self.detail_format.clone()),
                                        )
                                        .child(div().flex_1())
                                        .child(
                                            btn("detail-search-toggle", BtnKind::Ghost, BtnSize::Xs)
                                                .child(t("common.search"))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(|this, _, _, cx| {
                                                        this.detail_search_open =
                                                            !this.detail_search_open;
                                                        this.detail_match_ix = 0;
                                                        cx.notify();
                                                    }),
                                                ),
                                        )
                                        .child(
                                            btn("copy-value-btn", BtnKind::Ghost, BtnSize::Xs)
                                                .child(t("common.copy"))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(|this, _, _, cx| {
                                                        this.copy_value(false, cx)
                                                    }),
                                                ),
                                        ),
                                )
                                // 截断警告
                                .when(msg.vt, |d| {
                                    d.child(
                                        div()
                                            .flex()
                                            .flex_none()
                                            .items_center()
                                            .gap(px(6.))
                                            .px(px(8.))
                                            .py(px(4.))
                                            .rounded(px(5.))
                                            .bg(theme::warning())
                                            .child(
                                                icon("warn")
                                                    .size(px(12.))
                                                    .text_color(theme::warning_content()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(theme::warning_content())
                                                    .child(t("messages.truncated")),
                                            )
                                            .child(
                                                btn("load-full", BtnKind::Ghost, BtnSize::Xs)
                                                    .child(t("messages.loadFull"))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.load_full_message(cx)
                                                        }),
                                                    ),
                                            ),
                                    )
                                })
                                .child(
                                    div().id("views_messages_rs_2")
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .track_scroll(&self.detail_scroll)
                                        .rounded(px(5.))
                                        .bg(theme::base_content_alpha(0.05))
                                        .p(px(8.))
                                        .font_family("monospace")
                                        .text_size(px(11.))
                                        .child(
                                            StyledText::new(value_text)
                                                .with_highlights(highlights),
                                        ),
                                ),
                        ),
                ),
        )
    }

    fn render_history_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let search = self.history_search_input.read(cx).text().to_lowercase();
        let items: Vec<SentMessageItem> = self
            .history_items
            .iter()
            .filter(|i| search.is_empty() || i.topic_name.to_lowercase().contains(&search))
            .cloned()
            .collect();

        div()
            .absolute()
            .inset_0()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::base_100())
            // 头部
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(10.))
                    .h(px(36.))
                    .flex_none()
                    .border_b_1()
                    .border_color(theme::border_base_200())
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("sentMessageHistory.title")),
                    )
                    .child(
                        div()
                            .id("sent-clear")
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .text_color(theme::error())
                            .hover(|s| s.bg(theme::btn_ghost_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.clear_sent_history(cx)),
                            )
                            .child(icon("trash").size(px(13.))),
                    )
                    .child(
                        icon_btn("sent-refresh", "refresh", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.load_sent_history(cx)),
                        ),
                    )
                    .child(
                        icon_btn("sent-close", "x-mark", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.show_history = false;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            // 搜索
            .child(
                div().px(px(10.)).py(px(6.)).child(
                    div()
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(theme::input_bg())
                        .border_1()
                        .border_color(theme::base_content_alpha(0.15))
                        .text_size(px(11.))
                        .child(self.history_search_input.clone()),
                ),
            )
            // 列表
            .child(
                div().id("views_messages_rs_3")
                    .flex_1()
                    .overflow_y_scroll()
                    .when(self.history_loading, |d| {
                        d.child(loading_block(t("common.loading")))
                    })
                    .when(!self.history_loading && items.is_empty(), |d| {
                        d.child(empty_block(
                            "send",
                            if self.history_items.is_empty() {
                                t("sentMessageHistory.empty")
                            } else {
                                t("sentMessageHistory.noResults")
                            },
                            t("sentMessageHistory.emptyDesc"),
                        ))
                    })
                    .children(items.into_iter().enumerate().map(|(ix, item)| {
                        let item2 = item.clone();
                        div()
                            .id(("sent-item", ix))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .px(px(10.))
                            .py(px(6.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::context_menu_item_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    if event.click_count == 2 {
                                        this.resend_from_history(&item2, cx);
                                    }
                                }),
                            )
                            .child(
                                icon("send")
                                    .size(px(14.))
                                    .text_color(theme::text_secondary()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme::text_primary())
                                                    .child(item.topic_name.clone()),
                                            )
                                            .child(badge(item.cluster_name.clone(), BadgeKind::Ghost))
                                            .child(badge(
                                                format!("P{}", item.partition),
                                                BadgeKind::Primary,
                                            ))
                                            .child(div().flex_1())
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(theme::text_secondary())
                                                    .child(
                                                        crate::views::navigator::relative_time(
                                                            item.sent_at.as_deref(),
                                                        ),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(8.))
                                            .text_size(px(10.))
                                            .font_family("monospace")
                                            .text_color(theme::text_secondary())
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .child(format!(
                                                "K: {}",
                                                item.message_key.clone().unwrap_or_else(|| "-".into())
                                            ))
                                            .child(format!(
                                                "V: {}",
                                                item.message_value
                                                    .clone()
                                                    .unwrap_or_default()
                                                    .chars()
                                                    .take(100)
                                                    .collect::<String>()
                                            )),
                                    ),
                            )
                            .child(
                                icon_btn(("sent-del", ix), "trash", BtnSize::Xs)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _, _, cx| {
                                            this.delete_sent_item(item.id, cx)
                                        }),
                                    ),
                            )
                    })),
            )
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl Focusable for MessagesView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MessagesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = window;
        let sorted = self.sorted.clone();
        let selected = self.selected;
        let col_widths = self.col_widths;

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .key_context("KmMessages")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::msg_select_up))
            .on_action(cx.listener(Self::msg_select_down))
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                if let Some(col) = this.resizing_col {
                    // 列起点 = 侧栏宽 + 前面各列宽
                    let new_width = event.position.x
                        - px(232.0 + col_widths_sum(&this.col_widths, col));
                    this.col_widths[col] = f32::from(new_width).max(30.0);
                    cx.notify();
                }
                if this.resizing_panel {
                    let window_height = window.viewport_size().height;
                    let new_height = window_height - event.position.y;
                    this.panel_height = f32::from(new_height).clamp(150., 600.);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.resizing_col.is_some() || this.resizing_panel {
                        this.resizing_col = None;
                        this.resizing_panel = false;
                        cx.notify();
                    }
                }),
            )
            .child(self.render_toolbar(cx))
            .when(self.show_time_filters, |d| {
                d.child(self.render_time_panel(cx))
            })
            .child(self.render_status_bar(cx))
            // 列表 + 历史面板（relative 容器）
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    // 表头
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .h(px(28.))
                            .flex_none()
                            .border_b_1()
                            .border_color(theme::border_base_200())
                            .bg(theme::base_100())
                            .child(self.render_header_cell(COL_PARTITION, "Partition", cx))
                            .child(self.render_col_resizer(COL_PARTITION, cx))
                            .child(self.render_header_cell(COL_OFFSET, "Offset", cx))
                            .child(self.render_col_resizer(COL_OFFSET, cx))
                            .child(self.render_header_cell(COL_TIMESTAMP, "Timestamp", cx))
                            .child(self.render_col_resizer(COL_TIMESTAMP, cx))
                            .child(self.render_header_cell(COL_KEY, "Key", cx))
                            .child(self.render_col_resizer(COL_KEY, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(4.))
                                    .text_size(px(10.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme::text_secondary())
                                    .child("Value"),
                            )
                            .child(
                                div()
                                    .w(px(col_widths[COL_ACTIONS]))
                                    .flex_none()
                                    .flex()
                                    .justify_center()
                                    .text_size(px(10.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme::text_secondary())
                                    .child(t("common.actions")),
                            ),
                    )
                    // 列表体
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .when(self.loading && sorted.is_empty(), |d| {
                                d.child(loading_block(t("common.loading")))
                            })
                            .when(!self.loading && sorted.is_empty() && self.error.is_none(), |d| {
                                d.child(empty_block("chat", t("messages.empty"), ""))
                            })
                            .when(!sorted.is_empty(), |d| {
                                d.child(
                                    gpui::uniform_list(
                                        "message-list",
                                        sorted.len(),
                                        {
                                            let sorted = sorted.clone();
                                            move |range, _window, _cx| {
                                                let mut out = Vec::with_capacity(range.len());
                                                for ix in range {
                                                    let Some(msg) = sorted.get(ix) else {
                                                        continue;
                                                    };
                                                    let is_selected =
                                                        selected == Some((msg.p, msg.o));
                                                    out.push(
                                                        div()
                                                            .id(("msg-row", ix))
                                                            .child(render_row_static(
                                                                msg,
                                                                is_selected,
                                                                &col_widths,
                                                                ix,
                                                            ))
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                {
                                                                    let (p, o) = (msg.p, msg.o);
                                                                    move |_, _, cx| {
                                                                        root(cx).update(cx, |app, cx| {
                                                                            if let PageView::Messages(v) = &app.page_view {
                                                                                v.update(cx, |view, cx| {
                                                                                    view.select_message(p, o, cx)
                                                                                });
                                                                            }
                                                                        });
                                                                    }
                                                                },
                                                            ),
                                                    );
                                                }
                                                out
                                            }
                                        },
                                    )
                                    .track_scroll(&self.scroll)
                                    .size_full(),
                                )
                            }),
                    )
                    // 发送历史浮层
                    .when(self.show_history, |d| {
                        d.child(self.render_history_panel(cx))
                    }),
            )
            // 详情面板
            .when_some(self.render_detail_panel(cx), |d, panel| d.child(panel))
    }
}

fn col_widths_sum(widths: &[f32; 6], up_to: usize) -> f32 {
    widths[..up_to].iter().sum()
}

/// 静态行渲染（供 uniform_list 闭包使用）
fn render_row_static(msg: &MessageRecord, selected: bool, col_widths: &[f32; 6], ix: usize) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .h(px(24.))
        .text_size(px(11.))
        .cursor_pointer()
        .when(selected, |d| {
            d.bg(theme::selected_row_bg())
                .border_l_2()
                .border_color(theme::primary())
        })
        .hover(|s| {
            if selected {
                s.bg(theme::selected_row_bg())
            } else {
                s.bg(theme::table_row_hover())
            }
        })
        .child(
            div()
                .w(px(col_widths[COL_PARTITION]))
                .flex_none()
                .px(px(4.))
                .child(badge(msg.p.to_string(), BadgeKind::Ghost)),
        )
        .child(
            div()
                .w(px(col_widths[COL_OFFSET]))
                .flex_none()
                .px(px(4.))
                .font_family("monospace")
                .text_color(theme::text_primary())
                .overflow_hidden()
                .whitespace_nowrap()
                .child(msg.o.to_string()),
        )
        .child(
            div()
                .w(px(col_widths[COL_TIMESTAMP]))
                .flex_none()
                .px(px(4.))
                .text_color(theme::text_secondary())
                .overflow_hidden()
                .whitespace_nowrap()
                .child(
                    msg.ts
                        .map(|ts| crate::views::navigator::format_datetime(ts, false))
                        .unwrap_or_else(|| "-".into()),
                ),
        )
        .child(
            div()
                .w(px(col_widths[COL_KEY]))
                .flex_none()
                .px(px(4.))
                .font_family("monospace")
                .text_color(theme::text_primary())
                .overflow_hidden()
                .whitespace_nowrap()
                .child(if msg.k.is_empty() {
                    "-".into()
                } else {
                    msg.k.clone()
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(60.))
                .px(px(4.))
                .font_family("monospace")
                .text_color(theme::text_secondary())
                .overflow_hidden()
                .whitespace_nowrap()
                .child(msg.v.clone()),
        )
        .child(
            div()
                .w(px(col_widths[COL_ACTIONS]))
                .flex_none()
                .flex()
                .justify_center()
                .child(
                    div()
                        .id(("copy-value-row", ix))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(18.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .text_color(theme::text_secondary())
                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                        .on_mouse_down(MouseButton::Left, {
                            let value = msg.v.clone();
                            move |_, _, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                    format_json(&value),
                                ));
                                overlay::toast_success(cx, t("common.copied"));
                            }
                        })
                        .child(icon("clipboard").size(px(11.))),
                ),
        )
}

