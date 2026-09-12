//! Topic 右键菜单（对齐 TopicContextMenu.vue + ModernLayout.handleTopicAction）

use gpui::{App, Pixels, Point};

use crate::app::{backend, root, AppEvent, Page, Route};
use crate::i18n::t;
use crate::overlay;

pub fn open(cx: &mut App, position: Point<Pixels>, cluster: String, topic: String) {
    let mk = |_cx: &mut App| (cluster.clone(), topic.clone());
    let _ = mk;
    overlay::open_context_menu(cx, overlay::ContextMenuState {
        position,
        title: Some(topic.clone()),
        separators: vec![3, 5],
        items: vec![
            overlay::ContextItem {
                label: t("contextMenu.viewMessages"),
                icon: Some("chat"),
                danger: false,
                action: {
                    let (c, t2) = (cluster.clone(), topic.clone());
                    Box::new(move |cx| {
                        root(cx).update(cx, |app, cx| {
                            let route = Route::new(Page::Messages)
                                .with("cluster", &c)
                                .with("topic", &t2);
                            app.navigate(route, true, cx);
                        });
                    })
                },
            },
            overlay::ContextItem {
                label: t("contextMenu.viewDetails"),
                icon: Some("info"),
                danger: false,
                action: {
                    let (c, t2) = (cluster.clone(), topic.clone());
                    Box::new(move |cx| {
                        root(cx).update(cx, |app, cx| {
                            let route = Route::new(Page::Topics)
                                .with("cluster", &c)
                                .with("topic", &t2);
                            app.navigate(route, true, cx);
                        });
                    })
                },
            },
            overlay::ContextItem {
                label: t("contextMenu.viewPartitions"),
                icon: Some("grid"),
                danger: false,
                action: {
                    let (c, t2) = (cluster.clone(), topic.clone());
                    Box::new(move |cx| {
                        root(cx).update(cx, |app, cx| {
                            let route = Route::new(Page::Topics)
                                .with("cluster", &c)
                                .with("topic", &t2)
                                .with("tab", "partitions");
                            app.navigate(route, true, cx);
                        });
                    })
                },
            },
            overlay::ContextItem {
                label: t("contextMenu.sendMessage"),
                icon: Some("send"),
                danger: false,
                action: {
                    let (c, t2) = (cluster.clone(), topic.clone());
                    Box::new(move |cx| {
                        root(cx).update(cx, |app, cx| {
                            let route = Route::new(Page::Messages)
                                .with("cluster", &c)
                                .with("topic", &t2)
                                .with("action", "send");
                            app.navigate(route, true, cx);
                        });
                    })
                },
            },
            overlay::ContextItem {
                label: t("contextMenu.exportData"),
                icon: Some("download"),
                danger: false,
                // 与原应用一致：该菜单项暂无 handler，点击无效果
                action: Box::new(|_| {}),
            },
            overlay::ContextItem {
                label: t("contextMenu.deleteTopic"),
                icon: Some("trash"),
                danger: true,
                action: {
                    let (c, t2) = (cluster.clone(), topic.clone());
                    Box::new(move |cx| {
                        let title = t("common.confirm");
                        let message = format!("{} {}?", t("topics.deleteConfirm"), t2);
                        overlay::confirm(cx, title, message, true, move |cx| {
                            let b = backend(cx);
                            let (c, t2) = (c.clone(), t2.clone());
                            cx.spawn(async move |cx| {
                                let result = b
                                    .dispatch(
                                        "topic.delete",
                                        serde_json::json!({"cluster_id": c, "topic": t2}),
                                    )
                                    .await;
                                cx.update(|cx| {
                                    match result {
                                        Ok(_) => {
                                            overlay::toast_success(
                                                cx,
                                                t("topics.deleteSuccess"),
                                            );
                                            root(cx).update(cx, |app, cx| {
                                                app.publish(
                                                    AppEvent::TopicDeleted {
                                                        cluster: c.clone(),
                                                        topic: t2.clone(),
                                                    },
                                                    cx,
                                                );
                                            });
                                        }
                                        Err(e) => overlay::toast_error(cx, e),
                                    }
})
                            })
                            .detach();
                        });
                    })
                },
            },
        ],
    });
}
