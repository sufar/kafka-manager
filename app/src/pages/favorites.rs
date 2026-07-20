//! 收藏页：与旧版 TopicFavorites 一致（可折叠分组卡片）
//!
//! 头部：标题 + 描述；右上 [Add Group]
//! 分组卡片：chevron 折叠 + 名称 + 描述 + 计数徽标 + 编辑/删除组
//!   展开时：组内搜索框 + 条目行（图标 + 名称 + 集群徽标 + 描述 + 编辑/删除）
//! 对话框：创建/编辑组（名称/描述/排序）、编辑收藏（换组/描述/排序）、删除确认

use std::collections::HashMap;

use gpui::*;
use gpui_component::button::{Button, ButtonVariant, ButtonVariants};
use gpui_component::dialog::DialogButtonProps;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectState};
use gpui_component::spinner::Spinner;
use gpui_component::*;
use serde_json::json;

use crate::components::notify;
use crate::components::option_select::StringOption;
use crate::i18n::t;
use crate::state::{Backend, TokioRuntime};

#[derive(Clone, Debug)]
struct FavGroup {
    id: i64,
    name: String,
    description: Option<String>,
    items: Vec<FavItem>,
}

#[derive(Clone, Debug)]
struct FavItem {
    id: i64,
    cluster_id: String,
    topic_name: String,
    description: Option<String>,
    sort_order: i32,
}

/// 创建/编辑分组表单
struct GroupForm {
    id: Option<i64>,
    name: Entity<InputState>,
    description: Entity<InputState>,
    sort_order: Entity<InputState>,
}

/// 编辑收藏表单
struct ItemForm {
    id: i64,
    group: Entity<SelectState<SearchableVec<StringOption>>>,
    description: Entity<InputState>,
    sort_order: Entity<InputState>,
}

pub struct FavoritesPage {
    groups: Vec<FavGroup>,
    expanded: std::collections::HashSet<i64>,
    group_search: HashMap<i64, Entity<InputState>>,
    loading: bool,
    error: Option<String>,
    group_form: Option<GroupForm>,
    item_form: Option<ItemForm>,
    _subscriptions: Vec<Subscription>,
}

impl FavoritesPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = Self {
            groups: Vec::new(),
            expanded: std::collections::HashSet::new(),
            group_search: HashMap::new(),
            loading: true,
            error: None,
            group_form: None,
            item_form: None,
            _subscriptions: Vec::new(),
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
                        let groups: Vec<FavGroup> = value
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|g| {
                                        Some(FavGroup {
                                            id: g.get("id")?.as_i64()?,
                                            name: g.get("name")?.as_str()?.to_string(),
                                            description: g
                                                .get("description")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
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
                                                                sort_order: i
                                                                    .get("sort_order")
                                                                    .and_then(|v| v.as_i64())
                                                                    .unwrap_or(0)
                                                                    as i32,
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
                        // 第一组默认展开（与旧版一致）
                        if this.expanded.is_empty() {
                            if let Some(first) = groups.first() {
                                this.expanded.insert(first.id);
                            }
                        }
                        this.groups = groups;
                    }
                    Err(e) => this.error = Some(e),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_group(&mut self, id: i64, cx: &mut Context<Self>) {
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
        cx.notify();
    }

    /// 组内搜索框（惰性创建）
    fn search_for(&mut self, group_id: i64, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        if let Some(state) = self.group_search.get(&group_id) {
            return state.clone();
        }
        let state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t(cx, "favorites.searchPlaceholder"))
        });
        let sub = cx.subscribe(&state, |_this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        self._subscriptions.push(sub);
        self.group_search.insert(group_id, state.clone());
        state
    }

