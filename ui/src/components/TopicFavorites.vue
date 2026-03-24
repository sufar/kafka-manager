<template>
  <div class="topic-favorites">
    <!-- 收藏分组列表 -->
    <div class="favorite-groups">
      <div class="flex items-center justify-end mb-3">
        <button class="btn btn-primary btn-sm" @click="openCreateGroupModal">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          {{ t.favorites?.addGroup || '新建分组' }}
        </button>
      </div>

      <div v-if="loading" class="flex items-center justify-center py-8">
        <span class="loading loading-spinner loading-md text-primary"></span>
      </div>

      <div v-else-if="groups.length === 0" class="text-center py-8 text-base-content/50">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-12 h-12 mx-auto mb-2 opacity-50">
          <path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.563 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
        </svg>
        <p>{{ t.favorites?.empty || '暂无收藏分组' }}</p>
        <p class="text-sm mt-1">{{ t.favorites?.emptyHint || '点击右上角创建分组' }}</p>
      </div>

      <!-- Group search state management -->
      <div v-else class="space-y-3">
        <div v-for="group in groups" :key="group.id" class="group-card">
          <!-- 分组标题 -->
          <div class="group-header" @click="toggleGroup(group.id)">
            <div class="flex items-center gap-2 flex-1">
              <svg v-if="expandedGroups.has(group.id)" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 flex-shrink-0">
                <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
              </svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 flex-shrink-0">
                <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" />
              </svg>
              <span class="font-semibold flex-shrink-0">{{ group.name }}</span>
              <span v-if="group.description" class="text-xs text-base-content/40 truncate max-w-[100px] sm:max-w-[150px]" :title="group.description">{{ group.description }}</span>
              <span class="badge badge-sm badge-ghost flex-shrink-0">{{ filteredGroupItems(group.id)?.length || 0 }}</span>
            </div>
            <div class="flex items-center gap-1 flex-shrink-0">
              <button class="btn btn-ghost btn-xs" @click.stop="editGroup(group)" title="编辑分组">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" />
                </svg>
              </button>
              <button class="btn btn-ghost btn-xs text-error" @click.stop="deleteGroup(group.id)" title="删除分组">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
                </svg>
              </button>
            </div>
          </div>

          <!-- 搜索框 (在分组名下一行) -->
          <div v-show="expandedGroups.has(group.id)" class="group-search px-3 pt-2 pb-1">
            <div class="relative">
              <input
                v-model="groupSearches[group.id]"
                type="text"
                class="input input-bordered input-sm w-full pl-8"
                :placeholder="(t.favorites?.searchPlaceholder || '搜索 Topic 名、备注...')"
                @click.stop
              />
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 absolute left-2.5 top-1/2 -translate-y-1/2 text-base-content/40">
                <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
              </svg>
              <button
                v-if="groupSearches[group.id]"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/40 hover:text-base-content"
                @click.stop="groupSearches[group.id] = ''"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>

          <!-- 分组内容 -->
          <div v-show="expandedGroups.has(group.id)" class="group-content">
            <div v-if="!filteredGroupItems(group.id) || filteredGroupItems(group.id).length === 0" class="text-center py-4 text-base-content/40 text-sm">
              {{ groupSearches[group.id] ? (t.favorites?.noSearchResults || '无匹配的收藏') : (t.favorites?.noItems || '该分组暂无收藏') }}
            </div>
            <div v-else class="favorite-items">
              <div v-for="item in filteredGroupItems(group.id)" :key="item.id" class="favorite-item" @dblclick="navigateToTopic(item.cluster_id, item.topic_name)">
                <div class="flex items-center gap-2 flex-1 min-w-0">
                  <div class="w-5 h-5 rounded bg-primary/10 flex items-center justify-center flex-shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3 text-primary">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0v3.75" />
                    </svg>
                  </div>
                  <div class="flex items-center gap-2 flex-1 min-w-0">
                    <span class="font-medium text-xs truncate flex-shrink-0 min-w-0" :title="item.topic_name">{{ item.topic_name }}</span>
                    <span class="text-[10px] text-base-content/40 flex-shrink-0 hidden sm:inline">·</span>
                    <span class="badge badge-ghost badge-[10px] text-[9px] px-1 flex-shrink-0 truncate max-w-[80px] sm:max-w-[100px]" :title="item.cluster_id">{{ item.cluster_id }}</span>
                    <span v-if="item.description" class="text-[10px] text-base-content/40 truncate flex-shrink-0 hidden sm:inline max-w-[120px] lg:max-w-[180px]" :title="item.description">{{ item.description }}</span>
                  </div>
                </div>
                <div class="flex items-center gap-0.5 flex-shrink-0">
                  <button class="btn btn-ghost btn-xs px-1 h-auto" @click.stop="editFavorite(item)" title="编辑">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" />
                    </svg>
                  </button>
                  <button class="btn btn-ghost btn-xs px-1 h-auto text-error" @click.stop="deleteFavorite(item.id)" title="删除收藏">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 创建/编辑分组模态框 -->
    <Teleport to="body">
      <dialog ref="groupModalRef" class="modal" @click.self="closeGroupModal">
        <div class="modal-box w-full max-w-md mx-2 md:mx-auto">
          <h3 class="font-bold text-lg mb-4">{{ editingGroup ? (t.favorites?.editGroup || '编辑分组') : (t.favorites?.createGroup || '创建分组') }}</h3>
          <form @submit.prevent="saveGroup" class="space-y-4">
            <div>
              <label class="label">
                <span class="label-text">{{ t.favorites?.groupName || '分组名称' }}</span>
                <span class="label-text-alt text-error">*</span>
              </label>
              <input v-model="groupForm.name" type="text" maxlength="15" class="input input-bordered w-full" :placeholder="t.favorites?.groupNamePlaceholder || '请输入分组名称'" required />
            </div>
            <div>
              <label class="label">
                <span class="label-text">{{ t.favorites?.groupDescription || '分组描述' }}</span>
              </label>
              <textarea v-model="groupForm.description" class="textarea textarea-bordered w-full" :placeholder="t.favorites?.groupDescPlaceholder || '请输入分组描述（可选）'" rows="2"></textarea>
            </div>
            <div>
              <label class="label">
                <span class="label-text">{{ t.favorites?.sortOrder || '排序' }}</span>
              </label>
              <input v-model.number="groupForm.sort_order" type="number" class="input input-bordered w-full" :placeholder="t.favorites?.sortOrderPlaceholder || '数字越小越靠前'" />
            </div>
            <div class="modal-action">
              <button type="button" class="btn" @click="closeGroupModal">{{ t.common?.cancel || '取消' }}</button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="loading loading-spinner loading-xs"></span>
                {{ t.common?.save || '保存' }}
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- 编辑收藏模态框 -->
    <Teleport to="body">
      <dialog ref="favoriteModalRef" class="modal" @click.self="closeFavoriteModal">
        <div class="modal-box w-full max-w-md mx-2 md:mx-auto">
          <h3 class="font-bold text-lg mb-4">{{ t.favorites?.editFavorite || '编辑收藏' }}</h3>
          <form @submit.prevent="saveFavorite" class="space-y-4">
            <div>
              <label class="label">
                <span class="label-text">{{ t.favorites?.selectGroup || '选择分组' }}</span>
              </label>
              <select v-model.number="favoriteForm.group_id" class="select select-bordered w-full">
                <option v-for="group in groups" :key="group.id" :value="group.id">{{ group.name }}</option>
              </select>
            </div>
            <div>
              <label class="label">
                <span class="label-text">{{ t.favorites?.favoriteDescription || '描述' }}</span>
              </label>
              <textarea v-model="favoriteForm.description" class="textarea textarea-bordered w-full" :placeholder="t.favorites?.favoriteDescPlaceholder || '请输入描述（可选）'" rows="2"></textarea>
            </div>
            <div>
              <label class="label">
                <span class="label-text">{{ t.favorites?.sortOrder || '排序' }}</span>
              </label>
              <input v-model.number="favoriteForm.sort_order" type="number" class="input input-bordered w-full" :placeholder="t.favorites?.sortOrderPlaceholder || '数字越小越靠前'" />
            </div>
            <div class="modal-action">
              <button type="button" class="btn" @click="closeFavoriteModal">{{ t.common?.cancel || '取消' }}</button>
              <button type="submit" class="btn btn-primary" :disabled="saving">
                <span v-if="saving" class="loading loading-spinner loading-xs"></span>
                {{ t.common?.save || '保存' }}
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const { showSuccess, showError, confirm } = useToast();

