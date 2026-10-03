//! Extractor 测试（M4 上 §58–§70 语义 + 下 §124 矩阵前置）。

use super::*;

fn vals(content: &str, kind: ExtractKind) -> Vec<String> {
    extract_matches(content, kind, &ExtractOptions::default())
        .into_iter()
        .map(|m| m.value)
        .collect()
}

#[test]
fn url_extracts_and_trims_trailing_punctuation() {
    // §58：https://example.com). 中的 ). 不属于 URL
    assert_eq!(
        vals("see https://example.com).", ExtractKind::Url),
        vec!["https://example.com"]
    );
    assert_eq!(
        vals(
            "a http://x.io/a(b).com b https://y.cn/path?q=1&x=2",
            ExtractKind::Url
        ),
        vec!["http://x.io/a(b).com", "https://y.cn/path?q=1&x=2"]
    );
    // 平衡的括号属于 URL（wiki 风格）
    assert_eq!(
        vals(
            "(https://en.wikipedia.org/wiki/Rust_(programming))",
            ExtractKind::Url
        ),
        vec!["https://en.wikipedia.org/wiki/Rust_(programming)"]
    );
}

#[test]
fn email_practical_extraction() {
    let got = vals(
        "mail bob.smith+tag@example.co.uk and x@y.io; (a@b.cn)",
        ExtractKind::Email,
    );
    assert_eq!(got, vec!["bob.smith+tag@example.co.uk", "x@y.io", "a@b.cn"]);
}

#[test]
fn file_path_heuristics_are_conservative() {
    // §60：绝对盘符 / UNC / unix 绝对 / 显式相对；裸 word/word 不算
    let got = vals(
        "open C:\\foo\\bar.txt and \\\\srv\\share\\f.txt plus /home/u/f.txt and ../rel.txt, not word/word",
        ExtractKind::FilePath,
    );
    assert_eq!(
        got,
        vec![
            "C:\\foo\\bar.txt",
            "\\\\srv\\share\\f.txt",
            "/home/u/f.txt",
            "../rel.txt",
        ]
    );
}

#[test]
fn numbers_cover_required_forms_only() {
    // §61：整数/小数/负数/科学计数/百分比；D41：hex 不属于 Number；
    // currency 符号不并入匹配（"$5" 如实提取数值 "5"，符号留给调用方）
    let got = vals("42 -3.14 1e10 2.5E-3 75% x1 0xFF $5", ExtractKind::Number);
    assert_eq!(got, vec!["42", "-3.14", "1e10", "2.5E-3", "75%", "5"]);
}

#[test]
fn ipv4_validates_octet_range() {
    // §62：999.999.999.999 不是有效 IP
    let got = vals(
        "8.8.8.8 and 999.999.999.999 and 192.168.1.256",
        ExtractKind::Ipv4,
    );
    assert_eq!(got, vec!["8.8.8.8"]);
}

#[test]
fn ipv6_uses_real_parser() {
    let got = vals(
        "v6 ::1 and fe80::1ff:ce23:2931:ab but 12:34 is time and ::ffff:192.168.1.1 is v6",
        ExtractKind::Ipv6,
    );
    assert_eq!(
        got,
        vec!["::1", "fe80::1ff:ce23:2931:ab", "::ffff:192.168.1.1"]
    );
}

#[test]
fn json_balanced_scan_handles_strings_and_nesting() {
    // §64：字符串内的引号/花括号与嵌套
    let content =
        r#"prefix {"x":"}", "nested":{"a":[1,2,{"deep":true}]}} middle [1, 2] not {"broken": "#;
    let got = vals(content, ExtractKind::Json);
    assert_eq!(got.len(), 2);
    assert!(got[0].starts_with(r#"{"x":"}""#));
    assert!(got[0].contains(r#""deep":true"#));
    assert_eq!(got[1], "[1, 2]");
    // 未闭合/非法不产生匹配
    assert!(vals(r"no json { here", ExtractKind::Json).is_empty());
}

#[test]
fn markdown_links_skip_code_contexts() {
    // §65：label/url/range；代码围栏与行内代码不解释
    let content = "[docs](https://docs.rs) and `<https://in.code>`\n\n```\n[fake](https://fake.rs)\n```\n<https://autolink.example>";
    let matches = extract_matches(
        content,
        ExtractKind::MarkdownLink,
        &ExtractOptions::default(),
    );
    // D41 v1：只做 [text](url)；autolink 暂不提取（如实）
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].label.as_deref(), Some("docs"));
    assert_eq!(matches[0].value, "https://docs.rs");
    assert!(matches[0].raw_value.starts_with("[docs]("));
}

#[test]
fn user_regex_structured_error_and_zero_length_skip() {
    let err = extract_regex("x", "([", &ExtractOptions::default()).expect_err("bad pattern");
    assert_eq!(err.code, "text.invalidRegex");
    // 零长度匹配跳过（D41）："aba" 上 a* 得 a@0 与 a@2；@1 空匹配不产出
    let got = extract_regex("aba", "a*", &ExtractOptions::default()).expect("regex");
    assert!(got.iter().all(|m| m.end > m.start));
    assert_eq!(got.len(), 2);
}

#[test]
fn ordering_is_source_order_with_deterministic_ties() {
    // §69：start 升序
    let content = "https://b.io x@y.io https://a.io";
    let urls = extract_matches(content, ExtractKind::Url, &ExtractOptions::default());
    assert!(urls.windows(2).all(|w| w[0].start < w[1].start));
    assert_eq!(urls[0].value, "https://b.io");
}

#[test]
fn duplicate_policy_keeps_occurrences_and_optionally_uniques() {
    // §70：出现次数与唯一值都是事实
    let content = "a@x.io b@y.io a@x.io";
    let all = extract_matches(content, ExtractKind::Email, &ExtractOptions::default());
    assert_eq!(all.len(), 3);
    let uniq = extract_matches(
        content,
        ExtractKind::Email,
        &ExtractOptions {
            unique_values: true,
        },
    );
    assert_eq!(uniq.len(), 2);
}

#[test]
fn match_positions_are_line_and_utf16_column() {
    // §18/§57：line/column 契约（UTF-16 列）
    let content = "x\n联系 bob@example.com";
    let m = &extract_matches(content, ExtractKind::Email, &ExtractOptions::default())[0];
    assert_eq!(m.line, 2);
    // "联系 " = 2 汉字 + 1 空格 = 3 UTF-16 units → bob 起始于列 4
    assert_eq!(m.column, 4);
}
