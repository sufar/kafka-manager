/**
 * JSON 高亮工具函数
 * 将 JSON 字符串转换为带 HTML 标签的高亮文本
 * 适配 DaisyUI 主题
 */

export function highlightJson(json: string): string {
  return json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, (match) => {
      let cls = 'text-primary';
      if (/^"/.test(match)) {
        if (/:$/.test(match)) {
          cls = 'text-secondary font-semibold';
        } else {
          cls = 'text-accent';
        }
      } else if (/true|false/.test(match)) {
        cls = 'text-info font-bold';
      } else if (/null/.test(match)) {
        cls = 'text-warning font-bold';
      }
      return `<span class="${cls}">${match}</span>`;
    });
}

/**
 * 检查字符串是否为有效 JSON
 */
export function isValidJson(str: string): boolean {
  if (!str) return false;
  try {
    JSON.parse(str);
    return true;
  } catch {
    return false;
  }
}

/**
 * 格式化 JSON 字符串
 */
export function formatJson(str: string): string {
  if (!str) return '';
  try {
    const parsed = JSON.parse(str);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return str;
  }
}
