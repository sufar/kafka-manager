//! 工作区：顶部导航栏 + 可拖拽左侧导航器 + 页面内容
//!
//! 布局对齐旧版 Vue ModernLayout：
//! ┌──────────────────────────────────────┐
//! │ TopNavBar (h-10): logo/搜索/语言/主题/设置 │
//! ├───────────┬──────────────────────────┤
//! │ Navigator │        页面内容          │
//! │ (可拖拽)  │                          │
//! └───────────┴──────────────────────────┘

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::*;

use crate::components::navigator::{NavEvent, Navigator};
use crate::components::tree_navigator::TreeNavigator;
use crate::i18n::{t, I18n};
use crate::pages::clusters::ClustersPage;
use crate::pages::consumer_groups::ConsumerGroupsPage;
use crate::pages::favorites::FavoritesPage;
use crate::pages::messages::MessagesPage;
use crate::pages::schema_registry::SchemaRegistryPage;
use crate::pages::settings::SettingsPage;
use crate::pages::topic_consumer_groups::TopicConsumerGroupsPage;
use crate::pages::topics::TopicsPage;
use crate::state::{Backend, Page, SidebarMode, TokioRuntime};

actions!(workspace, [SearchNextResult, SearchPrevResult, FocusGlobalSearch, DismissSearch]);

/// 全局搜索结果
#[derive(Clone, Debug)]
struct SearchResult {
    cluster: String,
    topic: String,
}

/// 返回导航栈的历史快照
#[derive(Clone, Debug, PartialEq)]
struct NavSnapshot {
    page: Page,
    cluster: Option<String>,
    topic: Option<String>,
    group: Option<String>,
}

