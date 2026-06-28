export interface TourStep {
  selector: string;
  title: string;
  description: string;
  position?: 'top' | 'bottom' | 'left' | 'right';
}

export interface TourDefinition {
  route: string;
  steps: TourStep[];
}

// 全局步骤（所有页面都显示）
const globalSteps: TourStep[] = [
  { selector: '[data-tour="search"]', title: 'tour.globalSearch.title', description: 'tour.globalSearch.desc', position: 'bottom' },
  { selector: '[data-tour="share"]', title: 'tour.globalShare.title', description: 'tour.globalShare.desc', position: 'bottom' },
  { selector: '[data-tour="lang"]', title: 'tour.globalLang.title', description: 'tour.globalLang.desc', position: 'bottom' },
  { selector: '[data-tour="theme"]', title: 'tour.globalTheme.title', description: 'tour.globalTheme.desc', position: 'bottom' },
  { selector: '[data-tour="settings"]', title: 'tour.globalSettings.title', description: 'tour.globalSettings.desc', position: 'bottom' },
];

// 侧边栏通用步骤（TopicNavigator - 列表模式）
const sidebarSteps: TourStep[] = [
  { selector: '[data-tour="sidebar-view-switcher"]', title: 'tour.sidebarViewSwitcher.title', description: 'tour.sidebarViewSwitcher.desc', position: 'right' },
  { selector: '[data-tour="sidebar-clusters-btn"]', title: 'tour.sidebarClustersBtn.title', description: 'tour.sidebarClustersBtn.desc', position: 'right' },
  { selector: '[data-tour="sidebar-favorites-btn"]', title: 'tour.sidebarFavoritesBtn.title', description: 'tour.sidebarFavoritesBtn.desc', position: 'right' },
  { selector: '[data-tour="sidebar-schema-btn"]', title: 'tour.sidebarSchemaBtn.title', description: 'tour.sidebarSchemaBtn.desc', position: 'right' },
  { selector: '[data-tour="sidebar-history-btn"]', title: 'tour.sidebarHistoryBtn.title', description: 'tour.sidebarHistoryBtn.desc', position: 'right' },
  { selector: '[data-tour="sidebar-cluster-selector"]', title: 'tour.sidebarClusterSelector.title', description: 'tour.sidebarClusterSelector.desc', position: 'right' },
  { selector: '[data-tour="sidebar-refresh"]', title: 'tour.sidebarRefresh.title', description: 'tour.sidebarRefresh.desc', position: 'right' },
  { selector: '[data-tour="sidebar-search"]', title: 'tour.sidebarSearch.title', description: 'tour.sidebarSearch.desc', position: 'right' },
  { selector: '[data-tour="sidebar-topic-list"]', title: 'tour.sidebarTopicList.title', description: 'tour.sidebarTopicList.desc', position: 'left' },
  { selector: '[data-tour="sidebar-consumer-group-list"]', title: 'tour.sidebarConsumerGroupList.title', description: 'tour.sidebarConsumerGroupList.desc', position: 'left' },
  { selector: '[data-tour="sidebar-consumer-group-list"]', title: 'tour.sidebarConsumerGroupList.title', description: 'tour.sidebarConsumerGroupList.desc', position: 'left' },
  { selector: '[data-tour="sidebar-health-dot"]', title: 'tour.sidebarHealthDot.title', description: 'tour.sidebarHealthDot.desc', position: 'left' },
  { selector: '[data-tour="sidebar-topic-name"]', title: 'tour.sidebarTopicName.title', description: 'tour.sidebarTopicName.desc', position: 'left' },
  { selector: '[data-tour="sidebar-cluster-badge"]', title: 'tour.sidebarClusterBadge.title', description: 'tour.sidebarClusterBadge.desc', position: 'left' },
];

