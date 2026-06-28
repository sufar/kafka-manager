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
 * Expand tree containers in ClusterTreeNavigator to make hidden elements visible.
 * Strategy: Find the first cluster and expand it, then expand topics/consumer-groups folder if needed.
 */
function expandTreeContainers(el: HTMLElement): void {
  // Check if element or its ancestors are hidden (display: none)
  let isHidden = false;
  let current: HTMLElement | null = el;

  while (current) {
    const style = window.getComputedStyle(current);
    if (style.display === 'none') {
      isHidden = true;
      break;
    }
    current = current.parentElement;
  }

  if (!isHidden) {
    return; // Element is visible, no need to expand
  }

  // Determine what needs to be expanded based on the target element
  const tourAttr = el.getAttribute('data-tour') || '';
  const needsTopicsExpand = tourAttr.includes('tree-topics') || tourAttr.includes('tree-topic') ||
    el.closest('[data-tour="tree-topics-folder"]') !== null;
  const needsConsumerGroupsExpand = tourAttr.includes('tree-consumer-group') ||
    el.closest('[data-tour="tree-consumer-groups-folder"]') !== null;

  // Find and click the first cluster expand button (arrow icon in cluster header)
  const clusterHeaders = document.querySelectorAll('.flex.items-center.p-2.rounded-xl');
  for (const header of clusterHeaders) {
    const expandBtn = header.querySelector('button.btn-ghost.btn-xs.p-0') as HTMLElement;
    if (expandBtn && expandBtn.querySelector('svg')) {
      expandBtn.click();
      break; // Only expand first cluster
    }
  }

  // After cluster expansion, expand topics/consumer-groups folder if needed
  if (needsTopicsExpand) {
    setTimeout(() => {
      const topicsFolder = document.querySelector('[data-tour="tree-topics-folder"]') as HTMLElement;
      if (topicsFolder) {
        topicsFolder.click();
      }
    }, 150);
  }

  if (needsConsumerGroupsExpand) {
    setTimeout(() => {
      const cgFolder = document.querySelector('[data-tour="tree-consumer-groups-folder"]') as HTMLElement;
      if (cgFolder) {
        cgFolder.click();
      }
    }, 150);
  }
}

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

  // First, check if the element itself is visible with non-zero dimensions
  const elRect = el.getBoundingClientRect();
  if (elRect.width > 0 && elRect.height > 0) {
    // If element is visible, use it directly (even if no visual styling)
    // This handles buttons like btn-ghost which have transparent background
    return el;
  }

  while (check) {
    const rect = check.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      const style = window.getComputedStyle(check);
      // Relax the visual check: accept elements with any background (including transparent)
      // or elements with border/shadow, or elements that are interactive (buttons, inputs)
      const isInteractive = check.tagName === 'BUTTON' || check.tagName === 'INPUT' ||
        check.tagName === 'SELECT' || check.tagName === 'A' ||
        check.getAttribute('role') === 'button';
      const hasVisual =
        style.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
        style.backgroundColor !== 'transparent' ||
        style.borderStyle !== 'none' ||
        style.boxShadow !== 'none';
      if (hasVisual || isInteractive) {
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
          const sIsInteractive = sibling.tagName === 'BUTTON' || sibling.tagName === 'INPUT';
          const sHasVisual =
            sStyle.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
            sStyle.backgroundColor !== 'transparent' ||
            sStyle.borderStyle !== 'none' ||
            sStyle.boxShadow !== 'none';
          if (sHasVisual || sIsInteractive) {
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
      // Element not found, skip to next step
      if (currentStepIndex.value < currentSteps.value.length - 1) {
        currentStepIndex.value++;
        await updateTarget();
      } else {
        stop();
      }
      return;
    }

    // Check if element is hidden by v-show (display: none) and try to expand tree containers
    const elStyle = window.getComputedStyle(el);
    if (elStyle.display === 'none') {
      expandTreeContainers(el);
      // Wait for Vue to process the expansion and render
      await nextTick();
      // Additional delay for animation and RecycleScroller to settle
      await new Promise(resolve => setTimeout(resolve, 300));

      // Re-check if element is now visible
      const newStyle = window.getComputedStyle(el);
      if (newStyle.display === 'none') {
        // Element still hidden, skip to next step
        if (currentStepIndex.value < currentSteps.value.length - 1) {
          currentStepIndex.value++;
          await updateTarget();
        } else {
          stop();
        }
        return;
      }
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