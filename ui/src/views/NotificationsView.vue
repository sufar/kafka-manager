<template>
  <div>
    <div class="flex justify-between items-center mb-6">
      <div>
        <h2 class="text-3xl font-bold">{{ t.notifications.title }}</h2>
        <p class="text-base-content/60 mt-1">{{ t.notifications.description }}</p>
      </div>
      <button class="btn btn-primary" @click="openCreateModal">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        {{ t.notifications.addNotification }}
      </button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg text-primary"></span>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{{ error }}</span>
    </div>

    <!-- Empty state -->
    <div v-else-if="notifications.length === 0" class="text-center py-12">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-24 h-24 mx-auto text-base-content/30 mb-4">
        <path stroke-linecap="round" stroke-linejoin="round" d="M14.857 17.082a23.848 23.848 0 005.454-1.31 8.967 8.967 0 01-2.312-6.022V9a6 6 0 10-12 0v.75a8.967 8.967 0 01-2.312 6.022m19.615-4.409a23.848 23.848 0 01-5.454 1.31 8.967 8.967 0 002.312 6.022c-.621.25-1.277.45-1.958.59m-13.477 0a8.966 8.966 0 01-1.958-.59 8.967 8.967 0 002.312-6.022m19.615 4.409a23.848 23.848 0 01-5.454 1.31 8.967 8.967 0 00-2.312 6.022m-19.615-4.409a8.966 8.966 0 01-1.958-.59 8.967 8.967 0 002.312 6.022" />
      </svg>
      <h3 class="text-xl font-semibold mb-2">{{ t.notifications.noNotifications }}</h3>
      <p class="text-base-content/60 mb-4">{{ t.notifications.noNotificationsDesc }}</p>
      <button class="btn btn-primary" @click="openCreateModal">{{ t.notifications.addNotification }}</button>
    </div>

    <!-- Notifications Grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <div v-for="notif in notifications" :key="notif.id" class="card bg-base-100 shadow">
        <div class="card-body">
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2">
              <div :class="`badge ${notif.enabled ? 'badge-success' : 'badge-ghost'}`">
                {{ notif.enabled ? t.notifications.enabled : t.notifications.disabled }}
              </div>
              <h3 class="card-title">{{ notif.name }}</h3>
            </div>
            <div class="badge badge-ghost">{{ notif.type }}</div>
          </div>

          <div class="text-sm text-base-content/60 mb-4">
            <div v-for="(value, key) in notif.config" :key="key" class="flex justify-between py-1">
              <span class="capitalize">{{ key.replace(/_/g, ' ') }}:</span>
              <span class="font-mono">{{ maskSensitiveValue(key, value as string) }}</span>
            </div>
          </div>

          <div class="text-xs text-base-content/40 mb-4">
            Updated: {{ formatDate(notif.updated_at) }}
          </div>

          <div class="card-actions justify-end">
            <button
              class="btn btn-sm"
              :class="notif.enabled ? 'btn-warning' : 'btn-success'"
              @click="toggleEnabled(notif)"
            >
              {{ notif.enabled ? t.notifications.disable : t.notifications.enable }}
            </button>
            <button class="btn btn-sm btn-ghost" @click="editNotification(notif)">{{ t.notifications.edit }}</button>
            <button class="btn btn-sm btn-ghost text-error" @click="confirmDelete(notif.id)">{{ t.notifications.delete }}</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Create/Edit Modal -->
    <dialog ref="modalRef" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ editing ? t.notifications.editNotification : t.notifications.addNotification }}</h3>
        <form @submit.prevent="handleSubmit">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-semibold">{{ t.notifications.name }}</span>
            </label>
            <input v-model="formData.name" type="text" class="input input-bordered" required />
          </div>

          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-semibold">{{ t.notifications.type }}</span>
            </label>
            <select v-model="formData.type" class="select select-bordered" required :disabled="editing">
              <option value="slack">Slack</option>
              <option value="webhook">Webhook</option>
              <option value="email">Email</option>
            </select>
          </div>

          <!-- Slack config -->
          <div v-if="formData.type === 'slack'" class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.webhookUrl }}</span>
              </label>
              <input v-model="slackConfig.webhook_url" type="url" class="input input-bordered" placeholder="https://hooks.slack.com/..." />
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.channel }}</span>
              </label>
              <input v-model="slackConfig.channel" type="text" class="input input-bordered" placeholder="#alerts" />
            </div>
          </div>

          <!-- Webhook config -->
          <div v-if="formData.type === 'webhook'" class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.url }}</span>
              </label>
              <input v-model="webhookConfig.url" type="url" class="input input-bordered" />
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.method }}</span>
              </label>
              <select v-model="webhookConfig.method" class="select select-bordered">
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
              </select>
            </div>
          </div>

          <!-- Email config -->
          <div v-if="formData.type === 'email'" class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.smtpServer }}</span>
              </label>
              <input v-model="emailConfig.smtp_server" type="text" class="input input-bordered" placeholder="smtp.example.com" />
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.port }}</span>
              </label>
              <input v-model.number="emailConfig.port" type="number" class="input input-bordered" placeholder="587" />
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.fromEmail }}</span>
              </label>
              <input v-model="emailConfig.from_email" type="email" class="input input-bordered" />
            </div>
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t.notifications.toEmails }}</span>
              </label>
              <input v-model="emailConfig.to_emails" type="text" class="input input-bordered" placeholder="admin@example.com" />
              <label class="label">
                <span class="label-text-alt">{{ t.notifications.commaSeparatedEmails }}</span>
              </label>
            </div>
          </div>

          <div class="form-control mb-4 mt-4">
            <label class="cursor-pointer label justify-start gap-4">
              <input v-model="formData.enabled" type="checkbox" class="checkbox" />
              <span class="label-text">{{ t.notifications.enabled }}</span>
            </label>
          </div>

          <div class="modal-action">
            <button type="button" class="btn" @click="closeModal">{{ t.notifications.cancel }}</button>
            <button type="submit" class="btn btn-primary" :disabled="submitting">
              <span v-if="submitting" class="loading loading-spinner loading-sm"></span>
              {{ editing ? t.notifications.update : t.notifications.create }}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { apiClient } from '@/api/client';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import type { NotificationConfig } from '@/types/api';

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

