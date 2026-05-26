//! Translation struct definitions


/// Main translations container
#[derive(Debug, Clone)]
pub struct Translations {
    pub common: CommonTranslations,
    pub clusters: ClusterTranslations,
    pub topics: TopicTranslations,
    pub messages: MessageTranslations,
    pub consumer_groups: ConsumerGroupTranslations,
    pub settings: SettingsTranslations,
    pub schema_registry: SchemaRegistryTranslations,
    pub layout: LayoutTranslations,
    pub toast: ToastTranslations,
}

impl Translations {
    /// Create Chinese translations
    pub fn zh() -> Self {
        Self {
            common: CommonTranslations::zh(),
            clusters: ClusterTranslations::zh(),
            topics: TopicTranslations::zh(),
            messages: MessageTranslations::zh(),
            consumer_groups: ConsumerGroupTranslations::zh(),
            settings: SettingsTranslations::zh(),
            schema_registry: SchemaRegistryTranslations::zh(),
            layout: LayoutTranslations::zh(),
            toast: ToastTranslations::zh(),
        }
    }

    /// Create English translations
    pub fn en() -> Self {
        Self {
            common: CommonTranslations::en(),
            clusters: ClusterTranslations::en(),
            topics: TopicTranslations::en(),
            messages: MessageTranslations::en(),
            consumer_groups: ConsumerGroupTranslations::en(),
            settings: SettingsTranslations::en(),
            schema_registry: SchemaRegistryTranslations::en(),
            layout: LayoutTranslations::en(),
            toast: ToastTranslations::en(),
        }
    }
}

impl Default for Translations {
    fn default() -> Self {
        Self::zh()
    }
}

/// Common translations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommonTranslations {
    pub back: String,
    pub search: String,
    pub refresh: String,
    pub create: String,
    pub edit: String,
    pub delete: String,
    pub cancel: String,
    pub confirm: String,
    pub save: String,
    pub close: String,
    pub loading: String,
    pub no_data: String,
    pub actions: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub connected: String,
    pub disconnected: String,
    pub unknown: String,
    pub all: String,
    pub success: String,
    pub failed: String,
    pub retry: String,
    pub copy: String,
    pub copied: String,
}

impl CommonTranslations {
    pub fn zh() -> Self {
        Self {
            back: "返回".into(),
            search: "搜索".into(),
            refresh: "刷新".into(),
            create: "创建".into(),
            edit: "编辑".into(),
            delete: "删除".into(),
            cancel: "取消".into(),
            confirm: "确认".into(),
            save: "保存".into(),
            close: "关闭".into(),
            loading: "加载中".into(),
            no_data: "暂无数据".into(),
            actions: "操作".into(),
            name: "名称".into(),
            description: "描述".into(),
            status: "状态".into(),
            connected: "已连接".into(),
            disconnected: "未连接".into(),
            unknown: "未知".into(),
            all: "全部".into(),
            success: "成功".into(),
            failed: "失败".into(),
            retry: "重试".into(),
            copy: "复制".into(),
            copied: "已复制".into(),
        }
    }

    pub fn en() -> Self {
        Self {
            back: "Back".into(),
            search: "Search".into(),
            refresh: "Refresh".into(),
            create: "Create".into(),
            edit: "Edit".into(),
            delete: "Delete".into(),
            cancel: "Cancel".into(),
            confirm: "Confirm".into(),
            save: "Save".into(),
            close: "Close".into(),
            loading: "Loading".into(),
            no_data: "No data".into(),
            actions: "Actions".into(),
            name: "Name".into(),
            description: "Description".into(),
            status: "Status".into(),
            connected: "Connected".into(),
            disconnected: "Disconnected".into(),
            unknown: "Unknown".into(),
            all: "All".into(),
            success: "Success".into(),
            failed: "Failed".into(),
            retry: "Retry".into(),
            copy: "Copy".into(),
            copied: "Copied".into(),
        }
    }
}

/// Cluster translations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClusterTranslations {
    pub title: String,
    pub description: String,
    pub add_cluster: String,
    pub add_group: String,
    pub manage_groups: String,
    pub cluster_name: String,
    pub brokers: String,
    pub request_timeout: String,
    pub operation_timeout: String,
    pub group: String,
    pub test_connection: String,
    pub testing_connection: String,
    pub connection_success: String,
    pub connection_failed: String,
    pub connection_error: String,
    pub connected: String,
    pub create_topic: String,
    pub view_topics_link: String,
    pub disconnect: String,
    pub reconnect: String,
    pub clusters: String,
}

