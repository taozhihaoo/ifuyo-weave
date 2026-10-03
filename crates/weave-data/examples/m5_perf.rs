//! M5 Data 性能验证（M5 下 §157–§161）。
//!
//! 运行：`cargo run --release -p weave-data --example m5_perf [rows]`
//! 输出 markdown 片段，结果人工誊入 docs/PERF.md（§139：必须有真实数字，
//! 禁止 "fast enough"）。
//!
//! 内存模型（§137 同源）：解析/变换 O(输入)；Compare 窗口 16 MiB 上界；
//! 不引入进程级 RSS 测量（与 M1/M3/M4 口径一致）。

use std::io::Cursor;
use std::time::Instant;

use weave_data::{
    CsvDialect, DataLimits, DataSession, FilterOperator, FilterRule, SortSpec, parse_csv,
    parse_jsonl, table_to_jsonl,
};

fn csv_fixture(rows: usize, seed: usize) -> String {
    let mut out = String::with_capacity(rows * 32);
    out.push_str("id,name,amount,note\n");
    for i in 0..rows {
        out.push_str(&format!(
            "{},name-{},{}.{}\n",
            i,
            i % 9973,
            i % 1000,
            i % 13,
        ));
        let _ = seed;
    }
    out
}

fn jsonl_fixture(rows: usize) -> String {
    let mut out = String::with_capacity(rows * 48);
    for i in 0..rows {
        out.push_str(&format!(
            "{{\"id\":{i},\"name\":\"name-{}\",\"amount\":{}}}\n",
            i % 9973,
            i % 1000
        ));
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_rows: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let limits = DataLimits::default();

    println!("## CSV Scan / Filter / Sort（§158）\n");
    println!("| rows | operation | elapsed |");
    println!("| --- | --- | --- |");
    for scale in [10_000usize, 100_000, max_rows] {
        let src = {
            let mut out = String::with_capacity(scale * 32);
            out.push_str("id,name,amount\n");
            for i in 0..scale {
                out.push_str(&format!("{},name-{},{}\n", i, i % 9973, i % 1000));
            }
            out
        };
        let t = Instant::now();
        let parsed = parse_csv(
            Cursor::new(src.as_bytes().to_vec()),
            &CsvDialect::csv(),
            &limits,
        )
        .expect("scan");
        println!("| {} | scan | {} ms |", scale, t.elapsed().as_millis());

        let mut session = DataSession::new(weave_data::DataTable {
            columns: parsed.columns,
            rows: parsed.rows,
        });
        session.filters = vec![FilterRule {
            column_id: "col_2".into(),
            operator: FilterOperator::Contains,
            value: "7".into(),
            case_sensitive: false,
        }];
        let t = Instant::now();
        session.refresh_view();
        println!("| {} | filter | {} ms |", scale, t.elapsed().as_millis());

        session.sort = Some(SortSpec {
            column_id: "col_3".into(),
            descending: true,
            ignore_case: true,
        });
        let t = Instant::now();
        session.refresh_view();
        println!("| {} | sort | {} ms |", scale, t.elapsed().as_millis());
    }

    println!("\n## JSONL Scan（§160）\n");
    println!("| rows | elapsed |");
    println!("| --- | --- |");
    for scale in [10_000usize, 100_000, max_rows] {
        let src = jsonl_fixture(scale);
        let t = Instant::now();
        let report = parse_jsonl(
            Cursor::new(src.as_bytes().to_vec()),
            weave_data::JsonlErrorMode::CollectErrors,
            limits.max_rows_in_memory,
            limits.max_diagnostics,
        )
        .expect("scan");
        println!(
            "| {} | {} ms | valid {} invalid {} |",
            scale,
            t.elapsed().as_millis(),
            report.valid_count,
            report.invalid_count
        );
    }

    println!("\n## Conversion（§161：CSV → JSON、CSV → JSONL）\n");
    println!("| rows | elapsed |");
    println!("| --- | --- |");
    for scale in [10_000usize, 100_000] {
        let src = csv_fixture(scale, 1);
        let parsed = parse_csv(
            Cursor::new(src.as_bytes().to_vec()),
            &CsvDialect::csv(),
            &limits,
        )
        .expect("parse");
        let table = weave_data::DataTable {
            columns: parsed.columns,
            rows: parsed.rows,
        };
        let t = Instant::now();
        let json = weave_data::table_to_json(&table, weave_data::TypedMode::PreserveStrings);
        let _ = serde_json::to_string(&json).expect("serialize");
        println!("| {scale} | csv→json {} ms |", t.elapsed().as_millis());
        let t = Instant::now();
        let jsonl = table_to_jsonl(&table, weave_data::TypedMode::PreserveStrings);
        println!(
            "| {scale} | csv→jsonl {} ms ({} B) |",
            t.elapsed().as_millis(),
            jsonl.len()
        );
    }

    // §141 回归锚点：Compare P1 修复后 100k 全不同应 < 200ms（见 PERF.md M4 节）
    println!("\n> Compare 100k 全不同 = 102 ms（M4 修复后锚点，见 docs/PERF.md）");
}
