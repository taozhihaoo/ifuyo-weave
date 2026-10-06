//! M2.4：Privacy / Share Preflight IPC 服务。
//!
//! - 扫描 Job 化（JobTracker + 取消 + 进度；结果经 PrivacyResultCache 领取
//!   ——与 M2.1/M2.2 同一模式，无第二套 Job）。
//! - 导出 = stale 重验（§42 复用 M2.0 verification）→ 内容变换（weave-privacy
//!   cleanup）→ verify_safe_copy（§36）→ atomic_write（M2.0 共享写原语）→
//!   事务/历史（undo = 删除 Safe Copy；原文件全程只读 §21）。
//! - 隐私铁律（§14/§50）：命令与日志只携带 id/路径/计数/masked 证据。

#![expect(clippy::result_large_err)] // D10

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_core::prelude::{CancellationToken, WeaveError};
use weave_privacy::{
    PrivacyScanComparison, PrivacyScanSession, ScanLimits, apply_cleanup, compare_sessions,
    scan_bytes_auto, verify_safe_copy,
};

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("privacy_service")
        .into()
}

// ── 扫描（Job 化） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyScanRequestDto {
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyFindingDto {
    pub fingerprint: String,
    pub relative_path: String,
    pub category: String,
    pub sub_category: String,
    pub severity: String,
    pub evidence: String,
    pub location: String,
    pub description: String,
    pub evidence_summary: String,
    pub cleanup: String,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyReportDto {
    pub session_id: String,
    pub root: String,
    pub duration_ms: f64,
    pub status: String,
    pub files_scanned: f64,
    pub files_supported: f64,
    pub files_unsupported: f64,
    pub files_failed: f64,
    pub high_count: f64,
    pub medium_count: f64,
    pub low_count: f64,
    pub confirmed_count: f64,
    pub suspected_count: f64,
    pub findings: Vec<PrivacyFindingDto>,
    pub not_fully_checked: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

impl PrivacyReportDto {
    fn from_session(s: &PrivacyScanSession) -> Self {
        Self {
            session_id: s.session_id.clone(),
            root: s.root.clone(),
            duration_ms: s.duration_ms as f64,
            status: format!("{:?}", s.status).to_lowercase(),
            files_scanned: s.files_scanned as f64,
            files_supported: s.files_supported as f64,
            files_unsupported: s.files_unsupported as f64,
            files_failed: s.files_failed as f64,
            high_count: s.high_count() as f64,
            medium_count: s.medium_count() as f64,
            low_count: s.low_count() as f64,
            confirmed_count: s.confirmed_count() as f64,
            suspected_count: s.suspected_count() as f64,
            findings: s
                .findings
                .iter()
                .map(|f| PrivacyFindingDto {
                    fingerprint: f.fingerprint.clone(),
                    relative_path: f.relative_path.clone(),
                    category: format!("{:?}", f.category).to_lowercase(),
                    sub_category: f.sub_category.clone(),
                    severity: format!("{:?}", f.severity).to_lowercase(),
                    evidence: format!("{:?}", f.evidence).to_lowercase(),
                    location: f.location.clone(),
                    description: f.description.clone(),
                    evidence_summary: f.evidence_summary.clone(),
                    cleanup: format!("{:?}", f.cleanup).to_lowercase(),
                    recommended: f.cleanup.recommended_by_default(f.evidence),
                })
                .collect(),
            not_fully_checked: s
                .not_fully_checked
                .iter()
                .map(|n| (n.relative_path.clone(), n.reason.clone()))
                .collect(),
            warnings: s.warnings.clone(),
        }
    }
}

#[derive(Default)]
pub struct PrivacyResultCache {
    inner: std::sync::Mutex<std::collections::HashMap<String, PrivacyReportDto>>,
}

impl PrivacyResultCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, job_id: String, report: PrivacyReportDto) {
        self.inner
            .lock()
            .expect("privacy cache")
            .insert(job_id, report);
    }

    pub fn get(&self, job_id: &str) -> Option<PrivacyReportDto> {
        self.inner
            .lock()
            .expect("privacy cache")
            .get(job_id)
            .cloned()
    }
}

