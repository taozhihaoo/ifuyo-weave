//! M2.12 事实采集（§56 单遍扫描）：走输入路径一次，逐文件采集 FileFacts。
//!
//! - 递归（目录输入）+ 文件输入；上限 MAX_FILES=5000（§43 资源限制）。
//! - 图片 EXIF 日期：jpg/tiff ≤50MB 用 kamadak-exif（与 weave-media 同一
//!   依赖的只读 EXIF 段解析——不做全图解码 §44 Metadata→缩略图→OCR 顺序）。
//! - 文档 title：weave-documents::inspect_document（复用 M2.x parser §46）。
//! - partial_hash：仅 Cleanup 模式且 size>0（与 M3 同一原语 §2）。
//! - 单文件失败 ⇒ 记入 errors，不中断（§41 失败隔离）。

use std::path::{Path, PathBuf};

use weave_core::prelude::{CancellationToken, WeaveError};

use super::rules::FileFacts;

pub const MAX_FILES: usize = 5000;
const MAX_EXIF_BYTES: u64 = 50 * 1024 * 1024;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("smart::facts")
}

fn now_millis(ms: std::time::SystemTime) -> f64 {
    ms.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// 采集结果：(facts, (path, error-code) 列表)。
type CollectOutput = (Vec<FileFacts>, Vec<(String, String)>);

/// 采集入口。返回 (facts, errors)；cancel 在每文件间检查（§42 真取消）。
pub fn collect_facts(
    inputs: &[String],
    want_hashes: bool,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(usize),
) -> Result<CollectOutput, WeaveError> {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    let mut visited = std::collections::HashSet::new();
    for raw in inputs {
        if cancel.is_cancelled() {
            break;
        }
        let path = PathBuf::from(raw.trim());
        if path.is_file() {
            push_file(
                &path,
                want_hashes,
                cancel,
                &mut files,
                &mut errors,
                on_progress,
            );
        } else if path.is_dir() {
            walk_dir(
                &path,
                want_hashes,
                cancel,
                &mut visited,
                &mut files,
                &mut errors,
                on_progress,
            )?;
        } else {
            errors.push((raw.clone(), "smart.inputMissing".to_string()));
        }
        if files.len() >= MAX_FILES {
            break;
        }
    }
    Ok((files, errors))
}

fn walk_dir(
    dir: &Path,
    want_hashes: bool,
    cancel: &CancellationToken,
    visited: &mut std::collections::HashSet<PathBuf>,
    files: &mut Vec<FileFacts>,
    errors: &mut Vec<(String, String)>,
    on_progress: &mut dyn FnMut(usize),
) -> Result<(), WeaveError> {
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(canonical) {
        return Ok(()); // 符号链接/重复输入防环（M1 同策略）
    }
    if files.len() >= MAX_FILES {
        return Ok(());
    }
    let read = std::fs::read_dir(dir)
        .map_err(|e| err("smart.dirUnreadable", format!("{}: {e}", dir.display())))?;
    for entry in read.flatten() {
        if cancel.is_cancelled() || files.len() >= MAX_FILES {
            return Ok(());
        }
        let path = entry.path();
        if path.is_dir() {
            walk_dir(
                &path,
                want_hashes,
                cancel,
                visited,
                files,
                errors,
                on_progress,
            )?;
        } else if path.is_file() {
            push_file(&path, want_hashes, cancel, files, errors, on_progress);
        }
    }
    Ok(())
}

fn push_file(
    path: &Path,
    want_hashes: bool,
    cancel: &CancellationToken,
    files: &mut Vec<FileFacts>,
    errors: &mut Vec<(String, String)>,
    on_progress: &mut dyn FnMut(usize),
) {
    let Ok(meta) = std::fs::metadata(path) else {
        errors.push((
            path.to_string_lossy().into_owned(),
            "smart.statFailed".to_string(),
        ));
        return;
    };
    if !meta.is_file() {
        return;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut facts = FileFacts {
        path: path.to_string_lossy().into_owned(),
        name,
        ext: ext.clone(),
        size: meta.len(),
        modified_ms: now_millis(meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)),
        meta_date: None,
        title: None,
        partial_hash: None,
    };
    // EXIF 日期（jpg/tiff ≤50MB；解析失败 ⇒ None 如实）
    if matches!(ext.as_str(), "jpg" | "jpeg" | "tiff" | "tif") && meta.len() <= MAX_EXIF_BYTES {
        facts.meta_date = read_exif_date(path);
    }
    // 文档 title（pdf/docx；inspect 失败 ⇒ None 如实）
    if matches!(ext.as_str(), "pdf" | "docx" | "xlsx" | "pptx") {
        facts.title = read_doc_title(path);
    }
    // partial hash（Cleanup 证据；非空文件）
    if want_hashes && facts.size > 0 {
        let fs = weave_files::fs::StdFilesystem;
        match weave_files::duplicates::partial_hash(&fs, path, facts.size) {
            Ok(h) => facts.partial_hash = Some(h),
            Err(e) => errors.push((facts.path.clone(), e.code)),
        }
    }
    files.push(facts);
    on_progress(files.len());
    let _ = cancel;
}

/// EXIF DateTimeOriginal/DateTime（只读 EXIF 段，不解码像素 §44）。
fn read_exif_date(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader).ok()?;
    let dt = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)?;
    // "2026:08:20 14:30:00" → "2026-08-20"
    let raw = dt.display_value().to_string();
    let date = raw.split(' ').next()?;
    let parts: Vec<&str> = date.split(':').collect();
    if parts.len() == 3 {
        Some(format!("{}-{}-{}", parts[0], parts[1], parts[2]))
    } else {
        None
    }
}

/// 文档 core-properties title（复用 M2.x parser §46）。
fn read_doc_title(path: &Path) -> Option<String> {
    let facts = weave_documents::inspect_document(
        path,
        &weave_documents::DocumentResourceLimits::default(),
    );
    facts
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("title"))
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
