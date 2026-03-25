<template>
  <div>
    <!-- Cluster Context Menu -->
    <ClusterContextMenu
      v-if="contextMenus.cluster.visible"
      :visible="contextMenus.cluster.visible"
      :cluster-name="contextMenus.cluster.clusterName"
      :position="contextMenus.cluster.position"
      :refreshing="refreshingCluster === contextMenus.cluster.clusterName"
      :testing="testingCluster === contextMenus.cluster.clusterName"
      :disconnecting="disconnectingCluster === contextMenus.cluster.clusterName"
      :reconnecting="reconnectingCluster === contextMenus.cluster.clusterName"
      @close="closeClusterMenu"
      @action="(action, cluster) => $emit('cluster-action', action, cluster)"
    />

    <!-- Topics Folder Context Menu -->
    <TopicsFolderContextMenu
      v-if="contextMenus.topicsFolder.visible"
      :key="`topics-folder-${contextMenus.topicsFolder.clusterName}`"
      :visible="contextMenus.topicsFolder.visible"
      :cluster-name="contextMenus.topicsFolder.clusterName"
      :position="contextMenus.topicsFolder.position"
      @close="closeTopicsFolderMenu"
      @action="(action, cluster) => $emit('topics-folder-action', action, cluster)"
    />

    <!-- Topic Context Menu -->
    <TopicContextMenu
      v-if="contextMenus.topic.visible"
      :visible="contextMenus.topic.visible"
      :topic-name="contextMenus.topic.topicName"
      :cluster-name="contextMenus.topic.clusterName"
      :position="contextMenus.topic.position"
      @close="closeTopicMenu"
      @action="(action, topic, cluster) => $emit('topic-action', action, topic, cluster)"
    />

    <!-- Partition Context Menu -->
    <PartitionContextMenu
      v-if="contextMenus.partition.visible"
      :visible="contextMenus.partition.visible"
      :topic-name="contextMenus.partition.topicName"
      :cluster-name="contextMenus.partition.clusterName"
      :partition-id="contextMenus.partition.partitionId"
      :position="contextMenus.partition.position"
      @close="closePartitionMenu"
      @action="(action, topic, cluster, partitionId) => $emit('partition-action', action, topic, cluster, partitionId)"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import ClusterContextMenu from '@/components/ContextMenus/ClusterContextMenu.vue';
import TopicContextMenu from '@/components/ContextMenus/TopicContextMenu.vue';
import TopicsFolderContextMenu from '@/components/ContextMenus/TopicsFolderContextMenu.vue';
import PartitionContextMenu from '@/components/ContextMenus/PartitionContextMenu.vue';

const emit = defineEmits<{
  'cluster-action': [action: string, cluster: string];
  'topics-folder-action': [action: string, cluster: string];
  'topic-action': [action: string, topic: string, cluster: string];
  'partition-action': [action: string, topic: string, cluster: string, partitionId: number];
}>();

// Context menu state
const contextMenus = reactive({
  cluster: {
    visible: false,
    clusterName: '',
    position: { x: 0, y: 0 }
  },
  topicsFolder: {
    visible: false,
    clusterName: '',
    position: { x: 0, y: 0 }
  },
  topic: {
    visible: false,
    topicName: '',
    clusterName: '',
    position: { x: 0, y: 0 }
  },
  partition: {
    visible: false,
    topicName: '',
    clusterName: '',
    partitionId: 0,
    position: { x: 0, y: 0 }
  }
});

// Cluster action state
const refreshingCluster = ref<string | null>(null);
const testingCluster = ref<string | null>(null);
const disconnectingCluster = ref<string | null>(null);
const reconnectingCluster = ref<string | null>(null);

function closeClusterMenu() {
  contextMenus.cluster.visible = false;
}

function showClusterMenu(clusterName: string, x: number, y: number) {
  contextMenus.cluster.clusterName = clusterName;
  contextMenus.cluster.position = { x, y };
  contextMenus.cluster.visible = true;
}

function closeTopicsFolderMenu() {
  contextMenus.topicsFolder.visible = false;
}

function showTopicsFolderMenu(clusterName: string, x: number, y: number) {
  contextMenus.topicsFolder.clusterName = clusterName;
  contextMenus.topicsFolder.position = { x, y };
  contextMenus.topicsFolder.visible = true;
}

function closeTopicMenu() {
  contextMenus.topic.visible = false;
}

function showTopicMenu(topicName: string, clusterName: string, x: number, y: number) {
  contextMenus.topic.topicName = topicName;
  contextMenus.topic.clusterName = clusterName;
  contextMenus.topic.position = { x, y };
  contextMenus.topic.visible = true;
}

function closePartitionMenu() {
  contextMenus.partition.visible = false;
}

function showPartitionMenu(topicName: string, clusterName: string, partitionId: number, x: number, y: number) {
  contextMenus.partition.topicName = topicName;
  contextMenus.partition.clusterName = clusterName;
  contextMenus.partition.partitionId = partitionId;
  contextMenus.partition.position = { x, y };
  contextMenus.partition.visible = true;
}

// Expose methods to parent
defineExpose({
  showClusterMenu,
  showTopicsFolderMenu,
  showTopicMenu,
  showPartitionMenu,
  closeClusterMenu,
  closeTopicsFolderMenu,
  closeTopicMenu,
  closePartitionMenu,
  contextMenus,
  refreshingCluster,
  testingCluster,
  disconnectingCluster,
  reconnectingCluster
});
</script>
