//! YAML Formatter（M4 §31–§33）：yaml-rust2 真解析（非"行缩进解析器"）。
//!
//! 能力矩阵（D43，诚实边界）：
//! - Validate：真解析；ScanError 带 marker（line/column）。
//! - Sort：解析 → 递归 mapping 键排序 → 重新序列化。**重新序列化会丢失
//!   注释与锚点/别名展开为值** —— 以 Warning 诊断如实告知（§33 要求记录
//!   parser 边界），输出是规范化 YAML 而非原文修订。
//! - Format / Minify / Normalize：Unsupported（当前 parser 的 emitter 丢注释，
//!   §31/§46：做不到不破坏内容就明说）。

use super::{FormatOperation, FormatOptions, FormatOutcome};
use crate::diagnostics::{Severity, TextDiagnostic};
use yaml_rust2::Yaml;

pub fn run(operation: FormatOperation, content: &str, _options: &FormatOptions) -> FormatOutcome {
    match operation {
        FormatOperation::Validate => validate(content),
        FormatOperation::Sort => sort(content),
        _ => FormatOutcome::unsupported(
            "YAML re-serialization drops comments/anchors with the current parser (M4 §31/§46); only Validate and Sort are provided",
        ),
    }
}

fn parse(content: &str) -> Result<Vec<Yaml>, FormatOutcome> {
    match yaml_rust2::YamlLoader::load_from_str(content) {
        Ok(docs) => Ok(docs),
        Err(e) => {
            let marker = e.marker();
            Err(FormatOutcome::failed(vec![TextDiagnostic {
                code: "text.yamlInvalid".to_string(),
                severity: Severity::Error,
                message: e.to_string(),
                range: None,
                line: Some(marker.line() as u64),
                // YAML marker 列从 1 起（不同 parser 惯例——如实转换）
                column: Some(marker.col() as u64 + 1),
            }]))
        }
    }
}

fn validate(content: &str) -> FormatOutcome {
    match parse(content) {
        Ok(_) => FormatOutcome::success(None, false),
        Err(outcome) => outcome,
    }
}

fn sort(content: &str) -> FormatOutcome {
    let docs = match parse(content) {
        Ok(d) => d,
        Err(outcome) => return outcome,
    };
    let sorted: Vec<Yaml> = docs.into_iter().map(sort_yaml).collect();
    let mut out = String::new();
    let mut first = true;
    for doc in sorted {
        if !first {
            out.push_str("---\n");
        }
        first = false;
        let mut emitter = yaml_rust2::YamlEmitter::new(&mut out);
        if let Err(e) = emitter.dump(&doc) {
            return FormatOutcome::failed(vec![TextDiagnostic::without_range(
                "text.yamlEmitFailed",
                Severity::Error,
                e.to_string(),
            )]);
        }
        out.push('\n');
    }
    FormatOutcome {
        diagnostics: vec![TextDiagnostic::without_range(
            "text.yamlReserializeWarning",
            Severity::Warning,
            "YAML Sort re-serializes the document: comments are dropped and anchors/aliases are expanded (documented parser boundary, D43)",
        )],
        content: Some(out),
        changed: true,
    }
}

fn sort_yaml(y: Yaml) -> Yaml {
    match y {
        Yaml::Hash(hash) => {
            let mut entries: Vec<(Yaml, Yaml)> = hash.into_iter().collect();
            entries.sort_by_key(|(k, _)| key_text(k));
            let mut out = yaml_rust2::yaml::Hash::new();
            for (k, v) in entries {
                out.insert(k, sort_yaml(v));
            }
            Yaml::Hash(out)
        }
        Yaml::Array(items) => Yaml::Array(items.into_iter().map(sort_yaml).collect()),
        other => other,
    }
}

fn key_text(y: &Yaml) -> String {
    match y {
        Yaml::String(s) => s.clone(),
        Yaml::Integer(i) => i.to_string(),
        Yaml::Real(r) => r.clone(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_reports_marker_line_col() {
        let bad = "a: 1\nb: [unclosed\n";
        let out = run(FormatOperation::Validate, bad, &FormatOptions::default());
        assert!(out.is_error());
        let d = &out.diagnostics[0];
        assert_eq!(d.code, "text.yamlInvalid");
        assert!(d.line.is_some() && d.column.is_some());
    }

    #[test]
    fn sort_orders_mapping_keys_recursively_never_sequences() {
        let src = "z: 1\na:\n  y: 2\n  b: [3, 1, 2]\n";
        let out = run(FormatOperation::Sort, src, &FormatOptions::default());
        let text = out.content.expect("content");
        assert!(text.find("a:").expect("a first") < text.find("z:").expect("z second"));
        // 递归：内层 mapping 也排序（emitter 会给键加引号："y"）
        assert!(text.find("b:").expect("b") < text.find("\"y\":").expect("y"));
        // 序列保序（emitter 用 block 风格）
        let three = text.find("- 3").expect("3");
        let one = text.find("- 1").expect("1");
        let two = text.find("- 2").expect("2");
        assert!(three < one && one < two);
        // §33 边界：重序列化丢注释 ⇒ Warning 如实告知
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code == "text.yamlReserializeWarning")
        );
    }

    #[test]
    fn format_is_unsupported_with_reason() {
        let out = run(FormatOperation::Format, "a: 1\n", &FormatOptions::default());
        assert!(out.content.is_none());
        assert_eq!(out.diagnostics[0].code, "text.unsupportedOperation");
    }
}
