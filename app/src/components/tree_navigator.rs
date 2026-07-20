//! 树形导航器：与旧版 Vue ClusterTreeNavigator 行为一致
//!
//! 集群节点（健康检查 + 状态点）
//!  ├─ [N] Topics 文件夹（搜索框 + 刷新按钮，数据优先读 SQLite 缓存）
//!  └─ [N] Consumer Groups 文件夹（同上）
//! 附加：分组选择 chips、全局搜索、浏览历史面板、连接失败错误对话框

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::*;
use serde_json::json;

use crate::components::navigator::NavEvent;
use crate::i18n::t;
use crate::components::notify;
use crate::state::{Backend, Page, TokioRuntime};

#[derive(Clone, Debug)]
struct ClusterNode {
    name: String,
    group_id: Option<i64>,
    healthy: Option<bool>,
}

#[derive(Clone, Debug)]
struct HistoryItem {
    cluster: String,
    topic: String,
}

pub struct TreeNavigator {
    window_handle: AnyWindowHandle,
    clusters: Vec<ClusterNode>,
    groups: Vec<(i64, String)>,
    selected_group: Option<i64>,
    // 展开状态
    expanded_clusters: HashSet<String>,
    expanded_topic_folders: HashSet<String>,
    expanded_cg_folders: HashSet<String>,
    // 数据（优先 SQLite 缓存，空则 refresh 后重读）
    topics: HashMap<String, Rc<Vec<String>>>,
    consumer_groups: HashMap<String, Rc<Vec<String>>>,
    // 每个集群文件夹内的搜索框
    topic_search: HashMap<String, Entity<InputState>>,
    cg_search: HashMap<String, Entity<InputState>>,
    // 刷新中状态（按钮旋转）
    refreshing_topics: HashSet<String>,
    refreshing_cgs: HashSet<String>,
    favorites: HashSet<String>,
    selected_topic: Option<(String, String)>,
    // 全局搜索 + 历史面板
    search_input: Entity<InputState>,
    show_history: bool,
    history: Vec<HistoryItem>,
    loading: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NavEvent> for TreeNavigator {}

impl TreeNavigator {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "common.search"))
        });
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &search_input,
            |_this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        ));

        let this = Self {
            window_handle: window.window_handle(),
            clusters: Vec::new(),
            groups: Vec::new(),
            selected_group: None,
            expanded_clusters: HashSet::new(),
            expanded_topic_folders: HashSet::new(),
            expanded_cg_folders: HashSet::new(),
            topics: HashMap::new(),
            consumer_groups: HashMap::new(),
            topic_search: HashMap::new(),
            cg_search: HashMap::new(),
            refreshing_topics: HashSet::new(),
            refreshing_cgs: HashSet::new(),
            favorites: HashSet::new(),
            selected_topic: None,
            search_input,
            show_history: false,
            history: Vec::new(),
            loading: true,
            _subscriptions: subscriptions,
        };
        this.reload(cx);
        this
    }

    /// 加载集群、分组、收藏、保存的分组选择
    fn reload(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let clusters = crate::service::call(&rt, state.clone(), "cluster.list", json!({})).await;
            let groups = crate::service::call(&rt, state.clone(), "cluster_group.list", json!({})).await;
            let favorites = crate::service::call(&rt, state.clone(), "favorite.list", json!({})).await;
            let saved_group = crate::service::call(
                &rt,
                state,
                "settings.get",
                json!({ "keys": ["ui.selected_group_id"] }),
            )
            .await;

            this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(value) = clusters {
                    this.clusters = value
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|c| {
                                    Some(ClusterNode {
                                        name: c.get("name")?.as_str()?.to_string(),
                                        group_id: c.get("group_id").and_then(|v| v.as_i64()),
                                        healthy: None,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                }
                if let Ok(value) = groups {
                    this.groups = value
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|g| {
                                    Some((
                                        g.get("id")?.as_i64()?,
                                        g.get("name")?.as_str()?.to_string(),
                                    ))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
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
                // 恢复保存的分组选择
                if let Ok(value) = saved_group {
                    this.selected_group = value
                        .get("settings")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|s| s.get("value"))
                        .and_then(|v| v.as_str())
                        .and_then(|v| v.parse::<i64>().ok());
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn save_group_selection(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let value = self
            .selected_group
            .map(|g| g.to_string())
            .unwrap_or_else(|| "null".to_string());
        cx.spawn(async move |_this, _cx| {
            let _ = crate::service::call(
                &rt,
                state,
                "settings.update",
                json!({ "key": "ui.selected_group_id", "value": value }),
            )
            .await;
        })
        .detach();
    }

    /// 展开/收起集群；展开时做健康检查（与旧版一致）
    fn toggle_cluster(&mut self, cluster: String, cx: &mut Context<Self>) {
        if self.expanded_clusters.contains(&cluster) {
            self.expanded_clusters.remove(&cluster);
            cx.notify();
            return;
        }
        self.check_cluster_health(cluster, cx);
    }

    /// 健康检查：健康则展开；失败仍展开（可查看缓存数据），临时错误 Toast，其他错误弹窗
    fn check_cluster_health(&mut self, cluster: String, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "connection.health_check",
                json!({ "cluster_id": cluster }),
            )
            .await;

            let (healthy, error_msg) = match &result {
                Ok(v) => (
                    v.get("healthy").and_then(|h| h.as_bool()).unwrap_or(false),
                    v.get("error_message").and_then(|e| e.as_str()).map(|s| s.to_string()),
                ),
                Err(e) => (false, Some(e.clone())),
            };

            this.update(cx, |this, cx| {
                if let Some(node) = this.clusters.iter_mut().find(|c| c.name == cluster) {
                    node.healthy = Some(healthy);
                }
                // 无论健康与否都展开（可查看 SQLite 缓存数据）
                this.expanded_clusters.insert(cluster.clone());
                this.expanded_topic_folders.insert(cluster.clone());
                cx.notify();

                if !healthy {
                    let msg = error_msg.unwrap_or_else(|| "Cluster unavailable".to_string());
                    let is_transient = msg.contains("BrokerTransportFailure")
                        || msg.contains("timed out")
                        || msg.contains("Transport")
                        || msg.contains("Metadata fetch failed");
                    if is_transient {
                        let text = format!("{}: {} - {}", t(cx, "clusters.connectionFailed"), cluster, msg);
                        notify(cx, NotificationType::Error, text);
                    } else {
                        this.show_connection_error(cluster.clone(), msg, cx);
                    }
                }

                // 加载两个文件夹的数据
                if !this.topics.contains_key(&cluster) {
                    this.load_topics(cluster.clone(), cx);
                }
                if !this.consumer_groups.contains_key(&cluster) {
                    this.load_consumer_groups(cluster, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// 连接失败错误对话框（带重试）
    fn show_connection_error(&mut self, cluster: String, message: String, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let cluster_for_dialog = cluster.clone();
        let _ = self.window_handle.update(cx, |_, window, cx| {
            window.open_dialog(cx, move |dialog, _window, cx| {
                let entity = entity.clone();
                let cluster = cluster_for_dialog.clone();
                let danger = cx.theme().danger;
                dialog
                    .title("Connection Failed")
                    .w(px(420.0))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(format!("Cluster: {}", cluster)))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(danger)
                                    .child(message.clone()),
                            ),
                    )
                    .button_props(
                        DialogButtonProps::default()
                            .ok_text("Retry")
                            .ok_variant(ButtonVariant::Danger),
                    )
                    .on_ok(move |_, _window, cx| {
                        entity.update(cx, |this, cx| {
                            this.check_cluster_health(cluster.clone(), cx)
                        });
                        true
                    })
            });
        });
    }

    /// 加载 Topics：优先 SQLite 缓存（topic.saved），为空则刷新后重读
    fn load_topics(&self, cluster: String, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let saved = crate::service::call(
                &rt,
                state.clone(),
                "topic.saved",
                json!({ "cluster_id": cluster }),
            )
            .await
            .ok()
            .and_then(|v| v.get("topics").and_then(|t| t.as_array()).cloned())
            .unwrap_or_default();

            let names: Vec<String> = if saved.is_empty() {
                // 缓存为空：先从集群同步，再重读
                let _ = crate::service::call(
                    &rt,
                    state.clone(),
                    "topic.refresh",
                    json!({ "cluster_id": cluster }),
                )
                .await;
                crate::service::call(
                    &rt,
                    state,
                    "topic.saved",
                    json!({ "cluster_id": cluster }),
                )
                .await
                .ok()
                .and_then(|v| v.get("topics").and_then(|t| t.as_array()).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
            } else {
                saved
                    .iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            };

            this.update(cx, |this, cx| {
                this.topics.insert(cluster, Rc::new(names));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 加载消费组：优先 SQLite 缓存，为空则刷新后重读
    fn load_consumer_groups(&self, cluster: String, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let saved = crate::service::call(
                &rt,
                state.clone(),
                "consumer_group.saved",
                json!({ "cluster_id": cluster }),
            )
            .await
            .ok()
            .and_then(|v| v.get("groups").and_then(|g| g.as_array()).cloned())
            .unwrap_or_default();

            let names: Vec<String> = if saved.is_empty() {
                let _ = crate::service::call(
                    &rt,
                    state.clone(),
                    "consumer_group.refresh",
                    json!({ "cluster_id": cluster }),
                )
                .await;
                crate::service::call(
                    &rt,
                    state,
                    "consumer_group.saved",
                    json!({ "cluster_id": cluster }),
                )
                .await
                .ok()
                .and_then(|v| v.get("groups").and_then(|g| g.as_array()).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|g| g.as_str().map(|s| s.to_string()))
                .collect()
            } else {
                saved
                    .iter()
                    .filter_map(|g| g.as_str().map(|s| s.to_string()))
                    .collect()
            };

            this.update(cx, |this, cx| {
                this.consumer_groups.insert(cluster, Rc::new(names));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 刷新 Topics（搜索框非空时只刷新单个 topic）
    fn refresh_topics(&mut self, cluster: String, cx: &mut Context<Self>) {
        if self.refreshing_topics.contains(&cluster) {
            let text = t(cx, "clusters.refreshingBg");
            notify(cx, NotificationType::Info, text);
            return;
        }
        self.refreshing_topics.insert(cluster.clone());
        cx.notify();

        let single_topic = self
            .topic_search
            .get(&cluster)
            .map(|s| s.read(cx).value().to_string())
            .filter(|q| !q.trim().is_empty());

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let mut params = json!({ "cluster_id": cluster });
            if let Some(topic) = &single_topic {
                params["topic_name"] = json!(topic.trim());
            }
            let _ = crate::service::call(&rt, state.clone(), "topic.refresh", params).await;

            // 等待后台同步后重读缓存
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let names: Vec<String> = crate::service::call(
                &rt,
                state,
                "topic.saved",
                json!({ "cluster_id": cluster }),
            )
            .await
            .ok()
            .and_then(|v| v.get("topics").and_then(|t| t.as_array()).cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|t| t.as_str().map(|s| s.to_string()))
            .collect();

            this.update(cx, |this, cx| {
                this.topics.insert(cluster.clone(), Rc::new(names));
                this.refreshing_topics.remove(&cluster);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 刷新消费组
    fn refresh_consumer_groups(&mut self, cluster: String, cx: &mut Context<Self>) {
        if self.refreshing_cgs.contains(&cluster) {
            return;
        }
        self.refreshing_cgs.insert(cluster.clone());
        cx.notify();

        let single_group = self
            .cg_search
            .get(&cluster)
            .map(|s| s.read(cx).value().to_string())
            .filter(|q| !q.trim().is_empty());

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let mut params = json!({ "cluster_id": cluster });
            if let Some(group) = &single_group {
                params["group_name"] = json!(group.trim());
            }
            let _ = crate::service::call(&rt, state.clone(), "consumer_group.refresh", params).await;

            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let names: Vec<String> = crate::service::call(
                &rt,
                state,
                "consumer_group.saved",
                json!({ "cluster_id": cluster }),
            )
            .await
            .ok()
            .and_then(|v| v.get("groups").and_then(|g| g.as_array()).cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|g| g.as_str().map(|s| s.to_string()))
            .collect();

            this.update(cx, |this, cx| {
                this.consumer_groups.insert(cluster.clone(), Rc::new(names));
                this.refreshing_cgs.remove(&cluster);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_topic_folder(&mut self, cluster: String, cx: &mut Context<Self>) {
        if !self.expanded_topic_folders.remove(&cluster) {
            self.expanded_topic_folders.insert(cluster.clone());
            if !self.topics.contains_key(&cluster) {
                self.load_topics(cluster, cx);
            }
        }
        cx.notify();
    }

    fn toggle_cg_folder(&mut self, cluster: String, cx: &mut Context<Self>) {
        if !self.expanded_cg_folders.remove(&cluster) {
            self.expanded_cg_folders.insert(cluster.clone());
            if !self.consumer_groups.contains_key(&cluster) {
                self.load_consumer_groups(cluster, cx);
            }
        }
        cx.notify();
    }

    fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.expanded_clusters.clear();
        self.expanded_topic_folders.clear();
        self.expanded_cg_folders.clear();
        cx.notify();
    }

    /// 选中 Topic：按需重连后跳转消息页
    fn open_topic(&mut self, cluster: String, topic: String, cx: &mut Context<Self>) {
        self.selected_topic = Some((cluster.clone(), topic.clone()));
        cx.notify();
        self.ensure_connected(cluster.clone(), cx);
        cx.emit(NavEvent::OpenMessages { cluster, topic });
    }

    /// 集群未连接时静默重连
    fn ensure_connected(&self, cluster: String, cx: &mut Context<Self>) {
        let healthy = self
            .clusters
            .iter()
            .find(|c| c.name == cluster)
            .and_then(|c| c.healthy);
        if healthy == Some(true) {
            return;
        }
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        cx.spawn(async move |_this, _cx| {
            let _ = crate::service::call(
                &rt,
                state,
                "connection.reconnect",
                json!({ "cluster_name": cluster }),
            )
            .await;
        })
        .detach();
    }

    /// 切换浏览历史面板
    fn toggle_history(&mut self, cx: &mut Context<Self>) {
        self.show_history = !self.show_history;
        cx.notify();
        if self.show_history && self.history.is_empty() {
            self.load_history(cx);
        }
    }

    fn load_history(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic_history.list",
                json!({ "limit": 50 }),
            )
            .await;
            this.update(cx, |this, cx| {
                this.history = result
                    .ok()
                    .and_then(|v| {
                        v.get("history")
                            .or_else(|| v.get("items"))
                            .and_then(|h| h.as_array())
                            .cloned()
                    })
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|h| {
                        Some(HistoryItem {
                            cluster: h.get("cluster_id")?.as_str()?.to_string(),
                            topic: h.get("topic_name")?.as_str()?.to_string(),
                        })
                    })
                    .collect();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 文件夹内搜索框（惰性创建）
    fn folder_search(
        &mut self,
        cluster: &str,
        is_cg: bool,
        cx: &mut Context<Self>,
    ) -> Option<Entity<InputState>> {
        if is_cg {
            if let Some(state) = self.cg_search.get(cluster) {
                return Some(state.clone());
            }
        } else if let Some(state) = self.topic_search.get(cluster) {
            return Some(state.clone());
        }

        let state = self
            .window_handle
            .update(cx, |_, window, cx| {
                cx.new(|cx| {
                    InputState::new(window, cx).placeholder(t(cx, "common.search"))
                })
            })
            .ok()?;

        // 订阅变化 → 重绘（客户端过滤）
        let sub = cx.subscribe(&state, |_this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        self._subscriptions.push(sub);
        if is_cg {
            self.cg_search.insert(cluster.to_string(), state.clone());
        } else {
            self.topic_search.insert(cluster.to_string(), state.clone());
        }
        Some(state)
    }

    fn nav_button(
        &self,
        id: &'static str,
        icon: IconName,
        title: String,
        page: Page,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .icon(icon)
            .tooltip(title)
            .on_click(cx.listener(move |_this, _, _, cx| {
                cx.emit(NavEvent::OpenPage(page));
            }))
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (border, secondary, primary, muted) = {
            let theme = cx.theme();
            (theme.border, theme.secondary, theme.primary, theme.muted_foreground)
        };

        v_flex()
            .gap_1()
            .p_2()
            .border_b_1()
            .border_color(border)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .size_6()
                                    .rounded_md()
                                    .bg(secondary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Icon::new(IconName::Building2).text_color(primary)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(muted)
                                    .child(if self.show_history {
                                        t(cx, "history.title")
                                    } else {
                                        "CLUSTERS".to_string()
                                    }),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_0p5()
                            .when(self.show_history, |el| {
                                el.child(
                                    Button::new("back-clusters")
                                        .ghost()
                                        .icon(IconName::ArrowLeft)
                                        .tooltip(t(cx, "tree.backToClusters"))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.show_history = false;
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(!self.show_history, |el| {
                                el.child(
                                    Button::new("collapse-all")
                                        .ghost()
                                        .icon(IconName::ChevronUp)
                                        .tooltip(t(cx, "tree.collapseAll"))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.collapse_all(cx);
                                        })),
                                )
                                .child(self.nav_button(
                                    "tree-clusters",
                                    IconName::Building2,
                                    t(cx, "nav.clusters"),
                                    Page::Clusters,
                                    cx,
                                ))
                                .child(self.nav_button(
                                    "tree-favorites",
                                    IconName::Star,
                                    t(cx, "tree.topicFavorites"),
                                    Page::Favorites,
                                    cx,
                                ))
                                .child(self.nav_button(
                                    "tree-schema",
                                    IconName::BookOpen,
                                    t(cx, "tree.schemaRegistry"),
                                    Page::SchemaRegistry,
                                    cx,
                                ))
                                .child(
                                    Button::new("tree-history")
                                        .ghost()
                                        .icon(IconName::Calendar)
                                        .tooltip(t(cx, "history.title"))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_history(cx);
                                        })),
                                )
                            }),
                    ),
            )
            // 分组选择器（有分组时显示）
            .when(!self.groups.is_empty() && !self.show_history, |el| {
                el.child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .flex_wrap()
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child(format!("{}:", t(cx, "clusters.group"))),
                        )
                        .child(
                            Button::new("group-all")
                                .ghost()
                                .xsmall()
                                .label(t(cx, "common.all"))
                                .when(self.selected_group.is_none(), |b| b.outline())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.selected_group = None;
                                    this.save_group_selection(cx);
                                    cx.notify();
                                })),
                        )
                        .children(self.groups.iter().map(|(gid, name)| {
                            let gid = *gid;
                            let active = self.selected_group == Some(gid);
                            Button::new(("group", gid as usize))
                                .ghost()
                                .xsmall()
                                .label(name.clone())
                                .when(active, |b| b.outline())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.selected_group = Some(gid);
                                    this.save_group_selection(cx);
                                    cx.notify();
                                }))
                        })),
                )
            })
    }

    /// 文件夹行：[chevron] [count] 名称 + 刷新按钮
    fn render_folder_row(
        &self,
        cluster: &str,
        is_cg: bool,
        count: usize,
        expanded: bool,
        refreshing: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let cluster_toggle = cluster.to_string();
        let cluster_open = cluster.to_string();
        let cluster_refresh = cluster.to_string();
        let label = if is_cg { "Consumer Groups" } else { "Topics" };
        let id_prefix = if is_cg { "cgfolder" } else { "tfolder" };

        h_flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .pl_6()
            .rounded_md()
            .w_full()
            .hover(|el| el.bg(theme.list_hover))
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id(SharedString::from(format!("{}-chev-{}", id_prefix, cluster)))
                            .cursor_pointer()
                            .child(
                                Icon::new(IconName::ChevronRight)
                                    .size_3()
                                    .when(expanded, |i| {
                                        i.rotate(gpui::Radians(0.5 * std::f32::consts::PI))
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if is_cg {
                                    this.toggle_cg_folder(cluster_toggle.clone(), cx);
                                } else {
                                    this.toggle_topic_folder(cluster_toggle.clone(), cx);
                                }
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("{}-{}", id_prefix, cluster)))
                            .cursor_pointer()
                            .text_xs()
                            .child(format!("{} {}", count, label))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                // 点击文件夹：展开 + 导航到对应页面（与旧版一致）
                                if is_cg {
                                    if !this.expanded_cg_folders.contains(&cluster_open) {
                                        this.toggle_cg_folder(cluster_open.clone(), cx);
                                    }
                                    cx.emit(NavEvent::OpenConsumerGroups {
                                        cluster: cluster_open.clone(),
                                        group: None,
                                    });
                                } else {
                                    if !this.expanded_topic_folders.contains(&cluster_open) {
                                        this.toggle_topic_folder(cluster_open.clone(), cx);
                                    }
                                    cx.emit(NavEvent::OpenTopics {
                                        cluster: cluster_open.clone(),
                                    });
                                }
                            })),
                    ),
            )
            .child(
                Button::new(SharedString::from(format!("{}-refresh-{}", id_prefix, cluster)))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Redo2)
                    .loading(refreshing)
                    .tooltip(t(cx, "common.refresh"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if is_cg {
                            this.refresh_consumer_groups(cluster_refresh.clone(), cx);
                        } else {
                            this.refresh_topics(cluster_refresh.clone(), cx);
                        }
                    })),
            )
            .into_any_element()
    }

    /// 集群子树（两个文件夹 + 内容）
    fn render_cluster(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let (success_c, danger_c, warning_c, primary_c, hover_c, active_c, secondary_c, muted_c, border_c) = {
            let theme = cx.theme();
            (
                theme.success,
                theme.danger,
                theme.warning,
                theme.primary,
                theme.list_hover,
                theme.list_active,
                theme.secondary,
                theme.muted_foreground,
                theme.border,
            )
        };
        let _ = border_c;
        let cluster = self.clusters[ix].clone();
        let name = cluster.name.clone();
        let expanded = self.expanded_clusters.contains(&name);
        let topic_folder_expanded = self.expanded_topic_folders.contains(&name);
        let cg_folder_expanded = self.expanded_cg_folders.contains(&name);
        let global_search = self.search_input.read(cx).value().to_lowercase();

        // 文件夹内搜索过滤
        let topic_query = self
            .topic_search
            .get(&name)
            .map(|s| s.read(cx).value().to_lowercase())
            .unwrap_or_default();
        let cg_query = self
            .cg_search
            .get(&name)
            .map(|s| s.read(cx).value().to_lowercase())
            .unwrap_or_default();

        let all_topics = self.topics.get(&name).cloned().unwrap_or_default();
        let all_cgs = self.consumer_groups.get(&name).cloned().unwrap_or_default();

        let visible_topics: Vec<String> = all_topics
            .iter()
            .filter(|t| {
                (global_search.is_empty() || t.to_lowercase().contains(&global_search))
                    && (topic_query.is_empty() || t.to_lowercase().contains(&topic_query))
            })
            .cloned()
            .collect();
        let visible_cgs: Vec<String> = all_cgs
            .iter()
            .filter(|g| {
                (global_search.is_empty() || g.to_lowercase().contains(&global_search))
                    && (cg_query.is_empty() || g.to_lowercase().contains(&cg_query))
            })
            .cloned()
            .collect();

        // 全局搜索时：集群名不匹配且无匹配内容则隐藏
        if !global_search.is_empty()
            && !name.to_lowercase().contains(&global_search)
            && visible_topics.is_empty()
            && visible_cgs.is_empty()
        {
            return div().into_any_element();
        }

        let dot_color = match cluster.healthy {
            Some(true) => success_c,
            Some(false) => danger_c,
            None => warning_c,
        };

        // 集群行
        let name_toggle = name.clone();
        let cluster_row = h_flex()
            .items_center()
            .gap_1p5()
            .p_2()
            .rounded_lg()
            .w_full()
            .cursor_pointer()
            .hover(|el| el.bg(hover_c))
            .when(expanded, |el| el.bg(active_c))
            .child(
                Icon::new(IconName::ChevronRight)
                    .size_4()
                    .when(expanded, |i| i.rotate(gpui::Radians(0.5 * std::f32::consts::PI))),
            )
            .child(div().size_2().rounded_full().bg(dot_color))
            .child(Icon::new(IconName::Building2).text_color(primary_c))
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(name.clone()),
            )
            .id(SharedString::from(format!("cluster-{}", name)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_cluster(name_toggle.clone(), cx);
            }))
            .into_any_element();

        if !expanded {
            return v_flex().child(cluster_row).into_any_element();
        }

        let mut children: Vec<AnyElement> = Vec::new();

        // ===== Topics 文件夹 =====
        children.push(self.render_folder_row(
            &name,
            false,
            all_topics.len(),
            topic_folder_expanded,
            self.refreshing_topics.contains(&name),
            cx,
        ));
        if topic_folder_expanded {
            // 文件夹内搜索框
            if !all_topics.is_empty() {
                if let Some(search_state) = self.folder_search(&name, false, cx) {
                    let match_text = if topic_query.is_empty() {
                        format!("{} topics", all_topics.len())
                    } else {
                        format!("{} matching", visible_topics.len())
                    };
                    children.push(
                        v_flex()
                            .pl_8()
                            .pr_2()
                            .py_1()
                            .child(Input::new(&search_state).small())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_c)
                                    .pt_0p5()
                                    .child(match_text),
                            )
                            .into_any_element(),
                    );
                }
            }
            // Topic 行
            if visible_topics.is_empty() {
                children.push(
                    div()
                        .pl_12()
                        .py_1()
                        .text_xs()
                        .text_color(muted_c)
                        .child(t(cx, "common.noData"))
                        .into_any_element(),
                );
            } else {
                for topic in &visible_topics {
                    let key = format!("{}\u{1}{}", name, topic);
                    let is_fav = self.favorites.contains(&key);
                    let is_selected = self
                        .selected_topic
                        .as_ref()
                        .map(|(c, t)| c == &name && t == topic)
                        .unwrap_or(false);
                    let cluster_open = name.clone();
                    let topic_open = topic.clone();

                    children.push(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .pl_10()
                            .py_1()
                            .rounded_md()
                            .w_full()
                            .cursor_pointer()
                            .hover(|el| el.bg(hover_c))
                            .when(is_selected, |el| el.bg(active_c))
                            .child(Icon::new(IconName::FolderOpen).size_3().text_color(secondary_c))
                            .when(is_fav, |el| {
                                el.child(Icon::new(IconName::Star).size_3().text_color(warning_c))
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .child(topic.clone()),
                            )
                            .id(SharedString::from(format!("topic-{}", key)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_topic(cluster_open.clone(), topic_open.clone(), cx);
                            }))
                            .into_any_element(),
                    );
                }
            }
        }

        // ===== Consumer Groups 文件夹 =====
        children.push(self.render_folder_row(
            &name,
            true,
            all_cgs.len(),
            cg_folder_expanded,
            self.refreshing_cgs.contains(&name),
            cx,
        ));
        if cg_folder_expanded {
            if !all_cgs.is_empty() {
                if let Some(search_state) = self.folder_search(&name, true, cx) {
                    let match_text = if cg_query.is_empty() {
                        format!("{} consumer groups", all_cgs.len())
                    } else {
                        format!("{} matching", visible_cgs.len())
                    };
                    children.push(
                        v_flex()
                            .pl_8()
                            .pr_2()
                            .py_1()
                            .child(Input::new(&search_state).small())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_c)
                                    .pt_0p5()
                                    .child(match_text),
                            )
                            .into_any_element(),
                    );
                }
            }
            if visible_cgs.is_empty() {
                children.push(
                    div()
                        .pl_12()
                        .py_1()
                        .text_xs()
                        .text_color(muted_c)
                        .child(t(cx, "consumerGroups.noData"))
                        .into_any_element(),
                );
            } else {
                for group in &visible_cgs {
                    let cluster_open = name.clone();
                    let group_open = group.clone();
                    let key = format!("{}\u{1}{}", name, group);
                    children.push(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .pl_10()
                            .py_1()
                            .rounded_md()
                            .w_full()
                            .cursor_pointer()
                            .hover(|el| el.bg(hover_c))
                            .child(Icon::new(IconName::CircleUser).size_3().text_color(secondary_c))
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .child(group.clone()),
                            )
                            .id(SharedString::from(format!("cg-{}", key)))
                            .on_click(cx.listener(move |_this, _, _, cx| {
                                cx.emit(NavEvent::OpenConsumerGroups {
                                    cluster: cluster_open.clone(),
                                    group: Some(group_open.clone()),
                                });
                            }))
                            .into_any_element(),
                    );
                }
            }
        }

        v_flex().child(cluster_row).children(children).into_any_element()
    }

    /// 浏览历史面板
    fn render_history(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        if self.history.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(t(cx, "history.empty"))
                .into_any_element();
        }

        let rows: Vec<AnyElement> = self
            .history
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                let cluster = item.cluster.clone();
                let topic = item.topic.clone();
                h_flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_1p5()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|el| el.bg(theme.list_hover))
                    .child(
                        v_flex()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .child(item.topic.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(item.cluster.clone()),
                            ),
                    )
                    .id(("history", ix))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_topic(cluster.clone(), topic.clone(), cx);
                    }))
                    .into_any_element()
            })
            .collect();

        div()
            .id("history-scroll")
            .size_full()
            .overflow_y_scroll()
            .p_1()
            .child(v_flex().children(rows))
            .into_any_element()
    }
}

