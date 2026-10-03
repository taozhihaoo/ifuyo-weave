//! M3 Duplicate Finder 应用服务（scan → plan → execute → history → undo）。
//!
//! 职责（同 M2 §93 纪律）：parse DTO → 校验 → 调 domain → 组装 DTO。
//! 执行走 M1 的 JobTracker（进度 + 取消）；事务/历史在任务内落盘。
//!
//! Scan 复用（同 PlanCache 纪律）：扫描报告以 scan_id 缓存在服务端，
//! build_recycle_plan 只收 scan_id + 选择——扫描快照不经过 IPC。
//! 缓存为内存态：应用重启后旧 scan_id 失效（结构化错误，重扫即可）。

// IPC 边界 Err DTO 体积豁免（同 D10 / m2_commands）。
#![expect(clippy::result_large_err)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use weave_core::prelude::{CancellationToken, OperationKind, Plan, Progress, WeaveError};
use weave_files::{DuplicateScanReport, RecycleExecution};

use crate::commands::IpcError;
use crate::ops_dto::{PlanDto, RecycleSelectionDto};
use crate::rename_service::PlanCache;

/// 内存扫描缓存：scan → build_recycle_plan 的会话内衔接点。
#[derive(Default)]
pub struct ScanCache {
    scans: Mutex<HashMap<String, DuplicateScanReport>>,
}

impl ScanCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, report: &DuplicateScanReport) {
        self.scans
            .lock()
            .expect("scan cache")
            .insert(report.scan_id.to_string(), report.clone());
    }

    pub fn get(&self, scan_id: &str) -> Option<DuplicateScanReport> {
        self.scans.lock().expect("scan cache").get(scan_id).cloned()
    }
}

/// 重复扫描（三级管线；协作取消 ⇒ partial_result 报告，M3 §27）。
pub fn run_scan_job(
    roots: &[String],
    min_size: u64,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<DuplicateScanReport, WeaveError> {
    weave_files::scan_duplicates(
        &weave_files::fs::StdFilesystem,
        roots,
        min_size,
        cancel,
        on_progress,
    )
}

/// 从缓存扫描报告构建回收计划，并把 Plan 放进 PlanCache（execute 衔接）。
pub fn build_recycle_plan_service(
    scans: &ScanCache,
    plans: &PlanCache,
    scan_id: &str,
    selections: Vec<RecycleSelectionDto>,
) -> Result<PlanDto, IpcError> {
    let report = scans.get(scan_id).ok_or_else(|| {
        WeaveError::validation(
            "duplicates.scanUnknownOrExpired",
            format!("scan '{scan_id}' is unknown or expired (rescan first)"),
        )
        .with_location("duplicates_service::build_recycle_plan")
    })?;
    let domain: Vec<weave_files::RecycleSelection> = selections
        .into_iter()
        .map(RecycleSelectionDto::into_domain)
        .collect();
    let plan = weave_files::build_recycle_plan(&report, &domain)?;
    let dto = PlanDto::from_plan(&plan);
    plans.insert(&plan);
    Ok(dto)
}

/// 任务内执行回收：Plan → Revalidate + 回收站适配器 → 事务/历史落盘。
///
/// 历史写失败 ≠ 操作失败（M2 §84 同源纪律）：执行照常完成、undoable=false、
/// 每个成功条目附警告。
pub fn run_recycle_job(
    history_dir: &Path,
    plan: &Plan,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<RecycleExecution, (RecycleExecution, WeaveError)> {
    let exec = weave_files::execute_recycle_plan(plan, cancel, on_progress);

    let entry = weave_history::HistoryEntry {
        operation_id: exec.transaction.operation_id.clone(),
        kind: exec.transaction.kind,
        timestamp: exec.transaction.timestamp,
        summary: format!("Recycle {} files", exec.recycled),
        item_count: exec.transaction.items.len() as u64,
        success_count: exec.recycled,
        failed_count: exec.failed,
        skipped_count: exec.skipped,
        undoable: exec.recycled > 0,
        status: weave_history::OperationStatus::Completed,
        input_root: None,
        rule_summary: None,
    };

    let store = match weave_history::HistoryStore::open(history_dir) {
        Ok(s) => s,
        Err(e) => {
            warn_history_unavailable(&exec);
            return Err((exec, e));
        }
    };

    match store
        .save_transaction(&exec.transaction)
        .and_then(|_| store.upsert_entry(entry))
    {
        Ok(()) => Ok(exec),
        Err(e) => {
            warn_history_unavailable(&exec);
            Err((exec, e))
        }
    }
}

/// 历史不可用时如实标注（不谎报操作失败）。
fn warn_history_unavailable(exec: &RecycleExecution) {
    for item in &exec.plan.items {
        if item.errors.is_empty() {
            // plan 是执行后的克隆快照；这里的标注仅供 DTO 层提示（M2 §84 同源）。
            tracing::warn!(
                operation_id = %exec.transaction.operation_id,
                "history unavailable; undo may be unavailable"
            );
        }
    }
}

/// Undo 分派：DuplicateRecycle 事务走回收站恢复，其余走 M2 rename undo。
pub fn undo_dispatch(
    tx: &weave_history::OperationTransaction,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(Progress),
) -> weave_files::UndoReport {
    if tx.kind == OperationKind::DuplicateRecycle {
        weave_files::recycle::undo_recycle_transaction(tx, cancel, on_progress)
    } else {
        weave_files::undo_transaction(&weave_files::fs::StdFilesystem, tx, cancel, on_progress)
    }
}
