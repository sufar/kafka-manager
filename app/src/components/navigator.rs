//! 平铺导航器：与旧版 Vue TopicNavigator 行为一致
//!
//! ┌────────────────────────────┐
//! │ [视图切换▾] [集群][收藏][SR][历史] │
//! │ Cluster: [选择器▾] N/total [刷新] │
//! │ [搜索框 ✕]                  │
//! │ 虚拟列表（Topic / 消费组）  │
//! └────────────────────────────┘
//! 集群选择器：双栏弹层（分组 | 集群，多选 + Clear/Apply，持久化 ui.selected_clusters）
//! 分页：limit 1000 + has_more + 底部"加载更多"；刷新后轮询 refresh.status 直至完成

use std::collections::HashSet;
use std::rc::Rc;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::*;
use serde_json::json;

use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::state::{Backend, Page, TokioRuntime};

/// 导航器发给工作区的事件
#[derive(Clone, Debug)]
pub enum NavEvent {
    /// 打开消息页并预选集群 + Topic
    OpenMessages { cluster: String, topic: String },
    /// 打开 Topics 页并预选集群
    OpenTopics { cluster: String },
    /// 打开消费组页并预选集群（可带组名进入详情）
    OpenConsumerGroups { cluster: String, group: Option<String> },
    /// 打开 Topic 消费组视图
    OpenTopicConsumerGroups { cluster: String, topic: String },
    /// 切换到指定页面
    OpenPage(Page),
}

const PAGE_LIMIT: usize = 1000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum NavView {
    Topics,
    ConsumerGroups,
}

#[derive(Clone, Debug)]
struct TopicItem {
    name: String,
    cluster: String,
}

#[derive(Clone, Debug)]
struct GroupItem {
    name: String,
    cluster: String,
}

#[derive(Clone, Debug)]
struct ClusterInfo {
    name: String,
    group_id: Option<i64>,
}

#[derive(Clone, Debug)]
struct HistoryItem {
    cluster: String,
    topic: String,
}

/// 行渲染所需的主题色
#[derive(Clone, Copy)]
struct RowColors {
    hover: Hsla,
    active: Hsla,
    secondary: Hsla,
    muted: Hsla,
    warning: Hsla,
}

impl RowColors {
    fn from_cx(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            hover: theme.list_hover,
            active: theme.list_active,
            secondary: theme.secondary,
            muted: theme.muted_foreground,
            warning: theme.warning,
        }
    }
}

