//! 设置页（对齐 SettingsView.vue：系统设置/版本信息/意见反馈/JSON 高亮/导入导出/日志）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, HighlightStyle, IntoElement,
    MouseButton, Render, ScrollHandle, StyledText, Window,
};

use crate::app::{backend, root, AppEvent};
use crate::i18n::{self, Language};
use crate::icons::icon;
use crate::json::{styles_for_current_theme, token_color, tokenize, TokenKind};
use crate::overlay;
use crate::settings;
use crate::theme;
use crate::views::navigator::format_datetime;
use crate::widgets::common::*;
use crate::widgets::text_area::{TextArea, TextAreaEvent};
use crate::widgets::text_input::{TextInput, TextInputEvent};

pub struct SettingsView {
    language_select: Entity<Select>,
    exporting: bool,
    importing: bool,
    // 版本彩蛋
    version_clicks: usize,
    version_click_start: Option<std::time::Instant>,
    // 反馈
    feedback_available: bool,
    feedback_checked: bool,
    feedback_input: Entity<TextArea>,
    feedback_submitting: bool,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
}

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub url: String,
}

impl SettingsView {
    pub fn new(_route: crate::app::Route, cx: &mut Context<Self>) -> Self {
        let language_select = cx.new(|cx| {
            Select::new(
                vec![
                    SelectOption {
                        value: "zh".into(),
                        label: "中文".into(),
                    },
                    SelectOption {
                        value: "en".into(),
                        label: "English".into(),
                    },
                ],
                if i18n::language() == Language::En {
                    "en"
                } else {
                    "zh"
                },
                cx,
            )
        });
        let feedback_input = cx.new(TextArea::new);
        feedback_input.update(cx, |a, _| {
            a.set_placeholder(t("settings.feedbackPlaceholder"))
        });

        let this = Self {
            language_select: language_select.clone(),
            exporting: false,
            importing: false,
            version_clicks: 0,
            version_click_start: None,
            feedback_available: false,
            feedback_checked: false,
            feedback_input,
            feedback_submitting: false,
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        };

        cx.subscribe(&language_select, |_, _, event: &SelectEvent, cx| {
            let lang = if event.value.as_ref() == "en" {
                Language::En
            } else {
                Language::Zh
            };
            root(cx).update(cx, |app, cx| app.set_language(lang, cx));
        })
        .detach();

        this.check_feedback_connection(cx);
        this
    }

    // ==================== 系统设置 ====================

    fn toggle_sidebar_mode(&mut self, cx: &mut Context<Self>) {
        root(cx).update(cx, |app, cx| {
            app.prefs.sidebar_mode = if app.prefs.sidebar_mode == "tree" {
                "list".into()
            } else {
                "tree".into()
            };
            settings::save(&app.prefs);
            let value = app.prefs.sidebar_mode.clone();
            let b = backend(cx);
            cx.spawn(async move |_, _| {
                let _ = b
                    .dispatch(
                        "settings.update",
                        serde_json::json!({"key": "ui.sidebar_mode", "value": value}),
                    )
                    .await;
            })
            .detach();
            cx.notify();
        });
    }

    fn toggle_system_tray(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let b = backend(cx);
        let value = if enabled { "true" } else { "false" };
        cx.spawn(async move |_, _| {
            let _ = b
                .dispatch(
                    "settings.update",
                    serde_json::json!({"key": "ui.system_tray", "value": value}),
                )
                .await;
        })
        .detach();
        root(cx).update(cx, |app, _| {
            app.prefs.update_notify = enabled;
            settings::save(&app.prefs);
        });
        overlay::toast_success(
            cx,
            if enabled {
                "已开启系统托盘"
            } else {
                "已关闭系统托盘"
            },
        );
        cx.notify();
    }

    // ==================== 导入导出 ====================