/// 目录递归收集（有界 10k 文件；复用 space 的文件遍历纪律）。
fn collect_files(root: &std::path::Path, cancel: &CancellationToken) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if cancel.is_cancelled() || out.len() >= 10_000 {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[tauri::command]
#[specta::specta]
pub fn privacy_scan(
    app: tauri::AppHandle,
    request: PrivacyScanRequestDto,
) -> Result<JobHandleDto, IpcError> {
    let root = std::path::PathBuf::from(&request.root);
    if !root.is_dir() && !root.is_file() {
        return Err(err(
            "privacy.rootMissing",
            format!("'{}' not found", request.root),
        ));
    }
    let (job_id, cancel, sink, _) = {
        let state = app.state::<crate::state::AppState>();
        state.jobs.register()
    };
    let handle = app.clone();
    let job_id_for_task = job_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let limits = ScanLimits::default();
        let mut session =
            PrivacyScanSession::new(&root.to_string_lossy(), job_id_for_task.to_string());
        let targets = if root.is_file() {
            vec![root.clone()]
        } else {
            collect_files(&root, &cancel)
        };
        let total = targets.len() as u64;
        let root_prefix = root.to_string_lossy().into_owned();
        for (done, path) in targets.iter().enumerate() {
            if cancel.is_cancelled() {
                break;
            }
            sink.report(&weave_core::prelude::Progress::running(
                weave_core::prelude::OperationId::generate(),
                done as u64,
                Some(total),
            ));
            let Ok(bytes) = std::fs::read(path) else {
                session.files_failed += 1;
                continue;
            };
            let rel = path
                .strip_prefix(&root_prefix)
                .unwrap_or(path)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            let rel = if rel.is_empty() {
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                rel
            };
            scan_bytes_auto(&rel, &bytes, &mut session, &limits);
        }
        session.status = if cancel.is_cancelled() {
            weave_privacy::SessionStatus::PartiallyCompleted
        } else {
            weave_privacy::SessionStatus::Completed
        };
        session.duration_ms = session
            .started_at
            .elapsed()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let app_state = handle.state::<crate::state::AppState>();
        app_state.privacy_results.insert(
            job_id_for_task.to_string(),
            PrivacyReportDto::from_session(&session),
        );
        app_state
            .jobs
            .finish(&job_id_for_task, crate::jobs::JobOutcome::PrivacyCompleted);
    });

    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 领取扫描结果。
#[tauri::command]
#[specta::specta]
pub fn privacy_get_result(
    app: tauri::AppHandle,
    job_id: String,
) -> Result<PrivacyReportDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    state
        .privacy_results
        .get(&job_id)
        .ok_or_else(|| err("privacy.notReady", format!("not ready for '{job_id}'")))
}

