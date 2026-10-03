//! Plan 执行器（M2 §38–§45 / §70–§73 / §82–§83）。
//!
//! 语义（全部记录于 DECISIONS D28/D30/D31）：
//! - Preview 与 Execute 共用同一 Plan；执行前**逐项 Revalidate**
//!   （source 存在 + size/modified 匹配；目标不存在），变化 ⇒ 该项
//!   `rename.sourceChanged` / `rename.targetAlreadyExists` 失败，绝不智能纠正。
//! - **两阶段**：CaseOnly 或 Cycle 条目先全部改名到 `.weave-tmp-*` 唯一临时名，
//!   全部落定后再落到最终目标；失败/中断项由事务的 Executed 记录保证
//!   Undo 可恢复，正常完成时 temp 全部消失（有测试守护）。
//! - **取消是协作式**：仅在条目间安全点检查；取消后剩余 Ready 条目
//!   outcome=Cancelled，绝不遗留半程状态（两阶段 A/B 各自连续完成）。
//! - **默认 No Overwrite**：执行时目标重现 ⇒ 该项 Failed，不覆盖（§39/§98）。
//! - **Move 跨卷拒绝**（§71）：source 与 target 卷前缀不同 ⇒ 结构化失败。
//! - 事务按**实际应用顺序**记录（含两阶段的 temp 步），Undo 依赖 LIFO。

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use weave_core::prelude::{
    CancellationToken, CollisionKind, Plan, PlanItem, PlanItemStatus, Progress, WeaveError,
};
use weave_history::{OperationStatus, OperationTransaction, Reversibility, TransactionItemStatus};

use crate::fs::Filesystem;

/// 执行报告：更新后的 Plan（带 per-item outcome）+ 实际事务 + 计数。
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    pub plan: Plan,
    pub transaction: OperationTransaction,
    pub duration_ms: u64,
    /// 实际成功执行（可 Undo）的条目数。
    pub executed: u64,
    pub failed: u64,
    /// Conflict/Invalid/取消未执行等未变更条目数。
    pub skipped: u64,
}

