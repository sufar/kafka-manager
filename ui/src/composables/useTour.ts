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
      // 逐级向上查找内部滚动容器，确保目标元素在容器可见区域内
      // 注意：只操作容器自身的 scrollTop，不调用 scrollIntoView 以免影响文档滚动
      let parent: HTMLElement | null = el.parentElement;
      while (parent) {
        const style = window.getComputedStyle(parent);
        const isScrollable = (style.overflowY === 'auto' || style.overflowY === 'scroll' || style.overflow === 'auto' || style.overflow === 'scroll')
          && parent.scrollHeight > parent.clientHeight;
        if (isScrollable) {
          const elRect = el.getBoundingClientRect();
          const parentRect = parent.getBoundingClientRect();
          // 元素在可视区域上方：向上滚动
          if (elRect.top < parentRect.top) {
            parent.scrollTop += (elRect.top - parentRect.top) - 8;
          }
          // 元素在可视区域下方：向下滚动
          else if (elRect.bottom > parentRect.bottom) {
            parent.scrollTop += (elRect.bottom - parentRect.bottom) + 8;
          }
          break;
        }
        parent = parent.parentElement;
      }

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
