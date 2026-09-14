//! 右键菜单共享动作：集群测试/断开/重连、Topic 删除/配置查看、消费组删除
//!
//! 动作均为自包含的后端调用 + Toast 反馈；需要刷新调用方数据的通过 `on_done` 回调通知。

use gpui::*;
use gpui_component::dialog::DialogButtonProps;
use gpui_component::notification::NotificationType;
use gpui_component::v_flex;
use gpui_component::{ActiveTheme, WindowExt};

use crate::components::notify;
use crate::i18n::t;
use crate::service;
use crate::state::{Backend, TokioRuntime};

/// 测试集群连接（cluster.test）并 Toast 结果
pub fn test_cluster(cx: &mut App, cluster: String) {
    let Some(state) = Backend::state(cx) else { return };
    let rt = TokioRuntime::handle(cx);
    let ok_msg = t(cx, "clusters.connectionTestSuccess");
    let fail_msg = t(cx, "clusters.connectionTestFailed");
    cx.spawn(async move |cx| {
        match service::call(&rt, state, "cluster.test", serde_json::json!({ "name": cluster })).await
        {
            Ok(val) => {
                let success = val.get("success").and_then(|v| v.as_bool()).unwrap_or(true);
                let msg = val
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| if success { ok_msg } else { fail_msg });
                cx.update(|cx| {
                    notify(
                        cx,
                        if success { NotificationType::Success } else { NotificationType::Error },
                        msg,
                    );
                })
                .ok();
            }
            Err(e) => {
                cx.update(|cx| {
                    notify(cx, NotificationType::Error, format!("{}: {}", fail_msg, e));
                })
                .ok();
            }
        }
    })
    .detach();
}

/// 断开集群连接（connection.disconnect）
pub fn disconnect_cluster(cx: &mut App, cluster: String, on_done: impl Fn(&mut App) + 'static) {
    let Some(state) = Backend::state(cx) else { return };
    let rt = TokioRuntime::handle(cx);
    let ok_msg = t(cx, "clusters.disconnectedSuccess");
    let fail_msg = t(cx, "toast.operationFailed");
    cx.spawn(async move |cx| {
        let result = service::call(
            &rt,
            state,
            "connection.disconnect",
            serde_json::json!({ "cluster_id": cluster }),
        )
        .await;
        cx.update(|cx| {
            match result {
                Ok(_) => {
                    notify(cx, NotificationType::Success, ok_msg);
                    on_done(cx);
                }
                Err(e) => notify(cx, NotificationType::Error, format!("{}: {}", fail_msg, e)),
            }
        })
        .ok();
    })
    .detach();
}

/// 重连集群（connection.reconnect）
pub fn reconnect_cluster(cx: &mut App, cluster: String, on_done: impl Fn(&mut App) + 'static) {
    let Some(state) = Backend::state(cx) else { return };
    let rt = TokioRuntime::handle(cx);
    let ok_msg = t(cx, "clusters.reconnectSuccess");
    let fail_msg = t(cx, "clusters.reconnectFailed");
    cx.spawn(async move |cx| {
        let result = service::call(
            &rt,
            state,
            "connection.reconnect",
            serde_json::json!({ "cluster_id": cluster }),
        )
        .await;
        cx.update(|cx| {
            match result {
                Ok(_) => {
                    notify(cx, NotificationType::Success, ok_msg);
                    on_done(cx);
                }
                Err(e) => notify(cx, NotificationType::Error, format!("{}: {}", fail_msg, e)),
            }
        })
        .ok();
    })
    .detach();
}

/// 删除 Topic（确认对话框 → topic.delete）
pub fn delete_topic(
    window: &mut Window,
    cx: &mut App,
    cluster: String,
    topic: String,
    on_done: impl Fn(&mut App) + 'static,
) {
    let confirm_text = t(cx, "layout.confirmDeleteTopic").replace("{topic}", &topic);
    let ok_msg = t(cx, "topics.deletedSuccess");
    let fail_msg = t(cx, "toast.operationFailed");
    let on_done = std::rc::Rc::new(on_done);

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let on_done = on_done.clone();
        let cluster = cluster.clone();
        let topic = topic.clone();
        let ok_msg = ok_msg.clone();
        let fail_msg = fail_msg.clone();
        dialog
            .title(t(_cx, "topicContextMenu.deleteTopic"))
            .w(px(400.0))
            .button_props(
                DialogButtonProps::default()
                    .ok_variant(gpui_component::button::ButtonVariant::Danger),
            )
            .child(div().text_sm().child(confirm_text.clone()))
            .confirm()
            .on_ok(move |_, _window, cx| {
                let on_done = on_done.clone();
                let cluster = cluster.clone();
                let topic = topic.clone();
                let ok_msg = ok_msg.clone();
                let fail_msg = fail_msg.clone();
                let Some(state) = Backend::state(cx) else { return true };
                let rt = TokioRuntime::handle(cx);
                cx.spawn(async move |cx| {
                    let result = service::call(
                        &rt,
                        state,
                        "topic.delete",
                        serde_json::json!({ "cluster_id": cluster, "topic": topic }),
                    )
                    .await;
                    cx.update(|cx| {
                        match result {
                            Ok(_) => {
                                notify(cx, NotificationType::Success, ok_msg);
                                on_done(cx);
                            }
                            Err(e) => {
                                notify(cx, NotificationType::Error, format!("{}: {}", fail_msg, e))
                            }
                        }
                    })
                    .ok();
                })
                .detach();
                true
            })
    });
}

