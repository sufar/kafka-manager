/// JSON 格式化模块
/// 提供 JSON 美化、语法高亮功能

use serde_json::Value;

/// 格式化 JSON 字符串
/// 如果输入不是有效的 JSON，则原样返回
pub fn format_json(input: &str) -> String {
    match serde_json::from_str::<Value>(input) {
        Ok(value) => {
            match serde_json::to_string_pretty(&value) {
                Ok(pretty) => pretty,
                Err(_) => input.to_string(),
            }
        }
        Err(_) => input.to_string(),
    }
}

/// 尝试解析并格式化 JSON，如果失败则返回原始字符串
#[allow(dead_code)]
pub fn try_format_json(input: &str) -> (String, bool) {
    match serde_json::from_str::<Value>(input) {
        Ok(value) => (serde_json::to_string_pretty(&value).unwrap(), true),
        Err(_) => (input.to_string(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_json() {
        let input = r#"{"name":"test","value":123}"#;
        let output = format_json(input);
        assert!(output.contains('\n'));
        assert!(output.contains("  "));
    }

    #[test]
    fn test_format_json_invalid() {
        let input = "not json";
        let output = format_json(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_try_format_json() {
        let input = r#"{"key":"value"}"#;
        let (output, is_json) = try_format_json(input);
        assert!(is_json);
        assert!(output.contains('\n'));
    }
}
