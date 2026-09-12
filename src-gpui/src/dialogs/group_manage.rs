//! 分组管理对话框（分组列表 + 新增/编辑分组表单，对齐 ClustersView manage groups modal）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    Window,
};

use crate::app::{backend, root, AppEvent};
use crate::i18n::t;
use crate::icons::icon;
use crate::models::ClusterGroup;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::text_area::{TextArea, TextAreaEvent};
use crate::widgets::text_input::{TextInput, TextInputEvent};

pub struct GroupManageDialog {
    editing: Option<ClusterGroup>,
    show_form: bool,
    name: Entity<TextInput>,
    description: Entity<TextArea>,
    submitting: bool,
    focus_handle: FocusHandle,
}

impl GroupManageDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let name = cx.new(TextInput::new);
        name.update(cx, |i, _| i.set_placeholder(t("clusters.groupNamePlaceholder")));
        let description = cx.new(TextArea::new);
        description.update(cx, |a, _| a.set_placeholder(t("clusters.groupDescPlaceholder")));

        cx.subscribe(&name, |_, _, event: &TextInputEvent, cx| {
            if matches!(event, TextInputEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();
        cx.subscribe(&description, |_, _, event: &TextAreaEvent, cx| {
            if matches!(event, TextAreaEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();

        Self {
            editing: None,
            show_form: false,
            name,
            description,
            submitting: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn open_add(&mut self, cx: &mut Context<Self>) {
        self.editing = None;
        self.show_form = true;
        self.name.update(cx, |i, cx| i.reset(cx));
        self.description.update(cx, |a, cx| a.set_text("", cx));
        cx.notify();
    }

    fn open_edit(&mut self, group: ClusterGroup, cx: &mut Context<Self>) {
        self.name.update(cx, |i, cx| i.set_text(group.name.clone(), cx));
        self.description.update(cx, |a, cx| {
            a.set_text(group.description.clone().unwrap_or_default(), cx)
        });
        self.editing = Some(group);
        self.show_form = true;
        cx.notify();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.submitting {
            return;
        }
        let name = self.name.read(cx).text().trim().to_string();
        if name.is_empty() || name.len() > 15 {
            overlay::toast_error(cx, t("clusters.groupNameRequired"));
            return;
        }
        let description = self.description.read(cx).text().trim().to_string();
        self.submitting = true;
        cx.notify();
        let b = backend(cx);
        let editing = self.editing.clone();
        cx.spawn(async move |this, cx| {
            let result = if let Some(g) = &editing {
                b.dispatch(
                    "cluster_group.update",
                    serde_json::json!({"group_id": g.id, "name": name, "description": description}),
                )
                .await
            } else {
                b.dispatch(
                    "cluster_group.create",
                    serde_json::json!({"name": name, "description": description}),
                )
                .await
            };
            this.update(cx, |this, cx| {
                this.submitting = false;
                match result {
                    Ok(_) => {
                        this.show_form = false;
                        this.editing = None;
                        root(cx).update(cx, |app, cx| {
                            app.reload_clusters(cx);
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

    fn delete_group(&mut self, group: ClusterGroup, cx: &mut Context<Self>) {
        let title = t("clusters.deleteGroupTitle");
        let message = t("clusters.confirmDeleteGroup").replace("{name}", &group.name);
        overlay::confirm(cx, title, message, true, move |cx| {
            let b = backend(cx);
            let gid = group.id;
            cx.spawn(async move |cx| {
                let result = b
                    .dispatch(
                        "cluster_group.delete",
                        serde_json::json!({"group_id": gid}),
                    )
                    .await;
                cx.update(|cx| {
                    match result {
                        Ok(_) => {
                            root(cx).update(cx, |app, cx| {
                                app.reload_clusters(cx);
                                app.publish(AppEvent::ClustersChanged, cx);
                            });
                        }
                        Err(e) => overlay::toast_error(cx, e),
                    }
})
            })
            .detach();
        });
    }
}

impl Focusable for GroupManageDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GroupManageDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let groups = root(cx).read(cx).groups.clone();
        let clusters = root(cx).read(cx).clusters.clone();

        let mut body = div().flex().flex_col().gap(px(10.));

        if !self.show_form {
            // 分组列表
            let mut list = div().id("auto_dialogs_group_manage_rs_101").flex().flex_col().gap(px(6.)).max_h(px(300.)).overflow_y_scroll();
            for (ix, g) in groups.iter().enumerate() {
                let count = clusters.iter().filter(|c| c.group_id == Some(g.id)).count();
                let g_edit = g.clone();
                let g_del = g.clone();
                list = list.child(
                    div()
                        .id(("group-row", ix))
                        .flex()
                        .items_center()
                        .justify_between()
                        .p(px(10.))
                        .rounded(px(8.))
                        .bg(theme::base_content_alpha(0.05))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme::text_primary())
                                        .child(format!(
                                            "{}  ({} {})",
                                            g.name,
                                            count,
                                            t("clusters.clusters")
                                        )),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme::text_secondary())
                                        .child(
                                            g.description
                                                .clone()
                                                .unwrap_or_else(|| t("clusters.noDescription")),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(2.))
                                .child(
                                    icon_btn(("group-edit", ix), "pencil", BtnSize::Xs)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.open_edit(g_edit.clone(), cx)
                                            }),
                                        ),
                                )
                                .child(
                                    icon_btn(("group-del", ix), "trash", BtnSize::Xs)
                                        .text_color(theme::error())
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.delete_group(g_del.clone(), cx)
                                            }),
                                        ),
                                ),
                        ),
                );
            }
            if groups.is_empty() {
                list = list.child(
                    div()
                        .p(px(20.))
                        .flex()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(theme::text_secondary())
                        .child(t("clusters.noGroup")),
                );
            }
            body = body.child(list).child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        btn("gm-close", BtnKind::Outline, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("gm-add", BtnKind::Primary, BtnSize::Sm)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.open_add(cx)),
                            )
                            .child(icon("plus").size(px(13.)).into_any_element())
                            .child(t("clusters.addGroup")),
                    ),
            );
        } else {
            // 表单
            body = body
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .child(if self.editing.is_some() {
                            t("clusters.editGroup")
                        } else {
                            t("clusters.addGroup")
                        }),
                )
                .child(field(
                    t("clusters.groupName"),
                    input_frame(self.name.clone()).into_any_element(),
                ))
                .child(field(
                    t("clusters.groupDescription"),
                    div()
                        .h(px(70.))
                        .rounded(px(6.))
                        .bg(theme::input_bg())
                        .border_1()
                        .border_color(theme::base_content_alpha(0.15))
                        .text_size(px(12.))
                        .p(px(4.))
                        .child(self.description.clone())
                        .into_any_element(),
                ))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            btn("gm-form-cancel", BtnKind::Ghost, BtnSize::Sm)
                                .child(t("common.cancel"))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.show_form = false;
                                        this.editing = None;
                                        cx.notify();
                                    }),
                                ),
                        )
                        .child(
                            btn("gm-form-submit", BtnKind::Primary, BtnSize::Sm)
                                .when(self.submitting, |b| b.opacity(0.6))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| this.submit(cx)),
                                )
                                .child(if self.submitting {
                                    spinner(13.)
                                } else {
                                    icon("check").size(px(13.)).into_any_element()
                                })
                                .child(if self.editing.is_some() {
                                    t("common.save")
                                } else {
                                    t("common.create")
                                }),
                        ),
                );
        }

        div()
            .w(px(480.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(16.))
            .gap(px(12.))
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
                                icon("folder")
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
                            .child(t("clusters.manageGroups")),
                    )
                    .child(
                        icon_btn("close-group-manage", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(body)
    }
}
