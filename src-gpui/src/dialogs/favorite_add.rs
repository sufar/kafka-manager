//! 添加收藏：分组选择弹窗（对齐 FavoriteButton.vue 的 select-group modal）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    Window,
};

use crate::app::{backend, root, AppEvent};
use crate::i18n::t;
use crate::icons::icon;
use crate::models::FavoriteGroup;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::favorite_button::{FavoriteButton, FavoriteChanged};
use crate::widgets::text_input::{TextInput, TextInputEvent};

pub struct FavoriteAddDialog {
    cluster_id: String,
    topic_name: String,
    source: Entity<FavoriteButton>,
    groups: Vec<FavoriteGroup>,
    loading: bool,
    selected_group_id: Option<i64>,
    show_create_group: bool,
    new_group_name: Entity<TextInput>,
    new_group_desc: Entity<TextInput>,
    remark: Entity<TextInput>,
    creating_group: bool,
    saving: bool,
    focus_handle: FocusHandle,
}

impl FavoriteAddDialog {
    pub fn new(
        cluster_id: String,
        topic_name: String,
        source: Entity<FavoriteButton>,
        cx: &mut Context<Self>,
    ) -> Self {
        let new_group_name = cx.new(TextInput::new);
        new_group_name.update(cx, |i, _| {
            i.set_placeholder(t("favorites.groupNamePlaceholder"))
        });
        let new_group_desc = cx.new(TextInput::new);
        new_group_desc.update(cx, |i, _| {
            i.set_placeholder(t("favorites.groupDescPlaceholder"))
        });
        let remark = cx.new(TextInput::new);
        remark.update(cx, |i, _| i.set_placeholder(t("favorites.remarkPlaceholder")));

        for input in [&new_group_name, &new_group_desc, &remark] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }
        cx.subscribe(&new_group_name, |this, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EnterPressed) {
                this.submit_create_group(cx);
            }
        })
        .detach();

        let this = Self {
            cluster_id,
            topic_name,
            source,
            groups: vec![],
            loading: true,
            selected_group_id: None,
            show_create_group: false,
            new_group_name,
            new_group_desc,
            remark,
            creating_group: false,
            saving: false,
            focus_handle: cx.focus_handle(),
        };
        this.load_groups(cx);
        this
    }

    fn load_groups(&self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b.dispatch("favorite.group.list", serde_json::json!({})).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(v) = result {
                    this.groups = v
                        .get("groups")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    Some(FavoriteGroup {
                                        id: item.get("id")?.as_i64()?,
                                        name: item.get("name")?.as_str()?.to_string(),
                                        description: item
                                            .get("description")
                                            .and_then(|x| x.as_str())
                                            .map(|s| s.to_string()),
                                        sort_order: item
                                            .get("sort_order")
                                            .and_then(|x| x.as_i64())
                                            .unwrap_or(0),
                                        created_at: None,
                                        items: vec![],
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    this.selected_group_id = this.groups.first().map(|g| g.id);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn submit_create_group(&mut self, cx: &mut Context<Self>) {
        let name = self.new_group_name.read(cx).text().trim().to_string();
        if name.is_empty() || self.creating_group {
            return;
        }
        self.creating_group = true;
        cx.notify();
        let desc = self.new_group_desc.read(cx).text().trim().to_string();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "favorite.group.create",
                    serde_json::json!({"name": name, "description": desc, "sort_order": 0}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.creating_group = false;
                match result {
                    Ok(v) => {
                        let id = v.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
                        this.groups.push(FavoriteGroup {
                            id,
                            name: v
                                .get("name")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            description: None,
                            sort_order: 0,
                            created_at: None,
                            items: vec![],
                        });
                        this.selected_group_id = Some(id);
                        this.show_create_group = false;
                        this.new_group_name.update(cx, |i, cx| i.reset(cx));
                        this.new_group_desc.update(cx, |i, cx| i.reset(cx));
                        overlay::toast_success(cx, t("favorites.groupCreated"));
                    }
                    Err(e) => overlay::toast_error(cx, e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn confirm_add(&mut self, cx: &mut Context<Self>) {
        let Some(gid) = self.selected_group_id else {
            return;
        };
        if self.saving {
            return;
        }
        self.saving = true;
        cx.notify();
        let remark = self.remark.read(cx).text().trim().to_string();
        let (cid, tn) = (self.cluster_id.clone(), self.topic_name.clone());
        let b = backend(cx);
        let source = self.source.clone();
        cx.spawn(async move |this, cx| {
            let mut params = serde_json::json!({
                "group_id": gid,
                "cluster_id": cid,
                "topic_name": tn,
            });
            if !remark.is_empty() {
                params["description"] = serde_json::json!(remark);
            }
            let result = b.dispatch("favorite.create", params).await;
            this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, t("favorites.added"));
                        overlay::close_modal(cx);
                        source.update(cx, |btn, cx| {
                            btn.set_favorite(true, cx);
                            cx.emit(FavoriteChanged(true));
                        });
                        root(cx).update(cx, |app, cx| {
                            app.publish(AppEvent::FavoritesChanged, cx)
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

impl Focusable for FavoriteAddDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FavoriteAddDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap(px(10.));

        if self.loading {
            body = body.child(div().h(px(120.)).child(loading_block(t("common.loading"))));
        } else if self.groups.is_empty() && !self.show_create_group {
            body = body
                .child(
                    div()
                        .py(px(16.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            div().id("dialogs_favorite_add_rs_1")
                                .text_size(px(12.))
                                .text_color(theme::text_secondary())
                                .child(t("favorites.noGroups")),
                        )
                        .child(
                            btn("fav-create-group-empty", BtnKind::Primary, BtnSize::Sm)
                                .child(t("favorites.createGroup"))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.show_create_group = true;
                                        cx.notify();
                                    }),
                                ),
                        ),
                );
        } else {
            // 分组列表
            let mut list = div().id("auto_dialogs_favorite_add_rs_101").flex().flex_col().gap(px(4.)).max_h(px(220.)).overflow_y_scroll();
            // 新增分组虚线按钮
            if !self.show_create_group {
                list = list.child(
                    div()
                        .id("fav-new-group")
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(4.))
                        .h(px(34.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme::base_content_alpha(0.2))
                        .cursor_pointer()
                        .text_color(theme::text_secondary())
                        .hover(|s| s.bg(theme::btn_ghost_hover()))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.show_create_group = true;
                                cx.notify();
                            }),
                        )
                        .child(icon("plus").size(px(13.)))
                        .child(
                            div().text_size(px(12.)).child(t("favorites.createGroup")),
                        ),
                );
            }
            for (ix, g) in self.groups.iter().enumerate() {
                let selected = self.selected_group_id == Some(g.id);
                let gid = g.id;
                list = list.child(
                    div()
                        .id(("fav-group", ix))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(10.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .cursor_pointer()
                        .when(selected, |d| {
                            d.bg(theme::badge_primary_bg())
                                .border_1()
                                .border_color(theme::primary_alpha(0.4))
                        })
                        .when(!selected, |d| {
                            d.border_1().border_color(theme::base_content_alpha(0.1))
                        })
                        .hover(|s| s.bg(theme::context_menu_item_hover()))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.selected_group_id = Some(gid);
                                cx.notify();
                            }),
                        )
                        .child(
                            div()
                                .size(px(14.))
                                .rounded(px(7.))
                                .border_1()
                                .border_color(if selected {
                                    theme::primary()
                                } else {
                                    theme::base_content_alpha(0.3)
                                })
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(selected, |d| {
                                    d.child(
                                        div().size(px(7.)).rounded(px(4.)).bg(theme::primary()),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme::text_primary())
                                        .child(g.name.clone()),
                                )
                                .when_some(g.description.clone(), |d, desc| {
                                    d.child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(theme::text_secondary())
                                            .child(desc),
                                    )
                                }),
                        ),
                );
            }
            body = body.child(list);

            // 备注
            body = body.child(field(
                t("favorites.remark"),
                input_frame(self.remark.clone()).into_any_element(),
            ));
        }

        // 创建分组表单
        if self.show_create_group {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .p(px(8.))
                    .rounded(px(8.))
                    .bg(theme::base_content_alpha(0.05))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme::text_primary())
                            .child(t("favorites.createGroup")),
                    )
                    .child(input_frame(self.new_group_name.clone()))
                    .child(input_frame(self.new_group_desc.clone()))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(6.))
                            .child(
                                btn("fav-cancel-group", BtnKind::Ghost, BtnSize::Sm)
                                    .child(t("common.cancel"))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.show_create_group = false;
                                            cx.notify();
                                        }),
                                    ),
                            )
                            .child(
                                btn("fav-save-group", BtnKind::Primary, BtnSize::Sm)
                                    .when(self.creating_group, |b| b.opacity(0.6))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.submit_create_group(cx)
                                        }),
                                    )
                                    .child(if self.creating_group {
                                        spinner(12.)
                                    } else {
                                        icon("check").size(px(12.)).into_any_element()
                                    })
                                    .child(t("common.save")),
                            ),
                    ),
            );
        }

        div()
            .w(px(380.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            // 头部
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(36.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme::badge_primary_bg())
                            .child(
                                icon("star")
                                    .size(px(18.))
                                    .text_color(theme::badge_primary_text()),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("favorites.selectGroup")),
                    )
                    .child(
                        icon_btn("close-fav-add", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(body)
            // 底部
            .when(!self.groups.is_empty() || self.show_create_group, |d| {
                d.child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            btn("fav-add-cancel", BtnKind::Ghost, BtnSize::Sm)
                                .child(t("common.cancel"))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    overlay::close_modal(cx);
                                }),
                        )
                        .child(
                            btn("fav-add-confirm", BtnKind::Primary, BtnSize::Sm)
                                .when(
                                    self.selected_group_id.is_none() || self.saving,
                                    |b| b.opacity(0.5),
                                )
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| this.confirm_add(cx)),
                                )
                                .child(if self.saving {
                                    spinner(13.)
                                } else {
                                    icon("star-solid").size(px(13.)).into_any_element()
                                })
                                .child(t("common.confirm")),
                        ),
                )
            })
    }
}
