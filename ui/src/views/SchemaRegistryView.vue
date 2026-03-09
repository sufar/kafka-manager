<template>
  <div>
    <div class="flex justify-between items-center mb-4">
      <div>
        <h2 class="text-xl font-bold">{{ t.schemaRegistry.title }}</h2>
        <p class="text-base-content/60 mt-1 text-sm">{{ t.schemaRegistry.description }}</p>
      </div>
      <button class="btn btn-primary btn-xs" @click="openRegisterModal" :disabled="!selectedClusterId">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        {{ t.schemaRegistry.registerSchema }}
      </button>
    </div>

    <!-- Cluster selection required -->
    <div v-if="!selectedClusterId" class="alert alert-info">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-6 h-6">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{{ t.schemaRegistry.selectClusterDesc }}</span>
    </div>

    <!-- Loading state -->
    <div v-else-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-md text-primary"></span>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="alert alert-error py-2">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-5 w-5" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span class="text-sm">{{ error }}</span>
    </div>

    <!-- Empty state -->
    <div v-else-if="subjects.length === 0" class="text-center py-8">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16 mx-auto text-base-content/30 mb-3">
        <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
      </svg>
      <h3 class="text-lg font-semibold mb-2">{{ t.schemaRegistry.noSchemasFound }}</h3>
      <p class="text-base-content/60 mb-4 text-sm">{{ t.schemaRegistry.noSchemasFoundDesc }}</p>
      <button class="btn btn-primary btn-xs" @click="openRegisterModal">{{ t.schemaRegistry.registerSchema }}</button>
    </div>

    <!-- Schema Subjects Grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div
        v-for="subject in subjects"
        :key="subject"
        class="card bg-base-100 shadow hover:shadow-lg transition-shadow cursor-pointer"
        @click="viewSubject(subject)"
      >
        <div class="card-body p-3">
          <div class="flex items-center justify-between">
            <h3 class="card-title text-sm truncate">{{ subject }}</h3>
            <div class="badge badge-primary badge-xs">{{ getVersionsCount(subject) }} {{ t.schemaRegistry.versions }}</div>
          </div>
          <div class="text-xs text-base-content/60 truncate">
            {{ t.schemaRegistry.type }}: {{ getSchemaType(subject) }}
          </div>
          <div class="card-actions justify-end mt-3">
            <button class="btn btn-xs btn-ghost" @click.stop="viewSubject(subject)">{{ t.schemaRegistry.view }}</button>
            <button class="btn btn-xs btn-ghost text-error" @click.stop="confirmDeleteSubject(subject)">{{ t.schemaRegistry.deleteSchema }}</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Register Schema Modal -->
    <dialog ref="registerModalRef" class="modal">
      <div class="modal-box max-w-3xl p-4">
        <h3 class="font-bold text-base mb-3">{{ t.schemaRegistry.registerSchema }}</h3>
        <form @submit.prevent="handleRegisterSchema">
          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.schemaRegistry.subjectName }}</span>
            </label>
            <input
              v-model="newSchema.subject"
              type="text"
              placeholder="my-topic-value"
              class="input input-bordered input-sm font-mono"
              required
            />
          </div>

          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.schemaRegistry.schemaType }}</span>
            </label>
            <select v-model="newSchema.schema_type" class="select select-bordered select-sm">
              <option value="AVRO">Avro</option>
              <option value="JSON">JSON Schema</option>
              <option value="PROTOBUF">Protobuf</option>
            </select>
          </div>

          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.schemaRegistry.schemaDefinition }}</span>
            </label>
            <textarea
              v-model="newSchema.schema"
              class="textarea textarea-bordered font-mono h-48 text-sm"
              placeholder='{"type": "record", "name": "MyRecord", "fields": [{"name": "field1", "type": "string"}]}'
              required
            ></textarea>
          </div>

          <div class="modal-action py-3">
            <button type="button" class="btn btn-xs" @click="closeRegisterModal">{{ t.schemaRegistry.close }}</button>
            <button type="submit" class="btn btn-primary btn-xs" :disabled="registering">
              <span v-if="registering" class="loading loading-spinner loading-xs"></span>
              {{ t.schemaRegistry.registerSchema }}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" class="modal-backdrop" @click="closeRegisterModal">
        <button>close</button>
      </form>
    </dialog>

    <!-- Subject Detail Modal -->
    <dialog ref="detailModalRef" class="modal">
      <div class="modal-box max-w-4xl p-4">
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-bold text-base">Subject: {{ selectedSubject }}</h3>
          <div class="flex gap-2">
            <button class="btn btn-xs btn-ghost" @click="copySchema">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 01-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 011.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 00-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 01-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 00-3.375-3.375h-1.5a1.125 1.125 0 01-1.125-1.125v-1.5a3.375 3.375 0 00-3.375-3.375H9.75" />
              </svg>
              {{ t.schemaRegistry.copy }}
            </button>
            <button class="btn btn-xs btn-error" @click="confirmDeleteSubject(selectedSubject)">{{ t.schemaRegistry.deleteSchema }}</button>
          </div>
        </div>

        <!-- Version Tabs -->
        <div role="tablist" class="tabs tabs-boxed mb-3">
          <a
            v-for="version in selectedSubjectVersions"
            :key="version"
            role="tab"
            class="tab tab-xs"
            :class="{ 'tab-active': selectedVersion === version }"
            @click="selectVersion(version)"
          >
            v{{ version }}
          </a>
        </div>

        <!-- Schema Content -->
        <div v-if="selectedSchema" class="mockup-code bg-base-200 mb-3 text-xs">
          <pre><code>{{ selectedSchema.schema }}</code></pre>
        </div>

        <div class="grid grid-cols-2 gap-3 text-xs">
          <div>
            <span class="text-base-content/60 text-xs">{{ t.schemaRegistry.schemaId }}</span>
            <p class="font-mono text-xs">{{ selectedSchema?.id }}</p>
          </div>
          <div>
            <span class="text-base-content/60 text-xs">{{ t.schemaRegistry.type }}</span>
            <p class="font-semibold text-xs">{{ selectedSchema?.schema_type }}</p>
          </div>
        </div>

        <div class="modal-action py-3">
          <button class="btn btn-xs" @click="closeDetailModal">{{ t.schemaRegistry.close }}</button>
          <button class="btn btn-error btn-xs" @click="confirmDeleteVersion" :disabled="!selectedVersion">
            {{ t.schemaRegistry.deleteVersion }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop" @click="closeDetailModal">
        <button>close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useClusterStore } from '@/stores/cluster';
