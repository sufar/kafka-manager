//! 国际化：内嵌 zh/en 翻译（从 ui/src/i18n/translations.ts 机械转换）
//!
//! 用法：`i18n::t(cx, "nav.clusters")`，键为点分隔路径。

use gpui::{App, Global};
use serde_json::Value;

static ZH: &str = include_str!("../locales/zh.json");
static EN: &str = include_str!("../locales/en.json");

pub struct I18n {
    lang: &'static str,
    zh: Value,
    en: Value,
}

impl Global for I18n {}

impl I18n {
    pub fn new() -> Self {
        Self {
            lang: "zh",
            zh: serde_json::from_str(ZH).expect("invalid zh.json"),
            en: serde_json::from_str(EN).expect("invalid en.json"),
        }
    }

    pub fn init(cx: &mut App) {
        if !cx.has_global::<I18n>() {
            cx.set_global(I18n::new());
        }
    }

    pub fn global(cx: &App) -> &I18n {
        cx.global::<I18n>()
    }

    pub fn global_mut(cx: &mut App) -> &mut I18n {
        cx.global_mut::<I18n>()
    }

    pub fn lang(&self) -> &'static str {
        self.lang
    }

    pub fn is_zh(&self) -> bool {
        self.lang == "zh"
    }

    pub fn set_lang(&mut self, lang: &'static str) {
        self.lang = lang;
    }

    /// 按键（点分隔）查翻译，未命中时返回键本身
    pub fn lookup(&self, key: &str) -> String {
        let dict = if self.is_zh() { &self.zh } else { &self.en };
        let mut node = dict;
        for part in key.split('.') {
            match node.get(part) {
                Some(v) => node = v,
                None => return key.to_string(),
            }
        }
        node.as_str().unwrap_or(key).to_string()
    }
}

/// 全局翻译快捷函数
pub fn t(cx: &App, key: &str) -> String {
    I18n::global(cx).lookup(key)
}
