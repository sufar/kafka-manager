//! 消息查询页：流式查询、虚拟表格、详情面板、发送消息、导出

use std::rc::Rc;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::*;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::pages::clusters::notify;
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

/// 发送消息表单
struct SendForm {
    key: Entity<InputState>,
    value: Entity<InputState>,
    partition: Entity<InputState>,
}

/// 当前查询参数快照（用于导出）
#[derive(Clone)]
struct QueryParams {
    cluster: String,
    topic: String,
}

pub struct MessagesPage {
    window_handle: AnyWindowHandle,
    cluster_state: Option<Entity<SelectState<SearchableVec<StringOption>>>>,
    topic_state: Option<Entity<SelectState<SearchableVec<StringOption>>>>,
    mode_state: Entity<SelectState<SearchableVec<StringOption>>>,
    search_in_state: Entity<SelectState<SearchableVec<StringOption>>>,
    partition_input: Entity<InputState>,
    max_input: Entity<InputState>,
    search_input: Entity<InputState>,

    cluster_options: Vec<String>,
    topic_options: Vec<String>,
    pending_topic: Option<String>,
    messages: Rc<Vec<Message>>,
    streaming: bool,
    received: usize,
    total: usize,
    cancel_token: Option<CancellationToken>,
    selected: Option<usize>,
    sort_desc: bool,
    error: Option<String>,
    last_query: Option<QueryParams>,
    send_form: Option<SendForm>,
    _subscriptions: Vec<Subscription>,
}

