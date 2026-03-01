import { createApp } from 'vue';
import { createPinia } from 'pinia';
import './style.css';
import App from './App.vue';
import router from './router';
import { useThemeStore } from './stores/theme';
import { useLanguageStore } from './stores/language';

const app = createApp(App);
const pinia = createPinia();

// 初始化主题
const themeStore = useThemeStore(pinia);
themeStore.initTheme();

// 初始化语言
const languageStore = useLanguageStore(pinia);
languageStore.initLanguage();

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
