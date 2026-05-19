<template>
  <Teleport to="body">
    <div v-if="visible && currentStep && targetRect" class="tour-overlay" @click="onOverlayClick">
      <!-- SVG Spotlight Mask -->
      <svg class="tour-mask" width="100%" height="100%">
        <defs>
          <mask id="tour-spotlight">
            <!-- 全白表示全部可见 -->
            <rect width="100%" height="100%" fill="white" />
            <!-- 黑色矩形挖空（聚光灯区域） -->
            <rect
              :x="spotlightRect.x - spotlightPadding"
              :y="spotlightRect.y - spotlightPadding"
              :width="spotlightRect.width + spotlightPadding * 2"
              :height="spotlightRect.height + spotlightPadding * 2"
              rx="8"
              ry="8"
              fill="black"
            />
          </mask>
        </defs>
        <!-- 带遮罩的遮罩层 -->
        <rect
          width="100%"
          height="100%"
          fill="rgba(0, 0, 0, 0.6)"
          mask="url(#tour-spotlight)"
        />
      </svg>

      <!-- 聚光灯边框 -->
      <div
        class="tour-spotlight-border"
        :style="{
          left: spotlightRect.x - spotlightPadding + 'px',
          top: spotlightRect.y - spotlightPadding + 'px',
          width: spotlightRect.width + spotlightPadding * 2 + 'px',
          height: spotlightRect.height + spotlightPadding * 2 + 'px',
        }"
      ></div>

      <!-- Tooltip Card -->
      <div
        ref="tooltipRef"
        class="tour-tooltip card"
        :class="[
          `tour-tooltip--${adjustedPosition}`,
        ]"
        :style="tooltipStyle"
      >
        <div class="card-body p-4 gap-2">
          <!-- 步骤指示 -->
          <div class="tour-tooltip-header">
            <div class="tour-tooltip-steps">
              <span class="tour-tooltip-current">{{ currentStepIndex + 1 }}</span>
              <span class="tour-tooltip-total"> / {{ totalSteps }}</span>
            </div>
            <button class="tour-tooltip-close" @click="$emit('close')" :title="t.common?.close || 'Close'">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- 标题 -->
          <h3 class="tour-tooltip-title">{{ resolveTranslation(currentStep.title) }}</h3>

          <!-- 说明 -->
          <p class="tour-tooltip-desc">{{ resolveTranslation(currentStep.description) }}</p>

          <!-- 导航按钮 -->
          <div class="tour-tooltip-footer">
            <button
              class="btn btn-sm btn-ghost gap-1"
              :disabled="currentStepIndex === 0"
              @click="$emit('prev')"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3.5 h-3.5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5L8.25 12l7.5-7.5" />
              </svg>
              {{ t.tour?.prevStep || '上一步' }}
            </button>
            <button
              class="btn btn-sm btn-primary gap-1"
              @click="handleNext"
            >
              {{ currentStepIndex === totalSteps - 1 ? (t.tour?.finishTour || '完成') : (t.tour?.nextStep || '下一步') }}
              <svg v-if="currentStepIndex !== totalSteps - 1" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-3.5 h-3.5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import type { TourStep } from '@/tour/definitions';

const props = defineProps<{
  visible: boolean;
  currentStep: TourStep | null | undefined;
  currentStepIndex: number;
  totalSteps: number;
  targetRect: DOMRect | null;
  t: Record<string, any>;
}>();

const emit = defineEmits<{
  close: [];
  next: [];
  prev: [];
}>();

const tooltipRef = ref<HTMLDivElement | null>(null);
const spotlightPadding = 6;

const spotlightRect = computed(() => {
  if (!props.targetRect) {
    return { x: 0, y: 0, width: 0, height: 0 };
  }
  return {
    x: props.targetRect.x,
    y: props.targetRect.y,
    width: props.targetRect.width,
    height: props.targetRect.height,
  };
});

const tooltipStyle = computed(() => {
  if (!props.targetRect) return {};
  return {
    '--target-x': `${props.targetRect.left}px`,
    '--target-y': `${props.targetRect.top}px`,
    '--target-w': `${props.targetRect.width}px`,
    '--target-h': `${props.targetRect.height}px`,
  };
});