import { useLanguageStore } from '@/stores/language';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';
import type { SchemaInfo } from '@/types/api';

const route = useRoute();
const clusterStore = useClusterStore();
const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

// 从 URL 参数获取集群 ID
const clusterParam = computed(() => route.query.cluster as string || '');
const selectedClusterId = computed(() => clusterParam.value || clusterStore.selectedClusterId);
const schemaRegistryUrl = ref('http://localhost:8081');

const loading = ref(false);
const error = ref<string | null>(null);
const subjects = ref<string[]>([]);
const subjectVersions = ref<Record<string, number[]>>({});
const subjectSchemas = ref<Record<string, SchemaInfo>>({});

const registerModalRef = ref<HTMLDialogElement>();
const detailModalRef = ref<HTMLDialogElement>();

const registering = ref(false);
const newSchema = ref({
  subject: '',
  schema: '',
  schema_type: 'AVRO',
});

const selectedSubject = ref<string>('');
const selectedSubjectVersions = ref<number[]>([]);
const selectedVersion = ref<number | null>(null);
const selectedSchema = ref<SchemaInfo | null>(null);

function getVersionsCount(subject: string): number {
  return subjectVersions.value[subject]?.length || 0;
}

function getSchemaType(subject: string): string {
  return subjectSchemas.value[subject]?.schema_type || 'AVRO';
}

