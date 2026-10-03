//! Recycle 计划执行器（M3 §19–§26 / §58–§60）。
//!
//! 与 M2 execute_plan 分离：Recycle 的目标是回收站（平台适配器），
//! 不是文件系统 rename；事务 target 记录平台 token（诚实表达）。
//! 逐项处理：Revalidate（扫描快照 vs 当前状态）→ 适配器回收 → 事务记账。
//! 取消只在条目间安全点生效（M3 §26）：取消时未处理的条目原样保留，
//! 以 NotExecuted 进事务——绝不在用户取消后仍然回收它们。

use std::path::PathBuf;

use weave_core::prelude::{CancellationToken, Plan, PlanItem, Progress, WeaveError};
use weave_history::{
    OperationStatus, OperationTransaction, TransactionItem, TransactionItemStatus,
};

use crate::recycle::{RECYCLE_TARGET_PREFIX, RecycleAdapter, RecycleOutcome, StdRecycleAdapter};

/// 执行回收计划的完整结果（M3 §58）：更新后的 Plan + 事务 + 计数。
pub struct RecycleExecution {
    pub plan: Plan,
    pub transaction: OperationTransaction,
    pub recycled: u64,
    pub failed: u64,
    pub skipped: u64,
    pub duration_ms: u64,
}

/// 执行回收计划（生产入口：标准平台适配器）。
/// 逐项 Revalidate → 回收站适配器 → 事务记账；
/// 取消/校验失败/回收失败的条目互不牵连（失败隔离，M3 §26）。
pub fn execute_recycle_plan(
    plan: &Plan,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> RecycleExecution {
    execute_recycle_plan_with(plan, &StdRecycleAdapter, cancel, report_progress)
}

/// 注入适配器的执行入口（fault 注入测试 / 未来批处理引擎复用，M3 §91）。
pub fn execute_recycle_plan_with(
    plan: &Plan,
    adapter: &dyn RecycleAdapter,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> RecycleExecution {
    let started = std::time::Instant::now();
    let mut out = plan.clone();
    let mut tx_items: Vec<TransactionItem> = Vec::new();
    let (mut recycled, mut failed, mut skipped) = (0u64, 0u64, 0u64);
    let total = out.items.len() as u64;
    let mut processed: u64 = 0;
    let mut cancelled = false;

    for item in out.items.iter_mut() {
        if cancel.is_cancelled() {
            cancelled = true;
        }
        if cancelled {
            skipped += 1;
            tx_items.push(not_executed(item));
            continue;
        }
        processed += 1;
        report_progress(Progress::running(
            out.operation_id.clone(),
            processed,
            Some(total),
        ));

        // Revalidate：执行前重查（扫描快照 vs 当前状态，M3 §18/§110）
        if let Err(e) = revalidate_before_recycle(item) {
            failed += 1;
            item.errors.push(e);
            tx_items.push(not_executed(item));
            continue;
        }

        let path = PathBuf::from(&item.source_path);
        match adapter.recycle(&path) {
            RecycleOutcome::Success(receipt) => {
                recycled += 1;
                let target = match &receipt.token {
                    Some(token) => format!("{RECYCLE_TARGET_PREFIX}{token}"),
                    None => RECYCLE_TARGET_PREFIX.to_string(),
                };
                tx_items.push(TransactionItem {
                    item_id: item.item_id.clone(),
                    source_path: item.source_path.clone(),
                    target_path: target,
                    status: TransactionItemStatus::Executed,
                    timestamp: Some(receipt.deleted_at),
                    original_size: item.source_size,
                    original_modified: item.source_modified,
                    original_created: None,
                });
            }
            RecycleOutcome::Failed(msg) => {
                failed += 1;
                item.errors.push(WeaveError::io(
                    "recycle.failed",
                    format!("unable to recycle file: {msg}"),
                ));
                tx_items.push(not_executed(item));
            }
            RecycleOutcome::Unsupported(msg) => {
                failed += 1;
                item.errors.push(WeaveError::unsupported(
                    "recycle.unavailable",
                    format!("recycle unavailable: {msg}"),
                ));
                tx_items.push(not_executed(item));
            }
        }
    }

    let reversible = if recycled == 0 {
        weave_history::Reversibility::None
    } else if failed == 0 && skipped == 0 {
        weave_history::Reversibility::Full
    } else {
        weave_history::Reversibility::Partial
    };

    RecycleExecution {
        plan: out,
        transaction: OperationTransaction {
            operation_id: plan.operation_id.clone(),
            kind: plan.kind,
            status: OperationStatus::Completed,
            timestamp: started_wall(),
            reversible,
            items: tx_items,
        },
        recycled,
        failed,
        skipped,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// 未执行条目的事务记录（不制造假恢复依据，M2 §50 同源纪律）。
fn not_executed(item: &PlanItem) -> TransactionItem {
    TransactionItem {
        item_id: item.item_id.clone(),
        source_path: item.source_path.clone(),
        target_path: String::new(),
        status: TransactionItemStatus::NotExecuted,
        timestamp: None,
        original_size: item.source_size,
        original_modified: item.source_modified,
        original_created: None,
    }
}

fn started_wall() -> std::time::SystemTime {
    std::time::SystemTime::now()
}

/// Revalidate（M3 §18/§110）：source 存在 + size 匹配扫描快照。
fn revalidate_before_recycle(item: &PlanItem) -> Result<(), WeaveError> {
    let path = std::path::Path::new(&item.source_path);
    match std::fs::metadata(path) {
        Err(e) => {
            let code = if e.kind() == std::io::ErrorKind::NotFound {
                "rename.sourceNotFound"
            } else {
                "rename.statFailed"
            };
            Err(WeaveError::io(
                code,
                format!("source revalidation failed: {}", item.source_path),
            ))
        }
        // 快照大小不符 ⇒ 扫描后文件已被改动，拒绝回收（M3 §18 changedSinceScan）
        Ok(m) if item.source_size.is_some_and(|expected| m.len() != expected) => {
            Err(WeaveError::conflict(
                "duplicates.changedSinceScan",
                format!(
                    "source changed since scan (size {} != {})",
                    m.len(),
                    item.source_size.unwrap_or_default()
                ),
            )
            .with_location("weave-files::recycle_exec"))
        }
        Ok(_) => Ok(()),
    }
}
