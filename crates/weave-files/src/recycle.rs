//! 回收站适配器（M3 §22–§25 / §58–§60）。
//!
//! - **平台适配器隔离**：Shell API 只在此模块出现，Domain/UI 不直接调用
//!   （M3 §23/§116）。默认动作 = Move to Recycle Bin，绝不永久删除（M3 §22/§57）。
//! - 实现：`trash` crate 5.2（MIT；Windows 走 Shell 回收站 API，无 shell 进程，
//!   M3 §24 禁 shell 不违反——是进程内 API 调用）。
//! - **诚实表达能力**（M3 §58/§94）：Windows 可列出回收站条目（original_path、
//!   平台 id 与删除时间），Undo 以「original_path + 删除时间」匹配后恢复；
//!   若匹配不到或不唯一 ⇒ 如实报告，不猜路径、不覆盖。
//! - **Recycle Failure**：失败就是失败，不假装删除（M3 §25）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use weave_core::prelude::{CancellationToken, Progress, WeaveError};
use weave_history::{OperationTransaction, TransactionItemStatus};

use crate::undo::{UndoItemResult, UndoItemStatus, UndoReport};

/// 事务 target 中回收站 token 的前缀；后缀为空表示平台未提供 token
/// （Undo 时退化为 original_path + 时间窗匹配）。
pub const RECYCLE_TARGET_PREFIX: &str = "recycle-bin:";

/// 单次回收的回执：事务与 Undo 的平台事实来源（M3 §58）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecycleReceipt {
    pub original_path: PathBuf,
    /// 平台回收条目 token（Windows: trash id 序列化）。不可得时 None。
    pub token: Option<String>,
    /// 删除时刻（UTC 秒，用于回收站匹配消歧）。
    pub deleted_at: SystemTime,
}

/// 单条回收结果（M3 §94 Domain contract：不泄漏平台类型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecycleOutcome {
    Success(RecycleReceipt),
    /// 平台/环境不支持回收站。
    Unsupported(String),
    Failed(String),
}

/// 回收适配器接口（M3 §91 fault 注入缝隙；§94 平台隔离边界）。
/// Domain 与执行器只面向此 trait；生产实现走进程内 Shell 回收站 API。
pub trait RecycleAdapter {
    fn recycle(&self, path: &Path) -> RecycleOutcome;
}

/// 标准适配器：`trash` crate（Windows IFileOperation，进程内调用，非 shell 进程）。
pub struct StdRecycleAdapter;

impl RecycleAdapter for StdRecycleAdapter {
    fn recycle(&self, path: &Path) -> RecycleOutcome {
        recycle_one(path)
    }
}

/// 批量回收：逐条执行并返回逐条结果（失败隔离，M3 §26）。
pub fn recycle_paths(paths: &[PathBuf]) -> Vec<(PathBuf, RecycleOutcome)> {
    let adapter = StdRecycleAdapter;
    paths
        .iter()
        .map(|p| (p.clone(), adapter.recycle(p)))
        .collect()
}

fn recycle_one(path: &Path) -> RecycleOutcome {
    let deleted_at = SystemTime::now();
    match trash::delete(path) {
        Ok(()) => {
            // Windows：删除后从回收站列表按 original_path 反查 token；
            // 找不到时 token=None（诚实表达「平台只保证 success/failure」）。
            let token = find_token(path, deleted_at);
            RecycleOutcome::Success(RecycleReceipt {
                original_path: path.to_path_buf(),
                token,
                deleted_at,
            })
        }
        Err(e) => RecycleOutcome::Failed(format!("recycle failed: {e}")),
    }
}

fn find_token(path: &Path, deleted_at: SystemTime) -> Option<String> {
    let items = trash::os_limited::list().ok()?;
    let wanted = path.to_path_buf();
    let deleted_secs = secs(deleted_at);
    let mut matches: Vec<_> = items
        .into_iter()
        .filter(|item| item.original_path() == wanted)
        .filter(|item| {
            // 时间窗匹配（±5s）：回收站列表时间精度平台不一
            let item_secs = secs(
                SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(item.time_deleted.max(0) as u64),
            );
            let diff = item_secs.abs_diff(deleted_secs);
            diff <= 5
        })
        .collect();
    match matches.len() {
        1 => {
            let item = matches.remove(0);
            Some(format!("{:?}", item.id))
        }
        _ => None,
    }
}

