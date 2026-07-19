//! 设置页：主题、语言、版本、日志、数据导入导出

use gpui::*;
use gpui_component::button::Button;
use gpui_component::notification::NotificationType;
use gpui_component::switch::Switch;
use gpui_component::*;
use serde_json::json;

use crate::i18n::{t, I18n};
use crate::pages::clusters::notify;
use crate::state::{Backend, SidebarMode, TokioRuntime};

pub struct SettingsPage {
    version: String,
    dark_mode: bool,
    english: bool,
    auto_launch: bool,
    tray_enabled: bool,
    checking_update: bool,
    tree_mode: bool,
}

impl SettingsPage {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dark_mode = cx.theme().is_dark();
        let english = !I18n::global(cx).is_zh();
        let auto_launch = current_auto_launch().unwrap_or(false);
        let this = Self {
            version: String::new(),
            dark_mode,
            english,
            auto_launch,
            tray_enabled: false,
            checking_update: false,
            tree_mode: SidebarMode::is_tree(cx),
        };
        this.load_version(cx);
        this.load_tray_setting(cx);
        this
    }

    /// 当前开机自启状态（auto-launch crate，Windows 注册表 / macOS LaunchAgent / Linux .desktop）
    fn load_tray_setting(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };
        cx.spawn(async move |this, cx| {
            let result = crate::service::call(
                &rt,
                state,
                "settings.get",
                json!({ "keys": ["ui.system_tray"] }),
            )
            .await;
            this.update(cx, |this, cx| {
                if let Ok(value) = result {
                    this.tray_enabled = value
                        .get("settings")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|s| s.get("value"))
                        .and_then(|v| v.as_str())
                        .map(|v| v == "true")
                        .unwrap_or(false);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn load_version(&self, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |this, cx| {
            let result = crate::service::call(&rt, state, "app.version", json!({})).await;
            this.update(cx, |this, cx| {
                if let Ok(value) = result {
                    this.version = value
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// 切换主题并持久化
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

    /// 切换语言并持久化
    fn set_english(&mut self, english: bool, cx: &mut Context<Self>) {
        self.english = english;
        I18n::global_mut(cx).set_lang(if english { "en" } else { "zh" });
        let rt = TokioRuntime::handle(cx);
        if let Some(state) = Backend::state(cx) {
            cx.spawn(async move |_this, _cx| {
                let _ = crate::service::call(
                    &rt,
                    state,
                    "settings.update",
                    json!({ "key": "ui.language", "value": if english { "en" } else { "zh" } }),
                )
                .await;
            })
            .detach();
        }
        cx.refresh_windows();
        cx.notify();
    }

    /// 查看日志
    fn open_logs(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let rt = TokioRuntime::handle(cx);
        let Some(state) = Backend::state(cx) else { return };

        cx.spawn(async move |_this, cx| {
            let result = crate::service::call(&rt, state, "app.logs", json!({})).await;
            cx.update(|cx| match result {
                Ok(value) => {
                    let logs = value
                        .get("logs")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let log_file = value
                        .get("log_file")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    // 只显示最后 500 行
                    let lines: Vec<&str> = logs.lines().collect();
                    let start = lines.len().saturating_sub(500);
                    let tail: String = lines[start..].join("\n");

                    for w in cx.windows() {
                        let tail = tail.clone();
                        let log_file = log_file.clone();
                        let _ = w.update(cx, |_, window, cx| {
                            window.open_dialog(cx, move |dialog, _window, _cx| {
                                dialog
                                    .title(log_file.clone())
                                    .w(px(860.0))
                                    .child(
                                        div()
                                            .id("logs-scroll")
                                            .h(px(480.0))
                                            .overflow_y_scroll()
                                            .child(div().text_xs().child(tail.clone())),
                                    )
                                    .alert()
                            });
                        });
                    }
                }
                Err(e) => notify(cx, NotificationType::Error, e),
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
                    .set_file_name("kafka_manager_backup.json")
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
                rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
            })
            .await
            .ok()
            .flatten();
            let Some(path) = path else { return };
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    cx.update(|cx| notify(cx, NotificationType::Error, e.to_string())).ok();
                    return;
                }
            };
            let data: serde_json::Value = match serde_json::from_str(&content) {
                Ok(d) => d,
                Err(e) => {
                    cx.update(|cx| notify(cx, NotificationType::Error, e.to_string())).ok();
                    return;
                }
            };
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

    fn muted_color(&self, cx: &App) -> Hsla {
        cx.theme().muted_foreground
    }

    /// 切换开机自启
    fn set_auto_launch(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.auto_launch = enabled;
        match set_auto_launch_os(enabled) {
            Ok(_) => {}
            Err(e) => {
                self.auto_launch = !enabled;
                notify(cx, NotificationType::Error, e);
            }
        }
        cx.notify();
    }

    /// 切换系统托盘（持久化；图标动态创建/移除）
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
                cx.notify();
            })
            .ok();
            cx.update(|cx| match result {
                Ok(r) if r.available => {
                    let text = if I18n::global(cx).is_zh() {
                        format!("🔄 {} v{}", t(cx, "update.available"), r.version)
                    } else {
                        format!("🔄 Update available: v{}", r.version)
                    };
                    notify(cx, NotificationType::Warning, text);
                    // 有便携版下载链接时，后台下载并安装
                    if let Some(url) = r.portable_download_url.clone() {
                        let version = r.version.clone();
                        cx.spawn(async move |cx| {
                            let filename = format!("kafka-manager-{}-portable.zip", version);
                            cx.update(|cx| {
                                notify(cx, NotificationType::Info, t(cx, "update.downloadingInBackground"));
                            })
                            .ok();
                            match crate::updater::download_update(&url, &filename, |_, _| {}).await {
                                Ok(path) => {
                                    cx.update(|cx| {
                                        notify(cx, NotificationType::Success, t(cx, "update.downloadComplete"));
                                    })
                                    .ok();
                                    if let Err(e) = crate::updater::install_portable_update(&path) {
                                        cx.update(|cx| {
                                            let msg = format!("{}: {}", t(cx, "update.installFailed"), e);
                                            notify(cx, NotificationType::Error, msg);
                                        })
                                        .ok();
                                    }
                                }
                                Err(e) => {
                                    cx.update(|cx| {
                                        let msg = format!("{}: {}", t(cx, "update.downloadFailed"), e);
                                        notify(cx, NotificationType::Error, msg);
                                    })
                                    .ok();
                                }
                            }
                        })
                        .detach();
                    }
                }
                Ok(_) => notify(cx, NotificationType::Info, t(cx, "update.checkCompleteNoUpdate")),
                Err(e) => {
                    if e.contains("403") {
                        notify(cx, NotificationType::Error, t(cx, "update.rateLimitExceeded"));
                    } else {
                        notify(cx, NotificationType::Error, format!("{}: {}", t(cx, "update.checkFailed"), e));
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    fn render_row(
        &self,
        label: String,
        description: String,
        control: AnyElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let border = cx.theme().border;
        h_flex()
            .items_center()
            .justify_between()
            .py_3()
            .border_b_1()
            .border_color(border)
            .child(
                v_flex()
                    .gap_1()
                    .child(div().font_semibold().child(label))
                    .child(
                        div()
                            .text_xs()
                            .text_color(self.muted_color(cx))
                            .child(description),
                    ),
            )
            .child(control)
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let dark = self.dark_mode;
        let english = self.english;
        let auto_launch = self.auto_launch;
        let tray_enabled = self.tray_enabled;
        let checking_update = self.checking_update;
        let tree_mode = self.tree_mode;
        let version = if self.version.is_empty() {
            "...".to_string()
        } else {
            self.version.clone()
        };

        let header = v_flex()
            .gap_1()
            .mb_4()
            .child(div().text_xl().font_semibold().child(t(cx, "settings.title")))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(t(cx, "settings.description")),
            );

        let rows = v_flex()
            .child(self.render_row(
                t(cx, "settings.theme"),
                t(cx, "settings.themeDesc"),
                Switch::new("theme-switch")
                    .checked(dark)
                    .label(if dark {
                        t(cx, "settings.darkMode")
                    } else {
                        t(cx, "settings.lightMode")
                    })
                    .on_click(cx.listener(|this, checked: &bool, window, cx| {
                        this.set_dark_mode(*checked, window, cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.language"),
                String::new(),
                Switch::new("lang-switch")
                    .checked(english)
                    .label(if english { "English" } else { "中文" })
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.set_english(*checked, cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.sidebarMode"),
                if tree_mode {
                    t(cx, "settings.treeModeDesc")
                } else {
                    t(cx, "settings.flatModeDesc")
                },
                Switch::new("sidebar-mode-switch")
                    .checked(tree_mode)
                    .label(if tree_mode {
                        t(cx, "settings.treeMode")
                    } else {
                        t(cx, "settings.flatMode")
                    })
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.set_tree_mode(*checked, cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.version"),
                t(cx, "settings.versionDesc"),
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(version)
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.viewLogs"),
                String::new(),
                Button::new("view-logs")
                    .outline()
                    .label(t(cx, "settings.viewLogs"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_logs(window, cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.exportData"),
                String::new(),
                Button::new("export-data")
                    .outline()
                    .label(t(cx, "settings.exportData"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.export_data(cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.importData"),
                String::new(),
                Button::new("import-data")
                    .outline()
                    .label(t(cx, "settings.importData"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.import_data(cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.systemTray"),
                t(cx, "settings.systemTrayDesc"),
                Switch::new("tray-switch")
                    .checked(tray_enabled)
                    .label(if tray_enabled {
                        t(cx, "settings.systemTrayEnabled")
                    } else {
                        t(cx, "settings.systemTrayDisabled")
                    })
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.set_tray(*checked, cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "settings.autoLaunch"),
                t(cx, "settings.autoLaunchDesc"),
                Switch::new("autolaunch-switch")
                    .checked(auto_launch)
                    .label(if auto_launch {
                        t(cx, "settings.autoLaunchEnabled")
                    } else {
                        t(cx, "settings.autoLaunchDisabled")
                    })
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.set_auto_launch(*checked, cx);
                    }))
                    .into_any_element(),
                cx,
            ))
            .child(self.render_row(
                t(cx, "update.checkForUpdates"),
                String::new(),
                Button::new("check-updates")
                    .outline()
                    .label(if checking_update {
                        t(cx, "update.checking")
                    } else {
                        t(cx, "update.checkNow")
                    })
                    .disabled(checking_update)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.check_updates(cx);
                    }))
                    .into_any_element(),
                cx,
            ));

        v_flex()
            .size_full()
            .p_4()
            .child(header)
            .child(div().max_w(px(640.0)).child(rows))
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
