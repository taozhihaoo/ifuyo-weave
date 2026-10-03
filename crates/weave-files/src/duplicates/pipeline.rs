//! 三级管线实现（M3 §31）。
//!
//! Stage 顺序：Scanning → Filtering（size 分组）→ PartialHashing →
//! FullHashing → Grouping。内存以候选数量为界，绝不缓存文件内容。

use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use sha2::{Digest, Sha256};
use weave_core::prelude::{CancellationToken, OperationId, Progress, WeaveError};

use crate::fs::Filesystem;

use super::model::{DuplicateFileEntry, DuplicateGroup, DuplicateScanReport, ScanStage};

/// Partial hash 策略常量（M3 §4 验收硬化：单一集中定义，理由记 DECISIONS.md）。
///
/// 策略：SHA-256 over (head N + tail N)。文件不超过 2N 时等价于全量哈希；
/// 超过 2N 时为两次定位读共 2N 字节。N=4 KiB 平衡头部特征覆盖与 I/O 成本。
/// 局限：中部差异无法被 partial 区分——设计内行为，由 full hash 兜底，
/// partial 相同绝不视为重复（M3 §3）。
pub const PARTIAL_HASH_BYTES: u64 = 4 * 1024;

#[derive(Clone)]
struct Candidate {
    path: PathBuf,
    normalized: String,
    size: u64,
    modified: Option<SystemTime>,
}

#[derive(Default)]
struct ScanCounters {
    files_scanned: u64,
    directories_scanned: u64,
    other_entries: u64,
    stat_errors: u64,
}

fn collect_files(
    fs: &dyn Filesystem,
    root: &Path,
    visited: &mut HashSet<String>,
    counters: &mut ScanCounters,
    out: &mut Vec<Candidate>,
    cancel: &CancellationToken,
) -> Result<(), WeaveError> {
    let mut names = fs.read_dir(root).map_err(|e| {
        WeaveError::io("duplicates.rootReadFailed", format!("cannot read dir: {e}"))
            .with_location("weave-files::duplicates")
    })?;
    names.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });

    let mut subdirs: Vec<PathBuf> = Vec::new();
    for name in names {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let child = root.join(&name);
        let stat = match fs.stat(&child) {
            Ok(s) => s,
            Err(_) => {
                counters.stat_errors += 1;
                continue;
            }
        };
        match stat.kind {
            weave_core::prelude::FileKind::RegularFile => {
                counters.files_scanned += 1;
                out.push(Candidate {
                    path: child.clone(),
                    normalized: child.to_string_lossy().to_lowercase(),
                    size: stat.size,
                    modified: stat.modified,
                });
            }
            weave_core::prelude::FileKind::Directory => {
                let norm = child.to_string_lossy().to_lowercase();
                if visited.insert(norm) {
                    counters.directories_scanned += 1;
                    subdirs.push(child);
                }
            }
            _ => counters.other_entries += 1,
        }
    }
    for subdir in subdirs {
        if cancel.is_cancelled() {
            return Ok(());
        }
        collect_files(fs, &subdir, visited, counters, out, cancel)?;
    }
    Ok(())
}