// 树形模式侧边栏步骤（ClusterTreeNavigator）
const treeSidebarSteps: TourStep[] = [
  { selector: '[data-tour="tree-collapse-btn"]', title: 'tour.treeCollapseBtn.title', description: 'tour.treeCollapseBtn.desc', position: 'right' },
  { selector: '[data-tour="tree-clusters-btn"]', title: 'tour.treeClustersBtn.title', description: 'tour.treeClustersBtn.desc', position: 'right' },
  { selector: '[data-tour="tree-favorites-btn"]', title: 'tour.treeFavoritesBtn.title', description: 'tour.treeFavoritesBtn.desc', position: 'right' },
  { selector: '[data-tour="tree-schema-btn"]', title: 'tour.treeSchemaBtn.title', description: 'tour.treeSchemaBtn.desc', position: 'right' },
  { selector: '[data-tour="tree-history-btn"]', title: 'tour.treeHistoryBtn.title', description: 'tour.treeHistoryBtn.desc', position: 'right' },
  { selector: '[data-tour="tree-group-selector"]', title: 'tour.treeGroupSelector.title', description: 'tour.treeGroupSelector.desc', position: 'right' },
  { selector: '[data-tour="tree-cluster-health-dot"]', title: 'tour.treeClusterHealthDot.title', description: 'tour.treeClusterHealthDot.desc', position: 'left' },
  { selector: '[data-tour="tree-cluster-name"]', title: 'tour.treeClusterName.title', description: 'tour.treeClusterName.desc', position: 'left' },
  { selector: '[data-tour="tree-topics-folder"]', title: 'tour.treeTopicsFolder.title', description: 'tour.treeTopicsFolder.desc', position: 'left' },
  { selector: '[data-tour="tree-topics-refresh"]', title: 'tour.treeTopicsRefresh.title', description: 'tour.treeTopicsRefresh.desc', position: 'left' },
  { selector: '[data-tour="tree-topic-search"]', title: 'tour.treeTopicSearch.title', description: 'tour.treeTopicSearch.desc', position: 'left' },
  { selector: '[data-tour="tree-topic-name"]', title: 'tour.treeTopicName.title', description: 'tour.treeTopicName.desc', position: 'left' },
  { selector: '[data-tour="tree-consumer-groups-folder"]', title: 'tour.treeConsumerGroupsFolder.title', description: 'tour.treeConsumerGroupsFolder.desc', position: 'left' },
  { selector: '[data-tour="tree-consumer-groups-refresh"]', title: 'tour.treeConsumerGroupsRefresh.title', description: 'tour.treeConsumerGroupsRefresh.desc', position: 'left' },
  { selector: '[data-tour="tree-consumer-group-search"]', title: 'tour.treeConsumerGroupSearch.title', description: 'tour.treeConsumerGroupSearch.desc', position: 'left' },
  { selector: '[data-tour="tree-consumer-group-name"]', title: 'tour.treeConsumerGroupName.title', description: 'tour.treeConsumerGroupName.desc', position: 'left' },
];

const clustersSteps: TourStep[] = [
  { selector: '[data-tour="clusters-title"]', title: 'tour.clustersTitle.title', description: 'tour.clustersTitle.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-manage-groups"]', title: 'tour.clustersManageGroups.title', description: 'tour.clustersManageGroups.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-add"]', title: 'tour.clustersAdd.title', description: 'tour.clustersAdd.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-group"]', title: 'tour.clustersGroup.title', description: 'tour.clustersGroup.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-card"]', title: 'tour.clustersCard.title', description: 'tour.clustersCard.desc', position: 'right' },
  { selector: '[data-tour="clusters-status"]', title: 'tour.clustersStatus.title', description: 'tour.clustersStatus.desc', position: 'left' },
  { selector: '[data-tour="clusters-edit-btn"]', title: 'tour.clustersEditBtn.title', description: 'tour.clustersEditBtn.desc', position: 'top' },
  { selector: '[data-tour="clusters-delete-btn"]', title: 'tour.clustersDeleteBtn.title', description: 'tour.clustersDeleteBtn.desc', position: 'top' },
  { selector: '[data-tour="clusters-test-btn"]', title: 'tour.clustersTestBtn.title', description: 'tour.clustersTestBtn.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-reconnect-btn"]', title: 'tour.clustersReconnectBtn.title', description: 'tour.clustersReconnectBtn.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-topics-btn"]', title: 'tour.clustersTopicsBtn.title', description: 'tour.clustersTopicsBtn.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-add-topic-btn"]', title: 'tour.clustersAddTopicBtn.title', description: 'tour.clustersAddTopicBtn.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-refresh-topics-btn"]', title: 'tour.clustersRefreshTopicsBtn.title', description: 'tour.clustersRefreshTopicsBtn.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-empty"]', title: 'tour.clustersEmpty.title', description: 'tour.clustersEmpty.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-error"]', title: 'tour.clustersError.title', description: 'tour.clustersError.desc', position: 'bottom' },
  { selector: '[data-tour="clusters-modal"]', title: 'tour.clustersModal.title', description: 'tour.clustersModal.desc', position: 'top' },
  { selector: '[data-tour="clusters-modal-name"]', title: 'tour.clustersModalName.title', description: 'tour.clustersModalName.desc', position: 'right' },
  { selector: '[data-tour="clusters-modal-brokers"]', title: 'tour.clustersModalBrokers.title', description: 'tour.clustersModalBrokers.desc', position: 'right' },
  { selector: '[data-tour="clusters-modal-test"]', title: 'tour.clustersModalTest.title', description: 'tour.clustersModalTest.desc', position: 'right' },
  { selector: '[data-tour="clusters-modal-submit"]', title: 'tour.clustersModalSubmit.title', description: 'tour.clustersModalSubmit.desc', position: 'right' },
];

