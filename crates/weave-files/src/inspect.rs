//! File Inspector 服务（M1 §11，工具 `files.inspect`）。
//!
//! 设计要点：
//! - **默认不读文件内容**：stat → metadata 立即返回（M1 §11.3）；唯一的读取是
//!   有界嗅探（≤ [`SNIFF_LIMIT`] 字节），且只用于 RegularFile 的类型/编码判定。
//! - **错误 vs 警告分离**（M1 §11.2）：stat 失败 = 结构化错误（工具失败）；
//!   嗅探失败 = `Partial` + warning（元数据仍然可用）。
//! - Hash 不在这里：它是显式的昂贵操作，由独立命令/任务执行（M1 §34）。

use std::io::Read;
use std::path::PathBuf;
use std::time::SystemTime;

use weave_core::prelude::{FileKind, TextEncoding, WeaveError, validate_absolute_path};

use crate::classify::{
    Classification, ClassificationEvidence, category_with_encoding_hint, classify,
};
use crate::encoding::detect_encoding;
use crate::fs::Filesystem;

/// 嗅探上限：8 KiB。任何自动读取的内容都必须有边界（M1 §36）。
pub const SNIFF_LIMIT: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectOptions {
    /// 是否做有界嗅探（类型/编码判定）。扫描等纯 metadata 场景可关。
    pub sniff: bool,
}

impl Default for InspectOptions {
    fn default() -> Self {
        Self { sniff: true }
    }
}

/// 检查状态。硬失败走结构化错误（Err），软降级 = `Partial` + 警告——
/// 刻意没有 Failed/Cancelled 变体：检查是只读快速操作，取消不适用，
/// 无法进行的部分以警告表达（DECISIONS.md）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionStatus {
    Complete,
    Partial,
}

/// 一次检查的全部事实。
#[derive(Debug, Clone, PartialEq)]
pub struct FileInspection {
    pub status: InspectionStatus,
    /// 用户输入的原始路径（未改写）。
    pub requested: String,
    /// 校验/规范化后的绝对路径。
    pub normalized_path: String,
    pub name: String,
    pub extension: Option<String>,
    pub kind: FileKind,
    pub size: u64,
    pub readonly: bool,
    pub hidden: Option<bool>,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub classification: Classification,
    /// 仅在"类文本"场景给出（Text 类别或 BOM 命中）；不适用时 None。
    pub encoding: Option<TextEncoding>,
    pub warnings: Vec<String>,
}

