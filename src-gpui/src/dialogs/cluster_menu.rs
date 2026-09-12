//! 集群右键菜单动作处理（对齐 ClustersView.handleMenuAction）

use gpui::App;

use crate::app::{backend, root, AppEvent};
use crate::i18n::t;
use crate::overlay;

fn cluster_id_by_name(cx: &App, name: &str) -> Option<i64> {
    root(cx)
        .read(cx)
        .clusters
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.id)
}

pub fn test_connection(name: &str, cx: &mut App) {
    let Some(id) = cluster_id_by_name(cx, name) else { return };
    overlay::toast_info(cx, t("clusters.testing"));
    let b = backend(cx);
    let name = name.to_string();
    cx.spawn(async move |cx| {
        let result = b
            .dispatch("cluster.test", serde_json::json!({"cluster_id": id}))
            .await;
        cx.update(|cx| {
            match result {
                Ok(v) => {
                    let success = v.get("success").and_then(|x| x.as_bool()).unwrap_or(false);
                    if success {
                        let latency = v.get("latency_ms").and_then(|x| x.as_u64());
                        overlay::toast_success(
                            cx,
                            format!("{} ({}ms)", t("clusters.testSuccess"), latency.unwrap_or(0)),
                        );
                        root(cx).update(cx, |app, cx| {
                            app.tree_navigator.update(cx, |n, cx| {
                                n.set_health(&name, Some(true), None, cx);
                            });
                        });
                    } else {
                        overlay::toast_error(cx, t("clusters.testFailed"));
                        root(cx).update(cx, |app, cx| {
                            app.tree_navigator.update(cx, |n, cx| {
                                n.set_health(&name, Some(false), None, cx);
                            });
                        });
                    }
                }
                Err(e) => overlay::toast_error(cx, e),
            }
})
    })
    .detach();
}

pub fn refresh_connection(name: &str, cx: &mut App) {
    overlay::toast_info(cx, t("clusters.refreshingStatus"));
    let b = backend(cx);
    let name = name.to_string();
    cx.spawn(async move |cx| {
        let result = b
            .dispatch(
                "connection.health_check",
                serde_json::json!({"cluster_id": name}),
            )
            .await;
        cx.update(|cx| {
            let healthy = result
                .as_ref()
                .ok()
                .and_then(|v| v.get("healthy"))
                .and_then(|x| x.as_bool());
            let error = result
                .as_ref()
                .ok()
                .and_then(|v| v.get("error_message"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
            root(cx).update(cx, |app, cx| {
                app.tree_navigator.update(cx, |n, cx| {
                    n.set_health(&name, healthy, error, cx);
                });
            });
            if healthy == Some(true) {
                overlay::toast_success(cx, t("clusters.connectionHealthy"));
            } else {
                overlay::toast_error(cx, t("clusters.connectionUnhealthy"));
            }
})
    })
    .detach();
}

pub fn disconnect(name: &str, cx: &mut App) {
    let title = t("clusters.disconnectTitle");
    let message = format!("{} {}?", t("clusters.disconnectConfirm"), name);
    let name = name.to_string();
    overlay::confirm(cx, title, message, true, move |cx| {
        let b = backend(cx);
        let name = name.clone();
        cx.spawn(async move |cx| {
            let result = b
                .dispatch(
                    "connection.disconnect",
                    serde_json::json!({"cluster_name": name}),
                )
                .await;
            cx.update(|cx| {
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, t("clusters.disconnectSuccess"));
                        root(cx).update(cx, |app, cx| {
                            app.tree_navigator.update(cx, |n, cx| {
                                n.set_health(&name, Some(false), None, cx);
                            });
                        });
                    }
                    Err(e) => overlay::toast_error(cx, e),
                }
})
        })
        .detach();
    });
}

pub fn reconnect(name: &str, cx: &mut App) {
    overlay::toast_info(cx, t("clusters.reconnecting"));
    let b = backend(cx);
    let name = name.to_string();
    cx.spawn(async move |cx| {
        let result = b
            .dispatch(
                "connection.reconnect",
                serde_json::json!({"cluster_name": name}),
            )
            .await;
        cx.update(|cx| {
            match result {
                Ok(_) => {
                    overlay::toast_success(cx, t("clusters.reconnectSuccess"));
                    root(cx).update(cx, |app, cx| {
                        app.tree_navigator.update(cx, |n, cx| {
                            n.check_cluster_health_pub(&name, cx);
                        });
                    });
                }
                Err(e) => overlay::toast_error(cx, e),
            }
})
    })
    .detach();
}

pub fn remove_cluster(name: &str, cx: &mut App) {
    let Some(id) = cluster_id_by_name(cx, name) else { return };
    let title = t("common.confirmDelete");
    let message = format!("{} {}? {}", t("clusters.deleteConfirm"), name, t("clusters.deleteWarning"));
    let name = name.to_string();
    overlay::confirm(cx, title, message, true, move |cx| {
        let b = backend(cx);
        let _name = name.clone();
        cx.spawn(async move |cx| {
            let result = b
                .dispatch("cluster.delete", serde_json::json!({"cluster_id": id}))
                .await;
            cx.update(|cx| {
                match result {
                    Ok(_) => {
                        overlay::toast_success(cx, t("clusters.deleteSuccess"));
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