const topicsSteps: TourStep[] = [
  { selector: '[data-tour="topics-create"]', title: 'tour.topicsCreate.title', description: 'tour.topicsCreate.desc', position: 'bottom' },
  { selector: '[data-tour="topics-list"]', title: 'tour.topicsList.title', description: 'tour.topicsList.desc', position: 'right' },
  { selector: '[data-tour="topics-actions"]', title: 'tour.topicsActions.title', description: 'tour.topicsActions.desc', position: 'bottom' },
];

const messagesSteps: TourStep[] = [
  { selector: '[data-tour="messages-back"]', title: 'tour.messagesBack.title', description: 'tour.messagesBack.desc', position: 'bottom' },
  { selector: '[data-tour="messages-partition"]', title: 'tour.messagesPartition.title', description: 'tour.messagesPartition.desc', position: 'bottom' },
  { selector: '[data-tour="messages-mode"]', title: 'tour.messagesMode.title', description: 'tour.messagesMode.desc', position: 'bottom' },
  { selector: '[data-tour="messages-count"]', title: 'tour.messagesCount.title', description: 'tour.messagesCount.desc', position: 'bottom' },
  { selector: '[data-tour="messages-time-filter"]', title: 'tour.messagesTimeFilter.title', description: 'tour.messagesTimeFilter.desc', position: 'bottom' },
  { selector: '[data-tour="messages-time-presets"]', title: 'tour.messagesTimePresets.title', description: 'tour.messagesTimePresets.desc', position: 'bottom' },
  { selector: '[data-tour="messages-search"]', title: 'tour.messagesSearch.title', description: 'tour.messagesSearch.desc', position: 'bottom' },
  { selector: '[data-tour="messages-query"]', title: 'tour.messagesQuery.title', description: 'tour.messagesQuery.desc', position: 'bottom' },
  { selector: '[data-tour="messages-send"]', title: 'tour.messagesSend.title', description: 'tour.messagesSend.desc', position: 'bottom' },
  { selector: '[data-tour="messages-export"]', title: 'tour.messagesExport.title', description: 'tour.messagesExport.desc', position: 'bottom' },
  { selector: '[data-tour="messages-more"]', title: 'tour.messagesMore.title', description: 'tour.messagesMore.desc', position: 'bottom' },
  { selector: '[data-tour="messages-table-header"]', title: 'tour.messagesTableHeader.title', description: 'tour.messagesTableHeader.desc', position: 'bottom' },
  { selector: '[data-tour="messages-table-row"]', title: 'tour.messagesTableRow.title', description: 'tour.messagesTableRow.desc', position: 'left' },
  { selector: '[data-tour="messages-detail-panel"]', title: 'tour.messagesDetailPanel.title', description: 'tour.messagesDetailPanel.desc', position: 'top' },
  { selector: '[data-tour="messages-value-view-format"]', title: 'tour.messagesValueViewFormat.title', description: 'tour.messagesValueViewFormat.desc', position: 'top' },
];

const consumerGroupsSteps: TourStep[] = [
  { selector: '[data-tour="cg-back"]', title: 'tour.cgBack.title', description: 'tour.cgBack.desc', position: 'bottom' },
  { selector: '[data-tour="cg-group-name"]', title: 'tour.cgGroupName.title', description: 'tour.cgGroupName.desc', position: 'bottom' },
  { selector: '[data-tour="cg-state"]', title: 'tour.cgState.title', description: 'tour.cgState.desc', position: 'bottom' },
  { selector: '[data-tour="cg-refresh"]', title: 'tour.cgRefresh.title', description: 'tour.cgRefresh.desc', position: 'bottom' },
  { selector: '[data-tour="cg-actions"]', title: 'tour.cgActions.title', description: 'tour.cgActions.desc', position: 'bottom' },
  { selector: '[data-tour="cg-list"]', title: 'tour.cgList.title', description: 'tour.cgList.desc', position: 'top' },
  { selector: '[data-tour="cg-table-topic"]', title: 'tour.cgTableTopic.title', description: 'tour.cgTableTopic.desc', position: 'top' },
  { selector: '[data-tour="cg-table-partition"]', title: 'tour.cgTablePartition.title', description: 'tour.cgTablePartition.desc', position: 'top' },
  { selector: '[data-tour="cg-table-lag"]', title: 'tour.cgTableLag.title', description: 'tour.cgTableLag.desc', position: 'top' },
  { selector: '[data-tour="cg-table-row"]', title: 'tour.cgTableRow.title', description: 'tour.cgTableRow.desc', position: 'left' },
  { selector: '[data-tour="cg-empty-state"]', title: 'tour.cgEmptyState.title', description: 'tour.cgEmptyState.desc', position: 'bottom' },
  { selector: '[data-tour="cg-reset-dialog"]', title: 'tour.cgResetDialog.title', description: 'tour.cgResetDialog.desc', position: 'top' },
  { selector: '[data-tour="cg-no-group"]', title: 'tour.cgNoGroup.title', description: 'tour.cgNoGroup.desc', position: 'bottom' },
];

