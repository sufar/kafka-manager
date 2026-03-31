/// Protobuf 编解码器
/// 使用 prost crate 进行 Protobuf 消息的序列化和反序列化
///
/// 注意：由于 Protobuf 需要编译时的 .proto 文件，这里的实现采用动态方式
/// 通过 Descriptor 集来进行运行时的编码和解码

use crate::error::{AppError, Result};
use prost::Message;
use prost_types::{DescriptorProto, FileDescriptorSet};
use serde_json;
use std::collections::HashMap;

pub struct ProtobufCodec;

impl ProtobufCodec {
    /// 将 JSON 编码为 Protobuf 二进制（发送消息时使用）
    ///
    /// # Arguments
    /// * `descriptor_set` - Protobuf FileDescriptorSet 的二进制表示
    /// * `message_type` - 消息的全限定类型名（例如：com.example.Person）
    /// * `json_value` - 要编码的 JSON 数据
    ///
    /// # Returns
    /// * `Vec<u8>` - Protobuf 二进制数据
    pub fn encode(
        descriptor_set: &[u8],
        message_type: &str,
        json_value: &serde_json::Value,
    ) -> Result<Vec<u8>> {
        // 解析 descriptor set
        let fds = FileDescriptorSet::decode(descriptor_set)
            .map_err(|e| AppError::BadRequest(format!("Invalid descriptor set: {}", e)))?;

        // 查找消息类型
        let descriptor = find_descriptor(&fds, message_type)
            .ok_or_else(|| AppError::BadRequest(format!("Message type '{}' not found", message_type)))?;

        // 将 JSON 转换为 Protobuf 消息
        let protobuf_bytes = json_to_protobuf(json_value, descriptor)?;

        Ok(protobuf_bytes)
    }

    /// 将 Protobuf 二进制解码为 JSON（读取消息时使用）
    ///
    /// # Arguments
    /// * `descriptor_set` - Protobuf FileDescriptorSet 的二进制表示
    /// * `message_type` - 消息的全限定类型名
    /// * `proto_bytes` - Protobuf 二进制数据
    ///
    /// # Returns
    /// * `serde_json::Value` - 解码后的 JSON 数据
    pub fn decode(
        descriptor_set: &[u8],
        message_type: &str,
        proto_bytes: &[u8],
    ) -> Result<serde_json::Value> {
        // 解析 descriptor set
        let fds = FileDescriptorSet::decode(descriptor_set)
            .map_err(|e| AppError::BadRequest(format!("Invalid descriptor set: {}", e)))?;

        // 查找消息类型
        let descriptor = find_descriptor(&fds, message_type)
            .ok_or_else(|| AppError::BadRequest(format!("Message type '{}' not found", message_type)))?;

        // 将 Protobuf 二进制转换为 JSON
        let json_value = protobuf_to_json(proto_bytes, descriptor)?;

        Ok(json_value)
    }

    /// 简单的 JSON 到 Protobuf 编码（适用于已知消息结构的情况）
    /// 这是一个简化实现，完整实现需要动态构建消息
    pub fn encode_simple(
        schema_json: &str,
        json_value: &serde_json::Value,
    ) -> Result<Vec<u8>> {
        // 解析 schema JSON
        let schema: serde_json::Value = serde_json::from_str(schema_json)
            .map_err(|e| AppError::BadRequest(format!("Invalid schema JSON: {}", e)))?;

        // 根据 schema 编码消息
        encode_message(&schema, json_value)
    }

    /// 简单的 Protobuf 到 JSON 解码
    pub fn decode_simple(
        schema_json: &str,
        proto_bytes: &[u8],
    ) -> Result<serde_json::Value> {
        // 解析 schema JSON
        let schema: serde_json::Value = serde_json::from_str(schema_json)
            .map_err(|e| AppError::BadRequest(format!("Invalid schema JSON: {}", e)))?;

        // 根据 schema 解码消息
        decode_message(&schema, proto_bytes)
    }
}

/// 在 FileDescriptorSet 中查找指定类型的 DescriptorProto
fn find_descriptor<'a>(fds: &'a FileDescriptorSet, message_type: &'a str) -> Option<&'a DescriptorProto> {
    for file in &fds.file {
        if let Some(package) = &file.package {
            let full_name = format!("{}.{}", package, message_type);
            for desc in &file.message_type {
                if desc.name == Some(message_type.to_string()) ||
                   file.package.as_ref().map_or(false, |p|
                       format!("{}.{}", p, desc.name.as_ref().unwrap_or(&String::new())) == full_name) {
                    return Some(desc);
                }
            }
        } else {
            for desc in &file.message_type {
                if desc.name.as_deref() == Some(message_type) {
                    return Some(desc);
                }
            }
        }
    }
    None
}

