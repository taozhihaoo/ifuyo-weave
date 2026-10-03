//! 性质 / 不变量测试（M4 下 §126–§129）：fmt(run_formatter) 便捷别名下
//! 对代表性语料断言幂等、往返、确定性等性质。
//!
//! §126 纪律：不为测试强行定义错误的数学性质——只验证设计承诺的性质。

use weave_text::format::{FormatOperation, FormatOptions, run_formatter};
use weave_text::model::TextFormat;

fn fmt(format: TextFormat, operation: FormatOperation, src: &str) -> String {
    run_formatter(format, operation, src, &FormatOptions::default())
        .content
        .expect("content-producing operation")
}

/// 代表性语料（§177：确定性文本夹具，运行时构造，不提交大文件）。
const JSON_SAMPLES: [&str; 5] = [
    r#"{"b":1,"a":{"y":2,"x":[3,1,2]}}"#,
    r#"[true,false,null,"esc\"aped","中文😀"]"#,
    r#"{}"#,
    r#"[]"#,
    r#"{"n":-1.5e10,"z":0}"#,
];

const XML_SAMPLES: [&str; 4] = [
    r#"<root a="1"><item>x</item><empty/></root>"#,
    "<r><!-- c --><t><![CDATA[data < raw]]></t></r>",
    r#"<ns:doc xmlns:ns="urn:x"><ns:k v="&quot;q&quot;"/></ns:doc>"#,
    "<a><b/><c>text</c></a>",
];

const SQL_SAMPLES: [&str; 3] = [
    "SELECT id, name FROM users WHERE age > 18 ORDER BY name;",
    "INSERT INTO t (a, b) VALUES ('x, y', \"q\");",
    "UPDATE t SET v = 1 /* note */ WHERE id = 'it''s';",
];

const CSS_SAMPLES: [&str; 3] = [
    "body { color: red; } .x { margin: 0 }",
    "@media (min-width: 100px) { .a { color: blue } }",
    "a { background: url(img.png) }",
];

const MARKDOWN_SAMPLES: [&str; 3] = [
    "# T\ntext\n\n\n\n## S\ntail",
    "intro\n\n```\ncode\n  keep\n```\n\n# H\nx",
    "- a\n- b\n\n1. c\n",
];

#[test]
fn format_is_idempotent_for_canonical_formatters() {
    // §127：Format(Format(x)) == Format(x)
    for src in JSON_SAMPLES {
        let once = fmt(TextFormat::Json, FormatOperation::Format, src);
        let twice = fmt(TextFormat::Json, FormatOperation::Format, &once);
        assert_eq!(once, twice, "JSON idempotence broken for {src}");
    }
    for src in XML_SAMPLES {
        let once = fmt(TextFormat::Xml, FormatOperation::Format, src);
        let twice = fmt(TextFormat::Xml, FormatOperation::Format, &once);
        assert_eq!(once, twice, "XML idempotence broken for {src}");
    }
    for src in SQL_SAMPLES {
        let once = fmt(TextFormat::Sql, FormatOperation::Format, src);
        let twice = fmt(TextFormat::Sql, FormatOperation::Format, &once);
        assert_eq!(once, twice, "SQL idempotence broken for {src}");
    }
}

#[test]
fn normalize_is_idempotent() {
    // §128：Markdown Normalize 设计为 canonical ⇒ 幂等
    for src in MARKDOWN_SAMPLES {
        let once = fmt(TextFormat::Markdown, FormatOperation::Normalize, src);
        let twice = fmt(TextFormat::Markdown, FormatOperation::Normalize, &once);
        assert_eq!(once, twice, "MD normalize idempotence broken for {src}");
    }
}

#[test]
fn format_then_validate_always_valid() {
    // §126：Format 后 Validate 必须通过
    for src in JSON_SAMPLES {
        let formatted = fmt(TextFormat::Json, FormatOperation::Format, src);
        let outcome = run_formatter(
            TextFormat::Json,
            FormatOperation::Validate,
            &formatted,
            &FormatOptions::default(),
        );
        assert!(
            !outcome.is_error(),
            "formatted JSON failed validation: {formatted}"
        );
    }
    for src in XML_SAMPLES {
        let formatted = fmt(TextFormat::Xml, FormatOperation::Format, src);
        let outcome = run_formatter(
            TextFormat::Xml,
            FormatOperation::Validate,
            &formatted,
            &FormatOptions::default(),
        );
        assert!(
            !outcome.is_error(),
            "formatted XML failed validation: {formatted}"
        );
    }
}

