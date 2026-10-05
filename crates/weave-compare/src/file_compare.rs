//! 文件比较引擎（M2.1 §3/§8/§10/§14）。
//!
//! 分层策略（§8）：existence → size → (mtime) → SHA-256 流式 → 文本详情
//! 按需。绝不默认整读大文件（§9）：hash 是流式的，文本详情有尺寸上限，
//! 超限显式 TooLarge 而非假装比较成功。
//!
//! Read-only：本模块不产生任何文件系统修改（§14）。

use std::io::Read as _;
use std::path::Path;
use std::time::SystemTime;

use weave_core::prelude::{CancellationToken, TextEncoding, WeaveError};
use weave_files::fs::Filesystem;

use crate::model::{
    ComparisonEvidence, ComparisonSession, ContentDifference, DifferenceItem, DifferenceKind,
    EntryFacts, NodeKind, SessionKind,
};
use crate::options::{CompareOptions, ContentCheck};

const HEAD_SAMPLE_BYTES: usize = 64 * 1024;

/// 比较两个文件（或一侧缺失/类型不一致的诚实降级）。
pub fn compare_files(
    fs: &dyn Filesystem,
    left: &Path,
    right: &Path,
    options: &CompareOptions,
    cancel: &CancellationToken,
) -> ComparisonSession {
    let mut builder = ComparisonSession::builder(
        &left.to_string_lossy(),
        &right.to_string_lossy(),
        SessionKind::Files,
    );
    builder.effective_ignores(options.ignore.effective());

    let left_stat = fs.stat(left);
    let right_stat = fs.stat(right);

    // 一侧缺失（NotFound）⇒ Removed/Added（诚实映射），其余 stat 错误 ⇒ Unreadable
    let left_missing = matches!(&left_stat, Err(e) if e.kind() == std::io::ErrorKind::NotFound);
    let right_missing = matches!(&right_stat, Err(e) if e.kind() == std::io::ErrorKind::NotFound);

    let item = if left_missing || right_missing {
        let present = if left_missing {
            (&right_stat, right, false)
        } else {
            (&left_stat, left, true)
        };
        match present {
            (Ok(pstat), ppath, on_left) => {
                let mut item = DifferenceItem::new(
                    display_name(left, right),
                    node_of(pstat),
                    if on_left {
                        DifferenceKind::Removed
                    } else {
                        DifferenceKind::Added
                    },
                );
                if on_left {
                    item.left = Some(facts_from_stat(ppath, pstat, None));
                } else {
                    item.right = Some(facts_from_stat(ppath, pstat, None));
                }
                item
            }
            (Err(e), _, _) => {
                let mut item = DifferenceItem::new(
                    display_name(left, right),
                    NodeKind::File,
                    DifferenceKind::Error,
                );
                item.error = Some(
                    WeaveError::from_io(e, "compare", "cannot stat compare target")
                        .with_location("weave-compare::file"),
                );
                item
            }
        }
    } else {
        match (left_stat, right_stat) {
            (Err(e), _) | (_, Err(e)) => {
                let mut item = DifferenceItem::new(
                    display_name(left, right),
                    NodeKind::File,
                    DifferenceKind::Unreadable,
                );
                item.error = Some(
                    WeaveError::from_io(&e, "compare", "cannot stat compare target")
                        .with_location("weave-compare::file"),
                );
                item
            }
            (Ok(ls), Ok(rs)) => compare_paired_files(fs, left, right, ls, rs, options, cancel),
        }
    };
    builder.push(item);
    builder.finish()
}

