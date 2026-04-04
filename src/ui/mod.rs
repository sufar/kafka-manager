// Kafka Manager UI Module
// Slint-based user interface

slint::include_modules!();

/// 设置集群列表
pub fn set_cluster_names(ui: &MainWindow, clusters: Vec<String>) {
    let model = slint::VecModel::from(
        clusters
            .into_iter()
            .map(|s| slint::SharedString::from(s))
            .collect::<Vec<_>>()
    );
    ui.set_cluster_names(slint::ModelRc::new(model));
}

/// 设置 Topic 列表
pub fn set_topic_names(ui: &MainWindow, topics: Vec<String>) {
    let model = slint::VecModel::from(
        topics
            .into_iter()
            .map(|s| slint::SharedString::from(s))
            .collect::<Vec<_>>()
    );
    ui.set_topic_names(slint::ModelRc::new(model));
}

/// 设置 Consumer Group 列表
pub fn set_consumer_group_names(ui: &MainWindow, groups: Vec<String>) {
    let model = slint::VecModel::from(
        groups
            .into_iter()
            .map(|s| slint::SharedString::from(s))
            .collect::<Vec<_>>()
    );
    ui.set_consumer_group_names(slint::ModelRc::new(model));
}

/// 设置 Consumer Group 详情
pub fn set_consumer_group_detail(
    ui: &MainWindow,
    name: &str,
    state: &str,
    members: usize,
    topics: &str,
    total_lag: u64,
    partition_details: Vec<String>,
) {
    ui.set_consumer_group_detail_name(name.into());
    ui.set_consumer_group_detail_state(state.into());
    ui.set_consumer_group_detail_members(members.to_string().into());
    ui.set_consumer_group_detail_topics(topics.into());
    ui.set_consumer_group_detail_total_lag(format_number(total_lag).into());

    // 设置分区详情列表
    let model = slint::VecModel::from(
        partition_details
            .into_iter()
            .map(|s| slint::SharedString::from(s))
            .collect::<Vec<_>>()
    );
    ui.set_consumer_group_detail_partition_list(slint::ModelRc::new(model));
}

/// 设置 Topic 详情
pub fn set_topic_detail(
    ui: &MainWindow,
    name: &str,
    partitions: usize,
    replication: usize,
    status: &str,
    partition_details: Vec<String>,
    message_count: u64,
    size_bytes: u64,
) {
    ui.set_topic_detail_name(name.into());
    ui.set_topic_detail_partitions(partitions.to_string().into());
    ui.set_topic_detail_replication(replication.to_string().into());
    ui.set_topic_detail_status(status.into());

    // 设置分区详情列表
    let model = slint::VecModel::from(
        partition_details
            .into_iter()
            .map(|s| slint::SharedString::from(s))
            .collect::<Vec<_>>()
    );
    ui.set_topic_detail_partition_list(slint::ModelRc::new(model));

    // 设置统计信息
    ui.set_topic_detail_message_count(format_number(message_count).into());
    ui.set_topic_detail_size(format_size(size_bytes).into());
}

/// 格式化数字（添加千位分隔符）
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// 格式化字节大小为人类可读格式
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 设置加载状态
pub fn set_loading(ui: &MainWindow, loading: bool, message: &str) {
    ui.set_is_loading(loading);
    ui.set_loading_message(message.into());
}

/// 设置消息列表
pub fn set_view_messages(ui: &MainWindow, messages: Vec<String>) {
    let model = slint::VecModel::from(
        messages
            .into_iter()
            .map(|s| slint::SharedString::from(s))
            .collect::<Vec<_>>()
    );
    ui.set_view_messages_list(slint::ModelRc::new(model));
}

/// 运行 UI
pub fn run(ui: MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    ui.run()?;
    Ok(())
}