/// 删除消费组（确认对话框 → consumer_group.delete）
pub fn delete_consumer_group(
    window: &mut Window,
    cx: &mut App,
    cluster: String,
    group: String,
    on_done: impl Fn(&mut App) + 'static,
) {
    let confirm_text = t(cx, "consumerGroups.confirmDelete").replace("{name}", &group);
    let ok_msg = t(cx, "consumerGroups.deleted");
    let fail_msg = t(cx, "toast.operationFailed");
    let on_done = std::rc::Rc::new(on_done);

    window.open_dialog(cx, move |dialog, _window, _cx| {
        let on_done = on_done.clone();
        let cluster = cluster.clone();
        let group = group.clone();
        let ok_msg = ok_msg.clone();
        let fail_msg = fail_msg.clone();
        dialog
            .title(t(_cx, "consumerGroups.deleteGroup"))
            .w(px(400.0))
            .button_props(
                DialogButtonProps::default()
                    .ok_variant(gpui_component::button::ButtonVariant::Danger),
            )
            .child(div().text_sm().child(confirm_text.clone()))
            .confirm()
            .on_ok(move |_, _window, cx| {
                let on_done = on_done.clone();
                let cluster = cluster.clone();
                let group = group.clone();
                let ok_msg = ok_msg.clone();
                let fail_msg = fail_msg.clone();
                let Some(state) = Backend::state(cx) else { return true };
                let rt = TokioRuntime::handle(cx);
                cx.spawn(async move |cx| {
                    let result = service::call(
                        &rt,
                        state,
                        "consumer_group.delete",
                        serde_json::json!({ "cluster_id": cluster, "group_id": group }),
                    )
                    .await;
                    cx.update(|cx| {
                        match result {
                            Ok(_) => {
                                notify(cx, NotificationType::Success, ok_msg);
                                on_done(cx);
                            }
                            Err(e) => {
                                notify(cx, NotificationType::Error, format!("{}: {}", fail_msg, e))
                            }
                        }
                    })
                    .ok();
                })
                .detach();
                true
            })
    });
}

/// 查看 Topic 配置（topic.config_get → 只读表格对话框）
pub fn open_topic_config(window: &mut Window, cx: &mut App, cluster: String, topic: String) {
    let Some(state) = Backend::state(cx) else { return };
    let rt = TokioRuntime::handle(cx);
    let title = t(cx, "topics.topicDetails");
    let loading_label = t(cx, "common.loading");
    let fail_msg = t(cx, "toast.operationFailed");

    // 共享加载状态：None=加载中，Some(Ok(entries))/Some(Err(msg))
    let result: std::rc::Rc<std::cell::RefCell<Option<Result<Vec<(String, String)>, String>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    cx.spawn({
        let result = result.clone();
        let topic = topic.clone();
        async move |cx| {
            let call = service::call(
                &rt,
                state,
                "topic.config_get",
                serde_json::json!({ "cluster_id": cluster, "topic": topic }),
            )
            .await;
            let parsed = call.map(|val| {
                // 兼容 {"configs": [{name,value}]} 与 {name: value} 两种返回
                if let Some(arr) = val.get("configs").and_then(|v| v.as_array()) {
                    arr.iter()
                        .filter_map(|c| {
                            Some((
                                c.get("name")?.as_str()?.to_string(),
                                c.get("value")
                                    .map(|v| match v {
                                        serde_json::Value::String(s) => s.clone(),
                                        other => other.to_string(),
                                    })
                                    .unwrap_or_default(),
                            ))
                        })
                        .collect::<Vec<_>>()
                } else if let Some(obj) = val.as_object() {
                    obj.iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                },
                            )
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            });
            *result.borrow_mut() = Some(parsed);
            cx.update(|cx| {
                for w in cx.windows() {
                    let _ = w.update(cx, |_, window, _cx| window.refresh());
                }
            })
            .ok();
        }
    })
    .detach();

    let border = cx.theme().border;
    let muted = cx.theme().muted_foreground;
    window.open_dialog(cx, move |dialog, _window, _cx| {
        let state_view = result.borrow();
        let body: AnyElement = match &*state_view {
            None => div()
                .p_4()
                .text_sm()
                .text_color(muted)
                .child(loading_label.clone())
                .into_any_element(),
            Some(Err(e)) => div()
                .p_4()
                .text_sm()
                .text_color(gpui::red())
                .child(format!("{}: {}", fail_msg, e))
                .into_any_element(),
            Some(Ok(entries)) => {
                let rows: Vec<AnyElement> = entries
                    .iter()
                    .map(|(k, v)| {
                        h_flex_row(&border, k, v)
                    })
                    .collect();
                div()
                    .id("topic-config-scroll")
                    .max_h(px(360.0))
                    .overflow_y_scroll()
                    .child(v_flex().children(rows))
                    .into_any_element()
            }
        };
        dialog
            .title(format!("{} — {}", title, topic))
            .w(px(520.0))
            .child(body)
            .alert()
    });
}

fn h_flex_row(border: &gpui::Hsla, k: &str, v: &str) -> AnyElement {
    gpui_component::h_flex()
        .gap_2()
        .p_2()
        .border_b_1()
        .border_color(*border)
        .child(
            div()
                .w(px(220.0))
                .flex_shrink_0()
                .text_sm()
                .font_family("monospace")
                .child(k.to_string()),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .font_family("monospace")
                .child(v.to_string()),
        )
        .into_any_element()
}
