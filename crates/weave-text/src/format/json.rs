//! JSON Formatter（M4 §24–§28）：serde_json 真解析（preserve_order 保持键序）。
//!
//! 能力矩阵（D43）：
//! - Validate：serde_json 解析；错误带 line/column（parser 提供的）。
//! - Format：parse → pretty（键序保持，仅空白变化；浮点表示由 parser 归一）。
//! - Minify：parse → compact（不改任何值/键/数组顺序，§26）。
//! - Sort：递归对象键排序（§27：**绝不排序数组**——数组有序语义）。
//! - Normalize：Sort + Format 的规范序列化（§28 明确定义，非模糊"Normalize"）。

use super::{FormatOperation, FormatOptions, FormatOutcome};
use crate::diagnostics::{Severity, TextDiagnostic};
use crate::offset::TextRange;
use serde_json::Value;

pub fn run(operation: FormatOperation, content: &str, options: &FormatOptions) -> FormatOutcome {
    let parsed = serde_json::from_str::<Value>(content);
    let value = match parsed {
        Ok(v) => v,
        Err(e) => {
            let (line, column) = match (e.line(), e.column()) {
                (l, c) if l > 0 => (l as u64, c as u64),
                _ => (1, 1),
            };
            // serde_json 只给 line/col；byte range 由行号反算（有界）
            return FormatOutcome::failed(vec![TextDiagnostic {
                code: "text.jsonInvalid".to_string(),
                severity: Severity::Error,
                message: e.to_string(),
                range: None,
                line: Some(line),
                column: Some(column),
            }]);
        }
    };

    match operation {
        FormatOperation::Validate => {
            let _ = (value, options);
            FormatOutcome::success(None, false)
        }
        FormatOperation::Format => {
            let pretty = render(&value, options.indent_spaces, options.final_newline);
            let changed = pretty != content;
            FormatOutcome::success(Some(pretty), changed)
        }
        FormatOperation::Minify => {
            let compact = value.to_string();
            let compact = finish(&compact, options);
            let changed = compact != content;
            FormatOutcome::success(Some(compact), changed)
        }
        FormatOperation::Sort => {
            let sorted = sort_value(value);
            let pretty = render(&sorted, options.indent_spaces, options.final_newline);
            let changed = pretty != content;
            FormatOutcome::success(Some(pretty), changed)
        }
        FormatOperation::Normalize => {
            let sorted = sort_value(value);
            let pretty = render(&sorted, options.indent_spaces, options.final_newline);
            let changed = pretty != content;
            FormatOutcome::success(Some(pretty), changed)
        }
    }
}

/// 递归键排序（§27：对象递归；数组保序不排序）。
fn sort_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<(String, Value)> = map.into_iter().collect();
            keys.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = serde_json::Map::new();
            for (k, v) in keys {
                out.insert(k, sort_value(v));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sort_value).collect()),
        other => other,
    }
}

