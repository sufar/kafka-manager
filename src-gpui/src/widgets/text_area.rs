//! 多行文本编辑器（无软换行，纵/横向滚动）：
//! 用于 JSON Value 编辑、消息详情查看（read_only）、设置页大文本框等。
//! 支持光标移动/选区/拖选/双击选词/滚轮/只读模式/IME。

use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    actions, div, fill, point, px, relative, size, App, Bounds, ClipboardItem, ContentMask,
    Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    EventEmitter, FocusHandle, Focusable, GlobalElementId, Hsla, IntoElement, KeyBinding, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ScrollWheelEvent, ShapedLine, SharedString, Style, TextRun, UTF16Selection,
    Window,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::theme;

actions!(
    km_text_area,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        Home,
        End,
        DocHome,
        DocEnd,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectToLineStart,
        SelectToLineEnd,
        SelectAll,
        Enter,
        Escape,
        Tab,
        Paste,
        Cut,
        Copy,
    ]
);

pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("KmTextArea")),
        KeyBinding::new("delete", Delete, Some("KmTextArea")),
        KeyBinding::new("left", Left, Some("KmTextArea")),
        KeyBinding::new("right", Right, Some("KmTextArea")),
        KeyBinding::new("up", Up, Some("KmTextArea")),
        KeyBinding::new("down", Down, Some("KmTextArea")),
        KeyBinding::new("home", Home, Some("KmTextArea")),
        KeyBinding::new("end", End, Some("KmTextArea")),
        KeyBinding::new("ctrl-home", DocHome, Some("KmTextArea")),
        KeyBinding::new("ctrl-end", DocEnd, Some("KmTextArea")),
        KeyBinding::new("shift-left", SelectLeft, Some("KmTextArea")),
        KeyBinding::new("shift-right", SelectRight, Some("KmTextArea")),
        KeyBinding::new("shift-up", SelectUp, Some("KmTextArea")),
        KeyBinding::new("shift-down", SelectDown, Some("KmTextArea")),
        KeyBinding::new("shift-home", SelectToLineStart, Some("KmTextArea")),
        KeyBinding::new("shift-end", SelectToLineEnd, Some("KmTextArea")),
        KeyBinding::new("ctrl-a", SelectAll, Some("KmTextArea")),
        KeyBinding::new("enter", Enter, Some("KmTextArea")),
        KeyBinding::new("escape", Escape, Some("KmTextArea")),
        KeyBinding::new("tab", Tab, Some("KmTextArea")),
        KeyBinding::new("ctrl-v", Paste, Some("KmTextArea")),
        KeyBinding::new("ctrl-c", Copy, Some("KmTextArea")),
        KeyBinding::new("ctrl-x", Cut, Some("KmTextArea")),
    ]);
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextAreaEvent {
    Changed,
    EscapePressed,
}

pub struct TextArea {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    /// 每行起始字节偏移（首行恒为 0），变更时重建
    line_starts: Vec<usize>,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    is_selecting: bool,
    v_scroll: Pixels,
    h_scroll: Pixels,
    /// 上下移动时保持的目标 x
    goal_x: Option<Pixels>,
    read_only: bool,
    disabled: bool,
    last_bounds: Option<Bounds<Pixels>>,
    /// prepaint 缓存的可见行（起始行号 + shaped lines），供 hit-test 复用
    visible_lines: std::rc::Rc<std::cell::RefCell<(usize, Vec<ShapedLine>)>>,
    pub text_color: Option<Hsla>,
}

