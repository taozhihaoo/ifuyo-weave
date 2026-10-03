//! File Core 的 IPC DTO 层（M1 §18）。
//!
//! React 不接触 weave-core/weave-files 的内部对象：这里建立稳定、可序列化、
//! 版本化的公开契约。大整数按 D4 用 f64（JSON number）；时间戳统一为
//! epoch 毫秒（f64），无法取得的时间如实为 null。

use serde::Serialize;
use specta::Type;
use weave_core::prelude::{HashAlgorithm, HashStatus, TextEncoding};
use weave_core::prelude::{HashResult, ScanStatus};
use weave_files::{
    Classification, ClassificationEvidence, DirectoryScanReport, FileInspection, FileLineItem,
    InspectionStatus, ScanErrorEntry,
};

fn time_to_epoch_ms(time: Option<std::time::SystemTime>) -> Option<f64> {
    time.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64)
}

fn u64_to_num(value: u64) -> f64 {
    value as f64
}

// ─── 分类 ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationDto {
    pub category: String,
    pub evidence: String,
    pub mime: Option<String>,
}

impl From<Classification> for ClassificationDto {
    fn from(c: Classification) -> Self {
        Self {
            category: c.category.as_str().to_string(),
            evidence: match c.evidence {
                ClassificationEvidence::Extension => "extension",
                ClassificationEvidence::MagicBytes => "magicBytes",
                ClassificationEvidence::None => "none",
            }
            .to_string(),
            mime: c.mime.map(str::to_string),
        }
    }
}

// ─── File Inspector（M1 §11.1 的分组在 UI 呈现；契约是扁平事实）───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileInspectionDto {
    pub status: String,
    pub requested: String,
    pub normalized_path: String,
    pub name: String,
    pub extension: Option<String>,
    pub kind: String,
    pub size: f64,
    pub readonly: bool,
    pub hidden: Option<bool>,
    pub created_ms: Option<f64>,
    pub modified_ms: Option<f64>,
    pub accessed_ms: Option<f64>,
    pub classification: ClassificationDto,
    /// 仅类文本场景；null = 不适用（与 Unknown = 无法判断 区分）。
    pub encoding: Option<String>,
    pub warnings: Vec<String>,
}

impl FileInspectionDto {
    pub fn from_inspection(i: FileInspection) -> Self {
        Self {
            status: match i.status {
                InspectionStatus::Complete => "complete",
                InspectionStatus::Partial => "partial",
            }
            .to_string(),
            requested: i.requested,
            normalized_path: i.normalized_path,
            name: i.name,
            extension: i.extension,
            kind: kind_to_str(i.kind),
            size: u64_to_num(i.size),
            readonly: i.readonly,
            hidden: i.hidden,
            created_ms: time_to_epoch_ms(i.created),
            modified_ms: time_to_epoch_ms(i.modified),
            accessed_ms: time_to_epoch_ms(i.accessed),
            classification: i.classification.into(),
            encoding: i.encoding.map(encoding_to_str),
            warnings: i.warnings,
        }
    }
}

pub fn kind_to_str(kind: weave_core::prelude::FileKind) -> String {
    match kind {
        weave_core::prelude::FileKind::RegularFile => "file",
        weave_core::prelude::FileKind::Directory => "directory",
        weave_core::prelude::FileKind::Symlink => "symlink",
        weave_core::prelude::FileKind::Other => "other",
    }
    .to_string()
}

pub fn encoding_to_str(encoding: TextEncoding) -> String {
    match encoding {
        TextEncoding::Utf8 => "utf8",
        TextEncoding::Utf8Bom => "utf8Bom",
        TextEncoding::Utf16Le => "utf16Le",
        TextEncoding::Utf16Be => "utf16Be",
        TextEncoding::Ascii => "ascii",
        TextEncoding::Gb18030 => "gb18030",
        TextEncoding::Gbk => "gbk",
        TextEncoding::Latin1 => "latin1",
        TextEncoding::Unknown => "unknown",
    }
    .to_string()
}

// ─── Hash ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HashResultDto {
    pub algorithm: String,
    pub digest_hex: Option<String>,
    pub bytes_processed: f64,
    pub duration_ms: f64,
    pub status: String,
}

impl From<HashResult> for HashResultDto {
    fn from(h: HashResult) -> Self {
        Self {
            algorithm: match h.algorithm {
                HashAlgorithm::Sha256 => "sha256".to_string(),
            },
            digest_hex: h.digest_hex,
            bytes_processed: u64_to_num(h.bytes_processed),
            duration_ms: u64_to_num(h.duration_ms),
            status: match h.status {
                HashStatus::Completed => "completed",
                HashStatus::Cancelled => "cancelled",
                HashStatus::Unstable => "unstable",
            }
            .to_string(),
        }
    }
}

