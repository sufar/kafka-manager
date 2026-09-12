//! 简易文件选择/保存对话框（跨平台，不依赖系统原生对话框）

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, Render,
    ScrollHandle, Window,
};

use crate::i18n::t;
use crate::icons::icon;
use crate::overlay;
use crate::theme;
use crate::widgets::common::*;
use crate::widgets::text_input::{TextInput, TextInputEvent};

#[derive(Clone, PartialEq)]
pub enum PickerMode {
    Save { filename: String },
    Open { extension: String },
}

pub enum PickerEvent {
    Selected(std::path::PathBuf),
}

impl gpui::EventEmitter<PickerEvent> for FilePickerDialog {}

pub struct FilePickerDialog {
    mode: PickerMode,
    current_dir: std::path::PathBuf,
    entries: Vec<(String, bool)>, // (name, is_dir)
    name_input: Entity<TextInput>,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
}

impl FilePickerDialog {
    pub fn new(mode: PickerMode, cx: &mut Context<Self>) -> Self {
        let current_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let name_input = cx.new(TextInput::new);
        if let PickerMode::Save { filename } = &mode {
            name_input.update(cx, |i, cx| i.set_text(filename.clone(), cx));
        } else {
            name_input.update(cx, |i, _| i.set_placeholder("文件名"));
        }
        cx.subscribe(&name_input, |this, _, event: &TextInputEvent, cx| {
            match event {
                TextInputEvent::EscapePressed => overlay::close_modal(cx),
                TextInputEvent::EnterPressed => this.confirm(cx),
                _ => {}
            }
        })
        .detach();
        let mut this = Self {
            mode,
            current_dir,
            entries: vec![],
            name_input,
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        };
        this.read_dir(cx);
        this
    }

    fn read_dir(&mut self, cx: &mut Context<Self>) {
        let mut dirs_list: Vec<(String, bool)> = vec![];
        let mut files_list: Vec<(String, bool)> = vec![];
        if let Ok(rd) = std::fs::read_dir(&self.current_dir) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    dirs_list.push((name, true));
                } else {
                    // Open 模式按扩展名过滤
                    if let PickerMode::Open { extension } = &self.mode {
                        if !name.to_lowercase().ends_with(&extension.to_lowercase()) {
                            continue;
                        }
                    }
                    files_list.push((name, false));
                }
            }
        }
        dirs_list.sort();
        files_list.sort();
        self.entries = dirs_list.into_iter().chain(files_list).collect();
        cx.notify();
    }

    fn enter_dir(&mut self, name: &str, cx: &mut Context<Self>) {
        self.current_dir = self.current_dir.join(name);
        self.read_dir(cx);
    }

    fn go_up(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.current_dir = parent;
            self.read_dir(cx);
        }
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).text().trim().to_string();
        match &self.mode {
            PickerMode::Save { .. } => {
                if name.is_empty() {
                    return;
                }
                let path = self.current_dir.join(&name);
                overlay::close_modal(cx);
                cx.emit(PickerEvent::Selected(path));
            }
            PickerMode::Open { extension } => {
                let name = if name.is_empty() {
                    // 未输入时尝试用当前选中文件（通过点击文件已填入）
                    return;
                } else {
                    name
                };
                let path = if std::path::Path::new(&name).is_absolute() {
                    std::path::PathBuf::from(&name)
                } else {
                    self.current_dir.join(&name)
                };
                if !path.exists() || !name.to_lowercase().ends_with(&extension.to_lowercase()) && path.extension().map(|e| e.to_string_lossy().to_lowercase()) != Some(extension.to_lowercase()) {
                    if !path.exists() {
                        overlay::toast_error(cx, "文件不存在");
                        return;
                    }
                }
                overlay::close_modal(cx);
                cx.emit(PickerEvent::Selected(path));
            }
        }
    }
}

impl Focusable for FilePickerDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FilePickerDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_save = matches!(self.mode, PickerMode::Save { .. });
        let title = if is_save {
            t("common.save")
        } else {
            t("common.open")
        };

        div()
            .w(px(480.))
            .h(px(440.))
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
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme::text_primary())
                            .child(title),
                    )
                    .child(div().flex_1())
                    .child(
                        icon_btn("close-picker", "x-mark", BtnSize::Sm).on_mouse_down(
                            MouseButton::Left,
                            |_, _, cx| overlay::close_modal(cx),
                        ),
                    ),
            )
            // 路径栏
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        icon_btn("picker-up", "arrow-up", BtnSize::Xs).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.go_up(cx)),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_size(px(11.))
                            .font_family("monospace")
                            .text_color(theme::text_secondary())
                            .child(self.current_dir.to_string_lossy().to_string()),
                    ),
            )
            // 文件列表
            .child(
                div().id("dialogs_file_picker_rs_2")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme::border_base_200())
                    .p(px(4.))
                    .when(self.entries.is_empty(), |d| {
                        d.child(empty_block("folder", t("common.noData"), ""))
                    })
                    .children(self.entries.iter().enumerate().map(|(ix, (name, is_dir))| {
                        let name = name.clone();
                        let name_c = name.clone();
                        let is_dir = *is_dir;
                        div()
                            .id(("picker-entry", ix))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .h(px(26.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme::context_menu_item_hover()))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    if is_dir {
                                        if event.click_count == 2 {
                                            this.enter_dir(&name_c, cx);
                                        }
                                    } else {
                                        this.name_input.update(cx, |i, cx| {
                                            i.set_text(name_c.clone(), cx)
                                        });
                                        if event.click_count == 2 {
                                            this.confirm(cx);
                                        }
                                    }
                                }),
                            )
                            .child(
                                icon(if is_dir { "folder" } else { "document" })
                                    .size(px(13.))
                                    .text_color(if is_dir {
                                        theme::warning()
                                    } else {
                                        theme::text_secondary()
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::text_primary())
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(name),
                            )
                    })),
            )
            // 文件名 + 按钮
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().child(input_frame(self.name_input.clone())))
                    .child(
                        btn("picker-confirm", BtnKind::Primary, BtnSize::Sm)
                            .child(t("common.confirm"))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.confirm(cx)),
                            ),
                    ),
            )
    }
}
