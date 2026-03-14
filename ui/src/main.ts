import { createApp } from 'vue';
import { createPinia } from 'pinia';
import './style.css';
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css';
import App from './App.vue';
import router from './router';
import { useThemeStore } from './stores/theme';
import { useLanguageStore } from './stores/language';
import { createI18nPlugin } from './plugins/i18n';

const app = createApp(App);
const pinia = createPinia();

// 初始化主题
const themeStore = useThemeStore(pinia);
themeStore.initTheme();

// 初始化语言
const languageStore = useLanguageStore(pinia);
languageStore.initLanguage();

// 注册 i18n 插件（使用类型断言）
app.use(createI18nPlugin(languageStore as any));

// Global error handler
app.config.errorHandler = (err, vm, info) => {
  console.error('[Global Error Handler]', err, vm, info);
};

// Router error handling
router.onError((error, to) => {
  console.error('[Router Error]', error, to);
});

app.use(pinia);
app.use(router);
app.mount('#app');
