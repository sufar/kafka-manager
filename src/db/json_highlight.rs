/// JSON 高亮模板数据模块

use sqlx::{SqlitePool, FromRow};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// JSON 高亮模板结构
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JsonHighlightTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    /// 是否是内置模板
    pub is_builtin: bool,
    /// 模板样式配置（JSON 格式存储）
    pub style_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 模板样式配置结构（用于前端解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStyle {
    /// 浅色主题样式
    pub light: ThemeStyles,
    /// 深色主题样式
    pub dark: ThemeStyles,
}

/// 单个主题的样式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeStyles {
    /// 键名样式
    pub key: StyleConfig,
    /// 字符串值样式
    pub string: StyleConfig,
    /// 数字样式
    pub number: StyleConfig,
    /// 布尔值样式
    pub boolean: StyleConfig,
    /// null 样式
    pub null: StyleConfig,
    /// 括号样式
    pub bracket: StyleConfig,
    /// 冒号样式
    pub colon: StyleConfig,
    /// 逗号样式
    pub comma: StyleConfig,
}

/// 单个样式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    /// 字体颜色（十六进制）
    pub color: String,
    /// 字体粗细
    #[serde(default)]
    pub font_weight: Option<String>,
    /// 字体样式
    #[serde(default)]
    pub font_style: Option<String>,
}

impl JsonHighlightTemplate {
    /// 验证模板样式 JSON 是否包含所有必需字段
    pub fn validate_style_json(style_json: &str) -> Result<(), String> {
        // 首先尝试解析为 TemplateStyle 结构
        let style: TemplateStyle = serde_json::from_str(style_json)
            .map_err(|e| format!("无效的 JSON 格式：{}", e))?;

        // 验证浅色主题
        Self::validate_theme_styles(&style.light, "light")?;
        // 验证深色主题
        Self::validate_theme_styles(&style.dark, "dark")?;

        Ok(())
    }

    /// 验证单个主题的样式配置
    fn validate_theme_styles(theme: &ThemeStyles, theme_name: &str) -> Result<(), String> {
        // 验证所有必需字段
        Self::validate_style_config(&theme.key, &format!("{}.key", theme_name))?;
        Self::validate_style_config(&theme.string, &format!("{}.string", theme_name))?;
        Self::validate_style_config(&theme.number, &format!("{}.number", theme_name))?;
        Self::validate_style_config(&theme.boolean, &format!("{}.boolean", theme_name))?;
        Self::validate_style_config(&theme.null, &format!("{}.null", theme_name))?;
        Self::validate_style_config(&theme.bracket, &format!("{}.bracket", theme_name))?;
        Self::validate_style_config(&theme.colon, &format!("{}.colon", theme_name))?;
        Self::validate_style_config(&theme.comma, &format!("{}.comma", theme_name))?;

        Ok(())
    }