impl ClusterTranslations {
    pub fn zh() -> Self {
        Self {
            title: "集群管理".into(),
            description: "管理 Kafka 集群连接配置".into(),
            add_cluster: "添加集群".into(),
            add_group: "管理分组".into(),
            manage_groups: "管理分组".into(),
            cluster_name: "集群名称".into(),
            brokers: "Broker 地址".into(),
            request_timeout: "请求超时".into(),
            operation_timeout: "操作超时".into(),
            group: "分组".into(),
            test_connection: "测试连接".into(),
            testing_connection: "测试中...".into(),
            connection_success: "连接成功".into(),
            connection_failed: "连接失败".into(),
            connection_error: "连接错误".into(),
            connected: "已连接".into(),
            create_topic: "创建 Topic".into(),
            view_topics_link: "查看 Topics".into(),
            disconnect: "断开".into(),
            reconnect: "重连".into(),
            clusters: "集群".into(),
        }
    }

    pub fn en() -> Self {
        Self {
            title: "Clusters".into(),
            description: "Manage Kafka cluster connections".into(),
            add_cluster: "Add Cluster".into(),
            add_group: "Manage Groups".into(),
            manage_groups: "Manage Groups".into(),
            cluster_name: "Cluster Name".into(),
            brokers: "Brokers".into(),
            request_timeout: "Request Timeout".into(),
            operation_timeout: "Operation Timeout".into(),
            group: "Group".into(),
            test_connection: "Test Connection".into(),
            testing_connection: "Testing...".into(),
            connection_success: "Connection Success".into(),
            connection_failed: "Connection Failed".into(),
            connection_error: "Connection Error".into(),
            connected: "Connected".into(),
            create_topic: "Create Topic".into(),
            view_topics_link: "View Topics".into(),
            disconnect: "Disconnect".into(),
            reconnect: "Reconnect".into(),
            clusters: "Clusters".into(),
        }
    }
}

/// Topic translations
#[derive(Debug, Clone)]
pub struct TopicTranslations {
    pub title: String,
    pub description: String,
    pub topic_name: String,
    pub create_topic: String,
    pub delete_topic: String,
    pub partitions: String,
    pub replication_factor: String,
    pub create: String,
}

impl TopicTranslations {
    pub fn zh() -> Self {
        Self {
            title: "Topic 管理".into(),
            description: "管理 Kafka Topics".into(),
            topic_name: "Topic 名称".into(),
            create_topic: "创建 Topic".into(),
            delete_topic: "删除 Topic".into(),
            partitions: "分区数".into(),
            replication_factor: "副本数".into(),
            create: "创建".into(),
        }
    }

    pub fn en() -> Self {
        Self {
            title: "Topics".into(),
            description: "Manage Kafka Topics".into(),
            topic_name: "Topic Name".into(),
            create_topic: "Create Topic".into(),
            delete_topic: "Delete Topic".into(),
            partitions: "Partitions".into(),
            replication_factor: "Replication Factor".into(),
            create: "Create".into(),
        }
    }
}

/// Message translations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MessageTranslations {
    pub title: String,
    pub description: String,
    pub all_partitions: String,
    pub partition: String,
    pub newest: String,
    pub oldest: String,
    pub max_messages: String,
    pub search_value: String,
    pub value_placeholder: String,
    pub query: String,
    pub stop: String,
    pub send_message: String,
    pub no_messages: String,
    pub loading: String,
    pub partition_label: String,
    pub offset_label: String,
    pub timestamp_label: String,
    pub key: String,
    pub value: String,
    pub message_detail: String,
    pub copy_key: String,
    pub copy_value: String,
    pub json: String,
    pub raw: String,
    pub hex: String,
    pub export_messages: String,
    pub total_messages: String,
    pub messages: String,
    pub receiving: String,
    pub elapsed_time: String,
}

impl MessageTranslations {
    pub fn zh() -> Self {
        Self {
            title: "消息查询".into(),
            description: "查询 Kafka 消息".into(),
            all_partitions: "全部分区".into(),
            partition: "分区".into(),
            newest: "最新".into(),
            oldest: "最早".into(),
            max_messages: "数量".into(),
            search_value: "搜索内容".into(),
            value_placeholder: "搜索消息内容...".into(),
            query: "查询".into(),
            stop: "停止".into(),
            send_message: "发送消息".into(),
            no_messages: "暂无消息".into(),
            loading: "加载中".into(),
            partition_label: "分区".into(),
            offset_label: "Offset".into(),
            timestamp_label: "时间戳".into(),
            key: "Key".into(),
            value: "Value".into(),
            message_detail: "消息详情".into(),
            copy_key: "复制 Key".into(),
            copy_value: "复制 Value".into(),
            json: "JSON".into(),
            raw: "原始".into(),
            hex: "十六进制".into(),
            export_messages: "导出消息".into(),
            total_messages: "共".into(),
            messages: "条".into(),
            receiving: "接收中".into(),
            elapsed_time: "耗时".into(),
        }
    }

