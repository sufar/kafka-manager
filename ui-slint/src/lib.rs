// ui-slint library crate

// 导出 Slint 生成的模块（供 main.rs 使用）
slint::include_modules!();

// 声明测试可访问的模块
pub mod i18n;

// 单元测试模块
#[cfg(test)]
mod i18n_test;