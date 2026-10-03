//! 目录扫描 / Directory Analyzer 领域服务（M1 §12–§17）。
//!
//! 管线（M1 §12.1）：Input → Validate → Open → Walk → Classify → Collect →
//! Accumulate（增量）→ Finalize。统计全程流式增量，不缓存全量 FileInfo。
//!
//! 固定语义（详见 DECISIONS.md）：
//! - 深度：root = 0；子目录逐层 +1（M1 §12.5）
//! - 空目录：无直接普通文件且无直接子目录（M1 §12.4）
//! - 大小：普通文件逻辑大小之和；symlink 不跟随、不计入
//! - 排序：条目按名称 case-insensitive 处理；榜单规则见 stats::Ranking
//! - 失败聚合：单条目失败 → 记录 + 继续（998 成功 / 2 失败）；
//!   root 打不开 → 整体失败（M1 §15）

mod model;
mod stats;

pub use model::{
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_ENTRIES, DirectoryScanReport, FileLineItem, HARD_MAX_DEPTH,
    HARD_MAX_ENTRIES, ScanErrorEntry, ScanOptions,
};
pub use stats::TOP_K;

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use weave_core::prelude::{
    CancellationToken, FileKind, OperationId, Progress, ScanStatus, WeaveError,
    validate_absolute_path,
};

use crate::classify::classify_by_extension;
use crate::fs::Filesystem;
use stats::{Ranking, TopK};

/// 错误记录上限（超出部分计数但仍记录总数，`errors_truncated` 标记）。
const MAX_ERROR_ENTRIES: usize = 100;
/// 空目录记录上限（同上）。
const MAX_EMPTY_ENTRIES: usize = 1000;

struct Frame {
    abs: PathBuf,
    rel: String,
    depth: u32,
    names: Vec<String>,
    iter_pos: usize,
    files: u64,
    dirs: u64,
}

