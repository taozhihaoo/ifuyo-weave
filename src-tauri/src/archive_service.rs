//! M2.5：Archive / Integrity IPC 服务。
//!
//! - 创建/解压 Job 化（JobTracker + 取消 + 进度；结果经 ArchiveResultCache
//!   领取——与 M2.1/M2.2/M2.4 同一模式，无第二套 Job）。
//! - Manifest 生成/校验、单文件 Hash 校验同步（典型规模亚秒；hash_file 流式）。
//! - Audit：历史落盘仅路径/计数/验证状态（§41 无敏感内容）。

#![expect(clippy::result_large_err)] // D10

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_core::prelude::{CancellationToken, WeaveError};
use weave_files::archive::{
    ArchivePlan, ArchiveSource, CompressionLevel, apply_extraction, build_extraction_plan,
    create_zip, inspect_zip,
};
use weave_files::integrity::{
    HashVerifyStatus, ManifestDiffKind, create_manifest, serialize_manifest, verify_file_hash,
    verify_manifest,
};

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("archive_service")
        .into()
}

fn parse_level(name: &str) -> CompressionLevel {
    match name {
        "store" => CompressionLevel::Store,
        "fast" => CompressionLevel::Fast,
        "maximum" => CompressionLevel::Maximum,
        _ => CompressionLevel::Normal,
    }
}

// ── 创建归档（Job） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveCreateRequestDto {
    pub sources: Vec<String>,
    pub output: String,
    /// store | fast | normal | maximum
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveCreateResultDto {
    pub output: String,
    pub file_count: f64,
    pub input_bytes: f64,
    pub archive_bytes: f64,
    pub compression_ratio: Option<f64>,
    pub failed_files: Vec<(String, String)>,
    pub cancelled: bool,
}

#[derive(Default)]
pub struct ArchiveResultCache {
    inner: std::sync::Mutex<std::collections::HashMap<String, ArchiveCreateResultDto>>,
}

impl ArchiveResultCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, job_id: String, report: ArchiveCreateResultDto) {
        self.inner
            .lock()
            .expect("archive cache")
            .insert(job_id, report);
    }

    pub fn get(&self, job_id: &str) -> Option<ArchiveCreateResultDto> {
        self.inner
            .lock()
            .expect("archive cache")
            .get(job_id)
            .cloned()
    }
}

#[tauri::command]
#[specta::specta]
pub fn archive_create(
    app: tauri::AppHandle,
    request: ArchiveCreateRequestDto,
) -> Result<JobHandleDto, IpcError> {
    if request.sources.is_empty() {
        return Err(err("archive.emptySources", "no sources"));
    }
    let sources = request
        .sources
        .iter()
        .map(|s| ArchiveSource {
            path: std::path::PathBuf::from(s),
        })
        .collect();
    let plan = ArchivePlan {
        sources,
        output: std::path::PathBuf::from(&request.output),
        level: parse_level(&request.level),
    };
    let sources_for_audit = request.sources.clone();
    let (job_id, cancel, sink, _) = {
        let state = app.state::<crate::state::AppState>();
        state.jobs.register()
    };
    let handle = app.clone();
    let job_id_for_task = job_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let result = create_zip(&plan, &cancel, &mut |done, total, _bytes| {
            sink.report(&weave_core::prelude::Progress::running(
                weave_core::prelude::OperationId::generate(),
                done,
                Some(total),
            ));
        });
        // Fatal（无法建/写/验）⇒ job failed；Cancelled/部分失败 ⇒ 结果照常领取
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                let app_state = handle.state::<crate::state::AppState>();
                app_state.jobs.fail(&job_id_for_task, e);
                return;
            }
        };
        let dto = ArchiveCreateResultDto {
            output: result.output.to_string_lossy().into_owned(),
            file_count: result.file_count as f64,
            input_bytes: result.input_bytes as f64,
            archive_bytes: result.archive_bytes as f64,
            compression_ratio: result.compression_ratio(),
            failed_files: result
                .failed_files
                .iter()
                .map(|(p, e)| (p.to_string_lossy().into_owned(), e.clone()))
                .collect(),
            cancelled: result.cancelled,
        };
        // Audit：历史仅路径/计数/验证状态（§41 无内容）
        if !result.cancelled
            && let Ok(history_dir) = crate::rename_service::resolve_history_dir(&handle)
            && let Ok(store) = weave_history::HistoryStore::open(&history_dir)
        {
            let now = std::time::SystemTime::now();
            let tx = weave_history::OperationTransaction {
                operation_id: weave_core::prelude::OperationId::generate(),
                kind: weave_core::prelude::OperationKind::BatchExecute,
                status: weave_history::OperationStatus::Completed,
                timestamp: now,
                reversible: weave_history::Reversibility::None,
                items: vec![weave_history::TransactionItem {
                    item_id: "item_0000".to_string(),
                    source_path: sources_for_audit.join("; "),
                    target_path: result.output.to_string_lossy().into_owned(),
                    status: weave_history::TransactionItemStatus::Executed,
                    timestamp: Some(now),
                    original_size: Some(result.input_bytes),
                    original_modified: None,
                    original_created: None,
                }],
            };
            let entry = weave_history::HistoryEntry {
                operation_id: tx.operation_id.clone(),
                kind: tx.kind,
                timestamp: tx.timestamp,
                summary: format!(
                    "Archive created: {} files → {}",
                    result.file_count,
                    result.output.display()
                ),
                item_count: 1,
                success_count: 1,
                failed_count: result.failed_files.len() as u64,
                skipped_count: 0,
                undoable: false,
                status: weave_history::OperationStatus::Completed,
                input_root: None,
                rule_summary: Some(format!("zip level={:?}", plan.level)),
            };
            let _ = store
                .save_transaction(&tx)
                .and_then(|()| store.upsert_entry(entry));
        }
        let app_state = handle.state::<crate::state::AppState>();
        app_state
            .archive_results
            .insert(job_id_for_task.to_string(), dto);
        app_state
            .jobs
            .finish(&job_id_for_task, crate::jobs::JobOutcome::ArchiveCompleted);
    });

    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 领取创建/解压结果。
