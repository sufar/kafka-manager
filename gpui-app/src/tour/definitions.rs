//! Tour Definitions
//!
//! Defines tour steps for different pages with dynamic positioning support.

use serde::{Deserialize, Serialize};

/// Tour step position (can be static or dynamic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourPosition {
    /// Static X position (used when element not found)
    pub x: i32,
    /// Static Y position (used when element not found)
    pub y: i32,
    /// Width of spotlight
    pub width: i32,
    /// Height of spotlight
    pub height: i32,
    /// Optional element selector (data-tour attribute)
    pub target_selector: Option<String>,
    /// Position relative to target element
    pub placement: TourPlacement,
}

impl Default for TourPosition {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 200,
            height: 50,
            target_selector: None,
            placement: TourPlacement::Bottom,
        }
    }
}

/// Placement relative to target element
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TourPlacement {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

impl TourPlacement {
    /// Calculate offset for tooltip position
    pub fn tooltip_offset(&self) -> (i32, i32) {
        match self {
            TourPlacement::Top => (0, -100),
            TourPlacement::Bottom => (0, 50),
            TourPlacement::Left => (-350, 0),
            TourPlacement::Right => (50, 0),
            TourPlacement::Center => (0, 80),
        }
    }
}

/// Tour step priority
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TourStepPriority {
    Required,
    Optional,
    Info,
}

impl Default for TourStepPriority {
    fn default() -> Self {
        Self::Info
    }
}

/// Single tour step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourStep {
    /// Unique step ID
    pub id: String,
    /// Target element selector (data-tour attribute)
    pub target: String,
    /// Position for spotlight (static fallback)
    pub position: TourPosition,
    /// Step title
    pub title: String,
    /// Step description
    pub description: String,
    /// Step priority
    pub priority: TourStepPriority,
    /// Skip allowed
    pub skip_allowed: bool,
    /// Expected action (optional hint)
    pub expected_action: Option<String>,
}

impl TourStep {
    /// Create new tour step with simple position
    pub fn new(id: String, target: String, title: String, description: String, x: i32, y: i32, width: i32, height: i32) -> Self {
        let target_selector = target.clone();
        Self {
            id,
            target,
            position: TourPosition {
                x, y, width, height,
                target_selector: Some(target_selector),
                placement: TourPlacement::Bottom,
            },
            title,
            description,
            priority: TourStepPriority::Info,
            skip_allowed: true,
            expected_action: None,
        }
    }

    /// Create step with placement
    pub fn with_placement(mut self, placement: TourPlacement) -> Self {
        self.position.placement = placement;
        self
    }

    /// Create required step (cannot skip)
    pub fn required(mut self) -> Self {
        self.priority = TourStepPriority::Required;
        self.skip_allowed = false;
        self
    }

    /// Set expected action hint
    pub fn with_action(mut self, action: &str) -> Self {
        self.expected_action = Some(action.to_string());
        self
    }
}

/// Tour progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourProgress {
    /// Tour ID
    pub tour_id: String,
    /// Completed step IDs
    pub completed_steps: Vec<String>,
    /// Current step index
    pub current_step: usize,
    /// Tour started timestamp
    pub started_at: i64,
    /// Tour completed timestamp
    pub completed_at: Option<i64>,
    /// Skipped step IDs
    pub skipped_steps: Vec<String>,
}

impl TourProgress {
    /// Create new progress tracker
    pub fn new(tour_id: String) -> Self {
        Self {
            tour_id,
            completed_steps: Vec::new(),
            current_step: 0,
            started_at: chrono::Utc::now().timestamp_millis(),
            completed_at: None,
            skipped_steps: Vec::new(),
        }
    }

    /// Mark step as completed
    pub fn complete_step(&mut self, step_id: &str) {
        if !self.completed_steps.contains(&step_id.to_string()) {
            self.completed_steps.push(step_id.to_string());
        }
        self.current_step += 1;
    }

    /// Mark step as skipped
    pub fn skip_step(&mut self, step_id: &str) {
        if !self.skipped_steps.contains(&step_id.to_string()) {
            self.skipped_steps.push(step_id.to_string());
        }
        self.current_step += 1;
    }

    /// Check if tour is complete
    pub fn is_complete(&self, total_steps: usize) -> bool {
        self.current_step >= total_steps || self.completed_at.is_some()
    }

    /// Mark tour as complete
    pub fn finish(&mut self) {
        self.completed_at = Some(chrono::Utc::now().timestamp_millis());
    }

    /// Get completion percentage
    pub fn completion_percentage(&self, total_steps: usize) -> f32 {
        if total_steps == 0 {
            return 100.0;
        }
        (self.completed_steps.len() as f32 / total_steps as f32) * 100.0
    }

