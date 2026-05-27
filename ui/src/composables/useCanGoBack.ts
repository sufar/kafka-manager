import { ref, onMounted, watch } from 'vue';
import { useRouter } from 'vue-router';

/**
 * 判断是否可以返回上一页
 * 通过检查 history.state.back 是否存在来判断
 */
export function useCanGoBack() {
  const router = useRouter();
  const canGoBack = ref(false);

  const checkCanGoBack = () => {
    // 检查 history.state 是否有 back 属性
    // Vue Router 在导航时会设置 state.back
    const state = window.history.state;
    canGoBack.value = state && state.back !== null && state.back !== undefined;
  };

  onMounted(() => {
    checkCanGoBack();
  });

  // 监听路由变化，重新检查
  watch(
    () => router.currentRoute.value,
    () => {
      checkCanGoBack();
    },
    { immediate: true }
  );

  const goBack = () => {
    if (canGoBack.value) {
      router.back();
    }
  };

  return { canGoBack, goBack };
}