//! 应用根视图：路由、顶栏、侧边栏、启动流程、全局浮层渲染。

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, Global, IntoElement, MouseButton,
    Render, WeakEntity, Window,
};

use crate::backend::Backend;
use crate::i18n::{self, Language};
use crate::icons::icon;
use crate::models::*;
use crate::overlay::{self, Overlays, ToastKind};
use crate::settings::{self, UiPrefs};
use crate::theme;
use crate::views;
use crate::widgets::common::*;
use crate::widgets::text_input::TextInput;

// ==================== 全局句柄 ====================

pub struct RootHandle(pub WeakEntity<KafkaManagerApp>);
impl Global for RootHandle {}

pub fn root(cx: &App) -> Entity<KafkaManagerApp> {
    cx.global::<RootHandle>()
        .0
        .upgrade()
        .expect("root view alive")
}

pub struct BackendGlobal(pub Backend);
impl Global for BackendGlobal {}

pub fn backend(cx: &App) -> Backend {
    cx.global::<BackendGlobal>().0.clone()
}

// ==================== 路由 ====================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Clusters,
    Topics,
    Messages,
    ConsumerGroups,
    TopicConsumerGroups,
    SchemaRegistry,
    Favorites,
    Settings,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub page: Page,
    pub query: Vec<(String, String)>,
}

impl Route {
    pub fn new(page: Page) -> Self {
        Self {
            page,
            query: vec![],
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.query.retain(|(k, _)| k != key);
        if !value.is_empty() {
            self.query.push((key.to_string(), value.to_string()));
        }
        self
    }

    pub fn serialize(&self) -> String {
        let path = match self.page {
            Page::Clusters => "/clusters",
            Page::Topics => "/topics",
            Page::Messages => "/messages",
            Page::ConsumerGroups => "/consumer-groups",
            Page::TopicConsumerGroups => "/topic-consumer-groups",
            Page::SchemaRegistry => "/schema-registry",
            Page::Favorites => "/favorites",
            Page::Settings => "/settings",
        };
        if self.query.is_empty() {
            path.to_string()
        } else {
            let qs: Vec<String> = self
                .query
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding_encode(v)))
                .collect();
            format!("{}?{}", path, qs.join("&"))
        }
    }

    pub fn parse(s: &str) -> Self {
        let (path, qs) = s.split_once('?').unwrap_or((s, ""));
        let page = match path {
            "/clusters" => Page::Clusters,
            "/topics" => Page::Topics,
            "/messages" => Page::Messages,
            "/consumer-groups" => Page::ConsumerGroups,
            "/topic-consumer-groups" => Page::TopicConsumerGroups,
            "/schema-registry" => Page::SchemaRegistry,
            "/favorites" => Page::Favorites,
            "/settings" => Page::Settings,
            _ => Page::Clusters,
        };
        let query = qs
            .split('&')
            .filter(|p| !p.is_empty())
            .filter_map(|p| {
                let (k, v) = p.split_once('=').unwrap_or((p, ""));
                Some((k.to_string(), urlencoding_decode(v)))
            })
            .collect();
        Self { page, query }
    }

    pub fn title_key(&self) -> &'static str {
        match self.page {
            Page::Clusters => "nav.clusters",
            Page::Topics => "topics.title",
            Page::Messages => "nav.messages",
            Page::ConsumerGroups => "nav.consumerGroups",
            Page::TopicConsumerGroups => "nav.consumerGroups",
            Page::SchemaRegistry => "nav.schemaRegistry",
            Page::Favorites => "nav.favorites",
            Page::Settings => "nav.settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self.page {
            Page::Clusters | Page::Topics => "database",
            Page::Messages => "chat",
            Page::ConsumerGroups | Page::TopicConsumerGroups => "users",
            Page::SchemaRegistry => "book",
            Page::Favorites => "star",
            Page::Settings => "cog",
        }
    }
}

fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                c.to_string()
            } else {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf)
                    .bytes()
                    .map(|b| format!("%{:02X}", b))
                    .collect()
            }
        })
        .collect()
}