#[tauri::command]
#[specta::specta]
pub fn archive_get_result(
    app: tauri::AppHandle,
    job_id: String,
) -> Result<ArchiveCreateResultDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    state
        .archive_results
        .get(&job_id)
        .ok_or_else(|| err("archive.notReady", format!("not ready for '{job_id}'")))
}

// ── Inspect（同步，只读） ──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntryDto {
    pub name: String,
    pub is_dir: bool,
    pub compressed_size: f64,
    pub uncompressed_size: f64,
    pub crc32: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveInspectionDto {
    pub entry_count: f64,
    pub file_count: f64,
    pub dir_count: f64,
    pub total_uncompressed: f64,
    pub total_compressed: f64,
    pub duplicate_entries: Vec<String>,
    pub unsafe_entries: Vec<String>,
    pub bomb_suspect: bool,
    pub entries: Vec<ArchiveEntryDto>,
}

/// 归档检查（§25：只读，不默认解压）。
#[tauri::command]
#[specta::specta]
pub fn archive_inspect(path: String) -> Result<ArchiveInspectionDto, IpcError> {
    let inspection = inspect_zip(std::path::Path::new(&path)).map_err(IpcError::from)?;
    Ok(ArchiveInspectionDto {
        entry_count: inspection.entry_count as f64,
        file_count: inspection.file_count as f64,
        dir_count: inspection.dir_count as f64,
        total_uncompressed: inspection.total_uncompressed as f64,
        total_compressed: inspection.total_compressed as f64,
        duplicate_entries: inspection.duplicate_entries.clone(),
        unsafe_entries: inspection.unsafe_entries.clone(),
        bomb_suspect: inspection.bomb_suspect,
        entries: inspection
            .entries
            .iter()
            .map(|e| ArchiveEntryDto {
                name: e.name.clone(),
                is_dir: e.is_dir,
                compressed_size: e.compressed_size as f64,
                uncompressed_size: e.uncompressed_size as f64,
                crc32: e.crc32 as f64,
            })
            .collect(),
    })
}

// ── 解压（Job；计划先行 + Zip Bomb Block） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveExtractRequestDto {
    pub archive: String,
    pub destination: String,
    /// skip | overwrite | rename
    pub overwrite: String,
    /// 解压总量上限（Zip Bomb §34；0 = 默认 10 GiB）。
    pub max_total_uncompressed: f64,
}

