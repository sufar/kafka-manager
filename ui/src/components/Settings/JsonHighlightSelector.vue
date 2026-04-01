<template>
  <div class="card glass gradient-border hover:glow-primary transition-all duration-300">
    <div class="card-body p-3">
      <div class="flex items-center gap-2 mb-3">
        <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center glow-primary">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-primary">
            <path stroke-linecap="round" stroke-linejoin="round" d="M17.25 6.75 22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3-4.5 16.5" />
          </svg>
        </div>
        <div>
          <h2 class="text-base font-semibold text-gradient">{{ t.settings.jsonHighlight }}</h2>
          <p class="text-xs text-base-content/60">{{ t.settings.jsonHighlightDesc }}</p>
        </div>
      </div>

      <!-- 模板选择器 -->
      <div class="p-3 rounded-xl bg-base-100/50 mb-3">
        <label class="text-sm font-medium mb-2 block">{{ t.settings.selectTemplate }}</label>
        <select v-model="selectedTemplate" class="select select-bordered w-full" @change="onTemplateChange">
          <optgroup :label="t.settings.builtInTemplates">
            <option v-for="t in builtinTemplates" :key="t.name" :value="t.name">{{ t.name }} - {{ t.description }}</option>
          </optgroup>
          <optgroup :label="t.settings.customTemplates">
            <option v-for="t in customTemplates" :key="t.name" :value="t.name">{{ t.name }} - {{ t.description }}</option>
          </optgroup>
        </select>
      </div>

      <!-- 预览区域 -->
      <div class="p-3 rounded-xl bg-base-100/50 mb-3">
        <div class="flex items-center justify-between mb-2">
          <label class="text-sm font-medium">{{ t.settings.preview }}</label>
          <span class="text-xs text-base-content/50">{{ t.settings.templateFormat }}</span>
        </div>
        <div class="json-preview p-3 rounded-lg bg-base-200 font-mono text-xs overflow-auto max-h-48" v-html="previewHtml"></div>
      </div>

      <!-- 自定义模板操作 -->
      <div class="flex flex-wrap gap-2">
        <button v-if="!showCustomEditor && selectedTemplateInfo?.is_builtin" class="btn btn-sm btn-ghost" @click="copyCurrentTemplate">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 0 1-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 0 1 1.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 0 0-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 0 1-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 0 0-3.375-3.375h-1.5a1.125 1.125 0 0 1-1.125-1.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H9.75" />
          </svg>
          {{ t.settings.addCustomTemplate }}
        </button>
        <button v-if="!showCustomEditor && selectedTemplateInfo && !selectedTemplateInfo.is_builtin" class="btn btn-sm btn-ghost" @click="startEditCustomTemplate">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" />
          </svg>
          {{ t.common.edit }}
        </button>
        <button v-if="!showCustomEditor && selectedTemplateInfo && !selectedTemplateInfo.is_builtin" class="btn btn-sm btn-ghost text-error" @click="confirmDelete">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
          </svg>
          {{ t.settings.deleteTemplate }}
        </button>
      </div>

      <!-- 自定义模板编辑器 -->
      <div v-if="showCustomEditor" class="mt-3 space-y-3">
        <div>
          <label class="text-sm font-medium mb-1 block">{{ t.settings.templateName }}</label>
          <input v-model="customForm.name" type="text" class="input input-bordered input-sm w-full" :placeholder="t.settings.templateName" :disabled="isEditing" />
        </div>
        <div>
          <label class="text-sm font-medium mb-1 block">{{ t.settings.templateDescription }}</label>
          <input v-model="customForm.description" type="text" class="input input-bordered input-sm w-full" :placeholder="t.settings.templateDescription" />
        </div>
        <div>
          <label class="text-sm font-medium mb-1 block">{{ t.settings.templateStyle }}</label>
          <textarea v-model="customForm.style_json" class="textarea textarea-bordered w-full font-mono text-xs h-48" :placeholder="templateFormatHelp"></textarea>
        </div>
        <div class="flex gap-2">
          <button class="btn btn-sm btn-primary" @click="saveCustomTemplate" :disabled="saving">
            {{ saving ? t.common.loading : t.settings.saveTemplate }}
          </button>
          <button class="btn btn-sm btn-ghost" @click="cancelEdit">{{ t.common.cancel }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useLanguageStore } from '@/stores/language';
import { useThemeStore } from '@/stores/theme';
import { apiClient } from '@/api/client';
import { useToast } from '@/composables/useToast';

const languageStore = useLanguageStore();
const themeStore = useThemeStore();
const { showSuccess, showError, confirm } = useToast();
const t = computed(() => languageStore.t);
const { isDark } = themeStore;

interface ThemeStyles {
  key: { color: string; font_weight?: string };
  string: { color: string };
  number: { color: string };
  boolean: { color: string; font_weight?: string };
  null: { color: string; font_weight?: string };
  bracket: { color: string; font_weight?: string };
  colon: { color: string; font_weight?: string };
  comma: { color: string; font_weight?: string };
}