fn compare_paired_files(
    fs: &dyn Filesystem,
    left: &Path,
    right: &Path,
    ls: weave_files::metadata::FileStat,
    rs: weave_files::metadata::FileStat,
    options: &CompareOptions,
    cancel: &CancellationToken,
) -> DifferenceItem {
    let rel = display_name(left, right);
    let left_node = node_of(&ls);
    let right_node = node_of(&rs);

    if left_node != right_node {
        let mut item = DifferenceItem::new(&rel, left_node, DifferenceKind::TypeChanged);
        item.left = Some(facts_from_stat(left, &ls, None));
        item.right = Some(facts_from_stat(right, &rs, None));
        return item;
    }
    if left_node != NodeKind::File {
        let mut item = DifferenceItem::new(&rel, left_node, DifferenceKind::Unsupported);
        item.error = Some(
            WeaveError::unsupported(
                "compare.nonFilePair",
                "only regular file pairs are compared here; use directory compare for folders",
            )
            .with_location("weave-compare::file"),
        );
        item.left = Some(facts_from_stat(left, &ls, None));
        item.right = Some(facts_from_stat(right, &rs, None));
        return item;
    }

    // ── L1：size（不同 ⇒ 内容必不同，无需 hash）──
    if ls.size != rs.size {
        let mut item = DifferenceItem::new(&rel, NodeKind::File, DifferenceKind::Modified);
        item.content = ContentDifference::ContentChanged;
        item.left = Some(facts_from_stat(left, &ls, None));
        item.right = Some(facts_from_stat(right, &rs, None));
        item.evidence = Some(ComparisonEvidence {
            method: "size".to_string(),
            left_digest: None,
            right_digest: None,
            compared_at: SystemTime::now(),
        });
        return item;
    }

    // ── L2：内容核对（Metadata 档按 mtime；Hash 档流式 SHA-256）──
    let (kind, content, method, ld, rd) = match options.content_check {
        ContentCheck::Metadata => {
            let same_mtime = ls.modified.is_some() && ls.modified == rs.modified;
            if same_mtime {
                (
                    DifferenceKind::Same,
                    ContentDifference::NotCompared,
                    "size+mtime",
                    None,
                    None,
                )
            } else {
                (
                    DifferenceKind::Modified,
                    ContentDifference::MetadataOnly,
                    "size+mtime",
                    None,
                    None,
                )
            }
        }
        ContentCheck::Hash => {
            let left_digest = stream_digest(fs, left, cancel);
            let right_digest = stream_digest(fs, right, cancel);
            match (left_digest, right_digest) {
                (Ok(ld), Ok(rd)) => {
                    // Hash 档：内容是事实标准——digest 相等 ⇒ Same（mtime
                    // 差异保留在两侧 facts 里供 UI 提示，不升级为 Modified；
                    // 文件比较对象通常是两份不同文件，时间天然不同）。
                    if ld == rd {
                        (
                            DifferenceKind::Same,
                            ContentDifference::None,
                            "SHA-256",
                            Some(ld),
                            Some(rd),
                        )
                    } else {
                        (
                            DifferenceKind::Modified,
                            ContentDifference::ContentChanged,
                            "SHA-256",
                            Some(ld),
                            Some(rd),
                        )
                    }
                }
                (Err(e), _) | (_, Err(e)) => {
                    let mut item = DifferenceItem::new(&rel, NodeKind::File, DifferenceKind::Error);
                    item.left = Some(facts_from_stat(left, &ls, None));
                    item.right = Some(facts_from_stat(right, &rs, None));
                    item.error = Some(e);
                    return item;
                }
            }
        }
    };

    let mut item = DifferenceItem::new(&rel, NodeKind::File, kind);
    item.content = content;
    // 编码/行数：可可靠识别时的补充事实（§3/§10；复用 weave-files 检测）
    item.left = Some(enriched_facts(fs, left, &ls, ld.as_deref(), cancel));
    item.right = Some(enriched_facts(fs, right, &rs, rd.as_deref(), cancel));
    item.evidence = Some(ComparisonEvidence {
        method: method.to_string(),
        left_digest: ld,
        right_digest: rd,
        compared_at: SystemTime::now(),
    });
    item
}

/// 文本详情（按需 L3，M2.1 §17/§18）：行级 diff 复用 weave-text::compare
/// （有界 LCS + 超限诚实 TooLarge；CRLF 差异在行体归一中不误判整文件变化）。
pub struct TextDetail {
    pub left_encoding: TextEncoding,
    pub right_encoding: TextEncoding,
    /// 换行风格差异（内容逐行一致时 UI 可提示"仅换行符不同"）。
    pub line_ending_differs: bool,
    pub report: weave_text::compare::DiffReport,
    /// 超限：report 为空，如实标注（M2.1 §9）。
    pub too_large: bool,
}

