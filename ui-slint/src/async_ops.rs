// 异步操作处理工具

use slint::{ComponentHandle, Weak};
use tokio::task::spawn;

/// 在后台线程运行异步操作，完成后更新 UI
///
/// # 参数
/// - `app`: Slint 应用的弱引用
/// - `task`: 异步任务
/// - `on_complete`: 完成回调函数
///
/// # 示例
/// ```rust
/// spawn_async_task(
///     app_weak,
///     async move {
///         // 执行异步操作
///         let result = some_async_function().await;
///         result
///     },
///     |app, result| {
///         // 更新 UI
///         app.set_data(result);
///     }
/// );
/// ```
pub fn spawn_async_task<F, T>(
    app: Weak<crate::App>,
    task: F,
    on_complete: impl FnOnce(&crate::App, T) + Send + 'static,
) where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    spawn(async move {
        let result = task.await;

        // 回到主线程更新 UI
        let _ = app.upgrade_in_event_loop(move |app| {
            on_complete(&app, result);
        });
    });
}

/// 批量更新 UI（减少 UI 更新频率）
///
/// # 参数
/// - `app`: Slint 应用的弱引用
/// - `batch`: 批量数据
/// - `update_fn`: 更新函数
///
/// # 示例
/// ```rust
/// let messages = vec![...];
/// batch_update_ui(app_weak, messages, |app, batch| {
///     for msg in batch {
///         app.get_messages().push(msg);
///     }
/// });
/// ```
pub fn batch_update_ui<T>(
    app: Weak<crate::App>,
    batch: Vec<T>,
    update_fn: impl FnOnce(&crate::App, Vec<T>) + Send + 'static,
) where
    T: Send + 'static,
{
    let _ = app.upgrade_in_event_loop(move |app| {
        update_fn(&app, batch);
    });
}