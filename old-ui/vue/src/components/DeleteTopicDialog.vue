<template>
  <Teleport to="body">
    <dialog ref="dialogRef" class="modal modal-bottom sm:modal-middle" @click.self="close">
      <div class="modal-box w-full max-w-md mx-2 md:mx-auto p-5">
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2">
            <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-error/20 to-error/10 flex items-center justify-center">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-error">
                <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
              </svg>
            </div>
            <div>
              <h3 class="font-bold text-base">{{ t.topics.confirmDeleteTitle }}</h3>
              <span class="text-xs text-base-content/60">{{ t.topics.confirmDeleteHint }}</span>
            </div>
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="close">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="space-y-4">
          <!-- Topic Info -->
          <div class="bg-base-200 rounded-lg p-3">
            <div class="mb-2">
              <label class="text-xs text-base-content/60 block mb-1">Cluster</label>
              <span class="font-medium text-sm">{{ cluster }}</span>
            </div>
            <div>
              <label class="text-xs text-base-content/60 block mb-1">{{ t.topics.topicName }}</label>
              <div class="flex items-center gap-2">
                <code class="text-sm bg-base-100 px-2 py-1 rounded flex-1 truncate block">{{ topic }}</code>
                <button
                  class="btn btn-ghost btn-xs"
                  @click="copyTopicName"
                  :title="t.common.copy"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                  </svg>
                </button>
              </div>
            </div>
          </div>

          <!-- Confirmation Input -->
          <div>
            <label class="block text-sm font-medium mb-1.5">
              {{ t.topics.confirmDeleteInput }}
            </label>
            <input
              v-model="confirmInput"
              type="text"
              :placeholder="topic"
              class="input input-bordered w-full"
              @input="checkMatch"
            />
            <p v-if="matchError" class="text-xs text-error mt-1">{{ matchError }}</p>
          </div>
        </div>

        <div class="modal-action flex-wrap gap-2 pt-4">
          <button type="button" class="btn btn-ghost btn-sm" @click="close">{{ t.common.cancel }}</button>
          <button
            type="button"
            class="btn btn-error btn-sm"
            :disabled="!inputMatches || deleting"
            @click="handleDelete"
          >
            <span v-if="deleting" class="loading loading-spinner loading-sm"></span>
            {{ t.common.delete }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { apiClient } from '@/api/client';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';

const props = defineProps<{
  cluster: string;
  topic: string;
}>();

const emit = defineEmits<{
  deleted: [cluster: string, topic: string];
}>();

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showSuccess, showError } = useToast();

const dialogRef = ref<HTMLDialogElement>();
const deleting = ref(false);
const confirmInput = ref('');
const inputMatches = ref(false);
const matchError = ref('');

function open() {
  confirmInput.value = '';
  inputMatches.value = false;
  matchError.value = '';
  dialogRef.value?.showModal();
}

function close() {
  dialogRef.value?.close();
}

function checkMatch() {
  const input = confirmInput.value.trim();
  if (input === props.topic) {
    inputMatches.value = true;
    matchError.value = '';
  } else {
    inputMatches.value = false;
    matchError.value = '';
  }
}

async function copyTopicName() {
  try {
    await navigator.clipboard.writeText(props.topic);
    showSuccess(t.value.topics.copied || 'Copied');
  } catch {
    showError(t.value.topics.copyFailed || 'Copy failed');
  }
}

async function handleDelete() {
  if (!inputMatches.value) {
    matchError.value = t.value.topics.confirmDeleteMatchError || 'Topic name does not match';
    return;
  }

  deleting.value = true;
  try {
    await apiClient.deleteTopic(props.cluster, props.topic);
    showSuccess(t.value.topics.deletedSuccess || 'Topic deleted successfully');
    close();
    emit('deleted', props.cluster, props.topic);
  } catch (e) {
    showError((e as { message?: string }).message || 'Delete failed');
  } finally {
    deleting.value = false;
  }
}

defineExpose({ open, close });
</script>
