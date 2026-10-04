//! Undo 服务（M2 §48–§51）。
//!
//! Undo ≠ 反向再执行：先加载事务 → 校验当前状态 → 检查目标 → 确认原位
//! 安全 → 执行反向操作（M2 §48）。
//!
//! - **LIFO + 占位暂存**：按事务内实际应用顺序的逆序执行。当反向目标
//!   （原位置）被同事务中**尚未撤销**条目的当前位置占据（swap/cycle 场景），
//!   将占据者暂存到临时路径，并把它剩余的还原动作重定向到该临时路径——
//!   后续 LIFO 轮次会把它放进正确的原位。临时文件在全部条目处理后必被
//!   消费（`leaked_temps` 守护，有测试断言）。
//! - **安全优先**：原位被事务外的文件占据（大小/时间不符）⇒ `UndoConflict`
//!   跳过该项，绝不覆盖（M2 §49）。
//! - **NotExecuted / NoOp 条目**：绝不制造假恢复记录（M2 §50）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use weave_core::prelude::{CancellationToken, OperationId, OperationKind, Progress, WeaveError};
use weave_history::{OperationTransaction, TransactionItem, TransactionItemStatus};

use crate::fs::Filesystem;

/// 单条 Undo 结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoItemStatus {
    Restored,
    UndoConflict,
    Missing,
    NotUndoable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoItemResult {
    pub item_id: String,
    pub status: UndoItemStatus,
    /// 结构化原因（冲突时给用户看）。
    pub reason: Option<WeaveError>,
}

#[derive(Debug, Clone)]
pub struct UndoReport {
    pub operation_id: OperationId,
    pub results: Vec<UndoItemResult>,
    pub restored: u64,
    pub conflicts: u64,
    pub skipped: u64,
    /// 撤销完成后残留的暂存文件数（正常必须为 0；>0 即内部一致性缺陷）。
    pub leaked_temps: u64,
    pub duration_ms: u64,
}

impl UndoReport {
    /// 全部已执行项都被安全还原？
    pub fn fully_restored(&self) -> bool {
        self.restored > 0
            && self.conflicts == 0
            && self.leaked_temps == 0
            && self
                .results
                .iter()
                .all(|r| !matches!(r.status, UndoItemStatus::UndoConflict))
    }
}

