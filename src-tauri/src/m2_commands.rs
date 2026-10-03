//! M2：Rename / Organizer 的 IPC 命令（build → execute → history → undo）。
//!
//! 命令只做 orchestration（M2 §93）。Plan 由服务端缓存（operation_id 为键，
//! M2 §11 Preview/Execute 同一 Plan）；Execute 在任务内 Revalidate + 执行。

// IPC 边界与 D10 同理：Err DTO 体积不构成热路径问题（见 DECISIONS.md D10）。
#![expect(clippy::result_large_err)]

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;
use crate::jobs::JobOutcome;
use crate::ops_dto::{HistoryEntryDto, OrganizerRuleDto, PlanDto, RenameRuleDto, TransactionDto};
use tauri::Manager;
use weave_core::prelude::{OperationId, WeaveError};

#[tauri::command]
#[specta::specta]
pub fn build_rename_plan(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    rules: Vec<RenameRuleDto>,
    template: Option<String>,
) -> Result<PlanDto, IpcError> {
    let cache = &app.state::<crate::state::AppState>().plans;
    crate::rename_service::build_rename_plan_service(cache, inputs, rules, template)
}

#[tauri::command]
#[specta::specta]
pub fn build_organizer_plan(
    app: tauri::AppHandle,
    root: String,
    rules: Vec<OrganizerRuleDto>,
) -> Result<PlanDto, IpcError> {
    let cache = &app.state::<crate::state::AppState>().plans;
    crate::rename_service::build_organizer_plan_service(cache, root, rules)
}

/// 执行已确认的 Plan（服务端缓存取回；任务内 Revalidate + Safe Rename）。
#[tauri::command]
#[specta::specta]
pub fn execute_plan(app: tauri::AppHandle, operation_id: String) -> Result<JobHandleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    let Some(plan) = state.plans.take(&operation_id) else {
        return Err(WeaveError::validation(
            "plan.unknownOrExpired",
            format!("plan '{operation_id}' is unknown or expired (rebuild the plan)"),
        )
        .with_location("m2_commands::execute_plan")
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
        let outcome =
            match crate::rename_service::run_plan_job(&history_dir, &plan, &cancel, &mut |p| {
                sink.report(&p)
            }) {
                Ok(report) => {
                    let tx = weave_history::HistoryStore::open(&history_dir)
                        .and_then(|s| s.load_transaction(&plan.operation_id))
                        .ok()
                        .flatten();
                    JobOutcome::PlanExecuted {
                        report: Box::new(crate::ops_dto::ExecutionReportDto::from_report(
                            &report,
                            tx.is_some(),
                        )),
                        transaction: tx.as_ref().map(|tx| {
                            Box::new(crate::ops_dto::TransactionDto::from_transaction(tx))
                        }),
                    }
                }
                Err((report, _history_error)) => JobOutcome::PlanExecuted {
                    report: Box::new(crate::ops_dto::ExecutionReportDto::from_report(
                        &report, false,
                    )),
                    transaction: None,
                },
            };
        app_state.jobs.finish(&job_id_for_task, outcome);
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 撤销一个历史操作（LIFO + 占位暂存；任务内执行）。
#[tauri::command]
#[specta::specta]
pub fn undo_operation(
    app: tauri::AppHandle,
    operation_id: String,
) -> Result<JobHandleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    let store = weave_history::HistoryStore::open(&history_dir)?;
    let op =
        OperationId::parse(&operation_id).map_err(|e| WeaveError::validation("undo.badId", e))?;
    let Some(tx) = store.load_transaction(&op)? else {
        return Err(WeaveError::validation(
            "undo.unknownOperation",
            format!("no transaction found for '{operation_id}'"),
        )
        .into());
    };
    if tx.status == weave_history::OperationStatus::InProgress {
        return Err(WeaveError::conflict(
            "undo.incompleteTransaction",
            "operation was interrupted; recovery required before undo",
        )
        .into());
    }
    if tx.reversible == weave_history::Reversibility::None {
        return Err(WeaveError::unsupported(
            "undo.notReversible",
            "this operation recorded no reversible changes",
        )
        .into());
    }

    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let undo = crate::rename_service::undo_operation_service(
            &crate::rename_service::resolve_history_dir(&handle)
                .unwrap_or_else(|_| std::path::PathBuf::new()),
            &op,
            &cancel,
            &mut |p| sink.report(&p),
        );
        match undo {
            Ok(u) => app_state.jobs.finish(
                &job_id_for_task,
                JobOutcome::Undo {
                    restored: u.restored as f64,
                    conflicts: u.conflicts as f64,
                    leaked_temps: u.leaked_temps as f64,
                },
            ),
            Err(e) => app_state.jobs.fail(&job_id_for_task, e),
        }
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 最近操作（M2 §56 latest first）。
#[tauri::command]
#[specta::specta]
pub fn get_history(
    app: tauri::AppHandle,
    limit: Option<f64>,
) -> Result<Vec<HistoryEntryDto>, IpcError> {
    let limit = match limit {
        Some(v) if (1.0..=500.0).contains(&v) => v as usize,
        _ => 50,
    };
    let dir = crate::rename_service::resolve_history_dir(&app)?;
    let entries = crate::rename_service::recent_history(&dir, limit)?;
    Ok(entries.iter().map(HistoryEntryDto::from_entry).collect())
}

/// 读取某操作的事务详情（Undo 前检查）。
#[tauri::command]
#[specta::specta]
pub fn get_operation(
    app: tauri::AppHandle,
    operation_id: String,
) -> Result<TransactionDto, IpcError> {
    let dir = crate::rename_service::resolve_history_dir(&app)?;
    let store = weave_history::HistoryStore::open(&dir)?;
    let op =
        OperationId::parse(&operation_id).map_err(|e| WeaveError::validation("undo.badId", e))?;
    let tx = store.load_transaction(&op)?.ok_or_else(|| {
        WeaveError::validation(
            "undo.unknownOperation",
            format!("no transaction for '{operation_id}'"),
        )
    })?;
    Ok(TransactionDto::from_transaction(&tx))
}