#[tauri::command]
#[specta::specta]
pub fn archive_extract(
    app: tauri::AppHandle,
    request: ArchiveExtractRequestDto,
) -> Result<JobHandleDto, IpcError> {
    let archive = std::path::PathBuf::from(&request.archive);
    let destination = std::path::PathBuf::from(&request.destination);
    let policy = match request.overwrite.as_str() {
        "overwrite" => weave_files::archive::OverwritePolicy::Overwrite,
        "rename" => weave_files::archive::OverwritePolicy::Rename,
        _ => weave_files::archive::OverwritePolicy::Skip,
    };
    let max_total = if request.max_total_uncompressed > 0.0 {
        request.max_total_uncompressed as u64
    } else {
        10 * 1024 * 1024 * 1024
    };
    // 计划先行（§27）：Inspect → Security Validation → Plan；炸弹 ⇒ Blocked
    let plan =
        build_extraction_plan(&archive, &destination, policy, max_total).map_err(IpcError::from)?;
    if plan.bomb_suspect {
        return Err(err(
            "archive.bombSuspect",
            "extraction blocked: uncompressed size exceeds policy limit",
        ));
    }
    let (job_id, cancel, sink, _) = {
        let state = app.state::<crate::state::AppState>();
        state.jobs.register()
    };
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    let archive_display = archive.to_string_lossy().into_owned();

    tauri::async_runtime::spawn_blocking(move || {
        let result = apply_extraction(&archive, &plan, &cancel, &mut |done, total| {
            sink.report(&weave_core::prelude::Progress::running(
                weave_core::prelude::OperationId::generate(),
                done,
                Some(total),
            ));
        });
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                let app_state = handle.state::<crate::state::AppState>();
                app_state.jobs.fail(&job_id_for_task, e);
                return;
            }
        };
        let dto = ArchiveCreateResultDto {
            output: archive_display,
            file_count: result.extracted as f64,
            input_bytes: 0.0,
            archive_bytes: result.extracted_bytes as f64,
            compression_ratio: None,
            failed_files: result.failed.clone(),
            cancelled: result.cancelled,
        };
        let app_state = handle.state::<crate::state::AppState>();
        app_state
            .archive_results
            .insert(job_id_for_task.to_string(), dto);
        app_state
            .jobs
            .finish(&job_id_for_task, crate::jobs::JobOutcome::ArchiveCompleted);
    });

    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

// ── Integrity（§36/§22/§23；同步——hash 流式且典型规模快） ──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HashVerifyResultDto {
    /// match | mismatch | unreadable | cancelled
    pub status: String,
    pub actual_hash: Option<String>,
}

/// 单文件 Hash 校验。
#[tauri::command]
#[specta::specta]
pub fn integrity_verify_file(
    path: String,
    expected_sha256: String,
) -> Result<HashVerifyResultDto, IpcError> {
    let outcome = verify_file_hash(
        std::path::Path::new(&path),
        &expected_sha256,
        &CancellationToken::new(),
    )
    .map_err(IpcError::from)?;
    Ok(HashVerifyResultDto {
        status: match outcome.status {
            HashVerifyStatus::Match => "match".to_string(),
            HashVerifyStatus::Mismatch => "mismatch".to_string(),
            HashVerifyStatus::Unreadable => "unreadable".to_string(),
            HashVerifyStatus::Cancelled => "cancelled".to_string(),
        },
        actual_hash: outcome.actual_hash,
    })
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ManifestResultDto {
    pub text: String,
    pub entry_count: f64,
}

/// 目录 Manifest 生成（行式文本 §65）。
#[tauri::command]
#[specta::specta]
pub fn integrity_create_manifest(root: String) -> Result<ManifestResultDto, IpcError> {
    let manifest = create_manifest(
        std::path::Path::new(&root),
        &CancellationToken::new(),
        &mut |_, _| {},
    )
    .map_err(IpcError::from)?;
    let count = manifest.entries.len();
    Ok(ManifestResultDto {
        text: serialize_manifest(&manifest),
        entry_count: count as f64,
    })
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVerifyRequestDto {
    pub root: String,
    pub manifest_text: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVerifyResultDto {
    pub same: f64,
    pub modified: f64,
    pub added: f64,
    pub removed: f64,
    pub unreadable: f64,
    pub items: Vec<(String, String)>,
}

/// Manifest 校验（差异语义复用 M2.1 命名 §38）。
#[tauri::command]
#[specta::specta]
pub fn integrity_verify_manifest(
    request: ManifestVerifyRequestDto,
) -> Result<ManifestVerifyResultDto, IpcError> {
    let report = verify_manifest(
        std::path::Path::new(&request.root),
        &request.manifest_text,
        &CancellationToken::new(),
        &mut |_, _| {},
    )
    .map_err(IpcError::from)?;
    Ok(ManifestVerifyResultDto {
        same: report.count(ManifestDiffKind::Same) as f64,
        modified: report.count(ManifestDiffKind::Modified) as f64,
        added: report.count(ManifestDiffKind::Added) as f64,
        removed: report.count(ManifestDiffKind::Removed) as f64,
        unreadable: report.count(ManifestDiffKind::Unreadable) as f64,
        items: report
            .items
            .iter()
            .map(|i| {
                (
                    i.relative_path.clone(),
                    format!("{:?}", i.kind).to_lowercase(),
                )
            })
            .collect(),
    })
}
