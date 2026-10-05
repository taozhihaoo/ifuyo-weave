//! Golden 测试（M4 下 §178–§179）：input + case.json → 输出必须等于
//! expected.txt（基线由 `cargo run -p weave-text --example write_golden`
//! 生成，**每次重生成必须人工审阅 diff**）。
//!
//! fixture 一律按 LF 语义读取：换行风格是 VCS/检出的噪音（复用的 CI
//! 工作区可能残留 CRLF），不属于格式化器契约，比较前归一化。

use std::path::{Path, PathBuf};

use weave_text::format::{FormatOperation, FormatOptions, run_formatter};
use weave_text::model::TextFormat;

fn fixture_root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/text")).to_path_buf()
}

fn read_lf(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

fn parse_format(name: &str) -> TextFormat {
    match name {
        "json" => TextFormat::Json,
        "xml" => TextFormat::Xml,
        "yaml" => TextFormat::Yaml,
        "sql" => TextFormat::Sql,
        "javascript" => TextFormat::JavaScript,
        "css" => TextFormat::Css,
        "markdown" => TextFormat::Markdown,
        other => panic!("unknown format {other}"),
    }
}

fn parse_operation(name: &str) -> FormatOperation {
    match name {
        "validate" => FormatOperation::Validate,
        "format" => FormatOperation::Format,
        "minify" => FormatOperation::Minify,
        "sort" => FormatOperation::Sort,
        "normalize" => FormatOperation::Normalize,
        other => panic!("unknown operation {other}"),
    }
}

#[test]
fn golden_outputs_match_reviewed_baselines() {
    let root = fixture_root();
    let mut checked = 0usize;
    let mut names: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&root).expect("fixture root") {
        names.push(
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned(),
        );
    }
    names.sort();
    for name in names {
        let dir = root.join(&name);
        if !dir.is_dir() {
            continue;
        }
        let input = read_lf(&dir.join("input.txt"));
        let expected = read_lf(&dir.join("expected.txt"));
        let case: serde_json::Value =
            serde_json::from_str(&read_lf(&dir.join("case.json"))).expect("case");
        let outcome = run_formatter(
            parse_format(case["format"].as_str().expect("format")),
            parse_operation(case["operation"].as_str().expect("operation")),
            &input,
            &FormatOptions {
                indent_spaces: case["indent_spaces"].as_u64().unwrap_or(2) as u32,
                final_newline: case["final_newline"].as_bool().unwrap_or(true),
            },
        );
        assert!(
            !outcome.is_error(),
            "{name}: unexpected error diagnostics {:?}",
            outcome.diagnostics
        );
        let actual = outcome.content.unwrap_or_default();
        assert_eq!(
            actual, expected,
            "{name}: golden mismatch — 若为有意变更，需重跑 write_golden 并人工审阅 diff（§179）"
        );
        checked += 1;
    }
    assert!(
        checked >= 7,
        "golden fixtures missing: only {checked} found"
    );
}