// Props
const props = defineProps<{
  t: Record<string, any>;
}>();

// State
const loading = ref(false);
const saving = ref(false);
const groups = ref<any[]>([]);
const expandedGroups = ref<Set<number>>(new Set());
const groupSearches = ref<Record<number, string>>({}); // Search query for each group
const editingGroup = ref<any>(null);
const editingFavorite = ref<any>(null);
const groupModalRef = ref<HTMLDialogElement>();
const favoriteModalRef = ref<HTMLDialogElement>();

const groupForm = ref({
  name: '',
  description: '',
  sort_order: 0,
});

const favoriteForm = ref({
  group_id: 0,
  description: '',
  sort_order: 0,
});

// Load favorites
async function loadFavorites() {
  loading.value = true;
  try {
    const data = await apiClient.getFavorites();
    groups.value = data;
    // Expand first group by default
    if (data.length > 0 && expandedGroups.value.size === 0) {
      const firstGroup = data[0];
      if (firstGroup) {
        expandedGroups.value.add(firstGroup.id);
      }
    }
  } catch (error: any) {
    showError(error.message || '加载收藏失败');
  } finally {
    loading.value = false;
  }
}

// Toggle group expansion
function toggleGroup(groupId: number) {
  if (expandedGroups.value.has(groupId)) {
    expandedGroups.value.delete(groupId);
  } else {
    expandedGroups.value.add(groupId);
  }
}