fn secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Undo：从回收站恢复「实际成功回收」的文件（M3 §59–§60）。
///
/// - 以事务回执匹配回收站条目（original_path + token 优先，时间窗兜底）。
/// - 原位已被用户重建 ⇒ 该条目 UndoConflict，**不恢复**（M3 §67）。
/// - 其余条目 `restore_all` 恢复；恢复后逐条校验原位存在。
pub fn restore_recycled(
    receipts: &[RecycleReceipt],
) -> Vec<(RecycleReceipt, crate::undo::UndoItemStatus, Option<String>)> {
    let mut results = Vec::with_capacity(receipts.len());
    let mut staged: Vec<(RecycleReceipt, trash::TrashItem)> = Vec::new();

    // 1) 匹配回收站条目；原位被占的先记冲突
    for receipt in receipts {
        let original = &receipt.original_path;
        if Path::new(original).exists() {
            results.push((
                receipt.clone(),
                crate::undo::UndoItemStatus::UndoConflict,
                Some("original location is occupied".to_string()),
            ));
            continue;
        }
        match list_matching(receipt) {
            Ok(Some(item)) => {
                staged.push((receipt.clone(), item));
            }
            Ok(None) => {
                results.push((
                    receipt.clone(),
                    crate::undo::UndoItemStatus::Missing,
                    Some("no matching recycle-bin entry found".to_string()),
                ));
            }
            Err(e) => {
                results.push((
                    receipt.clone(),
                    crate::undo::UndoItemStatus::Missing,
                    Some(format!("recycle bin list failed: {e}")),
                ));
            }
        }
    }

    // 2) 批量恢复（Windows IFileOperation；失败/部分失败如实报告）
    let items: Vec<trash::TrashItem> = staged.iter().map(|(_, i)| i.clone()).collect();
    if let Err(e) = trash::os_limited::restore_all(items) {
        for (receipt, _) in &staged {
            results.push((
                receipt.clone(),
                crate::undo::UndoItemStatus::UndoConflict,
                Some(format!("restore failed: {e}")),
            ));
        }
        return results;
    }

    // 3) 恢复后校验
    for (receipt, _) in staged {
        if receipt.original_path.exists() {
            results.push((receipt, crate::undo::UndoItemStatus::Restored, None));
        } else {
            results.push((
                receipt,
                crate::undo::UndoItemStatus::UndoConflict,
                Some("restore reported success but original path still missing".to_string()),
            ));
        }
    }
    results
}

fn list_matching(receipt: &RecycleReceipt) -> Result<Option<trash::TrashItem>, trash::Error> {
    let items = trash::os_limited::list()?;
    let original_norm = receipt.original_path.to_string_lossy().to_lowercase();
    let window = 5u64;
    let deleted_secs = secs(receipt.deleted_at);

    let mut matches: Vec<trash::TrashItem> = items
        .into_iter()
        .filter(|item| item.original_path().to_string_lossy().to_lowercase() == original_norm)
        .filter(|item| {
            if let Some(token) = &receipt.token {
                return format!("{:?}", item.id) == *token;
            }
            let item_secs =
                secs(UNIX_EPOCH + std::time::Duration::from_secs(item.time_deleted.max(0) as u64));
            item_secs.abs_diff(deleted_secs) <= window
        })
        .collect();

    match matches.len() {
        1 => Ok(Some(matches.remove(0))),
        // 0 = 条目已被用户清空；>1 = 无法唯一匹配——都不猜测
        _ => Ok(None),
    }
}