pub struct Workspace {
    page: Page,
    navigator: Entity<Navigator>,
    tree_navigator: Entity<TreeNavigator>,
    clusters_page: Entity<ClustersPage>,
    topics_page: Entity<TopicsPage>,
    messages_page: Entity<MessagesPage>,
    consumer_groups_page: Entity<ConsumerGroupsPage>,
    schema_registry_page: Entity<SchemaRegistryPage>,
    favorites_page: Entity<FavoritesPage>,
    settings_page: Entity<SettingsPage>,
    topic_cg_page: Entity<TopicConsumerGroupsPage>,
    // 顶部全局搜索
    search_input: Entity<InputState>,
    search_results: Vec<SearchResult>,
    search_open: bool,
    search_selected: usize,
    // 返回导航栈
    nav_history: Vec<NavSnapshot>,
    window_handle: AnyWindowHandle,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let navigator = cx.new(|cx| Navigator::new(window, cx));
        let tree_navigator = cx.new(|cx| TreeNavigator::new(window, cx));
        let clusters_page = cx.new(|cx| ClustersPage::new(window, cx));
        let topics_page = cx.new(|cx| TopicsPage::new(window, cx));
        let messages_page = cx.new(|cx| MessagesPage::new(window, cx));
        let consumer_groups_page = cx.new(|cx| ConsumerGroupsPage::new(window, cx));
        let schema_registry_page = cx.new(|cx| SchemaRegistryPage::new(window, cx));
        let favorites_page = cx.new(|cx| FavoritesPage::new(window, cx));
        let settings_page = cx.new(|cx| SettingsPage::new(window, cx));
        let topic_cg_page = cx.new(|cx| TopicConsumerGroupsPage::new(window, cx));
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "layout.searchPlaceholder"))
        });

        let mut subscriptions = Vec::new();

        // 导航器事件（平铺与树形共用处理逻辑）
        subscriptions.push(cx.subscribe(&navigator, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&tree_navigator, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&topic_cg_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&topics_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&clusters_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&messages_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&consumer_groups_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&schema_registry_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&favorites_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));
        subscriptions.push(cx.subscribe(&settings_page, |this, _, event: &NavEvent, cx| {
            this.handle_nav_event(event, cx);
        }));

        // 全局搜索（防抖 300ms；回车打开选中结果）
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                match event {
                    InputEvent::Change => this.schedule_search(cx),
                    InputEvent::PressEnter { .. } => {
                        if this.search_open && !this.search_results.is_empty() {
                            let ix = this.search_selected.min(this.search_results.len() - 1);
                            let result = this.search_results[ix].clone();
                            this.select_search_result(result, cx);
                        }
                    }
                    _ => {}
                }
            },
        ));

        // 搜索下拉键盘导航 + Ctrl+K 聚焦搜索
        cx.bind_keys([
            KeyBinding::new("down", SearchNextResult, Some("GlobalSearch")),
            KeyBinding::new("up", SearchPrevResult, Some("GlobalSearch")),
            KeyBinding::new("escape", DismissSearch, Some("GlobalSearch")),
            KeyBinding::new("ctrl-k", FocusGlobalSearch, Some("Workspace")),
        ]);

        // 启动后从设置中加载语言与主题
        let rt = TokioRuntime::handle(cx);
        let state = Backend::state(cx);
        // 启动加载 JSON 高亮模板（消息详情 JSON 着色用）
        crate::utils::load_json_template(cx);
        cx.spawn(async move |this, cx| {
            let result = match state {
                Some(state) => crate::service::call(
                    &rt,
                    state,
                    "settings.get",
                    serde_json::json!({ "keys": ["ui.language", "ui.theme", "ui.sidebar_mode", "ui.last_page"] }),
                )
                .await
                .ok(),
                None => None,
            };
            if let Some(value) = result {
                // 恢复上次浏览的页面（需在 App 上下文中访问各页面实体）
                let last_page = value
                    .get("settings")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| {
                        arr.iter().find_map(|item| {
                            if item.get("key").and_then(|k| k.as_str()) == Some("ui.last_page") {
                                item.get("value").and_then(|v| v.as_str()).map(String::from)
                            } else {
                                None
                            }
                        })
                    });
                if let Some(last_page) = last_page {
                    this.update(cx, |this, cx| {
                        this.restore_last_page(&last_page, cx);
                    })
                    .ok();
                }
                cx.update(|cx| {
                    if let Some(settings) = value.get("settings").and_then(|v| v.as_array()) {
                        for item in settings {
                            let key = item.get("key").and_then(|k| k.as_str()).unwrap_or("");
                            let val = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            match (key, val) {
                                ("ui.language", "en") => I18n::global_mut(cx).set_lang("en"),
                                ("ui.language", "zh") => I18n::global_mut(cx).set_lang("zh"),
                                ("ui.theme", val) => crate::theme::apply_by_name(val, None, cx),
                                ("ui.sidebar_mode", "tree") => SidebarMode::set(cx, true),
                                ("ui.sidebar_mode", _) => SidebarMode::set(cx, false),
                                _ => {}
                            }
                        }
                    }
                    cx.refresh_windows();
                })
                .ok();
            }
        })
        .detach();

        Self {
            page: Page::Clusters,
            navigator,
            tree_navigator,
            clusters_page,
            topics_page,
            messages_page,
            consumer_groups_page,
            schema_registry_page,
            favorites_page,
            settings_page,
            topic_cg_page,
            search_input,
            search_results: Vec::new(),
            search_open: false,
            search_selected: 0,
            nav_history: Vec::new(),
            window_handle: window.window_handle(),
            _subscriptions: subscriptions,
        }
    }

    fn switch_page(&mut self, page: Page, cx: &mut Context<Self>) {
        tracing::info!("[NAV] switch_page: {:?}", page);
        self.page = page;
        cx.notify();
    }

    /// 处理导航器（平铺/树形）发来的导航事件
    fn handle_nav_event(&mut self, event: &NavEvent, cx: &mut Context<Self>) {
        tracing::info!("[NAV] handle_nav_event: {:?}", event);
        // 返回上一页：走独立逻辑，不压栈
        if matches!(event, NavEvent::GoBack) {
            self.go_back(cx);
            return;
        }
        let before = self.snapshot_current(cx);
        match event {
            NavEvent::OpenMessages { cluster, topic } => {
                self.messages_page.update(cx, |page, cx| {
                    page.select_cluster_topic(cluster.clone(), topic.clone(), cx)
                });
                self.switch_page(Page::Messages, cx);
            }
            NavEvent::OpenMessagesSend { cluster, topic } => {
                self.messages_page.update(cx, |page, cx| {
                    page.select_cluster_topic_send(cluster.clone(), topic.clone(), cx)
                });
                self.switch_page(Page::Messages, cx);
            }
            NavEvent::OpenTopics { cluster } => {
                self.topics_page.update(cx, |page, cx| {
                    page.select_cluster(cluster.clone(), cx)
                });
                self.switch_page(Page::Topics, cx);
            }
            NavEvent::OpenConsumerGroups { cluster, group } => {
                self.consumer_groups_page.update(cx, |page, cx| {
                    page.select_group(cluster.clone(), group.clone(), cx)
                });
                self.switch_page(Page::ConsumerGroups, cx);
            }
            NavEvent::OpenTopicConsumerGroups { cluster, topic } => {
                self.topic_cg_page.update(cx, |page, cx| {
                    page.select_cluster_topic(cluster.clone(), topic.clone(), cx)
                });
                self.switch_page(Page::TopicConsumerGroups, cx);
            }
            NavEvent::OpenClustersAction { cluster, action } => {
                let cluster = cluster.clone();
                let action = *action;
                let clusters_page = self.clusters_page.clone();
                let wh = self.window_handle;
                self.switch_page(Page::Clusters, cx);
                // 弹窗需要 window，经 window_handle 调用
                let _ = wh.update(cx, |_, window, cx| {
                    clusters_page.update(cx, |page, cx| {
                        page.open_action(cluster.clone(), action, window, cx);
                    });
                });
            }
            NavEvent::OpenPage(page) => {
                self.switch_page(*page, cx);
            }
            NavEvent::GoBack => unreachable!("GoBack 已在前面提前处理"),
        }
        self.after_navigate(before, cx);
    }

    /// 当前页面状态快照
    fn snapshot_current(&self, cx: &App) -> NavSnapshot {
        let (cluster, topic, group) = match self.page {
            Page::Messages => {
                let p = self.messages_page.read(cx);
                (p.current_cluster(), p.current_topic(), None)
            }
            Page::Topics => (self.topics_page.read(cx).current_cluster(), None, None),
            Page::ConsumerGroups => {
                let p = self.consumer_groups_page.read(cx);
                (p.current_cluster(), None, p.current_group())
            }
            Page::TopicConsumerGroups => {
                let p = self.topic_cg_page.read(cx);
                let ct = p.current_topic(); // Option<(String, String)> = (cluster, topic)
                (ct.as_ref().map(|(c, _)| c.clone()), ct.map(|(_, t)| t), None)
            }
            _ => (None, None, None),
        };
        NavSnapshot {
            page: self.page,
            cluster,
            topic,
            group,
        }
    }

    /// 导航后处理：状态变化时压入返回栈 + 持久化
    fn after_navigate(&mut self, before: NavSnapshot, cx: &mut Context<Self>) {
        let now = self.snapshot_current(cx);
        if now != before {
            self.nav_history.push(before);
            if self.nav_history.len() > 50 {
                self.nav_history.remove(0);
            }
            self.sync_can_go_back(cx);
            self.persist_last_page(cx);
        }
    }

    /// 同步全局「能否返回」状态（供各页头返回按钮读取）
    fn sync_can_go_back(&self, cx: &mut App) {
        cx.set_global(crate::components::back_button::CanGoBack(
            !self.nav_history.is_empty(),
        ));
    }

    /// 返回上一页
    fn go_back(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.nav_history.pop() else {
            return;
        };
        self.apply_snapshot(&snapshot, cx);
        self.sync_can_go_back(cx);
        self.persist_last_page(cx);
    }

    /// 应用历史快照（不压栈）
    fn apply_snapshot(&mut self, snapshot: &NavSnapshot, cx: &mut Context<Self>) {
        match snapshot.page {
            Page::Messages => {
                if let (Some(c), Some(t)) = (snapshot.cluster.clone(), snapshot.topic.clone()) {
                    self.messages_page
                        .update(cx, |p, cx| p.select_cluster_topic(c, t, cx));
                }
            }
            Page::Topics => {
                if let Some(c) = snapshot.cluster.clone() {
                    self.topics_page.update(cx, |p, cx| p.select_cluster(c, cx));
                }
            }
            Page::ConsumerGroups => {
                if let Some(c) = snapshot.cluster.clone() {
                    self.consumer_groups_page.update(cx, |p, cx| {
                        p.select_group(c, snapshot.group.clone(), cx)
                    });
                }
            }
            Page::TopicConsumerGroups => {
                if let (Some(c), Some(t)) = (snapshot.cluster.clone(), snapshot.topic.clone()) {
                    self.topic_cg_page
                        .update(cx, |p, cx| p.select_cluster_topic(c, t, cx));
                }
            }
            _ => {}
        }
        self.switch_page(snapshot.page, cx);
    }

    /// 页面 → 持久化键
    fn page_key(page: Page) -> &'static str {
        match page {
            Page::Clusters => "clusters",
            Page::Topics => "topics",
            Page::Messages => "messages",
            Page::ConsumerGroups => "consumerGroups",
            Page::TopicConsumerGroups => "topicConsumerGroups",
            Page::SchemaRegistry => "schemaRegistry",
            Page::Favorites => "favorites",
            Page::Settings => "settings",
        }
    }

    /// 持久化当前页面（启动时恢复）
    fn persist_last_page(&self, cx: &mut Context<Self>) {
        let snapshot = self.snapshot_current(cx);
        let value = serde_json::json!({
            "page": Self::page_key(snapshot.page),
            "cluster": snapshot.cluster,
            "topic": snapshot.topic,
            "group": snapshot.group,
        })
        .to_string();
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        cx.spawn(async move |_this, _cx| {
            let _ = crate::service::call(
                &rt,
                state,
                "settings.update",
                serde_json::json!({ "key": "ui.last_page", "value": value }),
            )
            .await;
        })
        .detach();
    }

    /// 启动时恢复上次浏览的页面
    fn restore_last_page(&mut self, value: &str, cx: &mut Context<Self>) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(value) else {
            return;
        };
        let page = match v.get("page").and_then(|p| p.as_str()) {
            Some("topics") => Page::Topics,
            Some("messages") => Page::Messages,
            Some("consumerGroups") => Page::ConsumerGroups,
            Some("topicConsumerGroups") => Page::TopicConsumerGroups,
            Some("schemaRegistry") => Page::SchemaRegistry,
            Some("favorites") => Page::Favorites,
            Some("settings") => Page::Settings,
            // clusters 是默认页，无需恢复
            _ => return,
        };
        let cluster = v
            .get("cluster")
            .and_then(|c| c.as_str())
            .map(String::from);
        let topic = v.get("topic").and_then(|c| c.as_str()).map(String::from);
        let group = v.get("group").and_then(|c| c.as_str()).map(String::from);
        match page {
            Page::Messages => {
                if let (Some(c), Some(t)) = (cluster, topic) {
                    self.messages_page
                        .update(cx, |p, cx| p.select_cluster_topic(c, t, cx));
                }
            }
            Page::Topics => {
                if let Some(c) = cluster {
                    self.topics_page.update(cx, |p, cx| p.select_cluster(c, cx));
                }
            }
            Page::ConsumerGroups => {
                if let Some(c) = cluster {
                    self.consumer_groups_page
                        .update(cx, |p, cx| p.select_group(c, group, cx));
                }
            }
            Page::TopicConsumerGroups => {
                if let (Some(c), Some(t)) = (cluster, topic) {
                    self.topic_cg_page
                        .update(cx, |p, cx| p.select_cluster_topic(c, t, cx));
                }
            }
            _ => {}
        }
        self.switch_page(page, cx);
    }

    /// 顶栏按钮导航（压入返回栈）
    fn navigate_to(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.page != page {
            let before = self.snapshot_current(cx);
            self.switch_page(page, cx);
            self.after_navigate(before, cx);
        }
    }

    /// 搜索下拉：关闭
    fn search_dismiss(&mut self, cx: &mut Context<Self>) {
        if self.search_open {
            self.search_open = false;
            cx.notify();
        }
    }

    /// 匹配子串高亮渲染：topic 名中命中查询的部分用主题色加粗
    fn highlight_match(text: &str, query: &str, accent: Hsla) -> AnyElement {
        let lower = text.to_lowercase();
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return div().text_sm().overflow_hidden().child(text.to_string()).into_any_element();
        }
        if let Some(start) = lower.find(&q) {
            let end = start + q.len();
            let pre = text[..start].to_string();
            let hit = text[start..end].to_string();
            let post = text[end..].to_string();
            h_flex()
                .overflow_hidden()
                .text_sm()
                .child(pre)
                .child(div().text_color(accent).font_semibold().child(hit))
                .child(post)
                .into_any_element()
        } else {
            div().text_sm().overflow_hidden().child(text.to_string()).into_any_element()
        }
    }

    /// 搜索下拉：下一项
    fn search_next(&mut self, cx: &mut Context<Self>) {
        if !self.search_results.is_empty() {
            self.search_selected = (self.search_selected + 1) % self.search_results.len();
            cx.notify();
        }
    }

    /// 搜索下拉：上一项
    fn search_prev(&mut self, cx: &mut Context<Self>) {
        if !self.search_results.is_empty() {
            self.search_selected = if self.search_selected == 0 {
                self.search_results.len() - 1
            } else {
                self.search_selected - 1
            };
            cx.notify();
        }
    }

    /// 全局搜索（防抖）
    fn schedule_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().to_string();
        self.search_open = !query.trim().is_empty();
        self.search_selected = 0;
        cx.notify();

        if query.trim().is_empty() {
            self.search_results = Vec::new();
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            // 防抖：等待 300ms 后再搜
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;
            let result = crate::service::call(
                &rt,
                state,
                "topic.search",
                serde_json::json!({ "keyword": query.trim() }),
            )
            .await;
            this.update(cx, |this, cx| {
                // 仅当搜索框内容未变化时更新结果
                if this.search_input.read(cx).value().trim() == query.trim() {
                    this.search_results = result
                        .ok()
                        .and_then(|v| v.get("results").and_then(|r| r.as_array()).cloned())
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|r| {
                            Some(SearchResult {
                                cluster: r.get("cluster")?.as_str()?.to_string(),
                                topic: r.get("topic")?.as_str()?.to_string(),
                            })
                        })
                        .collect();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn select_search_result(&mut self, result: SearchResult, cx: &mut Context<Self>) {
        let before = self.snapshot_current(cx);
        self.search_open = false;
        self.search_results = Vec::new();
        let (cluster, topic) = (result.cluster.clone(), result.topic.clone());
        self.messages_page.update(cx, |page, cx| {
            page.select_cluster_topic(cluster, topic, cx)
        });
        self.switch_page(Page::Messages, cx);
        self.after_navigate(before, cx);
    }

    fn toggle_language(&mut self, cx: &mut Context<Self>) {
        let now_en = I18n::global(cx).is_zh();
        I18n::global_mut(cx).set_lang(if now_en { "en" } else { "zh" });
        let rt = TokioRuntime::handle(cx);
        if let Some(state) = Backend::state(cx) {
            let value = if now_en { "en" } else { "zh" };
            cx.spawn(async move |_this, _cx| {
                let _ = crate::service::call(
                    &rt,
                    state,
                    "settings.update",
                    serde_json::json!({ "key": "ui.language", "value": value }),
                )
                .await;
            })
            .detach();
        }
        cx.refresh_windows();
        cx.notify();
    }

    /// 搜索结果下拉（绝对定位，作为根节点最后子元素绘制在最上层）
    fn render_search_dropdown(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        if self.search_open {
            let rows: Vec<AnyElement> = if self.search_results.is_empty() {
                vec![div()
                    .p_3()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(t(cx, "layout.noTopicsFound"))
                    .into_any_element()]
            } else {
                self.search_results
                    .iter()
                    .enumerate()
                    .map(|(ix, r)| {
                        let result = r.clone();
                        let query = self.search_input.read(cx).value().to_string();
                        h_flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .cursor_pointer()
                            .border_b_1()
                            .border_color(theme.border)
                            .when(ix == self.search_selected, |el| el.bg(theme.list_active))
                            .hover(|el| el.bg(theme.list_hover))
                            .id(("search-result", ix))
                            .child(Self::highlight_match(&result.topic, &query, theme.primary))
                            .child(
                                div()
                                    .text_xs()
                                    .px_1()
                                    .rounded_md()
                                    .bg(theme.secondary)
                                    .text_color(theme.muted_foreground)
                                    .child(result.cluster.clone()),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_search_result(result.clone(), cx);
                            }))
                            .into_any_element()
                    })
                    .collect()
            };

            div()
                .id("search-dropdown")
                .absolute()
                .top(px(38.0))
                .left(px(150.0))
                .w(px(480.0))
                .max_h(px(320.0))
                .overflow_hidden()
                .bg(theme.popover)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .shadow_lg()
                .child(v_flex().children(rows))
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.search_open = false;
                    cx.notify();
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }

    /// 顶部导航栏
    fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_zh = I18n::global(cx).is_zh();
        let dark = theme.is_dark();

        h_flex()
            .h_10()
            .items_center()
            .justify_between()
            .px_2()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .size_6()
                            .rounded_md()
                            .bg(theme.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(gpui_component::white())
                            .text_xs()
                            .child("K"),
                    )
                    .child(div().font_semibold().child("Kafka Manager"))
                    .child(
                        div()
                            .w_72()
                            .ml_2()
                            .key_context("GlobalSearch")
                            .child(Input::new(&self.search_input).small()),
                    ),
            )
            .child(
                h_flex()
                    .gap_0p5()
                    .items_center()
                    // 语言切换
                    .child(
                        Button::new("toggle-lang")
                            .ghost()
                            .label(if is_zh { "EN" } else { "中" })
                            .tooltip("Language")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_language(cx);
                            })),
                    )
                    // 主题切换（下拉选择）
                    .child({
                        let current = crate::theme::current_name(cx);
                        Button::new("toggle-theme")
                            .ghost()
                            .icon(if dark { IconName::Moon } else { IconName::Sun })
                            .tooltip("Theme")
                            .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _, _| {
                                let mut menu = menu;
                                for &(name, _) in crate::theme::THEMES {
                                    menu = menu.item(
                                        PopupMenuItem::new(name)
                                            .checked(current == name)
                                            .on_click(move |_, window, cx| {
                                                crate::theme::apply_by_name(
                                                    name,
                                                    Some(window),
                                                    cx,
                                                );
                                                crate::theme::persist(cx, name);
                                            }),
                                    );
                                }
                                menu
                            })
                    })
                    // 设置
                    .child(
                        Button::new("goto-settings")
                            .ghost()
                            .icon(IconName::Settings)
                            .tooltip(t(cx, "nav.settings"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.navigate_to(Page::Settings, cx);
                            })),
                    ),
            )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // gpui-component 的对话框/通知/Sheet 层需要应用根视图手动组合渲染
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        let (bg, border) = {
            let theme = cx.theme();
            (theme.background, theme.border)
        };
        let _ = border;
        let content: AnyElement = match self.page {
            Page::Clusters => self.clusters_page.clone().into_any_element(),
            Page::Topics => self.topics_page.clone().into_any_element(),
            Page::Messages => self.messages_page.clone().into_any_element(),
            Page::ConsumerGroups => self.consumer_groups_page.clone().into_any_element(),
            Page::SchemaRegistry => self.schema_registry_page.clone().into_any_element(),
            Page::Favorites => self.favorites_page.clone().into_any_element(),
            Page::Settings => self.settings_page.clone().into_any_element(),
            Page::TopicConsumerGroups => self.topic_cg_page.clone().into_any_element(),
        };

        // 侧边栏：导航器（平铺/树形按设置），与旧版一致（无底部页面菜单）
        let is_tree = SidebarMode::is_tree(cx);
        let sidebar = v_flex().size_full().child(if is_tree {
            self.tree_navigator.clone().into_any_element()
        } else {
            self.navigator.clone().into_any_element()
        });

        let dropdown = self.render_search_dropdown(cx);

        div()
            .relative()
            .size_full()
            .key_context("Workspace")
            .on_action(cx.listener(|this, _: &SearchNextResult, _, cx| {
                this.search_next(cx);
            }))
            .on_action(cx.listener(|this, _: &SearchPrevResult, _, cx| {
                this.search_prev(cx);
            }))
            .on_action(cx.listener(|this, _: &DismissSearch, _, cx| {
                this.search_dismiss(cx);
            }))
            .on_action(cx.listener(|this, _: &FocusGlobalSearch, window, cx| {
                let handle = this.search_input.focus_handle(cx);
                handle.focus(window);
            }))
            .child(
                v_flex()
                    .size_full()
                    .bg(bg)
                    .child(self.render_top_bar(cx))
                    .child(
                        div().flex_1().overflow_hidden().child(
                            h_resizable("main-split")
                                .child(
                                    resizable_panel()
                                        .size(px(320.0))
                                        .size_range(px(200.0)..px(600.0))
                                        .child(sidebar.into_any_element()),
                                )
                                .child(
                                    resizable_panel().child(
                                        div()
                                            .size_full()
                                            .p_2()
                                            .child(
                                                div()
                                                    .size_full()
                                                    .bg(bg)
                                                    .border_1()
                                                    .border_color(border)
                                                    .rounded_lg()
                                                    .overflow_hidden()
                                                    .child(content),
                                            )
                                            .into_any_element(),
                                    ),
                                ),
                        ),
                    ),
            )
            // 下拉最后绘制，覆盖在其他内容之上
            .child(dropdown)
            // gpui-component 层：Sheet → Dialog → Notification（对话框与通知才能显示）
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}
