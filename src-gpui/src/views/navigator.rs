//! TopicNavigator：列表模式左侧导航（Topics / Consumer Groups 双视图 + 历史面板 + 集群选择器）
//! 对齐 Vue TopicNavigator.vue 的全部行为。

use std::collections::HashSet;

use gpui::prelude::*;
use gpui::{
    actions, div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, KeyBinding,
    MouseButton, Render, ScrollStrategy, UniformListScrollHandle, Window,
};

use crate::app::{backend, root, Page, Route};
use crate::i18n::t;
use crate::icons::icon;
use crate::models::*;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::text_input::{TextInput, TextInputEvent};

actions!(km_navigator, [NavUp, NavDown, NavEnter]);

pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", NavUp, Some("KmNavigator")),
        KeyBinding::new("down", NavDown, Some("KmNavigator")),
        KeyBinding::new("enter", NavEnter, Some("KmNavigator")),
    ]);
}

const PAGE_SIZE: usize = 1000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavView {
    Topics,
    ConsumerGroups,
}

pub struct TopicNavigator {
    view: NavView,
    search_input: Entity<TextInput>,
    history_search_input: Entity<TextInput>,
    select_view: Entity<Select>,

    items: Vec<(String, String)>, // (name, cluster)
    total: usize,
    has_more: bool,
    offset: usize,
    loading: bool,
    loading_more: bool,
    refreshing: bool,

    selected: Option<(String, String)>,
    hovered: usize,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,

    show_history: bool,
    history_items: Vec<TopicHistoryItem>,
    history_loading: bool,

    // 集群选择器
    selector_open: bool,
    selected_clusters: HashSet<String>,
    selected_groups: HashSet<i64>,
    active_group_id: Option<i64>,
    selection_loaded: bool,

    backend_seen_ready: bool,
    search_generation: u64,
    last_route_sync: Option<(Page, String, String)>,
    clusters_cache: Vec<Cluster>,
    groups_cache: Vec<ClusterGroup>,
}

impl TopicNavigator {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(TextInput::new);
        search_input.update(cx, |i, _| i.set_placeholder(t("navigator.searchTopics")));
        let history_search_input = cx.new(TextInput::new);
        history_search_input.update(cx, |i, _| {
            i.set_placeholder(t("topicHistory.searchPlaceholder"))
        });

