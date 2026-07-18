//! 全局状态：后端句柄、页面导航、选中项

use gpui::{App, Global};

use kafka_manager_api::AppState;

/// 后端状态（初始化失败时为 None）
pub struct Backend(pub Option<AppState>);

impl Global for Backend {}

impl Backend {
    /// 获取 AppState；未初始化时返回 None
    pub fn state(cx: &App) -> Option<AppState> {
        cx.try_global::<Backend>().and_then(|b| b.0.clone())
    }
}

/// tokio runtime 句柄（业务调用必须在该 runtime 上执行，rdkafka/sqlx 依赖 tokio 上下文）
pub struct TokioRuntime(pub tokio::runtime::Handle);

impl Global for TokioRuntime {}

impl TokioRuntime {
    pub fn handle(cx: &App) -> tokio::runtime::Handle {
        cx.global::<TokioRuntime>().0.clone()
    }
}

/// 应用页面
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Clusters,
    Topics,
    Messages,
    ConsumerGroups,
    SchemaRegistry,
    Favorites,
    Settings,
}

impl Page {
    pub fn all() -> &'static [Page] {
        &[
            Page::Clusters,
            Page::Topics,
            Page::Messages,
            Page::ConsumerGroups,
            Page::SchemaRegistry,
            Page::Favorites,
            Page::Settings,
        ]
    }

    /// 对应的 i18n key（nav 节）
    pub fn i18n_key(&self) -> &'static str {
        match self {
            Page::Clusters => "nav.clusters",
            Page::Topics => "nav.topics",
            Page::Messages => "nav.messages",
            Page::ConsumerGroups => "nav.consumerGroups",
            Page::SchemaRegistry => "tree.schemaRegistry",
            Page::Favorites => "nav.favorites",
            Page::Settings => "nav.settings",
        }
    }
}