/// 执行一次 Undo（协作取消；LIFO + 占位暂存）。
pub fn undo_transaction(
    fs: &dyn Filesystem,
    transaction: &OperationTransaction,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> UndoReport {
    let started = std::time::Instant::now();
    let total = transaction.items.len() as u64;
    let mut results: Vec<UndoItemResult> = Vec::new();
    let (mut restored, mut conflicts, mut skipped) = (0u64, 0u64, 0u64);
    let mut leaked_temps: u64 = 0;

    // item_id → 该条目内容**当前**所在路径（初始 = target；被暂存时改写为 tmp）。
    let mut current: HashMap<String, PathBuf> = transaction
        .items
        .iter()
        .map(|i| (i.item_id.clone(), PathBuf::from(&i.target_path)))
        .collect();

    let total_items = transaction.items.len();
    for (rev_index, item) in transaction.items.iter().enumerate().rev() {
        if cancel.is_cancelled() {
            // 取消：剩余（更早应用的）条目不再处理
            for earlier in transaction.items.iter().take(rev_index) {
                results.push(UndoItemResult {
                    item_id: earlier.item_id.clone(),
                    status: UndoItemStatus::NotUndoable,
                    reason: Some(
                        WeaveError::cancelled(
                            "undo.cancelledBeforeItem",
                            "undo was cancelled before reaching this item",
                        )
                        .with_location("weave-files::undo"),
                    ),
                });
                skipped += 1;
            }
            break;
        }
        report_progress(Progress::running(
            transaction.operation_id.clone(),
            (total_items - rev_index) as u64,
            Some(total),
        ));

        let result = match item.status {
            TransactionItemStatus::NotExecuted | TransactionItemStatus::NoOp => {
                skipped += 1;
                UndoItemResult {
                    item_id: item.item_id.clone(),
                    status: UndoItemStatus::NotUndoable,
                    reason: None,
                }
            }
            TransactionItemStatus::Executed => {
                if transaction.kind == OperationKind::TextTransform
                    || transaction.kind == OperationKind::BatchExecute
                {
                    // M4 §94：文本覆盖写的原位**必然**被占（写回即覆盖）——
                    // 专用还原：校验当前 stat == 事务记录的写后状态（用户
                    // 未再改动）⇒ 备份 rename 覆盖原位；否则 UndoConflict。
                    undo_text_item(fs, item)
                } else {
                    undo_executed_item(fs, item, &mut current, &transaction.items)
                }
            }
        };
        match result.status {
            UndoItemStatus::Restored => restored += 1,
            UndoItemStatus::UndoConflict => conflicts += 1,
            _ => skipped += 1,
        }
        results.push(result);
    }

    // 暂存残留盘点：current 指向 .weave-undo-tmp 的条目都应已被后续轮次消费。
    for path in current.values() {
        if path
            .file_name()
            .map(|n| n.to_string_lossy().contains(".weave-undo-tmp"))
            .unwrap_or(false)
        {
            leaked_temps += 1;
        }
    }

    UndoReport {
        operation_id: transaction.operation_id.clone(),
        results,
        restored,
        conflicts,
        skipped,
        leaked_temps,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// 文本覆盖写的还原（M4 §94）：
/// 1) 备份（事务 target）必须存在；
/// 2) 原位当前 stat 必须 == 事务记录的**写后**状态——不符即用户已改动，
///    UndoConflict 拒绝覆盖（不猜测、不吞掉用户编辑）；
/// 3) rename 备份 → 原位（原子替换，备份随之消失）。
fn undo_text_item(fs: &dyn Filesystem, item: &TransactionItem) -> UndoItemResult {
    let target = Path::new(&item.target_path);
    let source = Path::new(&item.source_path);
    let conflict = |reason: String| UndoItemResult {
        item_id: item.item_id.clone(),
        status: UndoItemStatus::UndoConflict,
        reason: Some(
            WeaveError::conflict("undo.textConflict", reason).with_location("weave-files::undo"),
        ),
    };

    // 创建型写回（M5 Export 新文件）：无备份，撤销 = 删除已创建文件
    //（仍受 §94 保护：用户改过 ⇒ stat 不符 ⇒ 冲突）。
    if item.target_path.is_empty() {
        let matches_recorded = match fs.stat(source) {
            Ok(stat) => {
                item.original_size == Some(stat.size)
                    && item.original_modified.is_some()
                    && stat.modified == item.original_modified
            }
            Err(_) => false,
        };
        if !matches_recorded {
            return conflict(
                "file was modified after creation; refusing to delete user edits".to_string(),
            );
        }
        return match std::fs::remove_file(source) {
            Ok(()) => UndoItemResult {
                item_id: item.item_id.clone(),
                status: UndoItemStatus::Restored,
                reason: None,
            },
            Err(e) => UndoItemResult {
                item_id: item.item_id.clone(),
                status: UndoItemStatus::UndoConflict,
                reason: Some(
                    WeaveError::io("undo.textRestoreFailed", format!("delete failed: {e}"))
                        .with_location("weave-files::undo"),
                ),
            },
        };
    }

    let Ok(backup_stat) = fs.stat(target) else {
        return UndoItemResult {
            item_id: item.item_id.clone(),
            status: UndoItemStatus::Missing,
            reason: Some(
                WeaveError::io("undo.backupMissing", "text backup is missing")
                    .with_location("weave-files::undo"),
            ),
        };
    };
    if backup_stat.kind == weave_core::prelude::FileKind::Other {
        return conflict("text backup is not a restorable file".to_string());
    }

    // §94：写回之后用户又改过 ⇒ 拒绝覆盖
    match fs.stat(source) {
        Ok(stat) => {
            let matches_recorded = item.original_size == Some(stat.size)
                && item.original_modified.is_some()
                && stat.modified == item.original_modified;
            if !matches_recorded {
                return conflict(
                    "file was modified after the text operation; refusing to overwrite user edits"
                        .to_string(),
                );
            }
        }
        Err(_) => {
            // 原位消失（被删除）⇒ 仍可还原（把备份放回原位）
        }
    }

    if let Err(e) = fs.rename(target, source) {
        return UndoItemResult {
            item_id: item.item_id.clone(),
            status: UndoItemStatus::UndoConflict,
            reason: Some(
                WeaveError::io("undo.textRestoreFailed", format!("restore failed: {e}"))
                    .with_location("weave-files::undo"),
            ),
        };
    }
    UndoItemResult {
        item_id: item.item_id.clone(),
        status: UndoItemStatus::Restored,
        reason: None,
    }
}

fn undo_executed_item(
    fs: &dyn Filesystem,
    item: &TransactionItem,
    current: &mut HashMap<String, PathBuf>,
    all_items: &[TransactionItem],
) -> UndoItemResult {
    let target = Path::new(&item.target_path);
    let source = Path::new(&item.source_path);

    // 1) 该条目内容当前所在路径（可能被暂存改写过）
    let current_path = current
        .get(&item.item_id)
        .cloned()
        .unwrap_or_else(|| target.to_path_buf());
    let Ok(target_stat) = fs.stat(&current_path) else {
        return missing(item, "undo target is missing");
    };
    if target_stat.kind == weave_core::prelude::FileKind::Other {
        return missing(item, "undo target is not a restorable file");
    }

    // 2) 目标与事务记录一致（防外部替换，M2 §49）
    if let (Some(expected_size), Some(expected_mtime)) =
        (item.original_size, item.original_modified)
        && (target_stat.size != expected_size || target_stat.modified != Some(expected_mtime))
    {
        return conflict(item, "target has changed since this operation");
    }

    // 3) 原位被占——区分「同事务链式占据」（可解）与「外部文件」（冲突）
    if fs.exists(source) {
        let occupant_is_pending = all_items.iter().any(|other| {
            other.item_id != item.item_id
                && current
                    .get(&other.item_id)
                    .is_some_and(|p| p.as_os_str() == source.as_os_str())
        });
        if !occupant_is_pending {
            return conflict(item, "original location is occupied");
        }
        // 占据者是同事务中待撤销条目的当前位置：暂存到 temp 并重定向其
        // 还原起点；本轮把自己的内容放进原位。
        let tmp = temp_for(source);
        if let Err(e) = fs.rename(source, &tmp) {
            return UndoItemResult {
                item_id: item.item_id.clone(),
                status: UndoItemStatus::UndoConflict,
                reason: Some(
                    WeaveError::io("undo.stageFailed", format!("cannot stage: {e}"))
                        .with_location("weave-files::undo"),
                ),
            };
        }
        if let Some(other) = all_items
            .iter()
            .find(|o| o.item_id != item.item_id && o.target_path == item.source_path)
            && let Some(slot) = current.get_mut(&other.item_id)
        {
            *slot = tmp;
        }
        // 原位已清空，继续正常还原
    }

    // 4) 反向 rename（内容回到 source）
    if let Err(e) = fs.rename(&current_path, source) {
        return UndoItemResult {
            item_id: item.item_id.clone(),
            status: UndoItemStatus::UndoConflict,
            reason: Some(
                WeaveError::io("undo.renameFailed", format!("undo rename failed: {e}"))
                    .with_location("weave-files::undo"),
            ),
        };
    }
    if !fs.exists(source) {
        return UndoItemResult {
            item_id: item.item_id.clone(),
            status: UndoItemStatus::UndoConflict,
            reason: Some(
                WeaveError::io(
                    "undo.verifyFailed",
                    "undo rename reported success but source missing",
                )
                .with_location("weave-files::undo"),
            ),
        };
    }

    UndoItemResult {
        item_id: item.item_id.clone(),
        status: UndoItemStatus::Restored,
        reason: None,
    }
}

fn conflict(item: &TransactionItem, reason: &str) -> UndoItemResult {
    UndoItemResult {
        item_id: item.item_id.clone(),
        status: UndoItemStatus::UndoConflict,
        reason: Some(
            WeaveError::conflict("undo.conflict", format!("{reason}: {}", item.target_path))
                .with_location("weave-files::undo")
                .with_suggestion("resolve the conflict manually, then retry undo"),
        ),
    }
}

fn missing(item: &TransactionItem, reason: &str) -> UndoItemResult {
    UndoItemResult {
        item_id: item.item_id.clone(),
        status: UndoItemStatus::Missing,
        reason: Some(
            WeaveError::io(
                "undo.targetMissing",
                format!("{reason}: {}", item.target_path),
            )
            .with_location("weave-files::undo"),
        ),
    }
}

/// 同目录暂存名：`.<stem>.weave-undo-tmp{ext}`；已存在时追加序号。
fn temp_for(source: &Path) -> PathBuf {
    let parent = source.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let ext = source
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let base = parent.join(format!(".{stem}.weave-undo-tmp{ext}"));
    if !base.exists() {
        return base;
    }
    let mut n = 1u32;
    loop {
        let candidate = parent.join(format!(".{stem}.weave-undo-tmp-{n}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod text_undo_tests {
    use super::*;
    use crate::fs::StdFilesystem;
    use std::time::{Duration, SystemTime};
    use weave_history::{OperationStatus, Reversibility, TransactionItem, TransactionItemStatus};
    use weave_testkit::TempWorkspace;

    fn text_tx(
        source: &str,
        backup: &str,
        post_size: u64,
        post_mtime: SystemTime,
    ) -> OperationTransaction {
        OperationTransaction {
            operation_id: weave_core::id::OperationId::generate(),
            kind: weave_core::prelude::OperationKind::TextTransform,
            status: OperationStatus::Completed,
            timestamp: SystemTime::UNIX_EPOCH,
            reversible: Reversibility::Full,
            items: vec![TransactionItem {
                item_id: "item_0000".to_string(),
                source_path: source.to_string(),
                target_path: backup.to_string(),
                status: TransactionItemStatus::Executed,
                timestamp: Some(post_mtime),
                original_size: Some(post_size),
                original_modified: Some(post_mtime),
                original_created: None,
            }],
        }
    }

    #[test]
    fn undo_text_restores_backup_when_untouched() {
        let ws = TempWorkspace::new("undo-text-ok").expect("ws");
        let original = ws.path().join("doc.txt");
        std::fs::write(&original, "new content").expect("write");
        let backup = ws.path().join(".doc.txt.weave-text-bak-1");
        std::fs::write(&backup, "old content").expect("write");
        let post = SystemTime::now();
        {
            let f = std::fs::OpenOptions::new()
                .append(true)
                .open(&original)
                .expect("open");
            f.set_modified(post).expect("mtime");
        }

        let tx = text_tx(
            original.to_string_lossy().as_ref(),
            backup.to_string_lossy().as_ref(),
            std::fs::metadata(&original).expect("stat").len(),
            post,
        );
        let report = undo_transaction(
            &StdFilesystem,
            &tx,
            &weave_core::prelude::CancellationToken::new(),
            &mut |_| {},
        );
        assert_eq!(report.restored, 1, "{report:?}");
        assert_eq!(
            std::fs::read_to_string(&original).expect("read"),
            "old content"
        );
        assert!(!backup.exists(), "备份随 rename 消费");
    }

    #[test]
    fn undo_text_conflicts_when_user_edited_after_write() {
        let ws = TempWorkspace::new("undo-text-conflict").expect("ws");
        let original = ws.path().join("doc.txt");
        std::fs::write(&original, "user edited!").expect("write");
        let backup = ws.path().join(".doc.txt.weave-text-bak-2");
        std::fs::write(&backup, "old content").expect("write");
        let post = SystemTime::now() - Duration::from_secs(3600);

        let tx = text_tx(
            original.to_string_lossy().as_ref(),
            backup.to_string_lossy().as_ref(),
            99, // 与实际不符 ⇒ 用户改过
            post,
        );
        let report = undo_transaction(
            &StdFilesystem,
            &tx,
            &weave_core::prelude::CancellationToken::new(),
            &mut |_| {},
        );
        assert_eq!(report.conflicts, 1, "{report:?}");
        assert_eq!(
            std::fs::read_to_string(&original).expect("read"),
            "user edited!",
            "绝不覆盖用户编辑（§94）"
        );
        assert!(backup.exists(), "备份保留");
    }
}

#[cfg(test)]
mod creation_undo_tests {
    use super::*;
    use crate::fs::StdFilesystem;
    use std::time::SystemTime;
    use weave_history::{OperationStatus, Reversibility, TransactionItem, TransactionItemStatus};
    use weave_testkit::TempWorkspace;

    fn creation_tx(source: &str, post_size: u64, post_mtime: SystemTime) -> OperationTransaction {
        OperationTransaction {
            operation_id: weave_core::id::OperationId::generate(),
            kind: weave_core::prelude::OperationKind::TextTransform,
            status: OperationStatus::Completed,
            timestamp: SystemTime::UNIX_EPOCH,
            reversible: Reversibility::Full,
            items: vec![TransactionItem {
                item_id: "item_0000".to_string(),
                source_path: source.to_string(),
                target_path: String::new(), // 创建型：无备份
                status: TransactionItemStatus::Executed,
                timestamp: Some(post_mtime),
                original_size: Some(post_size),
                original_modified: Some(post_mtime),
                original_created: None,
            }],
        }
    }

    #[test]
    fn undo_created_file_removes_it_when_untouched() {
        let ws = TempWorkspace::new("undo-created").expect("ws");
        let created = ws.path().join("exported.jsonl");
        std::fs::write(&created, "{\"a\":1}").expect("write");
        let post = SystemTime::now();
        {
            let f = std::fs::OpenOptions::new()
                .append(true)
                .open(&created)
                .expect("open");
            f.set_modified(post).expect("mtime");
        }
        let tx = creation_tx(
            created.to_string_lossy().as_ref(),
            std::fs::metadata(&created).expect("stat").len(),
            post,
        );
        let report = undo_transaction(
            &StdFilesystem,
            &tx,
            &weave_core::prelude::CancellationToken::new(),
            &mut |_| {},
        );
        assert_eq!(report.restored, 1, "{report:?}");
        assert!(!created.exists(), "撤销 = 删除已创建文件");
    }

    #[test]
    fn undo_created_file_conflicts_when_user_edited() {
        let ws = TempWorkspace::new("undo-created-conflict").expect("ws");
        let created = ws.path().join("exported.jsonl");
        std::fs::write(&created, "user kept this").expect("write");
        let tx = creation_tx(
            created.to_string_lossy().as_ref(),
            999, // 与实际不符
            SystemTime::UNIX_EPOCH,
        );
        let report = undo_transaction(
            &StdFilesystem,
            &tx,
            &weave_core::prelude::CancellationToken::new(),
            &mut |_| {},
        );
        assert_eq!(report.conflicts, 1);
        assert_eq!(
            std::fs::read_to_string(&created).expect("read"),
            "user kept this",
            "绝不删除用户编辑后的文件"
        );
    }
}
