//! 收藏分组选择弹窗（对齐 Vue FavoriteButton 的分组选择交互）
//!
//! 点击未收藏的星标时弹出：选择分组（可内联新建分组）+ 备注，确认后创建收藏。

use std::rc::Rc;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::h_flex;
use gpui_component::input::{Input, InputState};
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::radio::Radio;
use gpui_component::v_flex;
use gpui_component::{ActiveTheme, Disableable, IconName, Sizable, WindowExt};

use crate::i18n::t;
use crate::service;
use crate::state::{Backend, TokioRuntime};

struct GroupRow {
    id: i64,
    name: String,
    item_count: i64,
}

pub struct FavoriteDialog {
    cluster: String,
    topic: String,
    groups: Vec<GroupRow>,
    loading: bool,
    selected: Option<i64>,
    /// 内联新建分组模式
    creating_group: bool,
    new_group_input: Entity<InputState>,
    note_input: Entity<InputState>,
    submitting: bool,
    on_done: Option<Rc<dyn Fn(&mut App)>>,
}

impl FavoriteDialog {
    /// 打开收藏分组选择弹窗。`on_done` 在收藏成功后调用（调用方刷新收藏状态）。
    pub fn open(
        window: &mut Window,
        cx: &mut App,
        cluster: String,
        topic: String,
        on_done: impl Fn(&mut App) + 'static,
    ) {
        let new_group_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "favorites.groupNamePlaceholder"))
        });
        let note_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "favorites.remarkPlaceholder"))
        });

        let entity = cx.new(|_| FavoriteDialog {
            cluster,
            topic,
            groups: Vec::new(),
            loading: true,
            selected: None,
            creating_group: false,
            new_group_input,
            note_input,
            submitting: false,
            on_done: Some(Rc::new(on_done)),
        });

        // 加载分组列表
        let rt = TokioRuntime::handle(cx);
        if let Some(state) = Backend::state(cx) {
            let entity = entity.clone();
            cx.spawn(async move |cx| {
                let result = service::call(&rt, state, "favorite.group.list", serde_json::json!({})).await;
                entity
                    .update(cx, |this, cx| {
                        if let Ok(val) = result {
                            let groups = val
                                .as_array()
                                .cloned()
                                .unwrap_or_default()
                                .into_iter()
                                .filter_map(|g| {
                                    Some(GroupRow {
                                        id: g.get("id")?.as_i64()?,
                                        name: g.get("name")?.as_str()?.to_string(),
                                        item_count: g
                                            .get("item_count")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(0),
                                    })
                                })
                                .collect::<Vec<_>>();
                            this.selected = groups.first().map(|g| g.id);
                            this.groups = groups;
                        }
                        this.loading = false;
                        cx.notify();
                    })
                    .ok();
            })
            .detach();
        } else {
            entity.update(cx, |this, _| this.loading = false);
        }

        let entity_c = entity.clone();
        let title = t(cx, "favorites.selectGroup");
        let new_group_label = t(cx, "favorites.addGroup");
        let remark_label = t(cx, "favorites.remark");
        let ok_label = t(cx, "common.confirm");
        let cancel_label = t(cx, "common.cancel");
        let empty_label = t(cx, "favorites.noGroups");
        let loading_label = t(cx, "common.loading");

        window.open_dialog(cx, move |dialog, _window, cx| {
            let entity = entity_c.clone();
            let (groups, loading, selected, creating_group, submitting) = {
                let s = entity.read(cx);
                let groups: Vec<GroupRowLite> = s
                    .groups
                    .iter()
                    .map(|g| GroupRowLite {
                        id: g.id,
                        name: g.name.clone(),
                        item_count: g.item_count,
                    })
                    .collect();
                (groups, s.loading, s.selected, s.creating_group, s.submitting)
            };

            let border = cx.theme().border;
            let muted = cx.theme().muted_foreground;

            // 分组行
            let rows: Vec<AnyElement> = groups
                .iter()
                .map(|g| {
                    let entity = entity.clone();
                    let id = g.id;
                    let checked = selected == Some(id);
                    h_flex()
                        .items_center()
                        .gap_2()
                        .p_2()
                        .border_b_1()
                        .border_color(border)
                        .cursor_pointer()
                        .child(
                            Radio::new(SharedString::from(format!("fav-group-{}", id)))
                                .checked(checked),
                        )
                        .child(div().flex_1().text_sm().child(g.name.clone()))
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child(format!("{}", g.item_count)),
                        )
                        .id(SharedString::from(format!("fav-group-row-{}", id)))
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.selected = Some(id);
                                cx.notify();
                            });
                        })
                        .into_any_element()
                })
                .collect();

            let body = if loading {
                div()
                    .p_4()
                    .text_sm()
                    .text_color(muted)
                    .child(loading_label.clone())
                    .into_any_element()
            } else {
                v_flex()
                    .child(
                        div()
                            .id("fav-group-scroll")
                            .max_h(px(200.0))
                            .overflow_y_scroll()
                            .child(v_flex().children(rows)),
                    )
                    .when(groups.is_empty(), |d| {
                        d.child(
                            div()
                                .p_2()
                                .text_sm()
                                .text_color(muted)
                                .child(empty_label.clone()),
                        )
                    })
                    .into_any_element()
            };

            // 内联新建分组 / 备注输入
            let entity_toggle = entity.clone();
            let new_group_row = h_flex()
                .items_center()
                .gap_2()
                .child(
                    Button::new("fav-toggle-new-group")
                        .ghost()
                        .xsmall()
                        .icon(if creating_group {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .label(new_group_label.clone())
                        .on_click(move |_, _, cx| {
                            entity_toggle.update(cx, |this, cx| {
                                this.creating_group = !this.creating_group;
                                cx.notify();
                            });
                        }),
                );
            let new_group_input_view = entity.read(cx).new_group_input.clone();
            let note_input_view = entity.read(cx).note_input.clone();

            let entity_ok = entity.clone();
            let ok_label = ok_label.clone();
            let cancel_label = cancel_label.clone();

            dialog
                .title(title.clone())
                .w(px(420.0))
                .overlay_closable(false)
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(ok_label.clone())
                        .cancel_text(cancel_label.clone()),
                )
                .child(
                    v_flex()
                        .gap_3()
                        .child(body)
                        .child(new_group_row)
                        .when(creating_group, |d| {
                            d.child(Input::new(&new_group_input_view).into_any_element())
                        })
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div().text_sm().text_color(muted).child(remark_label.clone()),
                                )
                                .child(Input::new(&note_input_view).into_any_element()),
                        ),
                )
                .footer(move |_ok, _cancel, _window, _cx| {
                    let entity_ok = entity_ok.clone();
                    vec![
                        Button::new("fav-cancel")
                            .label(cancel_label.clone())
                            .on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                            })
                            .into_any_element(),
                        Button::new("fav-ok")
                            .primary()
                            .label(ok_label.clone())
                            .loading(submitting)
                            .disabled(!creating_group && selected.is_none())
                            .on_click(move |_, window, cx| {
                                entity_ok.update(cx, |this, cx| {
                                    this.submit(window, cx);
                                });
                            })
                            .into_any_element(),
                    ]
                })
        });
    }

    /// 提交：必要时先建分组，再创建收藏
    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.submitting {
            return;
        }
        let Some(state) = Backend::state(cx) else {
            return;
        };
        let rt = TokioRuntime::handle(cx);

        self.submitting = true;
        cx.notify();

        let cluster = self.cluster.clone();
        let topic = self.topic.clone();
        let selected = self.selected;
        let creating_group = self.creating_group;
        let new_group_name = self.new_group_input.read(cx).value().trim().to_string();
        let note = self.note_input.read(cx).value().trim().to_string();
        let note = if note.is_empty() { None } else { Some(note) };
        let on_done = self.on_done.clone();
        let added_msg = t(cx, "favorites.added");
        let fail_msg = t(cx, "toast.operationFailed");

        cx.spawn_in(window, async move |this, cx| {
            // 1) 需要时先创建分组
            let group_id = if creating_group && !new_group_name.is_empty() {
                let created = service::call(
                    &rt,
                    state.clone(),
                    "favorite.group.create",
                    serde_json::json!({ "name": new_group_name }),
                )
                .await;
                match created {
                    Ok(val) => val.get("id").and_then(|v| v.as_i64()),
                    Err(e) => {
                        this.update_in(cx, |this, window, cx| {
                            this.submitting = false;
                            cx.notify();
                            window.push_notification(
                                Notification::new()
                                    .message(format!("{}: {}", fail_msg, e))
                                    .with_type(NotificationType::Error),
                                cx,
                            );
                        })
                        .ok();
                        return;
                    }
                }
            } else {
                selected
            };

            let Some(group_id) = group_id else {
                this.update_in(cx, |this, _window, cx| {
                    this.submitting = false;
                    cx.notify();
                })
                .ok();
                return;
            };

            // 2) 创建收藏
            let result = service::call(
                &rt,
                state,
                "favorite.create",
                serde_json::json!({
                    "group_id": group_id,
                    "cluster_id": cluster,
                    "topic_name": topic,
                    "description": note,
                }),
            )
            .await;

            this.update_in(cx, |this, window, cx| {
                this.submitting = false;
                match result {
                    Ok(_) => {
                        window.push_notification(
                            Notification::new()
                                .message(added_msg)
                                .with_type(NotificationType::Success),
                            cx,
                        );
                        if let Some(cb) = &on_done {
                            cb(cx);
                        }
                        window.close_dialog(cx);
                    }
                    Err(e) => {
                        cx.notify();
                        window.push_notification(
                            Notification::new()
                                .message(format!("{}: {}", fail_msg, e))
                                .with_type(NotificationType::Error),
                            cx,
                        );
                    }
                }
            })
            .ok();
        })
        .detach();
    }
}

/// 渲染期快照（避免在对话框内容闭包中重复借用）
struct GroupRowLite {
    id: i64,
    name: String,
    item_count: i64,
}