// Filter group items by search query - search only topic name and description (no cluster search)
function filteredGroupItems(groupId: number) {
  const group = groups.value.find(g => g.id === groupId);
  if (!group || !group.items) return [];

  const searchQuery = groupSearches.value[groupId];
  if (!searchQuery || searchQuery.trim() === '') {
    return group.items;
  }

  const query = searchQuery.toLowerCase();
  return group.items.filter((item: any) => {
    const matchTopic = item.topic_name?.toLowerCase().includes(query);
    const matchDesc = item.description?.toLowerCase().includes(query);
    return matchTopic || matchDesc;
  });
}

// Group CRUD
function openCreateGroupModal() {
  editingGroup.value = null;
  groupForm.value = { name: '', description: '', sort_order: 0 };
  groupModalRef.value?.showModal();
}

function editGroup(group: any) {
  editingGroup.value = group;
  groupForm.value = {
    name: group.name,
    description: group.description || '',
    sort_order: group.sort_order,
  };
  groupModalRef.value?.showModal();
}

function closeGroupModal() {
  groupModalRef.value?.close();
  editingGroup.value = null;
  groupForm.value = { name: '', description: '', sort_order: 0 };
}

async function saveGroup() {
  saving.value = true;
  try {
    if (editingGroup.value) {
      await apiClient.updateFavoriteGroup(editingGroup.value.id, {
        name: groupForm.value.name,
        description: groupForm.value.description || undefined,
        sort_order: groupForm.value.sort_order,
      });
      showSuccess('分组更新成功');
    } else {
      await apiClient.createFavoriteGroup({
        name: groupForm.value.name,
        description: groupForm.value.description || undefined,
        sort_order: groupForm.value.sort_order,
      });
      showSuccess('分组创建成功');
    }
    closeGroupModal();
    await loadFavorites();
  } catch (error: any) {
    showError(error.message || '保存失败');
  } finally {
    saving.value = false;
  }
}

async function deleteGroup(id: number) {
  if (!await confirm(props.t.favorites?.confirmDeleteGroup || '确定要删除这个分组吗？分组内的收藏也会被删除。')) {
    return;
  }
  try {
    await apiClient.deleteFavoriteGroup(id);
    showSuccess('分组删除成功');
    await loadFavorites();
  } catch (error: any) {
    showError(error.message || '删除失败');
  }
}

// Favorite CRUD
function editFavorite(item: any) {
  editingFavorite.value = item;
  favoriteForm.value = {
    group_id: item.group_id,
    description: item.description || '',
    sort_order: item.sort_order,
  };
  favoriteModalRef.value?.showModal();
}

function closeFavoriteModal() {
  favoriteModalRef.value?.close();
  editingFavorite.value = null;
  favoriteForm.value = { group_id: 0, description: '', sort_order: 0 };
}

async function saveFavorite() {
  if (!editingFavorite.value) return;

  saving.value = true;
  try {
    await apiClient.updateFavorite(editingFavorite.value.id, {
      group_id: favoriteForm.value.group_id,
      description: favoriteForm.value.description || undefined,
      sort_order: favoriteForm.value.sort_order,
    });
    showSuccess('收藏更新成功');
    closeFavoriteModal();
    await loadFavorites();
  } catch (error: any) {
    showError(error.message || '保存失败');
  } finally {
    saving.value = false;
  }
}

async function deleteFavorite(id: number) {
  if (!await confirm(props.t.favorites?.confirmDeleteFavorite || '确定要删除这个收藏吗？')) {
    return;
  }
  try {
    await apiClient.deleteFavorite(id);
    showSuccess('收藏删除成功');
    await loadFavorites();
  } catch (error: any) {
    showError(error.message || '删除失败');
  }
}

// Navigate to topic - 跳转到Topics列表并搜索该topic
function navigateToTopic(clusterId: string, topicName: string) {
  // 触发事件让 ModernLayout 统一处理导航
  // ModernLayout 知道当前的 sidebar mode，可以做出正确的处理
  window.dispatchEvent(new CustomEvent('navigate-to-topic-from-favorites', {
    detail: { clusterId, topicName }
  }));
}

onMounted(() => {
  loadFavorites();
});
</script>

<style scoped>
.topic-favorites {
  padding: 1rem;
}

.favorite-groups {
  max-width: 100%;
}

.group-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 12px;
  overflow: hidden;
}

:root[data-theme="light"] .group-card {
  background: rgba(0, 0, 0, 0.02);
  border-color: rgba(0, 0, 0, 0.1);
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  cursor: pointer;
  transition: background 0.2s;
}

.group-header:hover {
  background: rgba(255, 255, 255, 0.05);
}

:root[data-theme="light"] .group-header:hover {
  background: rgba(0, 0, 0, 0.03);
}

.group-content {
  padding: 0.25rem;
  border-top: 1px solid rgba(255, 255, 255, 0.03);
}

:root[data-theme="light"] .group-content {
  border-top-color: rgba(0, 0, 0, 0.05);
}

.group-search {
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
}

:root[data-theme="light"] .group-search {
  border-bottom-color: rgba(0, 0, 0, 0.05);
}

.favorite-items {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.favorite-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.375rem;
  padding: 0.375rem 0.5rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.favorite-item:hover {
  background: rgba(99, 102, 241, 0.1);
}

.favorite-item:active {
  transform: scale(0.98);
}
</style>
