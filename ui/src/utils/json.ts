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
          return `<span class="json-key">${string}</span>${colon}`;
        } else {
          // 字符串值
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
 * 通过 token 化方式重建格式化结果，不依赖 JSON.parse 处理数字
 */
export function formatJson(str: string): string {
  if (!str) return '';
  try {
    // 先用 JSON.parse 验证有效性
    JSON.parse(str);
  } catch {
    return str;
  }

  // Token 化：提取所有 JSON token，保留数字原始字符串
  const tokens: string[] = [];
  let i = 0;
  const len = str.length;

  while (i < len) {
    const ch = str[i];

    // 跳过空白
    if (ch === ' ' || ch === '\n' || ch === '\r' || ch === '\t') {
      i++;
      continue;
    }

    // 字符串
    if (ch === '"') {
      let j = i + 1;
      while (j < len) {
        if (str[j] === '\\') {
          j += 2;
        } else if (str[j] === '"') {
          j++;
          break;
        } else {
          j++;
        }
      }
      tokens.push(str.slice(i, j));
      i = j;
      continue;
    }

    // 结构字符和数字
    if (ch !== undefined && ('{}[],:'.includes(ch) || ch === '-' || (ch >= '0' && ch <= '9'))) {
      let j = i + 1;
      while (j < len) {
        const nextCh = str[j];
        if (nextCh === undefined || '{}[],: \n\r\t'.includes(nextCh)) break;
        j++;
      }
      tokens.push(str.slice(i, j));
      i = j;
      continue;
    }

    i++;
  }

  // 从 tokens 重建格式化 JSON
  let result = '';
  let indent = 0;
  const pad = () => '  '.repeat(indent);

  for (let idx = 0; idx < tokens.length; idx++) {
    const token = tokens[idx];
    const nextToken = idx < tokens.length - 1 ? tokens[idx + 1] : '';

    switch (token) {
      case '{':
      case '[':
        result += token + '\n';
        indent++;
        result += pad();
        break;
      case '}':
      case ']':
        indent--;
        result += '\n' + pad() + token;
        if (nextToken && nextToken !== ',' && nextToken !== '}' && nextToken !== ']') {
          // 这种情况不应该发生，因为逗号是独立 token
        }
        break;
      case ':':
        result += ': ';
        break;
      case ',':
        result += ',\n' + pad();
        break;
      default:
        result += token;
        break;
    }
  }

  return result;
}