// 智能位置调整：根据目标元素位置自动调整气泡位置，避免超出屏幕
const adjustedPosition = computed(() => {
  if (!props.targetRect) return 'bottom';
  const rect = props.targetRect;
  const position = props.currentStep?.position || 'bottom';
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  // 根据请求的位置检查是否有足够空间
  if (position === 'top' && rect.top > 200) return 'top';
  if (position === 'bottom' && vh - rect.bottom > 200) return 'bottom';
  if (position === 'left' && rect.left > 350) return 'left';
  if (position === 'right' && vw - rect.right > 350) return 'right';

  // 回退策略：选择有足够空间的方向
  if (vh - rect.bottom > 200) return 'bottom';
  if (rect.top > 200) return 'top';
  if (vw - rect.right > 350) return 'right';
  if (rect.left > 350) return 'left';
  return 'bottom';
});

function resolveTranslation(key: string): string {
  const keys = key.split('.');
  let result: any = props.t;
  for (const k of keys) {
    if (result && typeof result === 'object' && k in result) {
      result = result[k];
    } else {
      return key;
    }
  }
  return typeof result === 'string' ? result : key;
}

function handleNext() {
  emit('next');
}

function onOverlayClick(event: MouseEvent) {
  // 点击在遮罩上（不在 tooltip 上）时关闭
  const target = event.target as HTMLElement;
  if (target.classList.contains('tour-overlay') || target.classList.contains('tour-mask') || target.tagName === 'rect') {
    emit('close');
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (!props.visible) return;
  if (e.key === 'Escape') {
    emit('close');
  } else if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
    e.preventDefault();
    emit('next');
  } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
    e.preventDefault();
    emit('prev');
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown);
});
</script>

<style scoped>
.tour-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  pointer-events: auto;
}

.tour-overlay > .tour-mask {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

/* 聚光灯边框动画 */
.tour-spotlight-border {
  position: fixed;
  border: 2px solid rgb(var(--color-primary, 124, 58, 237));
  border-radius: 10px;
  pointer-events: none;
  z-index: 10001;
  box-shadow: 0 0 20px rgba(var(--color-primary, 124, 58, 237), 0.4);
  animation: tour-pulse 2s ease-in-out infinite;
  transition: left 0.3s ease, top 0.3s ease, width 0.3s ease, height 0.3s ease;
}

@keyframes tour-pulse {
  0%, 100% { box-shadow: 0 0 10px rgba(var(--color-primary, 124, 58, 237), 0.3); }
  50% { box-shadow: 0 0 25px rgba(var(--color-primary, 124, 58, 237), 0.6); }
}

/* Tooltip Card */
.tour-tooltip {
  position: fixed;
  z-index: 10002;
  background: var(--fallback-b1, oklch(var(--b1)));
  color: var(--fallback-bc, oklch(var(--bc)));
  border-radius: 1rem;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  max-width: 340px;
  min-width: 260px;
  animation: tour-fadeIn 0.25s ease-out;
  border: 1px solid var(--fallback-b2, oklch(var(--b2)));
}

.tour-tooltip .card-body {
  padding: 1rem;
}

.tour-tooltip--top {
  bottom: calc(100vh - var(--target-y) + 12px);
  left: calc(var(--target-x) + var(--target-w) / 2 - 140px);
}

.tour-tooltip--bottom {
  top: calc(var(--target-y) + var(--target-h) + 12px);
  left: calc(var(--target-x) + var(--target-w) / 2 - 140px);
}

.tour-tooltip--left {
  top: calc(var(--target-y) + var(--target-h) / 2 - 50px);
  right: calc(100vw - var(--target-x) + 12px);
  left: auto;
}

.tour-tooltip--right {
  top: calc(var(--target-y) + var(--target-h) / 2 - 50px);
  left: calc(var(--target-x) + var(--target-w) + 12px);
}

.tour-tooltip-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.tour-tooltip-steps {
  font-size: 11px;
  opacity: 0.5;
  font-family: ui-monospace, monospace;
}

.tour-tooltip-close {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: transparent;
  border: none;
  cursor: pointer;
  opacity: 0.5;
  transition: opacity 0.15s, background 0.15s;
  color: var(--fallback-bc, oklch(var(--bc)));
}

.tour-tooltip-close:hover {
  opacity: 1;
  background: var(--fallback-b2, oklch(var(--b2)));
}

.tour-tooltip-title {
  font-weight: 700;
  font-size: 0.95rem;
  margin: 0;
  line-height: 1.3;
}

.tour-tooltip-desc {
  font-size: 0.8rem;
  line-height: 1.5;
  opacity: 0.7;
  margin: 0;
}

.tour-tooltip-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-top: 1px solid var(--fallback-b2, oklch(var(--b2) / 0.3));
  padding-top: 8px;
  margin-top: 4px;
}

@keyframes tour-fadeIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