fn render(value: &Value, indent: u32, final_newline: bool) -> String {
    // serde_json to_string_pretty 固定 2 空格；其它缩进经二次替换（D43 v1）
    let mut s = if indent == 2 {
        serde_json::to_string_pretty(value).expect("serializable")
    } else {
        let pad = " ".repeat(indent as usize);
        reindent(
            &serde_json::to_string_pretty(value).expect("serializable"),
            &pad,
        )
    };
    if final_newline && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// 把 2 空格缩进重排为指定宽度（行首空白整段替换，字符串内部不受影响）。
fn reindent(pretty: &str, pad: &str) -> String {
    let mut out = String::with_capacity(pretty.len());
    for line in pretty.split('\n') {
        let spaces = line.len() - line.trim_start().len();
        if spaces > 0 {
            for _ in 0..spaces / 2 {
                out.push_str(pad);
            }
        }
        out.push_str(line.trim_start());
        out.push('\n');
    }
    out.pop();
    out
}

fn finish(s: &str, options: &FormatOptions) -> String {
    if options.final_newline && !s.ends_with('\n') {
        format!("{s}\n")
    } else {
        s.to_string()
    }
}

/// JSON 诊断可带 range：由 line/col 反算 byte offset（供 UI 高亮）。
pub fn range_of_line_col(content: &str, line: u64, column: u64) -> Option<TextRange> {
    let index = crate::offset::LineIndex::new(content);
    if line == 0 || line > index.line_count() {
        return None;
    }
    let r = index.line_range(content, line);
    // column 是 UTF-16 units；换算到 byte
    let line_text = &content[r.start as usize..r.end as usize];
    let mut units = 0u64;
    for (i, ch) in line_text.char_indices() {
        if units >= column.saturating_sub(1) {
            return Some(TextRange::new(
                r.start + i as u64,
                r.start + i as u64 + ch.len_utf8() as u64,
            ));
        }
        units += ch.len_utf16() as u64;
    }
    Some(TextRange::new(r.end, r.end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_reports_line_column_from_parser() {
        let bad = "{\n  \"a\": 1,\n}\n";
        let out = run(FormatOperation::Validate, bad, &FormatOptions::default());
        assert!(out.is_error());
        let d = &out.diagnostics[0];
        assert_eq!(d.code, "text.jsonInvalid");
        assert_eq!(d.line, Some(3));
    }

    #[test]
    fn format_preserves_key_order_and_only_whitespace_changes() {
        // §25/§26：键序保持（preserve_order），语义不变
        let src = "{\"b\":1,\"a\":[1,2]}";
        let out = run(FormatOperation::Format, src, &FormatOptions::default());
        let pretty = out.content.expect("content");
        assert!(pretty.contains("\"b\": 1"));
        assert!(pretty.find("\"b\"").expect("b first") < pretty.find("\"a\"").expect("a second"));
        // 再 parse 回来语义一致
        assert_eq!(
            serde_json::from_str::<Value>(src).unwrap(),
            serde_json::from_str::<Value>(&pretty).unwrap()
        );
    }

    #[test]
    fn minify_keeps_values_and_array_order() {
        // §26：字符串/数字/布尔/null/键名/数组顺序不动
        let src = "{\n  \"k\": \"a b\",\n  \"arr\": [3, 1, 2],\n  \"n\": null\n}";
        let out = run(FormatOperation::Minify, src, &FormatOptions::default());
        assert_eq!(
            out.content.expect("content"),
            "{\"k\":\"a b\",\"arr\":[3,1,2],\"n\":null}\n"
        );
    }

    #[test]
    fn sort_is_recursive_keys_only_never_arrays() {
        // §27：[3,1,2] 必须保持
        let src = "{\"b\":{\"d\":1,\"c\":2},\"a\":[3,1,2]}";
        let out = run(FormatOperation::Sort, src, &FormatOptions::default());
        let pretty = out.content.expect("content");
        assert!(pretty.find("\"a\"").expect("a first") < pretty.find("\"b\"").expect("b second"));
        // 内层递归排序：c 在 d 前
        assert!(pretty.find("\"c\"").expect("c") < pretty.find("\"d\"").expect("d"));
        // 数组保序（语义校验，而非字面排版）
        let sorted: Value = serde_json::from_str(&pretty).expect("valid");
        assert_eq!(sorted["a"], serde_json::json!([3, 1, 2]));
        assert_eq!(sorted["b"]["c"], serde_json::json!(2));
    }

    #[test]
    fn format_is_idempotent() {
        // 下 §127 前置：同输入同选项幂等
        let src = "{\"a\":1,\"b\":{\"c\":2}}";
        let once = run(FormatOperation::Format, src, &FormatOptions::default())
            .content
            .expect("once");
        let twice = run(FormatOperation::Format, &once, &FormatOptions::default())
            .content
            .expect("twice");
        assert_eq!(once, twice);
    }
}
