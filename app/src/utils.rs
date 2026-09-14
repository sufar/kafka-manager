//! 共享工具：相对时间格式化、JSON 高亮模板解析与着色

use std::ops::Range;

use gpui::{App, HighlightStyle, Hsla};
use serde::Deserialize;

use crate::i18n::t;

/// 将 RFC3339 时间戳格式化为相对时间（"刚刚 / N 分钟前 / N 小时前 / N 天前"）。
/// section 为 i18n 节名（"history" 或 "sentMessageHistory"，两节键名相同）。
/// 超过 7 天显示日期（MM-DD 或 YYYY-MM-DD）。
pub fn relative_time(cx: &App, rfc3339: &str, section: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(rfc3339)
        .or_else(|_| {
            // 兼容 "YYYY-MM-DD HH:MM:SS"（SQLite datetime('now') 风格，按 UTC 处理）
            chrono::NaiveDateTime::parse_from_str(rfc3339, "%Y-%m-%d %H:%M:%S")
                .map(|n| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(n, chrono::Utc).into())
        });
    let Ok(ts) = parsed else {
        return rfc3339.to_string();
    };
    let now = chrono::Utc::now();
    let secs = (now - ts.with_timezone(&chrono::Utc)).num_seconds();
    if secs < 60 {
        t(cx, &format!("{section}.justNow"))
    } else if secs < 3600 {
        format!("{} {}", secs / 60, t(cx, &format!("{section}.minutesAgo")))
    } else if secs < 86400 {
        format!("{} {}", secs / 3600, t(cx, &format!("{section}.hoursAgo")))
    } else if secs < 7 * 86400 {
        format!("{} {}", secs / 86400, t(cx, &format!("{section}.daysAgo")))
    } else {
        let local = ts.with_timezone(&chrono::Local);
        if local.format("%Y").to_string() == chrono::Local::now().format("%Y").to_string() {
            local.format("%m-%d").to_string()
        } else {
            local.format("%Y-%m-%d").to_string()
        }
    }
}

// ==================== JSON 高亮模板 ====================

/// 模板样式配置（与后端 src/db/json_highlight.rs 结构一致）
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateStyle {
    pub light: ThemeStyles,
    pub dark: ThemeStyles,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeStyles {
    pub key: StyleConfig,
    pub string: StyleConfig,
    pub number: StyleConfig,
    pub boolean: StyleConfig,
    pub null: StyleConfig,
    pub bracket: StyleConfig,
    pub colon: StyleConfig,
    pub comma: StyleConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StyleConfig {
    pub color: String,
    #[serde(default)]
    pub font_weight: Option<String>,
}

impl StyleConfig {
    fn highlight(&self) -> Option<HighlightStyle> {
        let color = parse_hex_color(&self.color)?;
        let mut style = HighlightStyle {
            color: Some(color),
            ..Default::default()
        };
        if let Some(w) = &self.font_weight {
            let weight = w.parse::<f32>().ok().map(|v| gpui::FontWeight(v));
            if let Some(fw) = weight {
                style.font_weight = Some(fw);
            } else if w == "bold" {
                style.font_weight = Some(gpui::FontWeight::BOLD);
            }
        }
        Some(style)
    }
}

/// 解析 "#rrggbb" / "#rgb" 十六进制颜色
pub fn parse_hex_color(s: &str) -> Option<Hsla> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    let (r, g, b) = match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        ),
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            (r * 17, g * 17, b * 17)
        }
        _ => return None,
    };
    Some(gpui::rgb(u32::from_be_bytes([0, r, g, b])).into())
}

/// 从后端加载当前 JSON 高亮模板并写入 JsonTemplate Global。
/// 幂等：启动时与设置页切换模板后各调一次。
pub fn load_json_template(cx: &mut App) {
    use crate::state::{Backend, JsonTemplate, TokioRuntime};
    let Some(state) = Backend::state(cx) else { return };
    let rt = TokioRuntime::handle(cx);
    cx.spawn(async move |cx| {
        let name = crate::service::call(
            &rt,
            state.clone(),
            "json_highlight.get_current",
            serde_json::json!({}),
        )
        .await
        .ok()
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(String::from));
        let Some(name) = name else { return };
        let list = crate::service::call(&rt, state, "json_highlight.list", serde_json::json!({}))
            .await
            .ok();
        let Some(list) = list else { return };
        let style = list
            .get("templates")
            .and_then(|t| t.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(name.as_str()))
            })
            .and_then(|t| t.get("style_json").and_then(|s| s.as_str()).map(String::from))
            .and_then(|s| serde_json::from_str::<TemplateStyle>(&s).ok());
        cx.update(|cx| JsonTemplate::set(cx, style)).ok();
    })
    .detach();
}

/// 对（已 pretty-print 的）JSON 文本做词法分析，返回模板着色的高亮区间。
/// 手写的容错 tokenizer：字符串（区分 key/字符串值）、数字、true/false/null、括号、冒号、逗号。
pub fn json_template_highlights(text: &str, theme: &ThemeStyles) -> Vec<(Range<usize>, HighlightStyle)> {    let bytes = text.as_bytes();
    let mut out: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    let mut i = 0usize;

    let push = |out: &mut Vec<(Range<usize>, HighlightStyle)>, range: Range<usize>, cfg: &StyleConfig| {
        if let Some(h) = cfg.highlight() {
            out.push((range, h));
        }
    };

    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                // 向后看：跳过空白后若是 ':' 则为 key
                let mut j = i;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b':' {
                    push(&mut out, start..i, &theme.key);
                } else {
                    push(&mut out, start..i, &theme.string);
                }
            }
            b'0'..=b'9' | b'-' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && matches!(bytes[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                {
                    i += 1;
                }
                push(&mut out, start..i, &theme.number);
            }
            b't' | b'f' => {
                // true / false
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                push(&mut out, start..i, &theme.boolean);
            }
            b'n' => {
                // null
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                push(&mut out, start..i, &theme.null);
            }
            b'{' | b'}' | b'[' | b']' => {
                push(&mut out, i..i + 1, &theme.bracket);
                i += 1;
            }
            b':' => {
                push(&mut out, i..i + 1, &theme.colon);
                i += 1;
            }
            b',' => {
                push(&mut out, i..i + 1, &theme.comma);
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    out
}