async function fetchSubjects() {
  if (!selectedClusterId.value) return;

  loading.value = true;
  error.value = null;
  try {
    subjects.value = await apiClient.getSchemaSubjects(selectedClusterId.value, schemaRegistryUrl.value);

    // Fetch versions and schemas for each subject
    for (const subject of subjects.value) {
      try {
        const versions = await apiClient.getSchemaSubjectVersions(selectedClusterId.value, subject, schemaRegistryUrl.value);
        subjectVersions.value[subject] = versions;

        // Get latest version schema
        if (versions.length > 0) {
          const latestVersion = Math.max(...versions);
          const schema = await apiClient.getSchema(selectedClusterId.value, subject, latestVersion.toString(), schemaRegistryUrl.value);
          subjectSchemas.value[subject] = schema;
        }
      } catch (e) {
        console.error(`Failed to fetch versions for ${subject}:`, e);
      }
    }
  } catch (e) {
    error.value = (e as { message: string }).message;
  } finally {
    loading.value = false;
  }
}

async function viewSubject(subject: string) {
  selectedSubject.value = subject;
  selectedSubjectVersions.value = subjectVersions.value[subject] || [];
  selectedVersion.value = null;
  selectedSchema.value = null;

  if (selectedSubjectVersions.value.length > 0) {
    selectVersion(Math.max(...selectedSubjectVersions.value));
  }

  detailModalRef.value?.showModal();
}

function selectVersion(version: number) {
  selectedVersion.value = version;
  selectedSchema.value = subjectSchemas.value[selectedSubject.value] || null;
}

function openRegisterModal() {
  newSchema.value = { subject: '', schema: '', schema_type: 'AVRO' };
  registerModalRef.value?.showModal();
}

function closeRegisterModal() {
  registerModalRef.value?.close();
}

async function handleRegisterSchema() {
  if (!selectedClusterId.value) return;

  registering.value = true;
  try {
    await apiClient.registerSchema(selectedClusterId.value, newSchema.value);
    showSuccess('Schema registered successfully');
    closeRegisterModal();
    fetchSubjects();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    registering.value = false;
  }
}

function confirmDeleteSubject(subject: string) {
  if (!selectedClusterId.value) return;
  if (confirm(t.value.schemaRegistry.confirmDeleteSubject.replace('{subject}', subject))) {
    apiClient.deleteSchema(selectedClusterId.value, subject, schemaRegistryUrl.value)
      .then(() => fetchSubjects())
      .catch(e => showError((e as { message: string }).message));
  }
}

function confirmDeleteVersion() {
  if (!selectedClusterId.value || !selectedSubject.value || !selectedVersion.value) return;

  if (confirm(t.value.schemaRegistry.confirmDeleteVersion.replace('{subject}', selectedSubject.value).replace('{version}', selectedVersion.value.toString()))) {
    apiClient.deleteSchemaVersion(selectedClusterId.value, selectedSubject.value, selectedVersion.value.toString(), schemaRegistryUrl.value)
      .then(() => fetchSubjects())
      .then(() => {
        if (selectedSubjectVersions.value.length > 0) {
          selectVersion(Math.max(...selectedSubjectVersions.value));
        } else {
          closeDetailModal();
        }
      })
      .catch(e => showError((e as { message: string }).message));
  }
}

function closeDetailModal() {
  detailModalRef.value?.close();
  selectedSubject.value = '';
  selectedSubjectVersions.value = [];
  selectedVersion.value = null;
  selectedSchema.value = null;
}

function copySchema() {
  if (selectedSchema.value) {
    navigator.clipboard.writeText(selectedSchema.value.schema);
    showSuccess(t.value.schemaRegistry.schemaCopied);
  }
}

watch(selectedClusterId, () => {
  fetchSubjects();
});

onMounted(() => {
  fetchSubjects();
});
</script>