        let select_view = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption {
                        value: "topics".into(),
                        label: t("navigator.topics").into(),
                    },
                    SelectOption {
                        value: "consumer-groups".into(),
                        label: t("navigator.consumerGroups").into(),
                    },
                ],
                "topics",
                cx,
            )
        });

        let this = Self {
            view: NavView::Topics,
            search_input: search_input.clone(),
            history_search_input: history_search_input.clone(),
            select_view: select_view.clone(),
            items: vec![],
            total: 0,
            has_more: false,
            offset: 0,
            loading: false,
            loading_more: false,
            refreshing: false,
            selected: None,
            hovered: 0,
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            show_history: false,
            history_items: vec![],
            history_loading: false,
            selector_open: false,
            selected_clusters: HashSet::new(),
            selected_groups: HashSet::new(),
            active_group_id: None,
            selection_loaded: false,
            backend_seen_ready: false,
            search_generation: 0,
            last_route_sync: None,
            clusters_cache: vec![],
            groups_cache: vec![],
        };

        cx.subscribe(&search_input, |this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::Changed) {
                this.search_generation += 1;
                let generation = this.search_generation;
                this.hovered = 0;
                cx.notify();
                // 300ms 防抖后端搜索
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(300))
                        .await;
                    this.update(cx, |this, cx| {
                        if this.search_generation == generation {
                            this.reload(cx);
                        }
                    })
                    .ok();
                })
                .detach();
            }
        })
        .detach();

        cx.subscribe(&history_search_input, |_this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::Changed) {
                cx.notify();
            }
        })
        .detach();

        cx.subscribe(&select_view, |this, _, event: &SelectEvent, cx| {
            this.view = if event.value.as_ref() == "consumer-groups" {
                NavView::ConsumerGroups
            } else {
                NavView::Topics
            };
            this.search_input.update(cx, |i, cx| {
                i.set_placeholder(if this.view == NavView::Topics {
                    t("navigator.searchTopics")
                } else {
                    t("navigator.searchConsumerGroups")
                });
                i.reset(cx);
            });
            this.reload(cx);
        })
        .detach();

        this
    }

    // ==================== 数据加载 ====================

    pub fn on_clusters_changed(
        &mut self,
        existing: HashSet<String>,
        clusters: Vec<Cluster>,
        groups: Vec<ClusterGroup>,
        cx: &mut Context<Self>,
    ) {
        self.clusters_cache = clusters;
        self.groups_cache = groups;
        // 过滤掉已不存在的集群
        self.selected_clusters.retain(|c| existing.contains(c));
        if !self.selection_loaded {
            self.selection_loaded = true;
            self.restore_selection(cx);
        }
        self.reload(cx);
    }

    fn restore_selection(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            if let Ok(v) = b
                .dispatch(
                    "settings.get",
                    serde_json::json!({"keys": ["ui.selected_clusters"]}),
                )
                .await
            {
                let saved = v
                    .get("settings")
                    .and_then(|s| s.get("ui.selected_clusters"))
                    .and_then(|x| x.as_str())
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
                this.update(cx, |this, cx| {
                    if let Some(saved) = saved {
                        if let Some(arr) = saved.get("clusters").and_then(|x| x.as_array()) {
                            this.selected_clusters = arr
                                .iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                        if let Some(arr) = saved.get("groups").and_then(|x| x.as_array()) {
                            this.selected_groups = arr
                                .iter()
                                .filter_map(|x| x.as_i64())
                                .collect();
                        }
                    }
                    this.reload(cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// 当前生效的集群名集合（空 = 全部 / 或 activeGroup）
    fn effective_clusters(&self) -> Vec<String> {
        if !self.selected_clusters.is_empty() || !self.selected_groups.is_empty() {
            let mut names: HashSet<String> = self.selected_clusters.clone();
            for gid in &self.selected_groups {
                for c in self
                    .clusters_cache
                    .iter()
                    .filter(|c| c.group_id == Some(*gid))
                {
                    names.insert(c.name.clone());
                }
            }
            names.into_iter().collect()
        } else if let Some(gid) = self.active_group_id {
            self.clusters_cache
                .iter()
                .filter(|c| c.group_id == Some(gid))
                .map(|c| c.name.clone())
                .collect()
        } else {
            vec![]
        }
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        if !backend(cx).is_ready() {
            return;
        }
        self.items = vec![];
        self.offset = 0;
        self.has_more = false;
        self.loading = true;
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
        self.load_page(cx);
    }

    fn load_page(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        let clusters = self.effective_clusters();
        let search = self.search_input.read(cx).text().trim().to_string();
        let offset = self.offset;
        let view = self.view;
        cx.spawn(async move |this, cx| {
            let params = match view {
                NavView::Topics => {
                    let mut p = serde_json::json!({"offset": offset, "limit": PAGE_SIZE});
                    if !clusters.is_empty() {
                        p["cluster_ids"] = serde_json::json!(clusters);
                    }
                    if !search.is_empty() {
                        p["search"] = serde_json::json!(search);
                    }
                    p
                }
                NavView::ConsumerGroups => {
                    let mut p = serde_json::json!({"offset": offset, "limit": 10000});
                    if !clusters.is_empty() {
                        p["cluster_ids"] = serde_json::json!(clusters);
                    }
                    if !search.is_empty() {
                        p["search"] = serde_json::json!(search);
                    }
                    p
                }
            };
            let method = match view {
                NavView::Topics => "topic.list_with_cluster",
                NavView::ConsumerGroups => "consumer_group.list",
            };
            let result = b.dispatch(method, params).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                this.loading_more = false;
                match result {
                    Ok(v) => {
                        let mut new_items: Vec<(String, String)> = match view {
                            NavView::Topics => v
                                .get("topics")
                                .and_then(|x| x.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|item| {
                                            Some((
                                                item.get("name")?.as_str()?.to_string(),
                                                item.get("cluster")?.as_str()?.to_string(),
                                            ))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                            NavView::ConsumerGroups => v
                                .get("groups")
                                .and_then(|x| x.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|item| {
                                            Some((
                                                item.get("group_name")?.as_str()?.to_string(),
                                                item.get("cluster_id")?.as_str()?.to_string(),
                                            ))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        };
                        // 无搜索词时前端排序（先 cluster 后 name）
                        if search.is_empty() {
                            new_items.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
                        }
                        this.total =
                            v.get("total").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                        this.has_more = v
                            .get("has_more")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false);
                        if this.offset == 0 {
                            this.items = new_items;
                        } else {
                            this.items.extend(new_items);
                        }
                        // 同步 hover 到选中项
                        if let Some(sel) = &this.selected {
                            if let Some(ix) = this
                                .items
                                .iter()
                                .position(|(n, c)| Some(&(n.clone(), c.clone())) == Some(sel))
                            {
                                this.hovered = ix;
                            }
                        }
                    }
                    Err(_) => {}
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        if self.loading_more || !self.has_more || self.loading {
            return;
        }
        self.loading_more = true;
        self.offset += if self.view == NavView::Topics {
            PAGE_SIZE
        } else {
            10000
        };
        self.load_page(cx);
    }

    // ==================== 刷新 ====================

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        cx.notify();
        match self.view {
            NavView::Topics => overlay::toast_success(cx, t("topics.refreshingBg")),
            NavView::ConsumerGroups => overlay::toast_success(cx, t("consumerGroups.refreshingBg")),
        }
        let b = backend(cx);
        let clusters = self.effective_clusters();
        let view = self.view;
        cx.spawn(async move |this, cx| {
            // fire-and-forget
            match view {
                NavView::Topics => {
                    if clusters.is_empty() {
                        let _ = b.dispatch("topic.refresh", serde_json::json!({})).await;
                    } else {
                        for c in &clusters {
                            let _ = b
                                .dispatch(
                                    "topic.refresh",
                                    serde_json::json!({"cluster_id": c}),
                                )
                                .await;
                        }
                    }
                }
                NavView::ConsumerGroups => {
                    if clusters.is_empty() {
                        let _ = b
                            .dispatch("consumer_group.refresh", serde_json::json!({}))
                            .await;
                    } else {
                        for c in &clusters {
                            let _ = b
                                .dispatch(
                                    "consumer_group.refresh",
                                    serde_json::json!({"cluster_id": c}),
                                )
                                .await;
                        }
                    }
                }
            }
            // 轮询 refresh.status
            let timeout_ms = if view == NavView::Topics { 120_000 } else { 180_000 };
            let start = std::time::Instant::now();
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(1000))
                    .await;
                if start.elapsed().as_millis() > timeout_ms {
                    break;
                }
                if let Ok(v) = b.dispatch("refresh.status", serde_json::json!({})).await {
                    let refreshing: Vec<String> = v
                        .get("refreshing_clusters")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if clusters.is_empty() {
                        if refreshing.is_empty() {
                            break;
                        }
                    } else if !clusters.iter().any(|c| refreshing.contains(c)) {
                        break;
                    }
                }
            }
            this.update(cx, |this, cx| {
                this.refreshing = false;
                this.reload(cx);
            })
            .ok();
        })
        .detach();
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

    fn clear_history(&mut self, cx: &mut Context<Self>) {
        overlay::confirm(cx, t("topicHistory.clearTitle"), t("topicHistory.clearConfirm"), true, |cx| {
            let b = backend(cx);
            cx.spawn(async move |cx| {
                let _ = b
                    .dispatch("topic_history.clear", serde_json::json!({}))
                    .await;
                cx.update(|cx| {
                    root(cx).update(cx, |app, cx| {
                        app.navigator.update(cx, |n, cx| {
                            n.history_items = vec![];
                            cx.notify();
                        });
                    });
})
            })
            .detach();
        });
    }

    fn delete_history_item(&mut self, id: i64, cx: &mut Context<Self>) {
        let b = backend(cx);
        self.history_items.retain(|i| i.id != id);
        cx.notify();
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch("topic_history.delete", serde_json::json!({"id": id}))
                .await;
        })
        .detach();
    }

    fn open_topic(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        self.selected = Some((topic.to_string(), cluster.to_string()));
        self.show_history = false;
        cx.notify();
        root(cx).update(cx, |app, cx| {
            let route = Route::new(Page::Messages)
                .with("cluster", cluster)
                .with("topic", topic);
            app.navigate(route, true, cx);
        });
    }

    fn open_consumer_group(&mut self, cluster: &str, group: &str, cx: &mut Context<Self>) {
        self.selected = Some((group.to_string(), cluster.to_string()));
        cx.notify();
        root(cx).update(cx, |app, cx| {
            let route = Route::new(Page::ConsumerGroups)
                .with("cluster", cluster)
                .with("group", group);
            app.navigate(route, true, cx);
        });
    }

    pub fn on_topic_deleted(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        let before = self.items.len();
        self.items
            .retain(|(n, c)| !(n == topic && c == cluster));
        if self.items.len() != before {
            self.total = self.total.saturating_sub(1);
            cx.notify();
        }
    }

    /// 当前路由变化时高亮对应项（有变化才处理，避免渲染循环）
    pub fn sync_route_highlight(&mut self, cx: &mut Context<Self>) {
        let app = root(cx);
        let route = app.read(cx).route.clone();
        let sync_key = (
            route.page,
            route.get("cluster").unwrap_or("").to_string(),
            route
                .get("topic")
                .or_else(|| route.get("group"))
                .unwrap_or("")
                .to_string(),
        );
        if self.last_route_sync.as_ref() == Some(&sync_key) {
            return;
        }
        self.last_route_sync = Some(sync_key);
        match route.page {
            Page::Messages => {
                if let (Some(c), Some(t)) = (route.get("cluster"), route.get("topic")) {
                    self.selected = Some((t.to_string(), c.to_string()));
                }
            }
            Page::ConsumerGroups => {
                if let (Some(c), Some(g)) = (route.get("cluster"), route.get("group")) {
                    self.selected = Some((g.to_string(), c.to_string()));
                }
            }
            _ => {}
        }
        // 路由同步 view mode
        let mode = if route.page == Page::ConsumerGroups {
            NavView::ConsumerGroups
        } else {
            NavView::Topics
        };
        if mode != self.view {
            self.view = mode;
            self.select_view.update(cx, |s, cx| {
                s.set_value(
                    if mode == NavView::ConsumerGroups {
                        "consumer-groups"
                    } else {
                        "topics"
                    },
                    cx,
                )
            });
            self.reload(cx);
        }
        if let Some(sel) = &self.selected {
            if let Some(ix) = self
                .items
                .iter()
                .position(|(n, c)| n == &sel.0 && c == &sel.1)
            {
                self.hovered = ix;
            }
        }
        cx.notify();
    }

    // ==================== 键盘 ====================

    fn nav_up(&mut self, _: &NavUp, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        self.hovered = self.hovered.saturating_sub(1);
        self.scroll
            .scroll_to_item(self.hovered, ScrollStrategy::Center);
        cx.notify();
    }

    fn nav_down(&mut self, _: &NavDown, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        self.hovered = (self.hovered + 1).min(self.items.len() - 1);
        self.scroll
            .scroll_to_item(self.hovered, ScrollStrategy::Center);
        cx.notify();
    }

    fn nav_enter(&mut self, _: &NavEnter, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((name, cluster)) = self.items.get(self.hovered).cloned() {
            match self.view {
                NavView::Topics => self.open_topic(&cluster, &name, cx),
                NavView::ConsumerGroups => self.open_consumer_group(&cluster, &name, cx),
            }
        }
    }

    // ==================== 集群选择器 ====================

    fn selector_summary(&self, _cx: &App) -> String {
        let total_selected = self.selected_clusters.len() + self.selected_groups.len();
        if total_selected == 0 {
            t("navigator.allClusters")
        } else if total_selected == 1 && self.selected_clusters.len() == 1 {
            self.selected_clusters.iter().next().unwrap().clone()
        } else {
            format!("{} {}", total_selected, t("tree.clusters"))
        }
    }

    fn apply_selection(&mut self, cx: &mut Context<Self>) {
        self.selector_open = false;
        let clusters: Vec<String> = self.selected_clusters.iter().cloned().collect();
        let groups: Vec<i64> = self.selected_groups.iter().cloned().collect();
        let b = backend(cx);
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch(
                    "settings.update",
                    serde_json::json!({
                        "key": "ui.selected_clusters",
                        "value": serde_json::json!({"clusters": clusters, "groups": groups}).to_string(),
                    }),
                )
                .await;
        })
        .detach();
        self.search_input.update(cx, |i, cx| i.reset(cx));
        self.reload(cx);
    }

    fn render_selector_popup(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);
        let app = app.read(cx);
        let groups = app.groups.clone();
        let clusters = app.clusters.clone();

        let active_group_id = self.active_group_id;
        let selected_clusters = self.selected_clusters.clone();
        let _selected_groups = self.selected_groups.clone();

        let visible_clusters: Vec<Cluster> = match active_group_id {
            Some(gid) => clusters
                .iter()
                .filter(|c| c.group_id == Some(gid))
                .cloned()
                .collect(),
            None => clusters.clone(),
        };
        let any_selected_in_view = visible_clusters
            .iter()
            .any(|c| selected_clusters.contains(&c.name));
        let visible_clusters_for_clear = visible_clusters.clone();

        gpui::deferred(
            div()
                .absolute()
                .top(px(76.))
                .left(px(4.))
                .w(px(300.))
                .h(px(300.))
                .flex()
                .flex_col()
                .bg(theme::context_menu_bg())
                .border_1()
                .border_color(theme::glass_border())
                .rounded(px(8.))
                .shadow_lg()
                .child(
                    // 双栏
                    div()
                        .flex()
                        .flex_1()
                        .overflow_hidden()
                        // 左列：所有集群 + 分组
                        .child(
                            div().id("views_navigator_rs_1")
                                .w(px(110.))
                                .flex_none()
                                .flex()
                                .flex_col()
                                .border_r_1()
                                .border_color(theme::border_base_200())
                                .overflow_y_scroll()
                                .child(
                                    div()
                                        .id("sel-all")
                                        .flex()
                                        .items_center()
                                        .h(px(28.))
                                        .px(px(8.))
                                        .cursor_pointer()
                                        .text_size(px(11.))
                                        .text_color(if active_group_id.is_none() {
                                            theme::badge_primary_text()
                                        } else {
                                            theme::text_primary()
                                        })
                                        .when(active_group_id.is_none(), |d| {
                                            d.bg(theme::badge_primary_bg())
                                        })
                                        .hover(|s| s.bg(theme::context_menu_item_hover()))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.active_group_id = None;
                                                cx.notify();
                                            }),
                                        )
                                        .child(t("navigator.allClusters")),
                                )
                                .children(groups.into_iter().enumerate().map(|(ix, g)| {
                                    let gid = g.id;
                                    let group_clusters: Vec<String> = clusters
                                        .iter()
                                        .filter(|c| c.group_id == Some(gid))
                                        .map(|c| c.name.clone())
                                        .collect();
                                    let fully = !group_clusters.is_empty()
                                        && group_clusters
                                            .iter()
                                            .all(|n| selected_clusters.contains(n));
                                    let active = active_group_id == Some(gid);
                                    div()
                                        .id(("sel-group", ix))
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .h(px(28.))
                                        .px(px(6.))
                                        .text_size(px(11.))
                                        .cursor_pointer()
                                        .text_color(if active {
                                            theme::badge_primary_text()
                                        } else {
                                            theme::text_primary()
                                        })
                                        .when(active, |d| d.bg(theme::badge_primary_bg()))
                                        .hover(|s| s.bg(theme::context_menu_item_hover()))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.active_group_id = Some(gid);
                                                cx.notify();
                                            }),
                                        )
                                        .child(
                                            div()
                                                .id(("sel-group-check", ix))
                                                .size(px(13.))
                                                .rounded(px(3.))
                                                .border_1()
                                                .border_color(theme::base_content_alpha(0.3))
                                                .when(fully, |d| {
                                                    d.bg(theme::primary())
                                                        .border_color(theme::primary())
                                                })
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        move |this, _, _, cx| {
                                                            let fully = !group_clusters.is_empty()
                                                                && group_clusters.iter().all(
                                                                    |n| {
                                                                        this.selected_clusters
                                                                            .contains(n)
                                                                    },
                                                                );
                                                            if fully {
                                                                for n in &group_clusters {
                                                                    this.selected_clusters
                                                                        .remove(n);
                                                                }
                                                                this.selected_groups.remove(&gid);
                                                            } else {
                                                                for n in &group_clusters {
                                                                    this.selected_clusters
                                                                        .insert(n.clone());
                                                                }
                                                                this.selected_groups.insert(gid);
                                                            }
                                                            cx.notify();
                                                        },
                                                    ),
                                                )
                                                .when(fully, |d| {
                                                    d.child(
                                                        icon("check")
                                                            .size(px(10.))
                                                            .text_color(gpui::white()),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .child(g.name),
                                        )
                                })),
                        )
                        // 右列：集群复选框
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
                                        .justify_end()
                                        .h(px(24.))
                                        .px(px(8.))
                                        .when(any_selected_in_view, |d| {
                                            d.child(
                                                div()
                                                    .id("sel-deselect-all")
                                                    .text_size(px(10.))
                                                    .text_color(theme::badge_primary_text())
                                                    .cursor_pointer()
                                                    .hover(|s| s.opacity(0.7))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, _, cx| {
                                                            for c in &visible_clusters_for_clear {
                                                                this.selected_clusters
                                                                    .remove(&c.name);
                                                            }
                                                            cx.notify();
                                                        }),
                                                    )
                                                    .child(t("navigator.deselectAll")),
                                            )
                                        }),
                                )
                                .child(
                                    div().id("auto_views_navigator_rs_101").flex_1().overflow_y_scroll().children(
                                        visible_clusters.into_iter().enumerate().map(
                                            |(ix, c)| {
                                                let checked =
                                                    selected_clusters.contains(&c.name);
                                                let name = c.name.clone();
                                                div()
                                                    .id(("sel-cluster", ix))
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .h(px(28.))
                                                    .px(px(8.))
                                                    .cursor_pointer()
                                                    .text_size(px(11.))
                                                    .text_color(theme::text_primary())
                                                    .hover(|s| {
                                                        s.bg(theme::context_menu_item_hover())
                                                    })
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, _, cx| {
                                                            if !this
                                                                .selected_clusters
                                                                .remove(&name)
                                                            {
                                                                this.selected_clusters
                                                                    .insert(name.clone());
                                                            }
                                                            cx.notify();
                                                        }),
                                                    )
                                                    .child(
                                                        div()
                                                            .size(px(13.))
                                                            .rounded(px(3.))
                                                            .border_1()
                                                            .border_color(
                                                                theme::base_content_alpha(0.3),
                                                            )
                                                            .when(checked, |d| {
                                                                d.bg(theme::primary())
                                                                    .border_color(theme::primary())
                                                            })
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(checked, |d| {
                                                                d.child(
                                                                    icon("check")
                                                                        .size(px(10.))
                                                                        .text_color(gpui::white()),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .child(c.name),
                                                    )
                                            },
                                        ),
                                    ),
                                ),
                        ),
                )
                // 底部按钮
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.))
                        .px(px(8.))
                        .py(px(6.))
                        .border_t_1()
                        .border_color(theme::border_base_200())
                        .child(
                            btn("sel-clear", BtnKind::Ghost, BtnSize::Xs)
                                .child(t("common.clear"))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.selected_clusters.clear();
                                        this.selected_groups.clear();
                                        cx.notify();
                                    }),
                                ),
                        )
                        .child(
                            btn("sel-apply", BtnKind::Primary, BtnSize::Xs)
                                .child(t("common.apply"))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.apply_selection(cx);
                                    }),
                                ),
                        ),
                ),
        )
        .with_priority(40)
    }

    // ==================== 渲染 ====================

    fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let search = self.history_search_input.read(cx).text().to_lowercase();
        let items: Vec<TopicHistoryItem> = self
            .history_items
            .iter()
            .filter(|i| search.is_empty() || i.topic_name.to_lowercase().contains(&search))
            .cloned()
            .collect();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .child(
                // 头部
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(8.))
                    .h(px(32.))
                    .border_b_1()
                    .border_color(theme::border_base_200())
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("topicHistory.title")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(
                                icon_btn("history-clear", "trash", BtnSize::Xs)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.clear_history(cx)
                                        }),
                                    ),
                            )
                            .child(
                                icon_btn("history-refresh", "refresh", BtnSize::Xs)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.load_history(cx)
                                        }),
                                    ),
                            )
                            .child(
                                icon_btn("history-close", "x-mark", BtnSize::Xs)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.show_history = false;
                                            cx.notify();
                                        }),
                                    ),
                            ),
                    ),
            )
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
                div().id("views_navigator_rs_2")
                    .flex_1()
                    .overflow_y_scroll()
                    .when(self.history_loading, |d| {
                        d.child(loading_block(t("common.loading")))
                    })
                    .when(!self.history_loading && items.is_empty(), |d| {
                        d.child(empty_block(
                            "clock",
                            if self.history_items.is_empty() {
                                t("topicHistory.empty")
                            } else {
                                t("topicHistory.noResults")
                            },
                            t("topicHistory.emptyDesc"),
                        ))
                    })
                    .children(items.into_iter().enumerate().map(|(ix, item)| {
                        let cluster = item.cluster_id.clone();
                        let topic = item.topic_name.clone();
                        let id = item.id;
                        div()
                            .id(("history-item", ix))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .px(px(8.))
                            .py(px(5.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::context_menu_item_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.open_topic(&cluster, &topic, cx)
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
                                            .text_color(theme::text_primary())
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .child(item.topic_name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(9.))
                                            .text_color(theme::text_secondary())
                                            .child(format!(
                                                "{} · {}",
                                                item.cluster_name,
                                                relative_time(item.last_accessed_at.as_deref())
                                            )),
                                    ),
                            )
                            .child(
                                icon_btn(("history-del", ix), "x-mark", BtnSize::Xs)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _, _, cx| {
                                            this.delete_history_item(id, cx)
                                        }),
                                    ),
                            )
                    })),
            )
    }
}

