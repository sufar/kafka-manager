import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { apiClient } from '@/api/client';
import type { Language } from '@/i18n/translations';
import { translations } from '@/i18n/translations';

export const useLanguageStore = defineStore('language', () => {
  const isLoading = ref(false);
  const isInitialized = ref(false);
  const currentLanguage = ref<Language>('zh');

  // 当前翻译
  const t = computed(() => translations[currentLanguage.value]);

  // 从 localStorage 加载临时语言偏好
  const loadTemporaryLanguage = (): Language => {
    if (typeof window === 'undefined') return 'zh';
    const saved = localStorage.getItem('kafka-manager-language');
    if (saved === 'zh' || saved === 'en') return saved;
    // 默认中文
    return 'zh';
  };

  // 应用语言到 HTML 元素
  const applyLanguage = (lang: Language) => {
    if (typeof document === 'undefined') return;
    document.documentElement.setAttribute('lang', lang);
  };

  // 从数据库加载语言设置
  const loadLanguageFromDatabase = async (): Promise<Language | null> => {
    try {
      const settings = await apiClient.getSettings(['ui.language']);
      if (settings.length > 0 && settings[0]?.value) {
        const lang = settings[0].value as Language;
        if (lang === 'zh' || lang === 'en') {
          return lang;
        }
      }
      return null;
    } catch (e) {
      console.warn('Failed to load language from database, using localStorage');
      return null;
    }
  };

  // 保存语言到数据库
  const saveLanguageToDatabase = async (lang: Language) => {
    try {
      await apiClient.updateSetting('ui.language', lang);
      // 成功后也更新 localStorage 作为缓存
      localStorage.setItem('kafka-manager-language', lang);
    } catch (e) {
      console.warn('Failed to save language to database, using localStorage');
      localStorage.setItem('kafka-manager-language', lang);
    }
  };

  // 切换语言
  const toggleLanguage = async () => {
    const newLang = currentLanguage.value === 'zh' ? 'en' : 'zh';
    currentLanguage.value = newLang;
    applyLanguage(newLang);
    await saveLanguageToDatabase(newLang);
  };

  // 设置语言
  const setLanguage = async (lang: Language) => {
    currentLanguage.value = lang;
    applyLanguage(lang);
    await saveLanguageToDatabase(lang);
  };

  // 初始化语言（从数据库加载）
  const initLanguage = async () => {
    if (isInitialized.value) return;

    isLoading.value = true;

    // 先应用临时语言（从 localStorage）
    const temporaryLang = loadTemporaryLanguage();
    applyLanguage(temporaryLang);
    currentLanguage.value = temporaryLang;

    // 然后从数据库加载持久化语言
    try {
      const savedLang = await loadLanguageFromDatabase();
      if (savedLang) {
        currentLanguage.value = savedLang;
        applyLanguage(savedLang);
      }
    } finally {
      isLoading.value = false;
      isInitialized.value = true;
    }
  };

  return {
    currentLanguage,
    t,
    isLoading,
    isInitialized,
    toggleLanguage,
    setLanguage,
    initLanguage,
  };
});