    /// 验证单个样式配置
    fn validate_style_config(config: &StyleConfig, field_path: &str) -> Result<(), String> {
        // 验证颜色字段
        if config.color.is_empty() {
            return Err(format!("字段 '{}' 的 'color' 不能为空", field_path));
        }
        // 验证颜色格式（简单的十六进制格式检查）
        if !config.color.starts_with('#') || config.color.len() != 7 {
            return Err(format!("字段 '{}' 的颜色 '{}' 必须是 6 位十六进制格式 (如 #RRGGBB)", field_path, config.color));
        }

        Ok(())
    }
    /// 获取所有模板（包括内置和自定义）
    pub async fn get_all_templates(pool: &SqlitePool) -> Result<Vec<JsonHighlightTemplate>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, description, is_builtin, style_json, created_at, updated_at
             FROM json_highlight_templates
             ORDER BY is_builtin DESC, name"
        )
        .fetch_all(pool)
        .await
    }

    /// 获取内置模板列表
    pub async fn get_builtin_templates(pool: &SqlitePool) -> Result<Vec<JsonHighlightTemplate>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, description, is_builtin, style_json, created_at, updated_at
             FROM json_highlight_templates
             WHERE is_builtin = 1
             ORDER BY name"
        )
        .fetch_all(pool)
        .await
    }

    /// 获取自定义模板列表
    pub async fn get_custom_templates(pool: &SqlitePool) -> Result<Vec<JsonHighlightTemplate>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, description, is_builtin, style_json, created_at, updated_at
             FROM json_highlight_templates
             WHERE is_builtin = 0
             ORDER BY name"
        )
        .fetch_all(pool)
        .await
    }

    /// 根据 ID 获取模板
    pub async fn get_template_by_id(pool: &SqlitePool, id: i64) -> Result<Option<JsonHighlightTemplate>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, description, is_builtin, style_json, created_at, updated_at
             FROM json_highlight_templates
             WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// 根据名称获取模板
    pub async fn get_template_by_name(pool: &SqlitePool, name: &str) -> Result<Option<JsonHighlightTemplate>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, name, description, is_builtin, style_json, created_at, updated_at
             FROM json_highlight_templates
             WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(pool)
        .await
    }

    /// 保存模板（新建或更新）
    pub async fn save_template(
        pool: &SqlitePool,
        name: &str,
        description: &str,
        is_builtin: bool,
        style_json: &str,
    ) -> Result<i64, sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        // 检查是否已存在同名模板
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM json_highlight_templates WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;

        if let Some((id,)) = existing {
            // 更新现有模板
            sqlx::query(
                r#"
                UPDATE json_highlight_templates
                SET description = ?, is_builtin = ?, style_json = ?, updated_at = ?
                WHERE id = ?
                "#
            )
            .bind(description)
            .bind(is_builtin)
            .bind(style_json)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;
            Ok(id)
        } else {
            // 创建新模板
            let result = sqlx::query(
                r#"
                INSERT INTO json_highlight_templates (name, description, is_builtin, style_json, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(name)
            .bind(description)
            .bind(is_builtin)
            .bind(style_json)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
            Ok(result.last_insert_rowid())
        }
    }

    /// 删除自定义模板（不能删除内置模板）
    pub async fn delete_template(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
        // 先检查是否是内置模板
        let is_builtin: Option<(bool,)> = sqlx::query_as(
            "SELECT is_builtin FROM json_highlight_templates WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if let Some((builtin,)) = is_builtin {
            if builtin {
                return Ok(false); // 不能删除内置模板
            }

            sqlx::query("DELETE FROM json_highlight_templates WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
            Ok(true)
        } else {
            Ok(false) // 模板不存在
        }
    }

    /// 初始化内置模板
    pub async fn init_builtin_templates(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let templates = get_builtin_templates_sql();

        for (name, description, style_json) in templates {
            let _ = Self::save_template(pool, name, description, true, style_json).await;
        }

        Ok(())
    }
}

/// 获取内置模板 SQL
fn get_builtin_templates_sql() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // 浅色默认主题 - 经典 Web 风格
        (
            "light_default",
            "浅色默认主题",
            r##"{
  "light": {
    "key": { "color": "#881391", "font_weight": "normal" },
    "string": { "color": "#c41a16" },
    "number": { "color": "#1c00cf" },
    "boolean": { "color": "#0d22aa", "font_weight": "bold" },
    "null": { "color": "#808080" },
    "bracket": { "color": "#24292e" },
    "colon": { "color": "#24292e" },
    "comma": { "color": "#24292e" }
  },
  "dark": {
    "key": { "color": "#c792ea", "font_weight": "normal" },
    "string": { "color": "#ecc486" },
    "number": { "color": "#f78c6c" },
    "boolean": { "color": "#ff5370", "font_weight": "bold" },
    "null": { "color": "#89ddff" },
    "bracket": { "color": "#eeffff" },
    "colon": { "color": "#eeffff" },
    "comma": { "color": "#eeffff" }
  }
}"##
        ),
        // 深色默认主题 - One Dark 风格
        (
            "dark_default",
            "深色默认主题",
            r##"{
  "light": {
    "key": { "color": "#881391", "font_weight": "normal" },
    "string": { "color": "#c41a16" },
    "number": { "color": "#1c00cf" },
    "boolean": { "color": "#0d22aa", "font_weight": "bold" },
    "null": { "color": "#808080" },
    "bracket": { "color": "#24292e" },
    "colon": { "color": "#24292e" },
    "comma": { "color": "#24292e" }
  },
  "dark": {
    "key": { "color": "#c792ea", "font_weight": "normal" },
    "string": { "color": "#ecc486" },
    "number": { "color": "#f78c6c" },
    "boolean": { "color": "#ff5370", "font_weight": "bold" },
    "null": { "color": "#89ddff" },
    "bracket": { "color": "#eeffff" },
    "colon": { "color": "#eeffff" },
    "comma": { "color": "#eeffff" }
  }
}"##
        ),
        // 默认主题（Default）- 高对比度现代风格，双主题优化
        (
            "default",
            "默认主题（高对比度，双主题优化）",
            r##"{
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
}"##
        ),
        // 深色主题（Monokai）- 经典深色主题，浅色主题已优化
        (
            "monokai",
            "经典深色主题，适合夜间编码",
            r##"{
  "light": {
    "key": { "color": "#c14fac", "font_weight": "normal" },
    "string": { "color": "#3a9c48" },
    "number": { "color": "#a36de6" },
    "boolean": { "color": "#a36de6", "font_weight": "bold" },
    "null": { "color": "#a36de6" },
    "bracket": { "color": "#475569" },
    "colon": { "color": "#475569" },
    "comma": { "color": "#475569" }
  },
  "dark": {
    "key": { "color": "#f92672", "font_weight": "normal" },
    "string": { "color": "#e6db74" },
    "number": { "color": "#ae81ff" },
    "boolean": { "color": "#ae81ff", "font_weight": "bold" },
    "null": { "color": "#ae81ff" },
    "bracket": { "color": "#f8f8f2" },
    "colon": { "color": "#f8f8f2" },
    "comma": { "color": "#f8f8f2" }
  }
}"##
        ),
        // GitHub 主题
        (
            "github",
            "GitHub 代码风格主题",
            r##"{
  "light": {
    "key": { "color": "#8250df", "font_weight": "normal" },
    "string": { "color": "#0a3070" },
    "number": { "color": "#0550ae" },
    "boolean": { "color": "#0550ae", "font_weight": "bold" },
    "null": { "color": "#0550ae" },
    "bracket": { "color": "#475569" },
    "colon": { "color": "#475569" },
    "comma": { "color": "#475569" }
  },
  "dark": {
    "key": { "color": "#b392f0", "font_weight": "normal" },
    "string": { "color": "#9ecbff" },
    "number": { "color": "#79b8ff" },
    "boolean": { "color": "#79b8ff", "font_weight": "bold" },
    "null": { "color": "#79b8ff" },
    "bracket": { "color": "#e1e4e8" },
    "colon": { "color": "#e1e4e8" },
    "comma": { "color": "#e1e4e8" }
  }
}"##
        ),
        // One Dark 主题 - Atom One Dark 风格，浅色主题已优化
        (
            "one_dark",
            "Atom One Dark 风格主题",
            r##"{
  "light": {
    "key": { "color": "#e06c75" },
    "string": { "color": "#98c379" },
    "number": { "color": "#d19a66" },
    "boolean": { "color": "#56b6c2", "font_weight": "bold" },
    "null": { "color": "#56b6c2" },
    "bracket": { "color": "#475569" },
    "colon": { "color": "#475569" },
    "comma": { "color": "#475569" }
  },
  "dark": {
    "key": { "color": "#e06c75" },
    "string": { "color": "#98c379" },
    "number": { "color": "#d19a66" },
    "boolean": { "color": "#56b6c2", "font_weight": "bold" },
    "null": { "color": "#56b6c2" },
    "bracket": { "color": "#abb2bf" },
    "colon": { "color": "#abb2bf" },
    "comma": { "color": "#abb2bf" }
  }
}"##
        ),
        // Dracula 主题 - 深色主题，浅色主题已优化
        (
            "dracula",
            "Dracula 深色主题",
            r##"{
  "light": {
    "key": { "color": "#d946a5" },
    "string": { "color": "#6b8f06" },
    "number": { "color": "#9063cd" },
    "boolean": { "color": "#9063cd", "font_weight": "bold" },
    "null": { "color": "#9063cd" },
    "bracket": { "color": "#475569" },
    "colon": { "color": "#475569" },
    "comma": { "color": "#475569" }
  },
  "dark": {
    "key": { "color": "#ff79c6" },
    "string": { "color": "#f1fa8c" },
    "number": { "color": "#bd93f9" },
    "boolean": { "color": "#bd93f9", "font_weight": "bold" },
    "null": { "color": "#bd93f9" },
    "bracket": { "color": "#f8f8f2" },
    "colon": { "color": "#f8f8f2" },
    "comma": { "color": "#f8f8f2" }
  }
}"##
        ),
        // Nord 主题 - 冷静蓝主题，浅色主题已优化
        (
            "nord",
            "Nord 冷静蓝主题",
            r##"{
  "light": {
    "key": { "color": "#5e81ac" },
    "string": { "color": "#a3be8c" },
    "number": { "color": "#b48ead" },
    "boolean": { "color": "#5e81ac", "font_weight": "bold" },
    "null": { "color": "#5e81ac" },
    "bracket": { "color": "#475569" },
    "colon": { "color": "#475569" },
    "comma": { "color": "#475569" }
  },
  "dark": {
    "key": { "color": "#81a1c1" },
    "string": { "color": "#a3be8c" },
    "number": { "color": "#b48ead" },
    "boolean": { "color": "#81a1c1", "font_weight": "bold" },
    "null": { "color": "#81a1c1" },
    "bracket": { "color": "#d8dee9" },
    "colon": { "color": "#d8dee9" },
    "comma": { "color": "#d8dee9" }
  }
}"##
        ),
    ]
}
