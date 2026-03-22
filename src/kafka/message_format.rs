/// Kafka 消息格式自动处理模块
///
/// 支持自动检测和解析多种消息格式：
/// - JSON：自动格式化
/// - 纯文本：直接显示
/// - 二进制数据：支持 Hex 和 Base64 编码
/// - Avro/Protobuf：预留支持

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 消息格式类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageFormat {
    /// JSON 格式
    Json,
    /// 纯文本
    Text,
    /// 二进制（Hex 编码）
    BinaryHex,
    /// 二进制（Base64 编码）
    BinaryBase64,
    /// Avro 格式（需要 Schema）
    Avro,
    /// Protobuf 格式（需要 Schema）
    Protobuf,
    /// 未知格式
    Unknown,
}

/// 格式化后的消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedMessage {
    /// 原始内容（字符串形式）
    pub raw: String,
    /// 格式化后的内容
    pub formatted: String,
    /// 检测到的格式类型
    pub format: MessageFormat,
    /// 解析后的结构化数据（仅 JSON 格式）
    pub parsed: Option<serde_json::Value>,
    /// 元数据
    pub metadata: MessageMetadata,
}

/// 消息元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageMetadata {
    /// 内容大小（字节）
    pub size_bytes: usize,
    /// 是否为空消息
    pub is_empty: bool,
    /// JSON 字段数量（仅 JSON 格式）
    pub field_count: Option<usize>,
    /// 嵌套深度（仅 JSON 格式）
    pub nesting_depth: Option<usize>,
    /// 二进制数据标识
    pub is_binary: bool,
}

/// 格式化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatConfig {
    /// JSON 是否美化输出
    pub json_pretty: bool,
    /// JSON 缩进空格数
    pub json_indent: usize,
    /// 二进制数据编码方式
    pub binary_encoding: BinaryEncoding,
    /// 最大格式化大小（超过此值不格式化）
    pub max_size: usize,
}

/// 二进制数据编码方式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BinaryEncoding {
    Hex,
    Base64,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            json_pretty: true,
            json_indent: 2,
            binary_encoding: BinaryEncoding::Hex,
            max_size: 1024 * 1024, // 1MB
        }
    }
}

/// 检测和格式化消息内容
pub fn detect_and_format(bytes: &[u8], config: Option<&FormatConfig>) -> FormattedMessage {
    let default_config = FormatConfig::default();
    let config = config.unwrap_or(&default_config);

    // 检查是否为空
    if bytes.is_empty() {
        return FormattedMessage {
            raw: String::new(),
            formatted: String::new(),
            format: MessageFormat::Unknown,
            parsed: None,
            metadata: MessageMetadata {
                size_bytes: 0,
                is_empty: true,
                field_count: None,
                nesting_depth: None,
                is_binary: false,
            },
        };
    }

    let raw = String::from_utf8_lossy(bytes).to_string();
    let size_bytes = bytes.len();

    // 尝试解析为 JSON
    if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(bytes) {
        let (formatted, field_count, nesting_depth) = if config.json_pretty {
            let formatted = serde_json::to_string_pretty(&json_value)
                .unwrap_or_else(|_| raw.clone());
            let field_count = count_json_fields(&json_value);
            let nesting_depth = calculate_json_depth(&json_value);
            (formatted, Some(field_count), Some(nesting_depth))
        } else {
            (raw.clone(), None, None)
        };

        return FormattedMessage {
            raw,
            formatted,
            format: MessageFormat::Json,
            parsed: Some(json_value),
            metadata: MessageMetadata {
                size_bytes,
                is_empty: false,
                field_count,
                nesting_depth,
                is_binary: false,
            },
        };
    }

    // 检查是否为有效的 UTF-8 文本
    if let Ok(text) = std::str::from_utf8(bytes) {
        // 检查是否包含不可打印字符（排除常见的空白字符）
        let has_non_printable = text.chars().any(|c| {
            !c.is_ascii_graphic() && !c.is_ascii_whitespace() && c as u32 > 127
        });

        if !has_non_printable {
            return FormattedMessage {
                raw: text.to_string(),
                formatted: text.to_string(),
                format: MessageFormat::Text,
                parsed: None,
                metadata: MessageMetadata {
                    size_bytes,
                    is_empty: false,
                    field_count: None,
                    nesting_depth: None,
                    is_binary: false,
                },
            };
        }
    }

    // 二进制数据
    let formatted = match config.binary_encoding {
        BinaryEncoding::Hex => {
            bytes.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ")
        }
        BinaryEncoding::Base64 => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(bytes)
        }
    };

    FormattedMessage {
        raw,
        formatted,
        format: MessageFormat::BinaryHex,
        parsed: None,
        metadata: MessageMetadata {
            size_bytes,
            is_empty: false,
            field_count: None,
            nesting_depth: None,
            is_binary: true,
        },
    }
}