interface TemplateStyle {
  light: ThemeStyles;
  dark: ThemeStyles;
}

interface Template {
  id: number | null;
  name: string;
  description: string;
  is_builtin: boolean;
  style_json: string;
}

const templates = ref<Template[]>([]);
const selectedTemplate = ref<string>('default');
const showCustomEditor = ref(false);
const isEditing = ref(false);
const saving = ref(false);

const customForm = ref({
  name: '',
  description: '',
  style_json: '',
});

const templateFormatHelp = `{
  "light": {
    "key": { "color": "#9333ea", "font_weight": "600" },
    "string": { "color": "#059669" },
    "number": { "color": "#d97706" },
    "boolean": { "color": "#0284c7", "font_weight": "700" },
    "null": { "color": "#475569", "font_weight": "700" },
    "bracket": { "color": "#475569" },
    "colon": { "color": "#64748b" },
    "comma": { "color": "#64748b" }
  },
  "dark": {
    "key": { "color": "#c084fc", "font_weight": "600" },
    "string": { "color": "#34d399" },
    "number": { "color": "#fbbf24" },
    "boolean": { "color": "#38bdf8", "font_weight": "700" },
    "null": { "color": "#94a3b8", "font_weight": "700" },
    "bracket": { "color": "#94a3b8" },
    "colon": { "color": "#cbd5e1" },
    "comma": { "color": "#cbd5e1" }
  }
}`;

const builtinTemplates = computed(() => templates.value.filter(t => t.is_builtin));
const customTemplates = computed(() => templates.value.filter(t => !t.is_builtin));

const selectedTemplateInfo = computed(() => templates.value.find(t => t.name === selectedTemplate.value));

// 预览 JSON 示例
const previewJson = `{
  "id": 1,
  "name": "example",
  "active": true,
  "count": 42,
  "data": null,
  "tags": ["json", "highlight"]
}`;

// 生成预览 HTML
const previewHtml = computed(() => {
  if (!selectedTemplateInfo.value) return '';

  try {
    const style: TemplateStyle = JSON.parse(selectedTemplateInfo.value.style_json);
    // 根据当前主题选择浅色或深色样式
    const currentStyle = isDark ? style.dark : style.light;
    if (!currentStyle) {
      console.warn('Template style is undefined for current theme');
      return previewJson;
    }
    return highlightJson(previewJson, currentStyle);
  } catch (e) {
    console.error('Failed to parse template style:', e);
    return previewJson;
  }
});

// JSON 高亮
function highlightJson(json: string, style: any): string {
  let html = json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  html = html.replace(
    /("(?:\\.|[^"\\])*")(\s*:)?|(-?\d+\.?\d*)|\b(true|false|null)\b/g,
    (match, string, colon, number, bool) => {
      if (string) {
        if (colon) {
          return `<span class="json-key" style="color: ${style.key.color}; ${style.key.font_weight ? 'font-weight: ' + style.key.font_weight : ''}">${string}</span>${colon}`;
        } else {
          return `<span class="json-string" style="color: ${style.string.color}">${string}</span>`;
        }
      }
      if (number) {
        return `<span class="json-number" style="color: ${style.number.color}">${number}</span>`;
      }
      if (bool) {
        if (bool === 'true' || bool === 'false') {
          return `<span class="json-boolean" style="color: ${style.boolean.color}; ${style.boolean.font_weight ? 'font-weight: ' + style.boolean.font_weight : ''}">${bool}</span>`;
        }
        if (bool === 'null') {
          return `<span class="json-null" style="color: ${style.null.color}">${bool}</span>`;
        }
      }
      return match;
    }
  );

  return html;
}

// 加载模板列表
async function loadTemplates() {
  try {
    templates.value = await apiClient.getJsonHighlightTemplates();
    // 更新 sessionStorage 缓存
    await updateSessionStorageCache();
    // 初始加载时也触发事件，通知其他组件模板已就绪
    window.dispatchEvent(new CustomEvent('json-highlight-changed', {
      detail: { templateName: selectedTemplate.value || 'default' }
    }));
  } catch (e) {
    console.error('Failed to load templates:', e);
    showError(t.value.toast.operationFailed);
  }
}

// 加载当前选中的模板
async function loadCurrentTemplate() {
  try {
    const current = await apiClient.getCurrentJsonHighlightTemplate();
    if (current.name) {
      selectedTemplate.value = current.name;
    }
  } catch (e) {
    console.error('Failed to load current template:', e);
  }
}

