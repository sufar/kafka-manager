<template>
  <Teleport to="body">
    <dialog ref="dialogRef" class="modal modal-bottom sm:modal-middle" @click.self="close">
      <div class="modal-box w-full max-w-2xl mx-2 md:mx-auto p-5">
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2">
            <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
              </svg>
            </div>
            <div>
              <h3 class="font-bold text-base">{{ t.topics.createTopic }}</h3>
              <span class="text-xs text-base-content/60 font-mono">{{ form.name || t.topics.topicNamePlaceholder }}</span>
            </div>
          </div>
          <button class="btn btn-sm btn-circle btn-ghost" @click="close">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <form @submit.prevent="submit" class="space-y-4">
          <!-- Topic Name -->
          <div>
            <label class="block text-sm font-medium mb-1.5">
              <span class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
                </svg>
                {{ t.topics.topicName }} <span class="text-error text-xs ml-1">{{ t.messages.required }}</span>
              </span>
            </label>
            <input
              v-model="form.name"
              type="text"
              :placeholder="t.topics.topicNamePlaceholder"
              class="input input-bordered input-sm w-full"
              required
              pattern="^[a-zA-Z0-9._-]+$"
              :title="t.topics.topicNameValidation"
            />
          </div>

          <!-- Partitions and Replication Factor Row -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label class="block text-sm font-medium mb-1.5">
                <span class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6Z" />
                  </svg>
                  {{ t.topics.numPartitions }}
                </span>
              </label>
              <input
                v-model.number="form.numPartitions"
                type="number"
                min="1"
                max="100"
                class="input input-bordered input-sm w-full"
                required
              />
              <p class="text-xs text-base-content/60 mt-1">{{ t.topics.numPartitionsHelp }}</p>
            </div>
            <div>
              <label class="block text-sm font-medium mb-1.5">
                <span class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M18 18.75a.75.75 0 0 0 .75-.75c0-.178-.012-.355-.036-.528A9.75 9.75 0 0 0 12 3.75c-1.324 0-2.595.274-3.75.772V18h9.75ZM12 2.25c-2.485 0-4.856.488-7.062 1.38a.75.75 0 0 0-.447.932l.958 3.758a.75.75 0 0 0 .973.536 8.25 8.25 0 0 1 10.572 0 .75.75 0 0 0 .973-.536l.958-3.758a.75.75 0 0 0-.447-.932A18.25 18.25 0 0 0 12 2.25Z" />
                  </svg>
                  {{ t.topics.replicationFactor }}
                </span>
              </label>
              <input
                v-model.number="form.replicationFactor"
                type="number"
                min="1"
                max="10"
                class="input input-bordered input-sm w-full"
                required
              />
              <p class="text-xs text-base-content/60 mt-1">{{ t.topics.replicationFactorHelp }}</p>
            </div>
          </div>

          <!-- Advanced Options -->
          <div class="border-t border-base-content/10 pt-4">
            <button type="button" class="btn btn-ghost btn-sm w-full justify-between group" @click="showAdvanced = !showAdvanced">
              <span class="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 transition-transform group-hover:rotate-90" :class="{ 'rotate-90': showAdvanced }">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                </svg>
                {{ t.topics.advancedOptions }}
              </span>
              <span class="text-xs text-base-content/50">{{ showAdvanced ? t.messages.hide : t.messages.show }}</span>
            </button>

            <div v-if="showAdvanced" class="mt-3 space-y-3 animate-fadeIn">
              <div>
                <label class="block text-sm font-medium mb-1.5">{{ t.topics.cleanupPolicy }}</label>
                <select v-model="form.config.cleanup_policy" class="select select-bordered select-sm w-full">
                  <option value="delete">delete</option>
                  <option value="compact">compact</option>
                  <option value="delete,compact">delete,compact</option>
                </select>
              </div>
              <div>
                <label class="block text-sm font-medium mb-1.5">{{ t.topics.retentionMs }}</label>
                <input
                  v-model="form.config.retention_ms"
                  type="text"
                  :placeholder="t.topics.retentionMsPlaceholder"
                  class="input input-bordered input-sm w-full"
                />
              </div>
              <div>
                <label class="block text-sm font-medium mb-1.5">{{ t.topics.retentionBytes }}</label>
                <input
                  v-model="form.config.retention_bytes"
                  type="text"
                  :placeholder="t.topics.retentionBytesPlaceholder"
                  class="input input-bordered input-sm w-full"
                />
              </div>
              <div>
                <label class="block text-sm font-medium mb-1.5">{{ t.topics.segmentBytes }}</label>
                <input
                  v-model="form.config.segment_bytes"
                  type="text"
                  :placeholder="t.topics.segmentBytesPlaceholder"
                  class="input input-bordered input-sm w-full"
                />
              </div>
            </div>
          </div>

          <div class="modal-action flex-wrap gap-2 pt-3">
            <button type="button" class="btn btn-ghost btn-sm" @click="close">{{ t.common.cancel }}</button>
            <button type="submit" class="btn btn-primary btn-sm flex items-center gap-2" :disabled="submitting">
              <span v-if="submitting" class="loading loading-spinner loading-sm"></span>
              {{ t.common.create }}
            </button>
          </div>
        </form>
      </div>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import { apiClient } from '@/api/client';

