<template>
  <div class="p-3 overflow-auto min-h-full">
    <!-- Page Header -->
    <div class="mb-4">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-2">
        <div>
          <h1 class="text-xl font-bold flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 text-primary">
              <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
            </svg>
            {{ t.schemaRegistry.title }}
          </h1>
          <p class="text-base-content/60 mt-1 text-sm">{{ t.schemaRegistry.description }}</p>
        </div>
        <div class="flex items-center gap-2">
          <button class="btn btn-sm btn-outline" @click="openClusterSelector">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14M5 12a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2M5 12a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2m-2-4h.01M17 16h.01" />
            </svg>
            {{ selectedCluster || t.schemaRegistry.selectCluster }}
          </button>
        </div>
      </div>
    </div>

    <!-- No cluster selected -->
    <div v-if="!selectedCluster" class="flex flex-col items-center justify-center py-8 text-center">
      <div class="text-base-content/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16">
          <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
        </svg>
      </div>
      <h3 class="text-lg font-semibold mb-2">{{ t.common.noData }}</h3>
      <p class="text-base-content/60 text-sm">{{ t.schemaRegistry.configNotSet }}</p>
    </div>

    <!-- Config not set -->
    <div v-else-if="!config && !loading" class="card glass gradient-border shadow-xl p-6">
      <div class="flex flex-col items-center justify-center py-8 text-center">
        <div class="text-base-content/30 mb-4">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z" />
          </svg>
        </div>
        <h3 class="text-lg font-semibold mb-2">{{ t.schemaRegistry.configNotSet }}</h3>
        <p class="text-base-content/60 mb-4 text-sm">{{ t.schemaRegistry.description }}</p>
        <button class="btn btn-primary btn-sm" @click="openConfigDialog">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          {{ t.schemaRegistry.configTitle }}
        </button>
      </div>
    </div>

    <!-- Loading state -->
    <div v-else-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-md text-primary"></span>
      <p class="ml-4 text-base-content/60 text-sm">{{ t.common.loading }}...</p>
    </div>

    <!-- Main content - Config set -->
    <div v-else class="space-y-4">
      <!-- Config Card -->
      <div class="card glass gradient-border shadow-xl">
        <div class="card-body p-4">
          <div class="flex items-center justify-between mb-2">
            <h2 class="font-bold text-base flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M10.34 15.84c-.688-.06-1.386-.09-2.09-.09H7.5a4.5 4.5 0 1 1 0-9h.75c.704 0 1.402-.03 2.09-.09m0 9.18c.253.962.584 1.892.985 2.783.247.55.06 1.21-.463 1.511l-.657.38c-.551.318-1.26.117-1.527-.461a20.845 20.845 0 0 1-1.44-4.282m3.102.069a18.03 18.03 0 0 1-.59-4.592m0 9.18a23.848 23.848 0 0 1 8.835 2.535M10.34 6.66a23.847 23.847 0 0 0 8.835-2.535m0 0A23.74 23.74 0 0 0 18.795 3m.38 1.125a23.91 23.91 0 0 1 1.014 5.395m-1.014 8.855c-.118.38-.245.754-.38 1.125m.38-1.125a23.91 23.91 0 0 0 1.014-5.395m0-3.46c.495.43.816 1.035.816 1.73 0 .695-.32 1.3-.816 1.73m0-3.46a24.347 24.347 0 0 1 0 3.46" />
              </svg>
              {{ t.schemaRegistry.configTitle }}
            </h2>
            <div class="flex gap-1">
              <button class="btn btn-ghost btn-xs" @click="openConfigDialog" :title="t.common.edit">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" />
                </svg>
              </button>
              <button class="btn btn-ghost btn-xs text-error" @click="confirmDeleteConfig" :title="t.common.delete">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                </svg>
              </button>
            </div>
          </div>
          <div class="text-sm space-y-1">
            <div class="flex items-center gap-2">
              <span class="text-base-content/60 w-24">{{ t.schemaRegistry.registryUrl }}:</span>
              <span class="font-mono">{{ config?.registry_url }}</span>
            </div>
            <div v-if="config?.username" class="flex items-center gap-2">
              <span class="text-base-content/60 w-24">{{ t.schemaRegistry.username }}:</span>
              <span>{{ config.username }}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-base-content/60 w-24">{{ t.common.status }}:</span>
              <div class="flex items-center gap-1">
                <div class="w-2 h-2 rounded-full" :class="connected ? 'bg-success' : 'bg-error'"></div>
                <span class="text-xs">{{ connected ? t.common.connected : t.common.disconnected }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Subjects List -->
      <div class="card glass gradient-border shadow-xl">
        <div class="card-body p-4">
          <div class="flex items-center justify-between mb-3">
            <h2 class="font-bold text-base flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-secondary">
                <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
              </svg>
              {{ t.schemaRegistry.subjects }}
            </h2>
            <button class="btn btn-primary btn-sm" @click="openRegisterSchemaDialog">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
              </svg>
              {{ t.schemaRegistry.registerSchema }}
            </button>
          </div>

          <!-- Subjects Table -->
          <div v-if="subjectsLoading" class="flex justify-center py-8">
            <span class="loading loading-spinner loading-sm"></span>
          </div>
          <div v-else-if="subjects.length === 0" class="text-center py-8 text-base-content/60">
            {{ t.schemaRegistry.noSubjects }}
          </div>
          <div v-else class="overflow-x-auto">
            <table class="table w-full">
              <thead>
                <tr>
                  <th>{{ t.common.name }}</th>
                  <th>{{ t.schemaRegistry.version }}</th>
                  <th>{{ t.schemaRegistry.schemaType }}</th>
                  <th>{{ t.schemaRegistry.compatibilityLevel }}</th>
                  <th>{{ t.common.actions }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="schema in subjects" :key="schema.subject" class="hover">
                  <td class="font-mono text-sm">{{ schema.subject }}</td>
                  <td>{{ schema.latest_version }}</td>
                  <td><span class="badge badge-ghost badge-sm">{{ schema.schema_type }}</span></td>
                  <td>{{ schema.compatibility_level || '-' }}</td>
                  <td>
                    <div class="flex gap-1">
                      <button class="btn btn-ghost btn-xs" @click="viewSchema(schema.subject)" :title="t.schemaRegistry.viewSchema">
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                          <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" />
                          <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
                        </svg>
                      </button>
                      <button class="btn btn-ghost btn-xs text-error" @click="confirmDeleteSchema(schema.subject)" :title="t.schemaRegistry.deleteSchema">
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                          <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                        </svg>
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <!-- Cluster Selector Dialog -->
    <Teleport to="body">
      <dialog ref="clusterDialogRef" class="modal modal-bottom sm:modal-middle" @click.self="clusterDialogRef?.close()">
        <div class="modal-box w-11/12 max-w-lg mx-2 md:mx-auto p-5">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-base">{{ t.schemaRegistry.selectCluster }}</h3>
            <button class="btn btn-sm btn-circle btn-ghost" @click="clusterDialogRef?.close()">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
          <div class="max-h-80 overflow-y-auto">
            <div v-for="cluster in clusterStore.clusters" :key="cluster.name" class="form-control mb-2">
              <label class="label cursor-pointer hover:bg-base-200 rounded-lg p-3 transition-all" @click="selectCluster(cluster.name)">
                <span class="label-text font-mono">{{ cluster.name }}</span>
                <input
                  type="radio"
                  name="cluster"
                  class="radio radio-sm radio-primary"
                  :checked="selectedCluster === cluster.name"
                />
              </label>
            </div>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Config Dialog -->
    <Teleport to="body">
      <dialog ref="configDialogRef" class="modal modal-bottom sm:modal-middle" @click.self="closeConfigDialog">
        <div class="modal-box w-full max-w-md mx-2 md:mx-auto p-5">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-base">{{ t.schemaRegistry.configTitle }}</h3>
            <button class="btn btn-sm btn-circle btn-ghost" @click="closeConfigDialog">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <form @submit.prevent="saveConfig" class="space-y-4">
            <div>
              <label class="block text-sm font-medium mb-1.5">{{ t.schemaRegistry.registryUrl }}</label>
              <input
                v-model="configForm.registryUrl"
                type="url"
                :placeholder="t.schemaRegistry.registryUrlPlaceholder"
                class="input input-bordered w-full"
                required
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-1.5">{{ t.schemaRegistry.authentication }}</label>
              <div class="space-y-2">
                <input
                  v-model="configForm.username"
                  type="text"
                  :placeholder="t.schemaRegistry.username"
                  class="input input-bordered w-full"
                />
                <input
                  v-model="configForm.password"
                  type="password"
                  :placeholder="t.schemaRegistry.password"
                  class="input input-bordered w-full"
                />
              </div>
            </div>
            <div class="flex justify-end gap-2">
              <button type="button" class="btn btn-ghost btn-sm" @click="() => testConnection()">
                {{ t.schemaRegistry.testConnection }}
              </button>
            </div>
            <div class="modal-action flex-wrap gap-2">
              <button type="button" class="btn btn-ghost btn-sm" @click="closeConfigDialog">{{ t.common.cancel }}</button>
              <button type="submit" class="btn btn-primary btn-sm" :disabled="savingConfig">
                <span v-if="savingConfig" class="loading loading-spinner loading-sm"></span>
                {{ t.schemaRegistry.save }}
              </button>
            </div>
          </form>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Schema Detail Dialog -->
    <Teleport to="body">
      <dialog ref="schemaDialogRef" class="modal modal-bottom sm:modal-middle" @click.self="closeSchemaDialog">
        <div class="modal-box w-full max-w-2xl mx-2 md:mx-auto p-5">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-base flex items-center gap-2">
              <span class="font-mono text-sm">{{ selectedSchema?.subject }}</span>
              <span class="badge badge-ghost badge-sm">v{{ selectedSchema?.version }}</span>
            </h3>
            <button class="btn btn-sm btn-circle btn-ghost" @click="closeSchemaDialog">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div v-if="selectedSchema" class="space-y-3">
            <div class="flex gap-2">
              <span class="badge badge-primary badge-sm">{{ selectedSchema.schema_type }}</span>
              <span v-if="selectedSchema.compatibility_level" class="badge badge-ghost badge-sm">{{ selectedSchema.compatibility_level }}</span>
            </div>
            <div>
              <label class="block text-xs font-medium text-base-content/60 mb-1">{{ t.schemaRegistry.schemaContent }}</label>
              <pre class="bg-base-200 rounded-lg p-3 overflow-auto max-h-96 text-xs font-mono"><code>{{ selectedSchema.schema_json }}</code></pre>
            </div>
          </div>
        </div>
        <form method="dialog" class="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </Teleport>

    <!-- Register Schema Dialog -->
    <Teleport to="body">
      <dialog ref="registerSchemaDialogRef" class="modal modal-bottom sm:modal-middle" @click.self="closeRegisterSchemaDialog">
        <div class="modal-box w-full max-w-2xl mx-2 md:mx-auto p-5">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-base">{{ t.schemaRegistry.registerSchema }}</h3>
            <button class="btn btn-sm btn-circle btn-ghost" @click="closeRegisterSchemaDialog">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <form @submit.prevent="registerSchema" class="space-y-4">
            <div>
              <label class="block text-sm font-medium mb-1.5">{{ t.common.name }}</label>
              <input
                v-model="registerForm.subject"
                type="text"
                placeholder="Subject name"
                class="input input-bordered w-full"
                required
              />
            </div>
            <div>
              <label class="block text-sm font-medium mb-1.5">{{ t.schemaRegistry.schemaType }}</label>
              <select v-model="registerForm.schemaType" class="select select-bordered w-full">
                <option value="AVRO">AVRO</option>
                <option value="PROTOBUF">PROTOBUF</option>
                <option value="JSON">JSON</option>
              </select>
            </div>
            <div>
              <label class="block text-sm font-medium mb-1.5">{{ t.schemaRegistry.schemaContent }}</label>
              <textarea
                v-model="registerForm.schemaJson"
                :placeholder="t.schemaRegistry.schemaContentPlaceholder"
                class="textarea textarea-bordered w-full h-64 font-mono text-xs"
                required
              ></textarea>
            </div>
            <div class="modal-action flex-wrap gap-2">
              <button type="button" class="btn btn-ghost btn-sm" @click="testRegisterCompatibility">{{ t.schemaRegistry.testCompatibility }}</button>
              <button type="button" class="btn btn-ghost btn-sm" @click="closeRegisterSchemaDialog">{{ t.common.cancel }}</button>
              <button type="submit" class="btn btn-primary btn-sm" :disabled="registeringSchema">
                <span v-if="registeringSchema" class="loading loading-spinner loading-sm"></span>
                {{ t.schemaRegistry.registerSchema }}
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
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import { apiClient } from '@/api/client';
import type { SchemaRegistryConfig, SchemaSummary, SchemaInfo } from '@/types/api';

const route = useRoute();
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const toast = useToast();
const t = computed(() => languageStore.t);
const router = useRouter();

// 从路由参数获取 cluster，如果没有则使用 clusterStore 中的选中集群
const selectedCluster = computed(() => {
  const clusterFromRoute = route.query.cluster as string;
  if (clusterFromRoute) {
    return clusterFromRoute;
  }
  return clusterStore.selectedClusterIds[0];
});

// State
const loading = ref(false);
const config = ref<SchemaRegistryConfig | null>(null);
const connected = ref(false);
const subjects = ref<SchemaSummary[]>([]);
const subjectsLoading = ref(false);

// Cluster selector
const showClusterSelector = ref(false);
const clusterDialogRef = ref<HTMLDialogElement>();

function openClusterSelector() {
  clusterDialogRef.value?.showModal();
}

function selectCluster(clusterName: string) {
  router.replace({
    path: '/schema-registry',
    query: { cluster: clusterName }
  });
  clusterDialogRef.value?.close();
}

// Config Dialog
const configDialogRef = ref<HTMLDialogElement>();
const savingConfig = ref(false);
const configForm = reactive({
  registryUrl: '',
  username: '',
  password: '',
});

function openConfigDialog() {
  if (config.value) {
    configForm.registryUrl = config.value.registry_url;
    configForm.username = config.value.username || '';
    configForm.password = '';
  }
  configDialogRef.value?.showModal();
}

function closeConfigDialog() {
  configDialogRef.value?.close();
}

// Schema Dialog
const schemaDialogRef = ref<HTMLDialogElement>();
const selectedSchema = ref<SchemaInfo | null>(null);

// Register Schema Dialog
const registerSchemaDialogRef = ref<HTMLDialogElement>();
const registeringSchema = ref(false);
const registerForm = reactive({
  subject: '',
  schemaType: 'AVRO',
  schemaJson: '',
});

async function loadConfig() {
  if (!selectedCluster.value) return;

  loading.value = true;
  try {
    config.value = await apiClient.getSchemaRegistryConfig(selectedCluster.value);
    if (config.value) {
      await testConnection(true);
    }
  } catch (e) {
    console.error('[SchemaRegistry] Failed to load config:', e);
  } finally {
    loading.value = false;
  }
}

async function loadSubjects() {
  if (!selectedCluster.value) return;

  subjectsLoading.value = true;
  try {
    subjects.value = await apiClient.getSchemasList(selectedCluster.value);
  } catch (e) {
    console.error('[SchemaRegistry] Failed to load subjects:', e);
  } finally {
    subjectsLoading.value = false;
  }
}

async function testConnection(silent: boolean = false) {
  if (!configForm.registryUrl && config.value) {
    configForm.registryUrl = config.value.registry_url;
    configForm.username = config.value.username || '';
  }

  try {
    const result = await apiClient.testSchemaRegistryConnection(
      configForm.registryUrl || config.value?.registry_url || '',
      configForm.username || undefined,
      configForm.password || undefined
    );
    connected.value = result.success;
    if (!silent) {
      if (result.success) {
        toast.showSuccess(t.value.schemaRegistry.connectionSuccess);
      } else {
        toast.showError(t.value.schemaRegistry.connectionFailed);
      }
    }
  } catch (e) {
    connected.value = false;
    if (!silent) {
      toast.showError(t.value.schemaRegistry.connectionFailed);
    }
  }
}

async function saveConfig() {
  if (!selectedCluster.value) return;

  savingConfig.value = true;
  try {
    await apiClient.saveSchemaRegistryConfig(
      selectedCluster.value,
      configForm.registryUrl,
      configForm.username || undefined,
      configForm.password || undefined
    );
    await loadConfig();
    await loadSubjects();
    closeConfigDialog();
    toast.showSuccess('Configuration saved');
  } catch (e) {
    console.error('[SchemaRegistry] Failed to save config:', e);
    toast.showError('Failed to save configuration');
  } finally {
    savingConfig.value = false;
  }
}

async function confirmDeleteConfig() {
  const confirmed = await toast.confirm(t.value.schemaRegistry.deleteConfirm);
  if (!confirmed) return;

  try {
    await apiClient.deleteSchemaRegistryConfig(selectedCluster.value!);
    config.value = null;
    subjects.value = [];
    toast.showSuccess('Configuration deleted');
  } catch (e) {
    console.error('[SchemaRegistry] Failed to delete config:', e);
    toast.showError('Failed to delete configuration');
  }
}

async function viewSchema(subject: string) {
  try {
    const schema = await apiClient.getLatestSchema(selectedCluster.value!, subject);
    selectedSchema.value = schema;
    schemaDialogRef.value?.showModal();
  } catch (e) {
    console.error('[SchemaRegistry] Failed to load schema:', e);
  }
}

function closeSchemaDialog() {
  schemaDialogRef.value?.close();
}

async function confirmDeleteSchema(subject: string) {
  const confirmed = await toast.confirm(`${t.value.schemaRegistry.deleteConfirm}\n\n${subject}`);
  if (!confirmed) return;

  try {
    await apiClient.deleteSchema(selectedCluster.value!, subject);
    await loadSubjects();
    toast.showSuccess('Schema deleted successfully');
  } catch (e) {
    console.error('[SchemaRegistry] Failed to delete schema:', e);
    toast.showError('Failed to delete schema');
  }
}

function closeRegisterSchemaDialog() {
  registerSchemaDialogRef.value?.close();
  registerForm.subject = '';
  registerForm.schemaType = 'AVRO';
  registerForm.schemaJson = '';
}

function openRegisterSchemaDialog() {
  registerSchemaDialogRef.value?.showModal();
}

async function testRegisterCompatibility() {
  if (!selectedCluster.value || !registerForm.subject || !registerForm.schemaJson) return;

  try {
    const result = await apiClient.testSchemaCompatibility(
      selectedCluster.value,
      registerForm.subject,
      registerForm.schemaJson,
      -1
    );
    if (result.compatible) {
      toast.showSuccess(t.value.schemaRegistry.compatible);
    } else {
      toast.showWarning(t.value.schemaRegistry.incompatible);
    }
  } catch (e) {
    console.error('[SchemaRegistry] Failed to test compatibility:', e);
  }
}

async function registerSchema() {
  if (!selectedCluster.value) return;

  registeringSchema.value = true;
  try {
    await apiClient.registerSchema(
      selectedCluster.value,
      registerForm.subject,
      registerForm.schemaJson,
      registerForm.schemaType
    );
    await loadSubjects();
    closeRegisterSchemaDialog();
    toast.showSuccess('Schema registered successfully');
  } catch (e) {
    console.error('[SchemaRegistry] Failed to register schema:', e);
    toast.showError('Failed to register schema');
  } finally {
    registeringSchema.value = false;
  }
}

// 监听路由变化，重新加载数据
watch(() => route.query.cluster, () => {
  loadConfig();
  loadSubjects();
}, { immediate: true });

onMounted(() => {
  // 数据已在 watch 中加载
});
</script>

<style scoped>
pre {
  white-space: pre-wrap;
  word-wrap: break-word;
}
</style>
