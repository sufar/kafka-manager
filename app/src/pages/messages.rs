//! 消息查询页：与旧版 MessageQueryTool 布局一致
//!
//! 工具栏：分区下拉 / 模式 / max / 时间范围切换(+子面板) / 搜索组合框 / 查询 / 停止 / 发送
//! 状态栏：topic+收藏星 / 流式指示 / 总数 / 导出·历史·消费组·删除 / 进度条
//! 表格：6 列（P/Offset/Time/Key/Value/Copy），逐列排序，列宽拖拽
//! 详情面板：可调高（json/raw/hex + 元数据 + 复制）
//! 发送模态（headers + send-and-continue + 选中预填）；历史覆盖面板（双击重发）

use std::rc::Rc;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectState};
use gpui_component::*;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::components::navigator::NavEvent;
use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::components::notify;
use crate::state::{Backend, TokioRuntime};

/// 单条消息
#[derive(Clone, Debug)]
struct Message {
    partition: i32,
    offset: i64,
    timestamp: i64,
    key: Option<String>,
    value: Option<String>,
}

/// 排序列
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SortCol {
    Partition,
    Offset,
    Timestamp,
    Key,
}

/// 详情面板 Value 显示格式
#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailFormat {
    Json,
    Raw,
    Hex,
}

/// 列宽（px）
#[derive(Clone, Copy)]
struct ColumnWidths {
    partition: f32,
    offset: f32,
    timestamp: f32,
    key: f32,
    actions: f32,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            partition: 48.0,
            offset: 64.0,
            timestamp: 112.0,
            key: 80.0,
            actions: 40.0,
        }
    }
}

/// 列宽拖拽状态
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ResizeCol {
    Partition,
    Offset,
    Timestamp,
    Key,
    Actions,
}

/// 发送消息表单
struct SendForm {
    partition: Entity<SelectState<SearchableVec<StringOption>>>,
    key: Entity<InputState>,
    value: Entity<InputState>,
    headers: Vec<(Entity<InputState>, Entity<InputState>)>,
}

/// 发送历史条目
#[derive(Clone, Debug)]
struct SentItem {
    cluster: String,
    topic: String,
    partition: i32,
    key: Option<String>,
    value: String,
}

