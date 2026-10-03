//! 目录扫描测试套件（M1 §23–§27 / §46 / §50）。
//!
//! 覆盖：golden 树计数/大小/深度不变量、类型分布排序、榜单排序、空目录语义、
//! 深度与条目上限、协作取消、root 失败 vs 子项失败聚合、扫描中消失的竞态、
//! Unicode 名称、相对路径不逃逸。

use super::*;
use crate::fs::{FaultFilesystem, FsFault, StdFilesystem};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use weave_core::prelude::ScanStatus;
use weave_testkit::{TempWorkspace, standard_tree};

const SEP: char = '\\';

fn scan_default(ws: &TempWorkspace) -> Result<DirectoryScanReport, WeaveError> {
    let raw = ws.path().to_string_lossy().into_owned();
    scan_directory(
        &StdFilesystem,
        &raw,
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |_| {},
    )
}

#[test]
fn golden_tree_counts_sizes_and_depth_match_fixture() {
    let ws = TempWorkspace::new("scan-golden").expect("ws");
    let expected = standard_tree(&ws).expect("fixture");
    let report = scan_default(&ws).expect("scan");

    assert_eq!(report.status, ScanStatus::Completed);
    assert_eq!(report.files_scanned, expected.files);
    assert_eq!(report.directories_scanned, expected.directories);
    assert_eq!(report.total_size, expected.total_size);
    // root(0) → nested(1) → e(2) → f(3)：观察到的最大深度是 3。
    assert_eq!(report.max_depth, 3);
    assert_eq!(report.other_entries, 0);
    assert_eq!(report.error_count, 0);
    assert_eq!(report.status, ScanStatus::Completed);
    assert!(report.warnings.is_empty());
}

#[test]
fn file_type_distribution_content_and_sorting() {
    let ws = TempWorkspace::new("scan-dist").expect("ws");
    standard_tree(&ws).expect("fixture");
    let report = scan_default(&ws).expect("scan");
    let dist = &report.file_type_distribution;
    let find = |label: &str| {
        dist.iter()
            .find(|(k, _)| k == label)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    };
    // text = a.txt + b.md + g.txt + UPPER.TXT + 中文 文件.txt = 5
    assert_eq!(find("text"), 5);
    assert_eq!(find("archive"), 1); // old.zip
    // picture.bin（.bin 不在表中→unknown）、.hidden、noext → unknown ×3
    assert_eq!(find("unknown"), 3);
    for pair in dist.windows(2) {
        assert!(
            pair[0].1 > pair[1].1 || (pair[0].1 == pair[1].1 && pair[0].0 < pair[1].0),
            "distribution must be count DESC then label ASC"
        );
    }
}

#[test]
fn largest_files_order_by_size_desc() {
    let ws = TempWorkspace::new("scan-largest").expect("ws");
    standard_tree(&ws).expect("fixture");
    let report = scan_default(&ws).expect("scan");
    let sizes: Vec<u64> = report.largest_files.iter().map(|f| f.size).collect();
    let mut sorted = sizes.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(sizes, sorted);
    let first = &report.largest_files[0];
    assert_eq!(first.size, 40);
    assert_eq!(first.relative_path, format!("archives{SEP}old.zip"));
}

#[test]
fn empty_directory_semantics_direct_children_only() {
    let ws = TempWorkspace::new("scan-empty").expect("ws");
    ws.dir("empty").expect("mkdir");
    ws.dir("outer").expect("mkdir");
    ws.dir("outer/inner").expect("mkdir");
    let report = scan_default(&ws).expect("scan");
    let empties = &report.empty_directories;
    assert!(empties.contains(&String::from("empty")));
    assert!(empties.contains(&format!("outer{SEP}inner")));
    assert!(!empties.contains(&String::from("outer")));
    assert!(!report.empty_directories_truncated);
}

