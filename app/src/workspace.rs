//! 工作区：侧边栏导航 + 页面切换

use gpui::*;
use gpui_component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem};
use gpui_component::*;

use crate::i18n::{t, I18n};
use crate::pages::clusters::ClustersPage;
use crate::pages::consumer_groups::ConsumerGroupsPage;
use crate::pages::favorites::FavoritesPage;
use crate::pages::messages::MessagesPage;
use crate::pages::schema_registry::SchemaRegistryPage;
use crate::pages::settings::SettingsPage;
use crate::pages::topics::TopicsPage;
use crate::state::{Backend, Page, TokioRuntime};

pub struct Workspace {
    page: Page,
    clusters_page: Entity<ClustersPage>,
    topics_page: Entity<TopicsPage>,
    messages_page: Entity<MessagesPage>,
    consumer_groups_page: Entity<ConsumerGroupsPage>,
    schema_registry_page: Entity<SchemaRegistryPage>,
    favorites_page: Entity<FavoritesPage>,
    settings_page: Entity<SettingsPage>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let clusters_page = cx.new(|cx| ClustersPage::new(window, cx));
        let topics_page = cx.new(|cx| TopicsPage::new(window, cx));
        let messages_page = cx.new(|cx| MessagesPage::new(window, cx));
        let consumer_groups_page = cx.new(|cx| ConsumerGroupsPage::new(window, cx));
        let schema_registry_page = cx.new(|cx| SchemaRegistryPage::new(window, cx));
        let favorites_page = cx.new(|cx| FavoritesPage::new(window, cx));
        let settings_page = cx.new(|cx| SettingsPage::new(window, cx));

        // 启动后从设置中加载语言与主题
        let rt = TokioRuntime::handle(cx);
        let state = Backend::state(cx);
        cx.spawn(async move |_this, cx| {
            let result = match state {
                Some(state) => crate::service::call(
                    &rt,
                    state,
                    "settings.get",
                    serde_json::json!({ "keys": ["ui.language", "ui.theme"] }),
                )
                .await
                .ok(),
                None => None,
            };
            if let Some(value) = result {
                cx.update(|cx| {
                    if let Some(settings) = value.get("settings").and_then(|v| v.as_array()) {
                        for item in settings {
                            let key = item.get("key").and_then(|k| k.as_str()).unwrap_or("");
                            let val = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            match (key, val) {
                                ("ui.language", "en") => I18n::global_mut(cx).set_lang("en"),
                                ("ui.language", "zh") => I18n::global_mut(cx).set_lang("zh"),
                                ("ui.theme", "dark") => {
                                    gpui_component::Theme::change(ThemeMode::Dark, None, cx)
                                }
                                ("ui.theme", "light") => {
                                    gpui_component::Theme::change(ThemeMode::Light, None, cx)
                                }
                                _ => {}
                            }
                        }
                    }
                    cx.refresh_windows();
                })
                .ok();
            }
        })
        .detach();

        Self {
            page: Page::Clusters,
            clusters_page,
            topics_page,
            messages_page,
            consumer_groups_page,
            schema_registry_page,
            favorites_page,
            settings_page,
        }
    }

    fn switch_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let items = Page::all().iter().map(|page| {
            let page = *page;
            let label = t(cx, page.i18n_key());
            let icon = match page {
                Page::Clusters => IconName::Building2,
                Page::Topics => IconName::FolderOpen,
                Page::Messages => IconName::Inbox,
                Page::ConsumerGroups => IconName::CircleUser,
                Page::SchemaRegistry => IconName::BookOpen,
                Page::Favorites => IconName::Star,
                Page::Settings => IconName::Settings,
            };
            SidebarMenuItem::new(label)
                .icon(icon)
                .active(self.page == page)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.switch_page(page, cx);
                }))
        });

        Sidebar::new(Side::Left)
            .collapsible(false)
            .header(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .size_8()
                            .rounded_md()
                            .bg(theme.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(gpui_component::white())
                            .child("K"),
                    )
                    .child(div().font_semibold().child("Kafka Manager")),
            )
            .children(std::iter::once(SidebarMenu::new().children(items)))
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let content: AnyElement = match self.page {
            Page::Clusters => self.clusters_page.clone().into_any_element(),
            Page::Topics => self.topics_page.clone().into_any_element(),
            Page::Messages => self.messages_page.clone().into_any_element(),
            Page::ConsumerGroups => self.consumer_groups_page.clone().into_any_element(),
            Page::SchemaRegistry => self.schema_registry_page.clone().into_any_element(),
            Page::Favorites => self.favorites_page.clone().into_any_element(),
            Page::Settings => self.settings_page.clone().into_any_element(),
        };

        h_flex()
            .size_full()
            .bg(theme.background)
            .child(self.render_sidebar(cx))
            .child(div().flex_1().h_full().overflow_hidden().child(content))
    }
}