impl MessagesPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 查询模式：newest / oldest
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
        // 搜索范围：all / key / value
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
        let partition_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "messages.allPartitions"))
        });
        let max_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("100");
            state.set_value("100".to_string(), window, cx);
            state
        });
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "messages.searchPlaceholder"))
        });

        let this = Self {
            window_handle: window.window_handle(),
            cluster_state: None,
            topic_state: None,
            cluster_options: Vec::new(),
            topic_options: Vec::new(),
            pending_topic: None,
            mode_state,
            search_in_state,
            partition_input,
            max_input,
            search_input,
            messages: Rc::new(Vec::new()),
            streaming: false,
            received: 0,
            total: 0,
            cancel_token: None,
            selected: None,
            sort_desc: true,
            error: None,
            last_query: None,
            send_form: None,
            _subscriptions: Vec::new(),
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

    fn selected_topic(&self, cx: &App) -> Option<String> {
        self.topic_state
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
            let cluster_arr = result
                .ok()
                .and_then(|value| value.as_array().cloned())
                .unwrap_or_default();
            let options: Vec<StringOption> = cluster_arr
                .iter()
                .filter_map(|c| {
                    let name = c.get("name")?.as_str()?.to_string();
                    Some(StringOption::new(name.clone(), name))
                })
                .collect();

            this.update_in(cx, |this, window, cx| {
                let has_options = !options.is_empty();
                this.cluster_options = options.iter().map(|o| o.value.to_string()).collect();
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
                            this.load_topics_for_cluster(cx);
                        }
                    },
                );
                this._subscriptions.push(sub);
                this.cluster_state = Some(select);
                cx.notify();
                this.load_topics_for_cluster(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 外部导航：预选集群 + Topic（由工作区在导航器/全局搜索选中时调用）
    pub fn select_cluster_topic(&mut self, cluster: String, topic: String, cx: &mut Context<Self>) {
        if let Some(ix) = self.cluster_options.iter().position(|c| c == &cluster) {
            if let Some(state) = &self.cluster_state {
                let handle = self.window_handle;
                let state = state.clone();
                handle
                    .update(cx, |_, window, cx| {
                        state.update(cx, |s, cx| {
                            s.set_selected_index(Some(IndexPath::new(ix)), window, cx);
                        });
                    })
                    .ok();
            }
        }
        self.pending_topic = Some(topic);
        self.load_topics_for_cluster(cx);
    }

    /// 集群变化后加载 Topic 列表并创建 Topic 选择器
    fn load_topics_for_cluster(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic.list_with_cluster",
                json!({ "cluster_id": cluster, "limit": 100000 }),
            )
            .await;
            let topic_arr = result
                .ok()
                .and_then(|value| value.get("topics").and_then(|v| v.as_array()).cloned())
                .unwrap_or_default();
            let options: Vec<StringOption> = topic_arr
                .iter()
                .filter_map(|t| {
                    let name = t.get("name")?.as_str()?.to_string();
                    Some(StringOption::new(name.clone(), name))
                })
                .collect();

            this.update(cx, |this, cx| {
                let has_options = !options.is_empty();
                this.topic_options = options.iter().map(|o| o.value.to_string()).collect();
                let window_handle = this.window_handle;
                let select = window_handle
                    .update(cx, |_, window, cx| {
                        cx.new(|cx| {
                            SelectState::new(
                                SearchableVec::new(options),
                                if has_options { Some(IndexPath::new(0)) } else { None },
                                window,
                                cx,
                            )
                        })
                    })
                    .ok();
                this.topic_state = select;

                // 处理外部预选：加载完成后定位 Topic 并自动查询
                if let Some(pending) = this.pending_topic.take() {
                    if let Some(ix) = this.topic_options.iter().position(|t| t == &pending) {
                        if let Some(state) = &this.topic_state {
                            let handle = this.window_handle;
                            let state = state.clone();
                            handle
                                .update(cx, |_, window, cx| {
                                    state.update(cx, |s, cx| {
                                        s.set_selected_index(Some(IndexPath::new(ix)), window, cx);
                                    });
                                })
                                .ok();
                        }
                        cx.notify();
                        this.start_query(cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 开始流式查询
    fn start_query(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.selected_cluster(cx) else { return };
        let Some(topic) = self.selected_topic(cx) else { return };
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
            .partition_input
            .read(cx)
            .value()
            .trim()
            .parse()
            .ok();
        let max_messages: i64 = self
            .max_input
            .read(cx)
            .value()
            .trim()
            .parse()
            .unwrap_or(100);
        let search = self.search_input.read(cx).value().to_string();

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

        self.messages = Rc::new(Vec::new());
        self.received = 0;
        self.total = 0;
        self.selected = None;
        self.streaming = true;
        self.error = None;
        self.last_query = Some(QueryParams { cluster, topic });
        cx.notify();

        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            // 在 tokio runtime 上启动流式查询
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

            // 消费事件流
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
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 处理单个流式事件
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
            "order" => {
                if let Some(sort) = parsed.get("sort").and_then(|v| v.as_str()) {
                    self.sort_desc = sort == "desc";
                }
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

    /// 停止查询
    fn stop_query(&mut self, cx: &mut Context<Self>) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.streaming = false;
        cx.notify();
    }

    /// 按时间戳排序切换
    fn toggle_sort(&mut self, cx: &mut Context<Self>) {
        self.sort_desc = !self.sort_desc;
        let mut messages = (*self.messages).clone();
        if self.sort_desc {
            messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        } else {
            messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }
        self.messages = Rc::new(messages);
        cx.notify();
    }

    /// 复制文本到剪贴板
    fn copy_text(text: String, cx: &mut App) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    /// 打开发送消息对话框
    fn open_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "messages.keyOptional"))
        });
        let value_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .auto_grow(3, 10)
                .placeholder(t(cx, "messages.valuePlaceholder"))
        });
        let partition_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "messages.optional"))
        });

        self.send_form = Some(SendForm {
            key: key_state.clone(),
            value: value_state.clone(),
            partition: partition_state.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "messages.sendMessage");
        let key_label = t(cx, "messages.key");
        let value_label = t(cx, "messages.value");
        let partition_label = t(cx, "messages.partition");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title(title.clone())
                .w(px(560.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(field_row(&key_label, Input::new(&key_state).into_any_element()))
                        .child(field_row(&value_label, Input::new(&value_state).into_any_element()))
                        .child(field_row(
                            &partition_label,
                            Input::new(&partition_state).into_any_element(),
                        )),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, window, cx| {
                    entity.update(cx, |this, cx| this.submit_send(window, cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn submit_send(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.send_form.take() else { return };
        let Some(query) = self.last_query.clone() else { return };
        let key = form.key.read(cx).value().to_string();
        let value = form.value.read(cx).value().to_string();
        let partition: Option<i32> = form.partition.read(cx).value().trim().parse().ok();

        if value.is_empty() {
            let msg = t(cx, "messages.valueRequired");
            notify(cx, NotificationType::Error, msg);
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |_this, cx| {
            let mut params = json!({
                "cluster_id": query.cluster,
                "topic": query.topic,
                "value": value,
            });
            if !key.is_empty() {
                params["key"] = json!(key);
            }
            if let Some(p) = partition {
                params["partition"] = json!(p);
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

    /// 导出当前消息为 JSON 文件（原生保存对话框）
    fn export_messages(&self, cx: &mut Context<Self>) {
        let Some(query) = self.last_query.clone() else { return };
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
        let filename = format!("{}_messages.json", query.topic);
        let success_msg = t(cx, "messages.exportSuccess");
        let failed_msg = t(cx, "messages.exportFailed");

        cx.spawn(async move |_this, cx| {
            // rfd 对话框在独立线程中运行（避免阻塞 UI）
            let path = tokio::task::spawn_blocking(move || {
                rfd::FileDialog::new()
                    .set_file_name(&filename)
                    .add_filter("JSON", &["json"])
                    .save_file()
            })
            .await
            .ok()
            .flatten();

            let result = match path {
                Some(path) => serde_json::to_string_pretty(&export_data)
                    .map_err(|e| e.to_string())
                    .and_then(|data| std::fs::write(&path, data).map_err(|e| e.to_string())),
                None => return, // 用户取消
            };
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, success_msg),
                Err(e) => notify(cx, NotificationType::Error, format!("{}: {}", failed_msg, e)),
            })
            .ok();
        })
        .detach();
    }

    /// 打开发送历史对话框
    fn open_history(&mut self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let query = self.last_query.clone();

        cx.spawn(async move |_this, cx| {
            let mut params = json!({ "limit": 50 });
            if let Some(q) = &query {
                params["cluster_id"] = json!(q.cluster);
                params["topic_name"] = json!(q.topic);
            }
            let result = crate::service::call(&rt, state, "sent_message.list", params).await;
            cx.update(|cx| match result {
                Ok(value) => {
                    let items: Vec<(String, String, String, String)> = value
                        .get("messages")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|m| {
                            Some((
                                m.get("cluster_id")?.as_str()?.to_string(),
                                m.get("topic_name")?.as_str()?.to_string(),
                                m.get("message_key")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                m.get("message_value")?.as_str()?.to_string(),
                            ))
                        })
                        .collect();
                    let empty_text = t(cx, "sentMessageHistory.empty");
                    let title = t(cx, "sentMessageHistory.title");
                    for w in cx.windows() {
                        let items = items.clone();
                        let title = title.clone();
                        let empty_text = empty_text.clone();
                        let _ = w.update(cx, |_, window, cx| {
                            let border = cx.theme().border;
                            let muted = cx.theme().muted_foreground;
                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                let rows: Vec<AnyElement> = if items.is_empty() {
                                    vec![div()
                                        .py_4()
                                        .text_color(muted)
                                        .child(empty_text.clone())
                                        .into_any_element()]
                                } else {
                                    items
                                        .iter()
                                        .map(|(cluster, topic, key, value)| {
                                            v_flex()
                                                .py_2()
                                                .border_b_1()
                                                .border_color(border)
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(muted)
                                                        .child(format!("{} / {}", cluster, topic)),
                                                )
                                                .when(!key.is_empty(), |el| {
                                                    el.child(
                                                        div()
                                                            .text_xs()
                                                            .child(format!("key: {}", key)),
                                                    )
                                                })
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .child(value.clone()),
                                                )
                                                .into_any_element()
                                        })
                                        .collect()
                                };
                                dialog
                                    .title(title.clone())
                                    .w(px(720.0))
                                    .child(
                                        div()
                                            .id("history-scroll")
                                            .max_h(px(480.0))
                                            .overflow_y_scroll()
                                            .child(v_flex().children(rows)),
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

    fn format_timestamp(ts: i64) -> String {
        chrono::DateTime::from_timestamp_millis(ts)
            .map(|dt| dt.format("%m-%d %H:%M:%S%.3f").to_string())
            .unwrap_or_else(|| ts.to_string())
    }
}

/// 表单字段行：标签 + 控件
fn field_row(label: &str, control: AnyElement) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_sm().child(label.to_string()))
        .child(control)
}

impl Render for MessagesPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let can_query = self.selected_cluster(cx).is_some()
            && self.selected_topic(cx).is_some()
            && !self.streaming;

        // 头部：标题 + 操作按钮
        let header = h_flex()
            .items_center()
            .justify_between()
            .mb_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().font_semibold().child(t(cx, "messages.title")))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "messages.description")),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("send")
                            .outline()
                            .label(t(cx, "messages.sendMessage"))
                            .disabled(self.selected_topic(cx).is_none())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_send(window, cx);
                            })),
                    )
                    .child(
                        Button::new("export")
                            .outline()
                            .label(t(cx, "messages.exportMessages"))
                            .disabled(self.messages.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.export_messages(cx);
                            })),
                    )
                    .child(
                        Button::new("history")
                            .outline()
                            .label(t(cx, "sentMessageHistory.title"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_history(cx);
                            })),
                    ),
            );

        // 查询工具栏
        let toolbar = h_flex()
            .gap_2()
            .mb_3()
            .flex_wrap()
            .children(
                self.cluster_state
                    .as_ref()
                    .map(|s| div().w_40().child(Select::new(s)).into_any_element()),
            )
            .children(
                self.topic_state
                    .as_ref()
                    .map(|s| div().w_48().child(Select::new(s)).into_any_element()),
            )
            .child(div().w(px(112.0)).child(Select::new(&self.mode_state)))
            .child(div().w_24().child(Input::new(&self.partition_input)))
            .child(div().w_20().child(Input::new(&self.max_input)))
            .child(div().w(px(112.0)).child(Select::new(&self.search_in_state)))
            .child(div().w_48().child(Input::new(&self.search_input)))
            .child(if self.streaming {
                Button::new("stop")
                    .danger()
                    .label(t(cx, "messages.stop"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.stop_query(cx);
                    }))
            } else {
                Button::new("query")
                    .primary()
                    .label(t(cx, "messages.query"))
                    .disabled(!can_query)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.start_query(cx);
                    }))
            })
            .child(
                Button::new("sort")
                    .ghost()
                    .icon(if self.sort_desc {
                        IconName::SortDescending
                    } else {
                        IconName::SortAscending
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_sort(cx);
                    })),
            );

        // 进度条
        let progress = if self.streaming {
            let pct = if self.total > 0 {
                (self.received as f32 / self.total as f32 * 100.0).min(100.0)
            } else {
                0.0
            };
            h_flex()
                .gap_2()
                .items_center()
                .mb_2()
                .child(div().flex_1().child(
                    gpui_component::progress::Progress::new().value(pct),
                ))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{}/{}", self.received, self.total)),
                )
                .into_any_element()
        } else if let Some(err) = &self.error {
            div()
                .text_sm()
                .text_color(theme.danger)
                .mb_2()
                .child(err.clone())
                .into_any_element()
        } else {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .mb_2()
                .child(format!("{}: {}", t(cx, "messages.totalMessages"), self.messages.len()))
                .into_any_element()
        };

        // 消息表格
        let table_header = h_flex()
            .px_2()
            .py_1()
            .bg(theme.table_head)
            .text_xs()
            .font_semibold()
            .child(div().w_16().child(t(cx, "messages.partitionLabel2")))
            .child(div().w_24().child(t(cx, "messages.offsetLabel")))
            .child(div().w_40().child(t(cx, "messages.timestampLabel")))
            .child(div().w_40().child(t(cx, "messages.key")))
            .child(div().flex_1().child(t(cx, "messages.value")));

        let messages = self.messages.clone();
        let selected = self.selected;
        let entity = cx.entity();
        let table_body: AnyElement = if messages.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .child(t(cx, "messages.noMessages"))
                .into_any_element()
        } else {
            let border = theme.border;
            let active = theme.table_active;
            uniform_list("message-list", messages.len(), move |range, _window, _cx| {
                range
                    .map(|ix| {
                        let m = messages[ix].clone();
                        let is_selected = selected == Some(ix);
                        let entity = entity.clone();
                        h_flex()
                            .px_2()
                            .py_0p5()
                            .w_full()
                            .border_b_1()
                            .border_color(border)
                            .when(is_selected, |el| el.bg(active))
                            .child(
                                div()
                                    .w_16()
                                    .text_xs()
                                    .child(m.partition.to_string()),
                            )
                            .child(
                                div()
                                    .w_24()
                                    .text_xs()
                                    .child(m.offset.to_string()),
                            )
                            .child(
                                div()
                                    .w_40()
                                    .text_xs()
                                    .child(MessagesPage::format_timestamp(m.timestamp)),
                            )
                            .child(
                                div()
                                    .w_40()
                                    .text_xs()
                                    .overflow_hidden()
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

        // 详情面板
        let detail: AnyElement = match self.selected.and_then(|ix| self.messages.get(ix)) {
            Some(m) => {
                let key_text = m.key.clone().unwrap_or_else(|| "-".to_string());
                let value_raw = m.value.clone().unwrap_or_default();
                // 尝试 JSON 美化
                let value_pretty = serde_json::from_str::<serde_json::Value>(&value_raw)
                    .ok()
                    .and_then(|v| serde_json::to_string_pretty(&v).ok())
                    .unwrap_or_else(|| value_raw.clone());
                let key_for_copy = key_text.clone();
                let value_for_copy = value_pretty.clone();

                v_flex()
                    .h(px(200.0))
                    .border_t_1()
                    .border_color(theme.border)
                    .p_2()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_4()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("{}: {}", t(cx, "messages.partitionLabel2"), m.partition))
                            .child(format!("{}: {}", t(cx, "messages.offsetLabel"), m.offset))
                            .child(MessagesPage::format_timestamp(m.timestamp))
                            .child(div().flex_1())
                            .child(
                                Button::new("copy-key")
                                    .ghost()
                                    .label(t(cx, "messages.copyKey"))
                                    .on_click(move |_, _, cx| {
                                        MessagesPage::copy_text(key_for_copy.clone(), cx);
                                    }),
                            )
                            .child(
                                Button::new("copy-value")
                                    .ghost()
                                    .label(t(cx, "messages.copyValue"))
                                    .on_click(move |_, _, cx| {
                                        MessagesPage::copy_text(value_for_copy.clone(), cx);
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .child(format!("{}: {}", t(cx, "messages.key"), key_text)),
                    )
                    .child(
                        div()
                            .id("detail-value-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .child(div().text_xs().child(value_pretty)),
                    )
                    .into_any_element()
            }
            None => div().into_any_element(),
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(toolbar)
            .child(progress)
            .child(table_header)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .child(table_body),
            )
            .child(detail)
    }
}