const props = defineProps<{
  clusterName: string;
}>();

const emit = defineEmits<{
  created: [];
}>();

const languageStore = useLanguageStore();
const t = languageStore.t;
const toast = useToast();

const dialogRef = ref<HTMLDialogElement>();
const submitting = ref(false);
const showAdvanced = ref(false);

const form = reactive({
  name: '',
  numPartitions: 3,
  replicationFactor: 1,
  config: {
    cleanup_policy: 'delete',
    retention_ms: '',
    retention_bytes: '',
    segment_bytes: '',
  } as Record<string, string>,
});

function open() {
  form.name = '';
  form.numPartitions = 3;
  form.replicationFactor = 1;
  form.config = {
    cleanup_policy: 'delete',
    retention_ms: '',
    retention_bytes: '',
    segment_bytes: '',
  };
  showAdvanced.value = false;
  dialogRef.value?.showModal();
}

function close() {
  dialogRef.value?.close();
}

async function submit() {
  const clusterId = props.clusterName;
  if (!clusterId) {
    toast.showError(t.topics.validationClusterIdRequired);
    return;
  }

  const trimmedName = form.name.trim();
  if (!trimmedName) {
    toast.showError(t.topics.validationTopicNameRequired);
    return;
  }
  if (trimmedName.length > 256) {
    toast.showError(t.topics.validationTopicNameTooLong);
    return;
  }
  if (trimmedName.includes(' ') || trimmedName.includes('"') || trimmedName.includes("'") || trimmedName.includes(',')) {
    toast.showError(t.topics.validationTopicNameInvalidChars);
    return;
  }
  const topicNameRegex = /^[a-zA-Z0-9._-]+$/;
  if (!topicNameRegex.test(trimmedName)) {
    toast.showError(t.topics.validationTopicNameFormat);
    return;
  }

  submitting.value = true;
  try {
    const config: Record<string, string> = {};
    if (showAdvanced.value) {
      if (form.config.cleanup_policy && form.config.cleanup_policy.trim()) {
        config['cleanup.policy'] = form.config.cleanup_policy.trim();
      }
      if (form.config.retention_ms && form.config.retention_ms.trim()) {
        const retentionMs = parseInt(form.config.retention_ms.trim(), 10);
        if (isNaN(retentionMs) || retentionMs < 0) {
          toast.showError(t.topics.validationRetentionMs);
          submitting.value = false;
          return;
        }
        config['retention.ms'] = retentionMs.toString();
      }
      if (form.config.retention_bytes && form.config.retention_bytes.trim()) {
        const retentionBytes = parseInt(form.config.retention_bytes.trim(), 10);
        if (isNaN(retentionBytes)) {
          toast.showError(t.topics.validationRetentionBytes);
          submitting.value = false;
          return;
        }
        config['retention.bytes'] = retentionBytes.toString();
      }
      if (form.config.segment_bytes && form.config.segment_bytes.trim()) {
        const segmentBytes = parseInt(form.config.segment_bytes.trim(), 10);
        if (isNaN(segmentBytes) || segmentBytes < 0) {
          toast.showError(t.topics.validationSegmentBytes);
          submitting.value = false;
          return;
        }
        config['segment.bytes'] = segmentBytes.toString();
      }
    }

    await apiClient.createTopic(clusterId, {
      name: trimmedName,
      num_partitions: form.numPartitions,
      replication_factor: form.replicationFactor,
      config: Object.keys(config).length > 0 ? config : undefined,
    });

    toast.showSuccess(t.topics.createdSuccess.replace('${name}', trimmedName));
    close();
    emit('created');
  } catch (e) {
    toast.showError(`${t.topics.createFailed}: ${(e as { message: string }).message}`);
  } finally {
    submitting.value = false;
  }
}

defineExpose({ open, close });
</script>