/// 扫描一个目录（协作取消 + 有界 + 确定性）。
pub fn scan_directory(
    fs: &dyn Filesystem,
    raw_path: &str,
    options: &ScanOptions,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> Result<DirectoryScanReport, WeaveError> {
    let started_wall = SystemTime::now();
    let started = Instant::now();
    let validation =
        validate_absolute_path(raw_path).map_err(|e| e.with_location("weave-files::scan"))?;
    let root_abs = PathBuf::from(&validation.normalized);
    let scan_id = OperationId::generate();

    let root_stat = fs.stat(&root_abs).map_err(|e| {
        let message = format!("cannot stat root '{}': {e}", validation.normalized);
        let base = match e.kind() {
            std::io::ErrorKind::NotFound => WeaveError::io("path.notFound", message),
            std::io::ErrorKind::PermissionDenied => {
                WeaveError::permission("path.permissionDenied", message)
            }
            _ => WeaveError::io("scan.rootFailed", message),
        };
        base.with_location("weave-files::scan")
    })?;
    if root_stat.kind != FileKind::Directory {
        return Err(WeaveError::validation(
            "scan.notDirectory",
            format!("'{}' is not a directory", validation.normalized),
        )
        .with_location("weave-files::scan"));
    }
    let root_names = fs.read_dir(&root_abs).map_err(|e| {
        // root 打不开 = 整个扫描失败（M1 §15），与子目录失败严格区分。
        WeaveError::io(
            "scan.rootReadFailed",
            format!("cannot open root '{}/': {e}", validation.normalized),
        )
        .with_location("weave-files::scan")
    })?;

    let max_depth = options
        .max_depth
        .unwrap_or(DEFAULT_MAX_DEPTH)
        .min(HARD_MAX_DEPTH);
    let max_entries = options
        .max_entries
        .unwrap_or(DEFAULT_MAX_ENTRIES)
        .min(HARD_MAX_ENTRIES);

    let mut files_scanned: u64 = 0;
    let mut directories_scanned: u64 = 0;
    let mut other_entries: u64 = 0;
    let mut error_count: u64 = 0;
    let mut entries_processed: u64 = 0;
    let mut total_size: u64 = 0;
    let mut max_depth_seen: u32 = 0;
    let mut distribution: BTreeMap<String, u64> = BTreeMap::new();
    let mut largest = TopK::new(Ranking::Largest, TOP_K);
    let mut oldest = TopK::new(Ranking::Oldest, TOP_K);
    let mut newest = TopK::new(Ranking::Newest, TOP_K);
    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<ScanErrorEntry> = Vec::new();
    let mut errors_truncated = false;
    let mut empty_directories: Vec<String> = Vec::new();
    let mut empty_truncated = false;
    let mut limited = false;
    let mut limited_reason: Option<String> = None;
    let mut cancelled = false;
    let mut depth_warning_emitted = false;

    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(root_abs.to_string_lossy().to_lowercase());

    let mut stack: Vec<Frame> = vec![Frame {
        abs: root_abs.clone(),
        rel: String::new(),
        depth: 0,
        names: sort_names(root_names),
        iter_pos: 0,
        files: 0,
        dirs: 0,
    }];
    // 新帧延迟到下一轮迭代顶推入：避免持有 stack.last_mut() 借用的同时 push。
    let mut pending_frame: Option<Frame> = None;

    loop {
        if let Some(new_frame) = pending_frame.take() {
            max_depth_seen = max_depth_seen.max(new_frame.depth);
            stack.push(new_frame);
            continue;
        }
        let Some(frame) = stack.last_mut() else {
            break;
        };
        if frame.iter_pos >= frame.names.len() {
            let done = stack.pop().expect("frame exists");
            if done.files == 0 && done.dirs == 0 {
                if empty_directories.len() < MAX_EMPTY_ENTRIES {
                    empty_directories.push(done.rel.clone());
                } else {
                    empty_truncated = true;
                }
            }
            if let Some(parent) = stack.last_mut() {
                parent.files += done.files;
                parent.dirs += done.dirs + 1;
            }
            continue;
        }

        if entries_processed >= max_entries {
            limited = true;
            limited_reason = Some(format!(
                "entry limit reached ({max_entries}); remaining entries were not scanned"
            ));
            break;
        }
        let name = frame.names[frame.iter_pos].clone();
        frame.iter_pos += 1;
        entries_processed += 1;

        if entries_processed.is_multiple_of(512) {
            // total 不可预知：不伪造百分比（M1 §31）。
            report_progress(Progress::running(scan_id.clone(), entries_processed, None));
        }
        if entries_processed.is_multiple_of(256) && cancel.is_cancelled() {
            cancelled = true;
        }
        if cancelled {
            break;
        }
        let child_abs = frame.abs.join(&name);
        let child_rel = if frame.rel.is_empty() {
            name.clone()
        } else {
            format!("{}\\{}", frame.rel, name)
        };

        let stat = match fs.stat(&child_abs) {
            Ok(stat) => stat,
            Err(e) => {
                error_count += 1;
                push_error(
                    &mut errors,
                    &mut errors_truncated,
                    ScanErrorEntry {
                        relative_path: child_rel,
                        code: io_error_code(&e),
                        message: format!("{e}"),
                    },
                );
                continue;
            }
        };

        match stat.kind {
            FileKind::RegularFile => {
                files_scanned += 1;
                frame.files += 1;
                total_size += stat.size;
                let extension = Path::new(&name)
                    .extension()
                    .map(|e| e.to_string_lossy().into_owned());
                let category = classify_by_extension(extension.as_deref()).category;
                *distribution
                    .entry(category.as_str().to_string())
                    .or_insert(0) += 1;
                let line = FileLineItem {
                    relative_path: child_rel,
                    size: stat.size,
                    modified: stat.modified,
                };
                largest.push(line.clone());
                oldest.push(line.clone());
                newest.push(line);
            }
            FileKind::Directory => {
                directories_scanned += 1;
                frame.dirs += 1;
                if frame.depth + 1 > max_depth {
                    limited = true;
                    if !depth_warning_emitted {
                        depth_warning_emitted = true;
                        warnings.push(format!(
                            "depth limit ({max_depth}) reached at '{child_rel}'; deeper content was not scanned"
                        ));
                    }
                    limited_reason
                        .get_or_insert_with(|| format!("max depth reached ({max_depth})"));
                    continue;
                }
                let key = child_abs.to_string_lossy().to_lowercase();
                if !visited.insert(key) {
                    // symlink/junction 策略下几乎不可达；防御性兜底：不重复进入。
                    warnings.push(format!(
                        "possible cycle at '{child_rel}'; entered only once"
                    ));
                    continue;
                }
                let names = match fs.read_dir(&child_abs) {
                    Ok(names) => names,
                    Err(e) => {
                        error_count += 1;
                        push_error(
                            &mut errors,
                            &mut errors_truncated,
                            ScanErrorEntry {
                                relative_path: child_rel,
                                code: io_error_code(&e),
                                message: format!("{e}"),
                            },
                        );
                        continue;
                    }
                };
                pending_frame = Some(Frame {
                    abs: child_abs,
                    rel: child_rel,
                    depth: frame.depth + 1,
                    names: sort_names(names),
                    iter_pos: 0,
                    files: 0,
                    dirs: 0,
                });
            }
            FileKind::Symlink | FileKind::Other => {
                other_entries += 1;
            }
        }
    }

    let finished_wall = SystemTime::now();
    let status = if cancelled {
        ScanStatus::Cancelled
    } else if error_count > 0 || limited || !warnings.is_empty() {
        ScanStatus::CompletedWithWarnings
    } else {
        ScanStatus::Completed
    };

    Ok(DirectoryScanReport {
        scan_id,
        root: validation.normalized,
        status,
        started_at: started_wall,
        finished_at: finished_wall,
        duration_ms: started.elapsed().as_millis() as u64,
        directories_scanned,
        files_scanned,
        other_entries,
        error_count,
        entries_processed,
        total_size,
        max_depth: max_depth_seen,
        empty_directories,
        empty_directories_truncated: empty_truncated,
        file_type_distribution: sort_distribution(distribution),
        largest_files: largest.into_sorted(),
        oldest_files: oldest.into_sorted(),
        newest_files: newest.into_sorted(),
        warnings,
        errors,
        errors_truncated,
        limited,
        limited_reason,
    })
}

/// 名称排序：case-insensitive（Windows 语义）→ 原名稳定定序。
fn sort_names(mut names: Vec<String>) -> Vec<String> {
    names.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    names
}

/// 分布排序：count DESC → label ASC（M1 §17）。
fn sort_distribution(distribution: BTreeMap<String, u64>) -> Vec<(String, u64)> {
    let mut list: Vec<(String, u64)> = distribution.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list
}

fn push_error(errors: &mut Vec<ScanErrorEntry>, truncated: &mut bool, entry: ScanErrorEntry) {
    if errors.len() < MAX_ERROR_ENTRIES {
        errors.push(entry);
    } else {
        *truncated = true;
    }
}

fn io_error_code(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "path.notFound".to_string(),
        std::io::ErrorKind::PermissionDenied => "path.permissionDenied".to_string(),
        _ => "scan.entryFailed".to_string(),
    }
}

#[cfg(test)]
mod tests;
