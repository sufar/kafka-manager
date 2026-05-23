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
    console.log('[tour] updateTarget called, step index:', currentStepIndex.value, 'selector:', step?.selector);
    if (!step) {
      targetRect.value = null;
      targetEl.value = null;
      return;
    }

    const el = document.querySelector(step.selector) as HTMLElement | null;
    if (el) {
      console.log('[tour] Found element for selector:', step.selector, el.tagName, el.className);

      // If the element is hidden (e.g. inside a collapsed tree node),
      // getBoundingClientRect returns zeros. Walk up to find the nearest
      // visible ancestor with non-zero dimensions.
      let target: HTMLElement = el;
      let check = el;
      while (check) {
        const rect = check.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
          target = check;
          break;
        }
        check = check.parentElement;
      }

      // 逐级向上查找内部滚动容器，确保目标元素在容器可见区域内
      // 注意：只操作容器自身的 scrollTop，不调用 scrollIntoView 以免影响文档滚动
      let parent: HTMLElement | null = el.parentElement;
      let foundScrollable = false;
      while (parent) {
        const style = window.getComputedStyle(parent);
        const isScrollable = (style.overflowY === 'auto' || style.overflowY === 'scroll' || style.overflow === 'auto' || style.overflow === 'scroll')
          && parent.scrollHeight > parent.clientHeight;
        if (isScrollable) {
          foundScrollable = true;
          console.log('[tour] Found scrollable parent:', parent.tagName, parent.className, 'scrollHeight:', parent.scrollHeight, 'clientHeight:', parent.clientHeight);
          const targetRect_ = target.getBoundingClientRect();
          const parentRect = parent.getBoundingClientRect();
          console.log('[tour] Target rect:', { top: targetRect_.top, bottom: targetRect_.bottom }, 'Parent rect:', { top: parentRect.top, bottom: parentRect.bottom });
          // 元素在可视区域上方：向上滚动
          if (targetRect_.top < parentRect.top) {
            parent.scrollTop += (targetRect_.top - parentRect.top) - 8;
          }
          // 元素在可视区域下方：向下滚动
          else if (targetRect_.bottom > parentRect.bottom) {
            parent.scrollTop += (targetRect_.bottom - parentRect.bottom) + 8;
          }
          break;
        }
        parent = parent.parentElement;
      }
      if (!foundScrollable) {
        console.log('[tour] No scrollable parent found for:', step.selector);
      }

      targetEl.value = el;
      targetRect.value = target.getBoundingClientRect();
      console.log('[tour] targetRect set to:', targetRect.value);
      observer = new ResizeObserver(() => {
        const rect = target.getBoundingClientRect();
        targetRect.value = rect;
      });
      observer.observe(target);
      // Also observe body for scroll-triggered changes
      const scrollHandler = () => {
        targetRect.value = target.getBoundingClientRect();
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
      console.warn('[tour] Element not found for selector:', step.selector, ', skipping to next or stopping');
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
