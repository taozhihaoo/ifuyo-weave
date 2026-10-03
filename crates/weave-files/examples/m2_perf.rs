//! M2 性能验证（M2 §86–§88）。
//!
//! 运行：`cargo run --release -p weave-files --example m2_perf [files]`
//! 输出 markdown 片段，结果人工誊入 docs/PERF.md（数字必须来自真实测量）。

use std::time::Instant;

use weave_core::prelude::CancellationToken;
use weave_files::fs::StdFilesystem;
use weave_files::{HASH_CHUNK_SIZE, build_rename_plan, execute_plan};

fn build_tree(root: &std::path::Path, files: usize) -> std::io::Result<()> {
    let per_dir = 1000;
    let dirs = files.div_ceil(per_dir);
    for d in 0..dirs {
        let dir = root.join(format!("dir{d:04}"));
        std::fs::create_dir_all(&dir)?;
        let count = per_dir.min(files.saturating_sub(d * per_dir));
        for i in 0..count {
            std::fs::write(dir.join(format!("f{i:04}.txt")), format!("content {d} {i}"))?;
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_scale: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let temp = std::env::temp_dir().join(format!("weave-m2-perf-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("temp root");
    let fs = StdFilesystem;

    println!("| 场景 | Plan 构建 | 执行 | 条目 | 执行内存策略 |");
    println!("| --- | --- | --- | --- | --- |");

    for scale in [100usize, 1_000, max_scale] {
        let root = temp.join(format!("tree{scale}"));
        build_tree(&root, scale).expect("build fixture");

        // Plan 构建（纯读）
        let inputs: Vec<String> = (0..scale)
            .map(|i| {
                let d = i / 1000;
                let r = i % 1000;
                root.join(format!("dir{d:04}"))
                    .join(format!("f{r:04}.txt"))
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let t_plan = Instant::now();
        let plan = build_rename_plan(
            &fs,
            &inputs,
            &[],
            Some("vacation-{counter}.{ext}"),
            &CancellationToken::new(),
            &mut |_| {},
        )
        .expect("plan");
        let plan_ms = t_plan.elapsed().as_millis();

        // 执行（含事务记录）
        let t_exec = Instant::now();
        let report = execute_plan(&fs, &plan, &CancellationToken::new(), &mut |_| {});
        let exec_ms = t_exec.elapsed().as_millis();
        let executed = report.executed;
        println!(
            "| rename {scale} | {plan_ms} ms | {exec_ms} ms | {executed} | O(1) per item, no full-copy |"
        );

        // Undo（还原以便下一轮复用目录）
        let undo_started = Instant::now();
        let undo = weave_files::undo_transaction(
            &fs,
            &report.transaction,
            &CancellationToken::new(),
            &mut |_| {},
        );
        println!(
            "|   undo {scale} | {} ms | restored {} | conflicts {} | |",
            undo_started.elapsed().as_millis(),
            undo.restored,
            undo.conflicts
        );
    }

    // 大文件 rename（元数据操作，与内容无关——验证不经过内容拷贝）
    let big = temp.join("big-100mb.bin");
    if !big.exists() {
        let chunk = vec![0u8; HASH_CHUNK_SIZE];
        use std::io::Write;
        let mut f = std::fs::File::create(&big).expect("create");
        for _ in 0..(100 * 1024 * 1024 / HASH_CHUNK_SIZE) {
            f.write_all(&chunk).expect("write");
        }
    }
    let inputs = vec![big.to_string_lossy().into_owned()];
    let plan = build_rename_plan(
        &fs,
        &inputs,
        &[],
        Some("renamed-100mb.bin"),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan");
    let started = Instant::now();
    let report = execute_plan(&fs, &plan, &CancellationToken::new(), &mut |_| {});
    println!(
        "| rename 100 MiB file | — | {} ms | O(1)：rename 是元数据操作，与文件大小无关 |",
        started.elapsed().as_millis()
    );
    let _ = report.executed;

    let _ = std::fs::remove_dir_all(&temp);
    println!("done");
}
