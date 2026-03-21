/**
 * JSON 高亮工具函数
 * 将 JSON 字符串转换为带 HTML 标签的高亮文本
 * 适配 DaisyUI 主题
 */

export function highlightJson(json: string): string {
  if (!json) return '';

  // 先转义 HTML 特殊字符
  let escaped = json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  // 使用单个正则表达式处理所有情况
  // 注意：数字前面的空格会保留，不会被匹配进去
  escaped = escaped.replace(
    /("(?:\\.|[^"\\])*")(\s*:)?|(-?\d+\.?\d*)|\b(true|false|null)\b/g,
    (match, string, colon, number, bool) => {
      if (string) {
        if (colon) {
          // 键名
          return `<span class="text-secondary font-semibold">${string}</span>${colon}`;
        } else {
          // 字符串值
          return `<span class="text-accent">${string}</span>`;
        }
      }
      if (number) {
        return `<span class="text-base-content">${number}</span>`;
      }
      if (bool) {
        if (bool === 'true' || bool === 'false') {
          return `<span class="text-info font-bold">${bool}</span>`;
        }
        if (bool === 'null') {
          return `<span class="text-warning font-bold">${bool}</span>`;
        }
      }
      return match;
    }
  );

  return escaped;
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
