// 事件处理器模块

pub mod cluster;
pub mod topic;
pub mod topic_crud;
pub mod message;
pub mod consumer_group;
pub mod consumer_group_detail;
pub mod settings;
pub mod phase11;  // Phase 11 Week 5-6新增handlers

// 重新导出所有处理器
pub use cluster::*;
pub use topic::*;
pub use topic_crud::*;
pub use message::*;
pub use consumer_group::*;
pub use consumer_group_detail::*;
pub use settings::*;
pub use phase11::*;  // 导出Phase 11新增handlers