/// Undo 入口（M3 §59–§60）：从事务重建回执 → 从回收站恢复 → 逐条校验。
///
/// - Executed 条目按 `recycle-bin:{token}` target 重建回执（token 空 ⇒
///   original_path + 时间窗匹配）。
/// - NotExecuted / NoOp：NotUndoable，绝不制造假恢复记录（M2 §50 同源纪律）。
/// - 恢复是单次批量平台调用：取消仅在开始前生效（之后为一次性原子动作）。
pub fn undo_recycle_transaction(
    transaction: &OperationTransaction,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> UndoReport {
    let started = std::time::Instant::now();
    let mut results: Vec<UndoItemResult> = Vec::new();
    let (mut restored, mut conflicts, mut skipped) = (0u64, 0u64, 0u64);

    if cancel.is_cancelled() {
        for item in &transaction.items {
            results.push(UndoItemResult {
                item_id: item.item_id.clone(),
                status: UndoItemStatus::NotUndoable,
                reason: Some(
                    WeaveError::cancelled(
                        "undo.cancelledBeforeItem",
                        "undo was cancelled before starting",
                    )
                    .with_location("weave-files::recycle"),
                ),
            });
            skipped += 1;
        }
        return UndoReport {
            operation_id: transaction.operation_id.clone(),
            results,
            restored,
            conflicts,
            skipped,
            leaked_temps: 0,
            duration_ms: started.elapsed().as_millis() as u64,
        };
    }

    report_progress(Progress::running(
        transaction.operation_id.clone(),
        0,
        Some(1),
    ));

    // 1) 从事务重建回执
    let mut receipts: Vec<RecycleReceipt> = Vec::new();
    let mut id_by_path: HashMap<PathBuf, String> = HashMap::new();
    for item in &transaction.items {
        if !matches!(item.status, TransactionItemStatus::Executed) {
            skipped += 1;
            results.push(UndoItemResult {
                item_id: item.item_id.clone(),
                status: UndoItemStatus::NotUndoable,
                reason: None,
            });
            continue;
        }
        match receipt_from_tx_item(item) {
            Some(receipt) => {
                id_by_path.insert(receipt.original_path.clone(), item.item_id.clone());
                receipts.push(receipt);
            }
            None => {
                skipped += 1;
                results.push(UndoItemResult {
                    item_id: item.item_id.clone(),
                    status: UndoItemStatus::NotUndoable,
                    reason: Some(
                        WeaveError::validation(
                            "duplicates.recycleTargetMalformed",
                            format!(
                                "transaction target is not a recycle-bin token: {}",
                                item.target_path
                            ),
                        )
                        .with_location("weave-files::recycle"),
                    ),
                });
            }
        }
    }

    // 2) 批量恢复 + 逐条校验
    report_progress(Progress::running(
        transaction.operation_id.clone(),
        1,
        Some(1),
    ));
    for (receipt, status, reason) in restore_recycled(&receipts) {
        let item_id = id_by_path
            .get(&receipt.original_path)
            .cloned()
            .unwrap_or_else(|| receipt.original_path.to_string_lossy().into_owned());
        let (status, reason) = match status {
            UndoItemStatus::Missing => (
                UndoItemStatus::Missing,
                reason.map(|m| {
                    WeaveError::io("duplicates.recycleEntryMissing", m)
                        .with_location("weave-files::recycle")
                }),
            ),
            other => (
                other,
                reason.map(|m| {
                    WeaveError::conflict("duplicates.undoConflict", m)
                        .with_location("weave-files::recycle")
                }),
            ),
        };
        match status {
            UndoItemStatus::Restored => restored += 1,
            UndoItemStatus::UndoConflict => conflicts += 1,
            _ => skipped += 1,
        }
        results.push(UndoItemResult {
            item_id,
            status,
            reason,
        });
    }

    UndoReport {
        operation_id: transaction.operation_id.clone(),
        results,
        restored,
        conflicts,
        skipped,
        leaked_temps: 0,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// 从事务条目重建回执：target = `recycle-bin:{token}`（token 可为空）。
fn receipt_from_tx_item(item: &weave_history::TransactionItem) -> Option<RecycleReceipt> {
    let token_raw = item.target_path.strip_prefix(RECYCLE_TARGET_PREFIX)?;
    let token = if token_raw.is_empty() {
        None
    } else {
        Some(token_raw.to_string())
    };
    Some(RecycleReceipt {
        original_path: PathBuf::from(&item.source_path),
        token,
        deleted_at: item.timestamp?,
    })
}
