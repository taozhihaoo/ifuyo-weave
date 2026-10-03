//! Golden 测试基线生成（M4 下 §178–§179）。
//!
//! 运行：`cargo run -p weave-text --example write_golden`
//! 为每个 fixture 写出 expected.txt。**§179 纪律：更新 formatter 后重跑此
//! 生成器时必须逐文件审阅 git diff，禁止盲目重生成后直接宣布测试通过。**

use std::path::Path;

use weave_text::format::{FormatOperation, FormatOptions, run_formatter};
use weave_text::model::TextFormat;

fn parse_format(name: &str) -> TextFormat {
    match name {
        "json" => TextFormat::Json,
        "xml" => TextFormat::Xml,
        "yaml" => TextFormat::Yaml,
        "sql" => TextFormat::Sql,
        "javascript" => TextFormat::JavaScript,
        "css" => TextFormat::Css,
        "markdown" => TextFormat::Markdown,
        _ => panic!("unknown format {name}"),
    }
}

fn parse_operation(name: &str) -> FormatOperation {
    match name {
        "validate" => FormatOperation::Validate,
        "format" => FormatOperation::Format,
        "minify" => FormatOperation::Minify,
        "sort" => FormatOperation::Sort,
        "normalize" => FormatOperation::Normalize,
        _ => panic!("unknown operation {name}"),
    }
}

fn main() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/text"));
    let mut count = 0usize;
    for entry in std::fs::read_dir(root).expect("fixture root") {
        let dir = entry.expect("entry").path();
        if !dir.is_dir() {
            continue;
        }
        let input = std::fs::read_to_string(dir.join("input.txt")).expect("input");
        let case: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).expect("case"))
                .expect("case json");
        let format = parse_format(case["format"].as_str().expect("format"));
        let operation = parse_operation(case["operation"].as_str().expect("operation"));
        let options = FormatOptions {
            indent_spaces: case["indent_spaces"].as_u64().unwrap_or(2) as u32,
            final_newline: case["final_newline"].as_bool().unwrap_or(true),
        };
        let outcome = run_formatter(format, operation, &input, &options);
        assert!(
            !outcome.is_error(),
            "{:?} produced error diagnostics: {:?}",
            dir,
            outcome.diagnostics
        );
        let output = outcome.content.expect("golden case must produce content");
        std::fs::write(dir.join("expected.txt"), output).expect("write expected");
        count += 1;
    }
    println!("wrote {count} golden baselines");
}
