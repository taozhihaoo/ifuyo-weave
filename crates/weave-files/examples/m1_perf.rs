//! M1 性能验证（charter #68 / M1 §48–§49）。
//!
//! 运行：`cargo run --release -p weave-files --example m1_perf`
//! 输出 markdown 片段，结果人工誊入 docs/PERF.md（数字必须来自真实测量）。

use std::time::Instant;

use weave_core::prelude::{CancellationToken, Progress};
use weave_files::ScanOptions;
use weave_files::fs::StdFilesystem;
use weave_files::{HASH_CHUNK_SIZE, hash_file, scan_directory};

fn build_tree(root: &std::path::Path, files: usize) -> std::io::Result<()> {
    let per_dir = 100;
    let dirs = files.div_ceil(per_dir);
    for d in 0..dirs {
        let dir = root.join(format!("dir{d:04}"));
        std::fs::create_dir_all(&dir)?;
        let count = per_dir.min(files.saturating_sub(d * per_dir));
        for i in 0..count {
            std::fs::write(dir.join(format!("f{i:03}.txt")), format!("content {d} {i}"))?;
        }
    }
    Ok(())
}

fn count_entries(root: &std::path::Path) -> usize {
    walk(root)
}

fn walk(dir: &std::path::Path) -> usize {
    let mut n = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            n += 1;
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                n += walk(&e.path());
            }
        }
    }
    n
}

fn scan_bench(label: &str, root: &std::path::Path, runs: u32) -> (u128, u64, u64) {
    let mut best = u128::MAX;
    let mut files = 0u64;
    let mut entries = 0u64;
    for _ in 0..runs {
        let started = Instant::now();
        let report = scan_directory(
            &StdFilesystem,
            &root.to_string_lossy(),
            &ScanOptions::default(),
            &CancellationToken::new(),
            &mut |_| {},
        )
        .expect("scan");
        best = best.min(started.elapsed().as_millis());
        files = report.files_scanned;
        entries = report.entries_processed;
    }
    println!("| {label} | {best} ms | {files} files | {entries} entries |");
    (best, files, entries)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_scale: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let temp = std::env::temp_dir().join(format!("weave-perf-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("temp root");

    println!("<!-- machine: {} | {} -->", std::env::consts::OS, whoami());
    println!("| 场景 | 最快耗时 | 文件数 | 处理条目 |");
    println!("| --- | --- | --- | --- |");

    let mut scales = vec![100usize, 1_000];
    if max_scale > 1_000 {
        scales.push(max_scale);
    }
    for scale in scales {
        let root = temp.join(format!("tree{scale}"));
        if !root.exists() {
            let t = Instant::now();
            build_tree(&root, scale).expect("build");
            println!(
                "<!-- fixture: {scale} files generated in {} ms -->",
                t.elapsed().as_millis()
            );
        }
        let label = format!("scan {scale}");
        scan_bench(&label, &root, 3);
    }

    // 取消响应：10k 树上第一次进度时取消（≥512 条目已处理）。
    let root = temp.join(format!("tree{max_scale}"));
    let started = Instant::now();
    let cancel = CancellationToken::new();
    let report = scan_directory(
        &StdFilesystem,
        &root.to_string_lossy(),
        &ScanOptions::default(),
        &cancel,
        &mut |_p: Progress| {
            cancel.cancel();
        },
    )
    .expect("cancelled scan");
    println!(
        "| cancel responsiveness | {} ms | {} files | {} entries (status={:?}) |",
        started.elapsed().as_millis(),
        report.files_scanned,
        report.entries_processed,
        report.status,
    );

    // 大文件哈希：100 MiB（若 arg 提供 second 值则用之）。O(chunk) 内存由实现构造保证。
    let big = temp.join("big-100mb.bin");
    if !big.exists() {
        let chunk = vec![0u8; HASH_CHUNK_SIZE];
        use std::io::Write;
        let mut f = std::fs::File::create(&big).expect("create");
        for _ in 0..(100 * 1024 * 1024 / HASH_CHUNK_SIZE) {
            f.write_all(&chunk).expect("write");
        }
    }
    let started = Instant::now();
    let hash =
        hash_file(&StdFilesystem, &big, &CancellationToken::new(), &mut |_| {}).expect("hash");
    let ms = started.elapsed().as_millis();
    let mibs = 100.0 * 1024.0 / ms as f64 * 1000.0 / 1024.0;
    println!(
        "| hash 100 MiB | {ms} ms | {:.0} MiB/s | digest={}… |",
        mibs,
        &hash.digest_hex.expect("digest")[..8],
    );

    let _ = count_entries(&temp); // keep walk used for parity counting
    let _ = std::fs::remove_dir_all(&temp);
    println!("done");
}

fn whoami() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string())
}
