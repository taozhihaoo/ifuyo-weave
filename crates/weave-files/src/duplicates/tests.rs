//! 重复扫描测试（M3 §88–§91：大小分组/partial/full/分组/排序/确定性）。

use super::*;
use crate::fs::StdFilesystem;
use weave_core::prelude::CancellationToken;
use weave_testkit::TempWorkspace;

fn scan(ws: &TempWorkspace) -> DuplicateScanReport {
    scan_duplicates(
        &StdFilesystem,
        &[ws.path().to_string_lossy().into_owned()],
        0,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("scan")
}

#[test]
fn size_grouping_only_size_peers_are_candidates() {
    let ws = TempWorkspace::new("dup-size").expect("ws");
    ws.file("a.txt", "same content").expect("write");
    ws.file("b.txt", "same content").expect("write");
    ws.file("c.txt", "different length").expect("write");
    let report = scan(&ws);
    // a/b 同 size 同内容 → 1 组 2 文件；c 尺寸不同不进候选
    assert_eq!(report.files_scanned, 3);
    assert_eq!(report.candidate_files, 2);
    assert_eq!(report.duplicate_groups, 1);
    assert_eq!(report.duplicate_files, 2);
    assert_eq!(report.potential_reclaimable_size, 12); // len("same content")
}

#[test]
fn same_size_different_content_is_not_duplicate() {
    let ws = TempWorkspace::new("dup-samesize").expect("ws");
    ws.file("a.bin", "AAAA").expect("write");
    ws.file("b.bin", "BBBB").expect("write");
    let report = scan(&ws);
    assert_eq!(report.candidate_files, 2, "同 size 进入候选");
    assert_eq!(report.duplicate_groups, 0, "内容不同 full hash 分开");
    assert_eq!(report.potential_reclaimable_size, 0);
}

#[test]
fn zero_byte_files_group_without_disk_reads() {
    let ws = TempWorkspace::new("dup-zero").expect("ws");
    ws.file("a.txt", "").expect("write");
    ws.file("b.txt", "").expect("write");
    ws.file("c.tmp", "").expect("write");
    let report = scan(&ws);
    assert_eq!(report.duplicate_groups, 1);
    assert_eq!(report.duplicate_files, 3);
}

#[test]
fn partial_hash_distinguishes_head_and_tail_differences() {
    // first-chunk-differs 与 last-chunk-differs 两类用例（M3 §4 验收硬化）
    let n = PARTIAL_HASH_BYTES as usize;
    let head_a = vec![1u8; n];
    let head_b = vec![2u8; n];
    let tail_diff_a = {
        let mut v = vec![7u8; n * 2];
        v[n..].copy_from_slice(&vec![3u8; n]);
        v
    };
    let tail_diff_b = {
        let mut v = vec![7u8; n * 2];
        v[n..].copy_from_slice(&vec![4u8; n]);
        v
    };
    let ws = TempWorkspace::new("dup-partial").expect("ws");
    ws.bytes("same_head_1.bin", &head_a).expect("w1");
    ws.bytes("same_head_2.bin", &head_a).expect("w2");
    ws.bytes("same_head_3.bin", &head_b).expect("w3");
    ws.bytes("tail_1.bin", &tail_diff_a).expect("t1");
    ws.bytes("tail_2.bin", &tail_diff_b).expect("t2");

    let p1 = partial_hash(
        &StdFilesystem,
        &ws.path().join("same_head_1.bin"),
        head_a.len() as u64,
    )
    .expect("p1");
    let p2 = partial_hash(
        &StdFilesystem,
        &ws.path().join("same_head_2.bin"),
        head_a.len() as u64,
    )
    .expect("p2");
    let p3 = partial_hash(
        &StdFilesystem,
        &ws.path().join("same_head_3.bin"),
        head_a.len() as u64,
    )
    .expect("p3");
    assert_eq!(p1, p2, "same head → same partial");
    assert_ne!(p1, p3, "different head → partial hash filters correctly");

    let t1 = partial_hash(
        &StdFilesystem,
        &ws.path().join("tail_1.bin"),
        tail_diff_a.len() as u64,
    )
    .expect("t1");
    let t2 = partial_hash(
        &StdFilesystem,
        &ws.path().join("tail_2.bin"),
        tail_diff_b.len() as u64,
    )
    .expect("t2");
    assert_ne!(t1, t2, "last-chunk-differs: partial hash filters correctly");
}

#[test]
fn full_hash_separates_what_partial_cannot() {
    // 同 size 同 partial（中部差异）：partial 不能定论，full hash 分开
    let n = PARTIAL_HASH_BYTES as usize;
    let mut a = vec![5u8; n * 3];
    let mut b = vec![5u8; n * 3];
    a[n] = 1;
    b[n] = 2;
    let ws = TempWorkspace::new("dup-mid").expect("ws");
    ws.bytes("mid_a.bin", &a).expect("wa");
    ws.bytes("mid_b.bin", &b).expect("wb");

    let pa =
        partial_hash(&StdFilesystem, &ws.path().join("mid_a.bin"), a.len() as u64).expect("pa");
    let pb =
        partial_hash(&StdFilesystem, &ws.path().join("mid_b.bin"), b.len() as u64).expect("pb");
    assert_eq!(pa, pb, "partial 无法区分中部差异（设计内行为）");

    let report = scan(&ws);
    assert_eq!(report.duplicate_groups, 0, "full hash separates");
    assert_eq!(report.full_hashed, 2);
}

#[test]
fn extension_does_not_affect_duplicate_detection() {
    let ws = TempWorkspace::new("dup-ext").expect("ws");
    ws.file("a.bin", "identical").expect("write");
    ws.file("b.jpg", "identical").expect("write");
    let report = scan(&ws);
    assert_eq!(
        report.duplicate_groups, 1,
        "内容判定，扩展名不参与（M3 §8）"
    );
}

#[test]
fn group_ordering_by_wasted_size_desc_and_stable_membership() {
    let ws = TempWorkspace::new("dup-order").expect("ws");
    // 小组（2×3B wasted 3）与大组（2×20B wasted 20）
    ws.file("small_1.txt", "aaa").expect("write");
    ws.file("small_2.txt", "aaa").expect("write");
    ws.file("big_1.bin", "01234567890123456789").expect("write");
    ws.file("big_2.bin", "01234567890123456789").expect("write");
    let r1 = scan(&ws);
    let r2 = scan(&ws);
    // 确定性：两次扫描组序一致（wasted DESC）
    let ids1: Vec<String> = r1.groups.iter().map(|g| g.group_id.clone()).collect();
    let ids2: Vec<String> = r2.groups.iter().map(|g| g.group_id.clone()).collect();
    assert_eq!(ids1, ids2);
    assert_eq!(ids1.len(), 2);
    // 第一组 wasted 最大
    let wastes: Vec<u64> = r1.groups.iter().map(|g| g.wasted_size).collect();
    assert!(wastes.windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn group_id_is_content_derived_not_scan_order() {
    let ws1 = TempWorkspace::new("dup-gid-1").expect("ws");
    let ws2 = TempWorkspace::new("dup-gid-2").expect("ws");
    ws1.file("name_one.txt", "shared content").expect("write");
    ws2.file("completely_different_name.txt", "shared content")
        .expect("write");
    let r1 = scan(&ws1);
    let r2 = scan(&ws2);
    assert_eq!(
        r1.groups
            .iter()
            .map(|g| g.group_id.clone())
            .collect::<Vec<_>>(),
        r2.groups
            .iter()
            .map(|g| g.group_id.clone())
            .collect::<Vec<_>>(),
        "GroupId 由内容派生（M3 §43）"
    );
}

#[test]
fn symlink_policy_reused_from_m1_not_followed() {
    let ws = TempWorkspace::new("dup-symlink").expect("ws");
    ws.file("real.txt", "real").expect("write");
    #[cfg(windows)]
    {
        let link = ws.path().join("link.txt");
        if std::os::windows::fs::symlink_file(ws.path().join("real.txt"), &link).is_err() {
            return; // 无符号链接特权环境：跳过（如实标注）
        }
    }
    let report = scan(&ws);
    // symlink 是 other_entries，不进候选也不计 files_scanned（lstat 策略）
    assert_eq!(report.duplicate_groups, 0);
}

// ── C2：回收计划构建与执行器（M3 §18/§26/§52–§61）──────────────────

use super::pipeline::RecycleSelection;
use weave_core::prelude::{
    CollisionKind, OperationId, OperationKind, Plan, PlanItem, PlanItemStatus,
};

fn recycle_plan(
    ws: &TempWorkspace,
    selections: &[RecycleSelection],
) -> Result<Plan, weave_core::prelude::WeaveError> {
    build_recycle_plan(&scan(ws), selections)
}

#[test]
fn recycle_plan_builds_snapshot_items_from_selection() {
    let ws = TempWorkspace::new("dup-plan-ok").expect("ws");
    ws.file("keep.txt", "same content").expect("write");
    ws.file("drop.txt", "same content").expect("write");
    let report = scan(&ws);
    let group_id = report.groups[0].group_id.clone();
    let drop_path = report.groups[0]
        .files
        .iter()
        .find(|f| f.path.ends_with("drop.txt"))
        .expect("entry")
        .path
        .clone();

    let plan = recycle_plan(
        &ws,
        &[RecycleSelection {
            group_id: group_id.clone(),
            recycle_paths: vec![drop_path.clone()],
        }],
    )
    .expect("plan");
    assert_eq!(plan.kind, OperationKind::DuplicateRecycle);
    assert_eq!(plan.items.len(), 1);
    let item = &plan.items[0];
    assert_eq!(item.source_path, drop_path);
    assert_eq!(item.status, PlanItemStatus::Ready);
    assert_eq!(item.target_path, "", "target 由回收站适配器决定（M3 §58）");
    assert_eq!(item.source_size, Some(12));
    assert!(item.source_modified.is_some());
    assert_eq!(item.item_id, "item_0000");
}

#[test]
fn recycle_plan_rejects_invalid_selections() {
    let ws = TempWorkspace::new("dup-plan-bad").expect("ws");
    ws.file("a.txt", "dup").expect("write");
    ws.file("b.txt", "dup").expect("write");
    ws.file("c.txt", "dup").expect("write");
    let report = scan(&ws);
    let group_id = report.groups[0].group_id.clone();
    let paths: Vec<String> = report.groups[0]
        .files
        .iter()
        .map(|f| f.path.clone())
        .collect();
    let sel = |ps: Vec<String>| RecycleSelection {
        group_id: group_id.clone(),
        recycle_paths: ps,
    };

    assert!(recycle_plan(&ws, &[]).is_err(), "空选择拒绝");
    assert!(recycle_plan(&ws, &[sel(vec![])]).is_err(), "组内空选择拒绝");
    let err =
        recycle_plan(&ws, &[sel(paths.clone())]).expect_err("全组回收必须拒绝（保留至少一份）");
    assert_eq!(err.code, "duplicates.keepAtLeastOne");
    let err = recycle_plan(
        &ws,
        &[sel(vec![
            "Q:
ope.txt"
                .to_string(),
        ])],
    )
    .expect_err("不属于该组的路径必须拒绝");
    assert_eq!(err.code, "duplicates.fileNotInGroup");
    let err = recycle_plan(
        &ws,
        &[RecycleSelection {
            group_id: "grp_deadbeefdeadbeef".to_string(),
            recycle_paths: vec![paths[0].clone()],
        }],
    )
    .expect_err("不存在的组必须拒绝");
    assert_eq!(err.code, "duplicates.groupNotFound");
    let err = recycle_plan(&ws, &[sel(vec![paths[0].clone(), paths[0].clone()])])
        .expect_err("同一路径重复选择必须拒绝");
    assert_eq!(err.code, "duplicates.duplicateSelection");
}

fn hand_plan(ws: &TempWorkspace, entries: &[(&str, Option<u64>)]) -> Plan {
    Plan {
        operation_id: OperationId::generate(),
        kind: OperationKind::DuplicateRecycle,
        created_at: std::time::SystemTime::UNIX_EPOCH,
        items: entries
            .iter()
            .enumerate()
            .map(|(i, (p, size))| PlanItem {
                item_id: format!("item_{i:04}"),
                source_path: ws.path().join(p).to_string_lossy().into_owned(),
                target_path: String::new(),
                status: PlanItemStatus::Ready,
                collision: CollisionKind::None,
                source_size: *size,
                source_modified: None,
                warnings: Vec::new(),
                errors: Vec::new(),
            })
            .collect(),
    }
}

#[test]
fn execute_recycle_cancel_before_start_touches_nothing() {
    let ws = TempWorkspace::new("dup-exec-cancel").expect("ws");
    ws.file("x.txt", "xxx").expect("write");
    ws.file("y.txt", "yyy").expect("write");
    let plan = hand_plan(&ws, &[("x.txt", Some(3)), ("y.txt", Some(3))]);
    let cancel = CancellationToken::new();
    cancel.cancel();

    let exec = execute_recycle_plan(&plan, &cancel, &mut |_| {});
    assert_eq!((exec.recycled, exec.failed, exec.skipped), (0, 0, 2));
    assert!(ws.path().join("x.txt").exists(), "取消后文件必须原样保留");
    assert!(ws.path().join("y.txt").exists());
    assert!(
        exec.transaction
            .items
            .iter()
            .all(|t| t.status == weave_history::TransactionItemStatus::NotExecuted)
    );
    assert_eq!(
        exec.transaction.reversible,
        weave_history::Reversibility::None
    );
}

#[test]
fn execute_recycle_revalidate_rejects_changed_or_missing_source() {
    let ws = TempWorkspace::new("dup-exec-reval").expect("ws");
    ws.file("grown.txt", "12345").expect("write");
    // 快照 size=999 ≠ 实际 5 ⇒ changedSinceScan，回收前拒绝（M3 §18）
    let plan = hand_plan(&ws, &[("grown.txt", Some(999)), ("vanished.txt", None)]);
    let exec = execute_recycle_plan(&plan, &CancellationToken::new(), &mut |_| {});
    assert_eq!((exec.recycled, exec.failed), (0, 2));
    assert!(
        ws.path().join("grown.txt").exists(),
        "校验失败的文件绝不进回收站"
    );
    assert!(
        exec.transaction
            .items
            .iter()
            .all(|t| t.status == weave_history::TransactionItemStatus::NotExecuted)
    );
    let codes: Vec<&str> = exec
        .plan
        .items
        .iter()
        .filter_map(|i| i.errors.first().map(|e| e.code.as_str()))
        .collect();
    assert!(codes.contains(&"duplicates.changedSinceScan"));
    assert!(codes.contains(&"rename.sourceNotFound"));
}

#[test]
fn undo_recycle_rejects_malformed_and_unexecuted_without_bin_access() {
    use weave_history::{OperationStatus, OperationTransaction, Reversibility, TransactionItem};

    let tx = OperationTransaction {
        operation_id: OperationId::generate(),
        kind: OperationKind::DuplicateRecycle,
        status: OperationStatus::Completed,
        timestamp: std::time::SystemTime::UNIX_EPOCH,
        reversible: Reversibility::Partial,
        items: vec![
            TransactionItem {
                item_id: "item_0000".to_string(),
                source_path: "C:.txt".to_string(),
                target_path: "not-a-recycle-token".to_string(),
                status: weave_history::TransactionItemStatus::Executed,
                timestamp: Some(std::time::SystemTime::UNIX_EPOCH),
                original_size: Some(1),
                original_modified: None,
                original_created: None,
            },
            TransactionItem {
                item_id: "item_0001".to_string(),
                source_path: "C:.txt".to_string(),
                target_path: String::new(),
                status: weave_history::TransactionItemStatus::NotExecuted,
                timestamp: None,
                original_size: None,
                original_modified: None,
                original_created: None,
            },
        ],
    };
    let report =
        crate::recycle::undo_recycle_transaction(&tx, &CancellationToken::new(), &mut |_| {});
    assert_eq!(report.results.len(), 2);
    assert!(
        report
            .results
            .iter()
            .all(|r| r.status == crate::undo::UndoItemStatus::NotUndoable)
    );
    let malformed = report
        .results
        .iter()
        .find(|r| r.item_id == "item_0000")
        .expect("item");
    assert_eq!(
        malformed.reason.as_ref().expect("reason").code,
        "duplicates.recycleTargetMalformed"
    );
}

#[test]
fn undo_recycle_cancelled_before_start_is_fully_not_undoable() {
    use weave_history::{
        OperationStatus, OperationTransaction, Reversibility, TransactionItem,
        TransactionItemStatus,
    };

    let tx = OperationTransaction {
        operation_id: OperationId::generate(),
        kind: OperationKind::DuplicateRecycle,
        status: OperationStatus::Completed,
        timestamp: std::time::SystemTime::UNIX_EPOCH,
        reversible: Reversibility::Full,
        items: vec![TransactionItem {
            item_id: "item_0000".to_string(),
            source_path: "C:.txt".to_string(),
            target_path: "recycle-bin:tok".to_string(),
            status: TransactionItemStatus::Executed,
            timestamp: Some(std::time::SystemTime::UNIX_EPOCH),
            original_size: None,
            original_modified: None,
            original_created: None,
        }],
    };
    let cancel = CancellationToken::new();
    cancel.cancel();
    let report = crate::recycle::undo_recycle_transaction(&tx, &cancel, &mut |_| {});
    assert_eq!(report.skipped, 1);
    assert_eq!(report.restored, 0);
    assert_eq!(
        report.results[0].reason.as_ref().expect("reason").code,
        "undo.cancelledBeforeItem"
    );
}

// ──（下）§91 Fault Injection：回收适配器注入矩阵 ─────────────────

use crate::recycle::{RecycleAdapter, RecycleOutcome, RecycleReceipt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

/// 全部返回 Unsupported（平台无回收站）。
struct UnavailableAdapter;
impl RecycleAdapter for UnavailableAdapter {
    fn recycle(&self, _path: &std::path::Path) -> RecycleOutcome {
        RecycleOutcome::Unsupported("no recycle bin on this platform".to_string())
    }
}

/// 前 ok 条返回伪成功回执，其余 Failed（逐条失败隔离）。
struct FlakyAdapter {
    ok_first: usize,
    calls: AtomicUsize,
}
impl RecycleAdapter for FlakyAdapter {
    fn recycle(&self, path: &std::path::Path) -> RecycleOutcome {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.ok_first {
            RecycleOutcome::Success(RecycleReceipt {
                original_path: path.to_path_buf(),
                token: Some(format!("fake-token-{n}")),
                deleted_at: SystemTime::UNIX_EPOCH,
            })
        } else {
            RecycleOutcome::Failed("file in use".to_string())
        }
    }
}

fn dup_ws(label: &str, n: usize) -> (TempWorkspace, Vec<String>) {
    let ws = TempWorkspace::new(label).expect("ws");
    let mut paths = Vec::new();
    for i in 0..n {
        let p = ws
            .file(&format!("f{i:02}.txt"), "same bytes")
            .expect("write");
        paths.push(p.to_string_lossy().into_owned());
    }
    (ws, paths)
}

fn plan_for(ws: &TempWorkspace, paths: &[String]) -> weave_core::prelude::Plan {
    let report = scan(ws);
    let group_id = report.groups[0].group_id.clone();
    build_recycle_plan(
        &report,
        &[RecycleSelection {
            group_id,
            recycle_paths: paths.to_vec(),
        }],
    )
    .expect("plan")
}

#[test]
fn fault_recycle_unavailable_is_structured_failure_not_deletion() {
    let (ws, all) = dup_ws("fault-unavail", 3);
    let paths = &all[1..]; // 每组保留一份（§56），回收其余两条
    let plan = plan_for(&ws, paths);
    let exec = execute_recycle_plan_with(
        &plan,
        &UnavailableAdapter,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!((exec.recycled, exec.failed, exec.skipped), (0, 2, 0));
    // 文件一个都没少（绝不 fallback 到永久删除，§126 P0）
    for p in &all {
        assert!(std::path::Path::new(p).exists(), "{p} must survive");
    }
    assert!(
        exec.plan
            .items
            .iter()
            .all(|i| i.errors.iter().any(|e| e.code == "recycle.unavailable"))
    );
    assert!(
        exec.transaction
            .items
            .iter()
            .all(|t| t.status == weave_history::TransactionItemStatus::NotExecuted)
    );
    assert_eq!(
        exec.transaction.reversible,
        weave_history::Reversibility::None
    );
}

#[test]
fn fault_partial_failure_tx_records_only_actual_mutations() {
    let (ws, all) = dup_ws("fault-partial", 4);
    let paths = &all[..3]; // 选中 3 条，保留 1 条
    let plan = plan_for(&ws, paths);
    let adapter = FlakyAdapter {
        ok_first: 2,
        calls: AtomicUsize::new(0),
    };
    let exec = execute_recycle_plan_with(&plan, &adapter, &CancellationToken::new(), &mut |_| {});
    // §111：15 success / 3 failed ⇒ transaction 只反映真实发生的 2 条
    assert_eq!((exec.recycled, exec.failed), (2, 1));
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 3);
    let executed: Vec<_> = exec
        .transaction
        .items
        .iter()
        .filter(|t| t.status == weave_history::TransactionItemStatus::Executed)
        .collect();
    assert_eq!(executed.len(), 2);
    assert!(executed.iter().all(|t| {
        t.target_path
            .starts_with(crate::recycle::RECYCLE_TARGET_PREFIX)
    }));
    assert_eq!(
        exec.transaction.reversible,
        weave_history::Reversibility::Partial
    );
    // 第 3 个文件仍在磁盘（适配器报告 Failed = 真没删）
    assert!(std::path::Path::new(&paths[2]).exists());
}

#[test]
fn fault_cancel_midway_stops_calling_adapter_at_safe_point() {
    let (ws, all) = dup_ws("fault-cancel-mid", 5);
    let paths = &all[..4]; // 选中 4 条，保留 1 条
    let plan = plan_for(&ws, paths);
    // 适配器在第 2 次调用后触发取消：第 3、4 条必须不再被调用
    struct CancelAfter {
        after: usize,
        calls: AtomicUsize,
        cancel: weave_core::prelude::CancellationToken,
    }
    impl RecycleAdapter for CancelAfter {
        fn recycle(&self, path: &std::path::Path) -> RecycleOutcome {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n >= self.after {
                panic!("adapter called after cancel safe-point");
            }
            if n + 1 == self.after {
                self.cancel.cancel();
            }
            RecycleOutcome::Success(RecycleReceipt {
                original_path: path.to_path_buf(),
                token: Some(format!("tok-{n}")),
                deleted_at: SystemTime::UNIX_EPOCH,
            })
        }
    }
    let cancel = CancellationToken::new();
    let adapter = CancelAfter {
        after: 2,
        calls: AtomicUsize::new(0),
        cancel: cancel.clone(),
    };
    let exec = execute_recycle_plan_with(&plan, &adapter, &cancel, &mut |_| {});
    assert_eq!(
        adapter.calls.load(Ordering::SeqCst),
        2,
        "安全点后不得再调用适配器"
    );
    assert_eq!((exec.recycled, exec.skipped), (2, 2));
    // 未处理条目 + 保留条目都原样存在
    for p in &all[2..] {
        assert!(std::path::Path::new(p).exists(), "{p} 必须原样保留");
    }
    let not_executed: Vec<_> = exec
        .transaction
        .items
        .iter()
        .filter(|t| t.status == weave_history::TransactionItemStatus::NotExecuted)
        .collect();
    assert_eq!(not_executed.len(), 2);
}

#[test]
fn fault_undo_conflict_when_original_occupied_and_missing_when_absent() {
    use crate::undo::UndoItemStatus;
    let ws = TempWorkspace::new("fault-undo").expect("ws");
    let occupied = ws.path().join("occupied.txt");
    std::fs::write(&occupied, "user recreated this").expect("write");
    let gone = ws.path().join("gone.txt");

    let receipts = vec![
        RecycleReceipt {
            original_path: occupied.clone(),
            token: Some("tok-a".to_string()),
            deleted_at: SystemTime::now(),
        },
        RecycleReceipt {
            original_path: gone.clone(),
            token: Some("tok-b".to_string()),
            deleted_at: SystemTime::UNIX_EPOCH,
        },
    ];
    let results = crate::recycle::restore_recycled(&receipts);
    // 原位被用户重建 ⇒ UndoConflict，不覆盖（§67）
    assert_eq!(results[0].1, UndoItemStatus::UndoConflict);
    assert_eq!(
        std::fs::read_to_string(&occupied).expect("read"),
        "user recreated this",
        "绝不覆盖用户文件"
    );
    // 从未回收过（回收站里自然没有）⇒ Missing，如实报告
    assert_eq!(results[1].1, UndoItemStatus::Missing);
}

// ──（下）§92 Integration Test：真实文件系统全闭环（M3 最关键测试组）──

#[test]
fn integration_closed_loop_scan_group_select_recycle_history_undo() {
    use weave_history::{HistoryEntry, HistoryStore, OperationStatus, Reversibility};

    // create temp root: A/B/C 重复 + 独立 D（中文/emoji/括号名，§96 Unicode）
    let ws = TempWorkspace::new("dup-loop").expect("ws");
    let sub = ws.dir("子目录").expect("dir");
    let content = "闭环测试内容 closed-loop";
    let a = ws.file("报告.txt", content).expect("write");
    let b = sub.join("副本 📋.txt");
    std::fs::write(&b, content).expect("write");
    let c = ws.file("c [2].txt", content).expect("write");
    let d = ws.file("unique.txt", "唯一内容 unique").expect("write");
    let all = [
        a.to_string_lossy().into_owned(),
        b.to_string_lossy().into_owned(),
        c.to_string_lossy().into_owned(),
        d.to_string_lossy().into_owned(),
    ];

    // scan → group（§95：路径大小写/名字不污染内容身份）
    let report = scan(&ws);
    assert_eq!(report.duplicate_groups, 1, "{:#?}", report.groups);
    assert_eq!(report.duplicate_files, 3);
    let group = &report.groups[0];
    // §137 不变量：组内 size/Hash 一致，GroupId 派生自内容
    assert!(group.files.iter().all(|f| f.size == content.len() as u64));
    assert!(
        group
            .files
            .iter()
            .all(|f| f.full_hash == group.files[0].full_hash)
    );

    // select（保留首个）→ preview(plan)
    let sel_paths: Vec<String> = group.files[1..].iter().map(|f| f.path.clone()).collect();
    let plan = build_recycle_plan(
        &report,
        &[RecycleSelection {
            group_id: group.group_id.clone(),
            recycle_paths: sel_paths.clone(),
        }],
    )
    .expect("plan");
    assert_eq!(plan.status_counts().0, 2, "2 ready");

    // execute recycle（真实回收站）
    let exec = execute_recycle_plan(&plan, &CancellationToken::new(), &mut |_| {});
    assert_eq!((exec.recycled, exec.failed, exec.skipped), (2, 0, 0));
    // verify filesystem：仅选中者受影响（§138 Mutation Invariants）
    // keeper = 组内排序首位（扫描序），与书写顺序无关
    let keeper = &group.files[0].path;
    assert!(std::path::Path::new(keeper).exists(), "保留首份必须在盘");
    for gone in &sel_paths {
        assert!(!std::path::Path::new(gone).exists(), "{gone} 应已回收");
    }
    assert!(std::path::Path::new(&all[3]).exists(), "unique 未受影响");

    // history（真实 HistoryStore 落盘；summary/entry 与服务层同构）
    let hist = TempWorkspace::new("dup-loop-history").expect("hist");
    let store = HistoryStore::open(hist.path()).expect("store");
    let entry = HistoryEntry {
        operation_id: exec.transaction.operation_id.clone(),
        kind: exec.transaction.kind,
        timestamp: exec.transaction.timestamp,
        summary: format!("Recycle {} files", exec.recycled),
        item_count: exec.transaction.items.len() as u64,
        success_count: exec.recycled,
        failed_count: exec.failed,
        skipped_count: exec.skipped,
        undoable: exec.recycled > 0,
        status: OperationStatus::Completed,
        input_root: None,
        rule_summary: None,
    };
    store
        .save_transaction(&exec.transaction)
        .and_then(|_| store.upsert_entry(entry))
        .expect("persist");
    drop(store);

    // 重启模拟：重开 store 读取事务（§112/§141：历史不丢）
    let reopened = HistoryStore::open(hist.path()).expect("reopen");
    let tx = reopened
        .load_transaction(&exec.transaction.operation_id)
        .expect("load")
        .expect("tx exists");
    assert_eq!(tx.status, OperationStatus::Completed);
    assert_eq!(tx.reversible, Reversibility::Full);
    assert_eq!(
        tx.items
            .iter()
            .filter(|i| i.status == weave_history::TransactionItemStatus::Executed)
            .count(),
        2
    );

    // undo（真实回收站还原）→ verify restore
    let undo =
        crate::recycle::undo_recycle_transaction(&tx, &CancellationToken::new(), &mut |_| {});
    assert_eq!(
        (undo.restored, undo.conflicts, undo.leaked_temps),
        (2, 0, 0),
        "{undo:?}"
    );
    for p in all.iter().take(3) {
        assert!(std::path::Path::new(p).exists(), "{p} 应被还原");
    }
    for p in all.iter().take(3) {
        assert_eq!(std::fs::read_to_string(p).expect("read"), content);
    }
    assert_eq!(
        std::fs::read_to_string(&all[3]).expect("read d"),
        "唯一内容 unique"
    );
}

#[test]
fn integration_fault_source_vanishes_between_scan_and_execute() {
    // §146：scan 与 execute 之间删除 source ⇒ 该条 skipped，绝不误回收
    let (ws, all) = dup_ws("fault-vanish", 4);
    let paths = &all[..3]; // 选中 3 条（含将消失的一条），保留 1 条
    let report = scan(&ws);
    let group_id = report.groups[0].group_id.clone();
    std::fs::remove_file(&paths[0]).expect("delete between scan and execute");
    let plan = build_recycle_plan(
        &report,
        &[RecycleSelection {
            group_id,
            recycle_paths: paths.to_vec(),
        }],
    );
    // 已删除的路径不在扫描结果之外——但它在扫描结果内，计划照建；
    // 执行时 Revalidate 发现不存在 ⇒ failed，其余两条真实回收后 Undo 还原。
    let plan = plan.expect("plan contains scanned entries");
    let exec = execute_recycle_plan(&plan, &CancellationToken::new(), &mut |_| {});
    assert_eq!(exec.failed, 1, "消失文件必须结构化失败");
    assert!(
        exec.plan
            .items
            .iter()
            .any(|i| i.errors.iter().any(|e| e.code == "rename.sourceNotFound"))
    );
    assert_eq!(exec.recycled, 2);
    let undo = crate::recycle::undo_recycle_transaction(
        &exec.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.restored, 2);
    assert!(!std::path::Path::new(&paths[0]).exists());
}

// ──（下）§95/§88/§137：大小写无关、百文件组、不变量 ─────────────

#[test]
fn case_insensitive_paths_do_not_pollute_content_identity() {
    // §95：A.TXT 与 a.txt 内容相同 ⇒ 必须是 duplicate（路径比较不污染内容判定）
    let ws = TempWorkspace::new("dup-case").expect("ws");
    let d1 = ws.dir("one").expect("dir");
    let d2 = ws.dir("two").expect("dir");
    std::fs::write(d1.join("a.txt"), "CaseContent").expect("write");
    std::fs::write(d2.join("A.TXT"), "CaseContent").expect("write");
    let report = scan(&ws);
    assert_eq!(report.duplicate_groups, 1);
    assert_eq!(report.duplicate_files, 2);
    // 两个原始路径都被如实保留（不做平台归一）
    let paths: Vec<&str> = report.groups[0]
        .files
        .iter()
        .map(|f| f.path.as_str())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("a.txt")));
    assert!(paths.iter().any(|p| p.ends_with("A.TXT")));
}

#[test]
fn group_of_hundred_files_stays_consistent() {
    // §88 Grouping：100 文件 + §137 不变量（size/Hash 一致、GroupId=内容派生）
    let ws = TempWorkspace::new("dup-hundred").expect("ws");
    let content = "hundred files same content";
    for i in 0..100 {
        let d = ws.dir(&format!("d{:02}", i / 10)).expect("dir");
        std::fs::write(d.join(format!("f{i:03}.txt")), content).expect("write");
    }
    let report = scan(&ws);
    assert_eq!(report.duplicate_groups, 1);
    let group = &report.groups[0];
    assert_eq!(group.file_count, 100);
    assert_eq!(group.wasted_size, 99 * content.len() as u64);
    let hash = &group.files[0].full_hash;
    assert!(
        group
            .files
            .iter()
            .all(|f| f.full_hash == *hash && f.size == content.len() as u64)
    );
    assert_eq!(group.group_id, format!("grp_{}", &hash[..16]));
}

#[test]
fn unique_sizes_never_reach_full_hash() {
    // §137：唯一 size ⇒ 在 Filtering 层即被排除，绝不进入 full hash
    let ws = TempWorkspace::new("dup-unique-size").expect("ws");
    ws.file("a.txt", "A").expect("write");
    ws.file("b.txt", "BB").expect("write");
    ws.file("c.bin", "CCC").expect("write");
    let report = scan(&ws);
    assert_eq!(report.candidate_files, 0);
    assert_eq!(report.partial_hashed, 0);
    assert_eq!(report.full_hashed, 0);
    assert_eq!(report.duplicate_groups, 0);
}
