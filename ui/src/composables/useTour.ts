import { ref, readonly, nextTick } from 'vue';

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

/**
 * Walk up the DOM tree from `el` to find the nearest ancestor that is both
 * visible (non-zero dimensions) AND has visual styling (background, border,
 * shadow, or non-transparent color). This avoids highlighting invisible
 * layout wrappers in tree mode.
 *
 * Also checks previous siblings — in tree mode, the cluster header (always visible)
 * is a sibling of the v-show container, not a parent.
 */
function findVisibleAncestor(el: HTMLElement): HTMLElement {
  let best: HTMLElement | null = null;
  let check: HTMLElement | null = el;
  while (check) {
    const rect = check.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      const style = window.getComputedStyle(check);
      const hasVisual =
        style.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
        style.backgroundColor !== 'transparent' ||
        style.borderStyle !== 'none' ||
        style.boxShadow !== 'none';
      if (hasVisual) {
        return check;
      }
      // Keep track of the first visible ancestor as fallback
      if (!best) {
        best = check;
      }
    }
    // Check previous siblings for elements with visual styling
    // In tree mode: the cluster header is a sibling of the v-show container
    if (check.parentElement) {
      let sibling = check.previousElementSibling as HTMLElement | null;
      while (sibling) {
        const sRect = sibling.getBoundingClientRect();
        if (sRect.width > 0 && sRect.height > 0) {
          const sStyle = window.getComputedStyle(sibling);
          const sHasVisual =
            sStyle.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
            sStyle.backgroundColor !== 'transparent' ||
            sStyle.borderStyle !== 'none' ||
            sStyle.boxShadow !== 'none';
          if (sHasVisual) {
            return sibling;
          }
        }
        sibling = sibling.previousElementSibling as HTMLElement | null;
      }
    }
    check = check.parentElement;
  }
  return best || el;
}

export function useTour() {
  async function start(steps: TourStep[]) {
    currentSteps.value = steps;
    currentStepIndex.value = 0;
    isActive.value = true;
    await updateTarget();
  }

  async function next() {
    if (currentStepIndex.value < currentSteps.value.length - 1) {
      currentStepIndex.value++;
      await updateTarget();
    } else {
      stop();
    }
  }

  async function prev() {
    if (currentStepIndex.value > 0) {
      currentStepIndex.value--;
      await updateTarget();
    }
  }

  async function goTo(index: number) {
    if (index >= 0 && index < currentSteps.value.length) {
      currentStepIndex.value = index;
      await updateTarget();
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

  async function updateTarget() {
    if (observer) {
      observer.disconnect();
    }

    const step = currentSteps.value[currentStepIndex.value];
    console.log('[tour] updateTarget step:', currentStepIndex.value, 'selector:', step?.selector);
    if (!step) {
      targetRect.value = null;
      targetEl.value = null;
      return;
    }

    // Wait for DOM to settle — important for tree mode where v-show and
    // RecycleScroller need an extra tick to render
    await nextTick();

    const el = document.querySelector(step.selector) as HTMLElement | null;
    if (!el) {
      console.warn('[tour] Element not found:', step.selector, ', skipping');
      if (currentStepIndex.value < currentSteps.value.length - 1) {
        currentStepIndex.value++;
        await updateTarget();
      } else {
        stop();
      }
      return;
    }

    // Use visible ancestor for spotlight when element is hidden (display:none via v-show)
    const target = findVisibleAncestor(el);

    // Ensure the target is scrolled into view within its scroll container
    let parent: HTMLElement | null = target.parentElement;
    while (parent) {
      const style = window.getComputedStyle(parent);
      const isScrollable = (style.overflowY === 'auto' || style.overflowY === 'scroll' || style.overflow === 'auto' || style.overflow === 'scroll')
        && parent.scrollHeight > parent.clientHeight;
      if (isScrollable) {
        const rect = target.getBoundingClientRect();
        const parentRect = parent.getBoundingClientRect();
        if (rect.top < parentRect.top) {
          parent.scrollTop += (rect.top - parentRect.top) - 8;
        } else if (rect.bottom > parentRect.bottom) {
          parent.scrollTop += (rect.bottom - parentRect.bottom) + 8;
        }
        break;
      }
      parent = parent.parentElement;
    }

    targetEl.value = el;
    targetRect.value = target.getBoundingClientRect();
    observer = new ResizeObserver(() => {
      targetRect.value = target.getBoundingClientRect();
    });
    observer.observe(target);
    const scrollHandler = () => {
      targetRect.value = target.getBoundingClientRect();
    };
    window.addEventListener('scroll', scrollHandler, true);
    const cleanup = () => {
      window.removeEventListener('scroll', scrollHandler, true);
    };
    (updateTarget as any).cleanup?.();
    (updateTarget as any).cleanup = cleanup;
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