// ── Safe Copy 导出（§22/§36/§42） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyExportItemDto {
    /// 原文件绝对路径。
    pub source: String,
    /// Safe Copy 输出绝对路径（§21：显式输出；默认由 UI 给 `<name>.safe.<ext>`）。
    pub output: String,
    /// 扫描时的源快照（stale 重验依据 §42）。
    pub size: f64,
    pub modified_ms: Option<f64>,
    /// 选中清理项（相对路径 + cleanup 类型 + sub_category）。
    pub findings: Vec<PrivacyExportFindingDto>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyExportFindingDto {
    pub relative_path: String,
    pub sub_category: String,
    pub cleanup: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyExportResultDto {
    pub exported: f64,
    pub stale_skipped: f64,
    pub failed: f64,
    pub operation_id: String,
    /// 每项：source → output（已验证）。
    pub outputs: Vec<(String, String)>,
}

fn parse_cleanup(name: &str) -> Option<weave_privacy::CleanupAction> {
    // DTO 只传命中的可清理动作；strip/clear/mask 三类可自动执行
    match name {
        "stripImageMetadata" => Some(weave_privacy::CleanupAction::StripImageMetadata),
        "clearDocumentCoreMetadata" => {
            Some(weave_privacy::CleanupAction::ClearDocumentCoreMetadata)
        }
        "maskDetectedValue" => Some(weave_privacy::CleanupAction::MaskDetectedValue),
        _ => None,
    }
}

/// 导出 Safe Copy（§21/§22/§42）：
/// stale 重验（size+mtime 与扫描快照一致）→ 内容变换 → verify → 原子写。
/// 原文件全程只读；输出已存在 ⇒ 拒绝（No accidental overwrite §22）。
#[tauri::command]
#[specta::specta]
pub fn privacy_export_safe_copy(
    app: tauri::AppHandle,
    items: Vec<PrivacyExportItemDto>,
) -> Result<PrivacyExportResultDto, IpcError> {
    use weave_core::prelude::VerificationPolicy;
    use weave_files::verify::verify_fingerprint;

    if items.is_empty() {
        return Err(err("privacy.emptySelection", "no files selected"));
    }
    let operation_id = weave_core::prelude::OperationId::generate();
    let now = std::time::SystemTime::now();
    let (mut exported, mut stale, mut failed) = (0u64, 0u64, 0u64);
    let mut outputs: Vec<(String, String)> = Vec::new();
    let mut tx_items = Vec::new();

    for item in &items {
        let source = std::path::PathBuf::from(&item.source);
        let output = std::path::PathBuf::from(&item.output);
        // §42 Stale 重验：size/mtime 与扫描快照一致
        let modified = item.modified_ms.and_then(|ms| {
            std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(ms as u64))
        });
        let fingerprint = weave_core::prelude::FileFingerprint {
            path: item.source.clone(),
            size: Some(item.size as u64),
            modified,
            digest_hex: None,
        };
        if verify_fingerprint(
            &weave_files::fs::StdFilesystem,
            &fingerprint,
            VerificationPolicy::SizeMtime,
            "privacy",
        )
        .is_err()
        {
            stale += 1;
            continue;
        }
        // §22 No accidental overwrite：输出已存在 ⇒ 该项失败（不覆盖）
        if output.exists() {
            failed += 1;
            continue;
        }
        // 内容变换 + 验证（§36）
        let Ok(original) = std::fs::read(&source) else {
            failed += 1;
            continue;
        };
        let selected: Vec<weave_privacy::PrivacyFinding> = item
            .findings
            .iter()
            .filter_map(|f| {
                parse_cleanup(&f.cleanup).map(|cleanup| {
                    weave_privacy::PrivacyFinding::new(weave_privacy::FindingFacts {
                        relative_path: f.relative_path.clone(),
                        category: weave_privacy::FindingCategory::SuspectedSecret,
                        sub_category: f.sub_category.clone(),
                        severity: weave_privacy::Severity::Medium,
                        evidence: weave_privacy::EvidenceLevel::Suspected,
                        location: String::new(),
                        description: String::new(),
                        evidence_summary: String::new(),
                        cleanup,
                        evidence_identity: format!("{}-{}", f.sub_category, f.cleanup),
                    })
                })
            })
            .collect();
        let safe_bytes = match apply_cleanup(&original, &item.source, &selected) {
            Ok(bytes) => bytes,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        if let Err(e) = verify_safe_copy(&safe_bytes, &item.source) {
            tracing::warn!(code = %e.code, "safe copy verification failed");
            failed += 1;
            continue;
        }
        // 原子写（M2.0 共享写原语）
        if let Err(e) = weave_files::atomic_write::atomic_write(
            &weave_files::fs::StdFilesystem,
            &output,
            &safe_bytes,
        ) {
            tracing::warn!(code = %e.code, "safe copy write failed");
            failed += 1;
            continue;
        }
        exported += 1;
        outputs.push((item.source.clone(), item.output.clone()));
        // 事务：undo = 删除 Safe Copy（创建型，target 空 ⇒ 既有 undo 语义）
        tx_items.push(weave_history::TransactionItem {
            item_id: format!("item_{:04}", tx_items.len()),
            source_path: output.to_string_lossy().into_owned(),
            target_path: String::new(),
            status: weave_history::TransactionItemStatus::Executed,
            timestamp: Some(now),
            original_size: Some(safe_bytes.len() as u64),
            original_modified: Some(now),
            original_created: None,
        });
    }

    // 事务/历史落盘（Audit §48：只有路径/计数，无内容/敏感值）
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    if !tx_items.is_empty() {
        if let Ok(store) = weave_history::HistoryStore::open(&history_dir) {
            let tx = weave_history::OperationTransaction {
                operation_id: operation_id.clone(),
                kind: weave_core::prelude::OperationKind::TextTransform,
                status: weave_history::OperationStatus::Completed,
                timestamp: now,
                reversible: if exported == 0 {
                    weave_history::Reversibility::None
                } else if stale + failed == 0 {
                    weave_history::Reversibility::Full
                } else {
                    weave_history::Reversibility::Partial
                },
                items: tx_items,
            };
            let entry = weave_history::HistoryEntry {
                operation_id: tx.operation_id.clone(),
                kind: tx.kind,
                timestamp: tx.timestamp,
                summary: format!("Privacy export: {exported} safe copies"),
                item_count: tx.items.len() as u64,
                success_count: exported,
                failed_count: failed,
                skipped_count: stale,
                undoable: exported > 0,
                status: weave_history::OperationStatus::Completed,
                input_root: None,
                rule_summary: Some("share preflight safe copy".to_string()),
            };
            let _ = store
                .save_transaction(&tx)
                .and_then(|()| store.upsert_entry(entry));
        }
    }

    Ok(PrivacyExportResultDto {
        exported: exported as f64,
        stale_skipped: stale as f64,
        failed: failed as f64,
        operation_id: operation_id.to_string(),
        outputs,
    })
}

// ── Re-scan 对比（§27） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyCompareRequestDto {
    pub before_job_id: String,
    pub after_job_id: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyComparisonDto {
    pub resolved: f64,
    pub remaining: f64,
    pub new: f64,
}

/// Before vs After（按稳定指纹；§27）。
#[tauri::command]
#[specta::specta]
pub fn privacy_compare(
    app: tauri::AppHandle,
    request: PrivacyCompareRequestDto,
) -> Result<PrivacyComparisonDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let to_session = |dto: &PrivacyReportDto| PrivacyScanSession {
        session_id: dto.session_id.clone(),
        root: dto.root.clone(),
        started_at: std::time::SystemTime::UNIX_EPOCH,
        duration_ms: dto.duration_ms as u64,
        status: weave_privacy::SessionStatus::Completed,
        files_scanned: dto.files_scanned as u64,
        files_supported: dto.files_supported as u64,
        files_unsupported: dto.files_unsupported as u64,
        files_failed: dto.files_failed as u64,
        findings: dto
            .findings
            .iter()
            .map(|f| weave_privacy::PrivacyFinding {
                fingerprint: f.fingerprint.clone(),
                relative_path: f.relative_path.clone(),
                category: weave_privacy::FindingCategory::SuspectedSecret,
                sub_category: f.sub_category.clone(),
                severity: weave_privacy::Severity::Medium,
                evidence: weave_privacy::EvidenceLevel::Suspected,
                location: f.location.clone(),
                description: String::new(),
                evidence_summary: String::new(),
                cleanup: weave_privacy::CleanupAction::ScanOnly,
                original_value_available: false,
            })
            .collect(),
        not_fully_checked: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };
    let before = state
        .privacy_results
        .get(&request.before_job_id)
        .ok_or_else(|| err("privacy.notReady", "before session missing"))?;
    let after = state
        .privacy_results
        .get(&request.after_job_id)
        .ok_or_else(|| err("privacy.notReady", "after session missing"))?;
    let cmp: PrivacyScanComparison = compare_sessions(&to_session(&before), &to_session(&after));
    Ok(PrivacyComparisonDto {
        resolved: cmp.resolved.len() as f64,
        remaining: cmp.remaining.len() as f64,
        new: cmp.new.len() as f64,
    })
}