pub struct MessagesPage {
    window_handle: AnyWindowHandle,
    // 查询上下文（由导航器设置）
    cluster: Option<String>,
    topic: Option<String>,
    // 工具栏控件
    partition_state: Option<Entity<SelectState<SearchableVec<StringOption>>>>,
    partitions: Vec<i32>,
    mode_state: Entity<SelectState<SearchableVec<StringOption>>>,
    max_input: Entity<InputState>,
    search_in_state: Entity<SelectState<SearchableVec<StringOption>>>,
    search_input: Entity<InputState>,
    start_input: Entity<InputState>,
    end_input: Entity<InputState>,
    show_time_filters: bool,
    // 数据
    messages: Rc<Vec<Message>>,
    streaming: bool,
    received: usize,
    total: usize,
    elapsed_ms: Option<u128>,
    cancel_token: Option<CancellationToken>,
    error: Option<String>,
    // 表格
    col_widths: ColumnWidths,
    sort: Option<(SortCol, bool)>, // (列, asc?)
    resizing: Option<(ResizeCol, f32, f32)>, // (列, 起始 x, 起始宽)
    // 详情面板
    selected: Option<usize>,
    detail_format: DetailFormat,
    panel_height: f32,
    panel_resizing: Option<(f32, f32)>, // (起始 y, 起始高)
    is_favorite: bool,
    // 发送 / 历史
    send_form: Option<SendForm>,
    send_and_continue: bool,
    show_history: bool,
    history: Vec<SentItem>,
    history_search: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NavEvent> for MessagesPage {}

impl MessagesPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mode_state = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![
                    StringOption::new(t(cx, "messages.newest"), "newest"),
                    StringOption::new(t(cx, "messages.oldest"), "oldest"),
                ]),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let search_in_state = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![
                    StringOption::new(t(cx, "messages.searchInAll"), "all"),
                    StringOption::new(t(cx, "messages.searchInKey"), "key"),
                    StringOption::new(t(cx, "messages.searchInValue"), "value"),
                ]),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let max_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("100");
            state.set_value("100".to_string(), window, cx);
            state
        });
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "messages.searchPlaceholder"))
        });
        let start_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("YYYY-MM-DD HH:mm:ss")
        });
        let end_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("YYYY-MM-DD HH:mm:ss")
        });
        let history_search = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "sentMessageHistory.searchPlaceholder"))
        });

        let mut subscriptions = Vec::new();
        // 搜索框回车触发查询
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.start_query(cx);
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &history_search,
            |_this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        ));

        Self {
            window_handle: window.window_handle(),
            cluster: None,
            topic: None,
            partition_state: None,
            partitions: Vec::new(),
            mode_state,
            max_input,
            search_in_state,
            search_input,
            start_input,
            end_input,
            show_time_filters: false,
            messages: Rc::new(Vec::new()),
            streaming: false,
            received: 0,
            total: 0,
            elapsed_ms: None,
            cancel_token: None,
            error: None,
            col_widths: ColumnWidths::default(),
            sort: Some((SortCol::Timestamp, false)),
            resizing: None,
            selected: None,
            detail_format: DetailFormat::Json,
            panel_height: 380.0,
            panel_resizing: None,
            is_favorite: false,
            send_form: None,
            send_and_continue: false,
            show_history: false,
            history: Vec::new(),
            history_search,
            _subscriptions: subscriptions,
        }
    }

    /// 外部导航：预选集群 + Topic（由工作区调用）
    pub fn select_cluster_topic(&mut self, cluster: String, topic: String, cx: &mut Context<Self>) {
        self.cluster = Some(cluster.clone());
        self.topic = Some(topic.clone());
        self.messages = Rc::new(Vec::new());
        self.selected = None;
        self.error = None;
        cx.notify();
        self.load_partitions(cluster, cx);
        self.check_favorite(cx);
        self.start_query(cx);
    }

    /// 加载分区列表并创建分区下拉
    fn load_partitions(&self, cluster: String, cx: &mut Context<Self>) {
        let Some(topic) = self.topic.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic.get",
                json!({ "cluster_id": cluster, "name": topic }),
            )
            .await;
            let ids: Vec<i32> = result
                .ok()
                .and_then(|v| v.get("partitions").and_then(|p| p.as_array()).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|p| p.get("id")?.as_i64().map(|v| v as i32))
                .collect();

            this.update(cx, |this, cx| {
                this.partitions = ids.clone();
                let all_label = t(cx, "messages.allPartitions");
                let mut options = vec![StringOption::new(all_label, "")];
                options.extend(ids.iter().map(|id| {
                    let s = id.to_string();
                    StringOption::new(s.clone(), s)
                }));
                let handle = this.window_handle;
                let select = handle
                    .update(cx, |_, window, cx| {
                        cx.new(|cx| {
                            SelectState::new(
                                SearchableVec::new(options),
                                Some(IndexPath::new(0)),
                                window,
                                cx,
                            )
                        })
                    })
                    .ok();
                this.partition_state = select;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 检查收藏状态
    fn check_favorite(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(topic) = self.topic.clone() else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "favorite.check",
                json!({ "cluster_id": cluster, "topic_name": topic }),
            )
            .await;
            this.update(cx, |this, cx| {
                this.is_favorite = result
                    .ok()
                    .and_then(|v| v.get("is_favorite").and_then(|b| b.as_bool()))
                    .unwrap_or(false);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_favorite(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(topic) = self.topic.clone() else { return };
        let is_fav = self.is_favorite;
        self.is_favorite = !is_fav;
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
            if result.is_err() {
                this.update(cx, |this, cx| {
                    this.is_favorite = is_fav;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    /// 解析时间输入为毫秒时间戳
    fn parse_time_input(value: &str) -> Option<i64> {
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

    fn format_time_input(ts: i64) -> String {
        chrono::DateTime::from_timestamp_millis(ts)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default()
    }

    /// 时间范围预设（minutes 前到现在）
    fn apply_time_preset(&mut self, minutes: i64, cx: &mut Context<Self>) {
        let now = chrono::Local::now().timestamp_millis();
        let start = now - minutes * 60_000;
        let start_s = Self::format_time_input(start);
        let end_s = Self::format_time_input(now);
        if let Some(w) = cx.windows().first() {
            let _ = w.update(cx, |_, window, cx| {
                self.start_input.update(cx, |s, cx| {
                    s.set_value(start_s, window, cx);
                });
                self.end_input.update(cx, |s, cx| {
                    s.set_value(end_s, window, cx);
                });
            });
        }
    }

    fn clear_time_range(&mut self, cx: &mut Context<Self>) {
        if let Some(w) = cx.windows().first() {
            let _ = w.update(cx, |_, window, cx| {
                self.start_input.update(cx, |s, cx| {
                    s.set_value(String::new(), window, cx);
                });
                self.end_input.update(cx, |s, cx| {
                    s.set_value(String::new(), window, cx);
                });
            });
        }
    }

    /// 开始流式查询
    fn start_query(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(topic) = self.topic.clone() else { return };
        if self.streaming {
            return;
        }

        let fetch_mode = self
            .mode_state
            .read(cx)
            .selected_value()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "newest".to_string());
        let search_in = self
            .search_in_state
            .read(cx)
            .selected_value()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "all".to_string());
        let partition: Option<i32> = self
            .partition_state
            .as_ref()
            .and_then(|s| s.read(cx).selected_value().map(|v| v.to_string()))
            .and_then(|v| v.trim().parse().ok());
        let max_messages: i64 = self
            .max_input
            .read(cx)
            .value()
            .trim()
            .parse()
            .unwrap_or(100);
        let search = self.search_input.read(cx).value().to_string();
        let start_time = Self::parse_time_input(&self.start_input.read(cx).value());
        let end_time = Self::parse_time_input(&self.end_input.read(cx).value());

        let mut params = json!({
            "cluster_id": cluster,
            "topic": topic,
            "max_messages": max_messages,
            "fetchMode": fetch_mode,
            "search_in": search_in,
        });
        if let Some(p) = partition {
            params["partition"] = json!(p);
        }
        if !search.trim().is_empty() {
            params["search"] = json!(search.trim());
        }
        if let Some(st) = start_time {
            params["start_time"] = json!(st);
        }
        if let Some(et) = end_time {
            params["end_time"] = json!(et);
        }

        self.messages = Rc::new(Vec::new());
        self.received = 0;
        self.total = 0;
        self.selected = None;
        self.streaming = true;
        self.error = None;
        self.elapsed_ms = None;
        cx.notify();

        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());
        let start_instant = std::time::Instant::now();

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let token_for_stream = token.clone();
            let rx_result = rt
                .spawn(async move {
                    kafka_manager_api::api::start_message_list_stream(state, params, token_for_stream).await
                })
                .await;

            let mut rx = match rx_result {
                Ok(Ok(rx)) => rx,
                Ok(Err(e)) => {
                    this.update(cx, |this, cx| {
                        this.streaming = false;
                        this.error = Some(e.to_message());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.streaming = false;
                        this.error = Some(e.to_string());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };

            while let Some(evt) = rx.recv().await {
                let alive = this
                    .update(cx, |this, cx| {
                        this.handle_stream_event(&evt, cx);
                        cx.notify();
                    })
                    .is_ok();
                if !alive {
                    break;
                }
                if matches!(evt.event.as_str(), "complete" | "error") {
                    break;
                }
            }
            this.update(cx, |this, cx| {
                this.streaming = false;
                this.cancel_token = None;
                this.elapsed_ms = Some(start_instant.elapsed().as_millis());
                this.apply_sort(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn handle_stream_event(&mut self, evt: &kafka_manager_api::api::StreamEvent, _cx: &mut Context<Self>) {
        let parsed: serde_json::Value = match serde_json::from_str(&evt.data) {
            Ok(v) => v,
            Err(_) => return,
        };
        match evt.event.as_str() {
            "start" => {
                self.total = parsed
                    .get("total_target")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
            }
            "batch" => {
                let new_messages: Vec<Message> = parsed
                    .get("messages")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| {
                                Some(Message {
                                    partition: m.get("partition")?.as_i64()? as i32,
                                    offset: m.get("offset")?.as_i64()?,
                                    timestamp: m.get("timestamp")?.as_i64()?,
                                    key: m.get("key").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    value: m.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.received = parsed
                    .get("progress")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let mut messages = (*self.messages).clone();
                messages.extend(new_messages);
                self.messages = Rc::new(messages);
            }
            "complete" => {}
            "error" => {
                self.error = parsed
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
            _ => {}
        }
    }

    fn stop_query(&mut self, cx: &mut Context<Self>) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.streaming = false;
        cx.notify();
    }

    /// 切换列排序（单列：asc → desc → 无）
    fn toggle_sort(&mut self, col: SortCol, cx: &mut Context<Self>) {
        self.sort = match self.sort {
            Some((c, true)) if c == col => Some((col, false)),
            Some((c, false)) if c == col => None,
            _ => Some((col, true)),
        };
        self.apply_sort(cx);
    }

    fn apply_sort(&mut self, cx: &mut Context<Self>) {
        if let Some((col, asc)) = self.sort {
            let mut messages = (*self.messages).clone();
            match col {
                SortCol::Partition => messages.sort_by(|a, b| cmp_ord(a.partition, b.partition, asc)),
                SortCol::Offset => messages.sort_by(|a, b| cmp_ord(a.offset, b.offset, asc)),
                SortCol::Timestamp => messages.sort_by(|a, b| cmp_ord(a.timestamp, b.timestamp, asc)),
                SortCol::Key => messages.sort_by(|a, b| {
                    let la = a.key.as_deref().map(|k| k.len());
                    let lb = b.key.as_deref().map(|k| k.len());
                    cmp_ord(la, lb, asc).then_with(|| {
                        cmp_ord(
                            a.key.as_deref().unwrap_or(""),
                            b.key.as_deref().unwrap_or(""),
                            asc,
                        )
                    })
                }),
            }
            self.messages = Rc::new(messages);
        }
        cx.notify();
    }

    fn start_column_resize(&mut self, col: ResizeCol, x: f32, cx: &mut Context<Self>) {
        let w = self.col_widths;
        let start_w = match col {
            ResizeCol::Partition => w.partition,
            ResizeCol::Offset => w.offset,
            ResizeCol::Timestamp => w.timestamp,
            ResizeCol::Key => w.key,
            ResizeCol::Actions => w.actions,
        };
        self.resizing = Some((col, x, start_w));
        cx.notify();
    }

    fn on_resize_move(&mut self, x: f32, cx: &mut Context<Self>) {
        if let Some((col, start_x, start_w)) = self.resizing {
            let new_w = (start_w + (x - start_x)).max(30.0);
            match col {
                ResizeCol::Partition => self.col_widths.partition = new_w,
                ResizeCol::Offset => self.col_widths.offset = new_w,
                ResizeCol::Timestamp => self.col_widths.timestamp = new_w,
                ResizeCol::Key => self.col_widths.key = new_w,
                ResizeCol::Actions => self.col_widths.actions = new_w,
            }
            cx.notify();
        }
    }

    fn on_panel_resize_move(&mut self, y: f32, cx: &mut Context<Self>) {
        if let Some((start_y, start_h)) = self.panel_resizing {
            // 向上拖增大高度
            self.panel_height = (start_h + (start_y - y)).clamp(120.0, 800.0);
            cx.notify();
        }
    }

    fn copy_text(text: String, cx: &mut App) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        notify(cx, NotificationType::Success, t(cx, "messages.copied"));
    }

    fn format_timestamp(ts: i64) -> String {
        chrono::DateTime::from_timestamp_millis(ts)
            .map(|dt| dt.format("%m-%d %H:%M:%S%.3f").to_string())
            .unwrap_or_else(|| ts.to_string())
    }

    fn format_detail_value(&self, value: &str) -> String {
        match self.detail_format {
            DetailFormat::Json => serde_json::from_str::<serde_json::Value>(value)
                .ok()
                .and_then(|v| serde_json::to_string_pretty(&v).ok())
                .unwrap_or_else(|| value.to_string()),
            DetailFormat::Raw => value.to_string(),
            DetailFormat::Hex => value
                .as_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// 导出消息
    fn export_messages(&self, cx: &mut Context<Self>) {
        let Some(topic) = self.topic.clone() else { return };
        if self.messages.is_empty() {
            return;
        }
        let export_data: Vec<serde_json::Value> = self
            .messages
            .iter()
            .map(|m| {
                json!({
                    "partition": m.partition,
                    "offset": m.offset,
                    "key": m.key,
                    "value": m.value,
                    "timestamp": m.timestamp,
                })
            })
            .collect();
        let filename = format!("{}_messages_{}.json", topic, chrono::Local::now().timestamp());
        let success_msg = t(cx, "messages.exportSuccess");
        let failed_msg = t(cx, "messages.exportFailed");

        cx.spawn(async move |_this, cx| {
            let path = tokio::task::spawn_blocking(move || {
                rfd::FileDialog::new()
                    .set_file_name(&filename)
                    .add_filter("JSON", &["json"])
                    .save_file()
            })
            .await
            .ok()
            .flatten();
            let Some(path) = path else { return };
            let result = serde_json::to_string_pretty(&export_data)
                .map_err(|e| e.to_string())
                .and_then(|data| std::fs::write(&path, data).map_err(|e| e.to_string()));
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, success_msg),
                Err(e) => notify(cx, NotificationType::Error, format!("{}: {}", failed_msg, e)),
            })
            .ok();
        })
        .detach();
    }

    /// 删除 Topic（确认对话框）
    fn confirm_delete_topic(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(topic) = self.topic.clone() else { return };
        let entity = cx.entity();
        let title = t(cx, "messages.deleteTopic");
        let _ = self.window_handle.update(cx, |_, window, cx| {
            window.open_dialog(cx, move |dialog, _window, _cx| {
                let entity = entity.clone();
                let cluster = cluster.clone();
                let topic = topic.clone();
                dialog
                    .confirm()
                    .title(title.clone())
                    .child(topic.clone())
                    .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |_this, cx| {
                            let rt = TokioRuntime::handle(cx);
                            let Some(state) = Backend::state(cx) else { return };
                            let cluster = cluster.clone();
                            let topic = topic.clone();
                            let entity2 = cx.entity();
                            cx.spawn(async move |_this2, cx| {
                                let result = crate::service::call(
                                    &rt,
                                    state,
                                    "topic.delete",
                                    json!({ "cluster_id": cluster, "topic": topic }),
                                )
                                .await;
                                match result {
                                    Ok(_) => {
                                        cx.update(|cx| {
                                            notify(cx, NotificationType::Success, t(cx, "messages.topicDeleted"));
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
        });
    }

    /// 打开发送消息模态（选中消息时预填）
    fn open_send(&mut self, prefill: Option<Message>, cx: &mut Context<Self>) {
        let topic = self.topic.clone().unwrap_or_default();

        let partition_options: Vec<StringOption> = self
            .partitions
            .iter()
            .map(|id| {
                let s = id.to_string();
                StringOption::new(s.clone(), s)
            })
            .collect();
        if partition_options.is_empty() {
            return;
        }
        let prefill_partition = prefill
            .as_ref()
            .and_then(|m| self.partitions.iter().position(|p| *p == m.partition));
        let handle = self.window_handle;
        let key_value = prefill.as_ref().and_then(|m| m.key.clone());
        let value_value = prefill.as_ref().and_then(|m| m.value.clone());

        let created = handle.update(cx, |_, window, cx| {
            let partition_state = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(partition_options),
                    Some(IndexPath::new(prefill_partition.unwrap_or(0))),
                    window,
                    cx,
                )
            });
            let key_state = cx.new(|cx| {
                let mut state = InputState::new(window, cx).placeholder(t(cx, "messages.keyOptional"));
                if let Some(k) = key_value {
                    state.set_value(k, window, cx);
                }
                state
            });
            let value_state = cx.new(|cx| {
                let mut state = InputState::new(window, cx)
                    .multi_line(true)
                    .auto_grow(5, 12)
                    .placeholder(t(cx, "messages.valuePlaceholder"));
                if let Some(v) = value_value {
                    state.set_value(v, window, cx);
                }
                state
            });
            (partition_state, key_state, value_state)
        });

        let Ok((partition_state, key_state, value_state)) = created else {
            return;
        };

        self.send_form = Some(SendForm {
            partition: partition_state.clone(),
            key: key_state.clone(),
            value: value_state.clone(),
            headers: Vec::new(),
        });

        let entity = cx.entity();
        let title = format!("{} — {}", t(cx, "messages.sendMessage"), topic);
        let partition_label = t(cx, "messages.partition");
        let key_label = t(cx, "messages.key");
        let value_label = t(cx, "messages.value");
        let send_label = t(cx, "messages.send");
        let send_continue_label = t(cx, "messages.sendAndNew");

        let _ = handle.update(cx, |_, window, cx| {
            window.open_dialog(cx, move |dialog, _window, _cx| {
                let entity = entity.clone();
                let entity_continue = entity.clone();
                let send_label_f = send_label.clone();
                let send_continue_label_f = send_continue_label.clone();
                dialog
                    .title(title.clone())
                    .w(px(640.0))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(
                                        div().w_32().child(field_row(
                                            &partition_label,
                                            Select::new(&partition_state).into_any_element(),
                                        )),
                                    )
                                    .child(
                                        div().flex_1().child(field_row(
                                            &key_label,
                                            Input::new(&key_state).into_any_element(),
                                        )),
                                    ),
                            )
                            .child(field_row(&value_label, Input::new(&value_state).into_any_element())),
                    )
                    .footer(move |_ok, _cancel, _window, cx| {
                        let entity = entity.clone();
                        let entity_continue = entity_continue.clone();
                        let send_label = send_label_f.clone();
                        let send_continue_label = send_continue_label_f.clone();
                        vec![
                            Button::new("send-cancel")
                                .ghost()
                                .label(t(cx, "common.cancel"))
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                                .into_any_element(),
                            Button::new("send-continue")
                                .outline()
                                .label(send_continue_label.clone())
                                .on_click(move |_, _window, cx| {
                                    entity_continue.update(cx, |this, cx| {
                                        this.send_and_continue = true;
                                        this.submit_send(cx);
                                    });
                                })
                                .into_any_element(),
                            Button::new("send-ok")
                                .primary()
                                .label(send_label.clone())
                                .on_click(move |_, window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.send_and_continue = false;
                                        this.submit_send(cx);
                                    });
                                    window.close_dialog(cx);
                                })
                                .into_any_element(),
                        ]
                    })
                    .overlay_closable(false)
            });
        });
    }

    fn submit_send(&mut self, cx: &mut Context<Self>) {
        let keep_open = self.send_and_continue;
        let form_owned = if keep_open { None } else { self.send_form.take() };
        let form = if keep_open {
            self.send_form.as_ref()
        } else {
            form_owned.as_ref()
        };
        let Some(form) = form else { return };
        let Some(cluster) = self.cluster.clone() else { return };
        let Some(topic) = self.topic.clone() else { return };

        let partition: Option<i32> = form
            .partition
            .read(cx)
            .selected_value()
            .and_then(|v| v.parse().ok());
        let key = form.key.read(cx).value().to_string();
        let value = form.value.read(cx).value().to_string();

        if value.is_empty() {
            let msg = t(cx, "messages.valueRequired");
            notify(cx, NotificationType::Error, msg);
            return;
        }

        let headers: serde_json::Map<String, serde_json::Value> = form
            .headers
            .iter()
            .filter_map(|(k, v)| {
                let key = k.read(cx).value().to_string();
                if key.trim().is_empty() {
                    None
                } else {
                    Some((key, json!(v.read(cx).value().to_string())))
                }
            })
            .collect();

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |_this, cx| {
            let mut params = json!({
                "cluster_id": cluster,
                "topic": topic,
                "value": value,
            });
            if !key.is_empty() {
                params["key"] = json!(key);
            }
            if let Some(p) = partition {
                params["partition"] = json!(p);
            }
            if !headers.is_empty() {
                params["headers"] = serde_json::Value::Object(headers);
            }
            let result = crate::service::call(&rt, state, "message.send", params).await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "messages.messageSent")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
        })
        .detach();
    }

    /// 打开历史覆盖面板
    fn open_history(&mut self, cx: &mut Context<Self>) {
        self.show_history = true;
        cx.notify();
        self.load_history(cx);
    }

    fn load_history(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let cluster = self.cluster.clone();
        let topic = self.topic.clone();

        cx.spawn(async move |this, cx| {
            let mut params = json!({ "limit": 50 });
            if let Some(c) = &cluster {
                params["cluster_id"] = json!(c);
            }
            if let Some(tp) = &topic {
                params["topic_name"] = json!(tp);
            }
            let result = crate::service::call(&rt, state, "sent_message.list", params).await;
            this.update(cx, |this, cx| {
                this.history = result
                    .ok()
                    .and_then(|v| v.get("messages").and_then(|m| m.as_array()).cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|m| {
                        Some(SentItem {
                            cluster: m.get("cluster_id")?.as_str()?.to_string(),
                            topic: m.get("topic_name")?.as_str()?.to_string(),
                            partition: m.get("partition")?.as_i64()? as i32,
                            key: m.get("message_key").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            value: m.get("message_value")?.as_str()?.to_string(),
                        })
                    })
                    .collect();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn clear_history(&mut self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "sent_message.clear", json!({})).await;
            if let Err(e) = result {
                cx.update(|cx| notify(cx, NotificationType::Error, e)).ok();
            }
            this.update(cx, |this, cx| {
                this.history.clear();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 双击历史条目：预填发送表单
    fn resend_from_history(&mut self, item: SentItem, cx: &mut Context<Self>) {
        self.show_history = false;
        cx.notify();
        self.open_send(
            Some(Message {
                partition: item.partition,
                offset: 0,
                timestamp: 0,
                key: item.key,
                value: Some(item.value),
            }),
            cx,
        );
    }
}

fn cmp_ord<T: Ord>(a: T, b: T, asc: bool) -> std::cmp::Ordering {
    if asc { a.cmp(&b) } else { b.cmp(&a) }
}

/// 表单字段行：标签 + 控件
fn field_row(label: &str, control: AnyElement) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_sm().child(label.to_string()))
        .child(control)
}

/// 时间预设按钮
fn preset_button(
    id: &'static str,
    minutes: i64,
    label_key: &'static str,
    cx: &mut Context<MessagesPage>,
) -> Button {
    Button::new(id)
        .ghost()
        .xsmall()
        .label(t(cx, label_key))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.apply_time_preset(minutes, cx);
        }))
}

/// 详情格式切换按钮
fn format_button(
    id: &'static str,
    format: DetailFormat,
    current: DetailFormat,
    cx: &mut Context<MessagesPage>,
) -> Button {
    let active = format == current;
    Button::new(id)
        .ghost()
        .xsmall()
        .label(id)
        .when(active, |b| b.outline())
        .on_click(cx.listener(move |this, _, _, cx| {
            this.detail_format = format;
            cx.notify();
        }))
}

impl Render for MessagesPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (border_c, secondary_c, muted_c, primary_c, info_c, success_c, danger_c, table_head_c, table_active_c, list_hover_c, bg_c) = {
            let theme = cx.theme();
            (
                theme.border,
                theme.secondary,
                theme.muted_foreground,
                theme.primary,
                theme.info,
                theme.success,
                theme.danger,
                theme.table_head,
                theme.table_active,
                theme.list_hover,
                theme.background,
            )
        };
        let has_topic = self.cluster.is_some() && self.topic.is_some();
        let can_query = has_topic && !self.streaming;

        // ========== 工具栏 ==========
        let toolbar = h_flex()
            .items_center()
            .gap_1p5()
            .p_1p5()
            .flex_wrap()
            .border_b_1()
            .border_color(border_c)
            .children(
                self.partition_state
                    .as_ref()
                    .map(|s| div().w(px(112.0)).child(Select::new(s).small()).into_any_element()),
            )
            .child(div().w_24().child(Select::new(&self.mode_state).small()))
            .child(div().w_16().child(Input::new(&self.max_input).small()))
            .child(
                Button::new("time-filter")
                    .ghost()
                    .icon(IconName::Calendar)
                    .xsmall()
                    .when(self.show_time_filters, |b| b.outline())
                    .tooltip(t(cx, "messages.timeRange"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_time_filters = !this.show_time_filters;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(160.0))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(div().w_24().child(Select::new(&self.search_in_state).small()))
                            .child(div().flex_1().child(Input::new(&self.search_input).small())),
                    ),
            )
            .child(
                Button::new("query")
                    .primary()
                    .icon(IconName::Search)
                    .small()
                    .loading(self.streaming)
                    .disabled(!can_query)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.start_query(cx);
                    })),
            )
            .when(self.streaming, |el| {
                el.child(
                    Button::new("stop")
                        .danger()
                        .icon(IconName::Close)
                        .small()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.stop_query(cx);
                        })),
                )
            })
            .child(
                Button::new("send")
                    .ghost()
                    .icon(IconName::ArrowRight)
                    .small()
                    .disabled(!has_topic)
                    .tooltip(t(cx, "messages.sendMessage"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let prefill = this.selected.and_then(|ix| this.messages.get(ix).cloned());
                        this.open_send(prefill, cx);
                    })),
            );

        // 时间范围子面板
        let time_panel: AnyElement = if self.show_time_filters {
            h_flex()
                .items_center()
                .gap_2()
                .p_1p5()
                .flex_wrap()
                .bg(secondary_c)
                .border_b_1()
                .border_color(border_c)
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_c)
                        .child(t(cx, "messages.startTime")),
                )
                .child(div().w_48().child(Input::new(&self.start_input).small()))
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_c)
                        .child(t(cx, "messages.endTime")),
                )
                .child(div().w_48().child(Input::new(&self.end_input).small()))
                .child(preset_button("5m", 5, "messages.recent5Minutes", cx))
                .child(preset_button("15m", 15, "messages.recent15Minutes", cx))
                .child(preset_button("30m", 30, "messages.recent30Minutes", cx))
                .child(preset_button("1h", 60, "messages.recent1Hour", cx))
                .child(preset_button("1d", 1440, "messages.recent1Day", cx))
                .child(
                    Button::new("clear-time")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Close)
                        .label(t(cx, "messages.clear"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.clear_time_range(cx);
                        })),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        // ========== 状态栏 ==========
        let pct = if self.total > 0 {
            (self.received as f32 / self.total as f32 * 100.0).min(100.0)
        } else {
            0.0
        };
        let status_bar = h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .text_xs()
            .border_b_1()
            .border_color(border_c)
            .bg(secondary_c)
            .child(
                div()
                    .font_semibold()
                    .text_color(primary_c)
                    .child(self.topic.clone().unwrap_or_default()),
            )
            .child(
                Button::new("fav")
                    .ghost()
                    .xsmall()
                    .icon(if self.is_favorite { IconName::Star } else { IconName::StarOff })
                    .disabled(!has_topic)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_favorite(cx);
                    })),
            )
            .child(
                div().text_color(info_c).child(if self.streaming {
                    format!(
                        "{}{}",
                        t(cx, "messages.receiving"),
                        if self.total > 0 {
                            format!(" {}/{}", self.received, self.total)
                        } else {
                            format!(" {}", self.received)
                        }
                    )
                } else if let Some(ms) = self.elapsed_ms {
                    format!("Elapsed: {}ms", ms)
                } else {
                    String::new()
                }),
            )
            .child(
                div()
                    .text_color(success_c)
                    .child(format!("{} {}", t(cx, "messages.totalMessages"), self.messages.len())),
            )
            .children(
                self.error
                    .as_ref()
                    .map(|e| div().text_color(danger_c).child(e.clone()).into_any_element()),
            )
            .child(div().flex_1())
            .child(
                Button::new("export")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ArrowDown)
                    .disabled(self.messages.is_empty())
                    .tooltip(t(cx, "messages.exportMessages"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.export_messages(cx);
                    })),
            )
            .child(
                Button::new("history")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Calendar)
                    .tooltip(t(cx, "sentMessageHistory.title"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_history(cx);
                    })),
            )
            .child(
                Button::new("topic-cgs")
                    .ghost()
                    .xsmall()
                    .icon(IconName::CircleUser)
                    .disabled(!has_topic)
                    .tooltip(t(cx, "topicConsumerGroups.title"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let (Some(cluster), Some(topic)) = (this.cluster.clone(), this.topic.clone()) {
                            cx.emit(NavEvent::OpenTopicConsumerGroups { cluster, topic });
                        }
                    })),
            )
            .child(
                Button::new("delete-topic")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Delete)
                    .disabled(!has_topic)
                    .tooltip(t(cx, "messages.deleteTopic"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirm_delete_topic(cx);
                    })),
            );

        let progress_bar: AnyElement = if self.streaming && self.total > 0 {
            div()
                .px_3()
                .py_1()
                .border_b_1()
                .border_color(border_c)
                .child(gpui_component::progress::Progress::new().value(pct))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        // ========== 表格 ==========
        let w = self.col_widths;
        let sort_header = |col: SortCol, label: String, width: f32, resize: ResizeCol| {
            let (icon, active) = match self.sort {
                Some((c, asc)) if c == col => (
                    if asc { IconName::SortAscending } else { IconName::SortDescending },
                    true,
                ),
                _ => (IconName::ChevronsUpDown, false),
            };
            h_flex()
                .items_center()
                .child(
                    h_flex()
                        .id(SharedString::from(format!("sort-{:?}", col)))
                        .items_center()
                        .gap_1()
                        .cursor_pointer()
                        .w(px(width))
                        .overflow_hidden()
                        .child(label)
                        .child(
                            Icon::new(icon)
                                .size_3()
                                .when(!active, |i| i.text_color(muted_c)),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_sort(col, cx);
                        })),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("resize-{:?}", resize)))
                        .w_1()
                        .h_full()
                        .cursor_col_resize()
                        .hover(|el| el.bg(primary_c))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                this.start_column_resize(resize, event.position.x.as_f32(), cx);
                            }),
                        ),
                )
        };

        let table_header = h_flex()
            .px_2()
            .py_0p5()
            .bg(table_head_c)
            .text_xs()
            .font_semibold()
            .child(sort_header(SortCol::Partition, t(cx, "messages.partitionLabel2"), w.partition, ResizeCol::Partition))
            .child(sort_header(SortCol::Offset, t(cx, "messages.offsetLabel"), w.offset, ResizeCol::Offset))
            .child(sort_header(SortCol::Timestamp, t(cx, "messages.timestampLabel"), w.timestamp, ResizeCol::Timestamp))
            .child(sort_header(SortCol::Key, t(cx, "messages.key"), w.key, ResizeCol::Key))
            .child(div().flex_1().child(t(cx, "messages.value")))
            .child(
                h_flex()
                    .items_center()
                    .child(div().w(px(w.actions)).child(t(cx, "messages.actions")))
                    .child(
                        div()
                            .id("resize-actions")
                            .w_1()
                            .h_full()
                            .cursor_col_resize()
                            .hover(|el| el.bg(primary_c))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    this.start_column_resize(ResizeCol::Actions, event.position.x.as_f32(), cx);
                                }),
                            ),
                    ),
            );

        let messages = self.messages.clone();
        let selected = self.selected;
        let entity = cx.entity();
        let table_body: AnyElement = if !has_topic {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(muted_c)
                .child(t(cx, "messages.noTopicSelected"))
                .into_any_element()
        } else if messages.is_empty() && !self.streaming {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(muted_c)
                .child(t(cx, "messages.noMessages"))
                .into_any_element()
        } else {
            let border = border_c;
            let active = table_active_c;
            let secondary = secondary_c;
            uniform_list("message-list", messages.len(), move |range, _window, _cx| {
                range
                    .map(|ix| {
                        let m = messages[ix].clone();
                        let is_selected = selected == Some(ix);
                        let entity = entity.clone();
                        let value_for_copy = m.value.clone().unwrap_or_default();
                        h_flex()
                            .px_2()
                            .py_0p5()
                            .w_full()
                            .h_6()
                            .items_center()
                            .border_b_1()
                            .border_color(border)
                            .when(is_selected, |el| el.bg(active))
                            .child(
                                div()
                                    .w(px(w.partition))
                                    .child(
                                        div()
                                            .text_xs()
                                            .px_1()
                                            .rounded_md()
                                            .bg(secondary)
                                            .child(m.partition.to_string()),
                                    ),
                            )
                            .child(div().w(px(w.offset)).text_xs().child(m.offset.to_string()))
                            .child(
                                div()
                                    .w(px(w.timestamp))
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .child(MessagesPage::format_timestamp(m.timestamp)),
                            )
                            .child(
                                div()
                                    .w(px(w.key))
                                    .text_xs()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(m.key.clone().unwrap_or_else(|| "-".to_string())),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(m.value.clone().unwrap_or_default()),
                            )
                            .child(
                                div().w(px(w.actions)).child(
                                    Button::new(("copy", ix))
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Copy)
                                        .on_click(move |_, _, cx| {
                                            MessagesPage::copy_text(value_for_copy.clone(), cx);
                                        }),
                                ),
                            )
                            .id(ix)
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.selected = Some(ix);
                                    cx.notify();
                                });
                            })
                            .into_any_element()
                    })
                    .collect()
            })
            .into_any_element()
        };

        // ========== 详情面板 ==========
        let detail: AnyElement = match self.selected.and_then(|ix| self.messages.get(ix)) {
            Some(m) => {
                let key_text = m.key.clone();
                let value_formatted = self.format_detail_value(&m.value.clone().unwrap_or_default());
                let value_for_copy = value_formatted.clone();
                let ts = m.timestamp;
                let partition = m.partition;
                let offset = m.offset;

                v_flex()
                    .h(px(self.panel_height))
                    .border_t_1()
                    .border_color(border_c)
                    .child(
                        // 拖拽调高把手
                        div()
                            .id("panel-resizer")
                            .h_1()
                            .w_full()
                            .cursor_row_resize()
                            .hover(|el| el.bg(primary_c))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                    this.panel_resizing =
                                        Some((event.position.y.as_f32(), this.panel_height));
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .px_2()
                            .py_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .child(t(cx, "messages.messageDetail")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_c)
                                    .child(format!(
                                        "{}: {}  {}: {}  {}",
                                        t(cx, "messages.partitionLabel2"),
                                        partition,
                                        t(cx, "messages.offsetLabel"),
                                        offset,
                                        MessagesPage::format_timestamp(ts)
                                    )),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("close-detail")
                                    .ghost()
                                    .xsmall()
                                    .label(t(cx, "messages.close"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.selected = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .children(key_text.map(|k| {
                        let k_for_copy = k.clone();
                        h_flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_c)
                                    .child(format!("{}:", t(cx, "messages.key"))),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(k.clone()),
                            )
                            .child(
                                Button::new("copy-key")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Copy)
                                    .on_click(move |_, _, cx| {
                                        MessagesPage::copy_text(k_for_copy.clone(), cx);
                                    }),
                            )
                            .into_any_element()
                    }))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_c)
                                    .child(format!("{}:", t(cx, "messages.value"))),
                            )
                            .child(
                                h_flex()
                                    .gap_0p5()
                                    .child(format_button("json", DetailFormat::Json, self.detail_format, cx))
                                    .child(format_button("raw", DetailFormat::Raw, self.detail_format, cx))
                                    .child(format_button("hex", DetailFormat::Hex, self.detail_format, cx)),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("copy-value")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Copy)
                                    .tooltip(t(cx, "messages.copyValue"))
                                    .on_click(move |_, _, cx| {
                                        MessagesPage::copy_text(value_for_copy.clone(), cx);
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id("detail-value-scroll")
                            .flex_1()
                            .mx_2()
                            .mb_2()
                            .p_2()
                            .bg(bg_c)
                            .border_1()
                            .border_color(border_c)
                            .rounded_md()
                            .overflow_y_scroll()
                            .child(div().text_xs().child(value_formatted)),
                    )
                    .into_any_element()
            }
            None => div().into_any_element(),
        };

        // ========== 历史覆盖面板 ==========
        let history_overlay: AnyElement = if self.show_history {
            let query = self.history_search.read(cx).value().to_lowercase();
            let visible: Vec<&SentItem> = self
                .history
                .iter()
                .filter(|item| {
                    query.is_empty()
                        || item.topic.to_lowercase().contains(&query)
                        || item.value.to_lowercase().contains(&query)
                        || item
                            .key
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&query)
                })
                .collect();

            let rows: Vec<AnyElement> = visible
                .iter()
                .enumerate()
                .map(|(ix, item)| {
                    let item = (*item).clone();
                    v_flex()
                        .gap_0p5()
                        .px_2()
                        .py_1p5()
                        .border_b_1()
                        .border_color(border_c)
                        .cursor_pointer()
                        .hover(|el| el.bg(list_hover_c))
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted_c)
                                .child(format!(
                                    "{} / {} / P{}",
                                    item.cluster, item.topic, item.partition
                                )),
                        )
                        .children(item.key.as_ref().map(|k| {
                            div().text_xs().child(format!("key: {}", k)).into_any_element()
                        }))
                        .child(
                            div()
                                .text_xs()
                                .overflow_hidden()
                                .child(item.value.clone()),
                        )
                        .id(("history-item", ix))
                        .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                            if event.click_count() == 2 {
                                this.resend_from_history(item.clone(), cx);
                            }
                        }))
                        .into_any_element()
                })
                .collect();

            div()
                .absolute()
                .inset_0()
                .bg(bg_c)
                .child(
                    v_flex()
                        .size_full()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .p_2()
                                .border_b_1()
                                .border_color(border_c)
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .child(t(cx, "sentMessageHistory.title")),
                                )
                                .child(div().flex_1().child(Input::new(&self.history_search).small()))
                                .child(
                                    Button::new("history-refresh")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Redo2)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.load_history(cx);
                                        })),
                                )
                                .child(
                                    Button::new("history-clear")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Delete)
                                        .tooltip(t(cx, "sentMessageHistory.clearAll"))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.clear_history(cx);
                                        })),
                                )
                                .child(
                                    Button::new("history-close")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Close)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.show_history = false;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .id("history-scroll")
                                .flex_1()
                                .overflow_y_scroll()
                                .when(rows.is_empty(), |el| {
                                    el.child(
                                        div()
                                            .p_4()
                                            .text_center()
                                            .text_color(muted_c)
                                            .text_sm()
                                            .child(t(cx, "sentMessageHistory.empty")),
                                    )
                                })
                                .child(v_flex().children(rows)),
                        ),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        // ========== 列宽/面板拖拽全局覆盖层 ==========
        let resize_overlay: AnyElement = if self.resizing.is_some() || self.panel_resizing.is_some() {
            div()
                .absolute()
                .inset_0()
                .id("resize-overlay")
                .cursor_col_resize()
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                    if this.resizing.is_some() {
                        this.on_resize_move(event.position.x.as_f32(), cx);
                    }
                    if this.panel_resizing.is_some() {
                        this.on_panel_resize_move(event.position.y.as_f32(), cx);
                    }
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.resizing = None;
                        this.panel_resizing = None;
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
                    .child(toolbar)
                    .child(time_panel)
                    .child(status_bar)
                    .child(progress_bar)
                    .child(table_header)
                    .child(div().flex_1().overflow_hidden().child(table_body))
                    .child(detail),
            )
            .child(history_overlay)
            .child(resize_overlay)
    }
}