fn urlencoding_decode(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 <= bytes.len() - 1 + 1 {
            if let Ok(v) = u8::from_str_radix(s.get(i + 1..i + 3).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// ==================== 应用事件（跨组件广播） ====================

#[derive(Clone, Debug)]
pub enum AppEvent {
    /// topic 被删除（从各列表本地剔除 / 消息页清空）
    TopicDeleted { cluster: String, topic: String },
    /// 收藏变化
    FavoritesChanged,
    /// JSON 高亮模板变化
    JsonHighlightChanged,
    /// 集群列表变化（增删改后）
    ClustersChanged,
    /// 在树中选中 topic（TopicsView 双击行）
    SelectTopicInTree { cluster: String, topic: String },
    /// topic 创建成功
    TopicCreated { cluster: String },
}

// ==================== 当前页 ====================

pub enum PageView {
    None,
    Clusters(Entity<views::clusters::ClustersView>),
    Topics(Entity<views::topics::TopicsView>),
    Messages(Entity<views::messages::MessagesView>),
    ConsumerGroups(Entity<views::consumer_groups::ConsumerGroupsView>),
    TopicConsumerGroups(Entity<views::consumer_groups::TopicConsumerGroupsView>),
    SchemaRegistry(Entity<views::schema_registry::SchemaRegistryView>),
    Favorites(Entity<views::favorites::FavoritesView>),
    Settings(Entity<views::settings::SettingsView>),
}

// ==================== 根视图 ====================

pub struct KafkaManagerApp {
    pub prefs: UiPrefs,
    pub backend_ready: bool,
    pub backend_error: Option<String>,
    pub route: Route,
    pub back_stack: Vec<Route>,
    pub page_view: PageView,

    // 全局数据缓存
    pub clusters: Vec<Cluster>,
    pub groups: Vec<ClusterGroup>,
    pub cluster_status: std::collections::HashMap<String, ClusterStatus>,
    pub json_templates: Vec<crate::json::HighlightTemplate>,
    pub current_template: String,
    pub app_version: String,
    pub has_update: bool,

    // 侧边栏
    pub navigator: Entity<views::navigator::TopicNavigator>,
    pub tree_navigator: Entity<views::cluster_tree::ClusterTreeNavigator>,

    // 顶栏
    pub search_input: Entity<TextInput>,
    pub search_results: Vec<(String, String)>, // (cluster, topic)
    pub search_open: bool,
    pub searching: bool,

    // 侧栏
    pub sidebar_collapsed: bool,
    pub sidebar_resizing: bool,

    // 更新
    pub update_info: Option<views::settings::UpdateInfo>,
    pub checking_update: bool,

    pub focus_handle: FocusHandle,
}

impl KafkaManagerApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prefs = settings::load();
        theme::set_mode(if prefs.theme == "dark" {
            theme::Mode::Dark
        } else {
            theme::Mode::Light
        });
        i18n::set_language(if prefs.language == "en" {
            Language::En
        } else {
            Language::Zh
        });

        let search_input = cx.new(TextInput::new);
        search_input.update(cx, |i, _| {
            i.set_placeholder(i18n::t("topnav.searchPlaceholder"));
        });

        let app = Self {
            prefs: prefs.clone(),
            backend_ready: false,
            backend_error: None,
            route: Route::new(Page::Clusters),
            back_stack: vec![],
            page_view: PageView::None,
            clusters: vec![],
            groups: vec![],
            cluster_status: Default::default(),
            json_templates: vec![],
            current_template: "default".into(),
            app_version: String::new(),
            has_update: false,
            navigator: cx.new(|cx| views::navigator::TopicNavigator::new(window, cx)),
            tree_navigator: cx.new(|cx| views::cluster_tree::ClusterTreeNavigator::new(window, cx)),
            search_input,
            search_results: vec![],
            search_open: false,
            searching: false,
            sidebar_collapsed: false,
            sidebar_resizing: false,
            update_info: None,
            checking_update: false,
            focus_handle: cx.focus_handle(),
        };

        // 顶栏搜索：防抖 300ms
        cx.subscribe(
            &app.search_input,
            |this: &mut Self, _input, event: &crate::widgets::text_input::TextInputEvent, cx| {
                use crate::widgets::text_input::TextInputEvent;
                match event {
                    TextInputEvent::Changed => this.on_search_changed(cx),
                    TextInputEvent::EnterPressed => this.on_search_enter(cx),
                    TextInputEvent::EscapePressed => {
                        this.search_open = false;
                        cx.notify();
                    }
                }
            },
        )
        .detach();

        app
    }

    /// 启动流程：轮询后端就绪 → 初始加载
    pub fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                let (ready, err) = cx
                    .update(|_window, cx| {
                        let g = cx.global::<BackendGlobal>();
                        (g.0.is_ready(), g.0.init_error())
                    })
                    .unwrap_or((false, None));
                if let Some(e) = err {
                    this.update(cx, |app, cx| {
                        app.backend_error = Some(e);
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                if ready {
                    break;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
            }
            this.update(cx, |app, cx| {
                app.on_backend_ready(cx);
            })
            .ok();
        })
        .detach();
    }

    fn on_backend_ready(&mut self, cx: &mut Context<Self>) {
        self.backend_ready = true;
        cx.notify();
        self.reload_clusters(cx);
        self.load_app_version(cx);
        self.load_json_templates(cx);
        // 恢复上次路由
        let last = self.prefs.last_route.clone();
        if !last.is_empty() {
            let route = Route::parse(&last);
            self.navigate(route, false, cx);
        } else {
            self.navigate(Route::new(Page::Clusters), false, cx);
        }
        // 启动后自动检查更新（非手动，发现新版本仅显示红点）
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(3))
                .await;
            this.update(cx, |app: &mut Self, cx| {
                app.check_for_updates(false, cx);
            })
            .ok();
        })
        .detach();
    }

    pub fn load_app_version(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            if let Ok(v) = b.dispatch("app.version", serde_json::json!({})).await {
                let version = v
                    .get("version")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                this.update(cx, |app: &mut Self, cx| {
                    app.app_version = version;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn load_json_templates(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let mut templates = Vec::new();
            if let Ok(v) = b.dispatch("json_highlight.list", serde_json::json!({})).await {
                if let Some(arr) = v.get("templates").and_then(|x| x.as_array()) {
                    for item in arr {
                        let style_json = item
                            .get("style_json")
                            .and_then(|x| x.as_str())
                            .unwrap_or("");
                        if let Some(style) = crate::json::parse_template_style(style_json) {
                            templates.push(crate::json::HighlightTemplate {
                                id: item.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
                                name: item
                                    .get("name")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                description: item
                                    .get("description")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                is_builtin: item
                                    .get("is_builtin")
                                    .and_then(|x| x.as_bool())
                                    .unwrap_or(false),
                                style,
                            });
                        }
                    }
                }
            }
            let mut current = "default".to_string();
            if let Ok(v) = b
                .dispatch("json_highlight.get_current", serde_json::json!({}))
                .await
            {
                if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
                    current = name.to_string();
                }
            }
            this.update(cx, |app: &mut Self, cx| {
                app.json_templates = templates;
                app.current_template = current;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 当前生效的高亮模板样式
    pub fn current_highlight_style(&self) -> crate::json::TemplateStyle {
        self.json_templates
            .iter()
            .find(|t| t.name == self.current_template)
            .map(|t| t.style.clone())
            .unwrap_or_else(crate::json::default_template)
    }

    pub fn reload_clusters(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let clusters = b
                .dispatch("cluster.list", serde_json::json!({}))
                .await
                .ok()
                .and_then(|v| {
                    serde_json::from_value::<Vec<Cluster>>(
                        v.get("clusters").cloned().unwrap_or(serde_json::json!([])),
                    )
                    .ok()
                })
                .unwrap_or_default();
            let groups = b
                .dispatch("cluster_group.list", serde_json::json!({}))
                .await
                .ok()
                .and_then(|v| {
                    serde_json::from_value::<Vec<ClusterGroup>>(
                        v.get("groups").cloned().unwrap_or(serde_json::json!([])),
                    )
                    .ok()
                })
                .unwrap_or_default();
            this.update(cx, |app: &mut Self, cx| {
                app.clusters = clusters;
                app.groups = groups;
                app.publish(AppEvent::ClustersChanged, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ==================== 导航 ====================

    pub fn navigate(&mut self, route: Route, push_history: bool, cx: &mut Context<Self>) {
        if push_history && self.route != route {
            self.back_stack.push(self.route.clone());
        }
        let same_target = self.route == route;
        self.route = route.clone();
        self.prefs.last_route = route.serialize();
        settings::save(&self.prefs);

        match route.page {
            Page::Clusters => {
                let view = cx.new(|cx| views::clusters::ClustersView::new(cx));
                self.page_view = PageView::Clusters(view);
            }
            Page::Topics => {
                let view = cx.new(|cx| views::topics::TopicsView::new(route.clone(), cx));
                self.page_view = PageView::Topics(view);
            }
            Page::Messages => {
                // 与 Vue :key="cluster-topic" 一致：目标相同则复用并走 watch 路径
                let reuse = if same_target {
                    if let PageView::Messages(v) = &self.page_view {
                        Some(v.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(v) = reuse {
                    v.update(cx, |view, cx| view.on_route_changed(route.clone(), cx));
                    self.page_view = PageView::Messages(v);
                } else {
                    let view = cx.new(|cx| views::messages::MessagesView::new(route.clone(), cx));
                    self.page_view = PageView::Messages(view);
                }
            }
            Page::ConsumerGroups => {
                let view = cx.new(|cx| {
                    views::consumer_groups::ConsumerGroupsView::new(route.clone(), cx)
                });
                self.page_view = PageView::ConsumerGroups(view);
            }
            Page::TopicConsumerGroups => {
                let view = cx.new(|cx| {
                    views::consumer_groups::TopicConsumerGroupsView::new(route.clone(), cx)
                });
                self.page_view = PageView::TopicConsumerGroups(view);
            }
            Page::SchemaRegistry => {
                let view = cx.new(|cx| {
                    views::schema_registry::SchemaRegistryView::new(route.clone(), cx)
                });
                self.page_view = PageView::SchemaRegistry(view);
            }
            Page::Favorites => {
                let view = cx.new(|cx| views::favorites::FavoritesView::new(cx));
                self.page_view = PageView::Favorites(view);
            }
            Page::Settings => {
                let view = cx.new(|cx| views::settings::SettingsView::new(route.clone(), cx));
                self.page_view = PageView::Settings(view);
            }
        }
        cx.notify();
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        if let Some(prev) = self.back_stack.pop() {
            self.navigate(prev, false, cx);
        }
    }

    // ---- 页面实体 getter（避免 borrow 冲突）----
    pub fn page_messages(&self) -> Option<Entity<views::messages::MessagesView>> {
        match &self.page_view {
            PageView::Messages(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn page_consumer_groups(&self) -> Option<Entity<views::consumer_groups::ConsumerGroupsView>> {
        match &self.page_view {
            PageView::ConsumerGroups(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn page_schema_registry(&self) -> Option<Entity<views::schema_registry::SchemaRegistryView>> {
        match &self.page_view {
            PageView::SchemaRegistry(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn page_favorites(&self) -> Option<Entity<views::favorites::FavoritesView>> {
        match &self.page_view {
            PageView::Favorites(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn page_topics(&self) -> Option<Entity<views::topics::TopicsView>> {
        match &self.page_view {
            PageView::Topics(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// 跨组件事件广播（延迟一拍执行，避免在 app 自身 update 期间回读 root 实体）
    pub fn publish(&mut self, event: AppEvent, cx: &mut Context<Self>) {
        cx.defer(move |cx| {
            root(cx).update(cx, |app, cx| app.publish_now(event, cx));
        });
    }

    fn publish_now(&mut self, event: AppEvent, cx: &mut Context<Self>) {
        match &event {
            AppEvent::TopicDeleted { cluster, topic } => {
                self.navigator.update(cx, |n, cx| {
                    n.on_topic_deleted(cluster, topic, cx)
                });
                self.tree_navigator.update(cx, |n, cx| {
                    n.on_topic_deleted(cluster, topic, cx)
                });
                if let PageView::Messages(v) = &self.page_view {
                    v.update(cx, |view, cx| view.on_topic_deleted(cluster, topic, cx));
                }
            }
            AppEvent::FavoritesChanged => {
                if let PageView::Favorites(v) = &self.page_view {
                    v.update(cx, |view, cx| view.reload(cx));
                }
                if let PageView::Topics(v) = &self.page_view {
                    v.update(cx, |view, cx| view.reload_favorites(cx));
                }
                self.tree_navigator.update(cx, |n, cx| n.on_favorites_changed(cx));
            }
            AppEvent::JsonHighlightChanged => {
                if let PageView::Messages(v) = &self.page_view {
                    v.update(cx, |view, cx| view.on_highlight_changed(cx));
                }
            }
            AppEvent::ClustersChanged => {
                // 数据注入，避免子组件回读 root（此刻 app 正在被 update）
                let existing: std::collections::HashSet<String> =
                    self.clusters.iter().map(|c| c.name.clone()).collect();
                let groups = self.groups.clone();
                let clusters = self.clusters.clone();
                self.navigator.update(cx, |n, cx| {
                    n.on_clusters_changed(existing.clone(), clusters, groups.clone(), cx)
                });
                self.tree_navigator
                    .update(cx, |n, cx| n.on_clusters_changed(existing, groups, cx));
            }
            AppEvent::SelectTopicInTree { cluster, topic } => {
                self.tree_navigator.update(cx, |n, cx| {
                    n.select_topic_in_tree(cluster, topic, cx);
                });
            }
            AppEvent::TopicCreated { cluster } => {
                self.tree_navigator
                    .update(cx, |n, cx| n.on_topic_created(cluster, cx));
                if let PageView::Topics(v) = &self.page_view {
                    v.update(cx, |view, cx| view.reload(cx));
                }
            }
        }
    }

    // ==================== 顶栏搜索 ====================

    fn on_search_changed(&mut self, cx: &mut Context<Self>) {
        let keyword = self.search_input.read(cx).text();
        if keyword.trim().is_empty() {
            self.search_results = vec![];
            self.search_open = false;
            cx.notify();
            return;
        }
        self.searching = true;
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;
            let result = b
                .dispatch(
                    "topic.search",
                    serde_json::json!({ "keyword": keyword.trim() }),
                )
                .await;
            this.update(cx, |app: &mut Self, cx| {
                app.searching = false;
                // 输入已变则丢弃
                if app.search_input.read(cx).text().trim() != keyword.trim() {
                    return;
                }
                app.search_results = result
                    .ok()
                    .and_then(|v| v.get("results").cloned())
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|item| {
                        Some((
                            item.get("cluster")?.as_str()?.to_string(),
                            item.get("topic")?.as_str()?.to_string(),
                        ))
                    })
                    .collect();
                app.search_open = true;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn on_search_enter(&mut self, cx: &mut Context<Self>) {
        if let Some((cluster, topic)) = self.search_results.first().cloned() {
            self.jump_to_topic(&cluster, &topic, cx);
        }
    }

    pub fn jump_to_topic(&mut self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        self.search_open = false;
        self.search_results = vec![];
        let route = Route::new(Page::Messages)
            .with("cluster", cluster)
            .with("topic", topic);
        self.navigate(route, true, cx);
    }

    // ==================== 更新检查 ====================

    pub fn check_for_updates(&mut self, manual: bool, cx: &mut Context<Self>) {
        if self.checking_update {
            return;
        }
        self.checking_update = true;
        cx.notify();
        let current = self.app_version.clone();
        cx.spawn(async move |this, cx| {
            let url = format!(
                "https://github.com/sufar/kafka-manager/releases/download/v{}/latest.json",
                current
            );
            let result = cx
                .background_executor()
                .spawn(async move {
                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(10))
                        .build()
                        .ok()?;
                    let resp = client.get(&url).send().ok()?;
                    if !resp.status().is_success() {
                        return Some(Err(format!("HTTP {}", resp.status())));
                    }
                    let v: serde_json::Value = resp.json().ok()?;
                    Some(Ok(v))
                })
                .await;
            this.update(cx, |app: &mut Self, cx| {
                app.checking_update = false;
                match result {
                    Some(Ok(v)) => {
                        let version = v
                            .get("version")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .trim_start_matches('v')
                            .to_string();
                        let notes = v
                            .get("notes")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !version.is_empty()
                            && views::settings::version_newer(&current, &version)
                        {
                            app.update_info = Some(views::settings::UpdateInfo {
                                version: version.clone(),
                                notes,
                                url: format!(
                                    "https://github.com/sufar/kafka-manager/releases/tag/v{}",
                                    version
                                ),
                            });
                            app.has_update = true;
                            if manual {
                                if let Some(info) = app.update_info.clone() {
                                    let view = cx.new(|cx| {
                                        views::settings::UpdateDialog::new(info, cx)
                                    });
                                    overlay::open_modal(cx, view.into());
                                }
                            }
                        } else if manual {
                            overlay::toast_success(cx, i18n::t("update.latest"));
                        }
                    }
                    Some(Err(e)) => {
                        if manual {
                            if e.contains("403") {
                                overlay::toast_error(cx, "访问受限，请稍后重试");
                            } else {
                                overlay::toast_error(cx, i18n::t("update.checkFailed"));
                            }
                        }
                    }
                    None => {
                        if manual {
                            overlay::toast_error(cx, i18n::t("update.checkFailed"));
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ==================== 主题/语言 ====================

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        theme::toggle();
        self.prefs.theme = if theme::is_dark() { "dark" } else { "light" }.into();
        settings::save(&self.prefs);
        let b = backend(cx);
        let value = self.prefs.theme.clone();
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch(
                    "settings.update",
                    serde_json::json!({"key": "ui.theme", "value": value}),
                )
                .await;
        })
        .detach();
        cx.refresh_windows();
    }

    pub fn set_language(&mut self, lang: Language, cx: &mut Context<Self>) {
        i18n::set_language(lang);
        self.prefs.language = if lang == Language::En { "en" } else { "zh" }.into();
        settings::save(&self.prefs);
        let b = backend(cx);
        let value = self.prefs.language.clone();
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch(
                    "settings.update",
                    serde_json::json!({"key": "ui.language", "value": value}),
                )
                .await;
        })
        .detach();
        // 顶栏搜索框 placeholder 随语言更新
        self.search_input.update(cx, |i, _| {
            i.set_placeholder(i18n::t("topnav.searchPlaceholder"));
        });
        cx.refresh_windows();
    }

    // ==================== 渲染 ====================

    fn render_topbar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = theme::is_dark();
        let tree_mode = self.prefs.sidebar_mode == "tree";
        let collapsed = false; // 折叠状态在 sidebar 内部；顶栏始终显示面包屑
        let _ = collapsed;

        div()
            .h(px(48.))
            .flex_none()
            .flex()
            .items_center()
            .px(px(16.))
            .gap(px(8.))
            .bg(theme::navbar_bg())
            .border_b_1()
            .border_color(theme::border_base_200())
            // 面包屑
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    // 列表模式折叠开关
                    .when(!tree_mode, |d| {
                        d.child(
                            icon_btn("sidebar-toggle", "list", BtnSize::Sm).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.sidebar_collapsed = !this.sidebar_collapsed;
                                    cx.notify();
                                }),
                            ),
                        )
                    })
                    .when(self.can_go_back(), |d| {
                        d.child(
                            icon_btn("nav-back", "arrow-left", BtnSize::Sm).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.go_back(cx)),
                            ),
                        )
                    })
                    .child(
                        icon(self.route.icon())
                            .size(px(18.))
                            .text_color(theme::badge_primary_text()),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(i18n::t(self.route.title_key())),
                    ),
            )
            .child(div().flex_1())
            // 全局搜索（仅树模式）
            .when(tree_mode, |d| {
                d.child(self.render_global_search(window, cx))
            })
            // 主题切换
            .child(
                icon_btn("theme-toggle", if is_dark { "sun" } else { "moon" }, BtnSize::Sm)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.toggle_theme(cx)),
                    ),
            )
            // 语言切换
            .child(self.render_language_menu(cx))
            // 分隔线
            .child(div().w(px(1.)).h(px(20.)).bg(theme::base_content_alpha(0.15)))
            // 标题+版本
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child("Kafka Manager"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::text_secondary())
                            .child(format!("v{}", self.app_version)),
                    )
                    .when(self.has_update, |d| {
                        d.child(
                            div()
                                .size(px(7.))
                                .rounded(px(4.))
                                .bg(theme::error()),
                        )
                    }),
            )
    }

    fn render_language_menu(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = i18n::language();
        icon_btn("lang-toggle", "globe", BtnSize::Sm).on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_, event: &gpui::MouseDownEvent, _, cx| {
                let position = event.position;
                overlay::open_context_menu(
                    cx,
                    overlay::ContextMenuState {
                        position,
                        title: None,
                        separators: vec![],
                        items: vec![
                            overlay::ContextItem {
                                label: format!(
                                    "{} 中文",
                                    if current == Language::Zh { "✓" } else { "  " }
                                ),
                                icon: None,
                                danger: false,
                                action: Box::new(|cx| {
                                    root(cx).update(cx, |app, cx| {
                                        app.set_language(Language::Zh, cx)
                                    });
                                }),
                            },
                            overlay::ContextItem {
                                label: format!(
                                    "{} English",
                                    if current == Language::En { "✓" } else { "  " }
                                ),
                                icon: None,
                                danger: false,
                                action: Box::new(|cx| {
                                    root(cx).update(cx, |app, cx| {
                                        app.set_language(Language::En, cx)
                                    });
                                }),
                            },
                        ],
                    },
                );
            }),
        )
    }

    fn render_global_search(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let results = self.search_results.clone();
        let open = self.search_open && !results.is_empty();
        div()
            .relative()
            .w(px(240.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .h(px(30.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(theme::input_bg())
                    .border_1()
                    .border_color(theme::base_content_alpha(0.15))
                    .text_size(px(12.))
                    .child(
                        icon("search")
                            .size(px(13.))
                            .text_color(theme::text_secondary()),
                    )
                    .child(div().flex_1().child(self.search_input.clone())),
            )
            .when(open, |d| {
                d.child(
                    gpui::deferred(
                        div().id("app_rs_1")
                            .absolute()
                            .top(px(34.))
                            .left(px(0.))
                            .w(px(280.))
                            .max_h(px(320.))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .p(px(4.))
                            .gap(px(2.))
                            .bg(theme::context_menu_bg())
                            .border_1()
                            .border_color(theme::glass_border())
                            .rounded(px(8.))
                            .shadow_lg()
                            .children(results.into_iter().enumerate().map(
                                |(ix, (cluster, topic))| {
                                    let (cluster_c, topic_c) = (cluster.clone(), topic.clone());
                                    div()
                                        .id(("search-result", ix))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .h(px(30.))
                                        .px(px(8.))
                                        .rounded(px(5.))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme::context_menu_item_hover()))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.jump_to_topic(&cluster_c, &topic_c, cx)
                                            }),
                                        )
                                        .child(
                                            icon("database")
                                                .size(px(12.))
                                                .text_color(theme::text_secondary()),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .text_size(px(12.))
                                                .text_color(theme::text_primary())
                                                .child(topic),
                                        )
                                        .child(badge(cluster, BadgeKind::Ghost))
                                },
                            )),
                    )
                    .with_priority(50),
                )
            })
    }

    fn render_overlays(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (dropdown, has_menu, has_modal, has_confirm, modal, toasts) = {
            let overlays = cx.global::<Overlays>();
            (
                overlays.dropdown.clone(),
                overlays.context_menu.is_some(),
                overlays.modal.is_some(),
                overlays.confirm.is_some(),
                overlays.modal.clone(),
                overlays
                    .toasts
                    .iter()
                    .map(|t| (t.id, t.kind, t.message.clone()))
                    .collect::<Vec<_>>(),
            )
        };
        let mut container = div().size_full();

        // dropdown
        if let Some((position, select)) = dropdown {
            container = container.child(dropdown_popup(select, position, cx));
        }

        // context menu
        if has_menu {
            container = container.child(render_context_menu(cx));
        }

        // modal
        if has_modal {
            if let Some(modal) = modal {
                container = container.child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(theme::modal_backdrop())
                        .child(modal),
                );
            }
        }

        // confirm
        if has_confirm {
            container = container.child(render_confirm(cx));
        }

        // toasts
        if !toasts.is_empty() {
            container = container.child(
                div()
                    .absolute()
                    .bottom(px(16.))
                    .right(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(toasts.into_iter().map(|(id, kind, message)| {
                        let (bg, icon_name) = match kind {
                            ToastKind::Success => (theme::success(), "check-circle"),
                            ToastKind::Error => (theme::error(), "warn"),
                            ToastKind::Warning => (theme::warning(), "warn"),
                            ToastKind::Info => (theme::info(), "info"),
                        };
                        div()
                            .id(("toast", id as usize))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(8.))
                            .bg(bg)
                            .shadow_lg()
                            .child(
                                icon(icon_name)
                                    .size(px(16.))
                                    .text_color(gpui::white()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(gpui::white())
                                    .max_w(px(360.))
                                    .child(message),
                            )
                    })),
            );
        }

        container
    }
}

fn render_context_menu(cx: &mut App) -> impl IntoElement {
    let overlays = cx.global::<Overlays>();
    let menu = overlays.context_menu.as_ref().unwrap();
    let position = menu.position;
    let title = menu.title.clone();
    let separators = menu.separators.clone();
    let item_count = menu.items.len();
    let items_meta: Vec<(String, Option<&'static str>, bool)> = menu
        .items
        .iter()
        .map(|i| (i.label.clone(), i.icon, i.danger))
        .collect();

    let mut list = div()
        .flex()
        .flex_col()
        .w(px(200.))
        .p(px(4.))
        .bg(theme::context_menu_bg())
        .border_1()
        .border_color(theme::glass_border())
        .rounded(px(8.))
        .shadow_lg();

    if let Some(title) = title {
        list = list.child(
            div()
                .px(px(8.))
                .py(px(4.))
                .text_size(px(11.))
                .font_family("monospace")
                .text_color(theme::text_secondary())
                .border_b_1()
                .border_color(theme::border_base_200())
                .mb(px(4.))
                .overflow_hidden()
                .whitespace_nowrap()
                .child(title),
        );
    }

    for ix in 0..item_count {
        let (label, icon_name, danger) = items_meta[ix].clone();
        if separators.contains(&ix) {
            list = list.child(
                div()
                    .h(px(1.))
                    .mx(px(4.))
                    .my(px(4.))
                    .bg(theme::border_base_200()),
            );
        }
        list = list.child(
            div()
                .id(("ctx-item", ix))
                .flex()
                .items_center()
                .gap(px(8.))
                .h(px(28.))
                .px(px(8.))
                .rounded(px(5.))
                .cursor_pointer()
                .text_size(px(12.))
                .text_color(if danger {
                    theme::error()
                } else {
                    theme::text_primary()
                })
                .hover(|s| s.bg(theme::context_menu_item_hover()))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    // 取走 action 并执行
                    let action = {
                        let overlays = cx.global_mut::<Overlays>();
                        overlays
                            .context_menu
                            .as_mut()
                            .and_then(|m| m.items.get_mut(ix))
                            .map(|item| {
                                let placeholder: Box<dyn FnOnce(&mut App)> = Box::new(|_| {});
                                std::mem::replace(&mut item.action, placeholder)
                            })
                    };
                    overlay::close_context_menu(cx);
                    if let Some(action) = action {
                        action(cx);
                    }
                })
                .when_some(icon_name, |d, name| {
                    d.child(icon(name).size(px(14.)))
                })
                .child(label),
        );
    }

    div().size_full().child(
        div()
            .id("ctx-backdrop")
            .absolute()
            .inset_0()
            .size_full()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                overlay::close_context_menu(cx);
            })
            .on_mouse_down(MouseButton::Right, |_, _, cx| {
                overlay::close_context_menu(cx);
            })
            .child(
                gpui::deferred(
                    gpui::anchored()
                        .anchor(gpui::Anchor::TopLeft)
                        .position(position)
                        .snap_to_window_with_margin(px(8.))
                        .child(list),
                )
                .with_priority(200),
            ),
    )
}

fn render_confirm(cx: &mut App) -> impl IntoElement {
    let overlays = cx.global::<Overlays>();
    let confirm = overlays.confirm.as_ref().unwrap();
    let title = confirm.title.clone();
    let message = confirm.message.clone();
    let confirm_label = confirm.confirm_label.clone();
    let danger = confirm.danger;

    div()
        .absolute()
        .inset_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme::modal_backdrop())
        .child(
            div()
                .w(px(400.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .p(px(20.))
                .bg(theme::modal_bg())
                .border_1()
                .border_color(theme::glass_border())
                .rounded(px(10.))
                .shadow_lg()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme::text_secondary())
                        .child(message),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            btn("confirm-cancel", BtnKind::Ghost, BtnSize::Sm)
                                .child(i18n::t("common.cancel"))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    overlay::close_confirm(cx);
                                }),
                        )
                        .child(
                            btn(
                                "confirm-ok",
                                if danger { BtnKind::Error } else { BtnKind::Primary },
                                BtnSize::Sm,
                            )
                            .child(confirm_label)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                let action = {
                                    let overlays = cx.global_mut::<Overlays>();
                                    overlays
                                        .confirm
                                        .as_mut()
                                        .and_then(|c| c.on_confirm.take())
                                };
                                overlay::close_confirm(cx);
                                if let Some(action) = action {
                                    action(cx);
                                }
                            }),
                        ),
                ),
        )
}

impl Focusable for KafkaManagerApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for KafkaManagerApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: gpui::AnyElement = if !self.backend_ready {
            if let Some(err) = &self.backend_error {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_color(theme::error())
                            .text_size(px(13.))
                            .child(err.clone()),
                    )
                    .into_any_element()
            } else {
                loading_block(i18n::t("common.loading")).into_any_element()
            }
        } else {
            match &self.page_view {
                PageView::None => div().size_full().into_any_element(),
                PageView::Clusters(v) => v.clone().into_any_element(),
                PageView::Topics(v) => v.clone().into_any_element(),
                PageView::Messages(v) => v.clone().into_any_element(),
                PageView::ConsumerGroups(v) => v.clone().into_any_element(),
                PageView::TopicConsumerGroups(v) => v.clone().into_any_element(),
                PageView::SchemaRegistry(v) => v.clone().into_any_element(),
                PageView::Favorites(v) => v.clone().into_any_element(),
                PageView::Settings(v) => v.clone().into_any_element(),
            }
        };

        let tree_mode = self.prefs.sidebar_mode == "tree";
        let collapsed = self.sidebar_collapsed && !tree_mode;
        let sidebar_width = if tree_mode {
            px(self.prefs.sidebar_width.clamp(224., 800.))
        } else if collapsed {
            px(48.)
        } else {
            px(224.)
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg_primary())
            .text_color(theme::text_primary())
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.sidebar_resizing {
                    this.prefs.sidebar_width =
                        f32::from(event.position.x).clamp(224., 800.);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.sidebar_resizing {
                        this.sidebar_resizing = false;
                        settings::save(&this.prefs);
                        cx.notify();
                    }
                }),
            )
            .child(self.render_topbar(window, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    // 侧边栏
                    .child(
                        div()
                            .w(sidebar_width)
                            .flex_none()
                            .h_full()
                            .border_r_1()
                            .border_color(theme::border_base_200())
                            .bg(theme::glass_bg())
                            .when(self.backend_ready, |d| {
                                if tree_mode {
                                    d.child(self.tree_navigator.clone())
                                } else if collapsed {
                                    d.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .gap(px(8.))
                                            .py(px(10.))
                                            .size_full()
                                            .child(
                                                icon_btn("collapsed-clusters", "server", BtnSize::Sm)
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|_, _, _, cx| {
                                                            root(cx).update(cx, |app, cx| {
                                                                app.navigate(
                                                                    Route::new(Page::Clusters),
                                                                    true,
                                                                    cx,
                                                                )
                                                            });
                                                        }),
                                                    ),
                                            )
                                            .child(
                                                icon_btn("collapsed-favorites", "star", BtnSize::Sm)
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|_, _, _, cx| {
                                                            root(cx).update(cx, |app, cx| {
                                                                app.navigate(
                                                                    Route::new(Page::Favorites),
                                                                    true,
                                                                    cx,
                                                                )
                                                            });
                                                        }),
                                                    ),
                                            )
                                            .child(
                                                icon_btn("collapsed-history", "clock", BtnSize::Sm),
                                            ),
                                    )
                                } else {
                                    d.child(self.navigator.clone())
                                }
                            }),
                    )
                    // 树模式宽度拖拽手柄
                    .when(tree_mode, |d| {
                        d.child(
                            div()
                                .id("sidebar-resizer")
                                .w(px(4.))
                                .flex_none()
                                .h_full()
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme::primary_alpha(0.2)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.sidebar_resizing = true;
                                        cx.notify();
                                    }),
                                ),
                        )
                    })
                    // 主内容
                    .child(div().flex_1().h_full().overflow_hidden().child(content)),
            )
            .child(self.render_overlays(cx))
    }
}