/// 相对时间（刚刚/N分钟前/N小时前/N天前/日期）
pub fn relative_time(iso: Option<&str>) -> String {
    let Some(iso) = iso else {
        return String::new();
    };
    let Some(ts) = parse_iso8601(iso) else {
        return iso.to_string();
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let diff = now - ts;
    if diff < 60_000 {
        t("time.justNow")
    } else if diff < 3_600_000 {
        format!("{}{}", diff / 60_000, t("time.minutesAgo"))
    } else if diff < 86_400_000 {
        format!("{}{}", diff / 3_600_000, t("time.hoursAgo"))
    } else if diff < 7 * 86_400_000 {
        format!("{}{}", diff / 86_400_000, t("time.daysAgo"))
    } else {
        format_datetime(ts, true)
    }
}

/// 解析 ISO8601 / "YYYY-MM-DD HH:mm:ss" 为 epoch millis
pub fn parse_iso8601(s: &str) -> Option<i64> {
    let s = s.trim().replace('T', " ").replace('Z', "");
    let s = s.split('.').next().unwrap_or("");
    let mut parts = s.split(' ');
    let date = parts.next()?;
    let time = parts.next().unwrap_or("00:00:00");
    let d: Vec<i64> = date.split('-').filter_map(|x| x.parse().ok()).collect();
    let t: Vec<i64> = time.split(':').filter_map(|x| x.parse().ok()).collect();
    if d.len() != 3 || t.is_empty() {
        return None;
    }
    let (y, mo, da) = (d[0], d[1], d[2]);
    let (h, mi, se) = (
        *t.first().unwrap_or(&0),
        *t.get(1).unwrap_or(&0),
        *t.get(2).unwrap_or(&0),
    );
    Some(civil_to_millis(y, mo, da, h, mi, se))
}

fn civil_to_millis(y: i64, m: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
    // days from civil (Howard Hinnant 算法)
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    ((days * 24 + h) * 3600 + mi * 60 + s) * 1000
}

/// epoch millis -> "YYYY/MM/DD HH:mm:ss" 或仅日期（本地时区）
pub fn format_datetime(millis: i64, date_only: bool) -> String {
    let secs = millis / 1000 + local_tz_offset_secs();
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    if date_only {
        format!("{:04}/{:02}/{:02}", y, m, d)
    } else {
        format!(
            "{:04}/{:02}/{:02} {:02}:{:02}:{:02}",
            y,
            m,
            d,
            rem / 3600,
            (rem % 3600) / 60,
            rem % 60
        )
    }
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

thread_local! {
    static TZ_OFFSET: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

fn local_tz_offset_secs() -> i64 {
    TZ_OFFSET.with(|c| {
        if let Some(v) = c.get() {
            return v;
        }
        // 通过比较 UTC 与本地时间计算偏移
        let v = unsafe {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as libc::c_long;
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            tm.tm_gmtoff
        };
        c.set(Some(v));
        v
    })
}

impl Focusable for TopicNavigator {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TopicNavigator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.backend_seen_ready && backend(cx).is_ready() {
            self.backend_seen_ready = true;
            let (existing, clusters, groups) = {
                let app = root(cx);
                let app = app.read(cx);
                (
                    app.clusters
                        .iter()
                        .map(|c| c.name.clone())
                        .collect::<HashSet<String>>(),
                    app.clusters.clone(),
                    app.groups.clone(),
                )
            };
            self.on_clusters_changed(existing, clusters, groups, cx);
        }
        self.sync_route_highlight(cx);

        let search_empty = self.search_input.read(cx).is_empty();
        let refreshing = self.refreshing;

        div()
            .flex()
            .flex_col()
            .size_full()
            .key_context("KmNavigator")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::nav_up))
            .on_action(cx.listener(Self::nav_down))
            .on_action(cx.listener(Self::nav_enter))
            // Header：视图切换 + 快捷按钮
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
                        div()
                            .w(px(110.))
                            .when(self.show_history, |d| d.opacity(0.5))
                            .child(self.select_view.clone()),
                    )
                    .child(div().flex_1())
                    .child(
                        icon_btn("nav-clusters", "server", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| {
                                root(cx).update(cx, |app, cx| {
                                    app.navigate(Route::new(Page::Clusters), true, cx)
                                });
                            }),
                        ),
                    )
                    .child(
                        icon_btn("nav-favorites", "star", BtnSize::Xs).on_mouse_down(
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
                            .id("nav-history")
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
                            .hover(|s| {
                                s.bg(theme::btn_ghost_hover())
                                    .text_color(theme::text_primary())
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.toggle_history(cx)),
                            )
                            .child(icon("clock").size(px(13.))),
                    ),
            )
            // 状态栏（历史模式隐藏）
            .when(!self.show_history, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .px(px(8.))
                        .h(px(28.))
                        .text_size(px(11.))
                        .text_color(theme::text_secondary())
                        .child(t("navigator.cluster"))
                        .child(
                            div()
                                .id("cluster-selector")
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .px(px(4.))
                                .h(px(20.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .text_color(theme::text_primary())
                                .hover(|s| s.bg(theme::btn_ghost_hover()))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.selector_open = !this.selector_open;
                                        cx.notify();
                                    }),
                                )
                                .child(self.selector_summary(cx))
                                .child(
                                    icon(if self.selector_open {
                                        "chevron-up"
                                    } else {
                                        "chevron-down"
                                    })
                                    .size(px(10.))
                                    .text_color(theme::text_secondary()),
                                ),
                        )
                        .child(div().flex_1())
                        .child(format!("{} / {} ", self.items.len(), self.total))
                        .child(if self.view == NavView::Topics {
                            t("navigator.topics")
                        } else {
                            t("navigator.consumerGroups")
                        })
                        .child(
                            div()
                                .id("nav-refresh")
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(20.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .when(refreshing, |d| d.opacity(0.5))
                                .hover(|s| s.bg(theme::btn_ghost_hover()))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        if !this.refreshing {
                                            this.refresh(cx);
                                        }
                                    }),
                                )
                                .child(if refreshing {
                                    spinner(12.)
                                } else {
                                    icon("refresh")
                                        .size(px(12.))
                                        .text_color(theme::text_secondary())
                                        .into_any_element()
                                }),
                        ),
                )
            })
            // 搜索框（历史模式隐藏）
            .when(!self.show_history, |d| {
                d.child(
                    div().px(px(8.)).py(px(4.)).child(
                        div()
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(theme::input_bg())
                            .border_1()
                            .border_color(theme::base_content_alpha(0.15))
                            .text_size(px(11.))
                            .child(div().flex_1().child(self.search_input.clone()))
                            .child(if search_empty {
                                icon("search")
                                    .size(px(12.))
                                    .text_color(theme::text_secondary())
                                    .into_any_element()
                            } else {
                                div()
                                    .id("nav-search-clear")
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.search_input.update(cx, |i, cx| i.reset(cx));
                                            this.reload(cx);
                                        }),
                                    )
                                    .child(
                                        icon("x-mark")
                                            .size(px(12.))
                                            .text_color(theme::text_secondary()),
                                    )
                                    .into_any_element()
                            }),
                    ),
                )
            })
            // 主区域
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .when(self.show_history, |d| {
                        d.child(self.render_history(cx))
                    })
                    .when(!self.show_history, |d| {
                        d.child(self.render_list(cx))
                    })
                    .when(self.selector_open, |d| {
                        d.child(self.render_selector_popup(cx))
                    }),
            )
    }
}