    fn export_data(&mut self, cx: &mut Context<Self>) {
        if self.exporting {
            return;
        }
        self.exporting = true;
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b.dispatch("settings.export", serde_json::json!({})).await;
            this.update(cx, |this, cx| {
                this.exporting = false;
                match result {
                    Ok(data) => {
                        let counts = (
                            data.get("cluster_groups")
                                .and_then(|x| x.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0),
                            data.get("clusters")
                                .and_then(|x| x.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0),
                            data.get("topics")
                                .and_then(|x| x.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0),
                            data.get("favorites")
                                .and_then(|x| x.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0),
                            data.get("history")
                                .and_then(|x| x.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0),
                        );
                        let date = format_datetime(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as i64)
                                .unwrap_or(0),
                            true,
                        )
                        .replace('/', "-");
                        let filename = format!("kafka-manager-export-{}.json", date);
                        // 打开保存对话框，选择路径后写文件
                        {
                            let picker = cx.new(|cx| {
                                crate::dialogs::file_picker::FilePickerDialog::new(
                                    crate::dialogs::file_picker::PickerMode::Save { filename },
                                    cx,
                                )
                            });
                            cx.subscribe(&picker, {
                                let counts = counts;
                                move |_, _, event: &crate::dialogs::file_picker::PickerEvent, cx| {
                                    let crate::dialogs::file_picker::PickerEvent::Selected(path) = event;
                                    let json =
                                        serde_json::to_string_pretty(&data).unwrap_or_default();
                                    let ok = std::fs::write(path, json).is_ok();
                                    if ok {
                                        overlay::toast_success(
                                            cx,
                                            format!(
                                                "导出成功：{} 个分组，{} 个集群，{} 个 Topic，{} 个收藏分组，{} 条历史。",
                                                counts.0, counts.1, counts.2, counts.3, counts.4
                                            ),
                                        );
                                    } else {
                                        overlay::toast_error(cx, t("settings.exportFailed"));
                                    }
                                }
                            })
                            .detach();
                            overlay::open_modal(cx, picker.into());
                        }
                    }
                    Err(e) => {
                        if e.contains("正在进行中") || e.contains("operation in progress") {
                            overlay::toast_error(
                                cx,
                                "已有其他导入/导出操作正在进行中，请等待完成后再试",
                            );
                        } else {
                            overlay::toast_error(cx, e);
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn import_data(&mut self, cx: &mut Context<Self>) {
        if self.importing {
            return;
        }
        let picker = cx.new(|cx| {
            crate::dialogs::file_picker::FilePickerDialog::new(
                crate::dialogs::file_picker::PickerMode::Open {
                    extension: "json".into(),
                },
                cx,
            )
        });
        cx.subscribe(&picker, |this, _, event: &crate::dialogs::file_picker::PickerEvent, cx| {
            let crate::dialogs::file_picker::PickerEvent::Selected(path) = event;
            this.do_import(path.clone(), cx);
        })
        .detach();
        overlay::open_modal(cx, picker.into());
    }

    fn do_import(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.importing = true;
        cx.notify();
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
        match parsed {
            Err(_) => {
                self.importing = false;
                overlay::toast_error(cx, "Invalid export file format");
                cx.notify();
            }
            Ok(data) => {
                let valid = ["clusters", "favorites", "history", "cluster_groups", "topics"]
                    .iter()
                    .any(|k| data.get(k).is_some());
                if !valid {
                    self.importing = false;
                    overlay::toast_error(cx, "Invalid export file format");
                    cx.notify();
                    return;
                }
                let mut import_data = serde_json::Map::new();
                for k in ["cluster_groups", "clusters", "topics", "favorites", "history"] {
                    import_data.insert(
                        k.to_string(),
                        data.get(k).cloned().unwrap_or(serde_json::json!([])),
                    );
                }
                let b = backend(cx);
                cx.spawn(async move |this, cx| {
                    let result = b
                        .dispatch(
                            "settings.import",
                            serde_json::json!({
                                "data": serde_json::Value::Object(import_data),
                                "strategy": "skip",
                            }),
                        )
                        .await;
                    // 锁定 2 秒防重复
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(2))
                        .await;
                    this.update(cx, |this, cx| {
                        this.importing = false;
                        match result {
                            Ok(_) => {
                                overlay::toast_success(
                                    cx,
                                    "导入已在后台启动，完成后会自动释放",
                                );
                                root(cx).update(cx, |app, cx| {
                                    app.reload_clusters(cx);
                                    app.publish(AppEvent::FavoritesChanged, cx);
                                });
                            }
                            Err(e) => {
                                if e.contains("正在进行中")
                                    || e.contains("operation in progress")
                                {
                                    overlay::toast_error(
                                        cx,
                                        "已有其他导入/导出操作正在进行中，请等待完成后再试",
                                    );
                                } else {
                                    overlay::toast_error(cx, e);
                                }
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
        }
    }

    // ==================== 版本/更新 ====================

    fn click_version(&mut self, cx: &mut Context<Self>) {
        let now = std::time::Instant::now();
        match self.version_click_start {
            Some(start) if now.duration_since(start).as_millis() < 2000 => {
                self.version_clicks += 1;
            }
            _ => {
                self.version_clicks = 1;
                self.version_click_start = Some(now);
            }
        }
        if self.version_clicks >= 5 {
            self.version_clicks = 0;
            let already = root(cx).read(cx).prefs.dev_unlocked;
            if !already {
                overlay::toast_success(cx, "已开启开发者功能");
                root(cx).update(cx, |app, _| {
                    app.prefs.dev_unlocked = true;
                    settings::save(&app.prefs);
                });
            }
        }
        cx.notify();
    }

    fn check_updates(&mut self, manual: bool, cx: &mut Context<Self>) {
        root(cx).update(cx, |app, cx| app.check_for_updates(manual, cx));
    }

    fn open_update_modal(&self, cx: &mut Context<Self>) {
        let Some(info) = root(cx).read(cx).update_info.clone() else {
            return;
        };
        let view = cx.new(|cx| UpdateDialog::new(info, cx));
        overlay::open_modal(cx, view.into());
    }

    // ==================== 反馈 ====================

    fn check_feedback_connection(&self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            // 后端可能未就绪，稍后重试一次
            let mut connected = false;
            for _ in 0..2 {
                if let Ok(v) = b
                    .dispatch("telemetry.check_connection", serde_json::json!({}))
                    .await
                {
                    connected = v
                        .get("connected")
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false);
                    break;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(2))
                    .await;
            }
            this.update(cx, |this, cx| {
                this.feedback_checked = true;
                this.feedback_available = connected;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn submit_feedback(&mut self, cx: &mut Context<Self>) {
        let content = self.feedback_input.read(cx).text().trim().to_string();
        if content.is_empty() || self.feedback_submitting {
            return;
        }
        if content.chars().count() > 2000 {
            overlay::toast_error(cx, "反馈内容不能超过 2000 字");
            return;
        }
        self.feedback_submitting = true;
        cx.notify();
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b
                .dispatch(
                    "telemetry.submit_feedback",
                    serde_json::json!({"feedback_content": content}),
                )
                .await;
            this.update(cx, |this, cx| {
                this.feedback_submitting = false;
                match result {
                    Ok(v) => {
                        let success = v
                            .get("success")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false);
                        if success {
                            overlay::toast_success(cx, "反馈提交成功，感谢您的意见！");
                            this.feedback_input.update(cx, |a, cx| a.set_text("", cx));
                        } else {
                            let reason = v
                                .get("reason")
                                .and_then(|x| x.as_str())
                                .unwrap_or("反馈提交失败");
                            overlay::toast_error(cx, reason);
                        }
                    }
                    Err(_) => overlay::toast_error(cx, "反馈提交失败"),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ==================== 渲染 ====================

    fn render_row(
        &self,
        label: String,
        control: impl IntoElement,
    ) -> gpui::Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .py(px(8.))
            .px(px(12.))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme::text_primary())
                    .child(label),
            )
            .child(control)
    }

    fn render_card_header(
        &self,
        icon_name: &'static str,
        title: String,
        subtitle: String,
    ) -> gpui::Div {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .py(px(10.))
            .border_b_1()
            .border_color(theme::border_base_200())
            .child(
                div()
                    .size(px(32.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme::badge_primary_bg())
                    .child(
                        icon(icon_name)
                            .size(px(16.))
                            .text_color(theme::badge_primary_text()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::text_secondary())
                            .child(subtitle),
                    ),
            )
    }
}

pub fn version_newer(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim_start_matches('v')
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (c, l) = (parse(current), parse(latest));
    for i in 0..3 {
        let (cv, lv) = (c.get(i).copied().unwrap_or(0), l.get(i).copied().unwrap_or(0));
        if lv != cv {
            return lv > cv;
        }
    }
    false
}

use crate::i18n::t;

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = root(cx);
        let is_dark = theme::is_dark();
        let tree_mode = app.read(cx).prefs.sidebar_mode == "tree";
        let tray_enabled = app.read(cx).prefs.update_notify;
        let app_version = app.read(cx).app_version.clone();
        let dev_unlocked = app.read(cx).prefs.dev_unlocked;
        let has_update = app.read(cx).has_update;
        let lang = i18n::language();

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            // 页头
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .py(px(10.))
                    .flex_none()
                    .child(
                        icon_btn("settings-back", "arrow-left", BtnSize::Xs)
                            .when(!app.read(cx).can_go_back(), |b| b.opacity(0.5))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                root(cx).update(cx, |app, cx| app.go_back(cx));
                            }),
                    )
                    .child(
                        icon("cog")
                            .size(px(20.))
                            .text_color(theme::badge_primary_text()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme::text_primary())
                                    .child(t("settings.title")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::text_secondary())
                                    .child(t("settings.description")),
                            ),
                    ),
            )
            // 内容
            .child(
                div().id("views_settings_rs_2")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .px(px(12.))
                    .pb(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    // 卡片一：系统设置
                    .child(
                        card()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .child(self.render_card_header(
                                "cog",
                                t("settings.systemSettings"),
                                String::new(),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(self.render_row(
                                        format!(
                                            "{} · {}",
                                            t("settings.theme"),
                                            if is_dark {
                                                t("settings.darkMode")
                                            } else {
                                                t("settings.lightMode")
                                            }
                                        ),
                                        toggle("set-theme", is_dark).on_mouse_down(
                                            MouseButton::Left,
                                            |_, _, cx| {
                                                root(cx).update(cx, |app, cx| {
                                                    app.toggle_theme(cx)
                                                });
                                            },
                                        ),
                                    ))
                                    .child(div().h(px(1.)).bg(theme::border_base_200()))
                                    .child(self.render_row(
                                        format!(
                                            "{} · {}",
                                            t("settings.language"),
                                            if lang == Language::Zh { "中文" } else { "English" }
                                        ),
                                        div()
                                            .w(px(140.))
                                            .child(self.language_select.clone()),
                                    ))
                                    .child(div().h(px(1.)).bg(theme::border_base_200()))
                                    .child(self.render_row(
                                        format!(
                                            "{} · {}",
                                            t("settings.sidebarMode"),
                                            if tree_mode {
                                                t("settings.treeMode")
                                            } else {
                                                t("settings.listMode")
                                            }
                                        ),
                                        toggle("set-sidebar-mode", tree_mode).on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.toggle_sidebar_mode(cx)
                                            }),
                                        ),
                                    ))
                                    .child(div().h(px(1.)).bg(theme::border_base_200()))
                                    .child(self.render_row(
                                        t("settings.systemTray"),
                                        toggle("set-tray", tray_enabled).on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.toggle_system_tray(!tray_enabled, cx)
                                            }),
                                        ),
                                    ))
                                    .child(div().h(px(1.)).bg(theme::border_base_200()))
                                    .child(self.render_row(
                                        t("settings.importExport"),
                                        div()
                                            .flex()
                                            .gap(px(8.))
                                            .child(
                                                btn("export-data", BtnKind::Primary, BtnSize::Sm)
                                                    .when(self.exporting, |b| b.opacity(0.6))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.export_data(cx)
                                                        }),
                                                    )
                                                    .child(if self.exporting {
                                                        spinner(12.)
                                                    } else {
                                                        icon("download")
                                                            .size(px(12.))
                                                            .into_any_element()
                                                    })
                                                    .child(if self.exporting {
                                                        t("settings.exporting")
                                                    } else {
                                                        t("settings.exportData")
                                                    }),
                                            )
                                            .child(
                                                btn("import-data", BtnKind::Outline, BtnSize::Sm)
                                                    .when(self.importing, |b| b.opacity(0.6))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.import_data(cx)
                                                        }),
                                                    )
                                                    .child(if self.importing {
                                                        spinner(12.)
                                                    } else {
                                                        icon("upload")
                                                            .size(px(12.))
                                                            .into_any_element()
                                                    })
                                                    .child(if self.importing {
                                                        t("settings.importing")
                                                    } else {
                                                        t("settings.importData")
                                                    }),
                                            ),
                                    )),
                            ),
                    )
                    // 卡片二：版本信息
                    .child(
                        card()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .child(self.render_card_header(
                                "info",
                                t("settings.version"),
                                t("settings.versionDesc"),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        self.render_row(
                                            t("settings.currentVersion"),
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .id("version-badge")
                                                        .cursor_pointer()
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(|this, _, _, cx| {
                                                                this.click_version(cx)
                                                            }),
                                                        )
                                                        .child(badge(
                                                            format!("v{}", app_version),
                                                            BadgeKind::Primary,
                                                        )),
                                                )
                                                .when(has_update, |d| {
                                                    d.child(
                                                        btn("has-update", BtnKind::WarningOutline, BtnSize::Xs)
                                                            .child(
                                                                app.read(cx)
                                                                    .update_info
                                                                    .as_ref()
                                                                    .map(|i| {
                                                                        format!(
                                                                            "{} {}",
                                                                            t("update.newVersion"),
                                                                            i.version
                                                                        )
                                                                    })
                                                                    .unwrap_or_default(),
                                                            )
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener(|this, _, _, cx| {
                                                                    this.open_update_modal(cx)
                                                                }),
                                                            ),
                                                    )
                                                }),
                                        ),
                                    )
                                    .child(div().h(px(1.)).bg(theme::border_base_200()))
                                    .child(self.render_row(
                                        t("settings.author"),
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme::text_primary())
                                            .child("朱占全"),
                                    ))
                                    .child(div().h(px(1.)).bg(theme::border_base_200()))
                                    .child(self.render_row(
                                        t("settings.checkUpdate"),
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .when(dev_unlocked, |d| {
                                                d.child(
                                                    btn("view-logs", BtnKind::Outline, BtnSize::Sm)
                                                        .child(t("settings.viewLogs"))
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            |_, _, cx| {
                                                                let view = cx.new(LogViewerDialog::new);
                                                                overlay::open_modal(cx, view.into());
                                                            },
                                                        ),
                                                )
                                            })
                                            .child(
                                                btn("check-update", BtnKind::Primary, BtnSize::Sm)
                                                    .when(app.read(cx).checking_update, |b| {
                                                        b.opacity(0.6)
                                                    })
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(|this, _, _, cx| {
                                                            this.check_updates(true, cx)
                                                        }),
                                                    )
                                                    .child(if app.read(cx).checking_update {
                                                        spinner(12.)
                                                    } else {
                                                        icon("refresh")
                                                            .size(px(12.))
                                                            .into_any_element()
                                                    })
                                                    .child(if app.read(cx).checking_update {
                                                        t("update.checking")
                                                    } else {
                                                        t("update.checkNow")
                                                    }),
                                            ),
                                    )),
                            ),
                    )
                    // 卡片三：意见反馈（条件渲染）
                    .when(self.feedback_checked && self.feedback_available, |d| {
                        d.child(
                            card()
                                .flex_none()
                                .flex()
                                .flex_col()
                                .child(self.render_card_header(
                                    "chat",
                                    t("settings.feedback"),
                                    t("settings.feedbackDesc"),
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.))
                                        .p(px(12.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(4.))
                                                .child(
                                                    icon("book")
                                                        .size(px(13.))
                                                        .text_color(theme::badge_primary_text()),
                                                )
                                                .child(
                                                    div()
                                                        .id("docs-link")
                                                        .text_size(px(12.))
                                                        .text_color(theme::badge_primary_text())
                                                        .cursor_pointer()
                                                        .hover(|s| s.opacity(0.7))
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            |_, _, _| {
                                                                let _ = std::process::Command::new(
                                                                    "xdg-open",
                                                                )
                                                                .arg("http://wewiki.ky-tech.com.cn/pages/viewpage.action?pageId=145989789")
                                                                .spawn();
                                                            },
                                                        )
                                                        .child(t("settings.docs")),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .h(px(96.))
                                                .rounded(px(6.))
                                                .bg(theme::input_bg())
                                                .border_1()
                                                .border_color(theme::base_content_alpha(0.15))
                                                .text_size(px(12.))
                                                .p(px(6.))
                                                .child(self.feedback_input.clone()),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(theme::text_secondary())
                                                        .child(format!(
                                                            "{} / 2000",
                                                            self.feedback_input
                                                                .read(cx)
                                                                .text()
                                                                .chars()
                                                                .count()
                                                        )),
                                                )
                                                .child(
                                                    btn("submit-feedback", BtnKind::Primary, BtnSize::Sm)
                                                        .when(self.feedback_submitting, |b| {
                                                            b.opacity(0.6)
                                                        })
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(|this, _, _, cx| {
                                                                this.submit_feedback(cx)
                                                            }),
                                                        )
                                                        .child(if self.feedback_submitting {
                                                            spinner(12.)
                                                        } else {
                                                            icon("send")
                                                                .size(px(12.))
                                                                .into_any_element()
                                                        })
                                                        .child(if self.feedback_submitting {
                                                            t("settings.submitting")
                                                        } else {
                                                            t("settings.submitFeedback")
                                                        }),
                                                ),
                                        ),
                                ),
                        )
                    })
                    // 卡片四：JSON 高亮
                    .child(render_json_highlight_card(cx)),
            )
    }
}