const favoritesSteps: TourStep[] = [
  { selector: '[data-tour="fav-title"]', title: 'tour.favTitle.title', description: 'tour.favTitle.desc', position: 'bottom' },
  { selector: '[data-tour="fav-add"]', title: 'tour.favoritesAdd.title', description: 'tour.favoritesAdd.desc', position: 'bottom' },
  { selector: '[data-tour="fav-list"]', title: 'tour.favoritesList.title', description: 'tour.favoritesList.desc', position: 'right' },
  { selector: '[data-tour="fav-group-card"]', title: 'tour.favGroupCard.title', description: 'tour.favGroupCard.desc', position: 'right' },
  { selector: '[data-tour="fav-edit-group"]', title: 'tour.favEditGroup.title', description: 'tour.favEditGroup.desc', position: 'top' },
  { selector: '[data-tour="fav-delete-group"]', title: 'tour.favDeleteGroup.title', description: 'tour.favDeleteGroup.desc', position: 'top' },
  { selector: '[data-tour="fav-group-search"]', title: 'tour.favGroupSearch.title', description: 'tour.favGroupSearch.desc', position: 'right' },
  { selector: '[data-tour="fav-item-row"]', title: 'tour.favItemRow.title', description: 'tour.favItemRow.desc', position: 'left' },
  { selector: '[data-tour="fav-edit-item"]', title: 'tour.favEditItem.title', description: 'tour.favEditItem.desc', position: 'top' },
  { selector: '[data-tour="fav-delete-item"]', title: 'tour.favDeleteItem.title', description: 'tour.favDeleteItem.desc', position: 'top' },
  { selector: '[data-tour="fav-empty-state"]', title: 'tour.favEmptyState.title', description: 'tour.favEmptyState.desc', position: 'bottom' },
  { selector: '[data-tour="fav-group-modal"]', title: 'tour.favGroupModal.title', description: 'tour.favGroupModal.desc', position: 'top' },
  { selector: '[data-tour="fav-favorite-modal"]', title: 'tour.favFavoriteModal.title', description: 'tour.favFavoriteModal.desc', position: 'top' },
];

const settingsSteps: TourStep[] = [
  { selector: '[data-tour="settings-title"]', title: 'tour.settingsTitle.title', description: 'tour.settingsTitle.desc', position: 'bottom' },
  { selector: '[data-tour="settings-language"]', title: 'tour.settingsLanguage.title', description: 'tour.settingsLanguage.desc', position: 'right' },
  { selector: '[data-tour="settings-theme"]', title: 'tour.settingsTheme.title', description: 'tour.settingsTheme.desc', position: 'right' },
  { selector: '[data-tour="settings-sidebar-mode"]', title: 'tour.settingsSidebarMode.title', description: 'tour.settingsSidebarMode.desc', position: 'right' },
  { selector: '[data-tour="settings-version"]', title: 'tour.settingsVersion.title', description: 'tour.settingsVersion.desc', position: 'right' },
  { selector: '[data-tour="settings-version-badge"]', title: 'tour.settingsVersionBadge.title', description: 'tour.settingsVersionBadge.desc', position: 'top' },
  { selector: '[data-tour="settings-check-update"]', title: 'tour.settingsCheckUpdate.title', description: 'tour.settingsCheckUpdate.desc', position: 'top' },
  { selector: '[data-tour="settings-import-export"]', title: 'tour.settingsImportExport.title', description: 'tour.settingsImportExport.desc', position: 'right' },
  { selector: '[data-tour="settings-json-highlight"]', title: 'tour.settingsJsonHighlight.title', description: 'tour.settingsJsonHighlight.desc', position: 'top' },
  { selector: '[data-tour="settings-sections"]', title: 'tour.settingsSections.title', description: 'tour.settingsSections.desc', position: 'top' },
];