/// 检查一个文件系统条目。
pub fn inspect_file(
    fs: &dyn Filesystem,
    raw_path: &str,
    options: &InspectOptions,
) -> Result<FileInspection, WeaveError> {
    let validation =
        validate_absolute_path(raw_path).map_err(|e| e.with_location("weave-files::inspect"))?;
    let target = PathBuf::from(&validation.normalized);

    let stat = fs.stat(&target).map_err(|e| {
        let message = format!("cannot stat '{}': {e}", validation.normalized);
        let base = match e.kind() {
            std::io::ErrorKind::NotFound => WeaveError::io("path.notFound", message),
            std::io::ErrorKind::PermissionDenied => {
                WeaveError::permission("path.permissionDenied", message)
            }
            _ => WeaveError::io("inspect.statFailed", message),
        };
        base.with_location("weave-files::inspect")
            .with_recoverability(weave_core::prelude::Recoverability::UserActionRequired)
    })?;

    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| validation.normalized.clone());
    let extension = target.extension().map(|e| e.to_string_lossy().into_owned());

    let mut warnings = Vec::new();
    let mut status = InspectionStatus::Complete;

    // 嗅探：只对 RegularFile、有内容、且选项允许时进行；任何失败降级为警告。
    let mut sniffed: Option<Vec<u8>> = None;
    if options.sniff && stat.kind == FileKind::RegularFile && stat.size > 0 {
        match read_sniff_sample(fs, &target) {
            Ok(sample) => sniffed = Some(sample),
            Err(e) => {
                warnings.push(format!("type/encoding sniff failed: {e}"));
                status = InspectionStatus::Partial;
            }
        }
    }

    let sample_bytes = sniffed.as_deref();
    let mut classification = if stat.kind == FileKind::Directory {
        // 目录没有有意义的扩展名分类事实；Unknown 是正常状态（M1 §38）。
        Classification {
            category: crate::classify::FileCategory::Unknown,
            evidence: ClassificationEvidence::None,
            mime: None,
        }
    } else {
        classify(extension.as_deref(), sample_bytes)
    };

    // 编码：仅"类文本"场景（Text 类别或 BOM 命中）才给出，避免对二进制噪声。
    let mut encoding: Option<TextEncoding> = None;
    if let Some(sample) = sample_bytes {
        let detected = detect_encoding(sample);
        let bom_hit = !matches!(
            detected,
            TextEncoding::Unknown | TextEncoding::Ascii | TextEncoding::Utf8
        );
        if classification.category == crate::classify::FileCategory::Text || bom_hit {
            encoding = Some(detected);
            classification = category_with_encoding_hint(classification, detected);
        }
    }

    if stat.kind == FileKind::Symlink {
        warnings.push(
            "symlink entry: facts describe the link itself; the target is not followed (M1 symlink policy)"
                .to_string(),
        );
    }

    Ok(FileInspection {
        status,
        requested: raw_path.to_string(),
        normalized_path: validation.normalized,
        name,
        extension,
        kind: stat.kind,
        size: stat.size,
        readonly: stat.readonly,
        hidden: stat.hidden,
        created: stat.created,
        modified: stat.modified,
        accessed: stat.accessed,
        classification,
        encoding,
        warnings,
    })
}

