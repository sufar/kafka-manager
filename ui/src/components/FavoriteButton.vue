<template>
  <button
    class="favorite-btn"
    :class="{ 'is-favorite': isFavorite }"
    @click.stop="toggleFavorite"
    :title="isFavorite ? (t.favorites?.remove || '取消收藏') : (t.favorites?.add || '收藏')"
  >
    <svg
      xmlns="http://www.w3.org/2000/svg"
      :fill="isFavorite ? 'currentColor' : 'none'"
      viewBox="0 0 24 24"
      stroke-width="1.5"
      stroke="currentColor"
      class="w-4 h-4"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.563 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z"
      />
    </svg>
  </button>

  <!-- 选择分组弹窗 -->
  <Teleport to="body">
    <dialog ref="modalRef" class="modal" @click.self="closeModal">
      <div class="modal-box w-full max-w-sm mx-2 md:mx-auto">
        <h3 class="font-bold text-lg mb-4">{{ t.favorites?.selectGroup || '选择收藏分组' }}</h3>
        <div v-if="loading" class="flex items-center justify-center py-8">
          <span class="loading loading-spinner loading-md text-primary"></span>
        </div>
        <div v-else-if="groups.length === 0 || showCreateGroupForm" class="text-center py-4">
          <div v-if="!showCreateGroupForm">
            <p class="text-base-content/50">{{ t.favorites?.noGroups || '暂无分组' }}</p>
            <button class="btn btn-primary btn-sm mt-2" @click="openCreateGroupForm">
              {{ t.favorites?.createGroup || '创建分组' }}
            </button>
          </div>
          <div v-else class="space-y-3">
            <h4 class="font-medium text-sm text-left">{{ t.favorites?.createGroup || '创建分组' }}</h4>
            <div class="space-y-2">
              <input
                v-model="newGroupName"
                type="text"
                class="input input-bordered w-full input-sm"
                :placeholder="t.favorites?.groupNamePlaceholder || '请输入分组名称'"
                @keyup.enter="submitCreateGroup"
              />
              <input
                v-model="newGroupDesc"
                type="text"
                class="input input-bordered w-full input-sm"
                :placeholder="t.favorites?.groupDescPlaceholder || '请输入分组描述（可选）'"
                @keyup.enter="submitCreateGroup"
              />
            </div>
            <div class="flex gap-2 justify-end">
              <button class="btn btn-ghost btn-sm" @click="cancelCreateGroup">
                {{ t.common?.cancel || '取消' }}
              </button>
              <button
                class="btn btn-primary btn-sm"
                :disabled="!newGroupName.trim() || creatingGroup"
                @click="submitCreateGroup"
              >
                <span v-if="creatingGroup" class="loading loading-spinner loading-xs"></span>
                {{ t.common?.save || '保存' }}
              </button>
            </div>
          </div>
        </div>
        <div v-else class="space-y-2 max-h-64 overflow-y-auto">
          <div
            v-for="group in groups"
            :key="group.id"
            class="group-option"
            :class="{ 'is-selected': selectedGroupId === group.id }"
            @click="selectedGroupId = group.id"
          >
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
              </svg>
              <span>{{ group.name }}</span>
              <span class="badge badge-sm badge-ghost">{{ group.item_count || 0 }}</span>
            </div>
          </div>
        </div>
        <!-- 备注输入 -->
        <div v-if="groups.length > 0" class="mt-4">
          <label class="label">
            <span class="label-text">{{ t.favorites?.remark || '备注' }}</span>
            <span class="label-text-alt text-base-content/50">{{ t.common?.optional || '可选' }}</span>
          </label>
          <textarea
            v-model="remark"
            class="textarea textarea-bordered w-full textarea-sm"
            :placeholder="t.favorites?.remarkPlaceholder || '添加备注（可选）'"
            rows="2"
          ></textarea>
        </div>
        <div class="modal-action">
          <button type="button" class="btn btn-ghost" @click="closeModal">{{ t.common?.cancel || '取消' }}</button>
          <button
            type="button"
            class="btn btn-square btn-sm text-primary border border-primary/50 hover:bg-primary hover:text-white"
            :disabled="!selectedGroupId || saving"
            @click="confirmAdd"
          >
            <span v-if="saving" class="loading loading-spinner loading-xs"></span>
            {{ t.common?.confirm || '确定' }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop" @click="closeModal">
        <button>close</button>
      </form>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const { showSuccess, showError } = useToast();

// Props
const props = defineProps<{
  clusterId: string;
  topicName: string;
  t: Record<string, any>;
}>();

// Emits
const emit = defineEmits<{
  (e: 'update'): void;
}>();

// State
const isFavorite = ref(false);
const loading = ref(false);
const saving = ref(false);
const groups = ref<any[]>([]);
const selectedGroupId = ref<number | null>(null);
const remark = ref('');
const modalRef = ref<HTMLDialogElement>();

// Create group form state
const showCreateGroupForm = ref(false);
const newGroupName = ref('');
const newGroupDesc = ref('');
const creatingGroup = ref(false);

// Check if topic is favorite
async function checkFavorite() {
  try {
    isFavorite.value = await apiClient.checkFavorite(props.clusterId, props.topicName);
  } catch (error) {
    console.error('Failed to check favorite:', error);
  }
}

// Toggle favorite
async function toggleFavorite() {
  if (isFavorite.value) {
    // Remove from favorites
    try {
      await apiClient.deleteFavoriteByTopic(props.clusterId, props.topicName);
      isFavorite.value = false;
      showSuccess(props.t.favorites?.removed || '已取消收藏');
      emit('update');
    } catch (error: any) {
      showError(error.message || '取消收藏失败');
    }
  } else {
    // Show group selection modal
    await loadGroups();
    selectedGroupId.value = groups.value.length > 0 ? groups.value[0].id : null;
    showCreateGroupForm.value = false;
    newGroupName.value = '';
    newGroupDesc.value = '';
    modalRef.value?.showModal();
  }
}

// Load groups
async function loadGroups() {
  loading.value = true;
  try {
    const data = await apiClient.getFavoriteGroups();
    groups.value = data;
  } catch (error: any) {
    showError(error.message || '加载分组失败');
  } finally {
    loading.value = false;
  }
}

// Open create group form
function openCreateGroupForm() {
  showCreateGroupForm.value = true;
  newGroupName.value = '';
  newGroupDesc.value = '';
}

// Cancel create group
function cancelCreateGroup() {
  showCreateGroupForm.value = false;
  newGroupName.value = '';
  newGroupDesc.value = '';
}

// Submit create group
async function submitCreateGroup() {
  if (!newGroupName.value.trim()) return;

  creatingGroup.value = true;
  try {
    const newGroup = await apiClient.createFavoriteGroup({
      name: newGroupName.value.trim(),
      description: newGroupDesc.value.trim() || undefined,
      sort_order: 0,
    });

    // Add to groups list and select it
    groups.value.push({
      ...newGroup,
      item_count: 0,
    });
    selectedGroupId.value = newGroup.id;

    // Reset form
    showCreateGroupForm.value = false;
    newGroupName.value = '';
    newGroupDesc.value = '';

    showSuccess(props.t.favorites?.groupCreated || '分组创建成功');
  } catch (error: any) {
    showError(error.message || '创建分组失败');
  } finally {
    creatingGroup.value = false;
  }
}

// Confirm add to favorites
async function confirmAdd() {
  if (!selectedGroupId.value) return;

  saving.value = true;
  try {
    await apiClient.createFavorite({
      group_id: selectedGroupId.value,
      cluster_id: props.clusterId,
      topic_name: props.topicName,
      description: remark.value || undefined,
    });
    isFavorite.value = true;
    showSuccess(props.t.favorites?.added || '已添加到收藏');
    closeModal();
    emit('update');
  } catch (error: any) {
    showError(error.message || '添加收藏失败');
  } finally {
    saving.value = false;
  }
}

// Close modal
function closeModal() {
  modalRef.value?.close();
  selectedGroupId.value = null;
  remark.value = '';
  showCreateGroupForm.value = false;
  newGroupName.value = '';
  newGroupDesc.value = '';
}

onMounted(() => {
  checkFavorite();
});
</script>

<style scoped>
.favorite-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.25rem;
  border-radius: 6px;
  transition: all 0.2s;
  color: currentColor;
  opacity: 0.5;
}

.favorite-btn:hover {
  opacity: 1;
  background: rgba(99, 102, 241, 0.1);
}

.favorite-btn.is-favorite {
  color: #fbbf24;
  opacity: 1;
}

.favorite-btn.is-favorite:hover {
  color: #ef4444;
}

.group-option {
  display: flex;
  align-items: center;
  padding: 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
}

.group-option:hover {
  background: rgba(99, 102, 241, 0.05);
}

.group-option.is-selected {
  background: rgba(99, 102, 241, 0.1);
  border-color: rgba(99, 102, 241, 0.3);
}
</style>