// ==================== JSON 高亮卡片 ====================

const SAMPLE_JSON: &str = r#"{"id":1,"name":"example","active":true,"count":42,"data":null,"tags":["json","highlight"]}"#;

fn render_json_highlight_card(cx: &mut Context<SettingsView>) -> impl IntoElement {
    let app = root(cx);
    let templates = app.read(cx).json_templates.clone();
    let current = app.read(cx).current_template.clone();

    let mut card = card().flex_none().flex().flex_col();
    card = card.child(
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .py(px(10.))
            .border_b_1()
            .border_color(theme::border_base_200())
            .child(
                div()
                    .size(px(32.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme::badge_primary_bg())
                    .child(
                        icon("code")
                            .size(px(16.))
                            .text_color(theme::badge_primary_text()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("settings.jsonHighlight")),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::text_secondary())
                            .child(t("settings.jsonHighlightDesc")),
                    ),
            ),
    );

    // 模板选择
    let options: Vec<SelectOption> = templates
        .iter()
        .map(|tpl| SelectOption {
            value: tpl.name.clone().into(),
            label: format!("{} - {}", tpl.name, tpl.description).into(),
        })
        .collect();

    let template_select = cx.new(|cx| Select::new(options, current.clone(), cx));
    cx.subscribe(&template_select, |_, _, event: &SelectEvent, cx| {
        let name = event.value.to_string();
        let b = backend(cx);
        cx.spawn(async move |_, cx| {
            let _ = b
                .dispatch(
                    "json_highlight.set_current",
                    serde_json::json!({"name": name}),
                )
                .await;
            cx.update(|cx| {
                root(cx).update(cx, |app, cx| {
                    app.current_template = name;
                    app.publish(AppEvent::JsonHighlightChanged, cx);
                    cx.notify();
                });
                overlay::toast_success(cx, t("settings.templateChanged"));
})
        })
        .detach();
    })
    .detach();

    // 预览
    let current_tpl = templates
        .iter()
        .find(|tpl| tpl.name == current)
        .map(|tpl| tpl.style.clone())
        .unwrap_or_else(crate::json::default_template);
    let styles = styles_for_current_theme(&current_tpl);
    let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
    let mut offset = 0;
    for token in tokenize(SAMPLE_JSON) {
        let start = offset;
        let end = offset + token.text.len();
        offset = end;
        if token.kind == TokenKind::Whitespace {
            continue;
        }
        let (color, weight) = token_color(styles, token.kind);
        highlights.push((
            start..end,
            HighlightStyle {
                color: Some(color),
                font_weight: Some(weight),
                ..Default::default()
            },
        ));
    }

    let is_builtin = templates
        .iter()
        .find(|tpl| tpl.name == current)
        .map(|tpl| tpl.is_builtin)
        .unwrap_or(true);
    let current_tpl_for_edit = templates.iter().find(|tpl| tpl.name == current).cloned();

    card.child(
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(12.))
            .child(field(
                t("settings.selectTemplate"),
                template_select.into_any_element(),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme::text_secondary())
                            .child(t("settings.preview")),
                    )
                    .child(
                        div()
                            .rounded(px(6.))
                            .bg(theme::base_content_alpha(0.06))
                            .p(px(10.))
                            .font_family("monospace")
                            .text_size(px(11.))
                            .child(StyledText::new(SAMPLE_JSON).with_highlights(highlights)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .when(is_builtin, |d| {
                        let tpl_builtin = current_tpl_for_edit.clone();
                        d.child(
                            btn("add-custom-template", BtnKind::Outline, BtnSize::Sm)
                                .child(icon("plus").size(px(12.)).into_any_element())
                                .child(t("settings.addCustomTemplate"))
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    let Some(tpl) = tpl_builtin.clone() else {
                                        return;
                                    };
                                    let view = cx.new(|cx| {
                                            TemplateEditorDialog::new(
                                                TemplateEditorMode::Create {
                                                    name: format!("{}_custom", tpl.name),
                                                    description: format!("{} (Custom)", tpl.description),
                                                    style_json: serde_json::to_string_pretty(
                                                        &serde_json::json!({
                                                            "light": tpl.style.light,
                                                            "dark": tpl.style.dark,
                                                        }),
                                                    )
                                                    .unwrap_or_default(),
                                                },
                                                cx,
                                            )
                                        });
                                    overlay::open_modal(cx, view.into());
                                }),
                        )
                    })
                    .when(!is_builtin, |d| {
                        let tpl_edit = current_tpl_for_edit.clone();
                        let tpl_delete = current_tpl_for_edit.clone();
                        d.child(
                            btn("edit-template", BtnKind::Outline, BtnSize::Sm)
                                .child(icon("pencil").size(px(12.)).into_any_element())
                                .child(t("common.edit"))
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    let Some(tpl) = tpl_edit.clone() else { return };
                                    let view = cx.new(|cx| {
                                            TemplateEditorDialog::new(
                                                TemplateEditorMode::Edit(tpl),
                                                cx,
                                            )
                                        });
                                    overlay::open_modal(cx, view.into());
                                }),
                        )
                        .child(
                            btn("delete-template", BtnKind::ErrorOutline, BtnSize::Sm)
                                .child(icon("trash").size(px(12.)).into_any_element())
                                .child(t("settings.deleteTemplate"))
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    let Some(tpl) = tpl_delete.clone() else {
                                        return;
                                    };
                                    overlay::confirm(
                                        cx,
                                        t("common.confirm"),
                                        "确定要删除这个自定义模板吗？",
                                        true,
                                        move |cx| {
                                            let b = backend(cx);
                                            let (id, name) = (tpl.id, tpl.name.clone());
                                            cx.spawn(async move |cx| {
                                                let _ = b
                                                    .dispatch(
                                                        "json_highlight.delete",
                                                        serde_json::json!({"template_id": id}),
                                                    )
                                                    .await;
                                                cx.update(|cx| {
                                                    root(cx).update(cx, |app, cx| {
                                                        if app.current_template == name {
                                                            app.current_template =
                                                                "default".into();
                                                            let b2 = backend(cx);
                                                            cx.spawn(async move |_, _| {
                                                                let _ = b2.dispatch(
                                                                    "json_highlight.set_current",
                                                                    serde_json::json!({
                                                                        "name": "default"
                                                                    }),
                                                                )
                                                                .await;
                                                            })
                                                            .detach();
                                                        }
                                                        app.load_json_templates(cx);
                                                        app.publish(
                                                            AppEvent::JsonHighlightChanged,
                                                            cx,
                                                        );
                                                    });
})
                                            })
                                            .detach();
                                        },
                                    );
                                }),
                        )
                    }),
            ),
    )
}

