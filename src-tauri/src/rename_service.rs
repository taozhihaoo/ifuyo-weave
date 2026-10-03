//! Rename / Organizer 应用服务（M2）。
//!
//! 职责（M2 §93）：parse DTO → 校验 → 调 domain（weave-files / weave-history）
//! → 组装 DTO。执行走 M1 的 JobTracker（进度 + 取消），事务/历史在任务内落盘。
//!
// IPC 边界 Err DTO 体积豁免（同 D10 / m2_commands）。
#![expect(clippy::result_large_err)]

//! Plan 复用（M2 §11 Preview/Execute 同一 Plan）：构建后的 Plan 以
//! operation_id 为键缓存在服务端（[`PlanCache`]），execute 只收 operation_id
//! ——Revalidate 用的源快照（size/mtime）保留在服务端，不经过 IPC。
//! 缓存为内存态：应用重启后旧 Plan 失效（execute 返回结构化错误，
//! 记录 Known Limitations；事务本身的 crash safety 由 InProgress 落盘保证）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;
use weave_core::prelude::{
    CancellationToken, OperationId, OperationKind, Plan, Progress, WeaveError,
};
use weave_files::ExecutionReport;
use weave_history::OperationStatus;

use crate::commands::IpcError;
use crate::ops_dto::{OrganizerRuleDto, PlanDto, RenameRuleDto};

/// 内存 Plan 缓存：build → execute 的会话内衔接点。
#[derive(Default)]
pub struct PlanCache {
    plans: Mutex<HashMap<String, Plan>>,
}

impl PlanCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, plan: &Plan) {
        self.plans
            .lock()
            .expect("plan cache")
            .insert(plan.operation_id.to_string(), plan.clone());
    }

    pub fn take(&self, operation_id: &str) -> Option<Plan> {
        self.plans.lock().expect("plan cache").remove(operation_id)
    }
}

/// 历史目录：`%APPDATA%/ifuyo/Weave/history`（charter #36 路径约定）。
pub fn resolve_history_dir(app: &tauri::AppHandle) -> Result<PathBuf, WeaveError> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Ok(PathBuf::from(appdata)
            .join("ifuyo")
            .join("Weave")
            .join("history"));
    }
    app.path()
        .app_data_dir()
        .map(|p| p.join("history"))
        .map_err(|e| {
            WeaveError::internal(
                "history.pathUnavailable",
                format!("cannot resolve app data dir: {e}"),
            )
        })
}

fn std_fs() -> &'static weave_files::fs::StdFilesystem {
    &weave_files::fs::StdFilesystem
}

/// 构建重命名计划（纯读，无副作用——M2 §36）并缓存。
pub fn build_rename_plan_service(
    cache: &PlanCache,
    inputs: Vec<String>,
    rules: Vec<RenameRuleDto>,
    template: Option<String>,
) -> Result<PlanDto, IpcError> {
    let domain_rules: Result<Vec<_>, _> =
        rules.into_iter().map(RenameRuleDto::into_domain).collect();
    let plan = weave_files::build_rename_plan(
        std_fs(),
        &inputs,
        &domain_rules?,
        template.as_deref(),
        &CancellationToken::new(),
        &mut |_| {},
    )?;
    let dto = PlanDto::from_plan(&plan);
    cache.insert(&plan);
    Ok(dto)
}

/// 构建 Organizer 计划并缓存。
pub fn build_organizer_plan_service(
    cache: &PlanCache,
    root: String,
    rules: Vec<OrganizerRuleDto>,
) -> Result<PlanDto, IpcError> {
    let domain_rules: Result<Vec<_>, _> = rules
        .into_iter()
        .map(OrganizerRuleDto::into_domain)
        .collect();
    let plan = weave_files::build_organizer_plan(
        std_fs(),
        &root,
        &domain_rules?,
        &CancellationToken::new(),
        &mut |_| {},
    )?;
    let dto = PlanDto::from_plan(&plan);
    cache.insert(&plan);
    Ok(dto)
}

