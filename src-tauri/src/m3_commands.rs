//! M3：Duplicate Finder 的 IPC 命令（scan → plan → execute；undo 复用 M2 命令）。
//!
//! 命令只做 orchestration（M2 §93 同源纪律）。扫描报告以 scan_id 缓存服务端，
//! build_recycle_plan 只收 scan_id + 选择；Plan 缓存复用 M2 PlanCache。

// IPC 边界与 D10 同理：Err DTO 体积不构成热路径问题（见 DECISIONS.md D10）。
#![expect(clippy::result_large_err)]

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;
use crate::jobs::JobOutcome;
use crate::ops_dto::{PlanDto, PlanReportDto, RecycleSelectionDto, TransactionDto};
use tauri::Manager;
use weave_core::prelude::WeaveError;

fn validate_roots(roots: &[String]) -> Result<(), IpcError> {
    if roots.is_empty() {
        return Err(
            WeaveError::validation("duplicates.emptyRoots", "no scan roots provided").into(),
        );
    }
    for root in roots {
        weave_core::prelude::validate_absolute_path(root)?;
    }
    Ok(())
}

fn parse_min_size(min_size: Option<f64>) -> Result<u64, IpcError> {
    match min_size {
        None => Ok(0),
        Some(v) if v < 0.0 || v != v.trunc() || v > u64::MAX as f64 => Err(WeaveError::validation(
            "duplicates.invalidMinSize",
            format!("minSize '{v}' is out of range"),
        )
        .into()),
        Some(v) => Ok(v as u64),
    }
}

/// 重复扫描任务（三级管线；非阻塞；进度经 get_job 轮询，取消经 cancel_job）。
/// 取消时返回 partial_result 标记的部分报告（M3 §27）。
#[tauri::command]
#[specta::specta]
pub fn scan_duplicates(
    app: tauri::AppHandle,
    roots: Vec<String>,
    min_size: Option<f64>,
) -> Result<JobHandleDto, IpcError> {
    validate_roots(&roots)?;
    let min_size = parse_min_size(min_size)?;

    let state = app.state::<crate::state::AppState>();
    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        match crate::duplicates_service::run_scan_job(&roots, min_size, &cancel, &mut |p| {
            sink.report(&p)
        }) {
            Ok(report) => {
                app_state.scans.insert(&report);
                app_state.jobs.finish(
                    &job_id_for_task,
                    JobOutcome::DuplicateScan(Box::new(
                        crate::files_dto::DuplicateScanReportDto::from_report(&report),
                    )),
                );
            }
            Err(e) => app_state.jobs.fail(&job_id_for_task, e),
        }
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 从扫描结果构建回收计划（每组保留至少一份；快照来自扫描，M3 §61）。
/// Plan 进服务端缓存，execute 只收 operation_id。
#[tauri::command]
#[specta::specta]
pub fn build_recycle_plan(
    app: tauri::AppHandle,
    scan_id: String,
    selections: Vec<RecycleSelectionDto>,
) -> Result<PlanDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    crate::duplicates_service::build_recycle_plan_service(
        &state.scans,
        &state.plans,
        &scan_id,
        selections,
    )
}

/// 执行已确认的回收计划（任务内 Revalidate + 回收站适配器；
/// 事务/历史落盘，undo 经既有 undo_operation 命令）。
#[tauri::command]
#[specta::specta]
pub fn execute_recycle_plan(
    app: tauri::AppHandle,
    operation_id: String,
) -> Result<JobHandleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    let Some(plan) = state.plans.take(&operation_id) else {
        return Err(WeaveError::validation(
            "plan.unknownOrExpired",
            format!("plan '{operation_id}' is unknown or expired (rebuild the plan)"),
        )
        .with_location("m3_commands::execute_recycle_plan")
        .into());
    };
    crate::rename_service::persist_in_progress(&history_dir, &plan)?;

    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let history_dir = crate::rename_service::resolve_history_dir(&handle)
            .unwrap_or_else(|_| std::path::PathBuf::new());
        let (exec, history_ok) = match crate::duplicates_service::run_recycle_job(
            &history_dir,
            &plan,
            &cancel,
            &mut |p| sink.report(&p),
        ) {
            Ok(exec) => (exec, true),
            Err((exec, _history_error)) => (exec, false),
        };
        let undoable = exec.recycled > 0 && history_ok;
        let dto = PlanReportDto {
            operation_id: exec.plan.operation_id.to_string(),
            items: PlanDto::from_plan(&exec.plan).items,
            executed: exec.recycled as f64,
            failed: exec.failed as f64,
            skipped: exec.skipped as f64,
            undoable,
            duration_ms: exec.duration_ms as f64,
            transaction: Some(TransactionDto::from_transaction(&exec.transaction)),
        };
        app_state
            .jobs
            .finish(&job_id_for_task, JobOutcome::RecycleExecuted(Box::new(dto)));
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}
