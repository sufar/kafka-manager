//! 收藏页（对齐 FavoritesView.vue + TopicFavorites.vue）

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
use crate::widgets::common::*;
use crate::widgets::text_input::{TextInput, TextInputEvent};

pub struct FavoritesView {
    groups: Vec<FavoriteGroup>,
    loading: bool,
    expanded: HashSet<i64>,
    ever_expanded: bool,
    search_inputs: HashMap<i64, Entity<TextInput>>,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
}

impl FavoritesView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let this = Self {
            groups: vec![],
            loading: false,
            expanded: HashSet::new(),
            ever_expanded: false,
            search_inputs: HashMap::new(),
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        };
        this.reload(cx);
        this
    }

    pub fn reload(&self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.loading = true;
                cx.notify();
            })
            .ok();
            let result = b.dispatch("favorite.list", serde_json::json!({})).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(v) = result {
                    this.groups = v
                        .get("groups")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|g| {
                                    Some(FavoriteGroup {
                                        id: g.get("id")?.as_i64()?,
                                        name: g.get("name")?.as_str()?.to_string(),
                                        description: g
                                            .get("description")
                                            .and_then(|x| x.as_str())
                                            .map(|s| s.to_string()),
                                        sort_order: g
                                            .get("sort_order")
                                            .and_then(|x| x.as_i64())
                                            .unwrap_or(0),
                                        created_at: None,
                                        items: g
                                            .get("items")
                                            .and_then(|x| x.as_array())
                                            .map(|items| {
                                                items
                                                    .iter()
                                                    .filter_map(|item| {
                                                        Some(FavoriteItem {
                                                            id: item.get("id")?.as_i64()?,
                                                            group_id: item
                                                                .get("group_id")
                                                                .and_then(|x| x.as_i64())
                                                                .unwrap_or(0),
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
                                                            description: item
                                                                .get("description")
                                                                .and_then(|x| x.as_str())
                                                                .map(|s| s.to_string()),
                                                            sort_order: item
                                                                .get("sort_order")
                                                                .and_then(|x| x.as_i64())
                                                                .unwrap_or(0),
                                                            created_at: None,
                                                        })
                                                    })
                                                    .collect()
                                            })
                                            .unwrap_or_default(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    // 首次加载自动展开第一组
                    if !this.ever_expanded && !this.groups.is_empty() {
                        this.expanded.insert(this.groups[0].id);
                        this.ever_expanded = true;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn search_input(&mut self, group_id: i64, cx: &mut Context<Self>) -> Entity<TextInput> {
        if let Some(input) = self.search_inputs.get(&group_id) {
            return input.clone();
        }
        let input = cx.new(TextInput::new);
        input.update(cx, |i, _| {
            i.set_placeholder(t("favorites.searchPlaceholder"))
        });
        cx.subscribe(&input, |_, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::Changed) {
                cx.notify();
            }
        })
        .detach();
        self.search_inputs.insert(group_id, input.clone());
        input
    }

    fn filtered_items(&self, group: &FavoriteGroup, cx: &App) -> Vec<FavoriteItem> {
        let query = self
            .search_inputs
            .get(&group.id)
            .map(|i| i.read(cx).text().to_lowercase())
            .unwrap_or_default();
        group
            .items
            .iter()
            .filter(|item| {
                query.is_empty()
                    || item.topic_name.to_lowercase().contains(&query)
                    || item
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect()
    }

    fn toggle_group(&mut self, id: i64, cx: &mut Context<Self>) {
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
        cx.notify();
    }

    fn navigate_to_topic(&self, cluster: &str, topic: &str, cx: &mut Context<Self>) {
        let (cluster, topic) = (cluster.to_string(), topic.to_string());
        root(cx).update(cx, |app, cx| {
            if app.prefs.sidebar_mode == "tree" {
                app.tree_navigator.update(cx, |n, cx| {
                    n.highlight_and_select_topic(&cluster, &topic, cx);
                });
            } else {
                let route = Route::new(Page::Messages)
                    .with("cluster", &cluster)
                    .with("topic", &topic);
                app.navigate(route, true, cx);
            }
        });
    }

    fn delete_group(&self, group: &FavoriteGroup, cx: &mut Context<Self>) {
        overlay::confirm(
            cx,
            t("common.confirm"),
            "确定要删除这个分组吗？分组内的收藏也会被删除。",
            true,
            {
                let gid = group.id;
                move |cx| {
                    let b = backend(cx);
                    cx.spawn(async move |cx| {
                        let result = b
                            .dispatch(
                                "favorite.group.delete",
                                serde_json::json!({"group_id": gid}),
                            )
                            .await;
                        cx.update(|cx| {
                            match result {
                                Ok(_) => {
                                    overlay::toast_success(cx, "分组删除成功");
                                    root(cx).update(cx, |app, cx| {
                                        if let Some(v) = app.page_favorites() {
                                            v.update(cx, |view, cx| view.reload(cx));
                                        }
                                    });
                                }
                                Err(e) => overlay::toast_error(cx, e),
                            }
})
                    })
                    .detach();
                }
            },
        );
    }

    fn delete_item(&self, item: &FavoriteItem, cx: &mut Context<Self>) {
        overlay::confirm(cx, t("common.confirm"), "确定要删除这个收藏吗？", true, {
            let id = item.id;
            move |cx| {
                let b = backend(cx);
                cx.spawn(async move |cx| {
                    let result = b
                        .dispatch("favorite.delete", serde_json::json!({"favorite_id": id}))
                        .await;
                    cx.update(|cx| {
                        match result {
                            Ok(_) => {
                                overlay::toast_success(cx, "收藏删除成功");
                                root(cx).update(cx, |app, cx| {
                                    if let Some(v) = app.page_favorites() {
                                        v.update(cx, |view, cx| view.reload(cx));
                                    }
                                });
                            }
                            Err(e) => overlay::toast_error(cx, e),
                        }
})
                })
                .detach();
            }
        });
    }

    fn open_group_form(&self, editing: Option<FavoriteGroup>, cx: &mut Context<Self>) {
        let view = cx.new(|cx| FavoriteGroupForm::new(editing, cx));
        overlay::open_modal(cx, view.into());
    }

    fn open_item_form(&self, item: FavoriteItem, cx: &mut Context<Self>) {
        let groups = self.groups.clone();
        let view = cx.new(|cx| FavoriteItemForm::new(item, groups, cx));
        overlay::open_modal(cx, view.into());
    }
}

impl Focusable for FavoritesView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FavoritesView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);
        let groups = self.groups.clone();
        let loading = self.loading;

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
                    .gap(px(8.))
                    .child(
                        icon_btn("fav-back", "arrow-left", BtnSize::Xs)
                            .when(!app.read(cx).can_go_back(), |b| b.opacity(0.5))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                root(cx).update(cx, |app, cx| app.go_back(cx));
                            }),
                    )
                    .child(
                        icon("star")
                            .size(px(20.))
                            .text_color(theme::warning()),
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
                                    .child(t("favorites.title")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child(t("favorites.description")),
                            ),
                    )
                    .child(
                        btn("fav-new-group", BtnKind::Primary, BtnSize::Sm)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.open_group_form(None, cx)),
                            )
                            .child(icon("plus").size(px(13.)).into_any_element())
                            .child(t("favorites.addGroup")),
                    ),
            )
            // 内容
            .child(
                div().id("views_favorites_rs_2")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .when(loading, |d| d.child(loading_block(t("common.loading"))))
                    .when(!loading && groups.is_empty(), |d| {
                        d.child(empty_block(
                            "star",
                            "暂无收藏分组",
                            "点击右上角创建分组",
                        ))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .pb(px(12.))
                            .children(groups.iter().enumerate().map(|(gix, group)| {
                                self.render_group_card(group, gix, cx)
                            })),
                    ),
            )
    }
}

