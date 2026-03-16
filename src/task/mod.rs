/// 异步任务队列模块
///
/// 用于处理耗时任务（如大批量导出），避免请求超时

use crate::error::Result;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    ExportMessages,
    BulkImport,
}

/// 任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub progress: Option<f64>,  // 0.0 - 1.0
    pub message: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

/// 任务请求
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    #[serde(rename = "type")]
    pub task_type: TaskType,
    pub params: serde_json::Value,
}

/// 任务存储
#[derive(Clone)]
pub struct TaskStore {
    tasks: Arc<RwLock<HashMap<String, TaskInfo>>>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建新任务
    pub async fn create(&self, task_type: TaskType, _params: serde_json::Value) -> String {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();

        let task = TaskInfo {
            id: id.clone(),
            task_type,
            status: TaskStatus::Pending,
            progress: Some(0.0),
            message: Some("Task created".to_string()),
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        let mut tasks = self.tasks.write().await;
        tasks.insert(id.clone(), task);

        id
    }

    /// 获取任务状态
    pub async fn get(&self, id: &str) -> Option<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.get(id).cloned()
    }

    /// 更新任务状态
    pub async fn update(&self, id: &str, status: TaskStatus, progress: Option<f64>, message: Option<String>) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.status = status;
            task.progress = progress;
            task.message = message;
            task.updated_at = chrono::Utc::now().timestamp_millis();
        }
    }

    /// 标记任务完成
    pub async fn complete(&self, id: &str, result: serde_json::Value) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Completed;
            task.progress = Some(1.0);
            task.message = Some("Task completed".to_string());
            task.result = Some(result);
            task.completed_at = Some(chrono::Utc::now().timestamp_millis());
            task.updated_at = task.completed_at.expect("completed_at should be set");
        }
    }

    /// 标记任务失败
    pub async fn fail(&self, id: &str, error: String) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Failed;
            task.error = Some(error);
            task.completed_at = Some(chrono::Utc::now().timestamp_millis());
            task.updated_at = task.completed_at.expect("completed_at should be set");
        }
    }

    /// 删除已完成的任务
    pub async fn delete(&self, id: &str) -> bool {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id).is_some()
    }

    /// 清理旧任务（超过 24 小时的已完成/失败任务）
    pub async fn cleanup_old(&self, max_age_hours: i64) -> usize {
        let mut tasks = self.tasks.write().await;
        let now = chrono::Utc::now().timestamp_millis();
        let max_age_ms = max_age_hours * 3600 * 1000;

        let mut removed = 0;
        tasks.retain(|_, task| {
            if matches!(task.status, TaskStatus::Completed | TaskStatus::Failed) {
                if let Some(completed_at) = task.completed_at {
                    if now - completed_at > max_age_ms {
                        removed += 1;
                        return false;
                    }
                }
            }
            true
        });

        removed
    }

    /// 获取所有任务
    pub async fn list(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }
}

impl Default for TaskStore {
    fn default() -> Self {
        Self::new()
    }
}

/// 路由
pub fn routes() -> Router<crate::AppState> {
    Router::new()
        .route("/tasks", post(create_task))
        .route("/tasks", get(list_tasks))
        .route("/tasks/:id", get(get_task))
        .route("/tasks/:id", delete(delete_task))
}

async fn create_task(
    State(state): State<crate::AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<serde_json::Value>> {
    let task_id = state.task_store.create(req.task_type, req.params).await;
    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "pending"
    })))
}

async fn list_tasks(
    State(state): State<crate::AppState>,
) -> Result<Json<Vec<TaskInfo>>> {
    let tasks = state.task_store.list().await;
    Ok(Json(tasks))
}

async fn get_task(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<TaskInfo>> {
    let task = state.task_store.get(&id).await
        .ok_or_else(|| crate::error::AppError::NotFound(format!("Task {} not found", id)))?;
    Ok(Json(task))
}

async fn delete_task(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let deleted = state.task_store.delete(&id).await;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
