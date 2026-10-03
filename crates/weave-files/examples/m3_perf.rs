//! M3 Duplicate Finder 性能验证（M3 下 §101–§103）。
//!
//! 运行：`cargo run --release -p weave-files --example m3_perf [scale]`
//! 输出 markdown 片段，结果人工誊入 docs/PERF.md（数字必须来自真实测量）。
//!
//! 场景（§102）：
//! - A：unique-size 文件（1k/10k/50k）——几乎不进 full hash
//! - B：同 size 不同内容（10k × 64 KiB）——partial hash 明显缩减 full hash
//! - C：大量 exact duplicates（10k）——分组正确
//! - D：大量 large files（32 × 32 MiB，一半重复）——流式 + 有界内存

use std::time::Instant;

use weave_core::prelude::CancellationToken;
use weave_files::fs::StdFilesystem;
use weave_files::scan_duplicates;

const KIB: u64 = 1024;
const MIB: u64 = 1024 * 1024;

fn write_file(path: &std::path::Path, size: u64, seed: u64) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    // 64 KiB 分块伪随机内容（xorshift），避免压缩/全零路径的测量偏差。
    let mut chunk = vec![0u8; 64 * KIB as usize];
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut remaining = size;
    while remaining > 0 {
        let n = chunk.len().min(remaining as usize);
        for b in &mut chunk[..n] {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *b = (state & 0xFF) as u8;
        }
        f.write_all(&chunk[..n])?;
        remaining -= n as u64;
    }
    Ok(())
}

struct Layout {
    dirs: usize,
    per_dir: usize,
}
fn layout(files: usize) -> Layout {
    let per_dir = 1000;
    Layout {
        dirs: files.div_ceil(per_dir),
        per_dir,
    }
}

struct RunStats {
    duration_ms: u128,
    files_scanned: u64,
    candidate_files: u64,
    partial_hashed: u64,
    full_hashed: u64,
    duplicate_groups: u64,
    duplicate_files: u64,
    potential_reclaimable_size: u64,
    bytes: u64,
}