pub struct Navigator {
    view: NavView,
    view_state: Entity<SelectState<SearchableVec<StringOption>>>,
    // 集群多选状态
    clusters: Vec<ClusterInfo>,
    groups_meta: Vec<(i64, String)>,
    selected_clusters: HashSet<String>,
    selected_groups: HashSet<i64>,
    selector_open: bool,
    active_group: Option<i64>,
    // 数据
    topics: Vec<TopicItem>,
    topics_total: usize,
    topics_has_more: bool,
    cg_items: Vec<GroupItem>,
    cg_total: usize,
    cg_has_more: bool,
    loading: bool,
    refreshing: bool,
    // 选中项
    selected_topic: Option<(String, String)>,
    favorites: HashSet<String>,
    // 搜索 + 历史
    search_input: Entity<InputState>,
    show_history: bool,
    history: Vec<HistoryItem>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NavEvent> for Navigator {}

impl Navigator {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let view_state = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(vec![
                    StringOption::new("Topics", "topics"),
                    StringOption::new("Consumer Groups", "consumer-groups"),
                ]),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "common.search"))
        });

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &view_state,
            |this, _, event: &SelectEvent<SearchableVec<StringOption>>, cx| {
                if matches!(event, SelectEvent::Confirm(Some(_))) {
                    this.view = if this
                        .view_state
                        .read(cx)
                        .selected_value()
                        .map(|v| v.as_ref() == "consumer-groups")
                        .unwrap_or(false)
                    {
                        NavView::ConsumerGroups
                    } else {
                        NavView::Topics
                    };
                    // 切换视图时清空搜索（与旧版一致）
                    this.clear_search(cx);
                    cx.notify();
                    this.load_data(cx);
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.schedule_search(cx);
                }
            },
        ));

        let this = Self {
            view: NavView::Topics,
            view_state,
            clusters: Vec::new(),
            groups_meta: Vec::new(),
            selected_clusters: HashSet::new(),
            selected_groups: HashSet::new(),
            selector_open: false,
            active_group: None,
            topics: Vec::new(),
            topics_total: 0,
            topics_has_more: false,
            cg_items: Vec::new(),
            cg_total: 0,
            cg_has_more: false,
            loading: true,
            refreshing: false,
            selected_topic: None,
            favorites: HashSet::new(),
            search_input,
            show_history: false,
            history: Vec::new(),
            _subscriptions: subscriptions,
        };
        this.init(cx);
        this
    }

    /// 初始化：加载集群、分组、收藏、保存的集群选择
    fn init(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let clusters = crate::service::call(&rt, state.clone(), "cluster.list", json!({})).await;
            let groups = crate::service::call(&rt, state.clone(), "cluster_group.list", json!({})).await;
            let favorites = crate::service::call(&rt, state.clone(), "favorite.list", json!({})).await;
            let saved = crate::service::call(
                &rt,
                state,
                "settings.get",
                json!({ "keys": ["ui.selected_clusters"] }),
            )
            .await;

            this.update(cx, |this, cx| {
                if let Ok(value) = clusters {
                    this.clusters = value
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|c| {
                                    Some(ClusterInfo {
                                        name: c.get("name")?.as_str()?.to_string(),
                                        group_id: c.get("group_id").and_then(|v| v.as_i64()),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                }
                if let Ok(value) = groups {
                    this.groups_meta = value
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
                // 恢复保存的集群选择
                if let Ok(value) = saved {
                    if let Some(json_str) = value
                        .get("settings")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|s| s.get("value"))
                        .and_then(|v| v.as_str())
                    {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(clusters) = parsed.get("clusters").and_then(|c| c.as_array()) {
                                this.selected_clusters = clusters
                                    .iter()
                                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                                    .collect();
                            }
                            if let Some(groups) = parsed.get("groups").and_then(|g| g.as_array()) {
                                this.selected_groups = groups
                                    .iter()
                                    .filter_map(|g| g.as_i64())
                                    .collect();
                            }
                        }
                    }
                }
                this.loading = false;
                cx.notify();
                this.load_data(cx);
            })
            .ok();
        })
        .detach();
    }

    fn save_cluster_selection(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let selection = json!({
            "clusters": self.selected_clusters.iter().collect::<Vec<_>>(),
            "groups": self.selected_groups.iter().collect::<Vec<_>>(),
        });
        cx.spawn(async move |_this, _cx| {
            let _ = crate::service::call(
                &rt,
                state,
                "settings.update",
                json!({ "key": "ui.selected_clusters", "value": selection.to_string() }),
            )
            .await;
        })
        .detach();
    }

    /// 选中的集群名列表（空选择 = 全部集群）
    fn all_selected_cluster_names(&self) -> Vec<String> {
        if self.selected_clusters.is_empty() {
            if let Some(gid) = self.active_group.filter(|g| *g != 0) {
                let group_clusters: Vec<String> = self
                    .clusters
                    .iter()
                    .filter(|c| c.group_id == Some(gid))
                    .map(|c| c.name.clone())
                    .collect();
                if !group_clusters.is_empty() {
                    return group_clusters;
                }
            }
            self.clusters.iter().map(|c| c.name.clone()).collect()
        } else {
            self.selected_clusters.iter().cloned().collect()
        }
    }

    /// 选择器按钮摘要文本
    fn selector_summary(&self, cx: &App) -> String {
        if self.selected_clusters.is_empty() {
            t(cx, "navigator.allClusters")
        } else if self.selected_clusters.len() == 1 {
            self.selected_clusters.iter().next().unwrap().clone()
        } else {
            format!("{} clusters", self.selected_clusters.len())
        }
    }

    fn clusters_of_group(&self, group_id: Option<i64>) -> Vec<&ClusterInfo> {
        match group_id {
            None | Some(0) => self.clusters.iter().collect(),
            Some(gid) => self
                .clusters
                .iter()
                .filter(|c| c.group_id == Some(gid))
                .collect(),
        }
    }

    fn is_group_fully_selected(&self, group_id: i64) -> bool {
        let group_clusters = self.clusters_of_group(Some(group_id));
        !group_clusters.is_empty()
            && group_clusters
                .iter()
                .all(|c| self.selected_clusters.contains(&c.name))
    }

    fn toggle_group_full(&mut self, group_id: i64, cx: &mut Context<Self>) {
        let names: Vec<String> = self
            .clusters_of_group(Some(group_id))
            .iter()
            .map(|c| c.name.clone())
            .collect();
        if self.is_group_fully_selected(group_id) {
            for name in names {
                self.selected_clusters.remove(&name);
            }
            self.selected_groups.remove(&group_id);
        } else {
            for name in names {
                self.selected_clusters.insert(name);
            }
            self.selected_groups.insert(group_id);
        }
        cx.notify();
    }

    fn toggle_cluster_selection(&mut self, name: String, group_id: Option<i64>, cx: &mut Context<Self>) {
        if self.selected_clusters.contains(&name) {
            self.selected_clusters.remove(&name);
            if let Some(gid) = group_id {
                self.selected_groups.remove(&gid);
            }
        } else {
            self.selected_clusters.insert(name);
        }
        cx.notify();
    }

    fn clear_all_selections(&mut self, cx: &mut Context<Self>) {
        self.selected_clusters.clear();
        self.selected_groups.clear();
        self.apply_cluster_selection(cx);
    }

    fn apply_cluster_selection(&mut self, cx: &mut Context<Self>) {
        self.selector_open = false;
        self.clear_search(cx);
        self.save_cluster_selection(cx);
        cx.notify();
        self.load_data(cx);
    }

    /// 防抖搜索（300ms，与旧版一致）
    fn schedule_search(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;
            this.update(cx, |this, cx| this.load_data(cx)).ok();
        })
        .detach();
    }

    fn clear_search(&mut self, cx: &mut Context<Self>) {
        let value = self.search_input.read(cx).value().to_string();
        if !value.is_empty() {
            if let Some(w) = cx.windows().first() {
                let _ = w.update(cx, |_, window, cx| {
                    self.search_input.update(cx, |state, cx| {
                        state.set_value(String::new(), window, cx);
                    });
                });
            }
        }
    }

    fn search_text(&self, cx: &App) -> String {
        self.search_input.read(cx).value().to_string()
    }

    fn load_data(&self, cx: &mut Context<Self>) {
        match self.view {
            NavView::Topics => self.load_topics(false, cx),
            NavView::ConsumerGroups => self.load_consumer_groups(false, cx),
        }
    }

    /// 加载 Topics（load_more=true 时分页追加）
    fn load_topics(&self, load_more: bool, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let clusters = self.all_selected_cluster_names();
        let search = self.search_text(cx);
        let offset = if load_more { self.topics.len() } else { 0 };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "topic.list_with_cluster",
                json!({
                    "cluster_ids": clusters,
                    "offset": offset,
                    "limit": PAGE_LIMIT,
                    "search": search,
                }),
            )
            .await;

            this.update(cx, |this, cx| {
                this.loading = false;
                match &result {
                    Ok(value) => {
                        tracing::info!(
                            "[Navigator] topics response: total={:?}, topics_len={:?}",
                            value.get("total"),
                            value.get("topics").and_then(|t| t.as_array()).map(|a| a.len())
                        );
                    }
                    Err(e) => {
                        tracing::warn!("[Navigator] topics request failed: {}", e);
                    }
                }
                if let Ok(value) = result {
                    let mut new_topics: Vec<TopicItem> = value
                        .get("topics")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|t| {
                                    Some(TopicItem {
                                        name: t.get("name")?.as_str()?.to_string(),
                                        cluster: t
                                            .get("cluster")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or_default()
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    this.topics_total = value
                        .get("total")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    this.topics_has_more = value
                        .get("has_more")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if load_more {
                        this.topics.append(&mut new_topics);
                    } else {
                        // 无搜索词时按 cluster + name 排序（与旧版一致）
                        if search.trim().is_empty() {
                            new_topics.sort_by(|a, b| {
                                a.cluster
                                    .cmp(&b.cluster)
                                    .then_with(|| a.name.cmp(&b.name))
                            });
                        }
                        this.topics = new_topics;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_consumer_groups(&self, load_more: bool, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let clusters = self.all_selected_cluster_names();
        let search = self.search_text(cx);
        let offset = if load_more { self.cg_items.len() } else { 0 };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "consumer_group.list",
                json!({
                    "cluster_ids": clusters,
                    "offset": offset,
                    "limit": PAGE_LIMIT,
                    "search": search,
                }),
            )
            .await;

            this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(value) = result {
                    let mut new_items: Vec<GroupItem> = value
                        .get("groups")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|g| {
                                    Some(GroupItem {
                                        name: g.get("group_name")?.as_str()?.to_string(),
                                        cluster: g
                                            .get("cluster_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or_default()
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    this.cg_total = value
                        .get("total")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    this.cg_has_more = value
                        .get("has_more")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if load_more {
                        this.cg_items.append(&mut new_items);
                    } else {
                        this.cg_items = new_items;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 刷新：对所有选中集群 fire-and-forget，然后轮询 refresh.status 直至完成
    fn refresh_all(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        cx.notify();

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let clusters = self.all_selected_cluster_names();
        let is_topics = self.view == NavView::Topics;

        cx.spawn(async move |this, cx| {
            // fire-and-forget 刷新所有选中集群
            for cluster in &clusters {
                let method = if is_topics { "topic.refresh" } else { "consumer_group.refresh" };
                let state_c = state.clone();
                let cluster_c = cluster.clone();
                let rt_for_fut = rt.clone();
                let fut = async move {
                    crate::service::call(
                        &rt_for_fut,
                        state_c,
                        method,
                        json!({ "cluster_id": cluster_c }),
                    )
                    .await
                };
                rt.spawn(fut);
            }

            // 轮询等待刷新完成（最多 120s）
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                if std::time::Instant::now() > deadline {
                    break;
                }
                let status = crate::service::call(&rt, state.clone(), "refresh.status", json!({})).await;
                let still: Vec<String> = status
                    .ok()
                    .and_then(|v| {
                        v.get("refreshing_clusters")
                            .and_then(|r| r.as_array())
                            .cloned()
                    })
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect();
                if still.is_empty() || !still.iter().any(|c| clusters.contains(c)) {
                    break;
                }
            }

            this.update(cx, |this, cx| {
                this.refreshing = false;
                cx.notify();
                this.load_data(cx);
            })
            .ok();
        })
        .detach();
    }

    fn open_topic(&mut self, cluster: String, topic: String, cx: &mut Context<Self>) {
        self.selected_topic = Some((cluster.clone(), topic.clone()));
        cx.notify();
        cx.emit(NavEvent::OpenMessages { cluster, topic });
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

    /// 头部：视图切换 + 图标按钮
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .items_center()
            .justify_between()
            .p_1p5()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .w(px(150.0))
                    .child(Select::new(&self.view_state).small()),
            )
            .child(
                h_flex()
                    .gap_0p5()
                    .child(self.nav_button(
                        "nav-clusters",
                        IconName::Building2,
                        t(cx, "nav.clusters"),
                        Page::Clusters,
                        cx,
                    ))
                    .child(self.nav_button(
                        "nav-favorites",
                        IconName::Star,
                        t(cx, "nav.favorites"),
                        Page::Favorites,
                        cx,
                    ))
                    .child(self.nav_button(
                        "nav-schema",
                        IconName::BookOpen,
                        t(cx, "tree.schemaRegistry"),
                        Page::SchemaRegistry,
                        cx,
                    ))
                    .child(
                        Button::new("nav-history")
                            .ghost()
                            .icon(IconName::Calendar)
                            .tooltip(t(cx, "history.title"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_history(cx);
                            })),
                    ),
            )
    }

    /// 状态栏：Cluster: [选择器] 计数 [刷新]
    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let summary = self.selector_summary(cx);
        let count_text = match self.view {
            NavView::Topics => format!("{} / {} topics", self.topics.len(), self.topics_total),
            NavView::ConsumerGroups => {
                format!("{} / {} consumer groups", self.cg_items.len(), self.cg_total)
            }
        };

        h_flex()
            .items_center()
            .gap_1()
            .px_1p5()
            .py_1()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .flex_shrink_0()
                    .child("Cluster:"),
            )
            .child(
                Button::new("cluster-selector")
                    .ghost()
                    .xsmall()
                    .label(summary)
                    .icon(IconName::ChevronDown)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.selector_open = !this.selector_open;
                        this.active_group = None;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(count_text),
            )
            .child(
                Button::new("nav-refresh")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Redo2)
                    .loading(self.refreshing)
                    .tooltip(t(cx, "common.refresh"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.refresh_all(cx);
                    })),
            )
    }

    /// 集群选择器弹层（双栏：分组 | 集群，多选 + Clear/Apply）
    fn render_cluster_selector(&self, cx: &mut Context<Self>) -> AnyElement {
        if !self.selector_open {
            return div().into_any_element();
        }
        let theme = cx.theme();

        // 左栏：全部集群 + 分组
        let mut left_items: Vec<AnyElement> = vec![h_flex()
            .items_center()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                Checkbox::new("sel-all")
                    .checked(self.selected_clusters.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.selected_clusters.clear();
                        this.selected_groups.clear();
                        this.active_group = None;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .child(t(cx, "navigator.allClusters")),
            )
            .into_any_element()];

        for (gid, gname) in &self.groups_meta {
            let gid = *gid;
            let gname = gname.clone();
            let fully = self.is_group_fully_selected(gid);
            let active = self.active_group == Some(gid);
            left_items.push(
                h_flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .when(active, |el| el.bg(theme.list_active))
                    .child(
                        Checkbox::new(("sel-group", gid as usize))
                            .checked(fully)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_group_full(gid, cx);
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .overflow_hidden()
                            .child(gname.clone())
                            .id(("sel-group-name", gid as usize))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.active_group = Some(gid);
                                cx.notify();
                            })),
                    )
                    .into_any_element(),
            );
        }

        // 右栏：集群
        let right_clusters: Vec<AnyElement> = self
            .clusters_of_group(self.active_group)
            .iter()
            .map(|c| {
                let name = c.name.clone();
                let gid = c.group_id;
                let checked = self.selected_clusters.contains(&c.name);
                h_flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        Checkbox::new(SharedString::from(format!("sel-cluster-{}", name)))
                            .checked(checked)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_cluster_selection(name.clone(), gid, cx);
                            })),
                    )
                    .child(div().text_xs().overflow_hidden().child(c.name.clone()))
                    .into_any_element()
            })
            .collect();

        div()
            .absolute()
            .top(px(76.0))
            .left(px(6.0))
            .w(px(360.0))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_lg()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.selector_open = false;
                cx.notify();
            }))
            .child(
                h_flex()
                    .h(px(280.0))
                    .child(
                        v_flex()
                            .w_1_2()
                            .border_r_1()
                            .border_color(theme.border)
                            .child(
                                div()
                                    .p_2()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.muted_foreground)
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(t(cx, "navigator.groups")),
                            )
                            .child(
                                div()
                                    .id("selector-groups")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .children(left_items),
                            ),
                    )
                    .child(
                        v_flex()
                            .w_1_2()
                            .child(
                                div()
                                    .p_2()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.muted_foreground)
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(t(cx, "navigator.clusters")),
                            )
                            .child(
                                div()
                                    .id("selector-clusters")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .children(right_clusters),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        Button::new("sel-clear")
                            .ghost()
                            .xsmall()
                            .label(t(cx, "common.clear"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.clear_all_selections(cx);
                            })),
                    )
                    .child(
                        Button::new("sel-apply")
                            .primary()
                            .xsmall()
                            .label(t(cx, "common.apply"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_cluster_selection(cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_topic_row(
        entity: Entity<Navigator>,
        ix: usize,
        item: TopicItem,
        is_fav: bool,
        is_selected: bool,
        colors: &RowColors,
    ) -> AnyElement {
        let item_open = item.clone();
        let item_badge = item.clone();
        let entity_badge = entity.clone();

        h_flex()
            .items_center()
            .gap_1p5()
            .px_1p5()
            .py_1()
            .rounded_md()
            .w_full()
            .cursor_pointer()
            .hover(|el| el.bg(colors.hover))
            .when(is_selected, |el| el.bg(colors.active))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .text_xs()
                    .whitespace_nowrap()
                    .child(item.name.clone())
                    .id(("topic", ix))
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |this, cx| {
                            this.open_topic(item_open.cluster.clone(), item_open.name.clone(), cx);
                        });
                    }),
            )
            .when(is_fav, |el| {
                el.child(Icon::new(IconName::Star).size_3().text_color(colors.warning))
            })
            .child(
                div()
                    .id(("badge", ix))
                    .cursor_pointer()
                    .text_xs()
                    .px_1()
                    .rounded_md()
                    .bg(colors.secondary)
                    .text_color(colors.muted)
                    .max_w(px(80.0))
                    .overflow_hidden()
                    .child(item.cluster.clone())
                    .on_click(move |_, _, cx| {
                        entity_badge.update(cx, |_this, cx| {
                            cx.emit(NavEvent::OpenTopics {
                                cluster: item_badge.cluster.clone(),
                            });
                        });
                    }),
            )
            .into_any_element()
    }

    fn render_group_row(
        entity: Entity<Navigator>,
        ix: usize,
        item: GroupItem,
        colors: &RowColors,
    ) -> AnyElement {
        let item_open = item.clone();

        h_flex()
            .items_center()
            .gap_1p5()
            .px_1p5()
            .py_1()
            .rounded_md()
            .w_full()
            .cursor_pointer()
            .hover(|el| el.bg(colors.hover))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .text_xs()
                    .whitespace_nowrap()
                    .child(item.name.clone())
                    .id(("group", ix))
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |_this, cx| {
                            cx.emit(NavEvent::OpenConsumerGroups {
                                cluster: item_open.cluster.clone(),
                                group: Some(item_open.name.clone()),
                            });
                        });
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .px_1()
                    .rounded_md()
                    .bg(colors.secondary)
                    .text_color(colors.muted)
                    .max_w(px(80.0))
                    .overflow_hidden()
                    .child(item.cluster.clone()),
            )
            .into_any_element()
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

