// 消息数据模型

use serde::{Deserialize, Serialize};

/// 消息数据结构（用于 Slint UI）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MessageData {
    pub partition: i32,
    pub offset: i64,
    pub key: String,
    pub value: String,
    pub timestamp: i64,
    pub headers: Vec<MessageHeader>,
}

/// 消息头
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MessageHeader {
    pub key: String,
    pub value: String,
}