/// 统计 JSON 字段数量
fn count_json_fields(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            1 + map.values().map(count_json_fields).sum::<usize>()
        }
        serde_json::Value::Array(arr) => {
            arr.iter().map(count_json_fields).sum()
        }
        _ => 1,
    }
}

/// 计算 JSON 嵌套深度
fn calculate_json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            1 + map.values()
                .map(calculate_json_depth)
                .max()
                .unwrap_or(0)
        }
        serde_json::Value::Array(arr) => {
            1 + arr.iter()
                .map(calculate_json_depth)
                .max()
                .unwrap_or(0)
        }
        _ => 0,
    }
}

/// 格式化 Kafka 消息的 key 和 value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedKafkaMessage {
    pub partition: i32,
    pub offset: i64,
    pub timestamp: Option<i64>,
    pub key: Option<FormattedMessage>,
    pub value: Option<FormattedMessage>,
    pub headers: Option<HashMap<String, String>>,
}

impl FormattedKafkaMessage {
    /// 从原始消息创建格式化消息
    pub fn from_raw(
        partition: i32,
        offset: i64,
        timestamp: Option<i64>,
        key_bytes: Option<&[u8]>,
        value_bytes: Option<&[u8]>,
        headers: Option<&HashMap<String, Vec<u8>>>,
        config: Option<&FormatConfig>,
    ) -> Self {
        Self {
            partition,
            offset,
            timestamp,
            key: key_bytes.map(|b| detect_and_format(b, config)),
            value: value_bytes.map(|b| detect_and_format(b, config)),
            headers: headers.map(|h| {
                h.iter()
                    .map(|(k, v)| {
                        let formatted = detect_and_format(v, config);
                        (k.clone(), formatted.formatted)
                    })
                    .collect()
            }),
        }
    }
}

/// 检测消息是否为 JSON 格式
pub fn is_json(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    // 快速检查：JSON 必须以 { 或 [ 开头
    let first = bytes.iter().find(|&&b| !b.is_ascii_whitespace());
    match first {
        Some(&b'{') | Some(&b'[') => serde_json::from_slice::<serde_json::Value>(bytes).is_ok(),
        _ => false,
    }
}

/// 检测消息是否为二进制数据
pub fn is_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    // 检查是否包含大量非打印字符
    let null_count = bytes.iter().filter(|&&b| b == 0).count();
    let non_printable = bytes.iter()
        .filter(|&&b| b != 0 && !b.is_ascii_graphic() && !b.is_ascii_whitespace())
        .count();

    // 如果有空字节或超过 10% 的非打印字符，认为是二进制
    null_count > 0 || (non_printable as f64 / bytes.len() as f64) > 0.1
}

/// 将字节转换为 Hex 字符串
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 将字节转换为 Base64 字符串
pub fn to_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// 尝试从 Hex 字符串解码为字节
pub fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.replace(" ", "");
    if s.len() % 2 != 0 {
        return None;
    }

    let mut result = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i+2], 16).ok()?;
        result.push(byte);
    }
    Some(result)
}

/// 尝试从 Base64 字符串解码为字节
pub fn from_base64(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_detection() {
        let json = br#"{"name": "test", "value": 123}"#;
        let result = detect_and_format(json, None);
        assert_eq!(result.format, MessageFormat::Json);
        assert!(result.parsed.is_some());
    }

    #[test]
    fn test_text_detection() {
        let text = b"Hello, World!";
        let result = detect_and_format(text, None);
        assert_eq!(result.format, MessageFormat::Text);
    }

    #[test]
    fn test_binary_detection() {
        let binary = vec![0x00, 0x01, 0x02, 0xFF];
        let result = detect_and_format(&binary, None);
        assert!(result.metadata.is_binary);
    }

    #[test]
    fn test_hex_encoding() {
        let bytes = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let hex = to_hex(&bytes);
        assert_eq!(hex, "48 65 6c 6c 6f");

        let decoded = from_hex(&hex);
        assert_eq!(decoded, Some(bytes));
    }
}