#[test]
fn minify_roundtrip_is_semantically_equal() {
    // §129：Format → Minify → Parse 保持语义等价（键序不排序 ⇒ 直接相等）
    for src in JSON_SAMPLES {
        let formatted = fmt(TextFormat::Json, FormatOperation::Format, src);
        let minified = fmt(TextFormat::Json, FormatOperation::Minify, &formatted);
        let a: serde_json::Value = serde_json::from_str(src).expect("src valid");
        let b: serde_json::Value = serde_json::from_str(&minified).expect("minified valid");
        assert_eq!(a, b, "minify round-trip changed semantics for {src}");
    }
}

#[test]
fn css_minify_is_idempotent() {
    // Minify(Minify(x)) == Minify(x)（CSS 无 canonical Format ⇒ Minify 幂等）
    for src in CSS_SAMPLES {
        let once = fmt(TextFormat::Css, FormatOperation::Minify, src);
        let twice = fmt(TextFormat::Css, FormatOperation::Minify, &once);
        assert_eq!(once, twice, "CSS minify idempotence broken for {src}");
    }
}

#[test]
fn yaml_sort_is_idempotent() {
    // YAML Sort 输出为规范化 YAML ⇒ 再次 Sort 不变（§128 域内适用）
    let src = "z: 1\na:\n  y: 2\n  b: [3, 1, 2]\n";
    let once = fmt(TextFormat::Yaml, FormatOperation::Sort, src);
    let twice = fmt(TextFormat::Yaml, FormatOperation::Sort, &once);
    assert_eq!(once, twice);
}

#[test]
fn whitespace_and_case_ignore_properties() {
    // §126：纯空白差异在 Trailing 档不产生 diff；纯大小写差异在 Case Ignore 下消失
    use weave_text::{CompareLimits, CompareOptions, WhitespaceMode, compare_texts};
    let trailing = CompareOptions {
        whitespace: WhitespaceMode::Trailing,
        ignore_case: false,
    };
    let r =
        compare_texts("a \nb\n", "a\nb \n", &trailing, &CompareLimits::default()).expect("compare");
    assert!(r.identical);
    let ci = CompareOptions {
        whitespace: WhitespaceMode::None,
        ignore_case: true,
    };
    let r = compare_texts("ABC\n", "abc\n", &ci, &CompareLimits::default()).expect("compare");
    assert!(r.identical);
}

#[test]
fn dedupe_result_has_no_duplicates_and_sort_is_deterministic() {
    use weave_text::transform::{BlankPolicy, TransformKind, apply_transform};
    let src = "b\na\nb\na\nc\n";
    let out = apply_transform(
        src,
        &TransformKind::DeduplicateLines {
            keep: weave_text::KeepPolicy::First,
            blank: BlankPolicy::Included,
        },
    )
    .expect("dedupe");
    let lines: Vec<&str> = out.content.lines().collect();
    let mut unique = lines.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(lines.len(), unique.len(), "dedupe left duplicates: {out:?}");

    let kind = TransformKind::SortLines {
        descending: false,
        case_sensitive: true,
        blank: BlankPolicy::Included,
    };
    let s1 = apply_transform(src, &kind).expect("sort1");
    let s2 = apply_transform(src, &kind).expect("sort2");
    assert_eq!(s1, s2, "sort must be deterministic");
}

#[test]
fn prefix_add_is_reversible_by_trim() {
    // §126：可逆性质仅在数学适用处定义——prefix 的逆 = 移除该前缀
    use weave_text::transform::{TransformKind, apply_transform};
    let src = "one\ntwo\n";
    let added = apply_transform(
        src,
        &TransformKind::AddPrefix {
            text: "- ".into(),
            skip_blank: false,
        },
    )
    .expect("prefix");
    let removed = apply_transform(
        &added.content,
        &TransformKind::FindReplace {
            find: "(?m)^- ".into(),
            replacement: String::new(),
            regex: true,
            case_insensitive: false,
            first_only: false,
        },
    )
    .expect("strip");
    assert_eq!(removed.content, src);
}
