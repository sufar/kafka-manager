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
      // 降级方案：如果是错误则 alert，其他类型忽略
      if (type === 'error') {
        alert(message);
      }
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

  return {
    showToast,
    showError,
    showSuccess,
    showWarning,
    showInfo,
  };
}