    /// Serialize to JSON for storage
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Tour definition for a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourDefinition {
    /// Tour ID
    pub id: String,
    /// Page route
    pub page: String,
    /// Tour name
    pub name: String,
    /// Tour description
    pub description: String,
    /// Tour steps
    pub steps: Vec<TourStep>,
    /// Auto-start on page load
    pub auto_start: bool,
}

impl TourDefinition {
    /// Get tour for clusters page
    pub fn clusters() -> Self {
        Self {
            id: "tour-clusters".to_string(),
            page: "clusters".to_string(),
            name: "集群管理引导".to_string(),
            description: "了解如何管理 Kafka 集群".to_string(),
            auto_start: false,
            steps: vec![
                TourStep::new(
                    "clusters-add".to_string(),
                    "add-cluster-btn".to_string(),
                    "添加集群".to_string(),
                    "点击这里添加新的 Kafka 集群配置，填写名称、Brokers 地址等信息".to_string(),
                    100, 100, 120, 40,
                ).with_action("点击添加按钮"),
                TourStep::new(
                    "clusters-card".to_string(),
                    "cluster-card".to_string(),
                    "集群卡片".to_string(),
                    "每个集群显示为卡片，包含名称、Brokers 地址和连接状态".to_string(),
                    100, 200, 300, 200,
                ).with_placement(TourPlacement::Right),
                TourStep::new(
                    "clusters-test".to_string(),
                    "test-connection-btn".to_string(),
                    "测试连接".to_string(),
                    "添加集群前可以先测试连接是否成功".to_string(),
                    400, 150, 100, 40,
                ),
                TourStep::new(
                    "clusters-sidebar".to_string(),
                    "sidebar".to_string(),
                    "侧边栏导航".to_string(),
                    "使用侧边栏快速切换不同页面".to_string(),
                    0, 48, 200, 400,
                ).with_placement(TourPlacement::Right),
            ],
        }
    }

    /// Get tour for messages page
    pub fn messages() -> Self {
        Self {
            id: "tour-messages".to_string(),
            page: "messages".to_string(),
            name: "消息查询引导".to_string(),
            description: "了解如何查询和发送 Kafka 消息".to_string(),
            auto_start: false,
            steps: vec![
                TourStep::new(
                    "messages-toolbar".to_string(),
                    "query-toolbar".to_string(),
                    "查询工具栏".to_string(),
                    "选择分区、查询模式、消息数量等参数".to_string(),
                    200, 100, 600, 50,
                ).with_action("配置查询参数"),
                TourStep::new(
                    "messages-partition".to_string(),
                    "partition-select".to_string(),
                    "分区选择".to_string(),
                    "选择要查询的特定分区，或选择全部分区".to_string(),
                    220, 100, 100, 40,
                ),
                TourStep::new(
                    "messages-mode".to_string(),
                    "query-mode".to_string(),
                    "查询模式".to_string(),
                    "选择最新消息或最早消息模式".to_string(),
                    350, 100, 80, 40,
                ),
                TourStep::new(
                    "messages-list".to_string(),
                    "message-list".to_string(),
                    "消息列表".to_string(),
                    "实时显示从 Kafka 接收的消息流，支持虚拟滚动".to_string(),
                    200, 200, 800, 300,
                ).with_placement(TourPlacement::Bottom),
                TourStep::new(
                    "messages-detail".to_string(),
                    "message-detail-panel".to_string(),
                    "消息详情".to_string(),
                    "点击消息查看详细内容，支持 JSON 格式化显示".to_string(),
                    200, 500, 800, 180,
                ),
                TourStep::new(
                    "messages-send".to_string(),
                    "send-message-btn".to_string(),
                    "发送消息".to_string(),
                    "可以手动发送消息到 Kafka Topic".to_string(),
                    700, 100, 100, 40,
                ).with_action("点击发送按钮"),
            ],
        }
    }

    /// Get tour for topics page
    pub fn topics() -> Self {
        Self {
            id: "tour-topics".to_string(),
            page: "topics".to_string(),
            name: "Topic管理引导".to_string(),
            description: "了解如何管理 Kafka Topic".to_string(),
            auto_start: false,
            steps: vec![
                TourStep::new(
                    "topics-tree".to_string(),
                    "topic-tree".to_string(),
                    "Topic树形结构".to_string(),
                    "展开集群查看所有 Topic，点击 Topic 查看详情".to_string(),
                    0, 48, 200, 400,
                ).with_placement(TourPlacement::Right),
                TourStep::new(
                    "topics-create".to_string(),
                    "create-topic-btn".to_string(),
                    "创建Topic".to_string(),
                    "点击创建新的 Topic，设置分区数和副本数".to_string(),
                    100, 50, 120, 40,
                ),
                TourStep::new(
                    "topics-search".to_string(),
                    "topic-search".to_string(),
                    "搜索Topic".to_string(),
                    "输入关键词快速搜索 Topic 名称".to_string(),
                    200, 50, 200, 40,
                ),
            ],
        }
    }

