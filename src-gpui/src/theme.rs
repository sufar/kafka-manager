//! 主题系统：对齐 Vue 前端 DaisyUI light/dark 全部色值。
//! 颜色 token 为大写函数名（读进程级全局 mode）；切换用 `set_mode()` + `cx.refresh_windows()`。

use gpui::{Hsla, Rgba};
use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Light,
    Dark,
}

static MODE: RwLock<Option<Mode>> = RwLock::new(None);

pub fn mode() -> Mode {
    MODE.read().ok().and_then(|m| *m).unwrap_or(Mode::Light)
}

pub fn is_dark() -> bool {
    mode() == Mode::Dark
}

pub fn set_mode(m: Mode) {
    if let Ok(mut g) = MODE.write() {
        *g = Some(m);
    }
}

pub fn toggle() {
    set_mode(match mode() {
        Mode::Light => Mode::Dark,
        Mode::Dark => Mode::Light,
    });
}

fn rgb(hex: u32) -> Hsla {
    Rgba { r: ((hex >> 16) & 0xff) as f32 / 255.0, g: ((hex >> 8) & 0xff) as f32 / 255.0, b: (hex & 0xff) as f32 / 255.0, a: 1.0 }.into()
}

fn rgba(hex: u32, a: f32) -> Hsla {
    Rgba { r: ((hex >> 16) & 0xff) as f32 / 255.0, g: ((hex >> 8) & 0xff) as f32 / 255.0, b: (hex & 0xff) as f32 / 255.0, a }.into()
}

fn pick(light: u32, dark: u32) -> Hsla {
    if is_dark() { rgb(dark) } else { rgb(light) }
}

fn pick_a(light: (u32, f32), dark: (u32, f32)) -> Hsla {
    let (h, a) = if is_dark() { dark } else { light };
    rgba(h, a)
}

// ==================== DaisyUI base ====================

pub fn base_100() -> Hsla { pick(0xffffff, 0x1d232a) }
pub fn base_200() -> Hsla { pick(0xf8f8f8, 0x191e24) }
pub fn base_300() -> Hsla { pick(0xeeeeee, 0x15191e) }
pub fn base_content() -> Hsla { pick(0x18181b, 0xecf9ff) }

pub fn primary() -> Hsla { pick(0x422ad5, 0x605dff) }
pub fn primary_content() -> Hsla { pick(0xe0e7ff, 0xedf1fe) }
pub fn secondary() -> Hsla { rgb(0xf43098) }
pub fn secondary_content() -> Hsla { rgb(0xf9e4f0) }
pub fn accent() -> Hsla { rgb(0x00d3bb) }
pub fn accent_content() -> Hsla { rgb(0x084d49) }
pub fn neutral() -> Hsla { rgb(0x09090b) }
pub fn neutral_content() -> Hsla { rgb(0xe4e4e7) }
pub fn info() -> Hsla { rgb(0x00bafe) }
pub fn info_content() -> Hsla { rgb(0x042e49) }
pub fn success() -> Hsla { rgb(0x00d390) }
pub fn success_content() -> Hsla { rgb(0x004c39) }
pub fn warning() -> Hsla { rgb(0xfcb700) }
pub fn warning_content() -> Hsla { rgb(0x793205) }
pub fn error() -> Hsla { rgb(0xff627d) }
pub fn error_content() -> Hsla { rgb(0x4d0218) }

// ==================== 自定义变量（style.css） ====================

pub fn glass_bg() -> Hsla { pick_a((0xffffff, 0.7), (0x0f0f1a, 0.8)) }
pub fn glass_border() -> Hsla { pick_a((0x6366f1, 0.2), (0x3c3c50, 0.15)) }
pub fn bg_primary() -> Hsla { pick(0xf8fafc, 0x0f0f1a) }
pub fn bg_secondary() -> Hsla { pick(0xffffff, 0x1a1a2e) }
pub fn text_primary() -> Hsla { pick(0x1e293b, 0xf1f5f9) }
pub fn text_secondary() -> Hsla { pick(0x475569, 0x94a3b8) }

