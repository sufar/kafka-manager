<template>
  <div>
    <!-- Global Toast Notifications -->
    <div class="fixed top-4 right-4 toast toast-end z-[9999]">
      <div v-for="toast in toasts" :key="toast.id" class="alert" :class="getToastClass(toast.type)">
        <svg v-if="toast.type === 'error'" xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <svg v-else-if="toast.type === 'success'" xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <svg v-else-if="toast.type === 'warning'" xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <svg v-else xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span>{{ toast.message }}</span>
        <button class="btn btn-ghost btn-xs" @click="removeToast(toast.id)">✕</button>
      </div>
    </div>

    <!-- Confirmation Dialog -->
    <div v-show="confirmDialogVisible" class="fixed inset-0 z-[9999] flex items-center justify-center" style="background: rgba(0,0,0,0.5);">
      <!-- Dialog Box -->
      <div class="bg-base-100 text-base-content p-6 rounded-lg shadow-2xl" style="min-width: 300px; max-width: 90vw;">
        <h3 class="text-lg font-bold mb-4">{{ t.common.confirm }}</h3>
        <p class="mb-4">{{ confirmMessage }}</p>
        <div class="flex justify-end gap-2">
          <button class="btn btn-ghost btn-sm" @click="handleConfirmCancel">{{ t.common.cancel }}</button>
          <button class="btn btn-primary btn-sm" @click="handleConfirmConfirm">{{ t.common.confirm }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, provide, computed } from 'vue';
import { useLanguageStore } from '@/stores/language';

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);

// Toast notifications
interface Toast {
  id: number;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
}

const toasts = ref<Toast[]>([]);
let toastId = 0;

function showToast(type: 'success' | 'error' | 'warning' | 'info', message: string, duration: number = 5000) {
  const id = toastId++;
  toasts.value.push({ id, type, message });
  if (duration > 0) {
    setTimeout(() => removeToast(id), duration);
  }
}

function removeToast(id: number) {
  const index = toasts.value.findIndex(t => t.id === id);
  if (index > -1) {
    toasts.value.splice(index, 1);
  }
}

function getToastClass(type: string): string {
  const classes = {
    success: 'alert-success',
    error: 'alert-error',
    warning: 'alert-warning',
    info: 'alert-info'
  };
  return classes[type as keyof typeof classes] || 'alert-info';
}

// Confirmation dialog
const confirmMessage = ref('');
const confirmDialogVisible = ref(false);
let confirmResolve: ((value: boolean) => void) | null = null;

function showConfirm(message: string): Promise<boolean> {
  confirmMessage.value = message;
  confirmDialogVisible.value = true;
  return new Promise((resolve) => {
    confirmResolve = resolve;
  });
}

function handleConfirmConfirm() {
  confirmDialogVisible.value = false;
  if (confirmResolve) {
    confirmResolve(true);
    confirmResolve = null;
  }
}

function handleConfirmCancel() {
  confirmDialogVisible.value = false;
  if (confirmResolve) {
    confirmResolve(false);
    confirmResolve = null;
  }
}

// Provide to child components
provide('showToast', showToast);
provide('showConfirm', showConfirm);

// Expose methods to parent
defineExpose({
  showToast,
  showConfirm
});
</script>