impl TextArea {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: "".into(),
            line_starts: vec![0],
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            is_selecting: false,
            v_scroll: px(0.),
            h_scroll: px(0.),
            goal_x: None,
            read_only: false,
            disabled: false,
            last_bounds: None,
            visible_lines: std::rc::Rc::new(std::cell::RefCell::new((0, Vec::new()))),
            text_color: None,
        }
    }

    pub fn text(&self) -> String {
        self.content.to_string()
    }

    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.content = text.into().into();
        self.rebuild_lines();
        let len = self.content.len();
        self.selected_range = len..len;
        self.marked_range = None;
        self.v_scroll = px(0.);
        self.h_scroll = px(0.);
        cx.notify();
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>) {
        self.placeholder = placeholder.into();
    }

    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn rebuild_lines(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        for (idx, b) in self.content.as_bytes().iter().enumerate() {
            if *b == b'\n' {
                self.line_starts.push(idx + 1);
            }
        }
    }

    fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// 行号 -> (行起始, 行结束不含 \n)
    fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.line_starts[line];
        let mut end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1] - 1
        } else {
            self.content.len()
        };
        if end < start {
            end = start;
        }
        start..end
    }

    fn line_text(&self, line: usize) -> &str {
        &self.content[self.line_range(line)]
    }

    /// 字节偏移 -> (行号, 行内字节列)
    fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.content.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(l) => l,
            Err(l) => l - 1,
        };
        (line, offset - self.line_starts[line])
    }

    fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let line = line.min(self.line_count().saturating_sub(1));
        let range = self.line_range(line);
        (range.start + col).min(range.end)
    }

    // ---------- 光标 ----------

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.goal_x = None;
        cx.notify()
    }

    fn move_to_keep_goal(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    // ---------- 动作 ----------

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn vertical_move(&mut self, delta: isize, select: bool, window: &Window, cx: &mut Context<Self>) {
        let cursor = self.cursor_offset();
        let (line, col) = self.offset_to_line_col(cursor);
        let line_height = window.line_height();
        let _ = line_height;
        // 目标 x：优先沿用 goal_x，否则用当前列近似（按字符数 * 字宽，由 shape 精化成本过高，列近似即可）
        let goal = self.goal_x;
        let new_line = (line as isize + delta).clamp(0, self.line_count() as isize - 1) as usize;
        let target_col = if let Some(_gx) = goal {
            // 简单策略：保持列号
            col
        } else {
            col
        };
        let new_offset = self.line_col_to_offset(new_line, target_col);
        if select {
            self.select_to(new_offset, cx);
        } else {
            self.move_to_keep_goal(new_offset, cx);
        }
        self.goal_x = goal.or(Some(px(target_col as f32 * 8.0)));
    }

    fn up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(-1, false, window, cx);
    }

    fn down(&mut self, _: &Down, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(1, false, window, cx);
    }

    fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(-1, true, window, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        self.vertical_move(1, true, window, cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        self.move_to(self.line_starts[line], cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        self.move_to(self.line_range(line).end, cx);
    }

    fn doc_home(&mut self, _: &DocHome, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn doc_end(&mut self, _: &DocEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn select_to_line_start(&mut self, _: &SelectToLineStart, _: &mut Window, cx: &mut Context<Self>) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        self.select_to(self.line_starts[line], cx);
    }

    fn select_to_line_end(&mut self, _: &SelectToLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        self.select_to(self.line_range(line).end, cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                window.play_system_bell();
                return;
            }
            self.select_to(prev, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                window.play_system_bell();
                return;
            }
            self.select_to(next, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "  ", window, cx);
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TextAreaEvent::EscapePressed);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        } else if self.read_only {
            // 只读模式下无选区复制全部
            cx.write_to_clipboard(ClipboardItem::new_string(self.content.to_string()));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    // ---------- 鼠标 ----------

    fn offset_for_position(&self, position: Point<Pixels>, line_height: Pixels) -> usize {
        let Some(bounds) = self.last_bounds else {
            return 0;
        };
        let line = (((position.y - bounds.top() + self.v_scroll) / line_height).max(0.0) as usize)
            .min(self.line_count().saturating_sub(1));
        let col = {
            let cache = self.visible_lines.borrow();
            let (start, lines) = &*cache;
            let rel_x = position.x - bounds.left() + self.h_scroll;
            if line >= *start && line < *start + lines.len() {
                lines[line - *start].index_for_x(rel_x).unwrap_or_else(|| {
                    self.line_range(line).end - self.line_starts[line]
                })
            } else {
                // 未缓存的行：按字符近似
                let text = self.line_text(line);
                let approx = ((rel_x / px(7.0)).max(0.0)) as usize;
                approx.min(text.len())
            }
        };
        self.line_col_to_offset(line, col)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        window.focus(&self.focus_handle, cx);
        self.is_selecting = true;
        let index = self.offset_for_position(event.position, window.line_height());
        if event.modifiers.shift {
            self.select_to(index, cx);
        } else if event.click_count == 2 {
            let found = {
                let content = &self.content;
                content
                    .unicode_word_indices()
                    .find(|(idx, word)| index >= *idx && index <= idx + word.len())
                    .map(|(idx, word)| (idx, idx + word.len()))
            };
            if let Some((start, end)) = found {
                self.move_to(start, cx);
                self.select_to(end, cx);
            } else {
                self.move_to(index, cx);
            }
        } else {
            self.move_to(index, cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(
                self.offset_for_position(event.position, window.line_height()),
                cx,
            );
        }
    }

    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line_height = window.line_height();
        let delta = event.delta.pixel_delta(line_height);
        let Some(bounds) = self.last_bounds else { return };
        let content_height = line_height * self.line_count() as f32;
        let max_v = (content_height - bounds.size.height).max(px(0.));
        self.v_scroll = (self.v_scroll - delta.y).clamp(px(0.), max_v);
        // 横向：最长行宽未知，给一个宽松上限
        self.h_scroll = (self.h_scroll - delta.x).max(px(0.));
        cx.notify();
    }

    // ---------- utf16 转换 ----------

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

impl EntityInputHandler for TextArea {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.rebuild_lines();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        cx.emit(TextAreaEvent::Changed);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.rebuild_lines();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        cx.emit(TextAreaEvent::Changed);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let (line, _col) = self.offset_to_line_col(range.start);
        let line_height = window.line_height();
        let y = bounds.top() + line_height * line as f32 - self.v_scroll;
        Some(Bounds::new(
            point(bounds.left(), y),
            size(bounds.size.width, line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.offset_for_position(point, window.line_height());
        Some(self.offset_to_utf16(offset))
    }
}

impl EventEmitter<TextAreaEvent> for TextArea {}

// ==================== Element ====================

pub struct TextAreaElement {
    input: Entity<TextArea>,
}

impl TextAreaElement {
    pub fn new(input: Entity<TextArea>) -> Self {
        Self { input }
    }
}

pub struct TextAreaPrepaint {
    first_line: usize,
    lines: Vec<ShapedLine>,
    line_height: Pixels,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
    placeholder: Option<ShapedLine>,
}

impl IntoElement for TextAreaElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextAreaElement {
    type RequestLayoutState = ();
    type PrepaintState = TextAreaPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let line_height = window.line_height();
        let (v_scroll, mut h_scroll) = {
            let input = self.input.read(cx);
            (input.v_scroll, input.h_scroll)
        };

        // 光标可见性滚动调整
        let (cursor_offset, selected_range, content_empty) = {
            let input = self.input.read(cx);
            (
                input.cursor_offset(),
                input.selected_range.clone(),
                input.content.is_empty(),
            )
        };
        let (cursor_line, cursor_col) = self
            .input
            .read(cx)
            .offset_to_line_col(cursor_offset);

        let content_height = line_height * self.input.read(cx).line_count() as f32;
        let max_v = (content_height - bounds.size.height).max(px(0.));
        let v_scroll = v_scroll.clamp(px(0.), max_v);

        let cursor_y = line_height * cursor_line as f32;
        let mut v_scroll = v_scroll;
        if cursor_y < v_scroll {
            v_scroll = cursor_y;
        } else if cursor_y + line_height > v_scroll + bounds.size.height {
            v_scroll = cursor_y + line_height - bounds.size.height;
        }

        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let font = style.font();
        let text_color = self
            .input
            .read(cx)
            .text_color
            .unwrap_or(style.color);

        // 可见行范围
        let first_line = ((v_scroll / line_height).max(0.0) as usize)
            .min(self.input.read(cx).line_count().saturating_sub(1));
        let visible_count =
            ((bounds.size.height / line_height) as usize + 2).max(1);

        // 光标行 shaping（用于精确光标 x）
        let mut shaped: Vec<ShapedLine> = Vec::with_capacity(visible_count);
        let mut cursor_x = px(0.);
        {
            let input = self.input.read(cx);
            let line_count = input.line_count();
            for i in 0..visible_count {
                let line_idx = first_line + i;
                if line_idx >= line_count {
                    break;
                }
                let text = input.line_text(line_idx);
                let run = TextRun {
                    len: text.len(),
                    font: font.clone(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let shaped_line =
                    window
                        .text_system()
                        .shape_line(SharedString::from(text.to_string()), font_size, &[run], None);
                if line_idx == cursor_line {
                    cursor_x = shaped_line.x_for_index(cursor_col.min(
                        input.line_range(line_idx).end - input.line_starts[line_idx],
                    ));
                }
                shaped.push(shaped_line);
            }
        }

        // 横向滚动：保证光标可见
        let padding = px(8.);
        if cursor_x - h_scroll > bounds.size.width - padding {
            h_scroll = cursor_x - bounds.size.width + padding;
        } else if cursor_x < h_scroll + padding {
            h_scroll = (cursor_x - padding).max(px(0.));
        }

        self.input.update(cx, |input, _cx| {
            input.v_scroll = v_scroll;
            input.h_scroll = h_scroll;
        });

        // 光标 quad
        let cursor_quad = if selected_range.is_empty() {
            Some(fill(
                Bounds::new(
                    point(
                        bounds.left() + cursor_x - h_scroll,
                        bounds.top() + cursor_y - v_scroll,
                    ),
                    size(px(1.5), line_height),
                ),
                theme::primary_alpha(0.9),
            ))
        } else {
            None
        };

        // 选区 quads（可见行部分）
        let mut selections = Vec::new();
        if !selected_range.is_empty() {
            let input = self.input.read(cx);
            let (sel_start_line, sel_start_col) = input.offset_to_line_col(selected_range.start);
            let (sel_end_line, sel_end_col) = input.offset_to_line_col(selected_range.end);
            for (i, shaped_line) in shaped.iter().enumerate() {
                let line_idx = first_line + i;
                if line_idx < sel_start_line || line_idx > sel_end_line {
                    continue;
                }
                let line_len = input.line_range(line_idx).end - input.line_starts[line_idx];
                let start_col = if line_idx == sel_start_line { sel_start_col } else { 0 };
                let end_col = if line_idx == sel_end_line {
                    sel_end_col
                } else {
                    // 选到行尾（含换行则多给半个字符宽表示选中换行符）
                    line_len
                };
                let x0 = shaped_line.x_for_index(start_col.min(line_len));
                let mut x1 = shaped_line.x_for_index(end_col.min(line_len));
                if line_idx < sel_end_line {
                    x1 = x1 + px(6.);
                }
                selections.push(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + x0 - h_scroll,
                            bounds.top() + line_height * line_idx as f32 - v_scroll,
                        ),
                        point(
                            bounds.left() + x1 - h_scroll,
                            bounds.top() + line_height * (line_idx + 1) as f32 - v_scroll,
                        ),
                    ),
                    theme::primary_alpha(0.25),
                ));
            }
        }

        // 占位符
        let placeholder = if content_empty {
            let input = self.input.read(cx);
            if input.placeholder.is_empty() {
                None
            } else {
                let run = TextRun {
                    len: input.placeholder.len(),
                    font,
                    color: theme::base_content_alpha(0.4),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                Some(window.text_system().shape_line(
                    input.placeholder.clone(),
                    font_size,
                    &[run],
                    None,
                ))
            }
        } else {
            None
        };

        // 缓存可见行供 hit-test
        *self.input.read(cx).visible_lines.borrow_mut() = (first_line, shaped.clone());

        TextAreaPrepaint {
            first_line,
            lines: shaped,
            line_height,
            cursor: cursor_quad,
            selections,
            placeholder,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        let (v_scroll, h_scroll) = {
            let input = self.input.read(cx);
            (input.v_scroll, input.h_scroll)
        };
        let focused = focus_handle.is_focused(window);

        self.input.update(cx, |input, _cx| {
            input.last_bounds = Some(bounds);
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for quad in prepaint.selections.drain(..) {
                window.paint_quad(quad);
            }
            let lines = std::mem::take(&mut prepaint.lines);
            for (i, line) in lines.iter().enumerate() {
                let line_idx = prepaint.first_line + i;
                let origin = point(
                    bounds.left() - h_scroll,
                    bounds.top() + prepaint.line_height * line_idx as f32 - v_scroll,
                );
                let _ = line.paint(
                    origin,
                    prepaint.line_height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
            if let Some(ph) = prepaint.placeholder.take() {
                let _ = ph.paint(
                    point(bounds.left(), bounds.top()),
                    prepaint.line_height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
            if focused {
                if let Some(cursor) = prepaint.cursor.take() {
                    window.paint_quad(cursor);
                }
            }
        });
    }
}

impl gpui::Render for TextArea {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .key_context("KmTextArea")
            .track_focus(&self.focus_handle(cx))
            .cursor(if self.disabled {
                CursorStyle::Arrow
            } else {
                CursorStyle::IBeam
            })
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::doc_home))
            .on_action(cx.listener(Self::doc_end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_to_line_start))
            .on_action(cx.listener(Self::select_to_line_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::escape))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .child(TextAreaElement::new(cx.entity()))
    }
}

impl Focusable for TextArea {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