/// 执行一个已确认的 Plan。
///
/// 顶层不返回条目级错误（M2 §40 部分成功语义）：一切条目失败都在
/// 返回的 Plan/事务里结构化呈现。
pub fn execute_plan(
    fs: &dyn Filesystem,
    plan: &Plan,
    cancel: &CancellationToken,
    report_progress: &mut dyn FnMut(Progress),
) -> ExecutionReport {
    let started = Instant::now();
    let started_wall = SystemTime::now();
    let mut out_plan = plan.clone();
    let op_short: String = plan
        .operation_id
        .as_str()
        .trim_start_matches("op_")
        .chars()
        .take(8)
        .collect();

    let mut history_items: Vec<weave_history::TransactionItem> = Vec::new();
    let mut cancelled = false;
    let (mut executed, mut failed, mut skipped) = (0u64, 0u64, 0u64);

    // ── 划分 ──
    let total = out_plan.items.len() as u64;
    let mut direct: Vec<usize> = Vec::new();
    let mut two_phase: Vec<usize> = Vec::new();
    for (idx, item) in out_plan.items.iter().enumerate() {
        match (item.status, item.collision) {
            (PlanItemStatus::Ready, CollisionKind::CaseOnly)
            | (PlanItemStatus::Ready, CollisionKind::Cycle) => two_phase.push(idx),
            (PlanItemStatus::Ready, _) => direct.push(idx),
            (PlanItemStatus::Conflict, _) | (PlanItemStatus::Invalid, _) => skipped += 1,
            (PlanItemStatus::NoOp, _) => {}
        }
    }

    let mut processed: u64 = 0;
    let record_failure = |idx: usize, out: &mut Plan, error: WeaveError, counts: &mut u64| {
        out.items[idx].errors.push(error);
        *counts += 1;
    };

    // ── 直通条目 ──
    for &idx in &direct {
        if cancel.is_cancelled() {
            cancelled = true;
        }
        if cancelled {
            skipped += 1;
            history_items.push(hist_item(
                &out_plan.items[idx],
                TransactionItemStatus::NotExecuted,
                None,
            ));
            continue;
        }
        processed += 1;
        report_progress(Progress::running(
            out_plan.operation_id.clone(),
            processed,
            Some(total),
        ));
        let item = out_plan.items[idx].clone();
        match revalidate_and_rename(fs, &item, out_plan.kind) {
            Ok(final_target) => {
                executed += 1;
                out_plan.items[idx].target_path = final_target.to_string_lossy().into_owned();
                history_items.push(hist_item(
                    &item,
                    TransactionItemStatus::Executed,
                    Some(final_target),
                ));
            }
            Err(e) => {
                record_failure(idx, &mut out_plan, e, &mut failed);
                history_items.push(hist_item(&item, TransactionItemStatus::NotExecuted, None));
            }
        }
    }

    // ── 两阶段（Phase A 让位 → Phase B 落位）──
    let mut temp_map: Vec<(usize, PathBuf)> = Vec::new();
    for &idx in &two_phase {
        if cancel.is_cancelled() {
            cancelled = true;
        }
        if cancelled {
            skipped += 1;
            history_items.push(hist_item(
                &out_plan.items[idx],
                TransactionItemStatus::NotExecuted,
                None,
            ));
            continue;
        }
        processed += 1;
        report_progress(Progress::running(
            out_plan.operation_id.clone(),
            processed,
            Some(total),
        ));
        let item = out_plan.items[idx].clone();
        let temp = temp_name(Path::new(&item.source_path), &op_short, idx, fs);
        let result = revalidate_source(fs, &item)
            .and_then(|_| {
                fs.rename(Path::new(&item.source_path), &temp)
                    .map_err(|e| rename_error(e, &item.source_path))
            })
            .map(|_| temp.clone());
        match result {
            Ok(temp) => {
                executed += 1;
                history_items.push(hist_item(
                    &item,
                    TransactionItemStatus::Executed,
                    Some(temp.clone()),
                ));
                temp_map.push((idx, temp));
            }
            Err(e) => {
                record_failure(idx, &mut out_plan, e, &mut failed);
                history_items.push(hist_item(&item, TransactionItemStatus::NotExecuted, None));
            }
        }
    }

    if !cancelled {
        for (idx, temp) in &temp_map {
            if cancel.is_cancelled() {
                cancelled = true;
                skipped += 1;
            }
            if cancelled {
                continue;
            }
            processed += 1;
            report_progress(Progress::running(
                out_plan.operation_id.clone(),
                processed,
                Some(total),
            ));
            let item = out_plan.items[*idx].clone();
            let final_target = PathBuf::from(&item.target_path);
            if fs.exists(&final_target) {
                failed += 1;
                out_plan.items[*idx].errors.push(WeaveError::conflict(
                    "rename.targetAlreadyExists",
                    format!("target appeared before final phase: {}", item.target_path),
                ));
                // 条目留在 temp：事务里该条目已 Executed(source→temp)，Undo 可恢复。
                history_items.push(hist_item(&item, TransactionItemStatus::NotExecuted, None));
                continue;
            }
            match fs
                .rename(temp, &final_target)
                .map_err(|e| rename_error(e, temp.to_string_lossy().as_ref()))
            {
                Ok(()) => {
                    // 覆盖该条目 Phase A 的 temp 记录：最终事实是 source→target。
                    if let Some(last) = history_items.iter_mut().rev().find(|h| {
                        h.item_id == item.item_id && h.status == TransactionItemStatus::Executed
                    }) {
                        last.target_path = final_target.to_string_lossy().into_owned();
                    }
                    out_plan.items[*idx].target_path = final_target.to_string_lossy().into_owned();
                }
                Err(e) => {
                    failed += 1;
                    out_plan.items[*idx].errors.push(e);
                    history_items.push(hist_item(&item, TransactionItemStatus::NotExecuted, None));
                }
            }
        }
    }

    // ── 终态装配 ──
    let reversible = if executed == 0 {
        Reversibility::None
    } else if failed == 0 {
        Reversibility::Full
    } else {
        Reversibility::Partial
    };

    let transaction = OperationTransaction {
        operation_id: out_plan.operation_id.clone(),
        kind: out_plan.kind,
        status: OperationStatus::Completed,
        timestamp: started_wall,
        reversible,
        items: history_items,
    };

    ExecutionReport {
        plan: out_plan,
        transaction,
        duration_ms: started.elapsed().as_millis() as u64,
        executed,
        failed,
        skipped,
    }
}

fn hist_item(
    item: &PlanItem,
    status: TransactionItemStatus,
    actual_target: Option<PathBuf>,
) -> weave_history::TransactionItem {
    weave_history::TransactionItem {
        item_id: item.item_id.clone(),
        source_path: item.source_path.clone(),
        target_path: actual_target
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| item.target_path.clone()),
        status,
        timestamp: Some(SystemTime::now()),
        original_size: item.source_size,
        original_modified: item.source_modified,
        original_created: None,
    }
}

