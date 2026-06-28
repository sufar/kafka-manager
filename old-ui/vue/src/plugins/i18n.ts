import type { App, ComputedRef } from 'vue';
import type { Translation } from '@/i18n/translations';
import { useLanguageStore } from '@/stores/language';

declare module '@vue/runtime-core' {
  interface ComponentCustomProperties {
    $t: (key: string) => string;
  }
}

interface LanguageStoreWithComputedT {
  t: ComputedRef<Translation>;
}

export function createI18nPlugin(languageStore: LanguageStoreWithComputedT & ReturnType<typeof useLanguageStore>) {
  return {
    install(app: App) {
      // 全局方法 $t
      app.config.globalProperties.$t = (key: string): string => {
        const keys = key.split('.');
        let value: any = languageStore.t.value;

        for (const k of keys) {
          if (value && typeof value === 'object' && k in value) {
            value = value[k];
          } else {
            return key;
          }
        }

        return value || key;
      };
    },
  };
}