pub fn text_detail(
    fs: &dyn Filesystem,
    left: &Path,
    right: &Path,
    options: &CompareOptions,
    cancel: &CancellationToken,
) -> Result<TextDetail, WeaveError> {
    let ls = fs.stat(left).map_err(|e| {
        WeaveError::from_io(&e, "compare", "cannot stat left").with_location("weave-compare::file")
    })?;
    let rs = fs.stat(right).map_err(|e| {
        WeaveError::from_io(&e, "compare", "cannot stat right").with_location("weave-compare::file")
    })?;
    if ls.size > options.max_text_detail_bytes || rs.size > options.max_text_detail_bytes {
        return Ok(TextDetail {
            left_encoding: TextEncoding::Unknown,
            right_encoding: TextEncoding::Unknown,
            line_ending_differs: false,
            report: weave_text::compare::DiffReport {
                hunks: Vec::new(),
                stats: weave_text::compare::DiffStats::default(),
                identical: false,
                degraded: false,
            },
            too_large: true,
        });
    }
    let left_bytes = read_all(fs, left, cancel)?;
    let right_bytes = read_all(fs, right, cancel)?;
    let left_encoding = weave_files::encoding::detect_encoding(&left_bytes);
    let right_encoding = weave_files::encoding::detect_encoding(&right_bytes);
    let left_text = decode_lossless(&left_bytes, left_encoding)?;
    let right_text = decode_lossless(&right_bytes, right_encoding)?;
    let line_ending_differs = line_ending_style(&left_bytes) != line_ending_style(&right_bytes);
    let limits = weave_text::compare::CompareLimits::default();
    let wopts = weave_text::compare::CompareOptions {
        whitespace: weave_text::compare::WhitespaceMode::None,
        ignore_case: false,
    };
    let report = weave_text::compare::compare_texts(&left_text, &right_text, &wopts, &limits)
        .map_err(|e| {
            WeaveError::io("compare.textDiffFailed", format!("text diff failed: {e:?}"))
                .with_location("weave-compare::file")
        })?;
    Ok(TextDetail {
        left_encoding,
        right_encoding,
        line_ending_differs,
        report,
        too_large: false,
    })
}

// ── 共享辅助 ──

pub(crate) fn node_of(stat: &weave_files::metadata::FileStat) -> NodeKind {
    match stat.kind {
        weave_core::prelude::FileKind::Directory => NodeKind::Directory,
        weave_core::prelude::FileKind::Symlink => NodeKind::Symlink,
        _ => NodeKind::File,
    }
}

pub(crate) fn facts_from_stat(
    path: &Path,
    stat: &weave_files::metadata::FileStat,
    digest_hex: Option<&str>,
) -> EntryFacts {
    EntryFacts {
        path: path.to_string_lossy().into_owned(),
        size: Some(stat.size),
        modified: stat.modified,
        digest_hex: digest_hex.map(str::to_string),
        encoding: None,
        line_count: None,
        child_counts: None,
    }
}

/// 编码 + 行数补充事实（best-effort：失败静默缺省，不制造错误条目）。
fn enriched_facts(
    fs: &dyn Filesystem,
    path: &Path,
    stat: &weave_files::metadata::FileStat,
    digest_hex: Option<&str>,
    cancel: &CancellationToken,
) -> EntryFacts {
    let mut facts = facts_from_stat(path, stat, digest_hex);
    let head = read_head(fs, path, HEAD_SAMPLE_BYTES, cancel).unwrap_or_default();
    let encoding = weave_files::encoding::detect_encoding(&head);
    facts.encoding = Some(encoding);
    if is_probably_text(encoding, &head) {
        facts.line_count = count_lines(fs, path, cancel).ok();
    }
    facts
}

pub(crate) fn is_probably_text(encoding: TextEncoding, head: &[u8]) -> bool {
    if matches!(
        encoding,
        TextEncoding::Unknown | TextEncoding::Utf16Le | TextEncoding::Utf16Be
    ) {
        // UTF-16 无 BOM 时被 Unknown 吞掉；有 BOM 的 UTF-16 也按"非逐字节
        // 文本"处理——行数统计留给后续需要时的解码实现。
        return false;
    }
    !head.contains(&0u8)
}

pub(crate) fn stream_digest(
    fs: &dyn Filesystem,
    path: &Path,
    cancel: &CancellationToken,
) -> Result<String, WeaveError> {
    let result = weave_files::hash::hash_file(fs, path, cancel, &mut |_| {})?;
    result.digest_hex.map_or_else(
        || {
            Err(WeaveError::io(
                "compare.hashUnavailable",
                format!("hash unavailable for {}", path.display()),
            )
            .with_location("weave-compare::file"))
        },
        Ok,
    )
}

