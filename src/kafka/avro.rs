/// Avro 编解码器
/// 使用 apache-avro crate 进行 Avro 消息的序列化和反序列化

use crate::error::{AppError, Result};
use apache_avro::{types::Value, Reader, Schema, Writer};
use serde_json;

pub struct AvroCodec;

impl AvroCodec {
    /// 将 JSON 编码为 Avro 二进制（发送消息时使用）
    pub fn encode(schema_str: &str, json_value: &serde_json::Value) -> Result<Vec<u8>> {
        // 解析 Schema
        let schema = Schema::parse_str(schema_str)
            .map_err(|e| AppError::BadRequest(format!("Invalid Avro schema: {}", e)))?;

        // 使用 Writer 编码
        let mut writer = Writer::new(&schema, Vec::new());

        // 直接使用 Value::from 将 JSON 转换为 Avro Value
        let avro_value = serde_json_to_avro(json_value, &schema)?;

        writer
            .append(avro_value)
            .map_err(|e| AppError::Internal(format!("Failed to append Avro value: {}", e)))?;

        writer
            .flush()
            .map_err(|e| AppError::Internal(format!("Failed to flush Avro writer: {}", e)))?;

        Ok(writer.into_inner().map_err(|e| AppError::Avro(e))?)
    }

    /// 将 Avro 二进制解码为 JSON（读取消息时使用）
    pub fn decode(schema_str: &str, avro_bytes: &[u8]) -> Result<serde_json::Value> {
        // 解析 Schema
        let schema = Schema::parse_str(schema_str)
            .map_err(|e| AppError::BadRequest(format!("Invalid Avro schema: {}", e)))?;

        // 使用 Reader 解码
        let reader = Reader::with_schema(&schema, avro_bytes)
            .map_err(|e| AppError::BadRequest(format!("Failed to read Avro data: {}", e)))?;

        // 获取第一条记录（假设单条消息）
        let mut values: Vec<serde_json::Value> = Vec::new();
        for value in reader {
            let avro_value = value
                .map_err(|e| AppError::Internal(format!("Failed to read Avro value: {}", e)))?;
            let json_value = avro_to_json(avro_value);
            values.push(json_value);
        }

        // 如果只有一条记录，直接返回；否则返回数组
        if values.len() == 1 {
            Ok(values.remove(0))
        } else {
            Ok(serde_json::Value::Array(values))
        }
    }

    /// 验证数据是否符合 Schema
    pub fn validate(schema_str: &str, json_value: &serde_json::Value) -> Result<()> {
        let schema = Schema::parse_str(schema_str)
            .map_err(|e| AppError::BadRequest(format!("Invalid Avro schema: {}", e)))?;

        serde_json_to_avro(json_value, &schema)
            .map_err(|e| AppError::BadRequest(format!("Data does not match schema: {}", e)))?;

        Ok(())
    }
}

/// 将 JSON Value 转换为 Avro Value（简化版本）
fn serde_json_to_avro(json: &serde_json::Value, schema: &Schema) -> Result<Value> {
    match (json, schema) {
        (serde_json::Value::Null, _) => Ok(Value::Null),
        (serde_json::Value::Bool(b), Schema::Boolean) => Ok(Value::Boolean(*b)),
        (serde_json::Value::Number(n), Schema::Int) => {
            n.as_i64()
                .map(|i| Value::Int(i as i32))
                .ok_or_else(|| AppError::BadRequest("Cannot convert number to Int".to_string()))
        }
        (serde_json::Value::Number(n), Schema::Long) => {
            n.as_i64()
                .map(Value::Long)
                .ok_or_else(|| AppError::BadRequest("Cannot convert number to Long".to_string()))
        }
        (serde_json::Value::Number(n), Schema::Float) => {
            n.as_f64()
                .map(|f| Value::Float(f as f32))
                .ok_or_else(|| AppError::BadRequest("Cannot convert number to Float".to_string()))
        }
        (serde_json::Value::Number(n), Schema::Double) => {
            n.as_f64()
                .map(Value::Double)
                .ok_or_else(|| AppError::BadRequest("Cannot convert number to Double".to_string()))
        }
        (serde_json::Value::String(s), Schema::String) => Ok(Value::String(s.clone())),
        (serde_json::Value::Array(arr), Schema::Array(array_schema)) => {
            let values: Result<Vec<Value>> = arr
                .iter()
                .map(|v| serde_json_to_avro(v, &array_schema.items))
                .collect();
            Ok(Value::Array(values?))
        }
        (serde_json::Value::Object(map), Schema::Map(map_schema)) => {
            let values: Result<std::collections::HashMap<String, Value>> = map
                .iter()
                .map(|(k, v)| serde_json_to_avro(v, &map_schema.types).map(|avro_val| (k.clone(), avro_val)))
                .collect();
            Ok(Value::Map(values?))
        }
        (serde_json::Value::Object(map), Schema::Record(record_schema)) => {
            let mut avro_fields = Vec::new();
            for field in &record_schema.fields {
                let key = &field.name;
                let value = map.get(key).unwrap_or(&serde_json::Value::Null);
                let avro_value = serde_json_to_avro(value, &field.schema)?;
                avro_fields.push((key.clone(), avro_value));
            }
            Ok(Value::Record(avro_fields))
        }
        // 处理 Union 类型
        (_, Schema::Union(union_schema)) => {
            for (i, variant) in union_schema.variants().iter().enumerate() {
                if let Ok(avro_value) = serde_json_to_avro(json, variant) {
                    return Ok(Value::Union(i as u32, Box::new(avro_value)));
                }
            }
            Err(AppError::BadRequest(
                "No matching union variant found".to_string()
            ))
        }
        // 对于 Enum 和 Fixed，尝试作为字符串处理
        (serde_json::Value::String(s), Schema::Enum(_)) => Ok(Value::String(s.clone())),
        (serde_json::Value::String(s), Schema::Fixed(_)) => Ok(Value::String(s.clone())),
        _ => Err(AppError::BadRequest(format!(
            "Type mismatch: JSON {:?} cannot be converted to Schema {:?}",
            json, schema
        ))),
    }
}

