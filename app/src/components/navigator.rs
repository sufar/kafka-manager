//! 左侧导航器：视图切换（Topics/消费组）、集群选择、搜索、列表
//!
//! 布局对齐旧版 Vue TopicNavigator（flat 模式）：
//! ┌────────────────────────────┐
//! │ [视图切换▾] [图标按钮×4]    │
//! │ Cluster: [选择器▾] [刷新]   │
//! │ [搜索框]                   │
//! │ 虚拟列表（Topic / 消费组）  │
//! └────────────────────────────┘

use std::collections::HashSet;
use std::rc::Rc;

use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_component::*;
use serde_json::json;

use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::pages::clusters::notify;
use crate::state::{Backend, Page, TokioRuntime};

/// 导航器发给工作区的事件
#[derive(Clone, Debug)]
pub enum NavEvent {
    /// 打开消息页并预选集群 + Topic
    OpenMessages { cluster: String, topic: String },
    /// 打开 Topics 页并预选集群
    OpenTopics { cluster: String },
    /// 打开消费组页并预选集群
    OpenConsumerGroups { cluster: String },
    /// 切换到指定页面
    OpenPage(Page),
}

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

/// 行渲染所需的主题色（Hsla 为 Copy，可安全捕获进闭包）
#[derive(Clone, Copy)]
struct RowColors {
    hover: Hsla,
    secondary: Hsla,
    muted: Hsla,
}

impl RowColors {
    fn from_cx(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            hover: theme.list_hover,
            secondary: theme.secondary,
            muted: theme.muted_foreground,
        }
    }
}