impl TopicNavigator {
    fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let loading = self.loading;
        let items_empty = self.items.is_empty();
        let search_empty = self.search_input.read(cx).is_empty();
        let item_count = self.items.len();

        div()
            .size_full()
            .when(loading && items_empty, |d| {
                d.child(loading_block(t("common.loading")))
            })
            .when(!loading && items_empty, |d| {
                d.child(empty_block(
                    "database",
                    if search_empty {
                        if self.view == NavView::Topics {
                            t("navigator.noTopics")
                        } else {
                            t("consumerGroups.noGroups")
                        }
                    } else {
                        t("common.noData")
                    },
                    "",
                ))
            })
            .when(item_count > 0, |d| {
                d.child(
                    gpui::uniform_list(
                        "nav-list",
                        item_count,
                        cx.processor(|this: &mut TopicNavigator, range: std::ops::Range<usize>, _window, cx| {
                            // 无限滚动：接近底部时加载更多
                            if range.end >= this.items.len().saturating_sub(5) {
                                let entity = cx.entity();
                                cx.defer(move |cx| {
                                    entity.update(cx, |this, cx| this.load_more(cx));
                                });
                            }
                            let mut out = Vec::with_capacity(range.len());
                            for ix in range {
                                let Some((name, cluster)) = this.items.get(ix).cloned() else {
                                    continue;
                                };
                                let (name_c, cluster_c) = (name.clone(), cluster.clone());
                                let cluster_badge = cluster.clone();
                                let selected = this
                                    .selected
                                    .as_ref()
                                    .map(|s| s == &(name.clone(), cluster.clone()))
                                    .unwrap_or(false);
                                let hovered = ix == this.hovered;
                                let view = this.view;
                                out.push(
                                    div()
                                        .id(("nav-item", ix))
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .h(px(28.))
                                        .px(px(8.))
                                        .mx(px(4.))
                                        .rounded(px(5.))
                                        .cursor_pointer()
                                        .when(selected, |d| {
                                            d.bg(theme::badge_primary_bg())
                                        })
                                        .when(!selected && hovered, |d| {
                                            d.bg(theme::table_row_hover())
                                        })
                                        .hover(|s| {
                                            s.bg(if selected {
                                                theme::badge_primary_bg()
                                            } else {
                                                theme::table_row_hover()
                                            })
                                        })
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, window, cx| {
                                                this.hovered = ix;
                                                window.focus(&this.focus_handle, cx);
                                                match view {
                                                    NavView::Topics => {
                                                        this.open_topic(&cluster_c, &name_c, cx)
                                                    }
                                                    NavView::ConsumerGroups => this
                                                        .open_consumer_group(
                                                            &cluster_c, &name_c, cx,
                                                        ),
                                                }
                                            }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .text_size(px(11.))
                                                .text_color(theme::text_primary())
                                                .child(name),
                                        )
                                        .child(
                                            div()
                                                .id(("nav-badge", ix))
                                                .flex_none()
                                                .max_w(px(56.))
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .cursor_pointer()
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(move |_, _, _, cx| {
                                                        root(cx).update(cx, |app, cx| {
                                                            let route = Route::new(
                                                                Page::Topics,
                                                            )
                                                            .with("cluster", &cluster_badge);
                                                            app.navigate(route, true, cx);
                                                        });
                                                    }),
                                                )
                                                .child(badge(cluster.clone(), BadgeKind::Ghost)),
                                        ),
                                );
                            }
                            out
                        }),
                    )
                    .track_scroll(&self.scroll)
                    .size_full(),
                )
            })
    }
}
