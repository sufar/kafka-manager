//! 集群管理页（对齐 ClustersView.vue：搜索、分组卡片、集群卡片、统计、右键菜单）

use std::collections::HashMap;

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
use crate::widgets::common::*;
use crate::widgets::text_input::{TextInput, TextInputEvent};

#[derive(Clone, Default)]
struct ClusterCardStatus {
    health: Option<bool>,
    pools: usize,
    connections: usize,
    latency_ms: Option<u64>,
}

pub struct ClustersView {
    search_input: Entity<TextInput>,
    status: HashMap<String, ClusterCardStatus>,
    testing: std::collections::HashSet<String>,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
    handled_route_action: bool,
}

impl ClustersView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(TextInput::new);
        search_input.update(cx, |i, _| i.set_placeholder(t("clusters.searchPlaceholder")));
        let this = Self {
            search_input: search_input.clone(),
            status: HashMap::new(),
            testing: Default::default(),
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            handled_route_action: false,
        };
        cx.subscribe(&search_input, |_this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::Changed) {
                cx.notify();
            }
        })
        .detach();
        let this = this;
        // 延迟到 app update 结束后加载（构造函数可能在 app.navigate 内被调用）
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| this.load_all_status(cx)).ok();
        })
        .detach();
        this
    }

    fn load_all_status(&mut self, cx: &mut Context<Self>) {
        let clusters = root(cx).read(cx).clusters.clone();
        for c in clusters {
            self.load_cluster_metrics(&c.name, cx);
        }
    }

    fn load_cluster_metrics(&mut self, name: &str, cx: &mut Context<Self>) {
        let b = backend(cx);
        let name = name.to_string();
        cx.spawn(async move |this, cx| {
            let metrics = b
                .dispatch(
                    "connection.metrics",
                    serde_json::json!({"cluster_id": name}),
                )
                .await
                .ok();
            let health = b
                .dispatch(
                    "connection.health_check",
                    serde_json::json!({"cluster_id": name}),
                )
                .await
                .ok();
            this.update(cx, |this, cx| {
                let entry = this.status.entry(name.clone()).or_default();
                if let Some(m) = metrics {
                    let csize = m
                        .get("consumer_pool_size")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0) as usize;
                    let psize = m
                        .get("producer_pool_size")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0) as usize;
                    let cavail = m
                        .get("consumer_pool_available")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0) as usize;
                    let pavail = m
                        .get("producer_pool_available")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0) as usize;
                    entry.pools = csize + psize;
                    entry.connections = cavail + pavail;
                }
                if let Some(h) = health {
                    entry.health = h.get("healthy").and_then(|x| x.as_bool());
                    entry.latency_ms = h.get("latency_ms").and_then(|x| x.as_u64());
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn test_cluster(&mut self, cluster: &Cluster, cx: &mut Context<Self>) {
        if self.testing.contains(&cluster.name) {
            return;
        }
        self.testing.insert(cluster.name.clone());
        cx.notify();
        let b = backend(cx);
        let id = cluster.id;
        let name = cluster.name.clone();
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch("cluster.test", serde_json::json!({"cluster_id": id}))
                .await;
            this.update(cx, |this, cx| {
                this.testing.remove(&name);
                match result {
                    Ok(v) => {
                        let success = v.get("success").and_then(|x| x.as_bool()).unwrap_or(false);
                        let latency = v.get("latency_ms").and_then(|x| x.as_u64());
                        let entry = this.status.entry(name.clone()).or_default();
                        entry.health = Some(success);
                        entry.latency_ms = latency;
                        if success {
                            overlay::toast_success(
                                cx,
                                format!("{} ({}ms)", t("clusters.testSuccess"), latency.unwrap_or(0)),
                            );
                        } else {
                            overlay::toast_error(cx, t("clusters.testFailed"));
                        }
                        root(cx).update(cx, |app, cx| {
                            app.tree_navigator.update(cx, |n, cx| {
                                n.set_health(&name, Some(success), None, cx);
                            });
                        });
                    }
                    Err(e) => overlay::toast_error(cx, e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_create(&self, cx: &mut Context<Self>) {
        let view = cx.new(|cx| crate::dialogs::cluster_form::ClusterFormDialog::new(None, cx));
        overlay::open_modal(cx, view.into());
    }

    fn open_edit(&self, cluster: Cluster, cx: &mut Context<Self>) {
        let view = cx.new(|cx| crate::dialogs::cluster_form::ClusterFormDialog::new(Some(cluster), cx));
        overlay::open_modal(cx, view.into());
    }

    fn open_groups(&self, cx: &mut Context<Self>) {
        let view = cx.new(crate::dialogs::group_manage::GroupManageDialog::new);
        overlay::open_modal(cx, view.into());
    }

    fn delete_cluster(&self, cluster: &Cluster, cx: &mut Context<Self>) {
        crate::dialogs::cluster_menu::remove_cluster(&cluster.name, cx);
    }

    fn render_cluster_card(
        &self,
        cluster: &Cluster,
        ix: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let status = self.status.get(&cluster.name).cloned().unwrap_or_default();
        let testing = self.testing.contains(&cluster.name);
        let health_color = match status.health {
            Some(true) => theme::health_ok(),
            Some(false) => theme::health_bad(),
            None => theme::health_unknown(),
        };
        let c = cluster.clone();
        let c_menu = cluster.clone();
        let c_test = cluster.clone();
        let name = cluster.name.clone();

        div()
            .id(("cluster-card", ix))
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .rounded(px(10.))
            .bg(theme::glass_bg())
            .border_1()
            .border_color(theme::glass_border())
            .shadow_md()
            .cursor_pointer()
            .hover(|s| s.border_color(theme::primary_alpha(0.35)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, _, cx| {
                    root(cx).update(cx, |app, cx| {
                        let route = Route::new(Page::Topics).with("cluster", &name);
                        app.navigate(route, true, cx);
                    });
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |_this, event: &gpui::MouseDownEvent, _, cx| {
                    let pos = event.position;
                    let name = c_menu.name.clone();
                    root(cx).update(cx, |app, cx| {
                        app.tree_navigator.update(cx, |n, cx| {
                            n.open_cluster_menu(&name, pos, cx);
                        });
                    });
                }),
            )
            // 头部行
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .size(px(32.))
                            .rounded(px(8.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme::gradient_1())
                            .child(icon("server").size(px(16.)).text_color(gpui::white())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(cluster.name.clone()),
                    )
                    .child(div().size(px(8.)).rounded(px(4.)).bg(health_color))
                    .child(
                        div()
                            .id(("card-test", ix))
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
                                    this.test_cluster(&c_test, cx)
                                }),
                            )
                            .child(if testing {
                                spinner(12.)
                            } else {
                                icon("wifi").size(px(13.)).into_any_element()
                            }),
                    )
                    .child(
                        icon_btn(("card-edit", ix), "pencil", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener({
                                let c = c.clone();
                                move |this, _, _, cx| this.open_edit(c.clone(), cx)
                            }),
                        ),
                    )
                    .child(
                        div()
                            .id(("card-del", ix))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .text_color(theme::error())
                            .hover(|s| s.bg(theme::btn_ghost_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener({
                                    let c = c.clone();
                                    move |this, _, _, cx| this.delete_cluster(&c, cx)
                                }),
                            )
                            .child(icon("trash").size(px(13.))),
                    ),
            )
            // brokers
            .child(
                div()
                    .text_size(px(11.))
                    .font_family("monospace")
                    .text_color(theme::text_secondary())
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(cluster.brokers.clone()),
            )
            // 统计行
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .pt(px(6.))
                    .border_t_1()
                    .border_color(theme::base_content_alpha(0.08))
                    .child(stat_item(
                        t("clusters.pools"),
                        status.pools.to_string(),
                    ))
                    .child(stat_item(
                        t("clusters.connections"),
                        status.connections.to_string(),
                    ))
                    .child(stat_item(
                        t("clusters.latency"),
                        status
                            .latency_ms
                            .map(|l| format!("{}ms", l))
                            .unwrap_or_else(|| "-".into()),
                    )),
            )
    }
}

fn stat_item(label: String, value: String) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::text_secondary())
                .child(label),
        )
        .child(
            div()
                .text_size(px(11.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme::text_primary())
                .child(value),
        )
}

impl Focusable for ClustersView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ClustersView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);
        let route = app.read(cx).route.clone();
        // 处理路由 action 参数（来自树右键菜单）
        if !self.handled_route_action {
            self.handled_route_action = true;
            match route.get("action") {
                Some("create") => self.open_create(cx),
                Some("edit") => {
                    if let Some(name) = route.get("cluster") {
                        if let Some(c) = app
                            .read(cx)
                            .clusters
                            .iter()
                            .find(|c| c.name == name)
                            .cloned()
                        {
                            self.open_edit(c, cx);
                        }
                    }
                }
                _ => {}
            }
        }

        let clusters = app.read(cx).clusters.clone();
        let groups = app.read(cx).groups.clone();
        let keyword = self.search_input.read(cx).text().to_lowercase();
        let filtered: Vec<Cluster> = clusters
            .iter()
            .filter(|c| {
                keyword.is_empty()
                    || c.name.to_lowercase().contains(&keyword)
                    || c.brokers.to_lowercase().contains(&keyword)
            })
            .cloned()
            .collect();

        let ungrouped: Vec<Cluster> = filtered
            .iter()
            .filter(|c| c.group_id.is_none())
            .cloned()
            .collect();
        let keyword_empty = keyword.is_empty();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(12.))
            .gap(px(12.))
            .overflow_hidden()
            // 页头
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme::text_primary())
                                    .child(t("clusters.title")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child(t("clusters.description")),
                            ),
                    )
                    .child(
                        btn("clusters-create", BtnKind::Primary, BtnSize::Sm)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.open_create(cx)),
                            )
                            .child(icon("plus").size(px(13.)).into_any_element())
                            .child(t("clusters.addCluster")),
                    ),
            )
            // 搜索行
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div().w(px(288.)).child(
                            div()
                                .h(px(30.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
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
                        ),
                    )
                    .child(
                        btn("clusters-manage-groups", BtnKind::Outline, BtnSize::Sm)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.open_groups(cx)),
                            )
                            .child(icon("folder").size(px(13.)).into_any_element())
                            .child(t("clusters.manageGroups")),
                    ),
            )
            // 主区域
            .child(
                div().id("views_clusters_rs_2")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .when(clusters.is_empty(), |d| {
                        d.child(empty_block(
                            "server",
                            t("clusters.empty"),
                            t("clusters.emptyDesc"),
                        ))
                    })
                    .when(!clusters.is_empty() && filtered.is_empty(), |d| {
                        d.child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap(px(8.))
                                .child(
                                    icon("search")
                                        .size(px(40.))
                                        .text_color(theme::base_content_alpha(0.25)),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme::text_secondary())
                                        .child(t("clusters.noResults")),
                                )
                                .child(
                                    btn("clear-cluster-search", BtnKind::Ghost, BtnSize::Sm)
                                        .child(t("common.clearSearch"))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.search_input.update(cx, |i, cx| i.reset(cx));
                                            }),
                                        ),
                                ),
                        )
                    })
                    .when(!filtered.is_empty(), |d| {
                        let mut content = div().flex().flex_col().gap(px(14.)).pb(px(12.));
                        // 无分组集群
                        if !ungrouped.is_empty() {
                            content = content.child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap(px(10.))
                                    .children(ungrouped.iter().enumerate().map(|(ix, c)| {
                                        div()
                                            .w(px(300.))
                                            .child(self.render_cluster_card(c, ix, cx))
                                    })),
                            );
                        }
                        // 分组区
                        for (gix, g) in groups.iter().enumerate() {
                            let members: Vec<Cluster> = filtered
                                .iter()
                                .filter(|c| c.group_id == Some(g.id))
                                .cloned()
                                .collect();
                            if members.is_empty() && !keyword_empty {
                                continue;
                            }
                            content = content.child(
                                card()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        // 分组头
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .px(px(12.))
                                            .py(px(10.))
                                            .border_b_1()
                                            .border_color(theme::border_base_200())
                                            .child(
                                                icon("folder")
                                                    .size(px(16.))
                                                    .text_color(theme::badge_primary_text()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(theme::text_primary())
                                                    .child(g.name.clone()),
                                            )
                                            .when_some(g.description.clone(), |d, desc| {
                                                d.child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(theme::text_secondary())
                                                        .child(desc),
                                                )
                                            })
                                            .child(badge(
                                                format!("{} {}", members.len(), t("clusters.clusters")),
                                                BadgeKind::Ghost,
                                            ))
                                            .child(div().flex_1()),
                                    )
                                    .child(
                                        div().p(px(10.)).child(
                                            div()
                                                .flex()
                                                .flex_wrap()
                                                .gap(px(10.))
                                                .children(members.iter().enumerate().map(
                                                    |(ix, c)| {
                                                        div().w(px(300.)).child(
                                                            self.render_cluster_card(
                                                                c,
                                                                gix * 1000 + ix + 100,
                                                                cx,
                                                            ),
                                                        )
                                                    },
                                                )),
                                        ),
                                    ),
                            );
                        }
                        d.child(content)
                    }),
            )
    }
}
