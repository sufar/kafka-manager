//! API Module
//!
//! HTTP client and types for communicating with the backend.

mod client;
mod types;
mod sse;
mod sse_handler;

pub use client::{
    ApiClient, HttpClient, ApiError,
    ClusterApi, TopicApi, MessageApi, ConsumerGroupApi, SchemaRegistryApi,
    ConnectionTestResult, TopicConfigEntry, SendMessageResult, SchemaRegistrationResult,
};
pub use types::{
    ClusterResponse, CreateClusterRequest, UpdateClusterRequest,
    ClusterGroupResponse, CreateClusterGroupRequest,
    TopicResponse, CreateTopicRequest, PartitionInfo,
    MessageQueryRequest, KafkaMessage, SendMessageRequest,
    ConsumerGroupResponse, ConsumerGroupMember, TopicPartitionAssignment,
    SchemaSubject, SchemaVersion, RegisterSchemaRequest,
};
pub use sse::{StreamEvent, StreamMessage, StreamConfig, MessageQueryState, QueryMode, parse_sse_event, MessageStream};
pub use sse_handler::SseStreamHandler;
