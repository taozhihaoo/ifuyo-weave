//! 执行器 + Undo 测试矩阵（M2 §75–§78 / §100 / §101）。
//!
//! 全部使用真实临时目录 + 真实文件系统（§110：integration tests 必须真实）。

use super::execute::execute_plan;
use super::organize::build_organizer_plan;
use super::rename::build_rename_plan;
use super::undo::undo_transaction;
use crate::fs::StdFilesystem;
use weave_core::prelude::{CancellationToken, CollisionKind, OperationKind, PlanItemStatus};
use weave_history::TransactionItemStatus;
use weave_testkit::TempWorkspace;

fn rename_plan(ws: &TempWorkspace, inputs: &[&str], template: &str) -> weave_core::prelude::Plan {
    build_rename_plan(
        &StdFilesystem,
        &inputs
            .iter()
            .map(|r| ws.path().join(r).to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        &[],
        Some(template),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan")
}

fn names_of(ws: &TempWorkspace) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(ws.path())
        .expect("read")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// ─── 基本执行 ───

#[test]
fn simple_rename_executes_and_records_transaction() {
    let ws = TempWorkspace::new("ex-simple").expect("ws");
    ws.file("IMG_001.jpg", "content-a").expect("write");
    let plan = rename_plan(&ws, &["IMG_001.jpg"], "vacation-{counter}.{ext}");

    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );

    assert_eq!(report.executed, 1);
    assert_eq!(report.failed, 0);
    assert!(!names_of(&ws).contains(&"IMG_001.jpg".to_string()));
    assert!(names_of(&ws).contains(&"vacation-001.jpg".to_string()));

    assert_eq!(report.transaction.items.len(), 1);
    let item = &report.transaction.items[0];
    assert_eq!(item.status, TransactionItemStatus::Executed);
    assert!(item.source_path.ends_with("IMG_001.jpg"));
    assert!(item.target_path.ends_with("vacation-001.jpg"));
    assert_eq!(item.original_size, Some(9));
}

#[test]
fn precondition_failure_source_changed_is_safe_failure() {
    let ws = TempWorkspace::new("ex-changed").expect("ws");
    ws.file("a.txt", "original content").expect("write");
    let plan = rename_plan(&ws, &["a.txt"], "b-{counter}.txt");

    // Plan 之后外部修改文件内容（size 变化）
    std::fs::write(ws.path().join("a.txt"), "changed!").expect("modify");

    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(report.failed, 1);
    // 文件必须原地未动（safe failure，§74）
    assert!(ws.path().join("a.txt").exists());
    assert!(!ws.path().join("b-001.txt").exists());
    assert_eq!(report.plan.items[0].errors[0].code, "rename.sourceChanged");
}

#[test]
fn target_reappearing_after_plan_fails_without_overwrite() {
    let ws = TempWorkspace::new("ex-toctou").expect("ws");
    ws.file("a.txt", "source").expect("write");
    let plan = rename_plan(&ws, &["a.txt"], "b-{counter}.txt");

    // Plan 之后外部创建目标文件
    std::fs::write(ws.path().join("b-001.txt"), "external newcomer").expect("create target");

    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(report.failed, 1);
    assert!(ws.path().join("a.txt").exists(), "source untouched");
    let content = std::fs::read_to_string(ws.path().join("b-001.txt")).expect("read");
    assert_eq!(content, "external newcomer", "no overwrite (M2 §98)");
}

// ─── 两阶段：环与 case-only ───

#[test]
fn cycle_rename_two_phase_no_data_loss() {
    // 真环 A↔B：手工构造 Plan（uniform 规则不会产生真环——见 rename/tests.rs）。
    let ws = TempWorkspace::new("ex-cycle").expect("ws");
    ws.file("a.txt", "content-A").expect("write");
    ws.file("b.txt", "content-B").expect("write");
    let plan = weave_core::prelude::Plan {
        operation_id: weave_core::prelude::OperationId::generate(),
        kind: OperationKind::Rename,
        created_at: std::time::SystemTime::now(),
        items: vec![
            weave_core::prelude::PlanItem {
                item_id: "item_0000".to_string(),
                source_path: ws.path().join("a.txt").to_string_lossy().into_owned(),
                target_path: ws.path().join("b.txt").to_string_lossy().into_owned(),
                status: PlanItemStatus::Ready,
                collision: CollisionKind::Cycle,
                source_size: Some(9),
                source_modified: None,
                warnings: vec![],
                errors: vec![],
            },
            weave_core::prelude::PlanItem {
                item_id: "item_0001".to_string(),
                source_path: ws.path().join("b.txt").to_string_lossy().into_owned(),
                target_path: ws.path().join("a.txt").to_string_lossy().into_owned(),
                status: PlanItemStatus::Ready,
                collision: CollisionKind::Cycle,
                source_size: Some(9),
                source_modified: None,
                warnings: vec![],
                errors: vec![],
            },
        ],
    };

    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    eprintln!(
        "CYCLE-DEBUG executed={} items={:?}",
        report.executed,
        report
            .plan
            .items
            .iter()
            .map(|i| (
                &i.status,
                &i.collision,
                i.errors.iter().map(|e| e.code.clone()).collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(report.executed, 2);
    // 内容交换正确（两阶段让位，无数据丢失）
    assert_eq!(
        std::fs::read_to_string(ws.path().join("b.txt")).expect("read"),
        "content-A",
        "no data loss through two-phase"
    );
    assert_eq!(
        std::fs::read_to_string(ws.path().join("a.txt")).expect("read"),
        "content-B"
    );
    // temp 不泄漏
    let leftovers: Vec<_> = names_of(&ws)
        .into_iter()
        .filter(|n| n.contains(".weave-tmp-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp files must not leak: {leftovers:?}"
    );

    // Undo（LIFO）把两个文件都换回原内容
    let undo = super::undo::undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.restored, 2);
    assert_eq!(
        std::fs::read_to_string(ws.path().join("a.txt")).expect("read"),
        "content-A"
    );
    assert_eq!(
        std::fs::read_to_string(ws.path().join("b.txt")).expect("read"),
        "content-B"
    );
}

#[test]
fn case_only_rename_executes_via_temp() {
    let ws = TempWorkspace::new("ex-case").expect("ws");
    ws.file("Photo.jpg", "pic").expect("write");
    // Case 规则产生 Photo.jpg → photo.jpg（{original} 模板是 NoOp，不用）
    let plan = build_rename_plan(
        &StdFilesystem,
        &[ws.path().join("Photo.jpg").to_string_lossy().into_owned()],
        &[super::RenameRule::Case {
            form: super::CaseForm::Lower,
        }],
        None,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan");
    assert_eq!(plan.items[0].collision, CollisionKind::CaseOnly);

    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(report.executed, 1);
    assert!(names_of(&ws).contains(&"photo.jpg".to_string()));
    assert!(
        !names_of(&ws).contains(&"Photo.jpg".to_string()),
        "case rename must take effect"
    );
}

// ─── 取消 ───

#[test]
fn cancel_mid_batch_stops_with_partial_outcomes() {
    let ws = TempWorkspace::new("ex-cancel").expect("ws");
    for name in ["f01.txt", "f02.txt", "f03.txt", "f04.txt", "f05.txt"] {
        ws.file(name, "x").expect("write");
    }
    let inputs: Vec<String> = ["f01.txt", "f02.txt", "f03.txt", "f04.txt", "f05.txt"]
        .iter()
        .map(|r| ws.path().join(r).to_string_lossy().into_owned())
        .collect();
    let plan = build_rename_plan(
        &StdFilesystem,
        &inputs,
        &[super::RenameRule::Prefix {
            text: "done-".to_string(),
        }],
        None,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan");

    // 第一个条目处理后取消
    let cancel = CancellationToken::new();
    let first = std::sync::atomic::AtomicUsize::new(0);
    let report = execute_plan(&StdFilesystem, &plan, &cancel, &mut |_| {
        if first.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            cancel.cancel();
        }
    });

    assert_eq!(
        report.executed, 1,
        "only the first item executes before cancel"
    );
    assert!(names_of(&ws).contains(&"done-f01.txt".to_string()));
    assert!(
        names_of(&ws).contains(&"f02.txt".to_string()),
        "untouched items keep names"
    );
    // 事务如实记录：1 Executed + 4 NotExecuted
    let executed = report
        .transaction
        .items
        .iter()
        .filter(|i| i.status == TransactionItemStatus::Executed)
        .count();
    let not_executed = report
        .transaction
        .items
        .iter()
        .filter(|i| i.status == TransactionItemStatus::NotExecuted)
        .count();
    assert_eq!(executed, 1);
    assert_eq!(not_executed, 4);
}

// ─── Undo ───

#[test]
fn undo_simple_rename_restores_file() {
    let ws = TempWorkspace::new("ex-undo").expect("ws");
    ws.file("old.txt", "payload").expect("write");
    let plan = rename_plan(&ws, &["old.txt"], "new-{counter}.txt");
    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert!(
        report.failed == 0 && report.executed == 1,
        "execute: executed={} failed={} skipped={} items={:?}",
        report.executed,
        report.failed,
        report.skipped,
        report
            .plan
            .items
            .iter()
            .map(|i| (
                &i.status,
                &i.target_path,
                i.errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );
    assert!(
        ws.path().join("new-001.txt").exists(),
        "dir now: {:?}; plan target: {:?}",
        names_of(&ws),
        plan.items[0].target_path
    );

    let undo = undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.restored, 1);
    assert_eq!(undo.conflicts, 0);
    assert!(ws.path().join("old.txt").exists());
    assert!(!ws.path().join("new-001.txt").exists());
    let content = std::fs::read_to_string(ws.path().join("old.txt")).expect("read");
    assert_eq!(content, "payload");
}

#[test]
fn undo_multi_rename_is_lifo_and_restores_everything() {
    let ws = TempWorkspace::new("ex-undo-multi").expect("ws");
    for (name, content) in [("x1.txt", "1"), ("x2.txt", "2"), ("x3.txt", "3")] {
        ws.file(name, content).expect("write");
    }
    let inputs: Vec<String> = ["x1.txt", "x2.txt", "x3.txt"]
        .iter()
        .map(|r| ws.path().join(r).to_string_lossy().into_owned())
        .collect();
    let plan = build_rename_plan(
        &StdFilesystem,
        &inputs,
        &[super::RenameRule::Prefix {
            text: "y-".to_string(),
        }],
        None,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan");
    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(report.executed, 3);

    let undo = undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.restored, 3);
    let names = names_of(&ws);
    assert!(names.contains(&"x1.txt".to_string()));
    assert!(names.contains(&"x2.txt".to_string()));
    assert!(names.contains(&"x3.txt".to_string()));
    assert!(names.iter().all(|n| !n.starts_with("y-")));
}

#[test]
fn undo_cycle_rename_swaps_back() {
    let ws = TempWorkspace::new("ex-undo-cycle").expect("ws");
    ws.file("a.txt", "content-A").expect("write");
    ws.file("b.txt", "content-B").expect("write");
    // 真环 A↔B（手工构造；uniform 规则见 rename/tests.rs 的 ExistingTarget 场景）
    let plan = weave_core::prelude::Plan {
        operation_id: weave_core::prelude::OperationId::generate(),
        kind: OperationKind::Rename,
        created_at: std::time::SystemTime::now(),
        items: vec![
            weave_core::prelude::PlanItem {
                item_id: "item_0000".to_string(),
                source_path: ws.path().join("a.txt").to_string_lossy().into_owned(),
                target_path: ws.path().join("b.txt").to_string_lossy().into_owned(),
                status: PlanItemStatus::Ready,
                collision: CollisionKind::Cycle,
                source_size: Some(9),
                source_modified: None,
                warnings: vec![],
                errors: vec![],
            },
            weave_core::prelude::PlanItem {
                item_id: "item_0001".to_string(),
                source_path: ws.path().join("b.txt").to_string_lossy().into_owned(),
                target_path: ws.path().join("a.txt").to_string_lossy().into_owned(),
                status: PlanItemStatus::Ready,
                collision: CollisionKind::Cycle,
                source_size: Some(9),
                source_modified: None,
                warnings: vec![],
                errors: vec![],
            },
        ],
    };
    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(
        report.executed, 2,
        "both sides of the cycle execute via two-phase"
    );

    let undo = undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.restored, 2);
    // a.txt 回来了，内容是 A；b.txt 仍是 B
    assert_eq!(
        std::fs::read_to_string(ws.path().join("a.txt")).expect("read"),
        "content-A"
    );
    assert_eq!(
        std::fs::read_to_string(ws.path().join("b.txt")).expect("read"),
        "content-B"
    );
}

#[test]
fn undo_refuses_when_target_externally_replaced() {
    let ws = TempWorkspace::new("ex-undo-conflict").expect("ws");
    ws.file("old.txt", "original").expect("write");
    let plan = rename_plan(&ws, &["old.txt"], "new-{counter}.txt");
    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );

    // 用户把 new-001.txt 替换成另一个文件
    std::fs::write(ws.path().join("new-001.txt"), "user's newer file").expect("replace");

    let undo = undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(
        undo.conflicts, 1,
        "must refuse to overwrite external change (M2 §49)"
    );
    assert!(
        ws.path().join("new-001.txt").exists(),
        "user's file untouched"
    );
    assert!(!ws.path().join("old.txt").exists(), "no partial restore");
    let content = std::fs::read_to_string(ws.path().join("new-001.txt")).expect("read");
    assert_eq!(content, "user's newer file");
}

#[test]
fn undo_refuses_when_original_location_occupied() {
    let ws = TempWorkspace::new("ex-undo-occupied").expect("ws");
    ws.file("old.txt", "original").expect("write");
    let plan = rename_plan(&ws, &["old.txt"], "new-{counter}.txt");
    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );

    // 原位出现新文件
    std::fs::write(ws.path().join("old.txt"), "newcomer at source").expect("create");

    let undo = undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.conflicts, 1);
    assert!(
        ws.path().join("new-001.txt").exists(),
        "undo target untouched"
    );
    assert_eq!(
        std::fs::read_to_string(ws.path().join("old.txt")).expect("read"),
        "newcomer at source"
    );
}

#[test]
fn undo_missing_target_is_honest_missing() {
    let ws = TempWorkspace::new("ex-undo-missing").expect("ws");
    ws.file("old.txt", "original").expect("write");
    let plan = rename_plan(&ws, &["old.txt"], "new-{counter}.txt");
    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    std::fs::remove_file(ws.path().join("new-001.txt")).expect("remove target");

    let undo = undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.results[0].status, super::undo::UndoItemStatus::Missing);
    assert_eq!(undo.restored, 0);
}

#[test]
fn undo_partial_execution_only_touches_executed_items() {
    let ws = TempWorkspace::new("ex-undo-partial").expect("ws");
    ws.file("good.txt", "g").expect("write");
    let plan_full = rename_plan(&ws, &["good.txt"], "moved-{counter}.txt");
    let report = execute_plan(
        &StdFilesystem,
        &plan_full,
        &CancellationToken::new(),
        &mut |_| {},
    );

    // 手工构造一条含 NotExecuted 项的事务（部分执行场景）
    let mut tx = report.transaction.clone();
    tx.items.push(weave_history::TransactionItem {
        item_id: "item_9999".to_string(),
        source_path: ws.path().join("never.txt").to_string_lossy().into_owned(),
        target_path: ws
            .path()
            .join("never-moved.txt")
            .to_string_lossy()
            .into_owned(),
        status: weave_history::TransactionItemStatus::NotExecuted,
        timestamp: None,
        original_size: None,
        original_modified: None,
        original_created: None,
    });

    let undo = undo_transaction(&StdFilesystem, &tx, &CancellationToken::new(), &mut |_| {});
    assert_eq!(undo.restored, 1, "only the executed item is restored");
    assert!(
        ws.path().join("good.txt").exists(),
        "executed item restored"
    );
    assert!(
        !ws.path().join("never-moved.txt").exists(),
        "no fake recovery record (M2 §50)"
    );
}

// ─── Organizer 执行 ───

#[test]
fn diag_double_stat_size() {
    let ws = TempWorkspace::new("diag-stat").expect("ws");
    ws.file("a.jpg", "image-payload").expect("write");
    let p1 = ws.path().join("a.jpg");
    use crate::fs::Filesystem as _;
    let s1 = StdFilesystem.stat(&p1).expect("s1");
    let s2 = StdFilesystem.stat(&p1).expect("s2");
    eprintln!("STAT-DEBUG s1={} s2={}", s1.size, s2.size);
}

#[test]
fn organizer_executes_move_with_destination_creation_and_undo() {
    let ws = TempWorkspace::new("ex-org").expect("ws");
    ws.file("a.jpg", "image-payload").expect("write");
    let rules = vec![super::OrganizerRule {
        condition: super::OrganizerCondition::ExtensionIn {
            extensions: vec!["jpg".to_string()],
        },
        target_folder: "image".to_string(),
    }];
    let plan = build_organizer_plan(
        &StdFilesystem,
        &ws.path().to_string_lossy(),
        &rules,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan");
    assert_eq!(plan.kind, OperationKind::Move);
    {
        use crate::fs::Filesystem as _;
        let pre = StdFilesystem
            .stat(&ws.path().join("a.jpg"))
            .expect("pre-stat");
        eprintln!(
            "PRE-EXEC-DEBUG size={} plan_expected={:?}",
            pre.size, plan.items[0].source_size
        );
    }

    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    eprintln!(
        "ORG-DEBUG executed={} items={:?}",
        report.executed,
        report
            .plan
            .items
            .iter()
            .map(|i| (
                &i.status,
                &i.source_path,
                i.errors
                    .iter()
                    .map(|e| format!("{}: {}", e.code, e.message))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(report.executed, 1);
    // 目标目录被创建（§77 destination creation）
    let moved = ws.path().join("image").join("a.jpg");
    assert!(moved.exists(), "destination folder auto-created");
    assert_eq!(
        std::fs::read_to_string(moved).expect("read"),
        "image-payload",
        "content intact"
    );

    // Undo：移回根目录
    let undo = undo_transaction(
        &StdFilesystem,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(undo.restored, 1);
    assert!(ws.path().join("a.jpg").exists());
}

#[test]
fn organizer_cross_volume_move_is_refused_not_faked() {
    // 构造一个 target 在“另一卷”的 plan（不同盘符前缀）
    let ws = TempWorkspace::new("ex-cross").expect("ws");
    ws.file("a.txt", "x").expect("write");
    let plan = weave_core::prelude::Plan {
        operation_id: weave_core::prelude::OperationId::generate(),
        kind: OperationKind::Move,
        created_at: std::time::SystemTime::now(),
        items: vec![weave_core::prelude::PlanItem {
            item_id: "item_0000".to_string(),
            // 本机存在 C: 盘；目标用 Q:（几乎必然不存在/非同卷）
            source_path: ws.path().join("a.txt").to_string_lossy().into_owned(),
            target_path: "Q:\\somewhere\\a.txt".to_string(),
            status: PlanItemStatus::Ready,
            collision: CollisionKind::None,
            source_size: Some(1),
            source_modified: None,
            warnings: vec![],
            errors: vec![],
        }],
    };
    let report = execute_plan(
        &StdFilesystem,
        &plan,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(report.failed, 1);
    assert_eq!(
        report.plan.items[0].errors[0].code,
        "move.crossVolumeUnsupported"
    );
    assert!(ws.path().join("a.txt").exists(), "source untouched");
}

#[test]
fn rename_permission_failure_is_structured_and_batch_continues() {
    let ws = TempWorkspace::new("ex-perm").expect("ws");
    ws.file("locked.txt", "keep").expect("write");
    ws.file("movable.txt", "move me").expect("write");
    let inputs: Vec<String> = ["locked.txt", "movable.txt"]
        .iter()
        .map(|r| ws.path().join(r).to_string_lossy().into_owned())
        .collect();

    // 构造 Plan 后换用注入 rename 故障的 fs 执行第一个 rename
    let plan = build_rename_plan(
        &StdFilesystem,
        &inputs,
        &[super::RenameRule::Prefix {
            text: "renamed-".to_string(),
        }],
        None,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan");

    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    struct DenyFirstRename {
        inner: StdFilesystem,
        denied: AtomicBool,
    }
    impl crate::fs::Filesystem for DenyFirstRename {
        fn exists(&self, p: &Path) -> bool {
            self.inner.exists(p)
        }
        fn stat(&self, p: &Path) -> std::io::Result<crate::metadata::FileStat> {
            self.inner.stat(p)
        }
        fn read_dir(&self, p: &Path) -> std::io::Result<Vec<String>> {
            self.inner.read_dir(p)
        }
        fn open_read(&self, p: &Path) -> std::io::Result<Box<dyn std::io::Read + Send>> {
            self.inner.open_read(p)
        }
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            if !self.denied.swap(true, Ordering::SeqCst) {
                return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
            }
            self.inner.rename(from, to)
        }
        fn create_dir_all(&self, p: &Path) -> std::io::Result<()> {
            self.inner.create_dir_all(p)
        }
    }
    // 排序后 locked.txt 在 movable.txt 之前（case-insensitive ASC）→ 第一次 rename 被拒
    let faulty = DenyFirstRename {
        inner: StdFilesystem,
        denied: AtomicBool::new(false),
    };
    let report = execute_plan(&faulty, &plan, &CancellationToken::new(), &mut |_| {});
    eprintln!(
        "PERM-DEBUG executed={} failed={} skipped={} statuses={:?} errors={:?}",
        report.executed,
        report.failed,
        report.skipped,
        report
            .plan
            .items
            .iter()
            .map(|i| (
                &i.status,
                i.errors.iter().map(|e| e.code.clone()).collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        report
            .transaction
            .items
            .iter()
            .map(|i| (&i.item_id, &i.status))
            .collect::<Vec<_>>()
    );

    assert_eq!(report.failed, 1);
    assert_eq!(
        report.executed, 1,
        "second item must continue (§40 partial success)"
    );
    assert_eq!(
        report.plan.items[0].errors[0].code,
        "rename.permissionDenied"
    );
    assert!(
        ws.path().join("locked.txt").exists(),
        "failed item untouched"
    );
    assert!(
        ws.path().join("renamed-movable.txt").exists(),
        "batch continued; dir={:?}; item1 target={:?}",
        names_of(&ws),
        report.plan.items[1].target_path
    );

    // 部分成功的事务：1 Executed + 1 NotExecuted；undo 只恢复 executed
    let undo = undo_transaction(
        &faulty,
        &report.transaction,
        &CancellationToken::new(),
        &mut |_| {},
    );
    eprintln!(
        "UNDO-DEBUG restored={} conflicts={} results={:?}",
        undo.restored,
        undo.conflicts,
        undo.results
            .iter()
            .map(|r| (
                &r.status,
                r.reason
                    .as_ref()
                    .map(|e| format!("{}: {}", e.code, e.message))
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(undo.restored, 1);
    assert!(!ws.path().join("renamed-movable.txt").exists());
    assert!(ws.path().join("locked.txt").exists());
}