/// 读取 ≤ [`SNIFF_LIMIT`] 字节的文件头部样本。
fn read_sniff_sample(fs: &dyn Filesystem, target: &std::path::Path) -> std::io::Result<Vec<u8>> {
    let mut reader = fs.open_read(target)?;
    let mut sample = Vec::with_capacity(SNIFF_LIMIT);
    let mut chunk = [0u8; 1024];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        let remaining = SNIFF_LIMIT - sample.len();
        sample.extend_from_slice(&chunk[..n.min(remaining)]);
        if sample.len() >= SNIFF_LIMIT {
            break;
        }
    }
    Ok(sample)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::FileCategory;
    use crate::fs::{FaultFilesystem, FsFault, StdFilesystem};
    use weave_core::prelude::FileKind;
    use weave_testkit::TempWorkspace;

    fn inspect_default(ws: &TempWorkspace, rel: &str) -> Result<FileInspection, WeaveError> {
        let raw = ws.path().join(rel).to_string_lossy().into_owned();
        inspect_file(&StdFilesystem, &raw, &InspectOptions::default())
    }

    #[test]
    fn text_file_facts_are_complete() {
        let ws = TempWorkspace::new("insp-text").expect("ws");
        ws.file("notes.txt", "hello weave").expect("write");
        let result = inspect_default(&ws, "notes.txt").expect("inspect");

        assert_eq!(result.status, InspectionStatus::Complete);
        assert_eq!(result.kind, FileKind::RegularFile);
        assert_eq!(result.name, "notes.txt");
        assert_eq!(result.extension.as_deref(), Some("txt"));
        assert_eq!(result.size, 11);
        assert_eq!(result.classification.category, FileCategory::Text);
        assert_eq!(result.encoding, Some(TextEncoding::Ascii));
        assert!(result.modified.is_some());
        assert!(result.warnings.is_empty());
        assert_eq!(result.normalized_path, result.requested);
    }

    #[test]
    fn mislabeled_extension_is_reported_by_declared_and_detected() {
        let ws = TempWorkspace::new("insp-mislabeled").expect("ws");
        // 内容是 PNG，扩展名是 .bin：不做无证据推断。
        let png: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        ws.bytes("mystery.bin", &png).expect("write");
        let result = inspect_default(&ws, "mystery.bin").expect("inspect");
        assert_eq!(result.classification.category, FileCategory::Image);
        assert_eq!(
            result.classification.evidence,
            ClassificationEvidence::MagicBytes
        );
    }

    #[test]
    fn utf16_bom_promotes_text_and_reports_encoding() {
        let ws = TempWorkspace::new("insp-bom").expect("ws");
        let mut content = vec![0xFF, 0xFE];
        content.extend_from_slice(&[0x68, 0x00, 0x69, 0x00]); // "hi" UTF-16LE
        ws.bytes("export.dat", &content).expect("write");
        let result = inspect_default(&ws, "export.dat").expect("inspect");
        assert_eq!(result.encoding, Some(TextEncoding::Utf16Le));
        assert_eq!(result.classification.category, FileCategory::Text);
    }

    #[test]
    fn non_utf_text_reports_unknown_encoding_not_guesses() {
        let ws = TempWorkspace::new("insp-gb").expect("ws");
        let gb: [u8; 4] = [0xD6, 0xD0, 0xCE, 0xC4]; // GB18030 "中文"
        ws.bytes("legacy.txt", &gb).expect("write");
        let result = inspect_default(&ws, "legacy.txt").expect("inspect");
        assert_eq!(result.classification.category, FileCategory::Text);
        assert_eq!(result.encoding, Some(TextEncoding::Unknown));
    }

    #[test]
    fn directory_inspection_skips_sniff() {
        let ws = TempWorkspace::new("insp-dir").expect("ws");
        ws.dir("folder").expect("mkdir");
        let result = inspect_default(&ws, "folder").expect("inspect");
        assert_eq!(result.kind, FileKind::Directory);
        assert_eq!(result.status, InspectionStatus::Complete);
        assert!(result.encoding.is_none());
    }

    #[test]
    fn zero_byte_file_is_handled_without_sniff() {
        let ws = TempWorkspace::new("insp-empty").expect("ws");
        ws.file("empty.dat", "").expect("write");
        let result = inspect_default(&ws, "empty.dat").expect("inspect");
        assert_eq!(result.size, 0);
        assert_eq!(result.status, InspectionStatus::Complete);
    }

    #[test]
    fn missing_path_is_structured_not_found() {
        let ws = TempWorkspace::new("insp-missing").expect("ws");
        let err = inspect_default(&ws, "ghost.txt").expect_err("missing");
        assert_eq!(err.code, "path.notFound");
        assert_eq!(err.kind, weave_core::prelude::ErrorKind::Io);
    }

    #[test]
    fn sniff_failure_degrades_to_partial_with_warning_not_error() {
        let ws = TempWorkspace::new("insp-partial").expect("ws");
        ws.file(
            "broken.txt",
            "this content is longer than the injected failure point",
        )
        .expect("write");
        let raw = ws.path().join("broken.txt").to_string_lossy().into_owned();
        let fs = FaultFilesystem::wrapping(Box::new(StdFilesystem));
        fs.arm(FsFault::ReadFailure);

        let result = inspect_file(&fs, &raw, &InspectOptions::default()).expect("partial ok");
        assert_eq!(result.status, InspectionStatus::Partial);
        assert!(
            result.warnings.iter().any(|w| w.contains("sniff failed")),
            "warning must name the degraded part"
        );
        // 元数据仍然完整可用。
        assert_eq!(result.size, 54);
        assert!(result.modified.is_some());
    }

    #[test]
    fn symlink_entry_warns_about_no_follow_policy() {
        let ws = TempWorkspace::new("insp-symlink").expect("ws");
        ws.file("target.txt", "x").expect("write");
        let link = ws.path().join("link.txt");
        // Windows 创建符号链接需要开发者模式/特权：不可用时如实跳过（M1 §50 纪律）。
        if std::os::windows::fs::symlink_file(ws.path().join("target.txt"), &link).is_err() {
            return;
        }
        let raw = link.to_string_lossy().into_owned();
        let result =
            inspect_file(&StdFilesystem, &raw, &InspectOptions::default()).expect("inspect");
        assert_eq!(result.kind, FileKind::Symlink);
        assert!(result.warnings.iter().any(|w| w.contains("not followed")));
    }
}
