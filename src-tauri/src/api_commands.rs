//! Kafka Manager 业务 API 的 Tauri IPC 命令
//!
//! 前端通过 `api_request` 调用统一分发器（与原 HTTP `POST /api` 的 method 一一对应），
//! 消息流式查询通过 `message_list_stream` + Channel 推送事件，可随时取消。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tauri::ipc::Channel;
use tokio_util::sync::CancellationToken;

use kafka_manager_api::{api, AppState};

/// 后端状态（DB/Kafka 连接初始化失败时为 None，命令返回友好错误而不是 panic）
pub struct BackendState(pub Option<AppState>);

impl BackendState {
    fn get(&self) -> Result<AppState, String> {
        self.0
            .clone()
            .ok_or_else(|| "Backend not initialized (database failed to open)".to_string())
    }
}

/// 进行中的流式消息查询（request_id -> 取消令牌）
#[derive(Clone, Default)]
pub struct StreamRegistry(pub Arc<Mutex<HashMap<String, CancellationToken>>>);

/// 统一 API 入口：等价于原 POST /api + X-API-Method
#[tauri::command]
pub async fn api_request(
    state: tauri::State<'_, BackendState>,
    method: String,
    params: Value,
) -> Result<Value, String> {
    let state = state.get()?;
    api::dispatch_request(&method, state, params)
        .await
        .map_err(|e| e.to_message())
}

/// 流式消息查询：事件（start/batch/order/complete/error）通过 Channel 推送给前端
#[tauri::command]
pub async fn message_list_stream(
    state: tauri::State<'_, BackendState>,
    registry: tauri::State<'_, StreamRegistry>,
    request_id: String,
    params: Value,
    channel: Channel<api::StreamEvent>,
) -> Result<(), String> {
    let state = state.get()?;
    let cancel_token = CancellationToken::new();

    registry
        .0
        .lock()
        .unwrap()
        .insert(request_id.clone(), cancel_token.clone());

    let mut rx = api::start_message_list_stream(state, params, cancel_token.clone())
        .await
        .map_err(|e| {
            registry.0.lock().unwrap().remove(&request_id);
            e.to_message()
        })?;

    let registry_inner = registry.inner().clone();
    let cancel_for_forward = cancel_token.clone();

    // 转发任务：mpsc -> Tauri Channel
    tokio::spawn(async move {
        while let Some(evt) = rx.recv().await {
            if channel.send(evt).is_err() {
                // 前端已断开（窗口关闭等），停止查询
                cancel_for_forward.cancel();
                break;
            }
        }
        registry_inner.0.lock().unwrap().remove(&request_id);
    });

    // 超时保护（与原 SSE 实现的 120s 一致）；正常结束后取消令牌已无效，无副作用
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;
        cancel_token.cancel();
    });

    Ok(())
}

/// 取消进行中的流式消息查询
#[tauri::command]
pub fn cancel_message_list(
    registry: tauri::State<'_, StreamRegistry>,
    request_id: String,
) -> Result<(), String> {
    if let Some(token) = registry.0.lock().unwrap().remove(&request_id) {
        token.cancel();
    }
    Ok(())
}
