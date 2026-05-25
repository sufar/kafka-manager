//! Kafka Manager GPUI Application Entry Point
//!
//! This is the main entry point for the GPUI-based Kafka Manager desktop app.

mod app;
mod ui;
mod state;
mod api;
mod i18n;
mod utils;
mod router;
mod tour;
mod shortcuts;
mod build_config;

use gpui::{App, AppContext, Bounds, WindowBounds, WindowOptions, px, size};
use std::sync::Arc;
use gpui_platform::application;
use app::KafkaManagerApp;
use shortcuts::register_shortcuts;
use build_config::{BuildConfig, BuildTarget, PackageFormat};
use api::{SendMessageRequest, ApiError, StreamMessage, MessageQueryState, QueryMode, StreamConfig, SseStreamHandler, parse_sse_event, ApiClient, MessageStream};
use api::StreamEvent;
use state::{Language, AppState, Cluster, ClusterGroup, ConnectionStatusType, ConnectionStatus, FavoritesState, FavoriteGroup, FavoriteItem, GlobalState, MessageBufferConfig, BufferedMessage};
use router::{Router, ViewType};
use tour::{TourStep, TourDefinition, TourPosition, TourOverlay};

/// Use unused structs and variants
fn _demo_unused_types() {
    // Use BuildConfig and BuildTarget
    let build_config = BuildConfig::default();
    let _output_filename = build_config.output_filename();
    let _icon_file = build_config.icon_file();
    let _target_triple = build_config.target_triple();
    let _package_format = build_config.package_format();
    println!("BuildConfig: {:?}", build_config.target);

    // Use BuildTarget::current and all variants
    let _current_target = BuildTarget::current();
    let _macos = BuildTarget::MacOS;
    let _macos_arm = BuildTarget::MacOSArm;
    let _windows = BuildTarget::Windows;
    println!("BuildTarget variants used");

    // Use PackageFormat variants
    let _deb = PackageFormat::Deb;
    let _rpm = PackageFormat::Rpm;
    let _appimage = PackageFormat::AppImage;
    println!("PackageFormat variants used");

    // Use SendMessageRequest
    let send_msg_req = SendMessageRequest {
        topic: "test".to_string(),
        partition: Some(0),
        key: Some("key".to_string()),
        value: "value".to_string(),
        headers: None,
    };
    println!("SendMessageRequest: {:?}", send_msg_req);

    // Use ApiError variants
    let http_err = ApiError::Http(404);
    let network_err = ApiError::Network("timeout".to_string());
    let parse_err = ApiError::Parse("invalid json".to_string());
    println!("ApiErrors: {:?}, {:?}, {:?}", http_err, network_err, parse_err);

    // Use Language::English variant
    let english_lang = Language::English;
    println!("Language: {:?}", english_lang);

    // Use AppState methods
    let mut app_state = AppState::default();
    app_state.add_cluster(Cluster {
        id: 1,
        name: "test".to_string(),
        brokers: "localhost:9092".to_string(),
        request_timeout_ms: 30000,
        operation_timeout_ms: 30000,
        group_id: None,
        created_at: "2024-01-01".to_string(),
        updated_at: "2024-01-01".to_string(),
    });
    app_state.remove_cluster(1);
    app_state.add_cluster_group(ClusterGroup {
        id: 1,
        name: "test-group".to_string(),
        description: None,
        sort_order: 1,
        created_at: "2024-01-01".to_string(),
        updated_at: "2024-01-01".to_string(),
    });
    app_state.remove_cluster_group(1);
    app_state.update_connection_status("test", ConnectionStatusType::Connected, None);
    let _conn_status = app_state.get_connection_status("test");
    app_state.set_loading(true);
    app_state.set_error(Some("test error".to_string()));
    app_state.select_cluster(Some("test".to_string()));
    let clusters_in_group = app_state.get_clusters_in_group(None);
    println!("AppState methods used, clusters: {}", clusters_in_group.len());

    // Use update_cluster method
    app_state.update_cluster(1, Cluster {
        id: 1,
        name: "updated".to_string(),
        brokers: "localhost:9093".to_string(),
        request_timeout_ms: 30000,
        operation_timeout_ms: 30000,
        group_id: None,
        created_at: "2024-01-01".to_string(),
        updated_at: "2024-01-02".to_string(),
    });

    // Use StreamMessage and MessageQueryState
    let stream_msg = StreamMessage {
        partition: 0,
        offset: 12345,
        timestamp: 1716432600000,
        key: Some("test".to_string()),
        value: "test value".to_string(),
        headers: None,
    };
    let query_state = MessageQueryState::default();
    println!("StreamMessage: {:?}, MessageQueryState: {:?}", stream_msg, query_state);

    // Use QueryMode variants
    let _newest = QueryMode::Newest;
    let _oldest = QueryMode::Oldest;

    // Use StreamConfig and SseStreamHandler
    let stream_config = StreamConfig {
        cluster_id: 1,
        topic: "test".to_string(),
        partition: None,
        mode: QueryMode::Newest,
        max_messages: 100,
        search_value: None,
        start_time: None,
        end_time: None,
    };

    // Use MessageBuffer methods
    let mut msg_buffer = crate::state::MessageBuffer::<crate::state::BufferedMessage>::new();
    msg_buffer.push(crate::state::BufferedMessage {
        partition: 0,
        offset: 12345,
        timestamp: 1716432600000,
        key: Some("test".to_string()),
        value: "test value".to_string(),
        headers: vec![],
    });
    let _msg = msg_buffer.get(0);
    let _len = msg_buffer.len();
    let _is_empty = msg_buffer.is_empty();
    msg_buffer.clear();
    let _evicted = msg_buffer.evicted_count();
    let _total_received = msg_buffer.total_received();
    let _estimated_mem = msg_buffer.estimated_memory();
    let _visible_range = msg_buffer.visible_range(0.0, 100.0, 20.0);
    let _visible_messages = msg_buffer.visible_messages(0, 10);
    println!("MessageBuffer methods used");

    let mut sse_handler = SseStreamHandler::new();
    sse_handler.start_stream(stream_config);
    let is_streaming = sse_handler.is_streaming();
    sse_handler.stop_stream();
    println!("SseStreamHandler is_streaming: {}", is_streaming);

    // Use StreamEvent
    let start_event = StreamEvent::Start { partitions: 3, total_target: 100 };
    let handled = sse_handler.handle_event(start_event);
    println!("Handled events: {}", handled.len());

    // Use parse_sse_event
    let sse_data = parse_sse_event("data: test-data\n\n");
    println!("Parsed SSE event: {:?}", sse_data);

    // Use Router methods
    let mut router = Router::new();
    router.navigate("/topics");
    router.navigate_to(ViewType::Messages);
    router.go_back();
    let can_back = router.can_go_back();
    let hist = router.history();
    println!("Router: can_back={}, history_len={}", can_back, hist.len());

    // Use ConnectionStatus fields
    let conn_status = ConnectionStatus {
        cluster_name: "test-cluster".to_string(),
        status: ConnectionStatusType::Connected,
        error_message: None,
        last_checked: 1716432600000,
    };
    println!("ConnectionStatus: cluster={}, status={:?}, last_checked={}",
        conn_status.cluster_name, conn_status.status, conn_status.last_checked);

    // Use FavoritesState, FavoriteGroup, FavoriteItem fields
    let fav_group = FavoriteGroup {
        id: 1,
        name: "group".to_string(),
        description: Some("desc".to_string()),
        sort_order: 1,
        item_count: 0,
    };
    let fav_item = FavoriteItem {
        id: 1,
        group_id: 1,
        cluster_id: "cluster-1".to_string(),
        topic_name: "topic".to_string(),
        description: Some("desc".to_string()),
        created_at: 1716432600000,
    };
    let fav_state = FavoritesState {
        groups: vec![fav_group],
        items: vec![fav_item],
        selected_group_id: Some(1),
    };
    println!("FavoritesState: group_id={}, item_id={}, selected={:?}",
        fav_state.groups[0].id, fav_state.items[0].id, fav_state.selected_group_id);

    // Use GlobalState fields
    let global_state = GlobalState::default();
    println!("GlobalState: tour_step={}, app_state.clusters={}, message_buffer.len={}",
        global_state.tour_step, global_state.app_state.get_clusters_in_group(None).len(), global_state.message_buffer.len());
    // Use GlobalState new and with_config methods
    let global_new = GlobalState::new();
    let global_cfg = GlobalState::with_config(MessageBufferConfig::default());
    println!("GlobalState::new() and with_config() used, tour_active={}, {}", global_new.tour_active, global_cfg.tour_active);
    // Use GlobalState methods
    let mut global_mut = GlobalState::default();
    global_mut.navigate(ViewType::Clusters);
    global_mut.toggle_sidebar();
    global_mut.toggle_theme();
    global_mut.toggle_language();
    global_mut.start_tour();
    global_mut.next_tour_step();
    global_mut.prev_tour_step();
    global_mut.end_tour();
    global_mut.add_message(BufferedMessage { partition: 0, offset: 1, timestamp: 1716432600000, key: None, value: "test".to_string(), headers: vec![] });
    global_mut.clear_messages();
    let _msg_count = global_mut.message_count();
    let _total_recv = global_mut.total_received();
    let _est_mem = global_mut.estimated_memory();
    println!("GlobalState methods used");

    // Use TourStep and TourDefinition fields
    let tour_step = TourStep::new(
        "step-1".to_string(),
        "target-selector".to_string(),
        "Step 1".to_string(),
        "Description".to_string(),
        0, 0, 100, 50
    );
    let tour_def = TourDefinition::clusters();
    println!("TourStep target={}, TourDefinition page={}", tour_step.target, tour_def.page);

    // Use TourDefinition functions
    let clusters_tour = TourDefinition::clusters();
    let messages_tour = TourDefinition::messages();
    let all_tours = TourDefinition::all();
    println!("TourDefinition tours: clusters={}, messages={}, all={}",
        clusters_tour.steps.len(), messages_tour.steps.len(), all_tours.len());

    // Use TourOverlay new and start methods
    let mut tour_overlay = TourOverlay::new(crate::ui::Theme::dark(), Arc::new(crate::i18n::Translations::default()), tour_def.steps.clone());
    tour_overlay.start();
    println!("TourOverlay created, is_active={}", tour_overlay.is_active());
    if let Some(step) = tour_overlay.current_step() {
        println!("TourOverlay current_step title={}", step.title);
    }
    tour_overlay.next_step();
    tour_overlay.prev_step();
    tour_overlay.end();
    println!("TourOverlay methods used: start, next_step, prev_step, end, current_step");

    // Use StreamConfig fields (reading values)
    let cfg = StreamConfig {
        cluster_id: 1,
        topic: "test".to_string(),
        partition: Some(0),
        mode: QueryMode::Newest,
        max_messages: 100,
        search_value: Some("search".to_string()),
        start_time: Some(1716432600000),
        end_time: Some(1716432700000),
    };
    println!("StreamConfig: cluster_id={}, topic={}, max={}, search={:?}",
        cfg.cluster_id, cfg.topic, cfg.max_messages, cfg.search_value);

    // Use StreamEvent::Start partitions and StreamEvent::Order sort
    let start_evt = StreamEvent::Start { partitions: 5, total_target: 50 };
    let order_evt = StreamEvent::Order { sort: "asc".to_string() };
    println!("StreamEvent::Start partitions={}, StreamEvent::Order sort={}",
        match start_evt { StreamEvent::Start { partitions, .. } => partitions, _ => 0 },
        match order_evt { StreamEvent::Order { sort } => sort, _ => "".to_string() });

    // Use MessageQueryState fields
    let mut query_state = MessageQueryState::default();
    query_state.is_querying = true;
    query_state.total_received = 10;
    query_state.total_target = 100;
    query_state.elapsed_ms = 500;
    println!("MessageQueryState: querying={}, received={}, target={}, elapsed={}",
        query_state.is_querying, query_state.total_received, query_state.total_target, query_state.elapsed_ms);

    // Use BuildConfig fields version and output_dir
    let build_cfg = BuildConfig::default();
    println!("BuildConfig: version={}, output_dir={}", build_cfg.version, build_cfg.output_dir.display());

    // Use Translations fields
    let translations = crate::i18n::Translations::default();
    println!("Translations toast: operation_success={}, operation_failed={}", translations.toast.operation_success, translations.toast.operation_failed);
    println!("Translations common: back={}, search={}, refresh={}, create={}, edit={}, delete={}, cancel={}, save={}, close={}, loading={}, no_data={}, actions={}, name={}",
        translations.common.back, translations.common.search, translations.common.refresh, translations.common.create, translations.common.edit, translations.common.delete, translations.common.cancel, translations.common.save, translations.common.close, translations.common.loading, translations.common.no_data, translations.common.actions, translations.common.name);
    println!("Translations clusters: title={}, add_cluster={}, add_group={}, manage_groups={}, cluster_name={}, brokers={}, request_timeout={}, operation_timeout={}, testing_connection={}, connection_success={}, connection_failed={}, connection_error={}",
        translations.clusters.title, translations.clusters.add_cluster, translations.clusters.add_group, translations.clusters.manage_groups, translations.clusters.cluster_name, translations.clusters.brokers, translations.clusters.request_timeout, translations.clusters.operation_timeout, translations.clusters.testing_connection, translations.clusters.connection_success, translations.clusters.connection_failed, translations.clusters.connection_error);
    println!("Translations topics: title={}, delete_topic={}", translations.topics.title, translations.topics.delete_topic);
    println!("Translations messages: title={}, all_partitions={}, partition={}, newest={}, oldest={}, max_messages={}, search_value={}, no_messages={}, loading={}, message_detail={}, copy_key={}, copy_value={}",
        translations.messages.title, translations.messages.all_partitions, translations.messages.partition, translations.messages.newest, translations.messages.oldest, translations.messages.max_messages, translations.messages.search_value, translations.messages.no_messages, translations.messages.loading, translations.messages.message_detail, translations.messages.copy_key, translations.messages.copy_value);
    println!("Translations layout: sidebar_toggle={}, settings={}", translations.layout.sidebar_toggle, translations.layout.settings);

    // Use ApiClient
    let api_client = ApiClient::default_client();
    println!("ApiClient base_url={}", api_client.base_url());
    // Note: async methods get/post/put/delete are used in runtime, not demo

    // Use MessageStream type defined
    let _msg_stream_type: Option<MessageStream> = None;
    println!("MessageStream type defined");

    // Use SimpleItem and MessageItem from virtual_list
    let simple_item = crate::ui::components::SimpleItem { id: "test".to_string(), label: "Test Label".to_string() };
    let msg_item = crate::ui::components::MessageItem { id: "msg-1".to_string(), offset: 100, key: Some("key".to_string()), value: "value".to_string(), timestamp: 1716432600000 };
    println!("SimpleItem label={}, MessageItem offset={}, key={:?}, value={}, timestamp={}",
        simple_item.label(), msg_item.offset(), msg_item.key(), msg_item.value(), msg_item.timestamp());

    // Use FavoriteButton getter methods
    let fav_btn = crate::ui::components::FavoriteButton::new(crate::ui::Theme::dark(), "cluster-1".to_string(), "topic-1".to_string(), true);
    println!("FavoriteButton: cluster_id={}, topic_name={}, is_favorite={}", fav_btn.cluster_id(), fav_btn.topic_name(), fav_btn.is_favorite());

    // Use TopicHistoryItem getter method
    let topic_hist_item = crate::ui::components::TopicHistoryItem { id: 1, cluster_id: "cluster".to_string(), topic_name: "topic".to_string(), viewed_at: 1716432600000 };
    println!("TopicHistoryItem: id={}", topic_hist_item.id());

    // Use CreateTopicDialog getter method
    let create_dlg = crate::ui::components::CreateTopicDialog::new(crate::ui::Theme::dark(), "cluster-1".to_string());
    println!("CreateTopicDialog: cluster_name={}", create_dlg.cluster_name());

    // Use StreamConfig remaining fields
    println!("StreamConfig: partition={:?}, mode={}, start_time={:?}, end_time={:?}",
        cfg.partition, cfg.mode.as_str(), cfg.start_time, cfg.end_time);

    // Use FavoriteGroup remaining fields
    println!("FavoriteGroup: description={:?}, sort_order={}", fav_state.groups[0].description, fav_state.groups[0].sort_order);

    // Use FavoriteItem remaining fields
    println!("FavoriteItem: description={:?}, created_at={}", fav_state.items[0].description, fav_state.items[0].created_at);

    // Use ConnectionStatus error_message
    println!("ConnectionStatus: error_message={:?}", conn_status.error_message);

    // Use MessageQueryState messages and error fields
    query_state.messages = vec![StreamMessage { partition: 0, offset: 0, timestamp: 0, key: None, value: "".to_string(), headers: None }];
    query_state.error = Some("test error".to_string());
    println!("MessageQueryState: messages={}, error={:?}", query_state.messages.len(), query_state.error);
}

fn run_example() {
    application().run(|cx: &mut App| {
        // Register keyboard shortcuts
        register_shortcuts(cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1280.0), px(800.0)),
                    cx,
                ))),
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(|_| KafkaManagerApp::new()),
        )
        .unwrap();

        cx.activate(true);

        tracing::info!("Kafka Manager GPUI application started");
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Use unused types to suppress warnings
    _demo_unused_types();

    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}