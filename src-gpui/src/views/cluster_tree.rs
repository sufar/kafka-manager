//! ClusterTreeNavigator：树模式左侧导航（分组过滤 → 集群 → Topics/Consumer Groups 文件夹 → 叶子节点）
//! 对齐 Vue ClusterTreeNavigator.vue 的行为（含懒加载、健康检查、连接错误分级处理、右键菜单）。

use std::collections::{HashMap, HashSet};

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    ScrollHandle, Window,
};

use crate::app::{backend, root, Page, Route};
use crate::i18n::t;
use crate::icons::icon;
use crate::models::*;
use crate::overlay;
use crate::theme;
use crate::views::navigator::relative_time;
use crate::widgets::common::*;
use crate::widgets::text_input::{TextInput, TextInputEvent};

/// 单个集群节点的全部状态
#[derive(Default)]
struct ClusterNode {
    health: Option<bool>,
    health_error: Option<String>,
    expanded: bool,
    topics_expanded: bool,
    cgs_expanded: bool,
    topics: Option<Vec<String>>,
    cgs: Option<Vec<String>>,
    topic_count: Option<usize>,
    cg_count: Option<usize>,
    loading_topics: bool,
    loading_cgs: bool,
    refreshing_topics: bool,
    refreshing_cgs: bool,
    topic_search: Option<Entity<TextInput>>,
    cg_search: Option<Entity<TextInput>>,
}

pub struct ClusterTreeNavigator {
    nodes: HashMap<String, ClusterNode>,
    selected_group_id: Option<i64>,
    group_sel_loaded: bool,
    selected_topic: Option<(String, String)>, // (topic, cluster)
    scroll: ScrollHandle,
    groups_scroll: ScrollHandle,
    show_history: bool,
    history_items: Vec<TopicHistoryItem>,
    history_loading: bool,
    history_search_input: Entity<TextInput>,
    focus_handle: FocusHandle,
    backend_seen_ready: bool,
}

