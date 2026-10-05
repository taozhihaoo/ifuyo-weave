//! 回收站适配器（M3 §22–§25 / §58–§60）。
//!
//! - **平台适配器隔离**：Shell API 只在此模块出现，Domain/UI 不直接调用
//!   （M3 §23/§116）。默认动作 = Move to Recycle Bin，绝不永久删除（M3 §22/§57）。
//! - 实现：`trash` crate 5.2（MIT；Windows 走 Shell 回收站 API，无 shell 进程，
//!   M3 §24 禁 shell 不违反——是进程内 API 调用）。
//! - **诚实表达能力**（M3 §58/§94）：Windows 可列出回收站条目（original_path、
//!   平台 id 与删除时间），Undo 以「token 优先 + 归一化 original_path + 删除
//!   时间」匹配后恢复；若匹配不到或不唯一 ⇒ 如实报告，不猜路径、不覆盖。
//! - **Recycle Failure**：失败就是失败，不假装删除（M3 §25）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
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

/// 回收站匹配用的路径归一化：小写 + 剥离 `\\?\` / `\\?\UNC\` 扩展前缀。
/// Shell 列表与本地路径的表示可能不一致（尤其 CI/网络路径）。
fn normalize_path(s: &str) -> String {
    let lower = s.to_lowercase();
    if let Some(rest) = lower.strip_prefix(r"\\?\unc\") {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = lower.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    lower
}

fn item_time_secs(item: &trash::TrashItem) -> u64 {
    item.time_deleted.max(0) as u64
}

fn find_token(path: &Path, deleted_at: SystemTime) -> Option<String> {
    let wanted = normalize_path(&path.to_string_lossy());
    let deleted_secs = secs(deleted_at);
    // Shell 命名空间的可见性可能滞后于 delete 返回（CI 慢环境常见）：
    // 短重试窗口内仍找不到才接受 token=None（诚实退化，不伪造）。
    for attempt in 0..6u32 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let items = match trash::os_limited::list() {
            Ok(items) => items,
            Err(_) => continue,
        };
        // 新鲜度护栏：候选条目不得早于本次删除时刻（-2s 容忍秒级精度），
        // 防止把用户更早删除的同路径旧条目误认为本次条目。
        let mut scored: Vec<(u64, String)> = items
            .into_iter()
            .filter(|item| normalize_path(&item.original_path().to_string_lossy()) == wanted)
            .filter(|item| item_time_secs(item) + 2 >= deleted_secs)
            .map(|item| {
                let diff = item_time_secs(&item).abs_diff(deleted_secs);
                (diff, format!("{:?}", item.id))
            })
            .collect();
        scored.sort_by_key(|(diff, _)| *diff);
        let token = match (scored.first(), scored.get(1)) {
            (Some((_, id)), None) => Some(id.clone()),
            (Some((d0, id)), Some((d1, _))) if d0 < d1 => Some(id.clone()),
            _ => None, // 无候选或时间平局：不猜
        };
        if token.is_some() {
            return token;
        }
    }
    None
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
    let wanted = normalize_path(&receipt.original_path.to_string_lossy());
    let deleted_secs = secs(receipt.deleted_at);

    // 1) token 精确匹配（最强平台证据；token 由回收时刻的新鲜度护栏捕获）
    if let Some(token) = &receipt.token {
        let by_token: Vec<trash::TrashItem> = items
            .iter()
            .filter(|item| format!("{:?}", item.id) == *token)
            .cloned()
            .collect();
        if by_token.len() == 1 {
            return Ok(by_token.into_iter().next());
        }
        // token 失配 ⇒ 平台 id 表示可能随会话变化，退回路径匹配，不直接 Missing
    }

    // 2) 归一化路径匹配：唯一 ⇒ 唯一候选即事实（容忍 CI 时钟粒度与慢速
    //    shell 导致的大时间偏差）；多个 ⇒ 时间最近且唯一者；平局 ⇒ 不猜。
    let by_path: Vec<trash::TrashItem> = items
        .into_iter()
        .filter(|item| normalize_path(&item.original_path().to_string_lossy()) == wanted)
        .collect();
    match by_path.len() {
        0 => Ok(None),
        1 => Ok(by_path.into_iter().next()),
        _ => {
            let mut scored: Vec<(u64, trash::TrashItem)> = by_path
                .into_iter()
                .map(|item| {
                    let diff = item_time_secs(&item).abs_diff(deleted_secs);
                    (diff, item)
                })
                .collect();
            scored.sort_by_key(|(diff, _)| *diff);
            match (scored.first(), scored.get(1)) {
                (Some((d0, item)), Some((d1, _))) if d0 < d1 => Ok(Some(item.clone())),
                _ => Ok(None), // 时间平局：无法唯一归属，如实 Missing
            }
        }
    }
}

/// 回收站端到端可用性探测（集成测试用）：真实回收一个本进程创建的探针
/// 文件 → 匹配 → 恢复 → 校验原位。任一环节失败 ⇒ false（该环境回收站
/// 不可用/不可列表，CI 常见），调用方应如实跳过而非制造假失败。
/// 结果按进程缓存（OnceLock），并顺带预热 Shell 命名空间。
pub fn recycle_bin_operational() -> bool {
    static CACHE: OnceLock<bool> = OnceLock::new();
    *CACHE.get_or_init(probe_recycle_bin)
}

fn probe_recycle_bin() -> bool {
    let dir = std::env::temp_dir().join(format!("weave-recycle-probe-{}", std::process::id()));
    let probe = dir.join("probe.txt");
    let _ = std::fs::remove_dir_all(&dir);
    if !std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&probe, b"weave probe"))
        .is_ok()
    {
        let _ = std::fs::remove_dir_all(&dir);
        return false;
    }
    let operational = match recycle_one(&probe) {
        RecycleOutcome::Success(receipt) => {
            restore_recycled(std::slice::from_ref(&receipt))
                .iter()
                .all(|(_, status, _)| matches!(status, crate::undo::UndoItemStatus::Restored))
                && probe.exists()
        }
        _ => false,
    };
    // 探针现场清理：探针文件是本进程创建的临时物，非用户数据
    let _ = std::fs::remove_dir_all(&dir);
    operational
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
