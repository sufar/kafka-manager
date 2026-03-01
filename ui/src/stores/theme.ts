import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { apiClient } from '@/api/client';

export const useThemeStore = defineStore('theme', () => {
  const isLoading = ref(false);
  const isInitialized = ref(false);
  const currentTheme = ref<'light' | 'dark'>('light');
  const isDark = computed(() => currentTheme.value === 'dark');

  // 从 localStorage 加载临时主题偏好（在初始化之前使用）
  const loadTemporaryTheme = (): 'light' | 'dark' => {
    if (typeof window === 'undefined') return 'light';
    const saved = localStorage.getItem('kafka-manager-theme');
    if (saved === 'dark' || saved === 'light') return saved;
    // 默认跟随系统
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'dark';
    }
    return 'light';
  };

  // 应用主题到 HTML 元素（DaisyUI 5 使用 data-theme 属性）
  const applyTheme = (theme: 'light' | 'dark') => {
    if (typeof document === 'undefined') return;
    const html = document.documentElement;
    // DaisyUI 5 使用 data-theme 属性
    html.setAttribute('data-theme', theme);
    // 同时保留 dark class 用于 Tailwind 的 dark: 变体
    if (theme === 'dark') {
      html.classList.add('dark');
    } else {
      html.classList.remove('dark');
    }
  };

  // 从数据库加载主题设置
  const loadThemeFromDatabase = async (): Promise<'light' | 'dark' | null> => {
    try {
      const settings = await apiClient.getSettings(['ui.theme']);
      if (settings.length > 0 && settings[0]?.value) {
        const theme = settings[0].value as 'light' | 'dark';
        if (theme === 'light' || theme === 'dark') {
          return theme;
        }
      }
      return null;
    } catch (e) {
      console.warn('Failed to load theme from database, using localStorage');
      return null;
    }
  };

  // 保存主题到数据库
  const saveThemeToDatabase = async (theme: 'light' | 'dark') => {
    try {
      await apiClient.updateSetting('ui.theme', theme);
      // 成功后也更新 localStorage 作为缓存
      localStorage.setItem('kafka-manager-theme', theme);
    } catch (e) {
      console.warn('Failed to save theme to database, using localStorage');
      localStorage.setItem('kafka-manager-theme', theme);
    }
  };

  // 切换主题
  const toggleTheme = async () => {
    const newTheme = currentTheme.value === 'light' ? 'dark' : 'light';
    currentTheme.value = newTheme;
    applyTheme(newTheme);
    await saveThemeToDatabase(newTheme);
  };

  // 设置主题
  const setTheme = async (theme: 'light' | 'dark') => {
    currentTheme.value = theme;
    applyTheme(theme);
    await saveThemeToDatabase(theme);
  };

  // 初始化主题（从数据库加载）
  const initTheme = async () => {
    if (isInitialized.value) return;

    isLoading.value = true;

    // 先应用临时主题（从 localStorage 或系统偏好）
    const temporaryTheme = loadTemporaryTheme();
    applyTheme(temporaryTheme);
    currentTheme.value = temporaryTheme;

    // 然后从数据库加载持久化主题
    try {
      const savedTheme = await loadThemeFromDatabase();
      if (savedTheme) {
        currentTheme.value = savedTheme;
        applyTheme(savedTheme);
      }
    } finally {
      isLoading.value = false;
      isInitialized.value = true;
    }
  };

  return {
    currentTheme,
    isDark,
    isLoading,
    isInitialized,
    toggleTheme,
    setTheme,
    initTheme,
  };
});
