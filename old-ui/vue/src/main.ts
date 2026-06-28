import { createApp } from 'vue';
import { createPinia } from 'pinia';
import './style.css';
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css';
import App from './App.vue';
import router from './router';
import { useThemeStore } from './stores/theme';
import { useLanguageStore } from './stores/language';
import { createI18nPlugin } from './plugins/i18n';
import { apiClient } from './api/client';
import { preloadTemplates } from './utils/json-highlight';

const app = createApp(App);
const pinia = createPinia();

// 初始化主题
const themeStore = useThemeStore(pinia);
themeStore.initTheme();

// 初始化语言
const languageStore = useLanguageStore(pinia);
languageStore.initLanguage();

// 预加载 JSON 高亮模板到 localStorage（不是 sessionStorage）
// 这样即使异步加载未完成，组件也能使用上次保存的模板
async function loadJsonHighlightTemplates() {
  try {
    const templates = await apiClient.getJsonHighlightTemplates();
    preloadTemplates(templates);
  } catch (e) {
    console.error('[JSON Highlight] Failed to load templates:', e);
  }
}
// 在应用挂载前等待模板加载（但不阻塞太久）
loadJsonHighlightTemplates();

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

// 阻止浏览器默认的右键菜单
document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
  return false;
});

app.mount('#app');

// 监听后端启动时自动检查更新的结果
import { listen } from '@tauri-apps/api/event';

listen('update-available', (event) => {
  const payload = event.payload as { version: string; notes?: string };
  const isZh = languageStore.currentLanguage === 'zh';

  const el = document.createElement('div');
  el.className = 'fixed top-4 right-4 z-[9999] max-w-sm bg-warning/90 backdrop-blur rounded-lg px-4 py-3 shadow-lg text-sm text-base-content';
  if (isZh) {
    el.innerHTML = `<div class="font-bold mb-1">🔄 发现新版本 v${payload.version}</div><div class="text-xs text-base-content/80">请前往 <b>设置</b> 页面进行更新</div>`;
  } else {
    el.innerHTML = `<div class="font-bold mb-1">🔄 New version v${payload.version} available</div><div class="text-xs text-base-content/80">Please go to <b>Settings</b> to update</div>`;
  }
  document.body.appendChild(el);
  setTimeout(() => {
    el.style.opacity = '0';
    el.style.transition = 'opacity 0.3s';
    setTimeout(() => el.remove(), 300);
  }, 8000);
}).catch(() => {});