pub(crate) fn read_head(
    fs: &dyn Filesystem,
    path: &Path,
    cap: usize,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, WeaveError> {
    let mut reader = fs.open_read(path).map_err(|e| {
        WeaveError::from_io(&e, "compare", format!("cannot read {}", path.display()))
            .with_location("weave-compare::file")
    })?;
    let mut buf = vec![0u8; cap];
    let mut filled = 0usize;
    while filled < cap {
        if cancel.is_cancelled() {
            return Err(
                WeaveError::cancelled("compare.cancelled", "cancelled while reading")
                    .with_location("weave-compare::file"),
            );
        }
        let n = reader.read(&mut buf[filled..]).map_err(|e| {
            WeaveError::from_io(&e, "compare", format!("read failed: {}", path.display()))
                .with_location("weave-compare::file")
        })?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    buf.truncate(filled);
    Ok(buf)
}

fn read_all(
    fs: &dyn Filesystem,
    path: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, WeaveError> {
    let mut reader = fs.open_read(path).map_err(|e| {
        WeaveError::from_io(&e, "compare", format!("cannot read {}", path.display()))
            .with_location("weave-compare::file")
    })?;
    let mut out = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        if cancel.is_cancelled() {
            return Err(
                WeaveError::cancelled("compare.cancelled", "cancelled while reading")
                    .with_location("weave-compare::file"),
            );
        }
        let n = reader.read(&mut chunk).map_err(|e: std::io::Error| {
            WeaveError::from_io(&e, "compare", format!("read failed: {}", path.display()))
                .with_location("weave-compare::file")
        })?;
        if n == 0 {
            return Ok(out);
        }
        out.extend_from_slice(&chunk[..n]);
    }
}

pub(crate) fn count_lines(
    fs: &dyn Filesystem,
    path: &Path,
    cancel: &CancellationToken,
) -> Result<u64, WeaveError> {
    use std::io::Read;
    let mut reader = fs.open_read(path).map_err(|e| {
        WeaveError::from_io(&e, "compare", format!("cannot read {}", path.display()))
            .with_location("weave-compare::file")
    })?;
    let mut count = 0u64;
    let mut last = 0u8;
    let mut buf = [0u8; 64 * 1024];
    loop {
        if cancel.is_cancelled() {
            return Err(
                WeaveError::cancelled("compare.cancelled", "cancelled while counting")
                    .with_location("weave-compare::file"),
            );
        }
        let n = reader.read(&mut buf).map_err(|e| {
            WeaveError::from_io(&e, "compare", format!("read failed: {}", path.display()))
                .with_location("weave-compare::file")
        })?;
        if n == 0 {
            break;
        }
        count += buf[..n].iter().filter(|&&b| b == b'\n').count() as u64;
        last = buf[n - 1];
    }
    // 末行无换行符也算一行
    if last != b'\n' && last != 0 {
        count += 1;
    }
    Ok(count)
}

fn line_ending_style(bytes: &[u8]) -> u8 {
    if let Some(pos) = bytes.iter().position(|&b| b == b'\n') {
        if pos > 0 && bytes[pos - 1] == b'\r' {
            b'C'
        } else {
            b'L'
        }
    } else if bytes.contains(&b'\r') {
        b'R'
    } else {
        b'N'
    }
}

fn decode_lossless(bytes: &[u8], encoding: TextEncoding) -> Result<String, WeaveError> {
    match encoding {
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => String::from_utf16(
            &bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| {
                    if encoding == TextEncoding::Utf16Be {
                        u16::from_be_bytes([c[0], c[1]])
                    } else {
                        u16::from_le_bytes([c[0], c[1]])
                    }
                })
                .collect::<Vec<u16>>(),
        )
        .map_err(|e| {
            WeaveError::io("compare.decodeFailed", format!("utf16 decode failed: {e}"))
                .with_location("weave-compare::file")
        }),
        _ => Ok(String::from_utf8_lossy(bytes).into_owned()),
    }
}

fn display_name(left: &Path, right: &Path) -> String {
    left.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .or_else(|| right.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "<unnamed>".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{CompareOptions, ContentCheck};
    use weave_files::fs::StdFilesystem;
    use weave_testkit::TempWorkspace;

    fn opts(check: ContentCheck) -> CompareOptions {
        CompareOptions {
            content_check: check,
            ..CompareOptions::default()
        }
    }

    #[test]
    fn identical_files_are_same_with_hash_evidence() {
        let ws = TempWorkspace::new("cmp-file-same").expect("ws");
        let a = ws.file("a.txt", "identical").expect("write");
        let b = ws.file("b.txt", "identical").expect("write");
        let session = compare_files(
            &StdFilesystem,
            &a,
            &b,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
        );
        assert_eq!(session.summary.same, 1);
        let item = &session.items[0];
        assert_eq!(item.content, ContentDifference::None);
        assert_eq!(item.evidence.as_ref().expect("evidence").method, "SHA-256");
        assert!(item.left.as_ref().expect("left").line_count.is_some());
    }

    #[test]
    fn size_difference_is_modified_without_hash_cost() {
        let ws = TempWorkspace::new("cmp-file-size").expect("ws");
        let a = ws.file("a.txt", "short").expect("write");
        let b = ws.file("b.txt", "much longer content").expect("write");
        let session = compare_files(
            &StdFilesystem,
            &a,
            &b,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
        );
        assert_eq!(session.summary.modified, 1);
        assert_eq!(session.items[0].content, ContentDifference::ContentChanged);
        assert_eq!(
            session.items[0].evidence.as_ref().expect("evidence").method,
            "size"
        );
    }

    #[test]
    fn same_size_different_content_needs_hash_to_detect() {
        let ws = TempWorkspace::new("cmp-file-hash").expect("ws");
        let a = ws.file("a.txt", "AAAABBBB").expect("write");
        let b = ws.file("b.txt", "AAAACCCC").expect("write");
        // Metadata 档：size+mtime 同（同刻写入）⇒ 只能判 Same/mtime 差
        let meta = compare_files(
            &StdFilesystem,
            &a,
            &b,
            &opts(ContentCheck::Metadata),
            &CancellationToken::new(),
        );
        assert_ne!(meta.summary.modified + meta.summary.same, 0);
        // Hash 档：hash 抓住同 size 内容差异
        let hashed = compare_files(
            &StdFilesystem,
            &a,
            &b,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
        );
        assert_eq!(hashed.summary.modified, 1);
        assert_eq!(hashed.items[0].content, ContentDifference::ContentChanged);
    }

    #[test]
    fn missing_side_maps_to_removed_and_added() {
        let ws = TempWorkspace::new("cmp-file-missing").expect("ws");
        let a = ws.file("a.txt", "only left").expect("write");
        let missing = ws.path().join("ghost.txt");
        let s1 = compare_files(
            &StdFilesystem,
            &a,
            &missing,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
        );
        assert_eq!(s1.summary.removed, 1, "left only = removed");
        let s2 = compare_files(
            &StdFilesystem,
            &missing,
            &a,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
        );
        assert_eq!(s2.summary.added, 1, "right only = added");
    }

    #[test]
    fn file_vs_directory_is_type_changed() {
        let ws = TempWorkspace::new("cmp-file-type").expect("ws");
        let a = ws.file("entry", "content").expect("write");
        let d = ws.dir("entry2").expect("dir");
        let s = compare_files(
            &StdFilesystem,
            &a,
            &d,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
        );
        assert_eq!(s.summary.type_changed, 1);
    }

    #[test]
    fn text_detail_reports_hunks_and_line_endings() {
        let ws = TempWorkspace::new("cmp-file-detail").expect("ws");
        let a = ws.file("a.txt", "line1\nline2\nline3\n").expect("write");
        let b = ws.file("b.txt", "line1\nlineX\nline3\n").expect("write");
        let detail = text_detail(
            &StdFilesystem,
            &a,
            &b,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
        )
        .expect("detail");
        assert!(!detail.too_large);
        assert!(
            detail.report.stats.added + detail.report.stats.removed + detail.report.stats.changed
                > 0,
            "hunks present"
        );
        assert!(!detail.line_ending_differs);
    }

    #[test]
    fn text_detail_too_large_is_honest() {
        let ws = TempWorkspace::new("cmp-file-toolarge").expect("ws");
        let big = "x".repeat(64 * 1024);
        let a = ws.file("a.txt", &big).expect("write");
        let b = ws.file("b.txt", &big).expect("write");
        let o = CompareOptions {
            max_text_detail_bytes: 1024,
            ..CompareOptions::default()
        };
        let detail =
            text_detail(&StdFilesystem, &a, &b, &o, &CancellationToken::new()).expect("detail");
        assert!(detail.too_large, "must report too-large honestly");
    }

    #[test]
    fn cancellation_propagates_as_cancelled_error() {
        let ws = TempWorkspace::new("cmp-file-cancel").expect("ws");
        let content = "y".repeat(1024);
        let a = ws.file("a.txt", &content).expect("write");
        let b = ws.file("b.txt", &content).expect("write");
        let cancel = CancellationToken::new();
        cancel.cancel();
        let session = compare_files(&StdFilesystem, &a, &b, &opts(ContentCheck::Hash), &cancel);
        assert_eq!(
            session.summary.error, 1,
            "cancelled hash = error item, not session failure"
        );
    }
}
