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
  escaped = escaped.replace(
    /("(?:\\.|[^"\\])*")(\s*:)?|(-?\d+\.?\d*)|\b(true|false|null)\b/g,
    (match, string, colon, number, bool) => {
      if (string) {
        if (colon) {
          return `<span class="json-key">${string}</span>${colon}`;
        } else {
          return `<span class="json-string">${string}</span>`;
        }
      }
      if (number) {
        return `<span class="json-number">${number}</span>`;
      }
      if (bool) {
        if (bool === 'true' || bool === 'false') {
          return `<span class="json-boolean">${bool}</span>`;
        }
        if (bool === 'null') {
          return `<span class="json-null">${bool}</span>`;
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
 * 格式化 JSON 字符串，保留超过 Number.MAX_SAFE_INTEGER 的整数精度
 * 递归解析 JSON，数字保持原始字符串形式
 */
export function formatJson(str: string): string {
  if (!str) return '';
  try {
    JSON.parse(str);
  } catch {
    return str;
  }

  let i = 0;
  const len = str.length;

  function skipWhitespace() {
    while (i < len && /\s/.test(str[i]!)) i++;
  }

  function parseValue(indent: number): string {
    skipWhitespace();
    const ch = str[i] ?? '';

    if (ch === '"') return parseString();
    if (ch === '{') return parseObject(indent + 1);
    if (ch === '[') return parseArray(indent + 1);
    if (ch === '-' || (ch >= '0' && ch <= '9')) return parseNumber();
    return parseKeyword();
  }

  function parseString(): string {
    let j = i + 1;
    while (j < len) {
      if (str[j] === '\\') { j += 2; }
      else if (str[j] === '"') { j++; break; }
      else { j++; }
    }
    const result = str.slice(i, j);
    i = j;
    return result;
  }

  function parseNumber(): string {
    let j = i;
    if (str[j] === '-') j++;
    while (j < len && str[j]! >= '0' && str[j]! <= '9') j++;
    if (j < len && str[j] === '.') {
      j++;
      while (j < len && str[j]! >= '0' && str[j]! <= '9') j++;
    }
    if (j < len && (str[j] === 'e' || str[j] === 'E')) {
      j++;
      if (j < len && (str[j] === '+' || str[j] === '-')) j++;
      while (j < len && str[j]! >= '0' && str[j]! <= '9') j++;
    }
    const result = str.slice(i, j);
    i = j;
    return result;
  }

  function parseKeyword(): string {
    let j = i;
    while (j < len && /[a-zA-Z]/.test(str[j]!)) j++;
    const result = str.slice(i, j);
    i = j;
    return result;
  }

  function parseObject(indent: number): string {
    i++; // skip '{'
    skipWhitespace();
    if (str[i] === '}') { i++; return '{}'; }

    const contentPad = '  '.repeat(indent);
    const closePad = '  '.repeat(indent - 1);
    let result = '{\n';
    let first = true;

    while (true) {
      if (!first && str[i] === ',') { i++; }
      first = false;
      skipWhitespace();
      if (str[i] === '}') { i++; result += closePad + '}'; break; }

      const key = parseString();
      skipWhitespace();
      i++; // skip ':'
      const val = parseValue(indent);
      result += contentPad + key + ': ' + val;

      skipWhitespace();
      if (str[i] === ',') {
        result += ',\n';
      } else {
        result += '\n';
      }
    }
    return result;
  }

  function parseArray(indent: number): string {
    i++; // skip '['
    skipWhitespace();
    if (str[i] === ']') { i++; return '[]'; }

    const contentPad = '  '.repeat(indent);
    const closePad = '  '.repeat(indent - 1);
    let result = '[\n';
    let first = true;

    while (true) {
      if (!first && str[i] === ',') { i++; }
      first = false;
      skipWhitespace();
      if (str[i] === ']') { i++; result += closePad + ']'; break; }

      const val = parseValue(indent);
      result += contentPad + val;

      skipWhitespace();
      if (str[i] === ',') {
        result += ',\n';
      } else {
        result += '\n';
      }
    }
    return result;
  }

  return parseValue(0);
}