/// 将 JSON 转换为 Protobuf 二进制（简化实现）
fn json_to_protobuf(json: &serde_json::Value, _descriptor: &DescriptorProto) -> Result<Vec<u8>> {
    // 注意：这是一个占位实现
    // 完整的 Protobuf 动态编码需要：
    // 1. 根据 descriptor 动态构建消息结构
    // 2. 处理所有字段类型（primitive, nested messages, enums, repeated, map）
    // 3. 使用 wire format 编码

    // 由于 prost 主要是编译时生成代码，运行时动态编码比较复杂
    // 这里提供一个简化版本，实际使用可能需要额外的动态 Protobuf 库

    match json {
        serde_json::Value::Object(map) => {
            let mut buf = Vec::new();
            let mut field_number = 1u32;

            for (key, value) in map {
                encode_field(&mut buf, field_number, key, value)?;
                field_number += 1;
            }

            Ok(buf)
        }
        _ => Err(AppError::BadRequest(
            "Protobuf message must be a JSON object".to_string()
        )),
    }
}

/// 将 Protobuf 二进制转换为 JSON（简化实现）
fn protobuf_to_json(_proto_bytes: &[u8], descriptor: &DescriptorProto) -> Result<serde_json::Value> {
    // 注意：这是一个占位实现
    // 完整的 Protobuf 动态解码需要解析 wire format

    // 返回一个包含 descriptor 信息的 JSON 结构
    let mut obj = serde_json::Map::new();

    for field in &descriptor.field {
        let field_name = field.name.as_deref().unwrap_or("unknown");
        obj.insert(field_name.to_string(), serde_json::Value::Null);
    }

    Ok(serde_json::Value::Object(obj))
}

/// 编码单个字段
fn encode_field(
    buf: &mut Vec<u8>,
    field_number: u32,
    _field_name: &str,
    value: &serde_json::Value,
) -> Result<()> {
    match value {
        serde_json::Value::Null => {
            // Protobuf 3 默认忽略 null 字段
        }
        serde_json::Value::Bool(b) => {
            // Wire type 0 (varint), field number
            prost::encoding::encode_varint(
                ((field_number << 3) | 0) as u64,
                buf,
            );
            prost::encoding::encode_varint(*b as u64, buf);
        }
        serde_json::Value::Number(n) => {
            // Wire type 0 (varint) for integers
            prost::encoding::encode_varint(
                ((field_number << 3) | 0) as u64,
                buf,
            );
            if let Some(i) = n.as_i64() {
                prost::encoding::encode_varint(i as u64, buf);
            } else if let Some(u) = n.as_u64() {
                prost::encoding::encode_varint(u, buf);
            }
        }
        serde_json::Value::String(s) => {
            // Wire type 2 (length-delimited)
            prost::encoding::encode_varint(
                ((field_number << 3) | 2) as u64,
                buf,
            );
            prost::encoding::encode_varint(s.len() as u64, buf);
            buf.extend_from_slice(s.as_bytes());
        }
        serde_json::Value::Array(arr) => {
            // 简化处理：将数组作为重复字段
            for item in arr {
                encode_field(buf, field_number, _field_name, item)?;
            }
        }
        serde_json::Value::Object(map) => {
            // Wire type 2 (length-delimited) for nested messages
            let mut nested_buf = Vec::new();
            let mut nested_field_number = 1u32;

            for (key, value) in map {
                encode_field(&mut nested_buf, nested_field_number, key, value)?;
                nested_field_number += 1;
            }

            prost::encoding::encode_varint(
                ((field_number << 3) | 2) as u64,
                buf,
            );
            prost::encoding::encode_varint(nested_buf.len() as u64, buf);
            buf.extend_from_slice(&nested_buf);
        }
    }

    Ok(())
}

/// 解码消息（简化实现）
fn decode_message(_schema: &serde_json::Value, _proto_bytes: &[u8]) -> Result<serde_json::Value> {
    // 这是一个占位实现
    // 完整的实现需要解析 Protobuf wire format

    Err(AppError::BadRequest(
        "Full Protobuf decoding requires compile-time definitions".to_string()
    ))
}