impl FavoritesView {
    fn render_group_card(
        &mut self,
        group: &FavoriteGroup,
        gix: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let expanded = self.expanded.contains(&group.id);
        let search_input = self.search_input(group.id, cx);
        let filtered = self.filtered_items(group, cx);
        let searching = !search_input.read(cx).is_empty();
        let gid = group.id;
        let g_edit = group.clone();
        let g_del = group.clone();

        card()
            .flex()
            .flex_col()
            // 分组头
            .child(
                div()
                    .id(("fav-group-head", gix))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .h(px(42.))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| this.toggle_group(gid, cx)),
                    )
                    .child(
                        icon(if expanded {
                            "chevron-down"
                        } else {
                            "chevron-right"
                        })
                        .size(px(13.))
                        .text_color(theme::text_secondary()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(group.name.clone()),
                    )
                    .when_some(group.description.clone(), |d, desc| {
                        d.child(
                            div()
                                .max_w(px(150.))
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_size(px(11.))
                                .text_color(theme::base_content_alpha(0.4))
                                .child(desc),
                        )
                    })
                    .child(badge(filtered.len().to_string(), BadgeKind::Ghost))
                    .child(div().flex_1())
                    .child(
                        icon_btn(("fav-group-edit", gix), "pencil", BtnSize::Xs)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.open_group_form(Some(g_edit.clone()), cx)
                                }),
                            ),
                    )
                    .child(
                        div()
                            .id(("fav-group-del", gix))
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
                                cx.listener(move |this, _, _, cx| {
                                    this.delete_group(&g_del, cx)
                                }),
                            )
                            .child(icon("trash").size(px(13.))),
                    ),
            )
            // 组内搜索
            .when(expanded, |d| {
                d.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .border_t_1()
                        .border_color(theme::border_base_200())
                        .child(
                            div()
                                .h(px(26.))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(theme::input_bg())
                                .border_1()
                                .border_color(theme::base_content_alpha(0.15))
                                .text_size(px(11.))
                                .child(
                                    icon("search")
                                        .size(px(11.))
                                        .text_color(theme::text_secondary()),
                                )
                                .child(div().flex_1().child(search_input)),
                        ),
                )
            })
            // 收藏项
            .when(expanded, |d| {
                let mut list = div().flex().flex_col().border_t_1().border_color(theme::border_base_200());
                if filtered.is_empty() {
                    list = list.child(
                        div()
                            .py(px(16.))
                            .flex()
                            .justify_center()
                            .text_size(px(11.))
                            .text_color(theme::text_secondary())
                            .child(if searching {
                                "无匹配的收藏".to_string()
                            } else {
                                "该分组暂无收藏".to_string()
                            }),
                    );
                }
                for (ix, item) in filtered.iter().enumerate() {
                    let cluster = item.cluster_id.clone();
                    let topic = item.topic_name.clone();
                    let item_edit = item.clone();
                    let item_del = item.clone();
                    list = list.child(
                        div()
                            .id(("fav-item", gix * 10000 + ix))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .px(px(12.))
                            .py(px(6.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::table_row_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    if event.click_count == 2 {
                                        this.navigate_to_topic(&cluster, &topic, cx);
                                    }
                                }),
                            )
                            .child(
                                div()
                                    .size(px(20.))
                                    .rounded(px(5.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(theme::badge_primary_bg())
                                    .child(
                                        icon("database")
                                            .size(px(11.))
                                            .text_color(theme::badge_primary_text()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme::text_primary())
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(item.topic_name.clone()),
                            )
                            .child(badge(item.cluster_name.clone(), BadgeKind::Ghost))
                            .when_some(item.description.clone(), |d, desc| {
                                d.child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(theme::base_content_alpha(0.4))
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(desc),
                                )
                            })
                            .child(div().flex_1())
                            .child(
                                icon_btn(("fav-item-edit", gix * 10000 + ix), "pencil", BtnSize::Xs)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _, _, cx| {
                                            this.open_item_form(item_edit.clone(), cx)
                                        }),
                                    ),
                            )
                            .child(
                                div()
                                    .id(("fav-item-del", gix * 10000 + ix))
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
                                        cx.listener(move |this, _, _, cx| {
                                            this.delete_item(&item_del, cx)
                                        }),
                                    )
                                    .child(icon("trash").size(px(13.))),
                            ),
                    );
                }
                d.child(list)
            })
    }
}

