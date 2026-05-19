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
        class="tour-tooltip card bg-base-100 text-base-content"
        :style="tooltipStyle"
      >
        <div class="card-body p-4 gap-2">
          <!-- 步骤指示 -->
          <div class="tour-tooltip-header">
            <div class="tour-tooltip-steps text-base-content/50 badge badge-sm badge-ghost">
              {{ currentStepIndex + 1 }} / {{ totalSteps }}
            </div>
            <button class="btn btn-ghost btn-xs btn-circle" @click="$emit('close')" :title="t.common?.close || 'Close'">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <!-- 标题 -->
          <h3 class="tour-tooltip-title">{{ resolveTranslation(currentStep.title) }}</h3>

          <!-- 说明 -->
          <p class="tour-tooltip-desc text-sm text-base-content/70">{{ resolveTranslation(currentStep.description) }}</p>

          <!-- 导航按钮 -->
          <div class="tour-tooltip-footer divider divider-neutral mt-1 pt-2">
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
import { computed, ref, watch, nextTick, onMounted, onUnmounted } from 'vue';
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

// Tooltip measured dimensions (use defaults for first render)
const tooltipHeight = ref(220); // estimated default height
const tooltipWidth = ref(300);  // estimated default width

// Measure tooltip size using ResizeObserver
let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const h = entry.borderBoxSize?.[0]?.blockSize;
      const w = entry.borderBoxSize?.[0]?.inlineSize;
      if (h) tooltipHeight.value = h;
      if (w) tooltipWidth.value = w;
    }
  });
});

onUnmounted(() => {
  resizeObserver?.disconnect();
});

// When step changes, ensure tooltip is measured after render
watch(
  () => [props.currentStepIndex, props.visible],
  async () => {
    await nextTick();
    if (tooltipRef.value) {
      // Start observing if not already
      if (resizeObserver && tooltipRef.value) {
        resizeObserver.observe(tooltipRef.value);
      }
      const rect = tooltipRef.value.getBoundingClientRect();
      if (rect.height > 0) {
        tooltipHeight.value = rect.height;
        tooltipWidth.value = rect.width;
      }
    }
  },
);

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

// Dynamic tooltip positioning using JS calculations
const tooltipStyle = computed(() => {
  if (!props.targetRect) return {};
  const rect = props.targetRect;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const tw = tooltipWidth.value;
  const th = tooltipHeight.value;
  const gap = 12; // space between target and tooltip
  const margin = 16; // minimum distance from screen edge

  // Determine best position based on requested position + available space
  const requested = props.currentStep?.position || 'bottom';

  // Calculate positions for all 4 directions
  const positions = {
    bottom: { top: rect.bottom + gap, left: rect.left + rect.width / 2 - tw / 2 },
    top: { top: rect.top - th - gap, left: rect.left + rect.width / 2 - tw / 2 },
    right: { top: rect.top + rect.height / 2 - th / 2, left: rect.right + gap },
    left: { top: rect.top + rect.height / 2 - th / 2, left: rect.left - tw - gap },
  };

  // Check if a position fits within viewport
  function fits(pos: { top: number; left: number }): boolean {
    return (
      pos.top >= margin &&
      pos.top + th <= vh - margin &&
      pos.left >= margin &&
      pos.left + tw <= vw - margin
    );
  }

  // Try requested position first, then fallback order
  const fallbackOrder: ('top' | 'bottom' | 'left' | 'right')[] = requested === 'left' || requested === 'right'
    ? [requested, 'bottom', 'top', requested === 'left' ? 'right' : 'left']
    : [requested, 'bottom', 'top', 'right', 'left'];

  let chosen = requested;
  for (const pos of fallbackOrder) {
    if (fits(positions[pos as keyof typeof positions])) {
      chosen = pos;
      break;
    }
  }

  // Clamp to viewport
  let finalTop = positions[chosen as keyof typeof positions].top;
  let finalLeft = positions[chosen as keyof typeof positions].left;
  finalTop = Math.max(margin, Math.min(finalTop, vh - th - margin));
  finalLeft = Math.max(margin, Math.min(finalLeft, vw - tw - margin));

  return {
    top: `${finalTop}px`,
    left: `${finalLeft}px`,
    visibility: 'visible' as const,
  };
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
  border-radius: 1rem;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  max-width: 340px;
  min-width: 260px;
  animation: tour-fadeIn 0.25s ease-out;
  transition: top 0.3s ease, left 0.3s ease;
}

.tour-tooltip-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
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
  margin: 0;
}

.tour-tooltip-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
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