pub struct Navigator {
    view: NavView,
    view_state: Entity<SelectState<SearchableVec<StringOption>>>,
    cluster_state: Option<Entity<SelectState<SearchableVec<StringOption>>>>,
    search_input: Entity<InputState>,
    topics: Vec<TopicItem>,
    groups: Vec<GroupItem>,
    favorites: HashSet<String>,
    loading: bool,
    refreshing: bool,
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
                    cx.notify();
                    this.load_data(cx);
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &search_input,
            |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.load_data(cx);
                }
            },
        ));

        let this = Self {
            view: NavView::Topics,
            view_state,
            cluster_state: None,
            search_input,
            topics: Vec::new(),
            groups: Vec::new(),
            favorites: HashSet::new(),
            loading: true,
            refreshing: false,
            _subscriptions: subscriptions,
        };
        this.load_clusters(window, cx);
        this
    }

    fn selected_cluster(&self, cx: &App) -> Option<String> {
        let value = self
            .cluster_state
            .as_ref()?
            .read(cx)
            .selected_value()
            .map(|s| s.to_string())?;
        // "全部集群" 用空串表示
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    fn load_clusters(&self, window: &mut Window, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn_in(window, async move |this, cx| {
            let result = crate::service::call(&rt, state, "cluster.list", json!({})).await;
            let arr = result
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();

            this.update_in(cx, |this, window, cx| {
                let all_label = t(cx, "navigator.allClusters");
                let mut options = vec![StringOption::new(all_label, "")];
                options.extend(arr.iter().filter_map(|c| {
                    let name = c.get("name")?.as_str()?.to_string();
                    Some(StringOption::new(name.clone(), name))
                }));
                let select = cx.new(|cx| {
                    SelectState::new(
                        SearchableVec::new(options),
                        Some(IndexPath::new(0)),
                        window,
                        cx,
                    )
                });
                let sub = cx.subscribe(
                    &select,
                    |this, _, event: &SelectEvent<SearchableVec<StringOption>>, cx| {
                        if matches!(event, SelectEvent::Confirm(Some(_))) {
                            this.load_data(cx);
                        }
                    },
                );
                this._subscriptions.push(sub);
                this.cluster_state = Some(select);
                this.loading = false;
                cx.notify();
                this.load_data(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 按当前视图加载列表数据
    fn load_data(&self, cx: &mut Context<Self>) {
        match self.view {
            NavView::Topics => self.load_topics(cx),
            NavView::ConsumerGroups => self.load_groups(cx),
        }
    }

    fn load_topics(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let search = self.search_input.read(cx).value().to_string();
        // 未选集群（全部）→ 空数组表示所有集群
        let cluster_ids: Vec<String> = self
            .selected_cluster(cx)
            .map(|c| vec![c])
            .unwrap_or_default();

        cx.spawn(async move |this, cx| {
            let topics = crate::service::call(
                &rt,
                state.clone(),
                "topic.list_with_cluster",
                json!({ "cluster_ids": cluster_ids, "search": search, "limit": 100000 }),
            )
            .await;
            let favorites = crate::service::call(&rt, state, "favorite.list", json!({})).await;

            this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(value) = topics {
                    this.topics = value
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
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_groups(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let search = self.search_input.read(cx).value().to_string();
        let cluster_ids: Vec<String> = self
            .selected_cluster(cx)
            .map(|c| vec![c])
            .unwrap_or_default();

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "consumer_group.list",
                json!({ "cluster_ids": cluster_ids, "search": search, "limit": 100000 }),
            )
            .await;

            this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(value) = result {
                    this.groups = value
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
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 刷新（从集群重新拉取元数据）
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        cx.notify();

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let cluster = self.selected_cluster(cx);

        cx.spawn(async move |this, cx| {
            let _ = crate::service::call(
                &rt,
                state.clone(),
                "topic.refresh",
                json!({ "cluster_id": cluster }),
            )
            .await;
            let _ = crate::service::call(
                &rt,
                state,
                "consumer_group.refresh",
                json!({ "cluster_id": cluster }),
            )
            .await;
            this.update(cx, |this, cx| {
                this.refreshing = false;
                cx.notify();
                this.load_data(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 切换收藏
    fn toggle_favorite(&mut self, item: TopicItem, cx: &mut Context<Self>) {
        let key = format!("{}\u{1}{}", item.cluster, item.name);
        let is_fav = self.favorites.contains(&key);
        if is_fav {
            self.favorites.remove(&key);
        } else {
            self.favorites.insert(key);
        }
        cx.notify();

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = if is_fav {
                crate::service::call(
                    &rt,
                    state.clone(),
                    "favorite.delete_by_topic",
                    json!({ "cluster_id": item.cluster, "topic_name": item.name }),
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
                            json!({ "group_id": gid, "cluster_id": item.cluster, "topic_name": item.name }),
                        )
                        .await
                    }
                    None => Err("No favorite group available".to_string()),
                }
            };
            if let Err(e) = result {
                cx.update(|cx| notify(cx, NotificationType::Error, e)).ok();
                this.update(cx, |this, cx| this.load_topics(cx)).ok();
            }
        })
        .detach();
    }

    fn emit_nav(&self, event: NavEvent, cx: &mut Context<Self>) {
        cx.emit(event);
    }

    /// 头部图标按钮：跳转到页面
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
            .on_click(cx.listener(move |this, _, _, cx| {
                this.emit_nav(NavEvent::OpenPage(page), cx);
            }))
    }

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
                        Button::new("nav-refresh")
                            .ghost()
                            .icon(IconName::Redo2)
                            .loading(self.refreshing)
                            .tooltip(t(cx, "common.refresh"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh(cx);
                            })),
                    ),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_1()
            .px_1p5()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Cluster:"),
            )
            .children(
                self.cluster_state
                    .as_ref()
                    .map(|s| div().flex_1().child(Select::new(s).small()).into_any_element()),
            )
    }

    fn render_topic_row(
        entity: Entity<Navigator>,
        ix: usize,
        item: TopicItem,
        is_fav: bool,
        colors: &RowColors,
    ) -> AnyElement {
        let item_fav = item.clone();
        let item_open = item.clone();
        let item_badge = item.clone();
        let entity_fav = entity.clone();
        let entity_badge = entity.clone();

        h_flex()
            .items_center()
            .gap_1p5()
            .px_1p5()
            .py_1()
            .rounded_md()
            .w_full()
            .hover(|el| el.bg(colors.hover))
            .child(
                Button::new(("fav", ix))
                    .ghost()
                    .icon(if is_fav { IconName::Star } else { IconName::StarOff })
                    .xsmall()
                    .on_click(move |_, _, cx| {
                        entity_fav.update(cx, |this, cx| {
                            this.toggle_favorite(item_fav.clone(), cx)
                        });
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .id(("topic", ix))
                    .cursor_pointer()
                    .text_xs()
                    .whitespace_nowrap()
                    .child(item.name.clone())
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |this, cx| {
                            this.emit_nav(
                                NavEvent::OpenMessages {
                                    cluster: item_open.cluster.clone(),
                                    topic: item_open.name.clone(),
                                },
                                cx,
                            );
                        });
                    }),
            )
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
                        entity_badge.update(cx, |this, cx| {
                            this.emit_nav(
                                NavEvent::OpenTopics {
                                    cluster: item_badge.cluster.clone(),
                                },
                                cx,
                            );
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
        let item_badge = item.clone();
        let entity_badge = entity.clone();
        let item_open = item.clone();

        h_flex()
            .items_center()
            .gap_1p5()
            .px_1p5()
            .py_1()
            .rounded_md()
            .w_full()
            .hover(|el| el.bg(colors.hover))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .id(("group", ix))
                    .cursor_pointer()
                    .text_xs()
                    .whitespace_nowrap()
                    .child(item.name.clone())
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |this, cx| {
                            this.emit_nav(
                                NavEvent::OpenConsumerGroups {
                                    cluster: item_open.cluster.clone(),
                                },
                                cx,
                            );
                        });
                    }),
            )
            .child(
                div()
                    .id(("gbadge", ix))
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
                        entity_badge.update(cx, |this, cx| {
                            this.emit_nav(
                                NavEvent::OpenConsumerGroups {
                                    cluster: item_badge.cluster.clone(),
                                },
                                cx,
                            );
                        });
                    }),
            )
            .into_any_element()
    }
}

impl Render for Navigator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let search_bar = div()
            .px_1p5()
            .py_1()
            .border_b_1()
            .border_color(theme.border)
            .child(Input::new(&self.search_input).small());

        let entity = cx.entity();
        let content: AnyElement = if self.loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(gpui_component::spinner::Spinner::new())
                .into_any_element()
        } else {
            match self.view {
                NavView::Topics => {
                    if self.topics.is_empty() {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "common.noData"))
                            .into_any_element()
                    } else {
                        let topics = Rc::new(self.topics.clone());
                        let favorites = Rc::new(self.favorites.clone());
                        let colors = RowColors::from_cx(cx);
                        let count = topics.len();
                        uniform_list("nav-topics", count, move |range, _window, _cx| {
                            range
                                .map(|ix| {
                                    let item = topics[ix].clone();
                                    let is_fav = favorites
                                        .contains(&format!("{}\u{1}{}", item.cluster, item.name));
                                    Navigator::render_topic_row(
                                        entity.clone(),
                                        ix,
                                        item,
                                        is_fav,
                                        &colors,
                                    )
                                })
                                .collect()
                        })
                        .into_any_element()
                    }
                }
                NavView::ConsumerGroups => {
                    if self.groups.is_empty() {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "consumerGroups.noData"))
                            .into_any_element()
                    } else {
                        let groups = Rc::new(self.groups.clone());
                        let colors = RowColors::from_cx(cx);
                        let count = groups.len();
                        uniform_list("nav-groups", count, move |range, _window, _cx| {
                            range
                                .map(|ix| {
                                    Navigator::render_group_row(
                                        entity.clone(),
                                        ix,
                                        groups[ix].clone(),
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

        v_flex()
            .size_full()
            .bg(theme.sidebar)
            .child(self.render_header(cx))
            .child(self.render_status_bar(cx))
            .child(search_bar)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
