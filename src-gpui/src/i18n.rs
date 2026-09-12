//! 国际化：从嵌入的 translations.json（由 Vue 前端 translations.ts 提取）按点路径查文案。
//!
//! 用法：`t("nav.clusters")` 返回当前语言文案；`tf("key", &[("var", "x")])` 做 {var} 插值。
//! 语言全局进程级（与 theme 相同的模式），切换后由调用方 `cx.refresh_windows()`。

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    Zh,
    En,
}

static TRANSLATIONS: OnceLock<HashMap<String, serde_json::Value>> = OnceLock::new();
static CURRENT: RwLock<Option<Language>> = RwLock::new(None);

fn translations() -> &'static HashMap<String, serde_json::Value> {
    TRANSLATIONS.get_or_init(|| {
        let raw = include_str!("../assets/translations.json");
        serde_json::from_str(raw).expect("invalid translations.json")
    })
}

pub fn language() -> Language {
    CURRENT
        .read()
        .ok()
        .and_then(|g| *g)
        .unwrap_or(Language::Zh)
}

pub fn set_language(lang: Language) {
    if let Ok(mut g) = CURRENT.write() {
        *g = Some(lang);
    }
}

fn lookup(lang_key: &str, path: &str) -> Option<String> {
    let root = translations().get(lang_key)?;
    let mut node = root;
    for seg in path.split('.') {
        node = node.get(seg)?;
    }
    node.as_str().map(|s| s.to_string())
}

/// 按点路径取文案；当前语言缺失时回退英文，再回退中文，最后返回路径本身。
pub fn t(path: &str) -> String {
    let lang = match language() {
        Language::Zh => "zh",
        Language::En => "en",
    };
    lookup(lang, path)
        .or_else(|| lookup("en", path))
        .or_else(|| lookup("zh", path))
        .unwrap_or_else(|| path.to_string())
}

/// 取文案并做 `{name}` 占位符替换。
pub fn tf(path: &str, vars: &[(&str, &str)]) -> String {
    let mut s = t(path);
    for (k, v) in vars {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

/// 语言切换下拉用
pub fn language_label(lang: Language) -> &'static str {
    match lang {
        Language::Zh => "中文",
        Language::En => "English",
    }
}
