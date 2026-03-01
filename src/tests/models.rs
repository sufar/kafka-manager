//! Models 模块单元测试

use crate::models::{
    CreateTopicRequest, SendMessageRequest, MessageRecord,
};

#[test]
fn test_create_topic_request_default_values() {
    // 测试默认分区数和副本数
    let json = r#"{"name": "test-topic"}"#;
    let req: CreateTopicRequest = serde_json::from_str(json).unwrap();

    assert_eq!(req.name, "test-topic");
    assert_eq!(req.num_partitions, 1);
    assert_eq!(req.replication_factor, 1);
    assert!(req.config.is_empty());
}

#[test]
fn test_create_topic_request_with_values() {
    let json = r#"{
        "name": "test-topic",
        "num_partitions": 3,
        "replication_factor": 2,
        "config": {"retention.ms": "86400000"}
    }"#;
    let req: CreateTopicRequest = serde_json::from_str(json).unwrap();

    assert_eq!(req.name, "test-topic");
    assert_eq!(req.num_partitions, 3);
    assert_eq!(req.replication_factor, 2);
    assert_eq!(req.config.get("retention.ms"), Some(&"86400000".to_string()));
}

#[test]
fn test_send_message_request() {
    let json = r#"{
        "key": "test-key",
        "value": "test-value",
        "partition": 1
    }"#;
    let req: SendMessageRequest = serde_json::from_str(json).unwrap();

    assert_eq!(req.key, Some("test-key".to_string()));
    assert_eq!(req.value, "test-value");
    assert_eq!(req.partition, Some(1));
}

#[test]
fn test_send_message_request_optional_fields() {
    let json = r#"{"value": "test-value"}"#;
    let req: SendMessageRequest = serde_json::from_str(json).unwrap();

    assert_eq!(req.key, None);
    assert_eq!(req.value, "test-value");
    assert_eq!(req.partition, None);
}

#[test]
fn test_message_record_serialization() {
    let record = MessageRecord {
        partition: 0,
        offset: 100,
        key: Some("key".to_string()),
        value: Some("value".to_string()),
        timestamp: Some(1234567890),
    };

    let json = serde_json::to_string(&record).unwrap();

    assert!(json.contains("\"partition\":0"));
    assert!(json.contains("\"offset\":100"));
    assert!(json.contains("\"key\":\"key\""));
    assert!(json.contains("\"value\":\"value\""));
}

#[test]
fn test_message_record_with_null_fields() {
    let json = r#"{
        "partition": 0,
        "offset": 100,
        "key": null,
        "value": null,
        "timestamp": null
    }"#;
    let record: MessageRecord = serde_json::from_str(json).unwrap();

    assert_eq!(record.partition, 0);
    assert_eq!(record.offset, 100);
    assert_eq!(record.key, None);
    assert_eq!(record.value, None);
    assert_eq!(record.timestamp, None);
}
