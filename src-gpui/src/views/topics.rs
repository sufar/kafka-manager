//! Topics 页面（对齐 TopicsView.vue：搜索、虚拟滚动列表、收藏星标、创建/删除对话框）

use std::collections::HashSet;

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render, UniformListScrollHandle, Window,
};

use crate::app::{backend, root, AppEvent, Route};
use crate::i18n::t;
use crate::icons::icon;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::favorite_button::FavoriteButton;
use crate::widgets::text_input::{TextInput, TextInputEvent};

pub struct TopicsView {
    cluster: Option<String>,
    search_input: Entity<TextInput>,
    topics: Vec<String>,
    loading: bool,
    error: Option<String>,
    refreshing: bool,
    favorites: HashSet<String>, // "cluster-topic"
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    handled_route_action: bool,
    search_generation: u64,
    debounced_query: String,
}

impl TopicsView {
    pub fn new(route: Route, cx: &mut Context<Self>) -> Self {
        let cluster = route.get("cluster").map(|s| s.to_string());
        let search_input = cx.new(TextInput::new);
        search_input.update(cx, |i, _| i.set_placeholder(t("common.search")));
        if let Some(q) = route.get("search") {
            search_input.update(cx, |i, cx| i.set_text(q, cx));
        }
        let this = Self {
            cluster,
            search_input: search_input.clone(),
            topics: vec![],
            loading: false,
            error: None,
            refreshing: false,
            favorites: HashSet::new(),
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            handled_route_action: false,
            search_generation: 0,
            debounced_query: route.get("search").unwrap_or("").to_string(),
        };
        // 150ms 搜索防抖
        cx.subscribe(&search_input, |this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::Changed) {
                this.search_generation += 1;
                let generation = this.search_generation;
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(150))
                        .await;
                    this.update(cx, |this, cx| {
                        if this.search_generation == generation {
                            this.debounced_query = this.search_input.read(cx).text();
                            cx.notify();
                        }
                    })
                    .ok();
                })
                .detach();
            }
        })
        .detach();
        let mut this = this;
        this.reload(cx);
        this
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else {
            return;
        };
        self.loading = true;
        self.error = None;
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch("topic.list", serde_json::json!({"cluster_id": cluster}))
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(v) => {
                        this.topics = v
                            .get("topics")
                            .and_then(|x| x.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                    }
                    Err(e) => this.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.reload_favorites(cx);
    }

    pub fn reload_favorites(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            if let Ok(v) = b.dispatch("favorite.list", serde_json::json!({})).await {
                let mut set = HashSet::new();
                if let Some(groups) = v.get("groups").and_then(|x| x.as_array()) {
                    for g in groups {
                        if let Some(items) = g.get("items").and_then(|x| x.as_array()) {
                            for item in items {
                                let cid = item.get("cluster_id").and_then(|x| x.as_str());
                                let tn = item.get("topic_name").and_then(|x| x.as_str());
                                if let (Some(cid), Some(tn)) = (cid, tn) {
                                    set.insert(format!("{}-{}", cid, tn));
                                }
                            }
                        }
                    }
                }
                this.update(cx, |this, cx| {
                    this.favorites = set;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else {
            return;
        };
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        cx.notify();
        let query = self.search_input.read(cx).text().trim().to_string();
        let b = backend(cx);
        if !query.is_empty() {
            overlay::toast_info(cx, format!("Refreshing topic \"{}\"...", query));
            let c = cluster.clone();
            let q = query.clone();
            cx.spawn(async move |_, _| {
                let _ = b
                    .dispatch(
                        "topic.refresh",
                        serde_json::json!({"cluster_id": c, "topic_name": q}),
                    )
                    .await;
            })
            .detach();
        } else {
            overlay::toast_success(cx, t("topics.refreshingBg"));
            let c = cluster.clone();
            cx.spawn(async move |_, _| {
                let _ = b
                    .dispatch("topic.refresh", serde_json::json!({"cluster_id": c}))
                    .await;
            })
            .detach();
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            this.update(cx, |this, cx| {
                this.refreshing = false;
                this.reload(cx);
            })
            .ok();
        })
        .detach();
    }

    fn open_create(&self, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else {
            return;
        };
        let view = cx.new(|cx| crate::dialogs::create_topic::CreateTopicDialog::new(cluster, cx));
        overlay::open_modal(cx, view.into());
    }

    fn open_delete(&self, topic: &str, cx: &mut Context<Self>) {
        let Some(cluster) = self.cluster.clone() else {
            return;
        };
        let view = cx.new(|cx| {
            crate::dialogs::delete_topic::DeleteTopicDialog::new(
                cluster,
                topic.to_string(),
                cx,
            )
        });
        overlay::open_modal(cx, view.into());
    }

    fn filtered_topics(&self) -> Vec<String> {
        let query = self.debounced_query.to_lowercase();
        self.topics
            .iter()
            .filter(|n| query.is_empty() || n.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }
}

impl Focusable for TopicsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TopicsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);
        let route = app.read(cx).route.clone();
        if !self.handled_route_action {
            self.handled_route_action = true;
            if route.get("action") == Some("create") {
                self.open_create(cx);
            }
        }

        let cluster = self.cluster.clone();
        let filtered = self.filtered_topics();
        let query = self.debounced_query.clone();
        let loading = self.loading;
        let error = self.error.clone();

        let mut page = div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(12.))
            .gap(px(12.))
            .overflow_hidden();

        // 页头
        page = page.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    icon_btn("topics-back", "arrow-left", BtnSize::Xs)
                        .when(!app.read(cx).can_go_back(), |b| b.opacity(0.5))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            root(cx).update(cx, |app, cx| app.go_back(cx));
                        }),
                )
                .child(
                    icon("database")
                        .size(px(20.))
                        .text_color(theme::badge_primary_text()),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme::text_primary())
                                .child(t("topics.title")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::text_secondary())
                                .child(match &cluster {
                                    Some(c) => format!("{}: {}", t("topics.cluster"), c),
                                    None => t("topics.description"),
                                }),
                        ),
                )
                .child(
                    btn("topics-refresh", BtnKind::Outline, BtnSize::Xs)
                        .when(self.refreshing || cluster.is_none(), |b| b.opacity(0.5))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                if !this.refreshing {
                                    this.refresh(cx);
                                }
                            }),
                        )
                        .child(if self.refreshing {
                            spinner(12.)
                        } else {
                            icon("refresh").size(px(12.)).into_any_element()
                        })
                        .child(t("common.refresh")),
                )
                .child(
                    btn("topics-create", BtnKind::Primary, BtnSize::Xs)
                        .when(cluster.is_none(), |b| b.opacity(0.5))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                if this.cluster.is_some() {
                                    this.open_create(cx);
                                }
                            }),
                        )
                        .child(icon("plus").size(px(12.)).into_any_element())
                        .child(t("common.create")),
                ),
        );

        // 状态机
        if cluster.is_none() {
            return page.child(
                div().flex_1().child(empty_block(
                    "database",
                    t("common.noData"),
                    t("topics.description"),
                )),
            );
        }
        if loading {
            return page.child(div().flex_1().child(loading_block(t("common.loading"))));
        }
        if let Some(err) = error {
            return page.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .px(px(14.))
                            .py(px(10.))
                            .rounded(px(8.))
                            .bg(theme::error())
                            .child(icon("warn").size(px(16.)).text_color(gpui::white()))
                            .child(
                                div().text_size(px(12.)).text_color(gpui::white()).child(err),
                            ),
                    ),
            );
        }

        // 主列表卡片
        let cluster_name = cluster.clone().unwrap_or_default();
        page.child(
            card()
                .flex_1()
                .flex()
                .flex_col()
                .overflow_hidden()
                // 卡片头：搜索 + 计数
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(12.))
                        .h(px(44.))
                        .flex_none()
                        .bg(theme::base_100())
                        .border_b_1()
                        .border_color(theme::border_base_200())
                        .child(
                            icon("search")
                                .size(px(14.))
                                .text_color(theme::text_secondary()),
                        )
                        .child(
                            div().w(px(256.)).child(
                                div()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .px(px(6.))
                                    .text_size(px(12.))
                                    .child(self.search_input.clone()),
                            ),
                        )
                        .child(div().flex_1())
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::base_content_alpha(0.5))
                                .child(format!("{} topics", filtered.len())),
                        ),
                )
                // 表头
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px(px(12.))
                        .h(px(30.))
                        .flex_none()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text_secondary())
                        .border_b_1()
                        .border_color(theme::border_base_200())
                        .child(div().flex_1().child(t("topics.topicName")))
                        .child(
                            div()
                                .w(px(64.))
                                .flex()
                                .justify_end()
                                .child(t("common.actions")),
                        ),
                )
                // 列表
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .when(filtered.is_empty(), |d| {
                            d.child(
                                div().size_full().flex().flex_col().items_center().justify_center().gap(px(8.))
                                    .child(
                                        icon("search")
                                            .size(px(36.))
                                            .text_color(theme::base_content_alpha(0.25)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme::text_secondary())
                                            .child(if query.is_empty() {
                                                t("common.noData")
                                            } else {
                                                t("topics.noMatch")
                                            }),
                                    )
                                    .when(!query.is_empty(), |d| {
                                        d.child(
                                            btn("topics-clear-search", BtnKind::Ghost, BtnSize::Sm)
                                                .child(t("common.clearSearch"))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(|this, _, _, cx| {
                                                        this.search_input
                                                            .update(cx, |i, cx| i.reset(cx));
                                                        this.debounced_query = String::new();
                                                        cx.notify();
                                                    }),
                                                ),
                                        )
                                    }),
                            )
                        })
                        .when(!filtered.is_empty(), |d| {
                            let count = filtered.len();
                            d.child(
                                gpui::uniform_list(
                                    "topics-list",
                                    count,
                                    cx.processor(move |this: &mut TopicsView, range: std::ops::Range<usize>, _window, cx| {
                                        let items = this.filtered_topics();
                                        let cluster_name = cluster_name.clone();
                                        let mut out = Vec::with_capacity(range.len());
                                        for ix in range {
                                            let Some(name) = items.get(ix).cloned() else {
                                                continue;
                                            };
                                            let is_fav = this
                                                .favorites
                                                .contains(&format!("{}-{}", cluster_name, name));
                                            out.push(
                                                div()
                                                    .id(("topic-row", ix))
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .h(px(28.))
                                                    .px(px(12.))
                                                    .cursor_pointer()
                                                    .text_size(px(11.))
                                                    .hover(|s| {
                                                        s.bg(theme::table_row_hover())
                                                    })
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener({
                                                            let name = name.clone();
                                                            let cluster_name =
                                                                cluster_name.clone();
                                                            move |_, event: &gpui::MouseDownEvent, _, cx| {
                                                                if event.click_count == 2 {
                                                                    root(cx).update(cx, |app, cx| {
                                                                        app.publish(
                                                                            AppEvent::SelectTopicInTree {
                                                                                cluster: cluster_name
                                                                                    .clone(),
                                                                                topic: name.clone(),
                                                                            },
                                                                            cx,
                                                                        );
                                                                    });
                                                                }
                                                            }
                                                        }),
                                                    )
                                                    .child(
                                                        cx.new(|cx| {
                                                            FavoriteButton::new(
                                                                cluster_name.clone(),
                                                                name.clone(),
                                                                is_fav,
                                                                cx,
                                                            )
                                                        }),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .text_color(theme::text_primary())
                                                            .child(name.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .id(("topic-del", ix))
                                                            .flex_none()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .size(px(22.))
                                                            .rounded(px(5.))
                                                            .cursor_pointer()
                                                            .text_color(theme::error())
                                                            .hover(|s| {
                                                                s.bg(theme::btn_ghost_hover())
                                                            })
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener({
                                                                    let name = name.clone();
                                                                    move |this, _, _, cx| {
                                                                        this.open_delete(&name, cx)
                                                                    }
                                                                }),
                                                            )
                                                            .child(icon("trash").size(px(12.))),
                                                    ),
                                            );
                                        }
                                        out
                                    }),
                                )
                                .track_scroll(&self.scroll)
                                .size_full(),
                            )
                        }),
                ),
        )
    }
}