// ==================== 分组表单 ====================

pub struct FavoriteGroupForm {
    editing: Option<FavoriteGroup>,
    name: Entity<TextInput>,
    description: Entity<TextInput>,
    sort_order: Entity<TextInput>,
    saving: bool,
    focus_handle: FocusHandle,
}

impl FavoriteGroupForm {
    pub fn new(editing: Option<FavoriteGroup>, cx: &mut Context<Self>) -> Self {
        let name = cx.new(TextInput::new);
        name.update(cx, |i, _| i.set_placeholder("请输入分组名称"));
        let description = cx.new(TextInput::new);
        description.update(cx, |i, _| i.set_placeholder("请输入分组描述（可选）"));
        let sort_order = cx.new(TextInput::new);
        sort_order.update(cx, |i, _| i.set_placeholder("数字越小越靠前"));
        if let Some(g) = &editing {
            name.update(cx, |i, cx| i.set_text(g.name.clone(), cx));
            description.update(cx, |i, cx| {
                i.set_text(g.description.clone().unwrap_or_default(), cx)
            });
            sort_order.update(cx, |i, cx| i.set_text(g.sort_order.to_string(), cx));
        } else {
            sort_order.update(cx, |i, cx| i.set_text("0", cx));
        }
        for input in [&name, &description, &sort_order] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }
        Self {
            editing,
            name,
            description,
            sort_order,
            saving: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).text().trim().to_string();
        if name.is_empty() || self.saving {
            return;
        }
        self.saving = true;
        cx.notify();
        let description = self.description.read(cx).text().trim().to_string();
        let sort_order: i64 = self
            .sort_order
            .read(cx)
            .text()
            .trim()
            .parse()
            .unwrap_or(0);
        let b = backend(cx);
        let editing = self.editing.clone();
        cx.spawn(async move |this, cx| {
            let mut params = serde_json::json!({
                "name": name,
                "sort_order": sort_order,
            });
            if !description.is_empty() {
                params["description"] = serde_json::json!(description);
            }
            let result = if let Some(g) = &editing {
                params["group_id"] = serde_json::json!(g.id);
                b.dispatch("favorite.group.update", params).await
            } else {
                b.dispatch("favorite.group.create", params).await
            };
            this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(
                            cx,
                            if editing.is_some() {
                                "分组更新成功"
                            } else {
                                "分组创建成功"
                            },
                        );
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            if let Some(v) = app.page_favorites() {
                                v.update(cx, |view, cx| view.reload(cx));
                            }
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
}

impl Focusable for FavoriteGroupForm {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FavoriteGroupForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(if self.editing.is_some() {
                                "编辑分组"
                            } else {
                                "创建分组"
                            }),
                    )
                    .child(
                        icon_btn("close-fav-group-form", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(field(
                "分组名称 *",
                input_frame(self.name.clone()).into_any_element(),
            ))
            .child(field(
                "分组描述",
                input_frame(self.description.clone()).into_any_element(),
            ))
            .child(field(
                "排序",
                input_frame(self.sort_order.clone()).into_any_element(),
            ))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        btn("fgf-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("fgf-save", BtnKind::Primary, BtnSize::Sm)
                            .when(self.saving, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(cx)),
                            )
                            .child(if self.saving {
                                spinner(13.)
                            } else {
                                icon("check").size(px(13.)).into_any_element()
                            })
                            .child(t("common.save")),
                    ),
            )
    }
}

