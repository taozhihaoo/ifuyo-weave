//! 格式检测（M4 §21）：bounded / deterministic / conservative。
//!
//! 扩展名映射优先；无扩展名或未知扩展名时做有界内容嗅探（≤512 字符），
//! 只在高置信标记上宣称格式，其余如实 Unknown（UI 强制手动选择）。
//! 禁止激进猜测：`.foo` 永不被强行猜成某种语言。

use crate::model::TextFormat;

const SNIFF_LIMIT: usize = 512;

/// 按扩展名映射（大小写不敏感）。
pub fn detect_format_by_extension(extension: Option<&str>) -> Option<TextFormat> {
    let ext = extension?.to_ascii_lowercase();
    let format = match ext.as_str() {
        "json" | "jsonc" | "json5" => TextFormat::Json, // 后两者按 JSON 家族保守处理（严格度由 parser 决定）
        "xml" | "svg" | "xsl" | "plist" => TextFormat::Xml,
        "yaml" | "yml" => TextFormat::Yaml,
        "sql" => TextFormat::Sql,
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" => TextFormat::JavaScript,
        "css" | "scss" | "less" => TextFormat::Css,
        "md" | "markdown" => TextFormat::Markdown,
        _ => return None,
    };
    Some(format)
}

/// 有界内容嗅探（仅高置信标记）。
fn sniff(content: &str) -> Option<TextFormat> {
    let head: String = content.chars().take(SNIFF_LIMIT).collect();
    let trimmed = head.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("<?xml") {
        return Some(TextFormat::Xml);
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        // JSON vs JS/CSS 的歧义：`{` 也可能 JS/CSS——只承认能整体被
        // serde_json 成功解析的样本；解析失败 ⇒ Unknown（不猜）。
        let t = trimmed.trim_end();
        let bracketed = (t.ends_with('}') && trimmed.starts_with('{'))
            || (t.ends_with(']') && trimmed.starts_with('['));
        if bracketed && serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            return Some(TextFormat::Json);
        }
        return None;
    }
    // 首个非空行
    let first_line = trimmed.lines().next().unwrap_or("").trim();
    if first_line == "---" {
        return Some(TextFormat::Yaml);
    }
    // Markdown：ATX 标题 / setext 下的加粗强调等弱标记不认，只认 # 开头标题
    if first_line.starts_with("# ") {
        return Some(TextFormat::Markdown);
    }
    // SQL：常见起始关键字（整词、大小写不敏感）
    let upper = first_line.to_ascii_uppercase();
    for kw in [
        "SELECT ", "WITH ", "INSERT ", "UPDATE ", "DELETE ", "CREATE ", "ALTER ", "DROP ",
    ] {
        if upper.starts_with(kw) {
            return Some(TextFormat::Sql);
        }
    }
    None
}

/// 组合检测：扩展名优先，其次有界嗅探；两者皆无 ⇒ Unknown（§21 保守）。
pub fn detect_format(extension: Option<&str>, content: &str) -> TextFormat {
    detect_format_by_extension(extension)
        .unwrap_or_else(|| sniff(content).unwrap_or(TextFormat::Unknown))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_mapping_is_case_insensitive() {
        assert_eq!(
            detect_format_by_extension(Some("JSON")),
            Some(TextFormat::Json)
        );
        assert_eq!(
            detect_format_by_extension(Some("Md")),
            Some(TextFormat::Markdown)
        );
        assert_eq!(detect_format_by_extension(Some("foo")), None);
        assert_eq!(detect_format_by_extension(None), None);
    }

    #[test]
    fn sniff_is_conservative() {
        assert_eq!(
            detect_format(None, "<?xml version=\"1.0\"?><a/>"),
            TextFormat::Xml
        );
        assert_eq!(detect_format(None, "{\n  \"a\": 1\n}"), TextFormat::Json);
        assert_eq!(detect_format(None, "[1, 2, 3]"), TextFormat::Json);
        assert_eq!(detect_format(None, "---\na: 1\nb: 2"), TextFormat::Yaml);
        assert_eq!(detect_format(None, "# Title\n\ntext"), TextFormat::Markdown);
        assert_eq!(detect_format(None, "SELECT * FROM t"), TextFormat::Sql);
        // 低置信：JS 对象字面量 vs JSON 失败 ⇒ Unknown，不猜
        assert_eq!(detect_format(None, "{ a: 1 }"), TextFormat::Unknown);
        // CSS 歧义高：不嗅探
        assert_eq!(
            detect_format(None, "body { color: red; }"),
            TextFormat::Unknown
        );
    }

    #[test]
    fn extension_wins_over_content() {
        assert_eq!(detect_format(Some("md"), "SELECT 1"), TextFormat::Markdown);
    }
}
