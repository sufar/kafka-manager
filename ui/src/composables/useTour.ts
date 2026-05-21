import { ref, readonly } from 'vue';

export interface TourStep {
  selector: string;
  title: string;
  description: string;
  position?: 'top' | 'bottom' | 'left' | 'right';
}

const isActive = ref(false);
const currentStepIndex = ref(0);
const currentSteps = ref<TourStep[]>([]);
const targetRect = ref<DOMRect | null>(null);
const targetEl = ref<HTMLElement | null>(null);

let observer: ResizeObserver | null = null;

export function useTour() {
  function start(steps: TourStep[]) {
    currentSteps.value = steps;
    currentStepIndex.value = 0;
    isActive.value = true;
    updateTarget();
  }

  function next() {
    if (currentStepIndex.value < currentSteps.value.length - 1) {
      currentStepIndex.value++;
      updateTarget();
    } else {
      stop();
    }
  }

  function prev() {
    if (currentStepIndex.value > 0) {
      currentStepIndex.value--;
      updateTarget();
    }
  }

  function goTo(index: number) {
    if (index >= 0 && index < currentSteps.value.length) {
      currentStepIndex.value = index;
      updateTarget();
    }
  }

  function stop() {
    isActive.value = false;
    targetRect.value = null;
    targetEl.value = null;
    if (observer) {
      observer.disconnect();
      observer = null;
    }
  }

  function updateTarget() {
    if (observer) {
      observer.disconnect();
    }

    const step = currentSteps.value[currentStepIndex.value];
    if (!step) {
      targetRect.value = null;
      targetEl.value = null;
      return;
    }

    const el = document.querySelector(step.selector) as HTMLElement | null;
    if (el) {
      // 逐级向上查找滚动容器，确保目标元素在可见区域内
      let parent: HTMLElement | null = el.parentElement;
      while (parent) {
        const style = window.getComputedStyle(parent);
        const isScrollable = style.overflowY === 'auto' || style.overflowY === 'scroll' || style.overflow === 'auto' || style.overflow === 'scroll';
        if (isScrollable && parent.scrollHeight > parent.clientHeight) {
          const elRect = el.getBoundingClientRect();
          const parentRect = parent.getBoundingClientRect();
          // 如果元素在容器可视区域外，滚动到元素
          if (elRect.bottom > parentRect.bottom || elRect.top < parentRect.top) {
            el.scrollIntoView({ block: 'center' });
          }
          break;
        }
        parent = parent.parentElement;
      }
      // 也确保在文档视口中可见
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });

      targetEl.value = el;
      targetRect.value = el.getBoundingClientRect();
      observer = new ResizeObserver(() => {
        const rect = el.getBoundingClientRect();
        targetRect.value = rect;
      });
      observer.observe(el);
      // Also observe body for scroll-triggered changes
      const scrollHandler = () => {
        targetRect.value = el.getBoundingClientRect();
      };
      window.addEventListener('scroll', scrollHandler, true);
      // Clean up scroll listener on next/stop
      const cleanup = () => {
        window.removeEventListener('scroll', scrollHandler, true);
      };
      // Store cleanup for next invocation
      (updateTarget as any).cleanup?.();
      (updateTarget as any).cleanup = cleanup;
    } else {
      // 目标元素不存在，跳过到下一步
      if (currentStepIndex.value < currentSteps.value.length - 1) {
        currentStepIndex.value++;
        updateTarget();
      } else {
        stop();
      }
    }
  }

  return {
    isActive: readonly(isActive),
    currentStepIndex: readonly(currentStepIndex),
    currentSteps: readonly(currentSteps),
    targetRect: readonly(targetRect),
    currentStep: readonly(currentStepIndex),
    start,
    next,
    prev,
    goTo,
    stop,
  };
}

export type TourInstance = ReturnType<typeof useTour>;