#[test]
fn depth_limit_marks_limited_and_stops_recursion() {
    let ws = TempWorkspace::new("scan-depth").expect("ws");
    standard_tree(&ws).expect("fixture");
    let raw = ws.path().to_string_lossy().into_owned();
    let report = scan_directory(
        &StdFilesystem,
        &raw,
        &ScanOptions {
            max_depth: Some(1),
            max_entries: None,
        },
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("scan");
    assert!(report.limited);
    assert!(
        report
            .limited_reason
            .as_deref()
            .expect("reason required")
            .contains("depth")
    );
    assert_eq!(report.status, ScanStatus::CompletedWithWarnings);
    // 深度 2（nested/e）之下不再递归：g.txt 不可见 → 9 - 1 = 8。
    assert_eq!(report.files_scanned, 8);
    assert!(
        !report
            .largest_files
            .iter()
            .any(|f| f.relative_path.contains("g.txt"))
    );
}

#[test]
fn entry_limit_marks_limited_with_exact_count() {
    let ws = TempWorkspace::new("scan-limit").expect("ws");
    standard_tree(&ws).expect("fixture");
    let raw = ws.path().to_string_lossy().into_owned();
    let report = scan_directory(
        &StdFilesystem,
        &raw,
        &ScanOptions {
            max_depth: None,
            max_entries: Some(3),
        },
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("scan");
    assert!(report.limited);
    assert_eq!(report.entries_processed, 3);
    assert_eq!(report.status, ScanStatus::CompletedWithWarnings);
}

#[test]
fn cancellation_mid_scan_returns_cancelled_partial_facts() {
    let ws = TempWorkspace::new("scan-cancel").expect("ws");
    standard_tree(&ws).expect("fixture");
    // 进度回调每 512 个条目触发一次——用 600 个小文件保证中途取消真正发生。
    for i in 0..600 {
        ws.file(&format!("bulk{SEP}f{i:03}.txt"), "x")
            .expect("write");
    }
    ws.dir("bulk").expect("mkdir");
    let cancel = CancellationToken::new();
    let first = AtomicUsize::new(0);
    let raw = ws.path().to_string_lossy().into_owned();
    let report = scan_directory(
        &StdFilesystem,
        &raw,
        &ScanOptions::default(),
        &cancel,
        &mut |_p| {
            if first.fetch_add(1, Ordering::SeqCst) == 0 {
                cancel.cancel();
            }
        },
    )
    .expect("cancelled scan is Ok");
    assert_eq!(report.status, ScanStatus::Cancelled);
    assert!(
        report.entries_processed < 601,
        "cancel must stop the walk early"
    );
}

#[test]
fn missing_root_and_file_root_are_distinct_structured_errors() {
    let ws = TempWorkspace::new("scan-root").expect("ws");
    ws.file("plain.txt", "x").expect("write");

    let missing = ws.path().join("ghost").to_string_lossy().into_owned();
    let err = scan_directory(
        &StdFilesystem,
        &missing,
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect_err("missing root");
    assert_eq!(err.code, "path.notFound");

    let file_root = ws.path().join("plain.txt").to_string_lossy().into_owned();
    let err = scan_directory(
        &StdFilesystem,
        &file_root,
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect_err("file root");
    assert_eq!(err.code, "scan.notDirectory");
}

#[test]
fn root_read_failure_fails_whole_scan() {
    let ws = TempWorkspace::new("scan-fault-root").expect("ws");
    standard_tree(&ws).expect("fixture");
    let raw = ws.path().to_string_lossy().into_owned();

    let root_denied = FaultFilesystem::wrapping(Box::new(StdFilesystem));
    root_denied.arm(FsFault::ReadDirDenied);
    let err = scan_directory(
        &root_denied,
        &raw,
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect_err("root failure fails whole scan");
    assert_eq!(err.code, "scan.rootReadFailed");
    assert_eq!(root_denied.trips(FsFault::ReadDirDenied), 1);
}

#[test]
fn child_read_failure_aggregates_and_continues() {
    use crate::fs::Filesystem as Fs;
    struct ChildDeny {
        root: PathBuf,
    }
    impl Fs for ChildDeny {
        fn exists(&self, p: &Path) -> bool {
            StdFilesystem.exists(p)
        }
        fn stat(&self, p: &Path) -> io::Result<crate::metadata::FileStat> {
            StdFilesystem.stat(p)
        }
        fn read_dir(&self, p: &Path) -> io::Result<Vec<String>> {
            if p == self.root {
                StdFilesystem.read_dir(p)
            } else {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
        }
        fn open_read(&self, p: &Path) -> io::Result<Box<dyn crate::fs::ReadSeek>> {
            StdFilesystem.open_read(p)
        }
        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            StdFilesystem.rename(from, to)
        }
        fn create_dir_all(&self, path: &Path) -> io::Result<()> {
            StdFilesystem.create_dir_all(path)
        }
    }

    let ws = TempWorkspace::new("scan-fault-child").expect("ws");
    standard_tree(&ws).expect("fixture");
    let raw = ws.path().to_string_lossy().into_owned();
    let report = scan_directory(
        &ChildDeny {
            root: ws.path().to_path_buf(),
        },
        &raw,
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("child failure aggregates");
    assert_eq!(report.status, ScanStatus::CompletedWithWarnings);
    assert!(report.error_count >= 1);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.code == "path.permissionDenied")
    );
    // root 直属的 3 个文件（.hidden / noext / UPPER.TXT）不受子目录权限影响。
    assert_eq!(report.files_scanned, 3);
    assert!(report.directories_scanned >= 6);
    assert_eq!(report.total_size, 1 + 2 + 3);
}

#[test]
fn file_deleted_during_scan_is_recorded_not_silent() {
    use crate::fs::Filesystem as Fs;
    /// read_dir 报告 racy.txt，stat 却 NotFound：扫描中途消失的竞态替身。
    struct VanishFs {
        root: PathBuf,
        stats: AtomicUsize,
    }
    impl Fs for VanishFs {
        fn exists(&self, p: &Path) -> bool {
            StdFilesystem.exists(p)
        }
        fn stat(&self, p: &Path) -> io::Result<crate::metadata::FileStat> {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned());
            if name.as_deref() == Some("racy.txt")
                && p.parent() == Some(self.root.as_path())
                && self.stats.fetch_add(1, Ordering::SeqCst) >= 1
            {
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
            StdFilesystem.stat(p)
        }
        fn read_dir(&self, p: &Path) -> io::Result<Vec<String>> {
            let mut names = StdFilesystem.read_dir(p)?;
            if p == self.root && !names.iter().any(|n| n == "racy.txt") {
                names.push("racy.txt".to_string());
            }
            Ok(names)
        }
        fn open_read(&self, p: &Path) -> io::Result<Box<dyn crate::fs::ReadSeek>> {
            StdFilesystem.open_read(p)
        }
        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            StdFilesystem.rename(from, to)
        }
        fn create_dir_all(&self, path: &Path) -> io::Result<()> {
            StdFilesystem.create_dir_all(path)
        }
    }

    let ws = TempWorkspace::new("scan-race").expect("ws");
    ws.file("stable.txt", "s").expect("write");
    let raw = ws.path().to_string_lossy().into_owned();
    let report = scan_directory(
        &VanishFs {
            root: ws.path().to_path_buf(),
            stats: AtomicUsize::new(0),
        },
        &raw,
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("race aggregates, not fatal");
    assert_eq!(report.status, ScanStatus::CompletedWithWarnings);
    assert_eq!(report.error_count, 1);
    assert_eq!(report.files_scanned, 1, "stable file still counted");
    assert!(report.errors[0].relative_path.ends_with("racy.txt"));
    assert_eq!(report.errors[0].code, "path.notFound");
}

#[test]
fn unicode_names_are_preserved_in_reports() {
    let ws = TempWorkspace::new("scan-unicode").expect("ws");
    standard_tree(&ws).expect("fixture");
    let report = scan_default(&ws).expect("scan");
    assert!(
        report
            .largest_files
            .iter()
            .any(|f| f.relative_path.contains("中文 文件.txt"))
    );
}

#[test]
fn relative_paths_never_escape_root() {
    let ws = TempWorkspace::new("scan-invariant").expect("ws");
    standard_tree(&ws).expect("fixture");
    let report = scan_default(&ws).expect("scan");
    for file in report
        .largest_files
        .iter()
        .chain(report.oldest_files.iter())
        .chain(report.newest_files.iter())
    {
        assert!(!file.relative_path.starts_with(".."));
        assert!(!file.relative_path.starts_with('/'));
    }
    for empty in &report.empty_directories {
        assert!(!empty.starts_with(".."));
    }
}

#[test]
fn progress_stream_never_fakes_a_total() {
    let ws = TempWorkspace::new("scan-progress").expect("ws");
    standard_tree(&ws).expect("fixture");
    let raw = ws.path().to_string_lossy().into_owned();
    let mut progress_events: Vec<Progress> = Vec::new();
    let report = scan_directory(
        &StdFilesystem,
        &raw,
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |p| progress_events.push(p),
    )
    .expect("scan");
    // 小树低于 512 阈值 → 无进度事件；一旦有事件，total 必须是 None（诚实未知）。
    for p in &progress_events {
        assert!(p.total.is_none(), "scan totals are unknowable upfront");
    }
    assert!(report.entries_processed >= progress_events.len() as u64 * 512);
}