impl Render for TreeNavigator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (bg_sidebar, border_c, muted_c) = {
            let theme = cx.theme();
            (theme.sidebar, theme.border, theme.muted_foreground)
        };

        let search_bar = div()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(border_c)
            .child(Input::new(&self.search_input).small());

        let content: AnyElement = if self.show_history {
            self.render_history(cx)
        } else if self.loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(gpui_component::spinner::Spinner::new())
                .into_any_element()
        } else {
            let count = self
                .clusters
                .iter()
                .filter(|c| self.selected_group.is_none() || c.group_id == self.selected_group)
                .count();

            if count == 0 {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(muted_c)
                    .child(t(cx, "common.noData"))
                    .into_any_element()
            } else {
                let indices: Vec<usize> = (0..self.clusters.len())
                    .filter(|&ix| {
                        let c = &self.clusters[ix];
                        self.selected_group.is_none() || c.group_id == self.selected_group
                    })
                    .collect();
                let mut rows: Vec<AnyElement> = Vec::with_capacity(indices.len());
                for ix in indices {
                    rows.push(self.render_cluster(ix, cx));
                }

                div()
                    .id("tree-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .p_1()
                    .child(v_flex().children(rows))
                    .into_any_element()
            }
        };

        v_flex()
            .size_full()
            .bg(bg_sidebar)
            .child(self.render_header(cx))
            .when(!self.show_history, |el| el.child(search_bar))
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
