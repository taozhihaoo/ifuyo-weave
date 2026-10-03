//! Transformer 测试（M4 下 §125 前置；对应上 §72–§83 语义）。

use super::*;

fn run(content: &str, kind: &TransformKind) -> TransformResult {
    apply_transform(content, kind).expect("transform")
}

#[test]
fn trim_lines_vs_document_distinct() {
    // §72：line trim 保留换行；document trim 去掉首尾空白
    let r = run("  a  \n\t b \n", &TransformKind::TrimLines);
    assert_eq!(r.content, "a\nb\n");
    let r = run("  a\nb  \n", &TransformKind::TrimDocument);
    assert_eq!(r.content, "a\nb");
}

#[test]
fn dedupe_keep_first_preserves_order() {
    // §73：A B A C → A B C，绝不偷偷排序
    let r = run(
        "A\nB\nA\nC\n",
        &TransformKind::DeduplicateLines {
            keep: KeepPolicy::First,
            blank: BlankPolicy::Included,
        },
    );
    assert_eq!(r.content, "A\nB\nC\n");
    assert_eq!(r.removed_lines, Some(1));
}

#[test]
fn dedupe_keep_last_takes_last_position() {
    let r = run(
        "A\nB\nA\nC\n",
        &TransformKind::DeduplicateLines {
            keep: KeepPolicy::Last,
            blank: BlankPolicy::Included,
        },
    );
    assert_eq!(r.content, "B\nA\nC\n");
    assert_eq!(r.removed_lines, Some(1));
}

#[test]
fn dedupe_blank_policy_distinction() {
    // §74：空行参与去重时连续空行只留一条
    let r = run(
        "a\n\n\nb\n",
        &TransformKind::DeduplicateLines {
            keep: KeepPolicy::First,
            blank: BlankPolicy::Included,
        },
    );
    assert_eq!(r.content, "a\n\nb\n");
    // Preserve：空行原样保留
    let r = run(
        "a\n\n\nb\n",
        &TransformKind::DeduplicateLines {
            keep: KeepPolicy::First,
            blank: BlankPolicy::Preserve,
        },
    );
    assert_eq!(r.content, "a\n\n\nb\n");
}

#[test]
fn sort_lines_deterministic_with_spec_fixtures() {
    // §75 指定夹具：a A a10 a2 10 2 中文 accented
    let content = "a\nA\na10\na2\n10\n2\n中文\ncafé\n";
    let r = run(
        content,
        &TransformKind::SortLines {
            descending: false,
            case_sensitive: true,
            blank: BlankPolicy::Included,
        },
    );
    // 字节序确定性：10 2 A a a10 a2 café 中文（UTF-8 字节序）
    assert_eq!(r.content, "10\n2\nA\na\na10\na2\ncafé\n中文\n");
    let r = run(
        content,
        &TransformKind::SortLines {
            descending: false,
            case_sensitive: false,
            blank: BlankPolicy::Included,
        },
    );
    // 不敏感：数字在前；A/a 同键（稳定序保持输入相对序：输入 a 在前）；
    // 同为小写键后按字节序 a10 < a2（'1'<'2'，无自然排序——D40 明确）
    assert_eq!(r.content, "10\n2\na\nA\na10\na2\ncafé\n中文\n");
}

#[test]
fn sort_empty_line_policies() {
    let content = "b\n\na\n";
    let mk = |blank| TransformKind::SortLines {
        descending: false,
        case_sensitive: true,
        blank,
    };
    assert_eq!(run(content, &mk(BlankPolicy::First)).content, "\na\nb\n");
    assert_eq!(run(content, &mk(BlankPolicy::Last)).content, "a\nb\n\n");
    assert_eq!(run(content, &mk(BlankPolicy::Preserve)).content, "a\n\nb\n");
}

#[test]
fn prefix_suffix_respect_blank_skip() {
    let kind_p = TransformKind::AddPrefix {
        text: "- ".into(),
        skip_blank: true,
    };
    let r = run("apple\n\nbanana\n", &kind_p);
    assert_eq!(r.content, "- apple\n\n- banana\n");
    let kind_s = TransformKind::AddSuffix {
        text: ",".into(),
        skip_blank: true,
    };
    let r = run("apple\n\nbanana", &kind_s);
    assert_eq!(r.content, "apple,\n\nbanana,");
}

