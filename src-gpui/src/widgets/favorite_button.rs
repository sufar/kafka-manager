//! 收藏星标按钮（对齐 FavoriteButton.vue）

use gpui::prelude::*;
use gpui::{div, px, Context, EventEmitter, IntoElement, MouseButton, Render, Window};

use crate::app::{backend, root, AppEvent};
use crate::i18n::t;
use crate::icons::icon;
use crate::overlay;
use crate::theme;

pub struct FavoriteButton {
    pub cluster_id: String,
    pub topic_name: String,
    pub is_favorite: bool,
    busy: bool,
}

#[derive(Clone, Debug)]
pub struct FavoriteChanged(pub bool);

impl EventEmitter<FavoriteChanged> for FavoriteButton {}

impl FavoriteButton {
    pub fn new(cluster_id: String, topic_name: String, is_favorite: bool, _cx: &mut Context<Self>) -> Self {
        Self {
            cluster_id,
            topic_name,
            is_favorite,
            busy: false,
        }
    }

    /// 不确定初始状态时远程检查
    pub fn check(&mut self, cx: &mut Context<Self>) {
        let b = backend(cx);
        let (cid, tn) = (self.cluster_id.clone(), self.topic_name.clone());
        cx.spawn(async move |this, cx| {
            if let Ok(v) = b
                .dispatch(
                    "favorite.check",
                    serde_json::json!({"cluster_id": cid, "topic_name": tn}),
                )
                .await
            {
                let is_fav = v
                    .get("is_favorite")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false);
                this.update(cx, |this, cx| {
                    this.is_favorite = is_fav;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn set_favorite(&mut self, value: bool, cx: &mut Context<Self>) {
        self.is_favorite = value;
        cx.notify();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if self.is_favorite {
            // 取消收藏
            self.busy = true;
            let b = backend(cx);
            let (cid, tn) = (self.cluster_id.clone(), self.topic_name.clone());
            cx.spawn(async move |this, cx| {
                let result = b
                    .dispatch(
                        "favorite.delete_by_topic",
                        serde_json::json!({"cluster_id": cid, "topic_name": tn}),
                    )
                    .await;
                this.update(cx, |this, cx| {
                    this.busy = false;
                    match result {
                        Ok(_) => {
                            this.is_favorite = false;
                            overlay::toast_success(cx, t("favorites.removed"));
                            cx.emit(FavoriteChanged(false));
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
        } else {
            // 打开分组选择弹窗
            let (cid, tn) = (self.cluster_id.clone(), self.topic_name.clone());
            let entity = cx.entity();
            let view = cx.new(|cx| {
                crate::dialogs::favorite_add::FavoriteAddDialog::new(cid, tn, entity, cx)
            });
            overlay::open_modal(cx, view.into());
        }
    }
}

impl Render for FavoriteButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fav = self.is_favorite;
        div()
            .id("favorite-btn")
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(20.))
            .rounded(px(5.))
            .cursor_pointer()
            .hover(|s| s.bg(theme::btn_ghost_hover()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.toggle(cx)),
            )
            .child(if fav {
                icon("star-solid")
                    .size(px(13.))
                    .text_color(theme::warning())
                    .into_any_element()
            } else {
                icon("star")
                    .size(px(13.))
                    .text_color(theme::text_secondary())
                    .into_any_element()
            })
    }
}
