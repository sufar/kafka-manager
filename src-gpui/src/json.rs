//! JSON 工具：formatJson（保精度美化，移植自 Vue utils/json.ts）、
//! 高亮模板模型与高亮分段（供消息详情/JSON 编辑器着色渲染）。

use gpui::Hsla;
use serde::Deserialize;

use crate::theme;

// ==================== formatJson ====================

pub fn is_valid_json(s: &str) -> bool {
    !s.is_empty() && serde_json::from_str::<serde_json::Value>(s).is_ok()
}

/// 递归下降 JSON 美化器：数字保持原始字符串形式（保护大整数精度），缩进 2 空格；非法原样返回。
pub fn format_json(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if !is_valid_json(s) {
        return s.to_string();
    }
    let bytes: Vec<char> = s.chars().collect();
    let mut p = Parser { chars: &bytes, i: 0 };
    p.parse_value(0)
}

struct Parser<'a> {
    chars: &'a [char],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> char {
        *self.chars.get(self.i).unwrap_or(&'\0')
    }

    fn skip_ws(&mut self) {
        while self.i < self.chars.len() && self.chars[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn parse_value(&mut self, indent: usize) -> String {
        self.skip_ws();
        match self.peek() {
            '"' => self.parse_string(),
            '{' => self.parse_object(indent + 1),
            '[' => self.parse_array(indent + 1),
            '-' | '0'..='9' => self.parse_number(),
            _ => self.parse_keyword(),
        }
    }

    fn parse_string(&mut self) -> String {
        let start = self.i;
        let mut j = self.i + 1;
        while j < self.chars.len() {
            match self.chars[j] {
                '\\' => j += 2,
                '"' => {
                    j += 1;
                    break;
                }
                _ => j += 1,
            }
        }
        let j = j.min(self.chars.len());
        self.i = j;
        self.chars[start..j].iter().collect()
    }

    fn parse_number(&mut self) -> String {
        let start = self.i;
        let mut j = self.i;
        if self.chars.get(j) == Some(&'-') {
            j += 1;
        }
        while j < self.chars.len() && self.chars[j].is_ascii_digit() {
            j += 1;
        }
        if self.chars.get(j) == Some(&'.') {
            j += 1;
            while j < self.chars.len() && self.chars[j].is_ascii_digit() {
                j += 1;
            }
        }
        if matches!(self.chars.get(j), Some('e') | Some('E')) {
            j += 1;
            if matches!(self.chars.get(j), Some('+') | Some('-')) {
                j += 1;
            }
            while j < self.chars.len() && self.chars[j].is_ascii_digit() {
                j += 1;
            }
        }
        self.i = j;
        self.chars[start..j].iter().collect()
    }

    fn parse_keyword(&mut self) -> String {
        let start = self.i;
        let mut j = self.i;
        while j < self.chars.len() && self.chars[j].is_ascii_alphabetic() {
            j += 1;
        }
        self.i = j;
        self.chars[start..j].iter().collect()
    }

    fn parse_object(&mut self, indent: usize) -> String {
        self.i += 1; // '{'
        self.skip_ws();
        if self.peek() == '}' {
            self.i += 1;
            return "{}".into();
        }
        let content_pad = "  ".repeat(indent);
        let close_pad = "  ".repeat(indent - 1);
        let mut result = String::from("{\n");
        let mut first = true;
        loop {
            if !first && self.peek() == ',' {
                self.i += 1;
            }
            first = false;
            self.skip_ws();
            if self.peek() == '}' {
                self.i += 1;
                result += &close_pad;
                result += "}";
                break;
            }
            let key = self.parse_string();
            self.skip_ws();
            self.i += 1; // ':'
            let val = self.parse_value(indent);
            result += &content_pad;
            result += &key;
            result += ": ";
            result += &val;
            self.skip_ws();
            if self.peek() == ',' {
                result += ",\n";
            } else {
                result += "\n";
            }
            if self.i >= self.chars.len() {
                break;
            }
        }
        result
    }

    fn parse_array(&mut self, indent: usize) -> String {
        self.i += 1; // '['
        self.skip_ws();
        if self.peek() == ']' {
            self.i += 1;
            return "[]".into();
        }
        let content_pad = "  ".repeat(indent);
        let close_pad = "  ".repeat(indent - 1);
        let mut result = String::from("[\n");
        let mut first = true;
        loop {
            if !first && self.peek() == ',' {
                self.i += 1;
            }
            first = false;
            self.skip_ws();
            if self.peek() == ']' {
                self.i += 1;
                result += &close_pad;
                result += "]";
                break;
            }
            let val = self.parse_value(indent);
            result += &content_pad;
            result += &val;
            self.skip_ws();
            if self.peek() == ',' {
                result += ",\n";
            } else {
                result += "\n";
            }
            if self.i >= self.chars.len() {
                break;
            }
        }
        result
    }
}

// ==================== 高亮模板 ====================

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct TokenStyle {
    pub color: String,
    #[serde(default)]
    pub font_weight: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct ThemeStyles {
    pub key: TokenStyle,
    pub string: TokenStyle,
    pub number: TokenStyle,
    pub boolean: TokenStyle,
    pub null: TokenStyle,
    pub bracket: TokenStyle,
    pub colon: TokenStyle,
    pub comma: TokenStyle,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct TemplateStyle {
    pub light: ThemeStyles,
    pub dark: ThemeStyles,
}

#[derive(Clone, Debug)]
pub struct HighlightTemplate {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
    pub style: TemplateStyle,
}

/// 内置默认模板（高对比度现代风，与 Vue json-highlight.ts 一致）
pub fn default_template() -> TemplateStyle {
    TemplateStyle {
        light: ThemeStyles {
            key: TokenStyle { color: "#9333ea".into(), font_weight: Some(600) },
            string: TokenStyle { color: "#059669".into(), font_weight: None },
            number: TokenStyle { color: "#d97706".into(), font_weight: None },
            boolean: TokenStyle { color: "#0284c7".into(), font_weight: Some(700) },
            null: TokenStyle { color: "#475569".into(), font_weight: Some(700) },
            bracket: TokenStyle { color: "#475569".into(), font_weight: None },
            colon: TokenStyle { color: "#64748b".into(), font_weight: None },
            comma: TokenStyle { color: "#64748b".into(), font_weight: None },
        },
        dark: ThemeStyles {
            key: TokenStyle { color: "#c084fc".into(), font_weight: Some(600) },
            string: TokenStyle { color: "#34d399".into(), font_weight: None },
            number: TokenStyle { color: "#fbbf24".into(), font_weight: None },
            boolean: TokenStyle { color: "#38bdf8".into(), font_weight: Some(700) },
            null: TokenStyle { color: "#94a3b8".into(), font_weight: Some(700) },
            bracket: TokenStyle { color: "#94a3b8".into(), font_weight: None },
            colon: TokenStyle { color: "#cbd5e1".into(), font_weight: None },
            comma: TokenStyle { color: "#cbd5e1".into(), font_weight: None },
        },
    }
}

/// 校验模板结构（light/dark 各 8 字段均有 color）
pub fn is_valid_template(style: &TemplateStyle) -> bool {
    let check = |t: &ThemeStyles| {
        [&t.key, &t.string, &t.number, &t.boolean, &t.null, &t.bracket, &t.colon, &t.comma]
            .iter()
            .all(|s| !s.color.is_empty())
    };
    check(&style.light) && check(&style.dark)
}

pub fn parse_template_style(style_json: &str) -> Option<TemplateStyle> {
    let style: TemplateStyle = serde_json::from_str(style_json).ok()?;
    if is_valid_template(&style) {
        Some(style)
    } else {
        None
    }
}

// ==================== 高亮分段 ====================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    Key,
    String,
    Number,
    Boolean,
    Null,
    Punct, // bracket/colon/comma/其他
    Whitespace,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

/// 词法分析（与 JS 正则 `/(\"(?:\\.|[^\"\\])*\")(\s*:)?|(-?\d+\.?\d*)|\b(true|false|null)\b/g` 等价的扫描器）
pub fn tokenize(json: &str) -> Vec<Token> {
    let chars: Vec<char> = json.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut pending_string: Option<(usize, usize)> = None; // 字符串 token 待定（可能是 key）
    let mut pending_ws_start: Option<usize> = None;

    let flush_ws = |tokens: &mut Vec<Token>, chars: &[char], ws: &mut Option<usize>, end: usize| {
        if let Some(s) = ws.take() {
            if s < end {
                tokens.push(Token {
                    kind: TokenKind::Whitespace,
                    text: chars[s..end].iter().collect(),
                });
            }
        }
    };

    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            if pending_ws_start.is_none() {
                pending_ws_start = Some(i);
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            flush_ws(&mut tokens, &chars, &mut pending_ws_start, i);
            // 解析完整字符串
            let start = i;
            let mut j = i + 1;
            while j < chars.len() {
                match chars[j] {
                    '\\' => j += 2,
                    '"' => {
                        j += 1;
                        break;
                    }
                    _ => j += 1,
                }
            }
            let j = j.min(chars.len());
            i = j;
            // 若已有 pending_string，先按普通 string 落盘
            if let Some((s, e)) = pending_string.take() {
                tokens.push(Token {
                    kind: TokenKind::String,
                    text: chars[s..e].iter().collect(),
                });
            }
            pending_string = Some((start, j));
            continue;
        }
        if ch == ':' && pending_string.is_some() {
            let (s, e) = pending_string.take().unwrap();
            tokens.push(Token {
                kind: TokenKind::Key,
                text: chars[s..e].iter().collect(),
            });
            flush_ws(&mut tokens, &chars, &mut pending_ws_start, i);
            tokens.push(Token {
                kind: TokenKind::Punct,
                text: ":".into(),
            });
            i += 1;
            continue;
        }
        // 其他字符：落盘 pending_string 为 string
        if let Some((s, e)) = pending_string.take() {
            flush_ws(&mut tokens, &chars, &mut pending_ws_start, i);
            tokens.push(Token {
                kind: TokenKind::String,
                text: chars[s..e].iter().collect(),
            });
        }
        if ch == '-' || ch.is_ascii_digit() {
            flush_ws(&mut tokens, &chars, &mut pending_ws_start, i);
            let start = i;
            let mut j = i;
            if chars.get(j) == Some(&'-') {
                j += 1;
            }
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if chars.get(j) == Some(&'.') {
                j += 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: chars[start..j].iter().collect(),
            });
            i = j;
            continue;
        }
        if ch.is_ascii_alphabetic() {
            flush_ws(&mut tokens, &chars, &mut pending_ws_start, i);
            let start = i;
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_alphabetic() {
                j += 1;
            }
            let word: String = chars[start..j].iter().collect();
            let kind = match word.as_str() {
                "true" | "false" => TokenKind::Boolean,
                "null" => TokenKind::Null,
                _ => TokenKind::Punct,
            };
            tokens.push(Token { kind, text: word });
            i = j;
            continue;
        }
        flush_ws(&mut tokens, &chars, &mut pending_ws_start, i);
        tokens.push(Token {
            kind: TokenKind::Punct,
            text: ch.to_string(),
        });
        i += 1;
    }
    let end = chars.len();
    if let Some((s, e)) = pending_string.take() {
        flush_ws(&mut tokens, &chars, &mut pending_ws_start, e);
        tokens.push(Token {
            kind: TokenKind::String,
            text: chars[s..e].iter().collect(),
        });
    }
    flush_ws(&mut tokens, &chars, &mut pending_ws_start, end);
    tokens
}

/// 按当前主题取 token 颜色/字重
pub fn token_color(styles: &ThemeStyles, kind: TokenKind) -> (Hsla, gpui::FontWeight) {
    let s = match kind {
        TokenKind::Key => &styles.key,
        TokenKind::String => &styles.string,
        TokenKind::Number => &styles.number,
        TokenKind::Boolean => &styles.boolean,
        TokenKind::Null => &styles.null,
        TokenKind::Punct => &styles.bracket,
        TokenKind::Whitespace => &styles.bracket,
    };
    let color = theme::parse_hex(&s.color).unwrap_or_else(theme::text_primary);
    let weight = match s.font_weight.unwrap_or(400) {
        600 => gpui::FontWeight::SEMIBOLD,
        700 => gpui::FontWeight::BOLD,
        500 => gpui::FontWeight::MEDIUM,
        _ => gpui::FontWeight::NORMAL,
    };
    (color, weight)
}

pub fn styles_for_current_theme(tpl: &TemplateStyle) -> &ThemeStyles {
    if theme::is_dark() {
        &tpl.dark
    } else {
        &tpl.light
    }
}

// ==================== hex 工具 ====================

/// 字符串转空格分隔的两位十六进制（消息详情 hex 视图）
pub fn to_hex_dump(s: &str) -> String {
    s.as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