// ==================== 收藏项编辑表单 ====================

pub struct FavoriteItemForm {
    item: FavoriteItem,
    group_select: Entity<Select>,
    description: Entity<TextInput>,
    sort_order: Entity<TextInput>,
    saving: bool,
    focus_handle: FocusHandle,
}

impl FavoriteItemForm {
    pub fn new(item: FavoriteItem, groups: Vec<FavoriteGroup>, cx: &mut Context<Self>) -> Self {
        let group_select = cx.new(|cx| {
            Select::new(
                groups
                    .iter()
                    .map(|g| SelectOption {
                        value: g.id.to_string().into(),
                        label: g.name.clone().into(),
                    })
                    .collect(),
                item.group_id.to_string(),
                cx,
            )
        });
        let description = cx.new(TextInput::new);
        description.update(cx, |i, _| i.set_placeholder("请输入描述（可选）"));
        description.update(cx, |i, cx| {
            i.set_text(item.description.clone().unwrap_or_default(), cx)
        });
        let sort_order = cx.new(TextInput::new);
        sort_order.update(cx, |i, cx| i.set_text(item.sort_order.to_string(), cx));
        for input in [&description, &sort_order] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }
        Self {
            item,
            group_select,
            description,
            sort_order,
            saving: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        self.saving = true;
        cx.notify();
        let group_id: i64 = self
            .group_select
            .read(cx)
            .value
            .parse()
            .unwrap_or(self.item.group_id);
        let description = self.description.read(cx).text().trim().to_string();
        let sort_order: i64 = self
            .sort_order
            .read(cx)
            .text()
            .trim()
            .parse()
            .unwrap_or(0);
        let b = backend(cx);
        let id = self.item.id;
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "favorite.update",
                    serde_json::json!({
                        "favorite_id": id,
                        "group_id": group_id,
                        "description": description,
                        "sort_order": sort_order,
                    }),
                )
                .await;
            this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, "收藏更新成功");
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            if let Some(v) = app.page_favorites() {
                                v.update(cx, |view, cx| view.reload(cx));
                            }
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
}

impl Focusable for FavoriteItemForm {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FavoriteItemForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child("编辑收藏"),
                    )
                    .child(
                        icon_btn("close-fav-item-form", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(field(
                "选择分组",
                self.group_select.clone().into_any_element(),
            ))
            .child(field(
                "描述",
                input_frame(self.description.clone()).into_any_element(),
            ))
            .child(field(
                "排序",
                input_frame(self.sort_order.clone()).into_any_element(),
            ))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        btn("fif-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("fif-save", BtnKind::Primary, BtnSize::Sm)
                            .when(self.saving, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit(cx)),
                            )
                            .child(if self.saving {
                                spinner(13.)
                            } else {
                                icon("check").size(px(13.)).into_any_element()
                            })
                            .child(t("common.save")),
                    ),
            )
    }
}
