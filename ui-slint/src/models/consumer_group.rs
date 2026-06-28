// Consumer Group 数据模型

use serde::{Deserialize, Serialize};

/// Consumer Group 数据结构（用于 Slint UI）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConsumerGroupData {
    pub name: String,
    pub state: String,  // "Stable", "Empty", "PreparingRebalance", etc.
    pub members_count: i32,
    pub lag: i64,
}

/// Consumer Group 成员信息
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConsumerMember {
    pub member_id: String,
    pub client_id: String,
    pub host: String,
    pub assignments: Vec<MemberAssignment>,
}

/// 成员分配信息
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemberAssignment {
    pub topic: String,
    pub partitions: Vec<i32>,
}

// 从 ConsumerGroupData 转换为 Slint 的 ConsumerGroupInfo
impl From<ConsumerGroupData> for crate::ConsumerGroupInfo {
    fn from(data: ConsumerGroupData) -> Self {
        Self {
            name: slint::SharedString::from(data.name),
            state: slint::SharedString::from(data.state),
            members_count: data.members_count,
            lag: data.lag as i32,
        }
    }
}