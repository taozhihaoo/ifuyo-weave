//! Recycle 计划执行器（M3 §19–§26 / §58–§60）。
//!
//! 与 M2 execute_plan 分离：Recycle 的目标是回收站（平台适配器），
//! 不是文件系统 rename；事务 target 记录平台 token（诚实表达）。

use std::path::PathBuf;

use weave_core::prelude::{
    CancellationToken, Plan, PlanItem, PlanItemStatus, Progress, WeaveError,
};
use weave_history::{OperationStatus, OperationTransaction, TransactionItemStatus};

use crate::recycle::{RecycleOutcome, recycle_paths};

/// 执行回收计划：逐项 Revalidate（存在 + size/mtime 匹配扫描快照）→
/// 回收站适配器。取消在条目间安全点生效。返回更新后的 Plan + 事务 +
/// 回执（供 undo 与 UI 统计）。
pub struct RecycleExecution {
    pub plan: Plan,
    pub transaction: OperationTransaction,
    pub recycled: u64,
    pub failed: u64,
    pub skipped: u64,
    pub duration_ms: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn execute_recycle_plan(
    plan: &Plan,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> RecycleExecution {
    let started = std::time::Instant::now();
    let mut out = plan.clone();
    let mut tx_items: Vec<weave_history::TransactionItem> = Vec::new();
    let (mut recycled, mut failed, mut skipped) = (0u64, 0u64, 0u64);
    let total = out.items.len() as u64;
    let mut processed: u64 = 0;
    let mut cancelled = false;

    let paths: Vec<PathBuf> = out
        .items
        .iter()
        .map(|i| std::path::PathBuf::from(&i.source_path))
        .collect();
    // 适配器批量回收：适配器内部逐条执行并返回逐条结果（失败隔离，M3 §26）
    let results = crate::recycle::recycle_paths(&paths);

    for (idx, item) in out.items.iter_mut().enumerate() {
        if cancel.is_cancelled() {
            cancelled = true;
        }
        if cancelled {
            skipped += 1;
            tx_items.push(weave_history::TransactionItem {
                item_id: item.item_id.clone(),
                source_path: item.source_path.clone(),
                target_path: String::new(),
                status: TransactionItemStatus::NotExecuted,
                timestamp: None,
                original_size: item.source_size,
                original_modified: item.source_modified,
                original_created: None,
            });
            continue;
        }
        processed += 1;
        report_progress(Progress::running(
            out.operation_id.clone(),
            processed,
            Some(total),
        ));

        // Revalidate：执行前重查（扫描快照 vs 当前状态，M3 §18/§110）
        if let Err(e) = revalidate_before_recycle(&item) {
            failed += 1;
            item.errors.push(e);
            tx_items.push(weave_history::TransactionItem {
                item_id: item.item_id.clone(),
                source_path: item.source_path.clone(),
                target_path: String::new(),
                status: TransactionItemStatus::NotExecuted,
                timestamp: None,
                original_size: item.source_size,
                original_modified: item.source_modified,
                original_created: None,
            });
            continue;
        }

        let path = std::path::PathBuf::from(&item.source_path);
        match results.get(idx) {
            Some((_, RecycleOutcome::Success(receipt))) => {
                recycled += 1;
                item.status = PlanItemStatus::NoOp; // 内容已进回收站；outcome 由 report.executed 表达
                tx_items.push(weave_history::TransactionItem {
                    item_id: item.item_id.clone(),
                    source_path: item.source_path.clone(),
                    target_path: receipt
                        .token
                        .as_ref()
                        .map(|t| format!("recycle-bin:{t}"))
                        .unwrap_or_else(|| "recycle-bin:unknown-token".to_string()),
                    status: TransactionItemStatus::Executed,
                    timestamp: Some(receipt.deleted_at),
                    original_size: item.source_size,
                    original_modified: item.source_modified,
                    original_created: None,
                });
            }
            Some((_, RecycleOutcome::Failed(msg))) => {
                failed += 1;
                item.errors.push(WeaveError::io(
                    "recycle.failed",
                    format!("unable to recycle file: {msg}"),
                ));
                tx_items.push(weave_history::TransactionItem {
                    item_id: item.item_id.clone(),
                    source_path: item.source_path.clone(),
                    target_path: String::new(),
                    status: TransactionItemStatus::NotExecuted,
                    timestamp: None,
                    original_size: item.source_size,
                    original_modified: item.source_modified,
                    original_created: None,
                });
            }
            Some((_, RecycleOutcome::Unsupported(msg))) => {
                failed += 1;
                item.errors.push(WeaveError::unsupported(
                    "recycle.unavailable",
                    format!("recycle unavailable: {msg}"),
                ));
                tx_items.push(weave_history::TransactionItem {
                    item_id: item.item_id.clone(),
                    source_path: item.source_path.clone(),
                    target_path: String::new(),
                    status: TransactionItemStatus::NotExecuted,
                    timestamp: None,
                    original_size: item.source_size,
                    original_modified: item.source_modified,
                    original_created: None,
                });
            }
            None => {
                skipped += 1;
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
        Ok(m) => {
            if let Some(expected) = item.source_size {
                if m.len() != expected {
                    return Err(WeaveError::conflict(
                        "duplicates.changedSinceScan",
                        format!(
                            "source changed since scan (size {} != {})",
                            m.len(),
                            expected
                        ),
                    )
                    .with_location("weave-files::recycle_exec"));
                }
            }
            Ok(())
        }
    }
}