    /// 打开创建/编辑分组对话框
    fn open_group_form(&mut self, edit: Option<FavGroup>, window: &mut Window, cx: &mut Context<Self>) {
        let edit_name = edit.as_ref().map(|g| g.name.clone());
        let edit_desc = edit.as_ref().and_then(|g| g.description.clone());
        let edit_sort = edit.as_ref().map(|g| g.sort_order()).unwrap_or(0);
        let edit_id = edit.as_ref().map(|g| g.id);

        let name_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "favorites.groupNamePlaceholder"));
            if let Some(n) = edit_name {
                state.set_value(n, window, cx);
            }
            state
        });
        let desc_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "favorites.groupDescPlaceholder"));
            if let Some(d) = edit_desc {
                state.set_value(d, window, cx);
            }
            state
        });
        let sort_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "favorites.sortOrderPlaceholder"));
            state.set_value(edit_sort.to_string(), window, cx);
            state
        });

        self.group_form = Some(GroupForm {
            id: edit_id,
            name: name_state.clone(),
            description: desc_state.clone(),
            sort_order: sort_state.clone(),
        });

        let entity = cx.entity();
        let title = if edit_id.is_some() {
            t(cx, "favorites.editGroup")
        } else {
            t(cx, "favorites.createGroup")
        };
        let name_label = t(cx, "favorites.groupName");
        let desc_label = t(cx, "favorites.groupDescription");
        let sort_label = t(cx, "favorites.sortOrder");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title(title.clone())
                .w(px(480.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(field_row(&name_label, Input::new(&name_state).into_any_element()))
                        .child(field_row(&desc_label, Input::new(&desc_state).into_any_element()))
                        .child(field_row(&sort_label, Input::new(&sort_state).into_any_element())),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.submit_group_form(cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn submit_group_form(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.group_form.take() else { return };
        let name = form.name.read(cx).value().to_string();
        let description = form.description.read(cx).value().to_string();
        let sort_order: i64 = form.sort_order.read(cx).value().trim().parse().unwrap_or(0);
        if name.trim().is_empty() {
            return;
        }

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        let is_create = form.id.is_none();
        let method = if is_create { "favorite.group.create" } else { "favorite.group.update" };
        let mut params = json!({
            "name": name.trim(),
            "description": description.trim(),
            "sort_order": sort_order,
        });
        if let Some(id) = form.id {
            params["id"] = json!(id);
        }

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, method, params).await;
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

    /// 打开编辑收藏对话框（换组/描述/排序）
    fn open_item_form(&mut self, item: FavItem, window: &mut Window, cx: &mut Context<Self>) {
        let options: Vec<StringOption> = self
            .groups
            .iter()
            .map(|g| StringOption::new(g.name.clone(), g.id.to_string()))
            .collect();
        let selected_ix = self
            .groups
            .iter()
            .position(|g| g.items.iter().any(|i| i.id == item.id))
            .unwrap_or(0);

        let group_state = cx.new(|cx| {
            SelectState::new(SearchableVec::new(options), Some(IndexPath::new(selected_ix)), window, cx)
        });
        let desc_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "favorites.favoriteDescPlaceholder"));
            if let Some(d) = item.description.clone() {
                state.set_value(d, window, cx);
            }
            state
        });
        let sort_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(t(cx, "favorites.sortOrderPlaceholder"));
            state.set_value(item.sort_order.to_string(), window, cx);
            state
        });

        self.item_form = Some(ItemForm {
            id: item.id,
            group: group_state.clone(),
            description: desc_state.clone(),
            sort_order: sort_state.clone(),
        });

        let entity = cx.entity();
        let title = t(cx, "favorites.editFavorite");
        let group_label = t(cx, "favorites.group");
        let desc_label = t(cx, "favorites.favoriteDescription");
        let sort_label = t(cx, "favorites.sortOrder");

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            dialog
                .title(title.clone())
                .w(px(480.0))
                .child(
                    v_flex()
                        .gap_3()
                        .child(field_row(&group_label, Select::new(&group_state).into_any_element()))
                        .child(field_row(&desc_label, Input::new(&desc_state).into_any_element()))
                        .child(field_row(&sort_label, Input::new(&sort_state).into_any_element())),
                )
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Primary))
                .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.submit_item_form(cx));
                    true
                })
                .on_cancel(|_, _, _| true)
        });
    }

    fn submit_item_form(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.item_form.take() else { return };
        let group_id: i64 = form
            .group
            .read(cx)
            .selected_value()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let description = form.description.read(cx).value().to_string();
        let sort_order: i32 = form.sort_order.read(cx).value().trim().parse().unwrap_or(0);

        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "favorite.update",
                json!({
                    "id": form.id,
                    "group_id": group_id,
                    "description": description.trim(),
                    "sort_order": sort_order,
                }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "common.success")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    /// 删除收藏条目（确认）
    fn confirm_delete_item(&mut self, item: FavItem, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity();
        let title = t(cx, "favorites.confirmDeleteFavorite");
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let entity = entity.clone();
            let item = item.clone();
            dialog
                .confirm()
                .title(title.clone())
                .child(item.topic_name.clone())
                .button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))
                .on_ok(move |_, _window, cx| {
                    entity.update(cx, |this, cx| this.delete_item(item.id, cx));
                    true
                })
        });
    }

    fn delete_item(&mut self, id: i64, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "favorite.delete", json!({ "id": id })).await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "favorites.removed")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            this.update(cx, |this, cx| this.reload(cx)).ok();
        })
        .detach();
    }

    fn render_group(
        &mut self,
        group: &FavGroup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (border_c, hover_c, secondary_c, muted_c, primary_c) = {
            let theme = cx.theme();
            (
                theme.border,
                theme.list_hover,
                theme.secondary,
                theme.muted_foreground,
                theme.primary,
            )
        };
        let expanded = self.expanded.contains(&group.id);
        let search_query = self
            .group_search
            .get(&group.id)
            .map(|s| s.read(cx).value().to_lowercase())
            .unwrap_or_default();
        let visible_items: Vec<&FavItem> = group
            .items
            .iter()
            .filter(|i| {
                search_query.is_empty()
                    || i.topic_name.to_lowercase().contains(&search_query)
                    || i
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&search_query)
            })
            .collect();

        // 组头
        let gid = group.id;
        let group_edit = group.clone();
        let group_del = group.clone();
        let group_desc = group.description.clone();
        let header = h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .cursor_pointer()
            .hover(|el| el.bg(hover_c))
            .child(
                Icon::new(if expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                    .size_4()
                    .text_color(muted_c),
            )
            .child(div().font_semibold().child(group.name.clone()))
            .children(group_desc.map(|d| {
                div()
                    .text_xs()
                    .text_color(muted_c)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(d)
                    .into_any_element()
            }))
            .child(
                div()
                    .text_xs()
                    .px_2()
                    .rounded_md()
                    .bg(secondary_c)
                    .child(format!("{}", visible_items.len())),
            )
            .child(div().flex_1())
            .child(
                Button::new(("edit-group", gid as usize))
                    .ghost()
                    .xsmall()
                    .icon(IconName::ALargeSmall)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_group_form(Some(group_edit.clone()), window, cx);
                    })),
            )
            .child(
                Button::new(("del-group", gid as usize))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Delete)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.confirm_delete_group(group_del.clone(), window, cx);
                    })),
            )
            .id(("group-header", gid as usize))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_group(gid, cx);
            }));

        let mut card = v_flex()
            .border_1()
            .border_color(border_c)
            .rounded_lg()
            .child(header);

        if expanded {
            // 组内搜索框
            let search_state = self.search_for(group.id, window, cx);
            card = card.child(
                div()
                    .px_3()
                    .py_1()
                    .border_t_1()
                    .border_color(border_c)
                    .child(Input::new(&search_state).small()),
            );

            // 条目
            if visible_items.is_empty() {
                card = card.child(
                    div()
                        .p_3()
                        .text_xs()
                        .text_color(muted_c)
                        .child(if search_query.is_empty() {
                            t(cx, "favorites.noItems")
                        } else {
                            t(cx, "favorites.noSearchResults")
                        }),
                );
            } else {
                for item in visible_items {
                    let item_edit = item.clone();
                    let item_del = item.clone();
                    card = card.child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(border_c)
                            .child(
                                div()
                                    .size_6()
                                    .rounded_md()
                                    .bg(primary_c.opacity(0.15))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Icon::new(IconName::FolderOpen).size_3().text_color(primary_c)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(item.topic_name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .px_1()
                                    .rounded_md()
                                    .bg(secondary_c)
                                    .text_color(muted_c)
                                    .child(item.cluster_id.clone()),
                            )
                            .children(item.description.as_ref().map(|d| {
                                div()
                                    .text_xs()
                                    .text_color(muted_c)
                                    .overflow_hidden()
                                    .child(d.clone())
                                    .into_any_element()
                            }))
                            .child(div().flex_1())
                            .child(
                                Button::new(("edit-fav", item.id as usize))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::ALargeSmall)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_item_form(item_edit.clone(), window, cx);
                                    })),
                            )
                            .child(
                                Button::new(("del-fav", item.id as usize))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Delete)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.confirm_delete_item(item_del.clone(), window, cx);
                                    })),
                            ),
                    );
                }
            }
        }

        card.into_any_element()
    }
}

impl FavGroup {
    fn sort_order(&self) -> i64 {
        0
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                Button::new("create-group")
                    .primary()
                    .icon(IconName::Plus)
                    .label(t(cx, "favorites.createGroup"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_group_form(None, window, cx);
                    })),
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
            let sections: Vec<AnyElement> = self
                .groups
                .clone()
                .iter()
                .map(|g| self.render_group(g, window, cx))
                .collect();

            div()
                .id("favorites-scroll")
                .size_full()
                .overflow_y_scroll()
                .child(v_flex().gap_3().children(sections))
                .into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(div().flex_1().overflow_hidden().child(content))
    }
}