/// 将 Avro Value 转换为 JSON Value
fn avro_to_json(avro: Value) -> serde_json::Value {
    match avro {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(b),
        Value::Int(i) => serde_json::json!(i),
        Value::Long(l) => serde_json::json!(l),
        Value::Float(f) => serde_json::json!(f),
        Value::Double(d) => serde_json::json!(d),
        Value::Bytes(bytes) => {
            serde_json::Value::String(base64::encode(&bytes))
        }
        Value::String(s) => serde_json::Value::String(s),
        Value::Fixed(_, bytes) => {
            serde_json::Value::String(base64::encode(&bytes))
        }
        Value::Enum(_index, symbol) => serde_json::Value::String(symbol),
        Value::Union(_index, inner) => avro_to_json(*inner),
        Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(avro_to_json).collect())
        }
        Value::Map(map) => {
            serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, avro_to_json(v)))
                    .collect(),
            )
        }
        Value::Record(fields) => {
            serde_json::Value::Object(
                fields
                    .into_iter()
                    .map(|(k, v)| (k, avro_to_json(v)))
                    .collect(),
            )
        }
        Value::Decimal(val) => {
            serde_json::Value::String(format!("{:?}", val))
        }
        Value::Date(d) => serde_json::json!(d),
        Value::TimeMillis(t) => serde_json::json!(t),
        Value::TimeMicros(t) => serde_json::json!(t),
        Value::TimestampMillis(t) => serde_json::json!(t),
        Value::TimestampMicros(t) => serde_json::json!(t),
        Value::TimestampNanos(t) => serde_json::json!(t),
        Value::LocalTimestampMillis(t) => serde_json::json!(t),
        Value::LocalTimestampMicros(t) => serde_json::json!(t),
        Value::LocalTimestampNanos(t) => serde_json::json!(t),
        Value::Uuid(u) => serde_json::Value::String(u.to_string()),
        Value::Duration(d) => serde_json::json!({
            "months": u32::from(d.months()),
            "days": u32::from(d.days()),
            "millis": u32::from(d.millis())
        }),
        // 处理 BigDecimal
        Value::BigDecimal(val) => {
            serde_json::Value::String(val.to_string())
        }
        // 处理其他未覆盖的类型，返回 null
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_decode_simple() {
        let schema = r#"{
            "type": "record",
            "name": "TestRecord",
            "fields": [
                {"name": "name", "type": "string"},
                {"name": "age", "type": "int"}
            ]
        }"#;

        let input = json!({
            "name": "John",
            "age": 30
        });

        let encoded = AvroCodec::encode(schema, &input).unwrap();
        let decoded = AvroCodec::decode(schema, &encoded).unwrap();

        assert_eq!(decoded, input);
    }

    #[test]
    fn test_validate_valid() {
        let schema = r#"{
            "type": "record",
            "name": "TestRecord",
            "fields": [
                {"name": "name", "type": "string"}
            ]
        }"#;

        let input = json!({
            "name": "John"
        });

        assert!(AvroCodec::validate(schema, &input).is_ok());
    }

    #[test]
    fn test_validate_invalid() {
        let schema = r#"{
            "type": "record",
            "name": "TestRecord",
            "fields": [
                {"name": "name", "type": "string"},
                {"name": "age", "type": "int"}
            ]
        }"#;

        let input = json!({
            "name": "John"
        });

        // 缺少必需字段 age
        assert!(AvroCodec::validate(schema, &input).is_err());
    }
}