    /// Get tour for settings page
    pub fn settings() -> Self {
        Self {
            id: "tour-settings".to_string(),
            page: "settings".to_string(),
            name: "设置引导".to_string(),
            description: "了解应用设置选项".to_string(),
            auto_start: false,
            steps: vec![
                TourStep::new(
                    "settings-language".to_string(),
                    "language-selector".to_string(),
                    "语言设置".to_string(),
                    "切换中英文界面语言".to_string(),
                    100, 100, 150, 40,
                ),
                TourStep::new(
                    "settings-theme".to_string(),
                    "theme-toggle".to_string(),
                    "主题设置".to_string(),
                    "切换深色/浅色主题".to_string(),
                    200, 100, 150, 40,
                ),
                TourStep::new(
                    "settings-export".to_string(),
                    "export-btn".to_string(),
                    "数据导出".to_string(),
                    "导出集群配置、收藏和历史记录".to_string(),
                    100, 300, 100, 40,
                ),
            ],
        }
    }

    /// Get all available tours
    pub fn all() -> Vec<TourDefinition> {
        vec![
            Self::clusters(),
            Self::messages(),
            Self::topics(),
            Self::settings(),
        ]
    }

    /// Get tour by page
    pub fn for_page(page: &str) -> Option<TourDefinition> {
        Self::all().into_iter().find(|t| t.page == page)
    }

    /// Get default tour steps for initial onboarding
    pub fn default_steps() -> Vec<TourStep> {
        vec![
            TourStep::new(
                "welcome-sidebar".to_string(),
                "sidebar".to_string(),
                "导航侧边栏".to_string(),
                "使用侧边栏快速切换不同页面：集群管理、Topic 列表、消息查询等".to_string(),
                0, 48, 200, 400,
            ).with_placement(TourPlacement::Right),
            TourStep::new(
                "welcome-content".to_string(),
                "main-content".to_string(),
                "主内容区".to_string(),
                "当前页面的内容会在这里显示".to_string(),
                200, 100, 800, 500,
            ).with_placement(TourPlacement::Center),
            TourStep::new(
                "welcome-add".to_string(),
                "add-cluster-btn".to_string(),
                "添加集群".to_string(),
                "点击这里添加新的 Kafka 集群配置".to_string(),
                100, 100, 120, 40,
            ).with_action("点击添加按钮"),
        ]
    }

    /// Get total steps count
    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    /// Get step by index
    pub fn step_at(&self, index: usize) -> Option<&TourStep> {
        self.steps.get(index)
    }

    /// Check if step exists
    pub fn has_step(&self, step_id: &str) -> bool {
        self.steps.iter().any(|s| s.id == step_id)
    }
}

/// Tour manager for handling multiple tours
pub struct TourManager {
    tours: Vec<TourDefinition>,
    progress: HashMap<String, TourProgress>,
}

use std::collections::HashMap;

impl TourManager {
    /// Create new tour manager
    pub fn new() -> Self {
        Self {
            tours: TourDefinition::all(),
            progress: HashMap::new(),
        }
    }

    /// Get tour by ID
    pub fn get_tour(&self, tour_id: &str) -> Option<&TourDefinition> {
        self.tours.iter().find(|t| t.id == tour_id)
    }

    /// Get tour for page
    pub fn get_tour_for_page(&self, page: &str) -> Option<TourDefinition> {
        TourDefinition::for_page(page)
    }

    /// Start a tour
    pub fn start_tour(&mut self, tour_id: &str) -> Option<&TourProgress> {
        if let Some(_) = self.get_tour(tour_id) {
            let progress = TourProgress::new(tour_id.to_string());
            self.progress.insert(tour_id.to_string(), progress);
            self.progress.get(tour_id)
        } else {
            None
        }
    }

    /// Get progress for tour
    pub fn get_progress(&self, tour_id: &str) -> Option<&TourProgress> {
        self.progress.get(tour_id)
    }

    /// Complete step
    pub fn complete_step(&mut self, tour_id: &str, step_id: &str) {
        if let Some(progress) = self.progress.get_mut(tour_id) {
            progress.complete_step(step_id);
        }
    }

    /// Skip step
    pub fn skip_step(&mut self, tour_id: &str, step_id: &str) -> bool {
        if let Some(tour) = self.get_tour(tour_id) {
            if let Some(step) = tour.steps.iter().find(|s| s.id == step_id) {
                if step.skip_allowed {
                    if let Some(progress) = self.progress.get_mut(tour_id) {
                        progress.skip_step(step_id);
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Check if tour is complete
    pub fn is_tour_complete(&self, tour_id: &str) -> bool {
        if let Some(tour) = self.get_tour(tour_id) {
            if let Some(progress) = self.progress.get(tour_id) {
                return progress.is_complete(tour.total_steps());
            }
        }
        false
    }

    /// Get all completed tours
    pub fn completed_tours(&self) -> Vec<String> {
        self.progress.iter()
            .filter(|(_, p)| p.completed_at.is_some())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Save all progress to JSON
    pub fn save_progress(&self) -> Result<String, serde_json::Error> {
        let progress_map: HashMap<String, TourProgress> = self.progress.clone();
        serde_json::to_string(&progress_map)
    }

    /// Load progress from JSON
    pub fn load_progress(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let progress_map: HashMap<String, TourProgress> = serde_json::from_str(json)?;
        self.progress = progress_map;
        Ok(())
    }

    /// Get tours list
    pub fn tours(&self) -> &[TourDefinition] {
        &self.tours
    }
}

impl Default for TourManager {
    fn default() -> Self {
        Self::new()
    }
}