// 模板改变时保存
async function onTemplateChange() {
  try {
    await apiClient.setCurrentJsonHighlightTemplate(selectedTemplate.value);
    // 更新 localStorage
    localStorage.setItem('json_highlight_template', selectedTemplate.value);
    // 更新 sessionStorage 缓存
    await updateSessionStorageCache();
    // 通知其他组件模板已改变
    window.dispatchEvent(new CustomEvent('json-highlight-changed', {
      detail: { templateName: selectedTemplate.value }
    }));
    showSuccess(t.value.common.success);
  } catch (e) {
    console.error('Failed to save template:', e);
    showError(t.value.toast.operationFailed);
  }
}

// 更新 sessionStorage 缓存
async function updateSessionStorageCache() {
  try {
    const allTemplates = await apiClient.getJsonHighlightTemplates();
    const templateMap: Record<string, any> = {};
    for (const t of allTemplates) {
      try {
        const style = JSON.parse(t.style_json);
        // 验证模板是否包含必需字段
        if (isValidTemplate(style)) {
          templateMap[t.name] = style;
        } else {
          console.warn(`Template ${t.name} has invalid structure, skipping`);
        }
      } catch (e) {
        console.error(`Failed to parse template ${t.name}:`, e);
      }
    }
    sessionStorage.setItem('json_highlight_templates', JSON.stringify(templateMap));
  } catch (e) {
    console.error('Failed to update template cache:', e);
  }
}

// 验证模板是否包含所有必需字段
function isValidTemplate(style: any): boolean {
  if (!style?.light || !style?.dark) return false;

  const requiredFields = ['key', 'string', 'number', 'boolean', 'null', 'bracket', 'colon', 'comma'];
  for (const theme of [style.light, style.dark]) {
    for (const field of requiredFields) {
      if (!theme[field]?.color) return false;
    }
  }
  return true;
}

// 复制当前模板为自定义
function copyCurrentTemplate() {
  if (!selectedTemplateInfo.value) return;

  customForm.value = {
    name: `${selectedTemplateInfo.value.name}_custom`,
    description: `${selectedTemplateInfo.value.description} (Custom)`,
    style_json: selectedTemplateInfo.value.style_json,
  };
  isEditing.value = false;
  showCustomEditor.value = true;
}

// 编辑自定义模板
function startEditCustomTemplate() {
  if (!selectedTemplateInfo.value) return;

  customForm.value = {
    name: selectedTemplateInfo.value.name,
    description: selectedTemplateInfo.value.description,
    style_json: selectedTemplateInfo.value.style_json,
  };
  isEditing.value = true;
  showCustomEditor.value = true;
}

// 保存自定义模板
async function saveCustomTemplate() {
  if (!customForm.value.name || !customForm.value.style_json) {
    showError(t.value.toast.invalidFormat);
    return;
  }

  // 验证 JSON 格式
  try {
    const style = JSON.parse(customForm.value.style_json);

    // 验证模板是否包含所有必需字段
    if (!isValidTemplate(style)) {
      showError('模板格式无效：必须包含 light 和 dark 主题的所有字段 (key, string, number, boolean, null, bracket, colon, comma)，且每个字段必须有 color 属性');
      return;
    }
  } catch (e) {
    showError('JSON 格式无效：' + (e as Error).message);
    return;
  }

  saving.value = true;
  try {
    if (isEditing.value && selectedTemplateInfo.value?.id) {
      await apiClient.updateJsonHighlightTemplate(selectedTemplateInfo.value.id, {
        description: customForm.value.description,
        style_json: customForm.value.style_json,
      });
      showSuccess(t.value.common.success);
    } else {
      await apiClient.createJsonHighlightTemplate(customForm.value);
      showSuccess(t.value.favorites.groupCreated);
    }

    await loadTemplates();
    await updateSessionStorageCache();
    showCustomEditor.value = false;
  } catch (e) {
    console.error('Failed to save template:', e);
    const errorMsg = e as { message?: string };
    showError(errorMsg.message || t.value.toast.operationFailed);
  } finally {
    saving.value = false;
  }
}

// 取消编辑
function cancelEdit() {
  showCustomEditor.value = false;
}

// 确认删除
async function confirmDelete() {
  const confirmed = await confirm(t.value.settings.confirmDeleteTemplate);
  if (!confirmed) {
    return;
  }
  deleteTemplate();
}

// 删除模板
async function deleteTemplate() {
  if (!selectedTemplateInfo.value?.id) return;

  try {
    await apiClient.deleteJsonHighlightTemplate(selectedTemplateInfo.value.id);
    showSuccess(t.value.common.success);
    await loadTemplates();
    // 如果删除的是当前模板，切换到默认
    if (selectedTemplate.value === selectedTemplateInfo.value.name) {
      selectedTemplate.value = 'default';
      await onTemplateChange();
    }
  } catch (e) {
    console.error('Failed to delete template:', e);
    showError(t.value.toast.operationFailed);
  }
}

onMounted(() => {
  loadTemplates();
  loadCurrentTemplate();
});
</script>

<style scoped>
.json-preview {
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
