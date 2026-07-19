//! 收藏页：分组管理、收藏条目管理

use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::i18n::t;
use crate::pages::clusters::notify;
use crate::state::{Backend, TokioRuntime};

#[derive(Clone, Debug)]
struct FavGroup {
    id: i64,
    name: String,
    items: Vec<FavItem>,
}

#[derive(Clone, Debug)]
struct FavItem {
    id: i64,
    cluster_id: String,
    topic_name: String,
    description: Option<String>,
}

/// 创建分组表单
struct GroupForm {
    name: Entity<InputState>,
    description: Entity<InputState>,
}

pub struct FavoritesPage {
    groups: Vec<FavGroup>,
    loading: bool,
    error: Option<String>,
    group_form: Option<GroupForm>,
}

impl FavoritesPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = Self {
            groups: Vec::new(),
            loading: true,
            error: None,
            group_form: None,
        };
        this.reload(cx);
        this
    }

    fn reload(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "favorite.list", json!({})).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(value) => {
                        this.error = None;
                        this.groups = value
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|g| {
                                        Some(FavGroup {
                                            id: g.get("id")?.as_i64()?,
                                            name: g.get("name")?.as_str()?.to_string(),
                                            items: g
                                                .get("items")
                                                .and_then(|v| v.as_array())
                                                .map(|items| {
                                                    items
                                                        .iter()
                                                        .filter_map(|i| {
                                                            Some(FavItem {
                                                                id: i.get("id")?.as_i64()?,
                                                                cluster_id: i
                                                                    .get("cluster_id")?
                                                                    .as_str()?
                                                                    .to_string(),
                                                                topic_name: i
                                                                    .get("topic_name")?
                                                                    .as_str()?
                                                                    .to_string(),
                                                                description: i
                                                                    .get("description")
                                                                    .and_then(|v| v.as_str())
                                                                    .map(|s| s.to_string()),
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
                    }
                    Err(e) => this.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 打开创建分组对话框
    fn open_group_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "favorites.groupNamePlaceholder"))
        });
        let desc_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "favorites.groupDescPlaceholder"))
        });

        self.group_form = Some(GroupForm {
            name: name_state.clone(),
            description: desc_state.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "favorites.createGroup");
        let name_label = t(cx, "favorites.groupName");
        let desc_label = t(cx, "favorites.groupDescription");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title(title.clone())
                .w(px(480.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(field_row(&name_label, Input::new(&name_state).into_any_element()))
                        .child(field_row(&desc_label, Input::new(&desc_state).into_any_element())),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, window, cx| {
                    entity.update(cx, |this, cx| this.submit_group_form(window, cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn submit_group_form(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.group_form.take() else { return };
        let name = form.name.read(cx).value().to_string();
        let description = form.description.read(cx).value().to_string();
        if name.trim().is_empty() {
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "favorite.group.create",
                json!({ "name": name.trim(), "description": description.trim() }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "favorites.groupCreated")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    /// 删除分组（确认）
    fn confirm_delete_group(&mut self, group: FavGroup, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let title = t(cx, "favorites.confirmDeleteGroup");
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let group = group.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(group.name.clone())
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.delete_group(group.id, cx));
                    true
                })
        });
    }

    fn delete_group(&mut self, id: i64, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result =
                crate::service::call(&rt, state, "favorite.group.delete", json!({ "id": id })).await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "common.success")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    /// 删除收藏条目
    fn delete_item(&mut self, id: i64, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result =
                crate::service::call(&rt, state, "favorite.delete", json!({ "id": id })).await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "favorites.removed")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }
}

/// 表单字段行：标签 + 控件
fn field_row(label: &str, control: AnyElement) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_sm().child(label.to_string()))
        .child(control)
}

impl Render for FavoritesPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let header = h_flex()
            .items_center()
            .justify_between()
            .mb_4()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().font_semibold().child(t(cx, "favorites.title")))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "favorites.description")),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("refresh")
                            .outline()
                            .label(t(cx, "common.refresh"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.loading = true;
                                cx.notify();
                                this.reload(cx);
                            })),
                    )
                    .child(
                        Button::new("create-group")
                            .primary()
                            .label(t(cx, "favorites.createGroup"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_group_form(window, cx);
                            })),
                    ),
            );

        let content: AnyElement = if self.loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if let Some(err) = &self.error {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.danger)
                .child(err.clone())
                .into_any_element()
        } else if self.groups.is_empty() {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(theme.muted_foreground)
                .child(t(cx, "favorites.noGroups"))
                .child(div().text_sm().child(t(cx, "favorites.createGroupHint")))
                .into_any_element()
        } else {
            let entity = cx.entity();
            let sections: Vec<AnyElement> = self
                .groups
                .iter()
                .map(|group| {
                    let rows: Vec<AnyElement> = group
                        .items
                        .iter()
                        .map(|item| {
                            let entity = entity.clone();
                            let item_id = item.id;
                            h_flex()
                                .items_center()
                                .justify_between()
                                .px_3()
                                .py_2()
                                .border_b_1()
                                .border_color(theme.border)
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_center()
                                                .child(Icon::new(IconName::Star).text_color(theme.warning))
                                                .child(div().font_semibold().child(item.topic_name.clone())),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(format!(
                                                    "{} {}",
                                                    item.cluster_id,
                                                    item.description
                                                        .as_deref()
                                                        .map(|d| format!("— {}", d))
                                                        .unwrap_or_default()
                                                )),
                                        ),
                                )
                                .child(
                                    Button::new(("del-fav", item_id as usize))
                                        .ghost()
                                        .icon(IconName::Delete)
                                        .on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.delete_item(item_id, cx)
                                            });
                                        }),
                                )
                                .into_any_element()
                        })
                        .collect();

                    let entity = entity.clone();
                    let group_for_delete = group.clone();
                    v_flex()
                        .mb_4()
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .mb_2()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(div().font_semibold().child(group.name.clone()))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(format!("({})", group.items.len())),
                                        ),
                                )
                                .child(
                                    Button::new(("del-group", group.id as usize))
                                        .ghost()
                                        .icon(IconName::Delete)
                                        .on_click(move |_, window, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.confirm_delete_group(
                                                    group_for_delete.clone(),
                                                    window,
                                                    cx,
                                                )
                                            });
                                        }),
                                ),
                        )
                        .child(
                            v_flex()
                                .border_1()
                                .border_color(theme.border)
                                .rounded_md()
                                .children(rows),
                        )
                        .into_any_element()
                })
                .collect();

            v_flex().children(sections).into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
