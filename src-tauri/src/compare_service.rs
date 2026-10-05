//! M2.1：Compare IPC 命令（Read-only 比较 + Actionable → Plan 复用 M2.0 执行）。
//!
//! - 文件比较/文本详情：同步（有界，§8 分层策略）。
//! - 目录比较：Job 化（JobTracker + 取消 + 进度；结果按 jobId 存 AppState）。
//! - Actionable：Copy 计划进 PlanCache ⇒ 复用既有 execute_plan IPC；
//!   Recycle 直接复用 M3 recycle + 事务/undo 分派——**不设第二执行引擎**。

#![expect(clippy::result_large_err)] // D10：Err DTO 体积豁免（与其他 service 同）

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_compare::{
    CompareOptions, ComparisonSession, ContentCheck, DifferenceItem, DifferenceKind, EntryFacts,
    IgnoreRules, NodeKind,
};
use weave_core::prelude::{
    CancellationToken, CollisionKind, OperationId, OperationKind, Plan, PlanItem, PlanItemStatus,
    WeaveError,
};

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("compare_service")
        .into()
}

// ── DTO ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompareOptionsDto {
    /// metadata | hash
    pub content_check: String,
    pub ignore_user: Vec<String>,
    pub max_text_detail_bytes: Option<f64>,
}