/// 任务内执行：Plan → Revalidate + Execute → 事务/历史落盘 → 报告。
///
/// 历史写失败 ≠ 操作失败（M2 §84）：报告照常返回、undoable=false、
/// 每个成功条目附警告。
pub fn run_plan_job(
    history_dir: &std::path::Path,
    plan: &Plan,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<ExecutionReport, (ExecutionReport, WeaveError)> {
    let mut report = weave_files::execute_plan(std_fs(), plan, cancel, on_progress);
    let mut tx = report.transaction.clone();

    let executed_items = tx
        .items
        .iter()
        .filter(|i| i.status == weave_history::TransactionItemStatus::Executed)
        .count();
    tx.reversible = if executed_items == 0 {
        weave_history::Reversibility::None
    } else if executed_items == tx.items.len() {
        weave_history::Reversibility::Full
    } else {
        weave_history::Reversibility::Partial
    };

    let entry = weave_history::HistoryEntry {
        operation_id: tx.operation_id.clone(),
        kind: tx.kind,
        timestamp: tx.timestamp,
        summary: format!(
            "{} {} files",
            if tx.kind == OperationKind::Rename {
                "Rename"
            } else {
                "Move"
            },
            executed_items
        ),
        item_count: tx.items.len() as u64,
        success_count: executed_items as u64,
        failed_count: report.failed,
        skipped_count: report.skipped,
        undoable: executed_items > 0,
        status: OperationStatus::Completed,
        input_root: None,
        rule_summary: None,
    };

    let store = match weave_history::HistoryStore::open(history_dir) {
        Ok(s) => s,
        Err(e) => {
            for item in &mut report.plan.items {
                if item.errors.is_empty() {
                    item.warnings
                        .push("history unavailable; undo may be unavailable".to_string());
                }
            }
            return Err((report, e));
        }
    };

    match store
        .save_transaction(&tx)
        .and_then(|_| store.upsert_entry(entry))
    {
        Ok(()) => Ok(report),
        Err(e) => {
            // M2 §84：操作成功但历史失败——明确告知 Undo 不可用，不谎报全败。
            for item in &mut report.plan.items {
                if item.errors.is_empty() {
                    item.warnings
                        .push("history persistence failed; undo may be unavailable".to_string());
                }
            }
            Err((report, e))
        }
    }
}

/// 执行前落 InProgress 事务（M2 §82/§83 crash safety：崩溃遗留可识别）。
pub fn persist_in_progress(history_dir: &std::path::Path, plan: &Plan) -> Result<(), WeaveError> {
    let store = weave_history::HistoryStore::open(history_dir)?;
    let tx = weave_history::OperationTransaction {
        operation_id: plan.operation_id.clone(),
        kind: plan.kind,
        status: weave_history::OperationStatus::InProgress,
        timestamp: std::time::SystemTime::now(),
        reversible: weave_history::Reversibility::None,
        items: Vec::new(),
    };
    store.save_transaction(&tx)
}

/// Undo：校验事务 → 执行 → 结果。
pub fn undo_operation_service(
    history_dir: &std::path::Path,
    operation_id: &OperationId,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<weave_files::UndoReport, WeaveError> {
    let store = weave_history::HistoryStore::open(history_dir)?;
    let Some(tx) = store.load_transaction(operation_id)? else {
        return Err(WeaveError::validation(
            "undo.unknownOperation",
            format!("no transaction found for '{operation_id}'"),
        )
        .with_location("rename_service::undo"));
    };
    if tx.status == weave_history::OperationStatus::InProgress {
        return Err(WeaveError::conflict(
            "undo.incompleteTransaction",
            "operation was interrupted; recovery required before undo",
        )
        .with_location("rename_service::undo"));
    }
    Ok(weave_files::undo_transaction(
        std_fs(),
        &tx,
        cancel,
        on_progress,
    ))
}

/// 历史查询：最近条目（M2 §56）。
pub fn recent_history(
    history_dir: &std::path::Path,
    limit: usize,
) -> Result<Vec<weave_history::HistoryEntry>, WeaveError> {
    let store = weave_history::HistoryStore::open(history_dir)?;
    store.recent(limit)
}
