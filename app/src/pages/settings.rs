//! 设置页：与旧版 SettingsView 一致（卡片分区结构）
//!
//! 系统设置卡：主题/语言/侧边栏模式/系统托盘/开机自启/导入导出
//! 版本信息卡：版本徽标 + 更新按钮 + 作者 + 检查更新 + 下载进度
//! 反馈卡：文档链接 + 文本框 + 字数统计 + 提交
//! 日志模态：滚动到底/刷新/复制/清空；更新模态：版本对比 + 发布说明 + 进度

use gpui::{prelude::FluentBuilder, *};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::notification::NotificationType;
use gpui_component::select::{SearchableVec, Select, SelectState};
use gpui_component::switch::Switch;
use gpui_component::*;
use serde_json::json;

use crate::components::notify;
use crate::i18n::{t, I18n};
use crate::state::{Backend, SidebarMode, TokioRuntime};

/// 更新检查结果（页面内使用）
#[derive(Clone, Debug)]
struct UpdateInfo {
    version: String,
    notes: Option<String>,
    portable_url: Option<String>,
}

/// 下载进度
#[derive(Clone, Debug, Default)]
struct DownloadProgress {
    active: bool,
    downloaded: u64,
    total: u64,
}

pub struct SettingsPage {
    version: String,
    dark_mode: bool,
    tray_enabled: bool,
    auto_launch: bool,
    tree_mode: bool,
    lang_state: Entity<SelectState<SearchableVec<crate::components::option_select::StringOption>>>,
    checking_update: bool,
    update_info: Option<UpdateInfo>,
    download: DownloadProgress,
    logs: Option<String>,
    logs_handle: ScrollHandle,
    feedback_input: Entity<InputState>,
    feedback_available: bool,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang_state = cx.new(|cx| {
            let zh_selected = I18n::global(cx).is_zh();
            SelectState::new(
                SearchableVec::new(vec![
                    crate::components::option_select::StringOption::new("中文", "zh"),
                    crate::components::option_select::StringOption::new("English", "en"),
                ]),
                Some(IndexPath::new(if zh_selected { 0 } else { 1 })),
                window,
                cx,
            )
        });
        let feedback_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .auto_grow(3, 6)
                .placeholder(t(cx, "settings.feedbackPlaceholder"))
        });

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &lang_state,
            |_this, _, event: &gpui_component::select::SelectEvent<
                SearchableVec<crate::components::option_select::StringOption>,
            >, cx| {
                if let gpui_component::select::SelectEvent::Confirm(Some(lang)) = event {
                    let lang = lang.to_string();
                    I18n::global_mut(cx).set_lang(if lang == "en" { "en" } else { "zh" });
                    if let Some(rt) = cx.try_global::<TokioRuntime>().map(|r| r.0.clone()) {
                        if let Some(state) = Backend::state(cx) {
                            cx.spawn(async move |_this, _cx| {
                                let _ = crate::service::call(
                                    &rt,
                                    state,
                                    "settings.update",
                                    json!({ "key": "ui.language", "value": lang }),
                                )
                                .await;
                            })
                            .detach();
                        }
                    }
                    cx.refresh_windows();
                }
            },
        ));

        let this = Self {
            version: String::new(),
            dark_mode: cx.theme().is_dark(),
            tray_enabled: false,
            auto_launch: current_auto_launch().unwrap_or(false),
            tree_mode: SidebarMode::is_tree(cx),
            lang_state,
            checking_update: false,
            update_info: None,
            download: DownloadProgress::default(),
            logs: None,
            logs_handle: ScrollHandle::default(),
            feedback_input,
            feedback_available: false,
            _subscriptions: subscriptions,
        };
        this.init(cx);
        this
    }

    fn init(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let version = crate::service::call(&rt, state.clone(), "app.version", json!({})).await;
            let tray = crate::service::call(
                &rt,
                state.clone(),
                "settings.get",
                json!({ "keys": ["ui.system_tray"] }),
            )
            .await;
            let feedback = crate::service::call(&rt, state, "telemetry.check_connection", json!({})).await;

            this.update(cx, |this, cx| {
                if let Ok(v) = version {
                    this.version = v.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                }
                if let Ok(v) = tray {
                    this.tray_enabled = v
                        .get("settings")
                        .and_then(|s| s.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|s| s.get("value"))
                        .and_then(|v| v.as_str())
                        .map(|v| v == "true")
                        .unwrap_or(false);
                }
                if let Ok(v) = feedback {
                    this.feedback_available = v
                        .get("connected")
                        .and_then(|c| c.as_bool())
                        .unwrap_or(false);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn set_dark_mode(&mut self, dark: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.dark_mode = dark;
        gpui_component::Theme::change(
            if dark { ThemeMode::Dark } else { ThemeMode::Light },
            Some(window),
            cx,
        );
        let rt = TokioRuntime::handle(cx);
        if let Some(state) = Backend::state(cx) {
            cx.spawn(async move |_this, _cx| {
                let _ = crate::service::call(
                    &rt,
                    state,
                    "settings.update",
                    json!({ "key": "ui.theme", "value": if dark { "dark" } else { "light" } }),
                )
                .await;
            })
            .detach();
        }
        cx.notify();
    }

    fn set_tray(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.tray_enabled = enabled;
        crate::tray::set_tray_enabled(enabled);
        let rt = TokioRuntime::handle(cx);
        if let Some(state) = Backend::state(cx) {
            cx.spawn(async move |_this, _cx| {
                let _ = crate::service::call(
                    &rt,
                    state,
                    "settings.update",
                    json!({ "key": "ui.system_tray", "value": if enabled { "true" } else { "false" } }),
                )
                .await;
            })
            .detach();
        }
        cx.notify();
    }

    fn set_auto_launch(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.auto_launch = enabled;
        if let Err(e) = set_auto_launch_os(enabled) {
            self.auto_launch = !enabled;
            notify(cx, NotificationType::Error, e);
        }
        cx.notify();
    }

    /// 切换侧边栏模式（树形/平铺）
    fn set_tree_mode(&mut self, tree: bool, cx: &mut Context<Self>) {
        self.tree_mode = tree;
        SidebarMode::set(cx, tree);
        let rt = TokioRuntime::handle(cx);
        if let Some(state) = Backend::state(cx) {
            cx.spawn(async move |_this, _cx| {
                let _ = crate::service::call(
                    &rt,
                    state,
                    "settings.update",
                    json!({ "key": "ui.sidebar_mode", "value": if tree { "tree" } else { "flat" } }),
                )
                .await;
            })
            .detach();
        }
        cx.refresh_windows();
        cx.notify();
    }

    /// 检查更新
    fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.checking_update {
            return;
        }
        self.checking_update = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = crate::updater::do_check_updates().await;
            this.update(cx, |this, cx| {
                this.checking_update = false;
                match result {
                    Ok(r) if r.available => {
                        this.update_info = Some(UpdateInfo {
                            version: r.version,
                            notes: r.notes,
                            portable_url: r.portable_download_url,
                        });
                        cx.notify();
                        this.open_update_modal(cx);
                    }
                    Ok(_) => {
                        this.update_info = None;
                        cx.notify();
                        let msg = t(cx, "update.checkCompleteNoUpdate");
                        notify(cx, NotificationType::Info, msg);
                    }
                    Err(e) => {
                        cx.notify();
                        if e.contains("403") {
                            let msg = t(cx, "update.rateLimitExceeded");
                            notify(cx, NotificationType::Error, msg);
                        } else {
                            let msg = format!("{}: {}", t(cx, "update.checkFailed"), e);
                            notify(cx, NotificationType::Error, msg);
                        }
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// 更新模态（版本对比 + 发布说明 + 下载按钮/进度）
    fn open_update_modal(&mut self, cx: &mut Context<Self>) {
        let Some(info) = self.update_info.clone() else { return };
        let entity = cx.entity();
        let current = self.version.clone();
        let title = t(cx, "update.available");
        let notes_label = t(cx, "update.releaseNotes");
        let download_label = t(cx, "update.updateAndRestart");

        if let Some(w) = cx.windows().first().cloned() {
            let _ = w.update(cx, |_, window, cx| {
                let muted = cx.theme().muted_foreground;
                let primary = cx.theme().primary;
                let border = cx.theme().border;
                window.open_dialog(cx, move |dialog, _window, _cx| {
                    let entity = entity.clone();
                    dialog
                        .title(title.clone())
                        .w(px(560.0))
                        .child(
                            v_flex()
                                .gap_3()
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            div()
                                                .text_sm()
                                                .px_2()
                                                .py_1()
                                                .rounded_md()
                                                .border_1()
                                                .border_color(border)
                                                .child(format!("v{}", current)),
                                        )
                                        .child(Icon::new(IconName::ArrowRight).size_4())
                                        .child(
                                            div()
                                                .text_sm()
                                                .px_2()
                                                .py_1()
                                                .rounded_md()
                                                .bg(primary.opacity(0.15))
                                                .text_color(primary)
                                                .child(format!("v{}", info.version)),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(div().text_sm().font_semibold().child(notes_label.clone()))
                                        .child(
                                            div()
                                                .id("notes-scroll")
                                                .max_h(px(240.0))
                                                .overflow_y_scroll()
                                                .p_2()
                                                .border_1()
                                                .border_color(border)
                                                .rounded_md()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(muted)
                                                        .child(info.notes.clone().unwrap_or_default()),
                                                ),
                                        ),
                                )
                                .when(info.portable_url.is_some(), |el| {
                                    let url = info.portable_url.clone().unwrap();
                                    let version = info.version.clone();
                                    el.child(
                                        Button::new("download-install")
                                            .primary()
                                            .label(download_label.clone())
                                            .on_click(move |_, window, cx| {
                                                window.close_dialog(cx);
                                                entity.update(cx, |this, cx| {
                                                    this.download_and_install(url.clone(), version.clone(), cx);
                                                });
                                            }),
                                    )
                                }),
                        )
                        .alert()
                });
            });
        }
    }

    /// 后台下载并安装更新
    fn download_and_install(&mut self, url: String, version: String, cx: &mut Context<Self>) {
        self.download = DownloadProgress {
            active: true,
            downloaded: 0,
            total: 0,
        };
        cx.notify();

        cx.spawn(async move |this, cx| {
            let filename = format!("kafka-manager-{}-portable.zip", version);
            let progress = std::sync::Arc::new(std::sync::Mutex::new((0u64, 0u64)));
            let progress_cb = progress.clone();
            let progress_poll = progress.clone();
            let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let done_poll = done.clone();

            // 轮询进度更新 UI
            let this_poll = this.clone();
            cx.spawn(async move |cx| {
                while !done_poll.load(std::sync::atomic::Ordering::Relaxed) {
                    let (d, t) = *progress_poll.lock().unwrap();
                    this_poll
                        .update(cx, |this, cx| {
                            this.download.downloaded = d;
                            this.download.total = t;
                            cx.notify();
                        })
                        .ok();
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(200))
                        .await;
                }
            })
            .detach();

            let result = crate::updater::download_update(&url, &filename, move |downloaded, total| {
                *progress_cb.lock().unwrap() = (downloaded, total);
            })
            .await;
            done.store(true, std::sync::atomic::Ordering::Relaxed);

            this
                .update(cx, |this, cx| {
                    this.download.active = false;
                    cx.notify();
                })
                .ok();
            cx.update(|cx| {
                match result {
                    Ok(path) => {
                        notify(cx, NotificationType::Success, t(cx, "update.downloadComplete"));
                        if let Err(e) = crate::updater::install_portable_update(&path) {
                            notify(cx, NotificationType::Error, format!("{}: {}", t(cx, "update.installFailed"), e));
                        }
                    }
                    Err(e) => {
                        notify(cx, NotificationType::Error, format!("{}: {}", t(cx, "update.downloadFailed"), e));
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// 打开日志模态
    fn open_logs(&mut self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "app.logs", json!({})).await;
            this.update(cx, |this, cx| {
                if let Ok(value) = result {
                    this.logs = value.get("logs").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
                cx.notify();
            })
            .ok();
            // 打开模态
            this.update(cx, |_this, cx| {
                let entity = cx.entity();
                let title = t(cx, "settings.appLogs");
                if let Some(w) = cx.windows().first() {
                    let _ = w.update(cx, |_, window, cx| {
                        let border = cx.theme().border;
                        let muted = cx.theme().muted_foreground;
                        window.open_dialog(cx, move |dialog, _window, _cx| {
                            let entity = entity.clone();
                            let entity_bottom = entity.clone();
                            let entity_refresh = entity.clone();
                            let logs_text = entity
                                .read(_cx)
                                .logs
                                .clone()
                                .unwrap_or_else(|| t(_cx, "settings.noLogs"));
                            let lines: Vec<&str> = logs_text.lines().collect();
                            let start = lines.len().saturating_sub(500);
                            let tail: String = lines[start..].join("\n");
                            dialog
                                .title(title.clone())
                                .w(px(860.0))
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    Button::new("logs-bottom")
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::ArrowDown)
                                                        .tooltip(t(_cx, "settings.scrollToBottom"))
                                                        .on_click({
                                                            let handle = entity_bottom.read(_cx).logs_handle.clone();
                                                            move |_, _, _| {
                                                                handle.scroll_to_bottom();
                                                            }
                                                        }),
                                                )
                                                .child(
                                                    Button::new("logs-refresh")
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Redo2)
                                                        .tooltip(t(_cx, "settings.refreshLogs"))
                                                        .on_click(move |_, window, cx| {
                                                            window.close_dialog(cx);
                                                            entity_refresh.update(cx, |this, cx| this.open_logs(cx));
                                                        }),
                                                )
                                                .child(
                                                    Button::new("logs-copy")
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Copy)
                                                        .tooltip(t(_cx, "settings.copyLogs"))
                                                        .on_click({
                                                            let tail = tail.clone();
                                                            move |_, _, cx| {
                                                                cx.write_to_clipboard(ClipboardItem::new_string(tail.clone()));
                                                            }
                                                        }),
                                                )
                                                .child(
                                                    Button::new("logs-clear")
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::Delete)
                                                        .tooltip(t(_cx, "settings.clearLogs"))
                                                        .on_click(move |_, window, cx| {
                                                            window.close_dialog(cx);
                                                        }),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("logs-scroll")
                                                .h(px(480.0))
                                                .overflow_y_scroll()
                                                .track_scroll(&entity_bottom.read(_cx).logs_handle)
                                                .p_2()
                                                .border_1()
                                                .border_color(border)
                                                .rounded_md()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(muted)
                                                        .child(tail.clone()),
                                                ),
                                        ),
                                )
                                .alert()
                        });
                    });
                }
            })
            .ok();
        })
        .detach();
    }

    /// 导出数据
    fn export_data(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |_this, cx| {
            let result = crate::service::call(&rt, state, "settings.export", json!({})).await;
            let Ok(data) = result else {
                cx.update(|cx| notify(cx, NotificationType::Error, result.err().unwrap())).ok();
                return;
            };
            let path = tokio::task::spawn_blocking(|| {
                rfd::FileDialog::new()
                    .set_file_name(&format!(
                        "kafka-manager-export-{}.json",
                        chrono::Local::now().format("%Y-%m-%d")
                    ))
                    .add_filter("JSON", &["json"])
                    .save_file()
            })
            .await
            .ok()
            .flatten();
            let Some(path) = path else { return };
            let write_result = serde_json::to_string_pretty(&data)
                .map_err(|e| e.to_string())
                .and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()));
            cx.update(|cx| match write_result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "settings.exportSuccess")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
        })
        .detach();
    }

    /// 导入数据
    fn import_data(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |_this, cx| {
            let path = tokio::task::spawn_blocking(|| {
                rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file()
            })
            .await
            .ok()
            .flatten();
            let Some(path) = path else { return };
            let Ok(content) = std::fs::read_to_string(&path) else { return };
            let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) else { return };
            let result = crate::service::call(
                &rt,
                state,
                "settings.import",
                json!({ "data": data, "strategy": "skip" }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "settings.importStarted")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
        })
        .detach();
    }

    /// 提交反馈
    fn submit_feedback(&mut self, cx: &mut Context<Self>) {
        let content = self.feedback_input.read(cx).value().to_string();
        if content.trim().is_empty() {
            return;
        }
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "telemetry.submit_feedback",
                json!({ "feedback_content": content }),
            )
            .await;
            cx.update(|cx| match result {
                Ok(_) => notify(cx, NotificationType::Success, t(cx, "settings.feedbackSuccess")),
                Err(e) => notify(cx, NotificationType::Error, e),
            })
            .ok();
            // 清空输入
            this.update(cx, |this, cx| {
                if let Some(w) = cx.windows().first() {
                    let _ = w.update(cx, |_, window, cx| {
                        this.feedback_input.update(cx, |s, cx| {
                            s.set_value(String::new(), window, cx);
                        });
                    });
                }
            })
            .ok();
        })
        .detach();
    }

    /// 卡片容器
    fn card(title: String, desc: String, children: Vec<AnyElement>, cx: &App) -> Div {
        let theme = cx.theme();
        v_flex()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        v_flex()
                            .child(div().text_sm().font_semibold().child(title))
                            .when(!desc.is_empty(), |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(desc),
                                )
                            }),
                    ),
            )
            .child(v_flex().children(children))
    }

    /// 设置行：标签 + 描述 | 控件
    fn setting_row(label: String, desc: String, control: AnyElement, cx: &App) -> Div {
        let theme = cx.theme();
        h_flex()
            .items_center()
            .justify_between()
            .px_3()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                v_flex()
                    .gap_0p5()
                    .child(div().text_sm().child(label))
                    .when(!desc.is_empty(), |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(desc),
                        )
                    }),
            )
            .child(control)
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let dark = self.dark_mode;
        let tray = self.tray_enabled;
        let auto_launch = self.auto_launch;
        let tree_mode = self.tree_mode;
        let checking = self.checking_update;
        let version = if self.version.is_empty() { "...".to_string() } else { self.version.clone() };

        // ===== 系统设置卡 =====
        let system_card = Self::card(
            t(cx, "settings.systemSettings"),
            String::new(),
            vec![
                Self::setting_row(
                    t(cx, "settings.theme"),
                    if dark { t(cx, "settings.darkMode") } else { t(cx, "settings.lightMode") },
                    Switch::new("theme-switch")
                        .checked(dark)
                        .on_click(cx.listener(|this, checked: &bool, window, cx| {
                            this.set_dark_mode(*checked, window, cx);
                        }))
                        .into_any_element(),
                    cx,
                )
                .into_any_element(),
                Self::setting_row(
                    t(cx, "settings.language"),
                    String::new(),
                    div().w_32().child(Select::new(&self.lang_state).small()).into_any_element(),
                    cx,
                )
                .into_any_element(),
                Self::setting_row(
                    t(cx, "settings.sidebarMode"),
                    if tree_mode {
                        t(cx, "settings.treeModeDesc")
                    } else {
                        t(cx, "settings.flatModeDesc")
                    },
                    Switch::new("sidebar-mode-switch")
                        .checked(tree_mode)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.set_tree_mode(*checked, cx);
                        }))
                        .into_any_element(),
                    cx,
                )
                .into_any_element(),
                Self::setting_row(
                    t(cx, "settings.systemTray"),
                    t(cx, "settings.systemTrayDesc"),
                    Switch::new("tray-switch")
                        .checked(tray)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.set_tray(*checked, cx);
                        }))
                        .into_any_element(),
                    cx,
                )
                .into_any_element(),
                Self::setting_row(
                    t(cx, "settings.autoLaunch"),
                    t(cx, "settings.autoLaunchDesc"),
                    Switch::new("autolaunch-switch")
                        .checked(auto_launch)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.set_auto_launch(*checked, cx);
                        }))
                        .into_any_element(),
                    cx,
                )
                .into_any_element(),
                Self::setting_row(
                    t(cx, "settings.importExport"),
                    t(cx, "settings.importExportDesc"),
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("export-data")
                                .primary()
                                .xsmall()
                                .label(t(cx, "settings.exportData"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.export_data(cx);
                                })),
                        )
                        .child(
                            Button::new("import-data")
                                .outline()
                                .xsmall()
                                .label(t(cx, "settings.importData"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.import_data(cx);
                                })),
                        )
                        .into_any_element(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        );

        // ===== 版本信息卡 =====
        let mut version_rows: Vec<AnyElement> = vec![
            Self::setting_row(
                t(cx, "settings.currentVersion"),
                String::new(),
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(theme.primary.opacity(0.15))
                            .text_color(theme.primary)
                            .child(version.clone()),
                    )
                    .children(self.update_info.as_ref().map(|info| {
                        Button::new("update-badge")
                            .warning()
                            .xsmall()
                            .label(format!("{} v{}", t(cx, "update.newVersion"), info.version))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_update_modal(cx);
                            }))
                            .into_any_element()
                    }))
                    .into_any_element(),
                cx,
            )
            .into_any_element(),
            Self::setting_row(
                t(cx, "settings.author"),
                String::new(),
                div().text_sm().text_color(theme.muted_foreground).child("朱占全").into_any_element(),
                cx,
            )
            .into_any_element(),
            Self::setting_row(
                t(cx, "update.checkForUpdates"),
                String::new(),
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("view-logs")
                            .ghost()
                            .xsmall()
                            .label(t(cx, "settings.viewLogs"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_logs(cx);
                            })),
                    )
                    .child(
                        Button::new("check-updates")
                            .outline()
                            .xsmall()
                            .label(if checking { t(cx, "update.checking") } else { t(cx, "update.checkNow") })
                            .loading(checking)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.check_updates(cx);
                            })),
                    )
                    .into_any_element(),
                cx,
            )
            .into_any_element(),
        ];

        // 下载进度条
        if self.download.active {
            let pct = if self.download.total > 0 {
                self.download.downloaded as f32 / self.download.total as f32 * 100.0
            } else {
                0.0
            };
            version_rows.push(
                v_flex()
                    .gap_1()
                    .px_3()
                    .py_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_xs()
                            .child(format!(
                                "{} {}/{} ({:.0}%)",
                                t(cx, "update.downloading"),
                                self.download.downloaded,
                                self.download.total,
                                pct
                            )),
                    )
                    .child(gpui_component::progress::Progress::new().value(pct))
                    .into_any_element(),
            );
        }

        let version_card = Self::card(
            t(cx, "settings.version"),
            t(cx, "settings.versionDesc"),
            version_rows,
            cx,
        );

        // ===== 反馈卡 =====
        let content_len = self.feedback_input.read(cx).value().len();
        let feedback_card: AnyElement = if self.feedback_available {
            Self::card(
                t(cx, "settings.feedback"),
                t(cx, "settings.feedbackDesc"),
                vec![
                    div()
                        .px_3()
                        .py_2()
                        .child(Input::new(&self.feedback_input))
                        .into_any_element(),
                    h_flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(format!("{} / 2000", content_len)),
                        )
                        .child(
                            Button::new("submit-feedback")
                                .primary()
                                .xsmall()
                                .label(t(cx, "settings.feedbackSubmit"))
                                .disabled(content_len == 0)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.submit_feedback(cx);
                                })),
                        )
                        .into_any_element(),
                ],
                cx,
            )
            .into_any_element()
        } else {
            div().into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .child(
                v_flex()
                    .gap_1()
                    .mb_4()
                    .child(div().text_xl().font_semibold().child(t(cx, "settings.title")))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(t(cx, "settings.description")),
                    ),
            )
            .child(
                div()
                    .id("settings-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .gap_4()
                            .max_w(px(720.0))
                            .child(system_card)
                            .child(version_card)
                            .child(feedback_card),
                    ),
            )
    }
}

/// 当前开机自启状态
fn current_auto_launch() -> Option<bool> {
    let exe = std::env::current_exe().ok()?;
    let auto = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("kafka-manager")
        .set_app_path(exe.to_str()?)
        .build()
        .ok()?;
    auto.is_enabled().ok()
}

/// 设置开机自启
fn set_auto_launch_os(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let auto = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("kafka-manager")
        .set_app_path(exe.to_string_lossy().as_ref())
        .build()
        .map_err(|e| e.to_string())?;
    if enabled {
        auto.enable().map_err(|e| e.to_string())
    } else {
        auto.disable().map_err(|e| e.to_string())
    }
}