impl Render for Navigator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (sidebar_bg, border_c, muted_c) = {
            let theme = cx.theme();
            (theme.sidebar, theme.border, theme.muted_foreground)
        };

        let search_bar = div()
            .px_1p5()
            .py_1()
            .border_b_1()
            .border_color(border_c)
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .child(div().flex_1().child(Input::new(&self.search_input).small()))
                    .when(!self.search_text(cx).is_empty(), |el| {
                        el.child(
                            Button::new("clear-search")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Close)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.clear_search(cx);
                                    this.load_data(cx);
                                })),
                        )
                    }),
            );

        let entity = cx.entity();
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
            let colors = RowColors::from_cx(cx);
            match self.view {
                NavView::Topics => {
                    tracing::debug!("[Navigator] render topics list: {} items", self.topics.len());
                    if self.topics.is_empty() {
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
                        let topics = Rc::new(self.topics.clone());
                        let favorites = Rc::new(self.favorites.clone());
                        let selected = self.selected_topic.clone();
                        let has_more = self.topics_has_more;
                        let remaining = self.topics_total.saturating_sub(self.topics.len());
                        let count = topics.len() + if has_more { 1 } else { 0 };
                        let muted = colors.muted;

                        uniform_list("nav-topics", count, move |range, _window, _cx| {
                            range
                                .map(|ix| {
                                    if ix >= topics.len() {
                                        // 底部"加载更多"行
                                        let entity = entity.clone();
                                        return div()
                                            .id(("load-more", ix))
                                            .p_2()
                                            .text_xs()
                                            .text_center()
                                            .text_color(muted)
                                            .cursor_pointer()
                                            .child(format!("Load more ({} remaining)", remaining))
                                            .on_click(move |_, _, cx| {
                                                entity.update(cx, |this, cx| {
                                                    this.load_topics(true, cx)
                                                });
                                            })
                                            .into_any_element();
                                    }
                                    let item = topics[ix].clone();
                                    let is_fav = favorites
                                        .contains(&format!("{}\u{1}{}", item.cluster, item.name));
                                    let is_selected = selected
                                        .as_ref()
                                        .map(|(c, t)| c == &item.cluster && t == &item.name)
                                        .unwrap_or(false);
                                    Navigator::render_topic_row(
                                        entity.clone(),
                                        ix,
                                        item,
                                        is_fav,
                                        is_selected,
                                        &colors,
                                    )
                                })
                                .collect()
                        })
                        .into_any_element()
                    }
                }
                NavView::ConsumerGroups => {
                    if self.cg_items.is_empty() {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .text_color(muted_c)
                            .child(t(cx, "consumerGroups.noData"))
                            .into_any_element()
                    } else {
                        let items = Rc::new(self.cg_items.clone());
                        let has_more = self.cg_has_more;
                        let remaining = self.cg_total.saturating_sub(self.cg_items.len());
                        let count = items.len() + if has_more { 1 } else { 0 };
                        let muted = colors.muted;

                        uniform_list("nav-groups", count, move |range, _window, _cx| {
                            range
                                .map(|ix| {
                                    if ix >= items.len() {
                                        let entity = entity.clone();
                                        return div()
                                            .id(("load-more", ix))
                                            .p_2()
                                            .text_xs()
                                            .text_center()
                                            .text_color(muted)
                                            .cursor_pointer()
                                            .child(format!("Load more ({} remaining)", remaining))
                                            .on_click(move |_, _, cx| {
                                                entity.update(cx, |this, cx| {
                                                    this.load_consumer_groups(true, cx)
                                                });
                                            })
                                            .into_any_element();
                                    }
                                    Navigator::render_group_row(
                                        entity.clone(),
                                        ix,
                                        items[ix].clone(),
                                        &colors,
                                    )
                                })
                                .collect()
                        })
                        .into_any_element()
                    }
                }
            }
        };

        let selector = self.render_cluster_selector(cx);

        div()
            .relative()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .bg(sidebar_bg)
                    .child(self.render_header(cx))
                    .when(!self.show_history, |el| {
                        el.child(self.render_status_bar(cx)).child(search_bar)
                    })
                    .child(div().flex_1().overflow_hidden().child(content)),
            )
            // 集群选择器弹层绘制在最上层
            .child(selector)
    }
}
