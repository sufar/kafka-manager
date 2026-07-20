//! 工作区：顶部导航栏 + 可拖拽左侧导航器 + 页面内容
//!
//! 布局对齐旧版 Vue ModernLayout：
//! ┌──────────────────────────────────────┐
//! │ TopNavBar (h-10): logo/搜索/语言/主题/设置 │
//! ├───────────┬──────────────────────────┤
//! │ Navigator │        页面内容          │
//! │ (可拖拽)  │                          │
//! └───────────┴──────────────────────────┘

use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
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

/// 全局搜索结果
#[derive(Clone, Debug)]
struct SearchResult {
    cluster: String,
    topic: String,
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

        // 全局搜索（防抖 300ms）
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.schedule_search(cx);
                }
            },
        ));

        // 启动后从设置中加载语言与主题
        let rt = TokioRuntime::handle(cx);
        let state = Backend::state(cx);
        cx.spawn(async move |_this, cx| {
            let result = match state {
                Some(state) => crate::service::call(
                    &rt,
                    state,
                    "settings.get",
                    serde_json::json!({ "keys": ["ui.language", "ui.theme", "ui.sidebar_mode"] }),
                )
                .await
                .ok(),
                None => None,
            };
            if let Some(value) = result {
                cx.update(|cx| {
                    if let Some(settings) = value.get("settings").and_then(|v| v.as_array()) {
                        for item in settings {
                            let key = item.get("key").and_then(|k| k.as_str()).unwrap_or("");
                            let val = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            match (key, val) {
                                ("ui.language", "en") => I18n::global_mut(cx).set_lang("en"),
                                ("ui.language", "zh") => I18n::global_mut(cx).set_lang("zh"),
                                ("ui.theme", "dark") => {
                                    gpui_component::Theme::change(ThemeMode::Dark, None, cx)
                                }
                                ("ui.theme", "light") => {
                                    gpui_component::Theme::change(ThemeMode::Light, None, cx)
                                }
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
            _subscriptions: subscriptions,
        }
    }

    fn switch_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    /// 处理导航器（平铺/树形）发来的导航事件
    fn handle_nav_event(&mut self, event: &NavEvent, cx: &mut Context<Self>) {
        match event {
            NavEvent::OpenMessages { cluster, topic } => {
                self.messages_page.update(cx, |page, cx| {
                    page.select_cluster_topic(cluster.clone(), topic.clone(), cx)
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
            NavEvent::OpenPage(page) => {
                self.switch_page(*page, cx);
            }
        }
    }

    /// 全局搜索（防抖）
    fn schedule_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().to_string();
        self.search_open = !query.trim().is_empty();
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
        self.search_open = false;
        self.search_results = Vec::new();
        let (cluster, topic) = (result.cluster.clone(), result.topic.clone());
        self.messages_page.update(cx, |page, cx| {
            page.select_cluster_topic(cluster, topic, cx)
        });
        self.switch_page(Page::Messages, cx);
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

    fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dark = !cx.theme().is_dark();
        gpui_component::Theme::change(
            if dark { ThemeMode::Dark } else { ThemeMode::Light },
            Some(window),
            cx,
        );
        let rt = TokioRuntime::handle(cx);
        if let Some(state) = Backend::state(cx) {
            let value = if dark { "dark" } else { "light" };
            cx.spawn(async move |_this, _cx| {
                let _ = crate::service::call(
                    &rt,
                    state,
                    "settings.update",
                    serde_json::json!({ "key": "ui.theme", "value": value }),
                )
                .await;
            })
            .detach();
        }
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
                        h_flex()
                            .id(("search-result", ix))
                            .items_center()
                            .justify_between()
                            .p_2()
                            .cursor_pointer()
                            .border_b_1()
                            .border_color(theme.border)
                            .hover(|el| el.bg(theme.list_hover))
                            .child(
                                div()
                                    .text_sm()
                                    .overflow_hidden()
                                    .child(result.topic.clone()),
                            )
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
                    // 主题切换
                    .child(
                        Button::new("toggle-theme")
                            .ghost()
                            .icon(if dark { IconName::Sun } else { IconName::Moon })
                            .tooltip("Theme")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_theme(window, cx);
                            })),
                    )
                    // 设置
                    .child(
                        Button::new("goto-settings")
                            .ghost()
                            .icon(IconName::Settings)
                            .tooltip(t(cx, "nav.settings"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.switch_page(Page::Settings, cx);
                            })),
                    ),
            )
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
    }
}