// ==================== 模板编辑对话框 ====================

pub enum TemplateEditorMode {
    Create {
        name: String,
        description: String,
        style_json: String,
    },
    Edit(crate::json::HighlightTemplate),
}

pub struct TemplateEditorDialog {
    mode: TemplateEditorMode,
    name: Entity<TextInput>,
    description: Entity<TextInput>,
    style_json: Entity<TextArea>,
    saving: bool,
    focus_handle: FocusHandle,
}

impl TemplateEditorDialog {
    pub fn new(mode: TemplateEditorMode, cx: &mut Context<Self>) -> Self {
        let name = cx.new(TextInput::new);
        let description = cx.new(TextInput::new);
        let style_json = cx.new(TextArea::new);
        match &mode {
            TemplateEditorMode::Create {
                name: n,
                description: d,
                style_json: s,
            } => {
                name.update(cx, |i, cx| i.set_text(n.clone(), cx));
                description.update(cx, |i, cx| i.set_text(d.clone(), cx));
                style_json.update(cx, |a, cx| a.set_text(s.clone(), cx));
            }
            TemplateEditorMode::Edit(tpl) => {
                name.update(cx, |i, cx| {
                    i.set_text(tpl.name.clone(), cx);
                    i.set_disabled(true, cx);
                });
                description.update(cx, |i, cx| i.set_text(tpl.description.clone(), cx));
                let s = serde_json::to_string_pretty(&serde_json::json!({
                    "light": tpl.style.light,
                    "dark": tpl.style.dark,
                }))
                .unwrap_or_default();
                style_json.update(cx, |a, cx| a.set_text(s, cx));
            }
        }
        for input in [&name, &description] {
            cx.subscribe(input, |_, _, event: &TextInputEvent, cx| {
                if matches!(event, TextInputEvent::EscapePressed) {
                    overlay::close_modal(cx);
                }
            })
            .detach();
        }
        cx.subscribe(&style_json, |_, _, event: &TextAreaEvent, cx| {
            if matches!(event, TextAreaEvent::EscapePressed) {
                overlay::close_modal(cx);
            }
        })
        .detach();
        Self {
            mode,
            name,
            description,
            style_json,
            saving: false,
            focus_handle: cx.focus_handle(),
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).text().trim().to_string();
        let description = self.description.read(cx).text().trim().to_string();
        let style_json = self.style_json.read(cx).text().trim().to_string();
        if name.is_empty() || style_json.is_empty() {
            overlay::toast_error(cx, "模板名称和样式配置不能为空");
            return;
        }
        if crate::json::parse_template_style(&style_json).is_none() {
            overlay::toast_error(
                cx,
                "模板格式无效：必须包含 light 和 dark 主题的所有字段 (key, string, number, boolean, null, bracket, colon, comma)，且每个字段必须有 color 属性",
            );
            return;
        }
        if self.saving {
            return;
        }
        self.saving = true;
        cx.notify();
        let b = backend(cx);
        let edit_id = match &self.mode {
            TemplateEditorMode::Edit(tpl) => Some(tpl.id),
            _ => None,
        };
        cx.spawn(async move |this, cx| {
            let result = if let Some(id) = edit_id {
                b.dispatch(
                    "json_highlight.update",
                    serde_json::json!({
                        "template_id": id,
                        "description": description,
                        "style_json": style_json,
                    }),
                )
                .await
            } else {
                b.dispatch(
                    "json_highlight.create",
                    serde_json::json!({
                        "name": name,
                        "description": description,
                        "style_json": style_json,
                    }),
                )
                .await
            };
            this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(_) => {
                        overlay::close_modal(cx);
                        root(cx).update(cx, |app, cx| {
                            app.load_json_templates(cx);
                            app.publish(AppEvent::JsonHighlightChanged, cx);
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
}

impl Focusable for TemplateEditorDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TemplateEditorDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_edit = matches!(self.mode, TemplateEditorMode::Edit(_));
        div()
            .w(px(560.))
            .max_h(px(600.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(if is_edit {
                                t("common.edit")
                            } else {
                                t("settings.addCustomTemplate")
                            }),
                    )
                    .child(
                        icon_btn("close-template-editor", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(field(
                t("settings.templateName"),
                input_frame(self.name.clone()).into_any_element(),
            ))
            .child(field(
                t("settings.templateDesc"),
                input_frame(self.description.clone()).into_any_element(),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme::text_secondary())
                            .child(t("settings.templateStyle")),
                    )
                    .child(
                        div()
                            .h(px(220.))
                            .rounded(px(6.))
                            .bg(theme::input_bg())
                            .border_1()
                            .border_color(theme::base_content_alpha(0.15))
                            .font_family("monospace")
                            .text_size(px(11.))
                            .p(px(6.))
                            .child(self.style_json.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        btn("tpl-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("tpl-save", BtnKind::Primary, BtnSize::Sm)
                            .when(self.saving, |b| b.opacity(0.6))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.save(cx)),
                            )
                            .child(if self.saving {
                                spinner(13.)
                            } else {
                                icon("check").size(px(13.)).into_any_element()
                            })
                            .child(t("settings.saveTemplate")),
                    ),
            )
    }
}

// ==================== 日志查看对话框 ====================

pub struct LogViewerDialog {
    lines: Vec<String>,
    loading: bool,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
}

impl LogViewerDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let this = Self {
            lines: vec![],
            loading: true,
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        };
        this.load(cx);
        this
    }

    fn load(&self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let result = b.dispatch("app.logs", serde_json::json!({})).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(v) = result {
                    let logs = v.get("logs").and_then(|x| x.as_str()).unwrap_or("");
                    this.lines = logs.lines().map(|s| s.to_string()).collect();
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn clear(&self, cx: &mut Context<Self>) {
        let b = backend(cx);
        cx.spawn(async move |this, cx| {
            let _ = b.dispatch("app.logs.clear", serde_json::json!({})).await;
            this.update(cx, |this, cx| {
                this.lines = vec![];
                overlay::toast_success(cx, "日志已清除");
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn copy(&self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(self.lines.join("\n")));
        overlay::toast_success(cx, "已复制到剪贴板");
    }
}

impl Focusable for LogViewerDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LogViewerDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(800.))
            .h(px(560.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(16.))
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("settings.appLogs")),
                    )
                    .child(
                        icon_btn("log-bottom", "chevron-down", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, _| {
                                this.scroll.scroll_to_bottom();
                            }),
                        ),
                    )
                    .child(
                        icon_btn("log-refresh", "refresh", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.load(cx);
                                overlay::toast_success(cx, "刷新成功");
                            }),
                        ),
                    )
                    .child(
                        icon_btn("log-copy", "clipboard", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.copy(cx)),
                        ),
                    )
                    .child(
                        icon_btn("log-clear", "trash", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.clear(cx)),
                        ),
                    )
                    .child(
                        icon_btn("close-logs", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(
                div().id("views_settings_rs_4")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .rounded(px(8.))
                    .bg(theme::base_content_alpha(0.06))
                    .p(px(10.))
                    .font_family("monospace")
                    .text_size(px(11.))
                    .text_color(theme::text_primary())
                    .when(self.loading, |d| {
                        d.child(loading_block(t("common.loading")))
                    })
                    .when(!self.loading && self.lines.is_empty(), |d| {
                        d.child(empty_block("document", "暂无日志", ""))
                    })
                    .when(!self.lines.is_empty(), |d| {
                        d.child(self.lines.join("\n"))
                    }),
            )
    }
}

// ==================== 更新对话框 ====================

pub struct UpdateDialog {
    info: UpdateInfo,
    focus_handle: FocusHandle,
}

impl UpdateDialog {
    pub fn new(info: UpdateInfo, cx: &mut Context<Self>) -> Self {
        Self {
            info,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for UpdateDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for UpdateDialog {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current = root(_cx).read(_cx).app_version.clone();
        div()
            .w(px(480.))
            .max_h(px(520.))
            .flex()
            .flex_col()
            .bg(theme::modal_bg())
            .border_1()
            .border_color(theme::glass_border())
            .rounded(px(10.))
            .shadow_lg()
            .p(px(20.))
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(t("update.available")),
                    )
                    .child(
                        icon_btn("close-update", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(10.))
                    .py(px(8.))
                    .child(
                        div()
                            .font_family("monospace")
                            .text_size(px(13.))
                            .text_color(theme::text_secondary())
                            .child(format!("v{}", current)),
                    )
                    .child(
                        icon("arrow-left")
                            .size(px(14.))
                            .text_color(theme::text_secondary()),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme::badge_primary_text())
                            .child(format!("v{}", self.info.version)),
                    ),
            )
            .when(!self.info.notes.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme::text_secondary())
                                .child(t("update.releaseNotes")),
                        )
                        .child(
                            div().id("views_settings_rs_5")
                                .max_h(px(200.))
                                .overflow_y_scroll()
                                .rounded(px(6.))
                                .bg(theme::base_content_alpha(0.05))
                                .p(px(10.))
                                .text_size(px(11.))
                                .text_color(theme::text_primary())
                                .child(self.info.notes.clone()),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .pt(px(4.))
                    .child(
                        btn("update-cancel", BtnKind::Ghost, BtnSize::Sm)
                            .flex_1()
                            .child(t("common.cancel"))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                overlay::close_modal(cx);
                            }),
                    )
                    .child(
                        btn("update-download", BtnKind::Primary, BtnSize::Sm)
                            .flex_1()
                            .child(t("update.downloadAndInstall"))
                            .on_mouse_down(MouseButton::Left, {
                                let url = self.info.url.clone();
                                move |_, _, cx| {
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg(&url)
                                        .spawn();
                                    overlay::close_modal(cx);
                                }
                            }),
                    ),
            )
    }
}
