import { inject } from 'vue';

/**
 * 全局 Toast 通知 Hook
 * 用于在应用中显示统一风格的通知消息
 */
export function useToast() {
  // 注入全局 showToast 方法
  const showToast = inject<(type: 'success' | 'error' | 'warning' | 'info', message: string, duration?: number) => void>(
    'showToast',
    (type, message) => {
      // 降级方案：输出到控制台，不阻塞用户
      console.warn(`[Toast:${type}]`, message);
    }
  );

  // 注入全局 confirm 方法
  const showConfirm = inject<(message: string) => Promise<boolean>>(
    'showConfirm',
    () => {
      // 降级方案：默认返回 false（取消）
      console.warn('[Confirm] 降级方案：返回 false');
      return Promise.resolve(false);
    }
  );

  const showError = (message: string) => {
    showToast('error', message);
  };

  const showSuccess = (message: string, duration?: number) => {
    showToast('success', message, duration);
  };

  const showWarning = (message: string, duration?: number) => {
    showToast('warning', message, duration);
  };

  const showInfo = (message: string, duration?: number) => {
    showToast('info', message, duration);
  };

  const confirm = (message: string): Promise<boolean> => {
    return showConfirm(message);
  };

  return {
    showToast,
    showError,
    showSuccess,
    showWarning,
    showInfo,
    confirm,
  };
}