// 渐变（btn-primary / 滚动条 / stat）
pub fn gradient_1() -> Hsla { rgb(0x6366f1) }
pub fn gradient_2() -> Hsla { rgb(0x8b5cf6) }
pub fn gradient_3() -> Hsla { rgb(0xd946ef) }
pub fn gradient_4() -> Hsla { rgb(0xec4899) }
pub fn btn_primary_hover_1() -> Hsla { rgb(0x7c3aed) }
pub fn btn_primary_hover_2() -> Hsla { rgb(0xa855f7) }
pub fn btn_secondary_1() -> Hsla { rgb(0xec4899) }
pub fn btn_secondary_2() -> Hsla { rgb(0xf43f5e) }

// 常用透明度变体
pub fn primary_alpha(a: f32) -> Hsla { rgba(0x6366f1, a) }
pub fn base_content_alpha(a: f32) -> Hsla {
    if is_dark() { rgba(0xecf9ff, a) } else { rgba(0x18181b, a) }
}
pub fn border_base_200() -> Hsla {
    // dark 下 .border-base-200 强制 rgba(80,80,100,0.2)
    pick_a((0xf8f8f8, 1.0), (0x505064, 0.2))
}
pub fn table_row_hover() -> Hsla { rgba(0x6366f1, if is_dark() { 0.1 } else { 0.08 }) }
pub fn input_bg() -> Hsla { pick_a((0xffffff, 0.95), (0x1a1a2e, 0.95)) }
pub fn input_focus_bg() -> Hsla { rgba(0x6366f1, 0.1) }
pub fn input_focus_border() -> Hsla { rgb(0x6366f1) }
pub fn input_hover_border() -> Hsla { rgba(0x6366f1, 0.3) }
pub fn context_menu_bg() -> Hsla { pick_a((0xffffff, 0.98), (0x0f0f1a, 0.98)) }
pub fn context_menu_item_hover() -> Hsla { rgba(0x6366f1, 0.15) }
pub fn modal_bg() -> Hsla { pick_a((0xffffff, 0.98), (0x1a1a2e, 0.95)) }
pub fn navbar_bg() -> Hsla { pick_a((0xffffff, 0.85), (0x0f0f1a, 0.85)) }
pub fn badge_primary_bg() -> Hsla { rgba(0x6366f1, 0.15) }
pub fn badge_primary_text() -> Hsla { rgb(0x6366f1) }
pub fn badge_secondary_bg() -> Hsla { rgba(0xec4899, 0.15) }
pub fn badge_secondary_text() -> Hsla { rgb(0xec4899) }
pub fn overlay_bg() -> Hsla { rgba(0x000000, 0.5) }
pub fn modal_backdrop() -> Hsla { rgba(0x000000, 0.3) }
pub fn btn_ghost_hover() -> Hsla { base_content_alpha(0.1) }
pub fn selected_row_bg() -> Hsla {
    // bg-primary/30
    if is_dark() { rgba(0x605dff, 0.3) } else { rgba(0x422ad5, 0.3) }
}

// 健康状态圆点
pub fn health_ok() -> Hsla { success() }
pub fn health_bad() -> Hsla { error() }
pub fn health_unknown() -> Hsla { base_300() }

// 搜索高亮 mark
pub fn mark_bg() -> Hsla { warning() }
pub fn mark_text() -> Hsla { warning_content() }

// 面板内搜索高亮（消息详情 Ctrl+F）
pub fn search_highlight_bg() -> Hsla { rgb(0xfde68a) }
pub fn search_highlight_current() -> Hsla { rgb(0xf59e0b) }

/// 十六进制 #rrggbb 解析（JSON 高亮模板用）
pub fn parse_hex(s: &str) -> Option<Hsla> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(rgb(v))
}
