<template>
  <button
    class="favorite-btn"
    :class="{ 'is-favorite': isFavorite }"
    @click.stop="toggleFavorite"
    :title="isFavorite ? (t.favorites?.remove || 'Remove from Favorites') : (t.favorites?.add || 'Add to Favorites')"
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
    <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle" @click.self="closeModal">
      <div class="modal-box w-full max-w-sm mx-2 md:mx-auto p-5 overflow-visible">
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2">
            <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.563 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z" />
              </svg>
            </div>
            <div>
              <h3 class="font-bold text-base">{{ t.favorites?.selectGroup || 'Select Group' }}</h3>
            </div>
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeModal">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="space-y-4">
          <div v-if="loading" class="flex items-center justify-center py-8">
            <span class="loading loading-spinner loading-md text-primary"></span>
          </div>
          <div v-else-if="groups.length === 0" class="text-center py-4">
            <div v-if="!showCreateGroupForm">
              <p class="text-base-content/50 mb-3">{{ t.favorites?.noGroups || 'No groups yet' }}</p>
              <button class="btn btn-primary btn-sm" @click="openCreateGroupForm">
                {{ t.favorites?.createGroup || 'Create Group' }}
              </button>
            </div>
            <div v-else class="space-y-3 text-left">
              <h4 class="font-medium text-sm">{{ t.favorites?.createGroup || 'Create Group' }}</h4>
              <div class="space-y-2">
                <input
                  v-model="newGroupName"
                  type="text"
                  class="input input-bordered w-full input-sm"
                  :placeholder="t.favorites?.groupNamePlaceholder || 'Enter group name'"
                  @keyup.enter="submitCreateGroup"
                />
                <input
                  v-model="newGroupDesc"
                  type="text"
                  class="input input-bordered w-full input-sm"
                  :placeholder="t.favorites?.groupDescPlaceholder || 'Enter description (optional)'"
                  @keyup.enter="submitCreateGroup"
                />
              </div>
              <div class="flex gap-2 justify-end">
                <button class="btn btn-ghost btn-sm" @click="cancelCreateGroup">
                  {{ t.common?.cancel || 'Cancel' }}
                </button>
                <button
                  class="btn btn-primary btn-sm"
                  :disabled="!newGroupName.trim() || creatingGroup"
                  @click="submitCreateGroup"
                >
                  <span v-if="creatingGroup" class="loading loading-spinner loading-xs"></span>
                  {{ t.common?.save || 'Save' }}
                </button>
              </div>
            </div>
          </div>
          <div v-else class="space-y-2">
            <!-- 新增分组按钮 -->
            <button
              v-if="!showCreateGroupForm"
              class="w-full p-2 rounded-lg cursor-pointer transition-colors hover:bg-base-200 border border-dashed border-base-content/20 text-base-content/60 flex items-center justify-center gap-1"
              @click="openCreateGroupForm"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
              </svg>
              <span class="text-sm">{{ t.favorites?.createGroup || 'Create Group' }}</span>
            </button>

            <!-- 创建分组表单 -->
            <div v-if="showCreateGroupForm" class="p-2 rounded-lg bg-base-200/50 space-y-2">
              <input
                v-model="newGroupName"
                type="text"
                class="input input-bordered w-full input-sm"
                :placeholder="t.favorites?.groupNamePlaceholder || 'Enter group name'"
                @keyup.enter="submitCreateGroup"
              />
              <input
                v-model="newGroupDesc"
                type="text"
                class="input input-bordered w-full input-sm"
                :placeholder="t.favorites?.groupDescPlaceholder || 'Enter description (optional)'"
                @keyup.enter="submitCreateGroup"
              />
              <div class="flex gap-2 justify-end">
                <button class="btn btn-ghost btn-xs" @click="cancelCreateGroup">
                  {{ t.common?.cancel || 'Cancel' }}
                </button>
                <button
                  class="btn btn-primary btn-xs"
                  :disabled="!newGroupName.trim() || creatingGroup"
                  @click="submitCreateGroup"
                >
                  <span v-if="creatingGroup" class="loading loading-spinner loading-xs"></span>
                  {{ t.common?.save || 'Save' }}
                </button>
              </div>
            </div>

            <!-- 分组列表 - 支持滚动 -->
            <div class="max-h-48 overflow-y-auto space-y-1">
              <div
                v-for="group in groups"
                :key="group.id"
                class="p-2 rounded-lg cursor-pointer transition-colors hover:bg-base-200"
                :class="{ 'bg-primary/10 hover:bg-primary/20': selectedGroupId === group.id }"
                @click="selectedGroupId = group.id"
              >
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0121.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
                    </svg>
                    <span>{{ group.name }}</span>
                  </div>
                  <span class="badge badge-sm badge-ghost">{{ group.item_count || 0 }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 备注输入 -->
          <div v-if="groups.length > 0" class="pt-2 border-t border-base-content/10">
            <label class="block text-sm font-medium mb-1">
              <span class="flex items-center gap-1">
                <span>{{ t.favorites?.remark || 'Remark' }}</span>
                <span class="text-xs text-base-content/50">{{ t.common?.optional || 'Optional' }}</span>
              </span>
            </label>
            <textarea
              v-model="remark"
              class="textarea textarea-bordered w-full textarea-sm"
              :placeholder="t.favorites?.remarkPlaceholder || 'Add remark (optional)'"
              rows="2"
            ></textarea>
          </div>

          <!-- Actions -->
          <div class="modal-action flex-wrap gap-2 pt-3">
            <button type="button" class="btn btn-ghost btn-sm" @click="closeModal">{{ t.common?.cancel || 'Cancel' }}</button>
            <button
              type="button"
              class="btn btn-primary btn-sm"
              :disabled="!selectedGroupId || saving"
              @click="confirmAdd"
            >
              <span v-if="saving" class="loading loading-spinner loading-xs"></span>
              {{ t.common?.confirm || 'Confirm' }}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const { showSuccess, showError } = useToast();

// Props
const props = defineProps<{
  clusterId: string;
  topicName: string;
  t: Record<string, any>;
  favoriteCache?: Set<string>;
}>();

// Emits
const emit = defineEmits<{
  (e: 'update'): void;
  (e: 'change', isFavorite: boolean): void;
}>();

// State
const isFavorite = ref(false);
const loading = ref(false);
const saving = ref(false);
const groups = ref<any[]>([]);
const selectedGroupId = ref<number | null>(null);
const remark = ref('');
const showModal = ref(false);
const modalRef = ref<HTMLDialogElement | null>(null);

// Create group form state
const showCreateGroupForm = ref(false);
const newGroupName = ref('');
const newGroupDesc = ref('');
const creatingGroup = ref(false);

// Check if topic is favorite
async function checkFavorite() {
  // If favoriteCache is provided, use it instead of making an API call
  if (props.favoriteCache) {
    const key = `${props.clusterId}-${props.topicName}`;
    isFavorite.value = props.favoriteCache.has(key);
    return;
  }

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
      showSuccess(props.t.favorites?.removed || 'Removed from favorites');
      emit('update');
      emit('change', false);
    } catch (error: any) {
      showError(error.message || 'Failed to remove from favorites');
    }
  } else {
    // Show group selection modal
    await loadGroups();
    selectedGroupId.value = groups.value.length > 0 ? groups.value[0].id : null;
    showCreateGroupForm.value = false;
    newGroupName.value = '';
    newGroupDesc.value = '';
    showModal.value = true;
    // 打开原生 dialog
    await nextTick();
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
    showError(error.message || 'Failed to load groups');
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

    showSuccess(props.t.favorites?.groupCreated || 'Group created successfully');
  } catch (error: any) {
    showError(error.message || 'Failed to create group');
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
    showSuccess(props.t.favorites?.added || 'Added to favorites');
    closeModal();
    emit('update');
    emit('change', true);
  } catch (error: any) {
    showError(error.message || 'Failed to add to favorites');
  } finally {
    saving.value = false;
  }
}

// Close modal
function closeModal() {
  modalRef.value?.close();
  showModal.value = false;
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

/* 移除背景模糊效果 */
:global(.modal:has(.modal-box)::backdrop) {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:global(dialog[open]::backdrop) {
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
</style>