const schemaRegistrySteps: TourStep[] = [
  { selector: '[data-tour="sr-title"]', title: 'tour.schemaRegistryTitle.title', description: 'tour.schemaRegistryTitle.desc', position: 'bottom' },
  { selector: '[data-tour="sr-actions"]', title: 'tour.schemaRegistryActions.title', description: 'tour.schemaRegistryActions.desc', position: 'bottom' },
  { selector: '[data-tour="sr-no-cluster"]', title: 'tour.schemaRegistryNoCluster.title', description: 'tour.schemaRegistryNoCluster.desc', position: 'bottom' },
  { selector: '[data-tour="sr-not-configured"]', title: 'tour.schemaRegistryNotConfig.title', description: 'tour.schemaRegistryNotConfig.desc', position: 'bottom' },
  { selector: '[data-tour="sr-config"]', title: 'tour.schemaRegistryConfig.title', description: 'tour.schemaRegistryConfig.desc', position: 'right' },
  { selector: '[data-tour="sr-config-card"]', title: 'tour.schemaRegistryConfigCard.title', description: 'tour.schemaRegistryConfigCard.desc', position: 'top' },
  { selector: '[data-tour="sr-config-actions"]', title: 'tour.schemaRegistryConfigActions.title', description: 'tour.schemaRegistryConfigActions.desc', position: 'top' },
  { selector: '[data-tour="sr-subjects-card"]', title: 'tour.schemaRegistrySubjectsCard.title', description: 'tour.schemaRegistrySubjectsCard.desc', position: 'top' },
  { selector: '[data-tour="sr-subjects-table"]', title: 'tour.schemaRegistrySubjectsTable.title', description: 'tour.schemaRegistrySubjectsTable.desc', position: 'top' },
  { selector: '[data-tour="sr-cluster-dialog"]', title: 'tour.schemaRegistryClusterDialog.title', description: 'tour.schemaRegistryClusterDialog.desc', position: 'top' },
  { selector: '[data-tour="sr-config-dialog"]', title: 'tour.schemaRegistryConfigDialog.title', description: 'tour.schemaRegistryConfigDialog.desc', position: 'top' },
  { selector: '[data-tour="sr-schema-dialog"]', title: 'tour.schemaRegistrySchemaDialog.title', description: 'tour.schemaRegistrySchemaDialog.desc', position: 'top' },
  { selector: '[data-tour="sr-register-dialog"]', title: 'tour.schemaRegistryRegisterDialog.title', description: 'tour.schemaRegistryRegisterDialog.desc', position: 'top' },
];

const topicConsumerGroupsSteps: TourStep[] = [
  { selector: '[data-tour="tcg-title"]', title: 'tour.tcgTitle.title', description: 'tour.tcgTitle.desc', position: 'bottom' },
  { selector: '[data-tour="tcg-data-notice"]', title: 'tour.tcgDataNotice.title', description: 'tour.tcgDataNotice.desc', position: 'bottom' },
  { selector: '[data-tour="tcg-refresh"]', title: 'tour.tcgRefresh.title', description: 'tour.tcgRefresh.desc', position: 'bottom' },
  { selector: '[data-tour="tcg-table"]', title: 'tour.tcgTable.title', description: 'tour.tcgTable.desc', position: 'top' },
];

export const tourDefinitions: Record<string, TourStep[]> = {
  '/clusters': [...globalSteps, ...sidebarSteps, ...clustersSteps],
  '/topics': [...globalSteps, ...sidebarSteps, ...topicsSteps],
  '/messages': [...globalSteps, ...sidebarSteps, ...messagesSteps],
  '/consumer-groups': [...globalSteps, ...sidebarSteps, ...consumerGroupsSteps],
  '/favorites': [...globalSteps, ...sidebarSteps, ...favoritesSteps],
  '/settings': [...globalSteps, ...sidebarSteps, ...settingsSteps],
  '/schema-registry': [...globalSteps, ...sidebarSteps, ...schemaRegistrySteps],
  '/topic-consumer-groups': [...globalSteps, ...sidebarSteps, ...topicConsumerGroupsSteps],
};

// 默认 tour（未匹配时使用全局步骤 + 侧边栏步骤）
export const defaultTourSteps = [...globalSteps, ...sidebarSteps];

// 导出树形模式侧边栏步骤供外部使用
export { treeSidebarSteps };