/// Revalidate（M2 §38）：source 存在且 size/modified 与 Plan 快照一致。
fn revalidate_source(fs: &dyn Filesystem, item: &PlanItem) -> Result<(), WeaveError> {
    let stat = fs.stat(Path::new(&item.source_path)).map_err(|e| {
        let code = match e.kind() {
            std::io::ErrorKind::NotFound => "rename.sourceNotFound",
            std::io::ErrorKind::PermissionDenied => "rename.permissionDenied",
            _ => "rename.statFailed",
        };
        WeaveError::io(code, format!("source revalidation failed: {e}"))
            .with_location("weave-files::execute")
    })?;
    if let Some(expected) = item.source_size
        && stat.size != expected
    {
        return Err(WeaveError::conflict(
            "rename.sourceChanged",
            format!(
                "source changed since plan (size {} != {})",
                stat.size, expected
            ),
        )
        .with_location("weave-files::execute"));
    }
    if let (Some(expected), Some(actual)) = (item.source_modified, stat.modified)
        && expected != actual
    {
        return Err(WeaveError::conflict(
            "rename.sourceChanged",
            format!(
                "source was modified after the plan was built (mtime {:?} != {:?})",
                actual, expected
            ),
        )
        .with_location("weave-files::execute"));
    }
    Ok(())
}

fn revalidate_and_rename(
    fs: &dyn Filesystem,
    item: &PlanItem,
    kind: weave_core::prelude::OperationKind,
) -> Result<PathBuf, WeaveError> {
    revalidate_source(fs, item)?;
    let target = PathBuf::from(&item.target_path);
    if fs.exists(&target) {
        return Err(WeaveError::conflict(
            "rename.targetAlreadyExists",
            format!("target appeared after plan: {}", item.target_path),
        )
        .with_location("weave-files::execute"));
    }
    if kind == weave_core::prelude::OperationKind::Move {
        if !volumes_match(Path::new(&item.source_path), &target) {
            return Err(WeaveError::unsupported(
                "move.crossVolumeUnsupported",
                "cross-filesystem move is not supported in M2 (DECISIONS D30)",
            )
            .with_location("weave-files::execute"));
        }
        // Organizer 目标目录按已确认 Plan 创建（M2 §77 destination creation；
        // 仅 Move 计划内，绝不越出 Plan 条目）。
        if let Some(parent) = target.parent()
            && !fs.exists(parent)
        {
            fs.create_dir_all(parent).map_err(|e| {
                WeaveError::io(
                    "move.createDestinationFailed",
                    format!("cannot create destination dir: {e}"),
                )
                .with_location("weave-files::execute")
            })?;
        }
    }
    fs.rename(Path::new(&item.source_path), &target)
        .map_err(|e| rename_error(e, &item.source_path))?;
    if !fs.exists(&target) {
        return Err(WeaveError::io(
            "rename.verifyFailed",
            format!(
                "rename reported success but target missing: {}",
                item.target_path
            ),
        )
        .with_location("weave-files::execute"));
    }
    Ok(target)
}

fn rename_error(e: std::io::Error, path: &str) -> WeaveError {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "rename.sourceNotFound",
        std::io::ErrorKind::PermissionDenied => "rename.permissionDenied",
        _ => "rename.filesystemError",
    };
    WeaveError::io(code, format!("rename failed on '{path}': {e}"))
        .with_location("weave-files::execute")
        .with_recoverability(weave_core::prelude::Recoverability::UserActionRequired)
}

/// 两阶段临时名：同目录 `.weave-tmp-{op 短 id}-{item 序}.{ext}`；
/// 已存在时追加序号直至唯一（不覆盖任何文件）。
fn temp_name(source: &Path, op_short: &str, idx: usize, fs: &dyn Filesystem) -> PathBuf {
    let parent = source.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let ext = source
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let base = format!(".weave-tmp-{op_short}-{idx:04}");
    let mut candidate = parent.join(format!("{base}{ext}"));
    let mut n = 1u32;
    while fs.exists(&candidate) {
        candidate = parent.join(format!("{base}-{n}{ext}"));
        n += 1;
    }
    candidate
}

/// 卷前缀比较：同盘符（`X:`）或同 UNC 服务器+共享。
pub fn volumes_match(a: &Path, b: &Path) -> bool {
    let prefix = |p: &Path| -> String {
        let s = p.to_string_lossy().to_lowercase();
        if let Some(rest) = s.strip_prefix(r"\\?\unc\") {
            return format!(r"\\{rest}").chars().take(64).collect();
        }
        if s.starts_with(r"\\?\") {
            return s.chars().take(6).collect();
        }
        if s.starts_with(r"\\") {
            return s.splitn(4, '\\').take(4).collect::<Vec<_>>().join("\\");
        }
        s.chars().take(2).collect()
    };
    !prefix(a).is_empty() && prefix(a) == prefix(b)
}
