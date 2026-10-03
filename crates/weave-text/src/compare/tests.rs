//! Compare 测试（M4 上 §48–§55 语义 + §50 Unified）。

use super::*;

const OPTS: CompareOptions = CompareOptions {
    whitespace: WhitespaceMode::None,
    ignore_case: false,
};

fn diff(a: &str, b: &str) -> DiffReport {
    compare_texts(a, b, &OPTS, &CompareLimits::default()).expect("diff")
}

#[test]
fn identical_empty_and_single_line() {
    // §50 矩阵：empty / single line / identical
    assert!(diff("", "").identical);
    assert!(diff("one\n", "one\n").identical);
    assert!(diff("", "").hunks.is_empty());
    let r = diff("", "new\n");
    assert_eq!((r.stats.added, r.stats.removed), (1, 0));
}

#[test]
fn added_removed_changed_counts() {
    let r = diff("a\nb\nc\n", "a\nx\nc\nd\n");
    // b→x 为 Changed（等长配对），d 为 Added
    assert_eq!((r.stats.changed, r.stats.added), (1, 1));
    assert_eq!(r.stats.removed, 0);
    let hunk = &r.hunks[0];
    assert!(hunk.lines.iter().any(|l| l.change == LineChange::Changed));
}

#[test]
fn completely_different_is_one_block() {
    let r = diff("a\nb\n", "1\n2\n3\n");
    // 配对语义：等长部分成 Changed（a→1, b→2），余量 Added（3）
    assert_eq!(r.stats.changed, 2);
    assert_eq!(r.stats.added, 1);
    assert_eq!(r.hunks.len(), 1);
}

#[test]
fn final_newline_and_crlf_do_not_create_fake_changes() {
    // §17/D42：行尾换行不参与比较
    assert!(diff("a\nb", "a\nb\n").identical);
    assert!(diff("a\r\nb\r\n", "a\nb\n").identical);
}

#[test]
fn whitespace_modes_are_explicit_tiers() {
    // §51：Trailing 忽略行尾空白；All 忽略全部空白差异
    let trailing = CompareOptions {
        whitespace: WhitespaceMode::Trailing,
        ignore_case: false,
    };
    let r = compare_texts("a \nb\n", "a\nb\n", &trailing, &CompareLimits::default()).expect("ok");
    assert!(r.identical);
    // Trailing 不忽略中间空白差异
    let r = compare_texts("a b\n", "ab\n", &trailing, &CompareLimits::default()).expect("ok");
    assert!(!r.identical);
    let all = CompareOptions {
        whitespace: WhitespaceMode::All,
        ignore_case: false,
    };
    let r = compare_texts("a b\n", "ab\n", &all, &CompareLimits::default()).expect("ok");
    assert!(r.identical);
}

#[test]
fn case_ignore_is_unicode_folding() {
    // §52：A/a、重音字符统一折叠；中文不受影响
    let ci = CompareOptions {
        whitespace: WhitespaceMode::None,
        ignore_case: true,
    };
    let r =
        compare_texts("ABC\nCafé\n", "abc\nCAFÉ\n", &ci, &CompareLimits::default()).expect("ok");
    assert!(r.identical);
    let r = compare_texts("中文\n", "中文\n", &ci, &CompareLimits::default()).expect("ok");
    assert!(r.identical);
}

#[test]
fn moved_detection_pairs_identical_lines_once_each() {
    // §53：A B C → C A B：LCS 保留 alpha/beta 的相对序（Equal），gamma 两侧
    // 位置不同 ⇒ Moved 配对（引擎匹配事实，非语义断言）
    let r = diff("alpha\nbeta\ngamma\n", "gamma\nalpha\nbeta\n");
    assert_eq!(r.stats.moved, 1, "{:?}", r.stats);
    assert_eq!(r.stats.equal, 2);
    // 重复行按出现次数配对：B 两侧位置不同 ⇒ 1 Moved，两个 A 相对序不变
    let r = diff("A\nA\nB\n", "B\nA\nA\n");
    assert_eq!(r.stats.moved, 1, "{:?}", r.stats);
}

#[test]
fn determinism_same_input_same_output() {
    // §54：两次调用结果完全一致
    let a = "x\ny\nz\np\nq\n";
    let b = "x\nq\nz\np\ny\n";
    let r1 = diff(a, b);
    let r2 = diff(a, b);
    assert_eq!(r1, r2);
}

#[test]
fn too_large_is_explicit_not_silent() {
    // §55：超限显式 TooLarge
    let limits = CompareLimits {
        max_bytes: 10,
        max_lines: 100_000,
        lcs_window: 2000,
    };
    let err =
        compare_texts("a very long line indeed\n", "b\n", &OPTS, &limits).expect_err("too large");
    assert!(matches!(err, CompareError::TooLarge { .. }));
}

#[test]
fn degraded_flag_when_over_lcs_window() {
    // D42：超窗口 ⇒ 诚实降级（单块 + degraded 标注）；块内仍按配对语义
    let big_a: String = (0..2500).map(|i| format!("a{i}\n")).collect();
    let big_b: String = (0..2500).map(|i| format!("b{i}\n")).collect();
    let r = diff(&big_a, &big_b);
    assert!(r.degraded);
    assert_eq!(r.hunks.len(), 1);
    // 窗口只决定"走降级路径"；块内等长配对仍是全部 2500 对 Changed
    assert_eq!(r.stats.changed, 2500);
    assert_eq!(r.stats.added, 0);
    // 公共前后缀裁剪后小余量仍走最优
    let small_b = "HEAD\nmid\nTAIL\n";
    let r = diff("HEAD\nTAIL\n", small_b);
    assert!(!r.degraded);
    assert_eq!(r.stats.added, 1);
}

#[test]
fn unified_output_shape() {
    // §50：--- A / +++ B / @@ 行
    let r = diff("a\nb\nc\n", "a\nX\nc\n");
    let text = unified_diff(&r, "A.txt", "B.txt");
    assert!(text.starts_with("--- A.txt\n+++ B.txt\n"));
    assert!(text.contains("@@ -2,1 +2,1 @@"));
    assert!(text.contains("-b\n"));
    assert!(text.contains("+X\n"));
}