fn run_scan(root: &std::path::Path) -> RunStats {
    let t = Instant::now();
    let report = scan_duplicates(
        &StdFilesystem,
        &[root.to_string_lossy().into_owned()],
        0,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("scan");
    let bytes = walk_bytes(root);
    RunStats {
        duration_ms: t.elapsed().as_millis(),
        files_scanned: report.files_scanned,
        candidate_files: report.candidate_files,
        partial_hashed: report.partial_hashed,
        full_hashed: report.full_hashed,
        duplicate_groups: report.duplicate_groups,
        duplicate_files: report.duplicate_files,
        potential_reclaimable_size: report.potential_reclaimable_size,
        bytes,
    }
}

fn walk_bytes(root: &std::path::Path) -> u64 {
    let mut total = 0;
    let stack = vec![root.to_path_buf()];
    let mut stack = stack;
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in read.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

fn reduction(s: &RunStats) -> String {
    if s.candidate_files == 0 {
        return "—".to_string();
    }
    format!(
        "{:.1}%",
        100.0 * (1.0 - s.full_hashed as f64 / s.candidate_files as f64)
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_scale: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let temp = std::env::temp_dir().join(format!("weave-m3-perf-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("temp root");
    let t_all = Instant::now();

    // ── Scenario A：unique-size（1k / 10k / 50k）──
    println!("## Scenario A — unique-size files\n");
    println!(
        "| files | scan | files_scanned | candidates | partial | full | reduction(full/cand) | groups |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    for scale in [1_000usize, 10_000, max_scale.max(10_000)] {
        let root = temp.join(format!("a{scale}"));
        let l = layout(scale);
        for d in 0..l.dirs {
            let dir = root.join(format!("dir{d:04}"));
            std::fs::create_dir_all(&dir).expect("mkdir");
            let count = l.per_dir.min(scale.saturating_sub(d * l.per_dir));
            for i in 0..count {
                let idx = d * l.per_dir + i;
                // 唯一 size：1 + idx 字节（50k 规模总量 ≈ 1.25 GiB，可控）。
                let size = 1u64 + idx as u64;
                write_file(&dir.join(format!("f{idx:08}.bin")), size, idx as u64).expect("write");
            }
        }
        let s = run_scan(&root);
        println!(
            "| {scale} | {} ms | {} | {} | {} | {} | {} | {} |",
            s.duration_ms,
            s.files_scanned,
            s.candidate_files,
            s.partial_hashed,
            s.full_hashed,
            reduction(&s),
            s.duplicate_groups
        );
        std::fs::remove_dir_all(&root).ok();
    }

    // ── Scenario B：同 size 不同内容（partial 缩减 full）──
    println!("\n## Scenario B — same size, distinct content (64 KiB each)\n");
    {
        let scale = 10_000usize;
        let root = temp.join("b");
        let l = layout(scale);
        for d in 0..l.dirs {
            let dir = root.join(format!("dir{d:04}"));
            std::fs::create_dir_all(&dir).expect("mkdir");
            let count = l.per_dir.min(scale.saturating_sub(d * l.per_dir));
            for i in 0..count {
                let idx = d * l.per_dir + i;
                write_file(
                    &dir.join(format!("f{idx:08}.bin")),
                    64 * KIB,
                    (idx as u64) + 1,
                )
                .expect("write");
            }
        }
        let s = run_scan(&root);
        println!(
            "| files | scan | candidates | partial | full | reduction | groups |\n| --- | --- | --- | --- | --- | --- | --- |\n| {scale} | {} ms | {} | {} | {} | {} | {} |",
            s.duration_ms,
            s.candidate_files,
            s.partial_hashed,
            s.full_hashed,
            reduction(&s),
            s.duplicate_groups
        );
        std::fs::remove_dir_all(&root).ok();
    }

    // ── Scenario C：大量 exact duplicates（分组正确性 + 规模）──
    println!("\n## Scenario C — exact duplicates (10 distinct contents × N)\n");
    {
        let scale = 10_000usize;
        let root = temp.join("c");
        let l = layout(scale);
        for d in 0..l.dirs {
            let dir = root.join(format!("dir{d:04}"));
            std::fs::create_dir_all(&dir).expect("mkdir");
            let count = l.per_dir.min(scale.saturating_sub(d * l.per_dir));
            for i in 0..count {
                let idx = d * l.per_dir + i;
                write_file(
                    &dir.join(format!("f{idx:08}.bin")),
                    8 * KIB,
                    (idx % 10) as u64 + 1,
                )
                .expect("write");
            }
        }
        let s = run_scan(&root);
        println!(
            "| files | scan | candidates | partial | full | groups | dup files | reclaimable |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n| {scale} | {} ms | {} | {} | {} | {} | {} | {} |",
            s.duration_ms,
            s.candidate_files,
            s.partial_hashed,
            s.full_hashed,
            s.duplicate_groups,
            s.duplicate_files,
            s.potential_reclaimable_size
        );
        assert_eq!(s.duplicate_groups, 10, "10 distinct contents → 10 groups");
        assert_eq!(s.duplicate_files, scale as u64);
        std::fs::remove_dir_all(&root).ok();
    }

    // ── Scenario D：large files（流式 + 有界内存）──
    println!("\n## Scenario D — large files (32 × 32 MiB, half duplicates)\n");
    {
        let root = temp.join("d");
        std::fs::create_dir_all(&root).expect("mkdir");
        for i in 0..32usize {
            write_file(
                &root.join(format!("big{i:02}.bin")),
                32 * MIB,
                (i % 16) as u64 + 1,
            )
            .expect("write");
        }
        let s = run_scan(&root);
        let gib = s.bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let throughput = if s.duration_ms > 0 {
            gib / (s.duration_ms as f64 / 1000.0)
        } else {
            0.0
        };
        println!(
            "| files | bytes | scan | full | groups | throughput |\n| --- | --- | --- | --- | --- | --- |\n| {} | {:.2} GiB | {} ms | {} | {} | {:.2} GiB/s |",
            s.files_scanned, gib, s.duration_ms, s.full_hashed, s.duplicate_groups, throughput
        );
        assert_eq!(s.duplicate_groups, 16);
        std::fs::remove_dir_all(&root).ok();
    }

    println!("\n(total {} ms)", t_all.elapsed().as_millis());
    std::fs::remove_dir_all(&temp).ok();
}