// ─── Directory Analyzer ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileLineItemDto {
    pub relative_path: String,
    pub size: f64,
    pub modified_ms: Option<f64>,
}

impl From<FileLineItem> for FileLineItemDto {
    fn from(f: FileLineItem) -> Self {
        Self {
            relative_path: f.relative_path,
            size: u64_to_num(f.size),
            modified_ms: time_to_epoch_ms(f.modified),
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanErrorDto {
    pub relative_path: String,
    pub code: String,
    pub message: String,
}

impl From<ScanErrorEntry> for ScanErrorDto {
    fn from(e: ScanErrorEntry) -> Self {
        Self {
            relative_path: e.relative_path,
            code: e.code,
            message: e.message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileTypeCountDto {
    pub label: String,
    pub count: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanReportDto {
    pub scan_id: String,
    pub root: String,
    pub status: String,
    pub started_at_ms: Option<f64>,
    pub finished_at_ms: Option<f64>,
    pub duration_ms: f64,
    pub directories_scanned: f64,
    pub files_scanned: f64,
    pub other_entries: f64,
    pub error_count: f64,
    pub entries_processed: f64,
    pub total_size: f64,
    pub max_depth: f64,
    pub empty_directories: Vec<String>,
    pub empty_directories_truncated: bool,
    pub file_type_distribution: Vec<FileTypeCountDto>,
    pub largest_files: Vec<FileLineItemDto>,
    pub oldest_files: Vec<FileLineItemDto>,
    pub newest_files: Vec<FileLineItemDto>,
    pub warnings: Vec<String>,
    pub errors: Vec<ScanErrorDto>,
    pub errors_truncated: bool,
    pub limited: bool,
    pub limited_reason: Option<String>,
}

impl From<DirectoryScanReport> for ScanReportDto {
    fn from(r: DirectoryScanReport) -> Self {
        Self {
            scan_id: r.scan_id.to_string(),
            root: r.root,
            status: scan_status_to_str(r.status),
            started_at_ms: time_to_epoch_ms(Some(r.started_at)),
            finished_at_ms: time_to_epoch_ms(Some(r.finished_at)),
            duration_ms: u64_to_num(r.duration_ms),
            directories_scanned: u64_to_num(r.directories_scanned),
            files_scanned: u64_to_num(r.files_scanned),
            other_entries: u64_to_num(r.other_entries),
            error_count: u64_to_num(r.error_count),
            entries_processed: u64_to_num(r.entries_processed),
            total_size: u64_to_num(r.total_size),
            max_depth: u64_to_num(r.max_depth as u64),
            empty_directories: r.empty_directories,
            empty_directories_truncated: r.empty_directories_truncated,
            file_type_distribution: r
                .file_type_distribution
                .into_iter()
                .map(|(label, count)| FileTypeCountDto {
                    label,
                    count: u64_to_num(count),
                })
                .collect(),
            largest_files: r.largest_files.into_iter().map(Into::into).collect(),
            oldest_files: r.oldest_files.into_iter().map(Into::into).collect(),
            newest_files: r.newest_files.into_iter().map(Into::into).collect(),
            warnings: r.warnings,
            errors: r.errors.into_iter().map(Into::into).collect(),
            errors_truncated: r.errors_truncated,
            limited: r.limited,
            limited_reason: r.limited_reason,
        }
    }
}

pub fn scan_status_to_str(status: ScanStatus) -> String {
    match status {
        ScanStatus::Completed => "completed",
        ScanStatus::CompletedWithWarnings => "completedWithWarnings",
        ScanStatus::Failed => "failed",
        ScanStatus::Cancelled => "cancelled",
    }
    .to_string()
}

/// 扫描选项（IPC 侧）。数值用 f64 并在应用层校验边界（M1 §18.1）。
#[derive(Debug, Clone, Default, serde::Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptionsDto {
    pub max_depth: Option<f64>,
    pub max_entries: Option<f64>,
}

impl ScanOptionsDto {
    /// 校验并转换为领域选项。UI 数据一律不可信（M1 §18.1）。
    pub fn into_domain(self) -> Result<weave_files::ScanOptions, IpcValidationError> {
        fn bounded(
            value: Option<f64>,
            hard_max: f64,
            what: &str,
        ) -> Result<Option<u64>, IpcValidationError> {
            match value {
                None => Ok(None),
                Some(v) if v < 1.0 || v != v.trunc() || v > hard_max => Err(IpcValidationError {
                    what: what.to_string(),
                    value: v,
                }),
                Some(v) => Ok(Some(v as u64)),
            }
        }
        Ok(weave_files::ScanOptions {
            max_depth: bounded(
                self.max_depth,
                weave_files::HARD_MAX_DEPTH as f64,
                "maxDepth",
            )?
            .map(|v| v as u32),
            max_entries: bounded(
                self.max_entries,
                weave_files::HARD_MAX_ENTRIES as f64,
                "maxEntries",
            )?,
        })
    }
}

/// 选项校验失败的结构化错误（转 IpcError 用）。
#[derive(Debug)]
pub struct IpcValidationError {
    pub what: String,
    pub value: f64,
}

// ─── Jobs ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JobHandleDto {
    pub job_id: String,
}

/// 重复组内单个文件条目（M3 §33）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFileEntryDto {
    pub file_id: String,
    pub path: String,
    pub size: f64,
    pub modified_ms: Option<f64>,
    pub full_hash: String,
}

/// 精确重复组（M3 §12）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroupDto {
    pub group_id: String,
    pub file_count: f64,
    pub file_size: f64,
    pub wasted_size: f64,
    pub files: Vec<DuplicateFileEntryDto>,
}

/// 重复扫描报告（M3 §29 字段全集；数值 f64（D4）、时间 epoch-ms）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanReportDto {
    pub scan_id: String,
    pub roots: Vec<String>,
    pub status: String,
    pub stage: String,
    pub duration_ms: f64,
    pub files_scanned: f64,
    pub directories_scanned: f64,
    pub other_entries: f64,
    pub candidate_files: f64,
    pub partial_hashed: f64,
    pub full_hashed: f64,
    pub duplicate_groups: f64,
    pub duplicate_files: f64,
    pub potential_reclaimable_size: f64,
    pub skipped: f64,
    pub failed: f64,
    pub changed_during_scan: f64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub partial_result: bool,
    pub groups: Vec<DuplicateGroupDto>,
}

impl DuplicateScanReportDto {
    pub fn from_report(r: &weave_files::DuplicateScanReport) -> Self {
        Self {
            scan_id: r.scan_id.to_string(),
            roots: r.roots.clone(),
            status: scan_status_to_str(r.status).to_string(),
            stage: match r.stage {
                weave_files::duplicates::ScanStage::Scanning => "scanning",
                weave_files::duplicates::ScanStage::Filtering => "filtering",
                weave_files::duplicates::ScanStage::PartialHashing => "partialHashing",
                weave_files::duplicates::ScanStage::FullHashing => "fullHashing",
                weave_files::duplicates::ScanStage::Grouping => "grouping",
                weave_files::duplicates::ScanStage::Completed => "completed",
            }
            .to_string(),
            duration_ms: u64_to_num(r.duration_ms),
            files_scanned: u64_to_num(r.files_scanned),
            directories_scanned: u64_to_num(r.directories_scanned),
            other_entries: u64_to_num(r.other_entries),
            candidate_files: u64_to_num(r.candidate_files),
            partial_hashed: u64_to_num(r.partial_hashed),
            full_hashed: u64_to_num(r.full_hashed),
            duplicate_groups: u64_to_num(r.duplicate_groups),
            duplicate_files: u64_to_num(r.duplicate_files),
            potential_reclaimable_size: u64_to_num(r.potential_reclaimable_size),
            skipped: u64_to_num(r.skipped),
            failed: u64_to_num(r.failed),
            changed_during_scan: u64_to_num(r.changed_during_scan),
            warnings: r.warnings.clone(),
            errors: r.errors.clone(),
            partial_result: r.partial_result,
            groups: r
                .groups
                .iter()
                .map(|g| DuplicateGroupDto {
                    group_id: g.group_id.clone(),
                    file_count: u64_to_num(g.file_count),
                    file_size: u64_to_num(g.file_size),
                    wasted_size: u64_to_num(g.wasted_size),
                    files: g
                        .files
                        .iter()
                        .map(|f| DuplicateFileEntryDto {
                            file_id: f.file_id.clone(),
                            path: f.path.clone(),
                            size: u64_to_num(f.size),
                            modified_ms: time_to_epoch_ms(f.modified),
                            full_hash: f.full_hash.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusDto {
    pub job_id: String,
    /// running | completed | failed（cancelled 由 outcome 内的 status 表达，
    /// 因为"用户已点取消但任务尚未到达安全点"仍是 running）。
    pub state: String,
    pub progress_current: Option<f64>,
    pub hash: Option<HashResultDto>,
    pub scan: Option<ScanReportDto>,
    /// M2：Rename/Organizer 执行（或 Undo 计数）结果。
    pub plan: Option<super::ops_dto::PlanReportDto>,
    pub undo: Option<super::ops_dto::UndoReportDto>,
    /// M3：重复扫描报告。
    pub duplicate_scan: Option<DuplicateScanReportDto>,
    pub error: Option<super::commands::IpcError>,
}

// ─── Tool registry ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptorDto {
    pub id: String,
    pub category: String,
    pub input_kinds: Vec<String>,
}