interface SlackConfig {
  webhook_url: string;
  channel: string;
}

interface WebhookConfig {
  url: string;
  method: string;
}

interface EmailConfig {
  smtp_server: string;
  port: number;
  from_email: string;
  to_emails: string;
}

const loading = ref(false);
const error = ref<string | null>(null);
const notifications = ref<NotificationConfig[]>([]);

const modalRef = ref<HTMLDialogElement>();
const editing = ref(false);
const editingId = ref<number | null>(null);
const submitting = ref(false);

const formData = reactive({
  name: '',
  type: 'slack',
  config: {} as Record<string, string>,
  enabled: true,
});

const slackConfig = reactive<SlackConfig>({
  webhook_url: '',
  channel: '',
});

const webhookConfig = reactive<WebhookConfig>({
  url: '',
  method: 'POST',
});

const emailConfig = reactive<EmailConfig>({
  smtp_server: '',
  port: 587,
  from_email: '',
  to_emails: '',
});

async function fetchNotifications() {
  loading.value = true;
  error.value = null;
  try {
    notifications.value = await apiClient.getNotifications();
  } catch (e) {
    error.value = (e as { message: string }).message;
  } finally {
    loading.value = false;
  }
}

function openCreateModal() {
  editing.value = false;
  editingId.value = null;
  formData.name = '';
  formData.type = 'slack';
  formData.config = {};
  formData.enabled = true;

  slackConfig.webhook_url = '';
  slackConfig.channel = '';

  webhookConfig.url = '';
  webhookConfig.method = 'POST';

  emailConfig.smtp_server = '';
  emailConfig.port = 587;
  emailConfig.from_email = '';
  emailConfig.to_emails = '';

  modalRef.value?.showModal();
}

function editNotification(_notif: NotificationConfig) {
  showError('Editing is not supported. Please delete and recreate the notification channel.');
}

function closeModal() {
  modalRef.value?.close();
  editing.value = false;
  editingId.value = null;
}

function buildConfig(): Record<string, string> {
  if (formData.type === 'slack') {
    return {
      webhook_url: slackConfig.webhook_url,
      channel: slackConfig.channel,
    };
  } else if (formData.type === 'webhook') {
    return {
      url: webhookConfig.url,
      method: webhookConfig.method,
    };
  } else if (formData.type === 'email') {
    return {
      smtp_server: emailConfig.smtp_server,
      port: emailConfig.port.toString(),
      from_email: emailConfig.from_email,
      to_emails: emailConfig.to_emails,
    };
  }
  return {};
}

async function handleSubmit() {
  submitting.value = true;
  try {
    const config = buildConfig();
    await apiClient.createNotification({ ...formData, config });
    showSuccess(t.value.notifications.created);
    closeModal();
    fetchNotifications();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    submitting.value = false;
  }
}

function toggleEnabled(notif: NotificationConfig) {
  if (notif.enabled) {
    apiClient.disableNotification(notif.id)
      .then(() => fetchNotifications())
      .catch(e => showError((e as { message: string }).message));
  } else {
    apiClient.enableNotification(notif.id)
      .then(() => fetchNotifications())
      .catch(e => showError((e as { message: string }).message));
  }
}

function confirmDelete(id: number) {
  if (confirm(t.value.notifications.confirmDelete)) {
    apiClient.deleteNotification(id)
      .then(() => {
        showSuccess(t.value.notifications.deleted);
        fetchNotifications();
      })
      .catch(e => showError((e as { message: string }).message));
  }
}

function maskSensitiveValue(key: string, value: string): string {
  if (key.includes('token') || key.includes('password') || key.includes('secret') || key.includes('url')) {
    return value.length > 8 ? value.substring(0, 4) + '...' + value.substring(value.length - 4) : '****';
  }
  return value;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

onMounted(() => {
  fetchNotifications();
});
</script>