impl CompareOptionsDto {
    fn to_domain(&self) -> Result<CompareOptions, IpcError> {
        let content_check = match self.content_check.as_str() {
            "metadata" => ContentCheck::Metadata,
            "hash" => ContentCheck::Hash,
            other => {
                return Err(err(
                    "compare.unknownPolicy",
                    format!("unknown content check '{other}'"),
                ));
            }
        };
        Ok(CompareOptions {
            content_check,
            ignore: IgnoreRules {
                defaults: IgnoreRules::default().defaults,
                user: self.ignore_user.clone(),
            },
            max_text_detail_bytes: self.max_text_detail_bytes.unwrap_or(2.0 * 1024.0 * 1024.0)
                as u64,
            max_hash_bytes: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EntryFactsDto {
    pub path: String,
    pub size: Option<f64>,
    pub modified_ms: Option<f64>,
    pub digest_hex: Option<String>,
    pub encoding: Option<String>,
    pub line_count: Option<f64>,
}

impl EntryFactsDto {
    fn from_facts(f: &EntryFacts) -> Self {
        Self {
            path: f.path.clone(),
            size: f.size.map(|v| v as f64),
            modified_ms: f
                .modified
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as f64),
            digest_hex: f.digest_hex.clone(),
            encoding: f.encoding.map(|e| format!("{e:?}")),
            line_count: f.line_count.map(|v| v as f64),
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DifferenceItemDto {
    pub relative_path: String,
    pub node: String,
    pub kind: String,
    pub content: String,
    pub left: Option<EntryFactsDto>,
    pub right: Option<EntryFactsDto>,
    pub method: Option<String>,
    pub left_digest: Option<String>,
    pub right_digest: Option<String>,
    pub error_code: Option<String>,
}

fn kind_str(k: DifferenceKind) -> &'static str {
    match k {
        DifferenceKind::Same => "same",
        DifferenceKind::Added => "added",
        DifferenceKind::Removed => "removed",
        DifferenceKind::Modified => "modified",
        DifferenceKind::TypeChanged => "typeChanged",
        DifferenceKind::Moved => "moved",
        DifferenceKind::Renamed => "renamed",
        DifferenceKind::Unreadable => "unreadable",
        DifferenceKind::Ignored => "ignored",
        DifferenceKind::Unsupported => "unsupported",
        DifferenceKind::Error => "error",
    }
}

impl DifferenceItemDto {
    fn from_item(item: &DifferenceItem) -> Self {
        Self {
            relative_path: item.relative_path.clone(),
            node: match item.node {
                NodeKind::File => "file".to_string(),
                NodeKind::Directory => "directory".to_string(),
                NodeKind::Symlink => "symlink".to_string(),
            },
            kind: kind_str(item.kind).to_string(),
            content: match item.content {
                weave_compare::ContentDifference::None => "none".to_string(),
                weave_compare::ContentDifference::ContentChanged => "contentChanged".to_string(),
                weave_compare::ContentDifference::MetadataOnly => "metadataOnly".to_string(),
                weave_compare::ContentDifference::NotCompared => "notCompared".to_string(),
            },
            left: item.left.as_ref().map(EntryFactsDto::from_facts),
            right: item.right.as_ref().map(EntryFactsDto::from_facts),
            method: item.evidence.as_ref().map(|e| e.method.clone()),
            left_digest: item.evidence.as_ref().and_then(|e| e.left_digest.clone()),
            right_digest: item.evidence.as_ref().and_then(|e| e.right_digest.clone()),
            error_code: item.error.as_ref().map(|e| e.code.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompareSummaryDto {
    pub total: f64,
    pub same: f64,
    pub added: f64,
    pub removed: f64,
    pub modified: f64,
    pub type_changed: f64,
    pub moved: f64,
    pub renamed: f64,
    pub unreadable: f64,
    pub ignored: f64,
    pub unsupported: f64,
    pub error: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompareReportDto {
    pub operation_id: String,
    pub session_kind: String,
    pub left_root: String,
    pub right_root: String,
    pub effective_ignores: Vec<String>,
    pub items: Vec<DifferenceItemDto>,
    pub summary: CompareSummaryDto,
    pub duration_ms: f64,
    pub cancelled: bool,
}

impl CompareReportDto {
    fn from_session(s: &ComparisonSession) -> Self {
        Self {
            operation_id: s.operation_id.to_string(),
            session_kind: match s.kind {
                weave_compare::SessionKind::Files => "files".to_string(),
                weave_compare::SessionKind::Directories => "directories".to_string(),
            },
            left_root: s.left_root.clone(),
            right_root: s.right_root.clone(),
            effective_ignores: s.effective_ignores.clone(),
            items: s.items.iter().map(DifferenceItemDto::from_item).collect(),
            summary: CompareSummaryDto {
                total: s.summary.total as f64,
                same: s.summary.same as f64,
                added: s.summary.added as f64,
                removed: s.summary.removed as f64,
                modified: s.summary.modified as f64,
                type_changed: s.summary.type_changed as f64,
                moved: s.summary.moved as f64,
                renamed: s.summary.renamed as f64,
                unreadable: s.summary.unreadable as f64,
                ignored: s.summary.ignored as f64,
                unsupported: s.summary.unsupported as f64,
                error: s.summary.error as f64,
            },
            duration_ms: s.duration_ms as f64,
            cancelled: s.cancelled,
        }
    }
}

// ── Read-only 命令 ──

/// 文件比较（同步；有界：hash 流式，文本详情另行按需）。
#[tauri::command]
#[specta::specta]
pub fn compare_files(
    left: String,
    right: String,
    options: CompareOptionsDto,
) -> Result<CompareReportDto, IpcError> {
    let domain = options.to_domain()?;
    let session = weave_compare::compare_files(
        &weave_files::fs::StdFilesystem,
        std::path::Path::new(&left),
        std::path::Path::new(&right),
        &domain,
        &CancellationToken::new(),
    );
    Ok(CompareReportDto::from_session(&session))
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextDetailDto {
    pub left_encoding: Option<String>,
    pub right_encoding: Option<String>,
    pub line_ending_differs: bool,
    /// true ⇒ 超出尺寸上限，report 为 None（M2.1 §9 诚实降级）。
    pub too_large: bool,
    /// Diff 形状复用 text_dto::DiffReportDto（M4 已导出同一契约，不重复定义）。
    pub report: Option<crate::text_dto::DiffReportDto>,
}

/// 文本详情（按需 L3；超限如实 tooLarge，不假装比较成功）。
#[tauri::command]
#[specta::specta]
pub fn compare_file_detail(
    left: String,
    right: String,
    options: CompareOptionsDto,
) -> Result<TextDetailDto, IpcError> {
    let domain = options.to_domain()?;
    let detail = weave_compare::text_detail(
        &weave_files::fs::StdFilesystem,
        std::path::Path::new(&left),
        std::path::Path::new(&right),
        &domain,
        &CancellationToken::new(),
    )?;
    let report = if detail.too_large {
        None
    } else {
        Some(crate::text_dto::DiffReportDto::from_report(detail.report))
    };
    Ok(TextDetailDto {
        left_encoding: Some(format!("{:?}", detail.left_encoding)),
        right_encoding: Some(format!("{:?}", detail.right_encoding)),
        line_ending_differs: detail.line_ending_differs,
        too_large: detail.too_large,
        report,
    })
}

// ── 目录比较（Job 化） ──

/// 目录比较结果缓存（jobId → 已完成报告；AppState 持有）。
#[derive(Default)]
pub struct CompareResultCache {
    inner: std::sync::Mutex<std::collections::HashMap<String, CompareReportDto>>,
}

impl CompareResultCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, job_id: String, report: CompareReportDto) {
        self.inner
            .lock()
            .expect("compare cache")
            .insert(job_id, report);
    }

    pub fn get(&self, job_id: &str) -> Option<CompareReportDto> {
        self.inner
            .lock()
            .expect("compare cache")
            .get(job_id)
            .cloned()
    }
}

/// 目录比较（Job 化：进度 + 取消；结果经 compare_get_result 领取）。
#[tauri::command]
#[specta::specta]
pub fn compare_directories(
    app: tauri::AppHandle,
    left: String,
    right: String,
    options: CompareOptionsDto,
) -> Result<JobHandleDto, IpcError> {
    let domain = options.to_domain()?;
    if !std::path::Path::new(&left).is_dir() || !std::path::Path::new(&right).is_dir() {
        return Err(err(
            "compare.rootMissing",
            "both compare roots must exist and be directories",
        ));
    }
    let state_cell = app.state::<crate::state::AppState>();
    let (job_id, cancel, sink, _) = state_cell.jobs.register();
    let handle = app.clone();
    drop(state_cell);
    let job_id_for_task = job_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let report = weave_compare::compare_directories(
            &weave_files::fs::StdFilesystem,
            std::path::Path::new(&left),
            std::path::Path::new(&right),
            &domain,
            &cancel,
            &mut |done, total| {
                sink.report(&weave_core::prelude::Progress::running(
                    OperationId::generate(),
                    done,
                    total,
                ));
            },
        );
        let app_state = handle.state::<crate::state::AppState>();
        // 取消的会话同样缓存部分事实（诚实呈现）+ 终态 completed
        app_state.compare_results.insert(
            job_id_for_task.to_string(),
            CompareReportDto::from_session(&report),
        );
        app_state
            .jobs
            .finish(&job_id_for_task, crate::jobs::JobOutcome::CompareCompleted);
    });

    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 领取目录比较结果（未完成 ⇒ compare.notReady）。
#[tauri::command]
#[specta::specta]
pub fn compare_get_result(
    app: tauri::AppHandle,
    job_id: String,
) -> Result<CompareReportDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    state.compare_results.get(&job_id).ok_or_else(|| {
        err(
            "compare.notReady",
            format!("compare result not ready for '{job_id}'"),
        )
    })
}

// ── Actionable：差异 → Copy 计划（复用 M2.0 PlanCache/execute/事务/undo） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompareActionDto {
    pub relative_path: String,
    /// leftToRight | rightToLeft
    pub direction: String,
    /// 比较时的源快照证据（stale 检测依据；可空 ⇒ SizeMtime 档降级）
    pub size: Option<f64>,
    pub modified_ms: Option<f64>,
    pub digest_hex: Option<String>,
}

/// 从差异构建 Copy 计划（进入既有 PlanCache；执行走既有 execute_plan IPC）。
#[tauri::command]
#[specta::specta]
pub fn compare_build_plan(
    app: tauri::AppHandle,
    left_root: String,
    right_root: String,
    actions: Vec<CompareActionDto>,
) -> Result<crate::ops_dto::PlanDto, IpcError> {
    if actions.is_empty() {
        return Err(err("compare.emptySelection", "no differences selected"));
    }
    let left = std::path::Path::new(&left_root);
    let right = std::path::Path::new(&right_root);
    if !left.is_dir() || !right.is_dir() {
        return Err(err(
            "compare.rootMissing",
            "both compare roots must exist and be directories",
        ));
    }
    let mut items = Vec::with_capacity(actions.len());
    for (idx, action) in actions.iter().enumerate() {
        // 路径安全：rel 必须是纯相对段（拒绝绝对路径/越界）
        let rel = std::path::Path::new(&action.relative_path);
        if rel.is_absolute()
            || action
                .relative_path
                .split(['/', '\\'])
                .any(|seg| seg == ".." || seg.is_empty())
        {
            return Err(err(
                "path.relativeTraversal",
                format!("illegal relative path: {}", action.relative_path),
            ));
        }
        let (source, target) = match action.direction.as_str() {
            "leftToRight" => (left.join(rel), right.join(rel)),
            "rightToLeft" => (right.join(rel), left.join(rel)),
            other => {
                return Err(err(
                    "compare.unknownDirection",
                    format!("unknown direction '{other}'"),
                ));
            }
        };
        items.push(PlanItem {
            item_id: format!("item_{idx:04}"),
            source_path: source.to_string_lossy().into_owned(),
            target_path: target.to_string_lossy().into_owned(),
            status: PlanItemStatus::Ready,
            collision: CollisionKind::None,
            source_size: action.size.map(|v| v as u64),
            source_modified: action.modified_ms.and_then(|ms| {
                std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(ms as u64))
            }),
            source_digest: action.digest_hex.clone(),
            warnings: vec![],
            errors: vec![],
        });
    }
    let plan = Plan {
        operation_id: OperationId::generate(),
        kind: OperationKind::Copy,
        created_at: std::time::SystemTime::now(),
        items,
    };
    let state = app.state::<crate::state::AppState>();
    state.plans.insert(&plan);
    Ok(crate::ops_dto::PlanDto::from_plan(&plan))
}

// ── Actionable：Delete → Quarantine（复用 M3 回收站 + undo 分派） ──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompareRecycleResultDto {
    pub operation_id: String,
    pub recycled: f64,
    pub failed: f64,
    pub skipped: f64,
}

/// 从差异删除条目（安全回收 = Quarantine，M2.1 §20）：
/// 复用 M3 recycle_one + `recycle-bin:{token}` 事务目标 ⇒ 既有
/// undo_operation（回收站恢复）原样可用——不是第二套删除。
#[tauri::command]
#[specta::specta]
pub fn compare_recycle(
    app: tauri::AppHandle,
    root: String,
    relative_paths: Vec<String>,
) -> Result<CompareRecycleResultDto, IpcError> {
    use weave_files::recycle::{RECYCLE_TARGET_PREFIX, RecycleOutcome, recycle_paths};
    use weave_history::{
        HistoryEntry, OperationStatus, OperationTransaction, Reversibility, TransactionItem,
        TransactionItemStatus,
    };

    if relative_paths.is_empty() {
        return Err(err("compare.emptySelection", "no paths selected"));
    }
    let base = std::path::Path::new(&root);
    if !base.is_dir() {
        return Err(err(
            "compare.rootMissing",
            "compare root must exist and be a directory",
        ));
    }
    let mut abs_paths = Vec::with_capacity(relative_paths.len());
    for rel in &relative_paths {
        if rel
            .split(['/', std::path::MAIN_SEPARATOR])
            .any(|seg| seg == ".." || seg.is_empty())
        {
            return Err(err(
                "path.relativeTraversal",
                format!("illegal relative path: {rel}"),
            ));
        }
        abs_paths.push(base.join(rel));
    }

    let started = std::time::SystemTime::now();
    let operation_id = OperationId::generate();
    let results = recycle_paths(&abs_paths);
    let mut items = Vec::with_capacity(results.len());
    let (mut recycled, mut failed) = (0u64, 0u64);
    for (path, outcome) in &results {
        match outcome {
            RecycleOutcome::Success(receipt) => {
                recycled += 1;
                items.push(TransactionItem {
                    item_id: format!("item_{:04}", items.len()),
                    source_path: receipt.original_path.to_string_lossy().into_owned(),
                    target_path: match &receipt.token {
                        Some(t) => format!("{RECYCLE_TARGET_PREFIX}{t}"),
                        None => RECYCLE_TARGET_PREFIX.to_string(),
                    },
                    status: TransactionItemStatus::Executed,
                    timestamp: Some(receipt.deleted_at),
                    original_size: None,
                    original_modified: None,
                    original_created: None,
                });
            }
            _ => {
                failed += 1;
                items.push(TransactionItem {
                    item_id: format!("item_{:04}", items.len()),
                    source_path: path.to_string_lossy().into_owned(),
                    target_path: String::new(),
                    status: TransactionItemStatus::NotExecuted,
                    timestamp: Some(started),
                    original_size: None,
                    original_modified: None,
                    original_created: None,
                });
            }
        }
    }
    let skipped = 0u64;
    let reversible = if recycled == 0 {
        Reversibility::None
    } else if failed == 0 {
        Reversibility::Full
    } else {
        Reversibility::Partial
    };
    let tx = OperationTransaction {
        operation_id: operation_id.clone(),
        kind: OperationKind::DuplicateRecycle,
        status: OperationStatus::Completed,
        timestamp: started,
        reversible,
        items,
    };
    // 事务/历史落盘（undo 依赖；写失败不吞操作事实，仅标注）
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    if let Ok(store) = weave_history::HistoryStore::open(&history_dir) {
        let entry = HistoryEntry {
            operation_id: tx.operation_id.clone(),
            kind: tx.kind,
            timestamp: tx.timestamp,
            summary: format!("Recycle {} files (compare)", recycled),
            item_count: tx.items.len() as u64,
            success_count: recycled,
            failed_count: failed,
            skipped_count: skipped,
            undoable: recycled > 0,
            status: OperationStatus::Completed,
            input_root: None,
            rule_summary: None,
        };
        let _ = store
            .save_transaction(&tx)
            .and_then(|()| store.upsert_entry(entry));
    }

    Ok(CompareRecycleResultDto {
        operation_id: operation_id.to_string(),
        recycled: recycled as f64,
        failed: failed as f64,
        skipped: skipped as f64,
    })
}
