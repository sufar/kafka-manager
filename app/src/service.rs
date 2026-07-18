//! 业务服务封装：在 tokio runtime 上调用核心库统一分发器
//!
//! 所有函数返回的 Future 可安全地在 gpui executor 上 await：
//! 实际工作在 tokio runtime 的 JoinHandle 中执行。

use std::future::Future;

use kafka_manager_api::{api, AppState};
use serde_json::Value;

use crate::state::{Backend, TokioRuntime};

/// 调用统一分发器（等价于原 POST /api + X-API-Method）
pub async fn call(
    rt: &tokio::runtime::Handle,
    state: AppState,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let method = method.to_string();
    rt.spawn(async move {
        api::dispatch_request(&method, state, params)
            .await
            .map_err(|e| e.to_message())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 从 gpui App 上下文调用统一分发器
pub fn api_call(
    cx: &gpui::App,
    method: &str,
    params: Value,
) -> impl Future<Output = Result<Value, String>> {
    let rt = TokioRuntime::handle(cx);
    let state = Backend::state(cx);
    let method = method.to_string();
    async move {
        let state = state.ok_or_else(|| "Backend not initialized".to_string())?;
        call(&rt, state, &method, params).await
    }
}
