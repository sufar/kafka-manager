<template>
  <Teleport to="body">
    <dialog ref="modalRef" class="modal modal-bottom sm:modal-middle" @click.self="handleClickOutside">
      <div class="modal-box-wrapper" :class="wrapperClass">
        <!-- Header -->
        <div class="modal-header" v-if="title || $slots.header">
          <slot name="header">
            <div class="flex items-center justify-between w-full">
              <div class="flex items-center gap-2">
                <div v-if="iconSlot" class="modal-icon">
                  <slot name="icon"></slot>
                </div>
                <div>
                  <h3 class="modal-title">{{ title }}</h3>
                  <span v-if="subtitle" class="modal-subtitle">{{ subtitle }}</span>
                </div>
              </div>
              <button v-if="showClose" class="btn btn-sm btn-circle btn-ghost" @click="close">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </slot>
        </div>

        <!-- Content -->
        <div class="modal-content">
          <slot></slot>
        </div>

        <!-- Footer -->
        <div v-if="$slots.footer" class="modal-footer">
          <slot name="footer"></slot>
        </div>
      </div>
    </dialog>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';

const props = defineProps<{
  modelValue: boolean;
  title?: string;
  subtitle?: string;
  showClose?: boolean;
  maxWidth?: 'sm' | 'md' | 'lg' | 'xl' | '2xl';
  iconSlot?: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  'close': [];
}>();

const modalRef = ref<HTMLDialogElement | null>(null);

// 最大宽度映射
const maxWidthMap = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-xl',
  '2xl': 'max-w-2xl',
};

const wrapperClass = computed(() => {
  return maxWidthMap[props.maxWidth || '2xl'];
});

// 监听弹窗开关
watch(() => props.modelValue, async (newVal) => {
  if (newVal) {
    await new Promise(resolve => setTimeout(resolve, 10));
    modalRef.value?.showModal();
  } else {
    modalRef.value?.close();
  }
}, { immediate: true });

function handleClickOutside() {
  close();
}

function close() {
  emit('update:modelValue', false);
  emit('close');
}

// 暴露关闭方法给父组件
defineExpose({
  close
});
</script>

<style scoped>
/* 弹窗容器样式 */
.modal-box-wrapper {
  width: 100%;
  margin-left: 0.5rem;
  margin-right: 0.5rem;
  padding: 1.25rem;
  border-radius: 0.75rem;
}

/* 响应式：桌面端居中 */
@media (min-width: 768px) {
  .modal-box-wrapper {
    margin-left: auto;
    margin-right: auto;
  }
}

/* 最大宽度 */
.max-w-sm { max-width: 24rem; }
.max-w-md { max-width: 28rem; }
.max-w-lg { max-width: 32rem; }
.max-w-xl { max-width: 36rem; }
.max-w-2xl { max-width: 42rem; }

/* 头部样式 */
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1rem;
}

/* 图标容器样式 */
.modal-icon {
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 0.75rem;
  background: linear-gradient(to bottom right, rgb(var(--color-primary) / 0.2), rgb(var(--color-secondary) / 0.2));
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-icon :deep(svg) {
  width: 1rem;
  height: 1rem;
  color: rgb(var(--color-primary));
}

/* 标题样式 */
.modal-title {
  font-weight: 700;
  font-size: 1rem;
}

/* 副标题样式 */
.modal-subtitle {
  font-size: 0.75rem;
  opacity: 0.6;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* 内容区域样式 */
.modal-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

/* 底部操作区样式 */
.modal-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding-top: 0.75rem;
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

/* 淡入动画 */
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

:global(.modal-content > .animate-collapse) {
  animation: fadeIn 0.2s ease-out;
}
</style>
