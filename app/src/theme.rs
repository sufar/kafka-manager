//! 自定义主题
//!
//! gpui-component 默认的暗色主题是纯黑（#0a0a0a）+ 纯白文字，对比过于生硬。
//! 这里内置了几套主流深色主题，安装阶段用选中的一套替换默认暗色主题，
//! 之后所有 `Theme::change(ThemeMode::Dark, ..)` 切换都会使用这套配色。
//!
//! 切换主题：修改下面的 `ACTIVE_DARK_THEME` 常量后重新编译即可。
//! 可选值："Dark+"（VS Code，中性柔和，默认）、"One Dark"（Atom，偏暖）、
//! "Tokyo Night"（蓝紫调，色彩丰富）。

use std::rc::Rc;

use gpui::{App, SharedString, Window};
use gpui_component::{Theme, ThemeConfig, ThemeMode, ThemeSet};

use crate::state::{Backend, TokioRuntime};

/// 当前启用的默认暗色主题名（旧版设置的 "dark" 会映射到它）
const ACTIVE_DARK_THEME: &str = "Dark+";

/// 可选主题列表：(名称, 是否暗色)
///
/// "Light" 使用 gpui-component 默认浅色主题，暗色主题配置在
/// `app/assets/themes/` 下，新增主题文件后在 `THEME_FILES` 和这里各加一行即可。
pub const THEMES: &[(&str, bool)] = &[
    ("Light", false),
    ("Dark+", true),
    ("One Dark", true),
    ("Tokyo Night", true),
    ("Catppuccin Mocha", true),
    ("Dracula", true),
    ("Nord", true),
    ("Gruvbox", true),
    ("Everforest", true),
    ("GitHub Dark", true),
];

const THEME_FILES: &[&str] = &[
    include_str!("../assets/themes/dark-plus.json"),
    include_str!("../assets/themes/one-dark.json"),
    include_str!("../assets/themes/tokyo-night.json"),
    include_str!("../assets/themes/catppuccin-mocha.json"),
    include_str!("../assets/themes/dracula.json"),
    include_str!("../assets/themes/nord.json"),
    include_str!("../assets/themes/gruvbox.json"),
    include_str!("../assets/themes/everforest.json"),
    include_str!("../assets/themes/github-dark.json"),
];

fn load_theme(name: &str) -> Option<ThemeConfig> {
    let mut configs = Vec::new();
    for file in THEME_FILES {
        match serde_json::from_str::<ThemeSet>(file) {
            Ok(set) => configs.extend(set.themes),
            Err(e) => tracing::error!("解析主题文件失败: {}", e),
        }
    }
    let mut iter = configs.into_iter().filter(|t| t.mode.is_dark());
    // 找不到指定主题时回退到第一套，保证暗色模式始终可用
    iter.clone()
        .find(|t| t.name == name)
        .or_else(|| iter.next())
}

/// 用自定义暗色主题替换默认暗色主题，需在 `gpui_component::init` 之后调用。
pub fn install(cx: &mut App) {
    let Some(config) = load_theme(ACTIVE_DARK_THEME) else {
        tracing::error!("未找到任何自定义暗色主题");
        return;
    };
    let config = Rc::new(config);

    // Theme 全局可能尚未初始化（它在首次 Theme::change 时创建）
    if !cx.has_global::<Theme>() {
        Theme::sync_system_appearance(None, cx);
    }
    let theme = Theme::global_mut(cx);
    // 替换暗色主题配置，后续明/暗切换都会用到它
    theme.dark_theme = Rc::clone(&config);
    // 当前已处于暗色模式时立即应用
    if theme.is_dark() {
        theme.apply_config(&config);
    }
}

/// 当前主题名（浅色固定为 "Light"，暗色为所选暗色主题名）
pub fn current_name(cx: &App) -> SharedString {
    let theme = Theme::global(cx);
    if theme.is_dark() {
        theme.dark_theme.name.clone()
    } else {
        "Light".into()
    }
}

/// 规范化主题名，兼容旧版设置存储的 "dark" / "light"
fn normalize(name: &str) -> &str {
    match name {
        "dark" => ACTIVE_DARK_THEME,
        "light" => "Light",
        other => other,
    }
}

/// 按名称应用主题
pub fn apply_by_name(name: &str, window: Option<&mut Window>, cx: &mut App) {
    let name = normalize(name);
    if !cx.has_global::<Theme>() {
        Theme::sync_system_appearance(None, cx);
    }
    if name == "Light" {
        Theme::change(ThemeMode::Light, window, cx);
        return;
    }
    let Some(config) = load_theme(name) else {
        return;
    };
    Theme::global_mut(cx).dark_theme = Rc::new(config);
    Theme::change(ThemeMode::Dark, window, cx);
}

/// 持久化主题选择到设置（下次启动时恢复）
pub fn persist(cx: &mut App, name: &str) {
    let Some(state) = Backend::state(cx) else {
        return;
    };
    let rt = TokioRuntime::handle(cx);
    let value = name.to_string();
    cx.spawn(async move |_cx| {
        let _ = crate::service::call(
            &rt,
            state,
            "settings.update",
            serde_json::json!({ "key": "ui.theme", "value": value }),
        )
        .await;
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_theme_files_are_valid() {
        for file in THEME_FILES {
            let set: ThemeSet = serde_json::from_str(file).expect("主题 JSON 解析失败");
            let dark = set.themes.iter().find(|t| t.mode.is_dark());
            assert!(dark.is_some(), "缺少暗色主题定义");
            assert!(dark.unwrap().highlight.is_some(), "缺少语法高亮配置");
        }
    }

    #[test]
    fn active_theme_exists() {
        let theme = load_theme(ACTIVE_DARK_THEME).expect("未找到任何暗色主题");
        assert_eq!(theme.name, ACTIVE_DARK_THEME);
    }

    #[test]
    fn every_listed_theme_loads() {
        for &(name, dark) in THEMES {
            if dark {
                let theme = load_theme(name).unwrap_or_else(|| panic!("主题 {name} 加载失败"));
                assert_eq!(theme.name, name, "主题 {name} 未找到");
            }
        }
    }
}