#[test]
fn case_forms_defined_semantics() {
    let up = TransformKind::CaseConvert {
        form: CaseForm::Upper,
    };
    assert_eq!(run("héllo wörld\n", &up).content, "HÉLLO WÖRLD\n");
    let title = TransformKind::CaseConvert {
        form: CaseForm::Title,
    };
    assert_eq!(
        run("hello WORLD foo\n", &title).content,
        "Hello World Foo\n"
    );
    let sentence = TransformKind::CaseConvert {
        form: CaseForm::Sentence,
    };
    assert_eq!(run("hELLO wORLD\n", &sentence).content, "Hello world\n");
}

#[test]
fn line_numbering_start_step_separator_pad() {
    let kind = TransformKind::NumberLines {
        start: 8,
        step: 2,
        separator: ". ".into(),
        pad: PadMode::Zeros,
    };
    let r = run("alpha\nbeta\ngamma", &kind);
    assert_eq!(r.content, "08. alpha\n10. beta\n12. gamma");
}

#[test]
fn find_replace_literal_with_count() {
    let kind = TransformKind::FindReplace {
        find: "cat".into(),
        replacement: "dog".into(),
        regex: false,
        case_insensitive: false,
        first_only: false,
    };
    let r = run("cat catalog CAT", &kind);
    assert_eq!(r.content, "dog dogalog CAT");
    assert_eq!(r.match_count, Some(2));
    // §80：replace first
    let first = TransformKind::FindReplace {
        find: "cat".into(),
        replacement: "dog".into(),
        regex: false,
        case_insensitive: false,
        first_only: true,
    };
    let r = run("cat cat", &first);
    assert_eq!(r.content, "dog cat");
    assert_eq!(r.match_count, Some(1));
}

#[test]
fn regex_replace_uses_rust_replacement_syntax() {
    // §82：contract = Rust regex（$1）；日期重排
    let kind = TransformKind::FindReplace {
        find: r"(\d{4})-(\d{2})-(\d{2})".into(),
        replacement: "$3/$2/$1".into(),
        regex: true,
        case_insensitive: false,
        first_only: false,
    };
    let r = run("on 2026-10-04 ok", &kind);
    assert_eq!(r.content, "on 04/10/2026 ok");
    assert_eq!(r.match_count, Some(1));
}

#[test]
fn regex_errors_are_structured_and_empty_find_rejected() {
    let bad = TransformKind::FindReplace {
        find: "([".into(),
        replacement: String::new(),
        regex: true,
        case_insensitive: false,
        first_only: false,
    };
    let err = apply_transform("x", &bad).expect_err("invalid regex");
    assert_eq!(err.code, "text.invalidRegex");
    let empty = TransformKind::FindReplace {
        find: String::new(),
        replacement: "x".into(),
        regex: false,
        case_insensitive: false,
        first_only: false,
    };
    assert_eq!(
        apply_transform("x", &empty).expect_err("empty").code,
        "text.emptyFind"
    );
}

#[test]
fn line_endings_travel_with_lines() {
    // §17 Preserve：CRLF 结尾在变换后保持 CRLF
    let r = run("  a  \r\n  b  \r\n", &TransformKind::TrimLines);
    assert_eq!(r.content, "a\r\nb\r\n");
    let r = run(
        "b\r\na\r\n",
        &TransformKind::SortLines {
            descending: false,
            case_sensitive: true,
            blank: BlankPolicy::Included,
        },
    );
    assert_eq!(r.content, "a\r\nb\r\n");
}

#[test]
fn determinism_same_input_same_options() {
    // §71：同输入同选项必同输出
    let kind = TransformKind::SortLines {
        descending: true,
        case_sensitive: false,
        blank: BlankPolicy::Last,
    };
    let a = run("x\nY\nz\n", &kind);
    let b = run("x\nY\nz\n", &kind);
    assert_eq!(a, b);
}
