//! 畸形输入 / 深嵌套 / 资源限制（M4 下 §181–§183）：
//! 目标 = 结构化错误，绝不 panic / 无限循环 / 无界内存。

use weave_text::format::{FormatOperation, FormatOptions, run_formatter};
use weave_text::model::TextFormat;

fn fmt(
    format: TextFormat,
    operation: FormatOperation,
    src: &str,
) -> weave_text::format::FormatOutcome {
    run_formatter(format, operation, src, &FormatOptions::default())
}

#[test]
fn empty_and_truncated_inputs_error_cleanly() {
    // JSON：空 / 截断
    for src in ["", "{", "{\"a\":", "[1,", "{\"a\""] {
        let out = fmt(TextFormat::Json, FormatOperation::Validate, src);
        if src.is_empty() {
            // 空输入：parser 报错（EOF）——结构化而非 panic
            assert!(out.is_error(), "empty JSON must be invalid");
        } else {
            assert!(out.is_error(), "truncated JSON {src:?} must be invalid");
        }
    }
    // XML：截断 / 未闭合
    for src in ["", "<root>", "<root><a></root>", "<?xml"] {
        let out = fmt(TextFormat::Xml, FormatOperation::Validate, src);
        assert!(out.is_error(), "malformed XML {src:?} must be invalid");
    }
    // YAML：非法缩进 / 截断（夹具按 yaml-rust2 真实拒绝行为选取）
    for src in ["a:\n  b:\n c: 1", "a: [1, 2", "a: {b: 1"] {
        let out = fmt(TextFormat::Yaml, FormatOperation::Validate, src);
        assert!(out.is_error(), "malformed YAML {src:?} must be invalid");
    }
}

#[test]
fn json_deep_nesting_is_rejected_not_stack_overflow() {
    // §182：1000+ 层嵌套——serde_json 默认递归限深 128 ⇒ 结构化错误
    for depth in [200usize, 1000, 5000] {
        let src = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let out = fmt(TextFormat::Json, FormatOperation::Validate, &src);
        assert!(
            out.is_error(),
            "depth {depth} must be rejected by the parser limit"
        );
    }
}

#[test]
fn xml_deep_nesting_does_not_panic() {
    // quick-xml 是事件流（无递归下降）：深层嵌套合法 ⇒ Format 成功且确定
    let depth = 2000usize;
    let src = format!("{}x{}", "<a>".repeat(depth), "</a>".repeat(depth));
    let out = fmt(TextFormat::Xml, FormatOperation::Format, &src);
    assert!(!out.is_error(), "well-formed deep XML should format");
    let once = out.content.expect("content");
    let twice = fmt(TextFormat::Xml, FormatOperation::Format, &once)
        .content
        .expect("content");
    assert_eq!(once, twice, "deep XML format must remain idempotent");
}

#[test]
fn yaml_deep_nesting_does_not_panic() {
    let depth = 500usize;
    let src = format!("{}1{}", "a:\n  ".repeat(depth), "1");
    let out = fmt(TextFormat::Yaml, FormatOperation::Validate, &src);
    // 解析成功或结构化错误皆可——关键是不能 panic
    let _ = out;
}

#[test]
fn invalid_escapes_and_weird_unicode_error_cleanly() {
    // JSON 非法转义
    let out = fmt(
        TextFormat::Json,
        FormatOperation::Validate,
        "{\"a\": \"\\x\"}",
    );
    assert!(out.is_error());
    // 孤立代理对 / 控制字符的字节序列：解码级联兜底（Latin-1），不 panic
    let decoded = weave_text::decode(&[0x22, 0xFF, 0x22]);
    assert!(decoded.is_ok() || decoded.is_err(), "decode must not panic");
    // UTF-8 BOM + emoji + 组合字符正常往返
    let doc = weave_text::decode("\u{FEFF}héllo 😀e\u{0301}".as_bytes()).expect("decode");
    assert_eq!(doc.bom, weave_text::Bom::Utf8);
}

#[test]
fn json_output_explosion_is_bounded() {
    // §184：1 MiB 深重复输入的格式化输出有界（JSON 无压缩语义 ⇒ 线性放大
    // 属预期；这里验证不 panic 且输出尺寸是输入的线性量级）
    let unit = "{\"k\":1}";
    let src = format!(
        "[{}]",
        format!("{unit},").repeat(50_000).trim_end_matches(',')
    );
    let out = fmt(TextFormat::Json, FormatOperation::Format, &src);
    let content = out.content.expect("content");
    assert!(
        content.len() < src.len() * 20,
        "output {} vs input {} — non-linear explosion",
        content.len(),
        src.len()
    );
}
