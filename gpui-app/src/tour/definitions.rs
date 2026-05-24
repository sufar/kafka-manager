//! Tour Definitions
//!
//! Defines tour steps for different pages.

/// Tour step position
#[derive(Debug, Clone)]
pub struct TourPosition {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Single tour step
#[derive(Debug, Clone)]
pub struct TourStep {
    /// Target element selector (data-tour attribute)
    pub target: String,
    /// Position for spotlight
    pub position: TourPosition,
    /// Step title
    pub title: String,
    /// Step description
    pub description: String,
}

/// Tour definition for a page
#[derive(Debug, Clone)]
pub struct TourDefinition {
    pub page: String,
    pub steps: Vec<TourStep>,
}

impl TourDefinition {
    /// Get tour for clusters page
    pub fn clusters() -> Self {
        Self {
            page: "clusters".to_string(),
            steps: vec![
                TourStep {
                    target: "add-cluster-btn".to_string(),
                    position: TourPosition { x: 100, y: 100, width: 120, height: 40 },
                    title: "添加集群".to_string(),
                    description: "点击这里添加新的 Kafka 集群配置".to_string(),
                },
                TourStep {
                    target: "cluster-card".to_string(),
                    position: TourPosition { x: 100, y: 200, width: 300, height: 200 },
                    title: "集群卡片".to_string(),
                    description: "每个集群显示为卡片，包含名称、Brokers 地址和连接状态".to_string(),
                },
                TourStep {
                    target: "sidebar".to_string(),
                    position: TourPosition { x: 0, y: 48, width: 200, height: 400 },
                    title: "侧边栏导航".to_string(),
                    description: "使用侧边栏快速切换不同页面".to_string(),
                },
            ],
        }
    }

    /// Get tour for messages page
    pub fn messages() -> Self {
        Self {
            page: "messages".to_string(),
            steps: vec![
                TourStep {
                    target: "query-toolbar".to_string(),
                    position: TourPosition { x: 200, y: 100, width: 600, height: 50 },
                    title: "查询工具栏".to_string(),
                    description: "选择分区、查询模式、消息数量等参数".to_string(),
                },
                TourStep {
                    target: "message-list".to_string(),
                    position: TourPosition { x: 200, y: 200, width: 800, height: 300 },
                    title: "消息列表".to_string(),
                    description: "实时显示从 Kafka 接收的消息流".to_string(),
                },
                TourStep {
                    target: "send-message-btn".to_string(),
                    position: TourPosition { x: 700, y: 100, width: 100, height: 40 },
                    title: "发送消息".to_string(),
                    description: "可以手动发送消息到 Kafka Topic".to_string(),
                },
            ],
        }
    }

    /// Get all available tours
    pub fn all() -> Vec<TourDefinition> {
        vec![
            Self::clusters(),
            Self::messages(),
        ]
    }

    /// Get default tour steps for initial onboarding
    pub fn default_steps() -> Vec<TourStep> {
        vec![
            TourStep {
                target: "sidebar".to_string(),
                position: TourPosition { x: 0, y: 48, width: 200, height: 400 },
                title: "导航侧边栏".to_string(),
                description: "使用侧边栏快速切换不同页面：集群管理、Topic 列表、消息查询等".to_string(),
            },
            TourStep {
                target: "main-content".to_string(),
                position: TourPosition { x: 200, y: 100, width: 800, height: 500 },
                title: "主内容区".to_string(),
                description: "当前页面的内容会在这里显示".to_string(),
            },
            TourStep {
                target: "add-cluster-btn".to_string(),
                position: TourPosition { x: 100, y: 100, width: 120, height: 40 },
                title: "添加集群".to_string(),
                description: "点击这里添加新的 Kafka 集群配置".to_string(),
            },
        ]
    }
}