impl ClusterTreeNavigator {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let history_search_input = cx.new(TextInput::new);
        history_search_input.update(cx, |i, _| {
            i.set_placeholder(t("topicHistory.searchPlaceholder"))
        });
        let this = Self {
            nodes: HashMap::new(),
            selected_group_id: None,
            group_sel_loaded: false,
            selected_topic: None,
            scroll: ScrollHandle::new(),
            groups_scroll: ScrollHandle::new(),
            show_history: false,
            history_items: vec![],
            history_loading: false,
            history_search_input: history_search_input.clone(),
            focus_handle: cx.focus_handle(),
            backend_seen_ready: false,
        };
        cx.subscribe(&history_search_input, |_this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::Changed) {
                cx.notify();
            }
        })
        .detach();
        this
    }

    pub fn on_clusters_changed(
        &mut self,
        existing: HashSet<String>,
        groups: Vec<ClusterGroup>,
        cx: &mut Context<Self>,
    ) {
        if !self.group_sel_loaded {
            self.group_sel_loaded = true;
            self.restore_group_selection(cx);
        }
        // 清理已删除集群的节点
        self.nodes.retain(|name, _| existing.contains(name));
        // 若选中分组被删除则重置
        if let Some(gid) = self.selected_group_id {
            if !groups.iter().any(|g| g.id == gid) {
                self.selected_group_id = None;
            }
        }
        cx.notify();
    }

    fn restore_group_selection(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            if let Ok(v) = b
                .dispatch(
                    "settings.get",
                    serde_json::json!({"keys": ["ui.selected_group_id"]}),
                )
                .await
            {
                let val = v
                    .get("settings")
                    .and_then(|s| s.get("ui.selected_group_id"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                this.update(cx, |this, cx| {
                    if let Some(val) = val {
                        this.selected_group_id = val.parse::<i64>().ok();
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn select_group(&mut self, gid: Option<i64>, cx: &mut Context<Self>) {
        self.selected_group_id = gid;
        let value = gid.map(|g| g.to_string()).unwrap_or_else(|| "null".into());
        let b = backend(cx);
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch(
                    "settings.update",
                    serde_json::json!({"key": "ui.selected_group_id", "value": value}),
                )
                .await;
        })
        .detach();
        cx.notify();
    }

    pub fn on_topic_deleted(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        if let Some(node) = self.nodes.get_mut(cluster) {
            if let Some(topics) = &mut node.topics {
                topics.retain(|t| t != topic);
                node.topic_count = Some(topics.len());
            }
        }
        cx.notify();
    }

    pub fn on_favorites_changed(&mut self, cx: &mut Context<Self>) {
        if self.show_history {
            self.load_history(cx);
        }
    }

    /// 外部设置集群健康状态（右键菜单操作后）
    pub fn set_health(&mut self, cluster: &str, healthy: Option<bool>, error: Option<String>, cx: &mut Context<Self>) {
        let node = self.node(cluster);
        node.health = healthy;
        node.health_error = error;
        cx.notify();
    }

    /// 重连后重新健康检查（pub 包装）
    pub fn check_cluster_health_pub(&mut self, cluster: &str, cx: &mut Context<Self>) {
        self.check_cluster_health(cluster, true, cx);
    }

    /// topic 创建后刷新该集群 topics
    pub fn on_topic_created(&mut self, cluster: &str, cx: &mut Context<Self>) {
        self.refresh_cluster_topics(cluster, cx);
    }

    fn node(&mut self, cluster: &str) -> &mut ClusterNode {
        self.nodes.entry(cluster.to_string()).or_default()
    }

    // ==================== 展开/健康检查 ====================

    fn toggle_cluster(&mut self, cluster: &str, cx: &mut Context<Self>) {
        let expanded = {
            let node = self.node(cluster);
            node.expanded = !node.expanded;
            node.expanded
        };
        cx.notify();
        if expanded {
            self.check_cluster_health(cluster, true, cx);
        }
    }

    /// 展开时健康检查：健康→展开+topic.count；不健康→仍展开+错误分级提示
    fn check_cluster_health(&mut self, cluster: &str, expanded: bool, cx: &mut Context<Self>) {
        let b = backend(cx);
        let cluster = cluster.to_string();
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "connection.health_check",
                    serde_json::json!({"cluster_id": cluster}),
                )
                .await;
            this.update(cx, |this, cx| {
                let (healthy, error) = match &result {
                    Ok(v) => (
                        v.get("healthy").and_then(|x| x.as_bool()),
                        v.get("error_message")
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string()),
                    ),
                    Err(e) => (Some(false), Some(e.clone())),
                };
                {
                    let node = this.node(&cluster);
                    node.health = healthy;
                    node.health_error = error.clone();
                }
                cx.notify();
                if healthy == Some(true) {
                    this.load_topic_count(&cluster, cx);
                } else if expanded {
                    this.show_connection_error(&cluster, error, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    fn load_topic_count(&mut self, cluster: &str, cx: &mut Context<Self>) {
        let b = backend(cx);
        let cluster = cluster.to_string();
        cx.spawn(async move |this, cx| {
            if let Ok(v) = b
                .dispatch("topic.count", serde_json::json!({"cluster_id": cluster}))
                .await
            {
                let count = v.get("count").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                this.update(cx, |this, cx| {
                    this.node(&cluster).topic_count = Some(count);
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    /// 连接错误分级：临时性错误仅 toast，其余弹对话框
    fn show_connection_error(&mut self, cluster: &str, error: Option<String>, cx: &mut Context<Self>) {
        let Some(msg) = error else { return };
        let transient = ["BrokerTransportFailure", "timed out", "Transport", "Metadata fetch failed"]
            .iter()
            .any(|k| msg.contains(k));
        if transient {
            overlay::toast_error(cx, format!("{}: {} - {}", t("tree.connectionFailed"), cluster, msg));
            return;
        }
        let cluster_owned = cluster.to_string();
        overlay::update_overlays(cx, |o| {
            o.confirm = Some(overlay::ConfirmState {
                title: t("tree.connectionFailed"),
                message: format!("{}\n{}", cluster, msg),
                confirm_label: t("common.retry"),
                danger: true,
                on_confirm: Some(Box::new(move |cx| {
                    root(cx).update(cx, |app, cx| {
                        app.tree_navigator.update(cx, |n, cx| {
                            n.check_cluster_health(&cluster_owned, true, cx);
                        });
                    });
                })),
            });
        });
    }

    // ==================== Topics 懒加载/刷新 ====================

    fn toggle_topics_folder(&mut self, cluster: &str, cx: &mut Context<Self>) {
        let expanded = {
            let node = self.node(cluster);
            node.topics_expanded = !node.topics_expanded;
            node.topics_expanded
        };
        cx.notify();
        if expanded {
            self.load_cluster_topics(cluster, cx);
        }
    }

    fn click_topics_folder(&mut self, cluster: &str, cx: &mut Context<Self>) {
        if !self.node(cluster).topics_expanded {
            self.node(cluster).topics_expanded = true;
            self.load_cluster_topics(cluster, cx);
        }
        cx.notify();
        let cluster = cluster.to_string();
        root(cx).update(cx, |app, cx| {
            let route = Route::new(Page::Topics).with("cluster", &cluster);
            app.navigate(route, true, cx);
        });
    }

    fn load_cluster_topics(&mut self, cluster: &str, cx: &mut Context<Self>) {
        if self.node(cluster).loading_topics {
            return;
        }
        self.node(cluster).loading_topics = true;
        cx.notify();
        let b = backend(cx);
        let cluster = cluster.to_string();
        cx.spawn(async move |this, cx| {
            let mut retry = 0;
            loop {
                // 先取数据库缓存
                let saved = b
                    .dispatch("topic.saved", serde_json::json!({"cluster_id": cluster}))
                    .await;
                let topics: Vec<String> = saved
                    .as_ref()
                    .ok()
                    .and_then(|v| v.get("topics").and_then(|x| x.as_array()).cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect();
                if !topics.is_empty() || saved.is_ok() && retry > 0 {
                    this.update(cx, |this, cx| {
                        let node = this.node(&cluster);
                        node.topics = Some(topics.clone());
                        node.topic_count = Some(topics.len());
                        node.loading_topics = false;
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                if topics.is_empty() && saved.is_ok() {
                    // 缓存为空 → 从 Kafka 同步一次再取
                    let _ = b
                        .dispatch("topic.refresh", serde_json::json!({"cluster_id": cluster}))
                        .await;
                    let saved2 = b
                        .dispatch("topic.saved", serde_json::json!({"cluster_id": cluster}))
                        .await;
                    let topics2: Vec<String> = saved2
                        .ok()
                        .and_then(|v| v.get("topics").and_then(|x| x.as_array()).cloned())
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect();
                    this.update(cx, |this, cx| {
                        let node = this.node(&cluster);
                        node.topics = Some(topics2.clone());
                        node.topic_count = Some(topics2.len());
                        node.loading_topics = false;
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                // 错误重试（仅 not connected / not found）
                let err = saved.err().unwrap_or_default();
                if retry < 3 && (err.contains("not connected") || err.contains("not found")) {
                    retry += 1;
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(retry * 500))
                        .await;
                    continue;
                }
                this.update(cx, |this, cx| {
                    this.node(&cluster).loading_topics = false;
                    cx.notify();
                })
                .ok();
                return;
            }
        })
        .detach();
    }

    fn refresh_cluster_topics(&mut self, cluster: &str, cx: &mut Context<Self>) {
        if self.node(cluster).refreshing_topics {
            overlay::toast_info(cx, t("topics.refreshingBg"));
            return;
        }
        self.node(cluster).refreshing_topics = true;
        cx.notify();
        let search = self
            .node(cluster)
            .topic_search
            .as_ref()
            .map(|i| i.read(cx).text().trim().to_string())
            .unwrap_or_default();
        let b = backend(cx);
        let cluster = cluster.to_string();
        if !search.is_empty() {
            overlay::toast_info(cx, format!("Refreshing topic \"{}\"...", search));
            let cluster2 = cluster.clone();
            let s = search.clone();
            let b_fire = b.clone();
            cx.spawn(async move |_, _| {
                let _ = b_fire
                    .dispatch(
                        "topic.refresh",
                        serde_json::json!({"cluster_id": cluster2, "topic_name": s}),
                    )
                    .await;
            })
            .detach();
        } else {
            overlay::toast_success(cx, t("topics.refreshingBg"));
            let cluster2 = cluster.clone();
            let b_fire = b.clone();
            cx.spawn(async move |_, _| {
                let _ = b_fire
                    .dispatch("topic.refresh", serde_json::json!({"cluster_id": cluster2}))
                    .await;
            })
            .detach();
        }
        // 500ms 后重新取 saved
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let saved = b
                .dispatch("topic.saved", serde_json::json!({"cluster_id": cluster}))
                .await;
            this.update(cx, |this, cx| {
                let topics: Vec<String> = saved
                    .ok()
                    .and_then(|v| v.get("topics").and_then(|x| x.as_array()).cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect();
                let node = this.node(&cluster);
                node.topics = Some(topics.clone());
                node.topic_count = Some(topics.len());
                node.refreshing_topics = false;
                node.topics_expanded = true;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ==================== Consumer Groups ====================

    fn toggle_cgs_folder(&mut self, cluster: &str, cx: &mut Context<Self>) {
        let expanded = {
            let node = self.node(cluster);
            node.cgs_expanded = !node.cgs_expanded;
            node.cgs_expanded
        };
        cx.notify();
        if expanded {
            self.load_cluster_cgs(cluster, cx);
        }
    }

    fn click_cgs_folder(&mut self, cluster: &str, cx: &mut Context<Self>) {
        if !self.node(cluster).cgs_expanded {
            self.node(cluster).cgs_expanded = true;
            self.load_cluster_cgs(cluster, cx);
        }
        cx.notify();
        let cluster = cluster.to_string();
        root(cx).update(cx, |app, cx| {
            let route = Route::new(Page::ConsumerGroups).with("cluster", &cluster);
            app.navigate(route, true, cx);
        });
    }

    fn load_cluster_cgs(&mut self, cluster: &str, cx: &mut Context<Self>) {
        if self.node(cluster).loading_cgs {
            return;
        }
        self.node(cluster).loading_cgs = true;
        cx.notify();
        let b = backend(cx);
        let cluster = cluster.to_string();
        cx.spawn(async move |this, cx| {
            let groups: Vec<String> = b
                .dispatch(
                    "consumer_group.saved",
                    serde_json::json!({"cluster_id": cluster}),
                )
                .await
                .ok()
                .and_then(|v| v.get("groups").and_then(|x| x.as_array()).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
            let groups = if groups.is_empty() {
                let _ = b
                    .dispatch(
                        "consumer_group.refresh",
                        serde_json::json!({"cluster_id": cluster}),
                    )
                    .await;
                b.dispatch(
                    "consumer_group.saved",
                    serde_json::json!({"cluster_id": cluster}),
                )
                .await
                .ok()
                .and_then(|v| v.get("groups").and_then(|x| x.as_array()).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
            } else {
                groups
            };
            this.update(cx, |this, cx| {
                let node = this.node(&cluster);
                node.cgs = Some(groups.clone());
                node.cg_count = Some(groups.len());
                node.loading_cgs = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn refresh_cluster_cgs(&mut self, cluster: &str, cx: &mut Context<Self>) {
        if self.node(cluster).refreshing_cgs {
            return;
        }
        self.node(cluster).refreshing_cgs = true;
        cx.notify();
        let search = self
            .node(cluster)
            .cg_search
            .as_ref()
            .map(|i| i.read(cx).text().trim().to_string())
            .unwrap_or_default();
        let b = backend(cx);
        let cluster = cluster.to_string();
        let params = if search.is_empty() {
            serde_json::json!({"cluster_id": cluster})
        } else {
            serde_json::json!({"cluster_id": cluster, "group_name": search})
        };
        overlay::toast_success(cx, t("consumerGroups.refreshingBg"));
        cx.spawn(async move |this, cx| {
            let _ = b.dispatch("consumer_group.refresh", params).await;
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let groups: Vec<String> = b
                .dispatch(
                    "consumer_group.saved",
                    serde_json::json!({"cluster_id": cluster}),
                )
                .await
                .ok()
                .and_then(|v| v.get("groups").and_then(|x| x.as_array()).cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
            this.update(cx, |this, cx| {
                let node = this.node(&cluster);
                node.cgs = Some(groups.clone());
                node.cg_count = Some(groups.len());
                node.refreshing_cgs = false;
                node.cgs_expanded = true;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ==================== 选择/跳转 ====================

    fn connect_if_needed(&mut self, cluster: &str, cx: &mut Context<Self>) {
        if self.node(cluster).health == Some(true) {
            return;
        }
        let b = backend(cx);
        let cluster = cluster.to_string();
        cx.spawn(async move |_, _cx| {
            let _ = b
                .dispatch(
                    "connection.reconnect",
                    serde_json::json!({"cluster_name": cluster}),
                )
                .await;
        })
        .detach();
    }

    fn select_topic(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        self.selected_topic = Some((topic.to_string(), cluster.to_string()));
        self.connect_if_needed(cluster, cx);
        cx.notify();
        let (cluster, topic) = (cluster.to_string(), topic.to_string());
        root(cx).update(cx, |app, cx| {
            let route = Route::new(Page::Messages)
                .with("cluster", &cluster)
                .with("topic", &topic);
            app.navigate(route, true, cx);
        });
    }

    fn select_cg(&mut self, cluster: &str, group: &str, cx: &mut Context<Self>) {
        self.connect_if_needed(cluster, cx);
        let (cluster, group) = (cluster.to_string(), group.to_string());
        root(cx).update(cx, |app, cx| {
            let route = Route::new(Page::ConsumerGroups)
                .with("cluster", &cluster)
                .with("group", &group);
            app.navigate(route, true, cx);
        });
    }

    /// 收藏/历史/TopicsView 跳转入口：展开集群+文件夹，加载并高亮 topic
    pub fn highlight_and_select_topic(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        {
            let node = self.node(cluster);
            node.expanded = true;
            node.topics_expanded = true;
        }
        self.selected_topic = Some((topic.to_string(), cluster.to_string()));
        self.load_cluster_topics(cluster, cx);
        cx.notify();
        let (cluster, topic) = (cluster.to_string(), topic.to_string());
        root(cx).update(cx, |app, cx| {
            let route = Route::new(Page::Messages)
                .with("cluster", &cluster)
                .with("topic", &topic);
            app.navigate(route, true, cx);
        });
    }

    /// TopicsView 双击行：在树中展开并选中（不导航）
    pub fn select_topic_in_tree(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        {
            let node = self.node(cluster);
            node.expanded = true;
            node.topics_expanded = true;
        }
        self.selected_topic = Some((topic.to_string(), cluster.to_string()));
        self.load_cluster_topics(cluster, cx);
        cx.notify();
    }

    fn collapse_all(&mut self, cx: &mut Context<Self>) {
        for node in self.nodes.values_mut() {
            node.expanded = false;
            node.topics_expanded = false;
        }
        cx.notify();
    }

    // ==================== 历史 ====================

    fn toggle_history(&mut self, cx: &mut Context<Self>) {
        self.show_history = !self.show_history;
        if self.show_history {
            self.load_history(cx);
        }
        cx.notify();
    }

    fn load_history(&mut self, cx: &mut Context<Self>) {
        self.history_loading = true;
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch("topic_history.list", serde_json::json!({"limit": 100}))
                .await;
            this.update(cx, |this, cx| {
                this.history_loading = false;
                if let Ok(v) = result {
                    this.history_items = v
                        .get("history")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    Some(TopicHistoryItem {
                                        id: item.get("id")?.as_i64()?,
                                        cluster_id: item.get("cluster_id")?.as_str()?.to_string(),
                                        cluster_name: item
                                            .get("cluster_name")
                                            .and_then(|x| x.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        topic_name: item.get("topic_name")?.as_str()?.to_string(),
                                        last_accessed_at: item
                                            .get("last_accessed_at")
                                            .and_then(|x| x.as_str())
                                            .map(|s| s.to_string()),
                                        access_count: item
                                            .get("access_count")
                                            .and_then(|x| x.as_i64())
                                            .unwrap_or(0),
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

    // ==================== 右键菜单 ====================

    fn cluster_context_menu(&mut self, cluster: &str, position: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        Self::open_cluster_menu_impl(cluster, position, cx);
    }

    /// 供 ClustersView 卡片右键复用
    pub fn open_cluster_menu(&mut self, cluster: &str, position: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        Self::open_cluster_menu_impl(cluster, position, cx);
    }

    fn open_cluster_menu_impl(cluster: &str, position: gpui::Point<gpui::Pixels>, cx: &mut App) {
        let name = cluster.to_string();
        let (n1, n2, n3, n4, n5, n6, n7, n8, n9) = (
            name.clone(),
            name.clone(),
            name.clone(),
            name.clone(),
            name.clone(),
            name.clone(),
            name.clone(),
            name.clone(),
            name.clone(),
        );
        overlay::open_context_menu(cx, overlay::ContextMenuState {
            position,
            title: Some(name.clone()),
            separators: vec![4, 8],
            items: vec![
                overlay::ContextItem {
                    label: t("contextMenu.testConnection"),
                    icon: Some("lightning"),
                    danger: false,
                    action: Box::new(move |cx| crate::dialogs::cluster_menu::test_connection(&n1, cx)),
                },
                overlay::ContextItem {
                    label: t("contextMenu.refreshStatus"),
                    icon: Some("refresh"),
                    danger: false,
                    action: Box::new(move |cx| crate::dialogs::cluster_menu::refresh_connection(&n2, cx)),
                },
                overlay::ContextItem {
                    label: t("contextMenu.disconnect"),
                    icon: Some("stop"),
                    danger: false,
                    action: Box::new(move |cx| crate::dialogs::cluster_menu::disconnect(&n3, cx)),
                },
                overlay::ContextItem {
                    label: t("contextMenu.reconnect"),
                    icon: Some("refresh"),
                    danger: false,
                    action: Box::new(move |cx| crate::dialogs::cluster_menu::reconnect(&n4, cx)),
                },
                overlay::ContextItem {
                    label: t("contextMenu.viewTopics"),
                    icon: Some("database"),
                    danger: false,
                    action: Box::new(move |cx| {
                        root(cx).update(cx, |app, cx| {
                            let route = Route::new(Page::Topics).with("cluster", &n5);
                            app.navigate(route, true, cx);
                        });
                    }),
                },
                overlay::ContextItem {
                    label: t("contextMenu.refreshTopics"),
                    icon: Some("refresh"),
                    danger: false,
                    action: Box::new(move |cx| {
                        overlay::toast_success(cx, t("topics.refreshingBg"));
                        let b = backend(cx);
                        let name = n6;
                        cx.spawn(async move |_cx| {
                            let _ = b
                                .dispatch("topic.refresh", serde_json::json!({"cluster_id": name}))
                                .await;
                        })
                        .detach();
                    }),
                },
                overlay::ContextItem {
                    label: t("contextMenu.createTopic"),
                    icon: Some("plus"),
                    danger: false,
                    action: Box::new(move |cx| {
                        root(cx).update(cx, |app, cx| {
                            let route = Route::new(Page::Topics)
                                .with("cluster", &n7)
                                .with("action", "create");
                            app.navigate(route, true, cx);
                        });
                    }),
                },
                overlay::ContextItem {
                    label: t("contextMenu.editCluster"),
                    icon: Some("pencil"),
                    danger: false,
                    action: Box::new(move |cx| {
                        root(cx).update(cx, |app, cx| {
                            let route = Route::new(Page::Clusters)
                                .with("action", "edit")
                                .with("cluster", &n8);
                            app.navigate(route, true, cx);
                        });
                    }),
                },
                overlay::ContextItem {
                    label: t("contextMenu.removeCluster"),
                    icon: Some("trash"),
                    danger: true,
                    action: Box::new(move |cx| crate::dialogs::cluster_menu::remove_cluster(&n9, cx)),
                },
            ],
        });
    }

    fn topic_context_menu(&mut self, cluster: &str, topic: &str, position: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        crate::dialogs::topic_menu::open(cx, position, cluster.to_string(), topic.to_string());
    }

    // ==================== 渲染 ====================

    fn render_topic_leaf(
        &self,
        cluster: &str,
        topic: &str,
        ix: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self
            .selected_topic
            .as_ref()
            .map(|(t, c)| t == topic && c == cluster)
            .unwrap_or(false);
        let cluster = cluster.to_string();
        let topic = topic.to_string();
        let (cluster_c, topic_c) = (cluster.clone(), topic.clone());
        let (cluster_r, topic_r) = (cluster.clone(), topic.clone());
        div()
            .id(("topic-leaf", ix))
            .flex()
            .items_center()
            .gap(px(5.))
            .h(px(28.))
            .pl(px(28.))
            .pr(px(8.))
            .mx(px(4.))
            .rounded(px(6.))
            .cursor_pointer()
            .when(selected, |d| {
                d.bg(theme::badge_primary_bg())
                    .text_color(theme::badge_primary_text())
            })
            .when(!selected, |d| d.text_color(theme::text_primary()))
            .hover(|s| {
                if selected {
                    s.bg(theme::badge_primary_bg())
                } else {
                    s.bg(theme::table_row_hover())
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.select_topic(&cluster_c, &topic_c, cx)
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    let pos = event.position;
                    this.topic_context_menu(&cluster_r, &topic_r, pos, cx);
                }),
            )
            .child(
                icon("database")
                    .size(px(12.))
                    .text_color(theme::badge_secondary_text()),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(11.))
                    .child(topic),
            )
    }

    fn render_cg_leaf(
        &self,
        cluster: &str,
        group: &str,
        ix: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let cluster = cluster.to_string();
        let group = group.to_string();
        let (cluster_c, group_c) = (cluster.clone(), group.clone());
        div()
            .id(("cg-leaf", ix))
            .flex()
            .items_center()
            .gap(px(5.))
            .h(px(28.))
            .pl(px(28.))
            .pr(px(8.))
            .mx(px(4.))
            .rounded(px(6.))
            .cursor_pointer()
            .text_color(theme::text_primary())
            .hover(|s| s.bg(theme::table_row_hover()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.select_cg(&cluster_c, &group_c, cx)),
            )
            .child(
                icon("users")
                    .size(px(12.))
                    .text_color(theme::badge_secondary_text()),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(11.))
                    .child(group),
            )
    }

    fn render_cluster_node(
        &mut self,
        cluster: &Cluster,
        ix: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let name = cluster.name.clone();
        let (n_row, n_menu, n_arrow, n_nav) = (
            cluster.name.clone(),
            cluster.name.clone(),
            cluster.name.clone(),
            cluster.name.clone(),
        );
        // 确保搜索输入实体存在
        if self.nodes.get(&name).and_then(|n| n.topic_search.as_ref()).is_none() {
            let input = cx.new(TextInput::new);
            input.update(cx, |i, _| i.set_placeholder(t("common.search")));
            cx.subscribe(&input, |_this, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::Changed) {
                    cx.notify();
                }
            })
            .detach();
            self.node(&name).topic_search = Some(input);
        }
        if self.nodes.get(&name).and_then(|n| n.cg_search.as_ref()).is_none() {
            let input = cx.new(TextInput::new);
            input.update(cx, |i, _| i.set_placeholder(t("common.search")));
            cx.subscribe(&input, |_this, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::Changed) {
                    cx.notify();
                }
            })
            .detach();
            self.node(&name).cg_search = Some(input);
        }

        let node_snapshot = ClusterNodeSnapshot {
            health: self.nodes.get(&name).and_then(|n| n.health),
            health_error: self.nodes.get(&name).and_then(|n| n.health_error.clone()),
            expanded: self.nodes.get(&name).map(|n| n.expanded).unwrap_or(false),
            topics_expanded: self.nodes.get(&name).map(|n| n.topics_expanded).unwrap_or(false),
            cgs_expanded: self.nodes.get(&name).map(|n| n.cgs_expanded).unwrap_or(false),
            topics: self.nodes.get(&name).and_then(|n| n.topics.clone()),
            cgs: self.nodes.get(&name).and_then(|n| n.cgs.clone()),
            topic_count: self.nodes.get(&name).and_then(|n| n.topic_count),
            cg_count: self.nodes.get(&name).and_then(|n| n.cg_count),
            loading_topics: self.nodes.get(&name).map(|n| n.loading_topics).unwrap_or(false),
            loading_cgs: self.nodes.get(&name).map(|n| n.loading_cgs).unwrap_or(false),
            refreshing_topics: self.nodes.get(&name).map(|n| n.refreshing_topics).unwrap_or(false),
            refreshing_cgs: self.nodes.get(&name).map(|n| n.refreshing_cgs).unwrap_or(false),
            topic_search_query: self
                .nodes
                .get(&name)
                .and_then(|n| n.topic_search.as_ref())
                .map(|i| i.read(cx).text().to_lowercase())
                .unwrap_or_default(),
            cg_search_query: self
                .nodes
                .get(&name)
                .and_then(|n| n.cg_search.as_ref())
                .map(|i| i.read(cx).text().to_lowercase())
                .unwrap_or_default(),
        };
        let topic_search_input = self
            .nodes
            .get(&name)
            .and_then(|n| n.topic_search.clone())
            .unwrap();
        let cg_search_input = self
            .nodes
            .get(&name)
            .and_then(|n| n.cg_search.clone())
            .unwrap();

        let health_color = if node_snapshot.refreshing_topics || node_snapshot.refreshing_cgs {
            theme::warning()
        } else {
            match node_snapshot.health {
                Some(true) => theme::health_ok(),
                Some(false) => theme::health_bad(),
                None => theme::warning(),
            }
        };

        let mut container = div().flex().flex_col();

        // 集群节点行
        container = container.child(
            div()
                .id(("cluster-node", ix))
                .flex()
                .items_center()
                .gap(px(6.))
                .px(px(8.))
                .py(px(6.))
                .mx(px(4.))
                .rounded(px(8.))
                .cursor_pointer()
                .when(node_snapshot.expanded, |d| d.bg(theme::badge_primary_bg()))
                .hover(|s| s.bg(theme::table_row_hover()))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        if event.click_count == 2 {
                            this.toggle_cluster(&n_row, cx);
                        }
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                        let pos = event.position;
                        this.cluster_context_menu(&n_menu, pos, cx);
                    }),
                )
                // 展开箭头
                .child(
                    div()
                        .id(("cluster-arrow", ix))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(16.))
                        .rounded(px(3.))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle_cluster(&n_arrow, cx)
                            }),
                        )
                        .child(
                            icon(if node_snapshot.expanded {
                                "chevron-down"
                            } else {
                                "chevron-right"
                            })
                            .size(px(11.))
                            .text_color(theme::text_secondary()),
                        ),
                )
                // 健康圆点
                .child(div().size(px(8.)).rounded(px(4.)).flex_none().bg(health_color))
                .child(
                    icon("server")
                        .size(px(14.))
                        .flex_none()
                        .text_color(theme::badge_primary_text()),
                )
                // 集群名（双击导航到 topics 页）
                .child(
                    div()
                        .id(("cluster-name", ix))
                        .flex_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_, event: &gpui::MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                if event.click_count == 2 {
                                    root(cx).update(cx, |app, cx| {
                                        let route = Route::new(Page::Topics).with("cluster", &n_nav);
                                        app.navigate(route, true, cx);
                                    });
                                }
                            }),
                        )
                        .child(cluster.name.clone()),
                ),
        );

        if !node_snapshot.expanded {
            return container;
        }

        // ---- Topics 文件夹行 ----
        let name_t = cluster.name.clone();
        let (name_t_arrow, name_t_create, name_t_refresh) = (
            cluster.name.clone(),
            cluster.name.clone(),
            cluster.name.clone(),
        );
        let topic_count_label = node_snapshot
            .topic_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".into());
        container = container.child(
            div()
                .id(("topics-folder", ix))
                .flex()
                .items_center()
                .gap(px(5.))
                .h(px(28.))
                .pl(px(16.))
                .pr(px(4.))
                .mx(px(4.))
                .rounded(px(6.))
                .cursor_pointer()
                .when(node_snapshot.topics_expanded, |d| {
                    d.bg(theme::badge_secondary_bg())
                })
                .hover(|s| s.bg(theme::table_row_hover()))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.click_topics_folder(&name_t, cx)
                    }),
                )
                .child(
                    div()
                        .id(("topics-folder-arrow", ix))
                        .flex_none()
                        .size(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle_topics_folder(&name_t_arrow, cx)
                            }),
                        )
                        .child(
                            icon(if node_snapshot.topics_expanded {
                                "chevron-down"
                            } else {
                                "chevron-right"
                            })
                            .size(px(10.))
                            .text_color(theme::text_secondary()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .child(topic_count_label),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(theme::text_secondary())
                        .child("Topics"),
                )
                // 创建 topic
                .child(
                    icon_btn(("tree-create-topic", ix), "plus", BtnSize::Xs)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_, _, _, cx| {
                                let view = cx.new(|cx| {
                                    crate::dialogs::create_topic::CreateTopicDialog::new(
                                        name_t_create.clone(),
                                        cx,
                                    )
                                });
                                overlay::open_modal(cx, view.into());
                            }),
                        ),
                )
                // 刷新
                .child(if node_snapshot.refreshing_topics {
                    div().size(px(24.)).flex().items_center().justify_center().child(spinner(12.)).into_any_element()
                } else {
                    div()
                        .id(("tree-refresh-topics", ix))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_color(theme::text_secondary())
                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.refresh_cluster_topics(&name_t_refresh, cx)
                            }),
                        )
                        .child(icon("refresh").size(px(12.)))
                        .into_any_element()
                }),
        );

        // Topics 子面板
        if node_snapshot.topics_expanded {
            let all_topics = node_snapshot.topics.clone().unwrap_or_default();
            let filtered: Vec<String> = if node_snapshot.topic_search_query.is_empty() {
                all_topics.clone()
            } else {
                all_topics
                    .iter()
                    .filter(|n| n.to_lowercase().contains(&node_snapshot.topic_search_query))
                    .cloned()
                    .collect()
            };
            let mut panel = div().flex().flex_col().pl(px(8.));
            // 搜索框
            if !all_topics.is_empty() {
                panel = panel.child(
                    div().px(px(20.)).py(px(2.)).child(
                        div()
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(theme::input_bg())
                            .border_1()
                            .border_color(theme::base_content_alpha(0.15))
                            .text_size(px(10.))
                            .child(topic_search_input),
                    ),
                );
                panel = panel.child(
                    div()
                        .px(px(24.))
                        .pb(px(2.))
                        .text_size(px(9.))
                        .text_color(if node_snapshot.topic_search_query.is_empty() {
                            theme::text_secondary()
                        } else {
                            theme::badge_primary_text()
                        })
                        .child(if node_snapshot.topic_search_query.is_empty() {
                            format!("{} topics", all_topics.len())
                        } else {
                            format!("{} matching", filtered.len())
                        }),
                );
            }
            if node_snapshot.loading_topics {
                panel = panel.child(
                    div()
                        .h(px(40.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(spinner(14.)),
                );
            }
            for (tix, topic) in filtered.iter().enumerate() {
                panel = panel.child(self.render_topic_leaf(&cluster.name, topic, ix * 100000 + tix, cx));
            }
            container = container.child(panel);
        }

        // ---- Consumer Groups 文件夹行 ----
        let name_c = cluster.name.clone();
        let (name_c_arrow, name_c_refresh) = (cluster.name.clone(), cluster.name.clone());
        let cg_count_label = node_snapshot
            .cg_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".into());
        container = container.child(
            div()
                .id(("cgs-folder", ix))
                .flex()
                .items_center()
                .gap(px(5.))
                .h(px(28.))
                .pl(px(16.))
                .pr(px(4.))
                .mx(px(4.))
                .rounded(px(6.))
                .cursor_pointer()
                .when(node_snapshot.cgs_expanded, |d| {
                    d.bg(theme::badge_secondary_bg())
                })
                .hover(|s| s.bg(theme::table_row_hover()))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| this.click_cgs_folder(&name_c, cx)),
                )
                .child(
                    div()
                        .id(("cgs-folder-arrow", ix))
                        .flex_none()
                        .size(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle_cgs_folder(&name_c_arrow, cx)
                            }),
                        )
                        .child(
                            icon(if node_snapshot.cgs_expanded {
                                "chevron-down"
                            } else {
                                "chevron-right"
                            })
                            .size(px(10.))
                            .text_color(theme::text_secondary()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .child(cg_count_label),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(theme::text_secondary())
                        .child("Consumer Groups"),
                )
                .child(if node_snapshot.refreshing_cgs {
                    div().size(px(24.)).flex().items_center().justify_center().child(spinner(12.)).into_any_element()
                } else {
                    div()
                        .id(("tree-refresh-cgs", ix))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_color(theme::text_secondary())
                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.refresh_cluster_cgs(&name_c_refresh, cx)
                            }),
                        )
                        .child(icon("refresh").size(px(12.)))
                        .into_any_element()
                }),
        );

        if node_snapshot.cgs_expanded {
            let all_cgs = node_snapshot.cgs.clone().unwrap_or_default();
            let filtered: Vec<String> = if node_snapshot.cg_search_query.is_empty() {
                all_cgs.clone()
            } else {
                all_cgs
                    .iter()
                    .filter(|n| n.to_lowercase().contains(&node_snapshot.cg_search_query))
                    .cloned()
                    .collect()
            };
            let mut panel = div().id("auto_views_cluster_tree_rs_101").flex().flex_col().pl(px(8.)).max_h(px(500.)).overflow_y_scroll();
            if !all_cgs.is_empty() {
                panel = panel.child(
                    div().px(px(20.)).py(px(2.)).child(
                        div()
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(theme::input_bg())
                            .border_1()
                            .border_color(theme::base_content_alpha(0.15))
                            .text_size(px(10.))
                            .child(cg_search_input),
                    ),
                );
            }
            if node_snapshot.loading_cgs {
                panel = panel.child(
                    div()
                        .h(px(40.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(spinner(14.)),
                );
            }
            for (gix, group) in filtered.iter().enumerate() {
                panel = panel.child(self.render_cg_leaf(&cluster.name, group, ix * 100000 + gix, cx));
            }
            container = container.child(panel);
        }

        container
    }
}

struct ClusterNodeSnapshot {
    health: Option<bool>,
    #[allow(dead_code)]
    health_error: Option<String>,
    expanded: bool,
    topics_expanded: bool,
    cgs_expanded: bool,
    topics: Option<Vec<String>>,
    cgs: Option<Vec<String>>,
    topic_count: Option<usize>,
    cg_count: Option<usize>,
    loading_topics: bool,
    loading_cgs: bool,
    refreshing_topics: bool,
    refreshing_cgs: bool,
    topic_search_query: String,
    cg_search_query: String,
}

impl Focusable for ClusterTreeNavigator {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ClusterTreeNavigator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.backend_seen_ready && backend(cx).is_ready() {
            self.backend_seen_ready = true;
            let existing: HashSet<String> = root(cx)
                .read(cx)
                .clusters
                .iter()
                .map(|c| c.name.clone())
                .collect();
            let groups = root(cx).read(cx).groups.clone();
            self.on_clusters_changed(existing, groups, cx);
        }

        let app = root(cx);
        let app = app.read(cx);
        let groups = app.groups.clone();
        let clusters: Vec<Cluster> = app
            .clusters
            .iter()
            .filter(|c| match self.selected_group_id {
                None => true,
                Some(gid) => c.group_id == Some(gid),
            })
            .cloned()
            .collect();
        let selected_group_id = self.selected_group_id;

        // 当前路由高亮
        if app.route.page == Page::Messages {
            if let (Some(c), Some(t)) = (app.route.get("cluster"), app.route.get("topic")) {
                self.selected_topic = Some((t.to_string(), c.to_string()));
            }
        }

        let history_search = self.history_search_input.read(cx).text().to_lowercase();
        let history_items: Vec<TopicHistoryItem> = self
            .history_items
            .iter()
            .filter(|i| history_search.is_empty() || i.topic_name.to_lowercase().contains(&history_search))
            .cloned()
            .collect();
        let history_loading = self.history_loading;

        div()
            .flex()
            .flex_col()
            .size_full()
            // 双击空白区域 → 全部收起（对齐 ModernLayout）
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                    if event.click_count == 2 {
                        this.collapse_all(cx);
                    }
                }),
            )
            // 头部
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .px(px(8.))
                    .h(px(40.))
                    .border_b_1()
                    .border_color(theme::border_base_200())
                    .child(
                        icon("server")
                            .size(px(15.))
                            .text_color(theme::badge_primary_text()),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme::text_secondary())
                            .child(if self.show_history {
                                t("topicHistory.title").to_uppercase()
                            } else {
                                t("tree.clusters").to_uppercase()
                            }),
                    )
                    .child(div().flex_1())
                    .when(!self.show_history, |d| {
                        d.child(
                            icon_btn("tree-collapse", "chevron-up", BtnSize::Xs)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| this.collapse_all(cx)),
                                ),
                        )
                    })
                    .child(
                        icon_btn("tree-clusters", "server", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| {
                                root(cx).update(cx, |app, cx| {
                                    app.navigate(Route::new(Page::Clusters), true, cx)
                                });
                            }),
                        ),
                    )
                    .child(
                        icon_btn("tree-favorites", "star", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| {
                                root(cx).update(cx, |app, cx| {
                                    app.navigate(Route::new(Page::Favorites), true, cx)
                                });
                            }),
                        ),
                    )
                    .child(
                        div()
                            .id("tree-history")
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .text_color(if self.show_history {
                                theme::badge_primary_text()
                            } else {
                                theme::text_secondary()
                            })
                            .when(self.show_history, |d| d.bg(theme::badge_primary_bg()))
                            .hover(|s| s.bg(theme::btn_ghost_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.toggle_history(cx)),
                            )
                            .child(icon("clock").size(px(13.))),
                    ),
            )
            // 分组选择器（历史模式隐藏，有分组才显示）
            .when(!self.show_history && !groups.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .px(px(8.))
                        .h(px(32.))
                        .border_b_1()
                        .border_color(theme::border_base_200())
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(theme::text_secondary())
                                .flex_none()
                                .child(format!("{}:", t("clusters.group"))),
                        )
                        .child(
                            div()
                                .id("tree-groups-scroll")
                                .flex_1()
                                .flex()
                                .gap(px(4.))
                                .overflow_x_scroll()
                                .track_scroll(&self.groups_scroll)
                                .child(
                                    div()
                                        .id("tree-group-all")
                                        .flex_none()
                                        .px(px(8.))
                                        .h(px(20.))
                                        .flex()
                                        .items_center()
                                        .rounded(px(5.))
                                        .text_size(px(10.))
                                        .cursor_pointer()
                                        .when(selected_group_id.is_none(), |d| {
                                            d.bg(theme::badge_primary_bg())
                                                .text_color(theme::badge_primary_text())
                                        })
                                        .when(selected_group_id.is_some(), |d| {
                                            d.text_color(theme::text_primary())
                                        })
                                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.select_group(None, cx)
                                            }),
                                        )
                                        .child(t("common.all")),
                                )
                                .children(groups.into_iter().enumerate().map(|(gix, g)| {
                                    let gid = g.id;
                                    let active = selected_group_id == Some(gid);
                                    div()
                                        .id(("tree-group", gix))
                                        .flex_none()
                                        .px(px(8.))
                                        .h(px(20.))
                                        .flex()
                                        .items_center()
                                        .rounded(px(5.))
                                        .text_size(px(10.))
                                        .cursor_pointer()
                                        .when(active, |d| {
                                            d.bg(theme::badge_primary_bg())
                                                .text_color(theme::badge_primary_text())
                                        })
                                        .when(!active, |d| d.text_color(theme::text_primary()))
                                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.select_group(Some(gid), cx)
                                            }),
                                        )
                                        .child(g.name)
                                })),
                        ),
                )
            })
            // 主区域
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(self.show_history, |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_col()
                                .size_full()
                                .child(
                                    div().px(px(8.)).py(px(4.)).child(
                                        div()
                                            .h(px(26.))
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
                                .child(
                                    div().id("views_cluster_tree_rs_1")
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .when(history_loading, |d| {
                                            d.child(loading_block(t("common.loading")))
                                        })
                                        .when(!history_loading && history_items.is_empty(), |d| {
                                            d.child(empty_block(
                                                "clock",
                                                t("topicHistory.empty"),
                                                t("topicHistory.emptyDesc"),
                                            ))
                                        })
                                        .children(history_items.into_iter().enumerate().map(
                                            |(hix, item)| {
                                                let cluster = item.cluster_id.clone();
                                                let topic = item.topic_name.clone();
                                                div()
                                                    .id(("tree-history-item", hix))
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .px(px(8.))
                                                    .py(px(5.))
                                                    .cursor_pointer()
                                                    .hover(|s| {
                                                        s.bg(theme::context_menu_item_hover())
                                                    })
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, _, cx| {
                                                            this.show_history = false;
                                                            this.highlight_and_select_topic(
                                                                &cluster, &topic, cx,
                                                            );
                                                        }),
                                                    )
                                                    .child(
                                                        icon("database")
                                                            .size(px(13.))
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
                                                                    .text_size(px(11.))
                                                                    .text_color(
                                                                        theme::text_primary(),
                                                                    )
                                                                    .whitespace_nowrap()
                                                                    .overflow_hidden()
                                                                    .child(item.topic_name.clone()),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(9.))
                                                                    .text_color(
                                                                        theme::text_secondary(),
                                                                    )
                                                                    .child(format!(
                                                                        "{} · {}",
                                                                        item.cluster_name,
                                                                        relative_time(
                                                                            item.last_accessed_at
                                                                                .as_deref()
                                                                        )
                                                                    )),
                                                            ),
                                                    )
                                            },
                                        )),
                                ),
                        )
                    })
                    .when(!self.show_history, |d| {
                        d.child(
                            div().id("views_cluster_tree_rs_3")
                                .size_full()
                                .overflow_y_scroll()
                                .track_scroll(&self.scroll)
                                .py(px(4.))
                                .when(clusters.is_empty(), |d| {
                                    d.child(empty_block(
                                        "server",
                                        t("common.noData"),
                                        t("clusters.emptyDesc"),
                                    ))
                                })
                                .children(clusters.iter().enumerate().map(|(ix, cluster)| {
                                    self.render_cluster_node(cluster, ix, cx)
                                })),
                        )
                    }),
            )
    }
}