    pub fn en() -> Self {
        Self {
            title: "Messages".into(),
            description: "Query Kafka messages".into(),
            all_partitions: "All Partitions".into(),
            partition: "Partition".into(),
            newest: "Newest".into(),
            oldest: "Oldest".into(),
            max_messages: "Count".into(),
            search_value: "Search".into(),
            value_placeholder: "Search message content...".into(),
            query: "Query".into(),
            stop: "Stop".into(),
            send_message: "Send Message".into(),
            no_messages: "No messages".into(),
            loading: "Loading".into(),
            partition_label: "Partition".into(),
            offset_label: "Offset".into(),
            timestamp_label: "Timestamp".into(),
            key: "Key".into(),
            value: "Value".into(),
            message_detail: "Message Detail".into(),
            copy_key: "Copy Key".into(),
            copy_value: "Copy Value".into(),
            json: "JSON".into(),
            raw: "Raw".into(),
            hex: "Hex".into(),
            export_messages: "Export".into(),
            total_messages: "Total".into(),
            messages: "messages".into(),
            receiving: "Receiving".into(),
            elapsed_time: "Elapsed".into(),
        }
    }
}

/// Consumer Group translations
#[derive(Debug, Clone)]
pub struct ConsumerGroupTranslations {
    pub title: String,
    pub description: String,
    pub clusters: String,
    pub groupNamePrefix: String,
    pub resetOffset: String,
    pub deleteGroup: String,
}

impl ConsumerGroupTranslations {
    pub fn zh() -> Self {
        Self {
            title: "消费者组".into(),
            description: "管理 Kafka 消费者组".into(),
            clusters: "集群".into(),
            groupNamePrefix: "组名前缀".into(),
            resetOffset: "重置偏移".into(),
            deleteGroup: "删除组".into(),
        }
    }
    pub fn en() -> Self {
        Self {
            title: "Consumer Groups".into(),
            description: "Manage Kafka Consumer Groups".into(),
            clusters: "Clusters".into(),
            groupNamePrefix: "Group Prefix".into(),
            resetOffset: "Reset Offset".into(),
            deleteGroup: "Delete Group".into(),
        }
    }
}

/// Settings translations (placeholder)
#[derive(Debug, Clone)]
pub struct SettingsTranslations {
    pub title: String,
}

impl SettingsTranslations {
    pub fn zh() -> Self {
        Self { title: "设置".into() }
    }
    pub fn en() -> Self {
        Self { title: "Settings".into() }
    }
}

/// Schema Registry translations (placeholder)
#[derive(Debug, Clone)]
pub struct SchemaRegistryTranslations {
    pub title: String,
}

impl SchemaRegistryTranslations {
    pub fn zh() -> Self {
        Self { title: "Schema Registry".into() }
    }
    pub fn en() -> Self {
        Self { title: "Schema Registry".into() }
    }
}

/// Layout translations
#[derive(Debug, Clone)]
pub struct LayoutTranslations {
    pub sidebar_toggle: String,
    pub search_placeholder: String,
    pub language_toggle: String,
    pub theme_toggle: String,
    pub settings: String,
}

impl LayoutTranslations {
    pub fn zh() -> Self {
        Self {
            sidebar_toggle: "菜单".into(),
            search_placeholder: "搜索 Topics...".into(),
            language_toggle: "语言".into(),
            theme_toggle: "主题".into(),
            settings: "设置".into(),
        }
    }
    pub fn en() -> Self {
        Self {
            sidebar_toggle: "Menu".into(),
            search_placeholder: "Search Topics...".into(),
            language_toggle: "Language".into(),
            theme_toggle: "Theme".into(),
            settings: "Settings".into(),
        }
    }
}

/// Toast translations
#[derive(Debug, Clone)]
pub struct ToastTranslations {
    pub operation_success: String,
    pub operation_failed: String,
}

impl ToastTranslations {
    pub fn zh() -> Self {
        Self {
            operation_success: "操作成功".into(),
            operation_failed: "操作失败".into(),
        }
    }
    pub fn en() -> Self {
        Self {
            operation_success: "Operation successful".into(),
            operation_failed: "Operation failed".into(),
        }
    }
}