/// Full hash 复用 M1 SHA-256 流式实现 + 前后一致性检查（M3 §17）。
fn full_hash_stable(
    fs: &dyn Filesystem,
    path: &Path,
    expected_size: u64,
) -> Result<String, WeaveError> {
    let before = fs.stat(path).map_err(|e| {
        WeaveError::io("hash.failed", format!("pre-hash stat failed: {e}"))
            .with_location("weave-files::duplicates")
    })?;
    let mut reader = fs.open_read(path).map_err(|e| {
        WeaveError::io("hash.failed", format!("hash open failed: {e}"))
            .with_location("weave-files::duplicates")
    })?;
    let mut hasher = Sha256::new();
    let mut chunk = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut chunk).map_err(|e| {
            WeaveError::io("hash.failed", format!("hash read failed: {e}"))
                .with_location("weave-files::duplicates")
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
        total += n as u64;
    }
    let after = fs.stat(path).map_err(|e| {
        WeaveError::io("hash.failed", format!("post-hash stat failed: {e}"))
            .with_location("weave-files::duplicates")
    })?;
    let stable = before.size == after.size
        && before.modified == after.modified
        && total == after.size
        && after.size == expected_size;
    if !stable {
        return Err(WeaveError::conflict(
            "duplicates.changedDuringScan",
            format!(
                "file changed while being hashed: {}",
                path.to_string_lossy()
            ),
        )
        .with_location("weave-files::duplicates"));
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Partial hash：≤ 2N 等价全量；> 2N 读首 N + 末 N（两次定位读）。
pub fn partial_hash(fs: &dyn Filesystem, path: &Path, size: u64) -> Result<String, WeaveError> {
    let n = PARTIAL_HASH_BYTES as usize;
    let mut reader = fs.open_read(path).map_err(|e| {
        WeaveError::io("hash.failed", format!("hash open failed: {e}"))
            .with_location("weave-files::duplicates")
    })?;

    let mut head = vec![0u8; n];
    let head_read = read_up_to_weave(&mut reader, &mut head)?;
    head.truncate(head_read);

    let mut tail: Vec<u8> = Vec::new();
    if size > PARTIAL_HASH_BYTES {
        let tail_len = n.min((size - PARTIAL_HASH_BYTES) as usize);
        let mut reader2 = fs.open_read(path).map_err(|e| {
            WeaveError::io("hash.failed", format!("hash reopen failed: {e}"))
                .with_location("weave-files::duplicates")
        })?;
        use std::io::Seek;
        reader2
            .seek(std::io::SeekFrom::Start(size - tail_len as u64))
            .map_err(|e| {
                WeaveError::io("hash.failed", format!("hash seek failed: {e}"))
                    .with_location("weave-files::duplicates")
            })?;
        tail = vec![0u8; tail_len];
        let mut filled = 0usize;
        while filled < tail.len() {
            let got = reader2.read(&mut tail[filled..]).map_err(|e| {
                WeaveError::io("hash.failed", format!("hash tail read failed: {e}"))
                    .with_location("weave-files::duplicates")
            })?;
            if got == 0 {
                break;
            }
            filled += got;
        }
        tail.truncate(filled);
    }

    let mut hasher = Sha256::new();
    hasher.update(b"weave-partial-v1");
    hasher.update(&head);
    hasher.update(&tail);
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn read_up_to_weave(reader: &mut dyn Read, buf: &mut [u8]) -> Result<usize, WeaveError> {
    let mut filled = 0usize;
    while filled < buf.len() {
        let n = reader.read(&mut buf[filled..]).map_err(|e| {
            WeaveError::io("hash.failed", format!("hash read failed: {e}"))
                .with_location("weave-files::duplicates")
        })?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(filled)
}

/// 重复扫描（M3 §31 主算法骨架）。
pub fn scan_duplicates(
    fs: &dyn Filesystem,
    roots: &[String],
    min_size: u64,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> Result<DuplicateScanReport, WeaveError> {
    let started = Instant::now();
    let scan_id = OperationId::generate();

    if roots.is_empty() {
        return Err(
            WeaveError::validation("duplicates.emptySelection", "no scan roots selected")
                .with_location("weave-files::duplicates"),
        );
    }
    let mut validated_roots = Vec::with_capacity(roots.len());
    for raw in roots {
        let v = weave_core::prelude::validate_absolute_path(raw)
            .map_err(|e| e.with_location("weave-files::duplicates"))?;
        validated_roots.push(PathBuf::from(&v.normalized));
    }

    let mut warnings: Vec<String> = Vec::new();
    let mut files_scanned: u64 = 0;
    let mut directories_scanned: u64 = 0;
    let mut other_entries: u64 = 0;
    let mut stat_errors: u64 = 0;

    // ── Stage: Scanning（复用 M1 symlink 策略：lstat、不跟随、visited 防环）──
    let mut visited: HashSet<String> = HashSet::new();
    for root in &validated_roots {
        visited.insert(root.to_string_lossy().to_lowercase());
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut counters = ScanCounters::default();
    for root in &validated_roots {
        if cancel.is_cancelled() {
            break;
        }
        collect_files(
            fs,
            root,
            &mut visited,
            &mut counters,
            &mut candidates,
            cancel,
        )?;
        files_scanned += counters.files_scanned;
        directories_scanned += counters.directories_scanned;
        other_entries += counters.other_entries;
        stat_errors += counters.stat_errors;
    }
    if stat_errors > 0 {
        warnings.push(format!(
            "{stat_errors} entries could not be stat-ed and were skipped"
        ));
    }

    // ── Stage: Filtering（size 分组 count>=2；min_size 之下跳过并计数）──
    let mut by_size: BTreeMap<u64, Vec<Candidate>> = BTreeMap::new();
    let mut below_min_size: u64 = 0;
    for c in candidates {
        if c.size < min_size {
            below_min_size += 1;
            continue;
        }
        by_size.entry(c.size).or_default().push(c);
    }
    if below_min_size > 0 {
        warnings.push(format!(
            "{below_min_size} files below the min-size filter were skipped"
        ));
    }
    let mut size_groups: Vec<Vec<Candidate>> =
        by_size.into_values().filter(|g| g.len() >= 2).collect();
    size_groups.sort_by(|a, b| b[0].size.cmp(&a[0].size));
    let candidate_files: u64 = size_groups.iter().map(|g| g.len() as u64).sum();

    // ── Stage: Partial Hashing（候选缩减层——partial 相同 ≠ 重复，M3 §3）──
    let mut partial_groups: Vec<Vec<Candidate>> = Vec::new();
    let mut partial_hashed: u64 = 0;
    for group in &size_groups {
        if cancel.is_cancelled() {
            break;
        }
        let mut by_partial: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
        for c in group {
            match partial_hash(fs, &c.path, c.size) {
                Ok(h) => {
                    partial_hashed += 1;
                    by_partial.entry(h).or_default().push(c.clone());
                }
                Err(e) => {
                    warnings.push(format!(
                        "partial hash skipped for one candidate: {}",
                        e.code
                    ));
                }
            }
        }
        for g in by_partial.into_values() {
            if g.len() >= 2 {
                partial_groups.push(g);
            }
        }
        if partial_hashed.is_multiple_of(64) {
            report_progress(Progress::running(
                scan_id.clone(),
                files_scanned + partial_hashed,
                None,
            ));
        }
    }

    // ── Stage: Full Hashing（复用 M1 SHA-256；前后一致性检查 M3 §17）──
    let mut full_groups: BTreeMap<String, Vec<DuplicateFileEntry>> = BTreeMap::new();
    let mut full_hashed: u64 = 0;
    let mut changed_during_scan: u64 = 0;
    for group in &partial_groups {
        if cancel.is_cancelled() {
            break;
        }
        let mut by_full: BTreeMap<String, Vec<DuplicateFileEntry>> = BTreeMap::new();
        for c in group {
            match full_hash_stable(fs, &c.path, c.size) {
                Ok(hex) => {
                    full_hashed += 1;
                    by_full.entry(hex).or_default().push(DuplicateFileEntry {
                        file_id: String::new(),
                        path: c.normalized.clone(),
                        size: c.size,
                        modified: c.modified,
                        full_hash: String::new(),
                    });
                }
                Err(e) if e.code == "duplicates.changedDuringScan" => {
                    changed_during_scan += 1;
                    warnings.push(format!(
                        "file changed during scan; excluded: {}",
                        c.normalized
                    ));
                }
                Err(e) => {
                    warnings.push(format!("full hash skipped for one candidate: {}", e.code));
                }
            }
        }
        for (hex, mut files) in by_full {
            if files.len() >= 2 {
                files.sort_by(|a, b| a.path.cmp(&b.path));
                for (i, f) in files.iter_mut().enumerate() {
                    f.file_id = format!("{}-{i:02}", &hex[..16.min(hex.len())]);
                    f.full_hash = hex.clone();
                }
                full_groups.insert(hex, files);
            }
        }
        if full_hashed.is_multiple_of(16) {
            report_progress(Progress::running(
                scan_id.clone(),
                files_scanned + partial_hashed + full_hashed,
                None,
            ));
        }
    }

    // ── Stage: Grouping（GroupId = grp_{full hash 前 16 hex}，内容派生，§43）──
    let mut groups: Vec<DuplicateGroup> = full_groups
        .into_iter()
        .map(|(hex, files)| {
            let file_size = files[0].size;
            DuplicateGroup {
                group_id: format!("grp_{}", &hex[..16]),
                file_count: files.len() as u64,
                file_size,
                wasted_size: file_size * (files.len() as u64 - 1),
                files,
            }
        })
        .collect();
    // 展示排序：潜在可回收 DESC（UI 策略，非 duplicate truth，M3 §44）
    groups.sort_by_key(|g| std::cmp::Reverse(g.wasted_size));

    let cancelled = cancel.is_cancelled();
    let duplicate_files: u64 = groups.iter().map(|g| g.file_count).sum();
    let potential_reclaimable_size: u64 = groups.iter().map(|g| g.wasted_size).sum();

    Ok(DuplicateScanReport {
        scan_id,
        roots: validated_roots
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        status: if cancelled {
            weave_core::prelude::ScanStatus::Cancelled
        } else if !warnings.is_empty() {
            weave_core::prelude::ScanStatus::CompletedWithWarnings
        } else {
            weave_core::prelude::ScanStatus::Completed
        },
        stage: ScanStage::Completed,
        duration_ms: started.elapsed().as_millis() as u64,
        files_scanned,
        directories_scanned,
        other_entries,
        candidate_files,
        partial_hashed,
        full_hashed,
        duplicate_groups: groups.len() as u64,
        duplicate_files,
        potential_reclaimable_size,
        skipped: below_min_size,
        failed: stat_errors,
        changed_during_scan,
        warnings,
        errors: Vec::new(),
        partial_result: cancelled,
        groups,
    })
}
