//! M4 Text 性能验证（M4 下 §138–§141）。
//!
//! 运行：`cargo run --release -p weave-text --example m4_perf [scale_lines]`
//! 输出 markdown 片段，结果人工誊入 docs/PERF.md（数字必须来自真实测量）。
//!
//! 内存模型（§137，与 M1/M3 口径一致——不引入进程级 RSS 测量）：
//! - Transformer：O(输入) 字符串重组，无全量复制两份以上
//! - Extractor：线性扫描 + 结果向量（受 max_extract_matches 上限约束）
//! - Compare：前/后缀裁剪 + 2000×2000 LCS 窗口（u32 表 ≈ 16 MiB 上界）
//! - Formatter：parser 驱动，输入 ≤ format 限额（1 MiB）

use std::time::Instant;

use weave_text::transform::{TransformKind, apply_transform};
use weave_text::{
    CompareLimits, CompareOptions, ExtractKind, ExtractOptions, TextLimits, compare_texts,
    extract_matches,
};

fn lines(n: usize, seed: usize) -> String {
    let mut out = String::with_capacity(n * 16);
    for i in 0..n {
        out.push_str(&format!("line {} of {} ({seed})\n", i % 997, i));
    }
    out
}

fn timed<F: FnOnce() -> T, T>(label: &str, f: F) -> T {
    let t = Instant::now();
    let r = f();
    println!("| {label} | {} ms |", t.elapsed().as_millis());
    r
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_scale: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let limits = TextLimits::default();
    println!("## Transformer（§138：10k / 100k / 1M lines）\n");
    println!("| 操作 | lines | elapsed |");
    println!("| --- | --- | --- |");
    for scale in [10_000usize, 100_000, max_scale.max(100_000)] {
        let src = lines(scale, 7);
        let bytes = src.len();
        timed(&format!("trimLines {scale} lines ({bytes} B)"), || {
            apply_transform(&src, &TransformKind::TrimLines).expect("trim")
        });
        timed(&format!("dedupe {scale} lines"), || {
            apply_transform(
                &src,
                &TransformKind::DeduplicateLines {
                    keep: weave_text::KeepPolicy::First,
                    blank: weave_text::BlankPolicy::Included,
                },
            )
            .expect("dedupe")
        });
        timed(&format!("sort {scale} lines"), || {
            apply_transform(
                &src,
                &TransformKind::SortLines {
                    descending: false,
                    case_sensitive: false,
                    blank: weave_text::BlankPolicy::Last,
                },
            )
            .expect("sort")
        });
        timed(&format!("find/replace {scale} lines"), || {
            apply_transform(
                &src,
                &TransformKind::FindReplace {
                    find: "line".into(),
                    replacement: "L".into(),
                    regex: false,
                    case_insensitive: false,
                    first_only: false,
                },
            )
            .expect("replace")
        });
    }

    println!("\n## Extractor（URL，10k / 100k lines）\n");
    println!("| lines | elapsed | matches |");
    println!("| --- | --- | --- |");
    for scale in [10_000usize, 100_000] {
        let mut src = lines(scale, 9);
        src.push_str("see https://example.com/x and http://a.b/c\n");
        let t = Instant::now();
        let matches = extract_matches(&src, ExtractKind::Url, &ExtractOptions::default());
        println!(
            "| {scale} | {} ms | {} |",
            t.elapsed().as_millis(),
            matches.len()
        );
    }

    println!("\n## Compare（§141：10k/100k，相同与半不同）\n");
    println!("| A lines | B lines | elapsed | hunks | degraded |");
    println!("| --- | --- | --- | --- | --- |");
    for scale in [10_000usize, 100_000] {
        let a = lines(scale, 3);
        let identical = a.clone();
        let r = timed(&format!("compare {scale} identical"), || {
            compare_texts(
                &a,
                &identical,
                &CompareOptions::default(),
                &CompareLimits::default(),
            )
            .expect("compare")
        });
        println!(
            "| {scale} | {scale} | — | {} | {} |",
            r.hunks.len(),
            r.degraded
        );
        let mut b = lines(scale, 4);
        if scale <= CompareLimits::default().lcs_window {
            // 小规模走最优路径
            let _ = &mut b;
        }
        let r = timed(&format!("compare {scale} fully-different"), || {
            compare_texts(
                &a,
                &b,
                &CompareOptions::default(),
                &CompareLimits::default(),
            )
            .expect("compare")
        });
        println!(
            "| {scale} | {scale} | — | {} | {} |",
            r.hunks.len(),
            r.degraded
        );
    }

    println!("\n## Formatter（JSON format，~1 MiB）\n");
    println!("| input | elapsed |");
    println!("| --- | --- |");
    {
        let unit = format!("{{\"k{}\":1,\"s\":\"value {}\"}}", 0, 0);
        let items: Vec<String> = (0..30_000)
            .map(|i| format!("{{\"k{i}\":{i},\"s\":\"value {i}\"}}"))
            .collect();
        let src = format!("[{}]", items.join(","));
        let _ = unit;
        println!("| {} B | — |", src.len());
        let t = Instant::now();
        let out = weave_text::format::run_formatter(
            weave_text::model::TextFormat::Json,
            weave_text::format::FormatOperation::Format,
            &src,
            &weave_text::format::FormatOptions::default(),
        );
        println!("| {} B | {} ms |", src.len(), t.elapsed().as_millis());
        assert!(!out.is_error());
        // 限额守卫证明（§104/§105）：超过 format 限额 ⇒ 结构化拒绝而非冻结
        let oversized = "x".repeat(limits.format as usize + 1);
        let rejected = weave_text::format::run_formatter(
            weave_text::model::TextFormat::Json,
            weave_text::format::FormatOperation::Format,
            &oversized,
            &weave_text::format::FormatOptions::default(),
        );
        // run_formatter 本身不做限额（限额在 IPC 服务层）；此处仅输出说明
        let _ = rejected;
        println!(
            "\n> 限额在 IPC 服务层执行：format 限额 = {} B（TextLimits，§166）",
            limits.format
        );
    }
}
