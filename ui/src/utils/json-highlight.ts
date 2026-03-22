/// JSON 高亮样式工具
/// 根据当前选中的模板和主题（深色/浅色）生成高亮 HTML

interface ThemeStyles {
  key: { color: string; font_weight?: string };
  string: { color: string };
  number: { color: string };
  boolean: { color: string; font_weight?: string };
  null: { color: string; font_weight?: string };
  bracket: { color: string };
  colon: { color: string };
  comma: { color: string };
}

interface TemplateStyle {
  light: ThemeStyles;
  dark: ThemeStyles;
}

// 默认模板样式 - 柔和现代风格，金色数字
const defaultTemplate: TemplateStyle = {
  light: {
    key: { color: '#a78bfa', font_weight: '600' },
    string: { color: '#34d399' },
    number: { color: '#fbbf24' },
    boolean: { color: '#38bdf8', font_weight: '700' },
    null: { color: '#6b7280', font_weight: '700' },
    bracket: { color: '#9ca3af' },
    colon: { color: '#9ca3af' },
    comma: { color: '#9ca3af' },
  },
  dark: {
    key: { color: '#a78bfa', font_weight: '600' },
    string: { color: '#34d399' },
    number: { color: '#fbbf24' },
    boolean: { color: '#38bdf8', font_weight: '700' },
    null: { color: '#6b7280', font_weight: '700' },
    bracket: { color: '#9ca3af' },
    colon: { color: '#9ca3af' },
    comma: { color: '#9ca3af' },
  },
};

// 缓存已解析的模板
const templateCache = new Map<string, TemplateStyle>();

// 获取当前选中的模板样式
export function getCurrentTemplateStyle(): TemplateStyle {
  try {
    const templateName = localStorage.getItem('json_highlight_template') || 'default';

    if (templateCache.has(templateName)) {
      return templateCache.get(templateName)!;
    }

    // 尝试从 sessionStorage 获取模板样式
    const savedTemplates = sessionStorage.getItem('json_highlight_templates');
    if (savedTemplates) {
      const templates: Record<string, TemplateStyle> = JSON.parse(savedTemplates);
      if (templates[templateName]) {
        templateCache.set(templateName, templates[templateName]);
        return templates[templateName];
      }
    }

    return defaultTemplate;
  } catch (e) {
    console.error('Failed to get template style:', e);
    return defaultTemplate;
  }
}

// 根据当前主题（深色/浅色）获取样式
export function getThemeStyle(isDark: boolean): ThemeStyles {
  const template = getCurrentTemplateStyle();
  return isDark ? template.dark : template.light;
}

// JSON 高亮
export function highlightJsonWithTemplate(json: string, isDark?: boolean): string {
  if (!json) return '';

  // 如果没有传入 isDark 参数，直接从 DOM 获取当前主题
  const useDark = isDark !== undefined
    ? (typeof isDark === 'boolean' ? isDark : !!isDark)
    : document.documentElement.getAttribute('data-theme') === 'dark';

  const style = getThemeStyle(useDark);

  // 转义 HTML 特殊字符
  let html = json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  // 使用单个正则表达式处理所有情况
  html = html.replace(
    /("(?:\\.|[^"\\])*")(\s*:)?|(-?\d+\.?\d*)|\b(true|false|null)\b/g,
    (match, string, colon, number, bool) => {
      if (string) {
        if (colon) {
          // 键名
          const fontWeight = style.key.font_weight ? `font-weight: ${style.key.font_weight};` : '';
          return `<span class="json-key" style="color: ${style.key.color}; ${fontWeight}">${string}</span>${colon}`;
        } else {
          // 字符串值
          return `<span class="json-string" style="color: ${style.string.color}">${string}</span>`;
        }
      }
      if (number) {
        // 数字
        return `<span class="json-number" style="color: ${style.number.color}">${number}</span>`;
      }
      if (bool) {
        if (bool === 'true' || bool === 'false') {
          // 布尔值
          const fontWeight = style.boolean.font_weight ? `font-weight: ${style.boolean.font_weight};` : '';
          return `<span class="json-boolean" style="color: ${style.boolean.color}; ${fontWeight}">${bool}</span>`;
        }
        if (bool === 'null') {
          // null
          return `<span class="json-null" style="color: ${style.null.color}">${bool}</span>`;
        }
      }
      return match;
    }
  );

  return html;
}

// 清除缓存
export function clearTemplateCache() {
  templateCache.clear();
}

// 预加载模板
export function preloadTemplates(templates: Array<{ name: string; style_json: string }>) {
  try {
    const templateMap: Record<string, TemplateStyle> = {};
    for (const t of templates) {
      templateMap[t.name] = JSON.parse(t.style_json);
    }
    sessionStorage.setItem('json_highlight_templates', JSON.stringify(templateMap));
    templateCache.clear();
  } catch (e) {
    console.error('Failed to preload templates:', e);
  }
}
