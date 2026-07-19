//! 通用下拉选项类型

use gpui::SharedString;
use gpui_component::select::SelectItem;

/// 字符串下拉选项（title 与 value 相同）
#[derive(Clone)]
pub struct StringOption {
    pub label: SharedString,
    pub value: SharedString,
}

impl StringOption {
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

impl SelectItem for StringOption {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}