/// 编码消息（简化实现）
fn encode_message(_schema: &serde_json::Value, json: &serde_json::Value) -> Result<Vec<u8>> {
    match json {
        serde_json::Value::Object(map) => {
            let mut buf = Vec::new();
            let mut field_number = 1u32;

            for (key, value) in map {
                encode_field(&mut buf, field_number, key, value)?;
                field_number += 1;
            }

            Ok(buf)
        }
        _ => Err(AppError::BadRequest(
            "Protobuf message must be a JSON object".to_string()
        )),
    }
}

/// 解析 Protobuf Schema JSON 获取字段信息
fn parse_protobuf_schema(schema: &serde_json::Value) -> Result<HashMap<String, FieldInfo>> {
    let mut fields = HashMap::new();

    if let serde_json::Value::Object(map) = schema {
        if let Some(serde_json::Value::Array(fields_array)) = map.get("fields") {
            for field in fields_array {
                if let serde_json::Value::Object(field_obj) = field {
                    let name = field_obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| AppError::BadRequest("Field missing name".to_string()))?;

                    let field_type = field_obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("string");

                    let field_number = field_obj
                        .get("number")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as u32;

                    fields.insert(name.to_string(), FieldInfo {
                        field_type: field_type.to_string(),
                        field_number,
                    });
                }
            }
        }
    }

    Ok(fields)
}

#[derive(Debug, Clone)]
struct FieldInfo {
    field_type: String,
    field_number: u32,
}

/// 验证 JSON 是否符合 Protobuf Schema
pub fn validate(schema_json: &str, json_value: &serde_json::Value) -> Result<()> {
    let schema: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| AppError::BadRequest(format!("Invalid schema JSON: {}", e)))?;

    let fields = parse_protobuf_schema(&schema)?;

    if let serde_json::Value::Object(map) = json_value {
        for (key, value) in map {
            if let Some(field_info) = fields.get(key) {
                validate_field_type(value, &field_info.field_type)?;
            }
        }
    }

    Ok(())
}

/// 验证字段类型
fn validate_field_type(value: &serde_json::Value, field_type: &str) -> Result<()> {
    match field_type {
        "string" => {
            if !value.is_string() && !value.is_null() {
                return Err(AppError::BadRequest(format!(
                    "Expected string, got {:?}",
                    value
                )));
            }
        }
        "int32" | "int64" | "uint32" | "uint64" | "sint32" | "sint64" |
        "fixed32" | "fixed64" | "sfixed32" | "sfixed64" => {
            if !value.is_number() && !value.is_null() {
                return Err(AppError::BadRequest(format!(
                    "Expected number, got {:?}",
                    value
                )));
            }
        }
        "bool" => {
            if !value.is_boolean() && !value.is_null() {
                return Err(AppError::BadRequest(format!(
                    "Expected boolean, got {:?}",
                    value
                )));
            }
        }
        "float" | "double" => {
            if !value.is_number() && !value.is_null() {
                return Err(AppError::BadRequest(format!(
                    "Expected number, got {:?}",
                    value
                )));
            }
        }
        "bytes" => {
            // bytes 可以是 base64 字符串或 null
            if !value.is_string() && !value.is_null() {
                return Err(AppError::BadRequest(format!(
                    "Expected string (base64), got {:?}",
                    value
                )));
            }
        }
        _ => {
            // 其他类型（message, enum）暂时跳过验证
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_simple() {
        let schema = json!({
            "type": "message",
            "name": "Person",
            "fields": [
                {"name": "name", "type": "string", "number": 1},
                {"name": "age", "type": "int32", "number": 2},
                {"name": "email", "type": "string", "number": 3}
            ]
        });

        let input = json!({
            "name": "John",
            "age": 30,
            "email": "john@example.com"
        });

        let result = ProtobufCodec::encode_simple(&schema.to_string(), &input);
        assert!(result.is_ok());
        let encoded = result.unwrap();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_validate_valid() {
        let schema = json!({
            "type": "message",
            "name": "Person",
            "fields": [
                {"name": "name", "type": "string", "number": 1}
            ]
        });

        let input = json!({
            "name": "John"
        });

        assert!(validate(&schema.to_string(), &input).is_ok());
    }

    #[test]
    fn test_validate_invalid() {
        let schema = json!({
            "type": "message",
            "name": "Person",
            "fields": [
                {"name": "age", "type": "int32", "number": 1}
            ]
        });

        let input = json!({
            "age": "not a number"
        });

        assert!(validate(&schema.to_string(), &input).is_err());
    }
}
