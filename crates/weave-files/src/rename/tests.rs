//! Rename Plan 构建器测试（M2 §75/§79/§80：golden、碰撞矩阵、排序确定性、
//! Preview 纯净性、case-only、cycle、NoOp）。

use super::*;
use crate::fs::StdFilesystem;
use crate::scan::ScanOptions;
use weave_core::prelude::{CancellationToken, CollisionKind};
use weave_core::prelude::{Plan, PlanItemStatus};
use weave_testkit::{TempWorkspace, standard_tree};

const SEP: char = '\\';

fn plan_for(_ws: &TempWorkspace, inputs: &[String], rules: &[RenameRule]) -> Plan {
    build_rename_plan(
        &StdFilesystem,
        inputs,
        rules,
        None,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("plan")
}

fn p(ws: &TempWorkspace, rel: &str) -> String {
    ws.path().join(rel).to_string_lossy().into_owned()
}

#[test]
fn golden_vacation_counter_template() {
    let ws = TempWorkspace::new("rn-golden").expect("ws");
    for name in ["IMG_001.jpg", "IMG_002.jpg", "IMG_003.jpg"] {
        ws.file(name, "x").expect("write");
    }
    let inputs = vec![
        p(&ws, "IMG_001.jpg"),
        p(&ws, "IMG_002.jpg"),
        p(&ws, "IMG_003.jpg"),
    ];
    let plan = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Template {
            template: "vacation-{counter}.{ext}".to_string(),
        }],
    );

    assert_eq!(plan.kind, weave_core::prelude::OperationKind::Rename);
    assert_eq!(plan.items.len(), 3);
    // 排序：case-insensitive path ASC → IMG_001 < IMG_002 < IMG_003
    let pairs: Vec<(&str, &str, PlanItemStatus)> = plan
        .items
        .iter()
        .map(|i| {
            (
                i.source_path.rsplit(SEP).next().expect("name"),
                i.target_path.rsplit(SEP).next().expect("name"),
                i.status,
            )
        })
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("IMG_001.jpg", "vacation-001.jpg", PlanItemStatus::Ready),
            ("IMG_002.jpg", "vacation-002.jpg", PlanItemStatus::Ready),
            ("IMG_003.jpg", "vacation-003.jpg", PlanItemStatus::Ready),
        ]
    );
    // Plan 无错误项
    assert!(plan.items.iter().all(|i| i.errors.is_empty()));
}

#[test]
fn existing_target_becomes_conflict_with_structured_error() {
    let ws = TempWorkspace::new("rn-existing").expect("ws");
    ws.file("a.jpg", "a").expect("write");
    ws.file("b.jpg", "pre-existing target").expect("write");
    let inputs = vec![p(&ws, "a.jpg")];
    let plan = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Replace {
            find: "a".to_string(),
            replace_with: "b".to_string(),
        }],
    );
    assert_eq!(plan.items[0].status, PlanItemStatus::Conflict);
    assert_eq!(plan.items[0].collision, CollisionKind::ExistingTarget);
    assert_eq!(plan.items[0].errors[0].code, "rename.targetExists");
}

#[test]
fn internal_collision_marks_all_involved_items() {
    let ws = TempWorkspace::new("rn-internal").expect("ws");
    ws.file("a.jpg", "a").expect("write");
    ws.file("b.jpg", "b").expect("write");
    // 两条规则把 a/b 都变成 photo.jpg：用 Template 直接验证批内重复
    let inputs = vec![p(&ws, "a.jpg"), p(&ws, "b.jpg")];
    let plan = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Template {
            template: "photo.{ext}".to_string(),
        }],
    );
    let conflicts: Vec<_> = plan
        .items
        .iter()
        .filter(|i| i.status == PlanItemStatus::Conflict)
        .collect();
    assert_eq!(
        conflicts.len(),
        2,
        "both items targeting photo.jpg must conflict"
    );
    assert!(
        conflicts
            .iter()
            .all(|i| i.collision == CollisionKind::InternalTarget)
    );
    assert!(conflicts.iter().all(|i| {
        i.errors
            .iter()
            .any(|e| e.code == "rename.internalCollision")
    }));
}

#[test]
fn identity_rename_is_noop_not_error() {
    let ws = TempWorkspace::new("rn-noop").expect("ws");
    ws.file("photo.jpg", "x").expect("write");
    let inputs = vec![p(&ws, "photo.jpg")];
    // Prefix 空文本 = 无变化？Prefix 空文本也是 NoOp 的一种；直接用空规则列表
    let plan = plan_for(&ws, &inputs, &[]);
    assert_eq!(plan.items[0].status, PlanItemStatus::NoOp);
}

#[test]
fn case_only_rename_is_ready_with_case_collision_marked() {
    let ws = TempWorkspace::new("rn-case").expect("ws");
    ws.file("Photo.jpg", "x").expect("write");
    let inputs = vec![p(&ws, "Photo.jpg")];
    let plan = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Case {
            form: CaseForm::Lower,
        }],
    );
    assert_eq!(plan.items[0].status, PlanItemStatus::Ready);
    assert_eq!(plan.items[0].collision, CollisionKind::CaseOnly);
    // Preview 显示真实意图 foo → foo（小写），不暴露内部两阶段细节（M2 §23）
    assert!(plan.items[0].target_path.ends_with("photo.jpg"));
}

#[test]
fn cycle_rename_detected_as_cycle_collision_ready() {
    // a.txt → b.txt 且 b.txt 在批内：target 是另一条目的 source ⇒ Cycle。
    let ws = TempWorkspace::new("rn-cycle").expect("ws");
    ws.file("a.txt", "a").expect("write");
    ws.file("b.txt", "b").expect("write");
    let inputs = vec![p(&ws, "a.txt"), p(&ws, "b.txt")];
    let plan = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Replace {
            find: "a".to_string(),
            replace_with: "b".to_string(),
        }],
    );
    let item_a = plan
        .items
        .iter()
        .find(|i| i.source_path.ends_with("a.txt"))
        .expect("a");
    let item_b = plan
        .items
        .iter()
        .find(|i| i.source_path.ends_with("b.txt"))
        .expect("b");
    assert_eq!(item_a.collision, CollisionKind::Cycle);
    assert_eq!(
        item_a.status,
        PlanItemStatus::Ready,
        "cycle is executable via two-phase"
    );
    assert_eq!(item_b.status, PlanItemStatus::NoOp, "b → b is identity");
}

#[test]
fn ordering_is_case_insensitive_path_not_os_enumeration() {
    let ws = TempWorkspace::new("rn-order").expect("ws");
    for name in ["a2.txt", "a10.txt", "B.txt", "a.txt"] {
        ws.file(name, "x").expect("write");
    }
    let inputs = vec![
        p(&ws, "a10.txt"),
        p(&ws, "a2.txt"),
        p(&ws, "B.txt"),
        p(&ws, "a.txt"),
    ];
    let plan = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Counter {
            start: 1,
            step: 1,
            width: 3,
        }],
    );
    // case-insensitive ASC：a.txt < a10.txt < a2.txt < b.txt
    // Counter 替换 base；扩展名 .txt 保留（§17 base/ext 分离）
    let counters: Vec<&str> = plan
        .items
        .iter()
        .map(|i| i.target_path.rsplit(SEP).next().expect("name"))
        .collect();
    assert_eq!(counters, vec!["001.txt", "002.txt", "003.txt", "004.txt"]);
}

#[test]
fn invalid_inputs_become_invalid_items_not_plan_failure() {
    let ws = TempWorkspace::new("rn-invalid").expect("ws");
    ws.file("keep.txt", "x").expect("write");
    let missing = p(&ws, "ghost.txt");
    let relative = "relative\\path.txt".to_string();
    let inputs = vec![p(&ws, "keep.txt"), missing.clone(), relative];
    let plan = plan_for(&ws, &inputs, &[]);
    let statuses: Vec<PlanItemStatus> = plan.items.iter().map(|i| i.status).collect();
    // 排序（§20）：ghost < keep < relative（完整路径 case-insensitive ASC）
    assert_eq!(
        statuses,
        vec![
            PlanItemStatus::Invalid,
            PlanItemStatus::NoOp,
            PlanItemStatus::Invalid
        ]
    );
    assert_eq!(plan.items[0].errors[0].code, "path.notFound");
    assert_eq!(plan.items[2].errors[0].code, "path.relative");
}

#[test]
fn rule_errors_fail_fast_before_any_plan_item() {
    let ws = TempWorkspace::new("rn-rule-err").expect("ws");
    ws.file("a.txt", "x").expect("write");
    let inputs = vec![p(&ws, "a.txt")];
    let err = build_rename_plan(
        &StdFilesystem,
        &inputs,
        &[RenameRule::RegexReplace {
            pattern: "(".to_string(),
            replacement: "x".to_string(),
        }],
        None,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect_err("invalid regex must fail plan build");
    assert_eq!(err.code, "rename.invalidRegex");
}

#[test]
fn empty_selection_is_structured_error() {
    let _ws = TempWorkspace::new("rn-empty").expect("ws");
    let err = build_rename_plan(
        &StdFilesystem,
        &[],
        &[],
        None,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect_err("empty");
    assert_eq!(err.code, "rename.emptySelection");
}

#[test]
fn preview_purity_plan_building_never_writes() {
    // §36/§37：构建 Plan（含 Dry Run 语义）不得改动文件系统。
    let ws = TempWorkspace::new("rn-purity").expect("ws");
    let expected = standard_tree(&ws).expect("fixture");
    let inputs: Vec<String> = ["documents/a.txt", "images/picture.bin", "UPPER.TXT"]
        .iter()
        .map(|r| p(&ws, r))
        .collect();
    let _ = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Template {
            template: "renamed-{counter}.{ext}".to_string(),
        }],
    );
    // 快照对比：文件集合与大小完全不变
    let after = crate::scan::scan_directory(
        &StdFilesystem,
        &ws.path().to_string_lossy(),
        &ScanOptions::default(),
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("scan");
    assert_eq!(after.files_scanned, expected.files);
    assert_eq!(after.total_size, expected.total_size);
    assert_eq!(after.directories_scanned, expected.directories);
}

#[test]
fn deterministic_same_input_same_plan() {
    let ws = TempWorkspace::new("rn-det").expect("ws");
    for name in ["b.jpg", "a.jpg", "c.jpg"] {
        ws.file(name, "same content").expect("write");
    }
    let inputs = vec![p(&ws, "c.jpg"), p(&ws, "a.jpg"), p(&ws, "b.jpg")];
    let p1 = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Template {
            template: "n-{counter}.{ext}".to_string(),
        }],
    );
    // 乱序重放
    let inputs_rev: Vec<String> = inputs.iter().rev().cloned().collect();
    let p2 = plan_for(
        &ws,
        &inputs_rev,
        &[RenameRule::Template {
            template: "n-{counter}.{ext}".to_string(),
        }],
    );
    let pairs = |plan: &Plan| -> Vec<(String, String)> {
        plan.items
            .iter()
            .map(|i| (i.source_path.clone(), i.target_path.clone()))
            .collect()
    };
    assert_eq!(
        pairs(&p1),
        pairs(&p2),
        "same input set must yield identical plan"
    );
}

#[test]
fn unicode_names_flow_through_plan() {
    let ws = TempWorkspace::new("rn-unicode").expect("ws");
    ws.file("中文 文件.txt", "x").expect("write");
    let inputs = vec![p(&ws, "中文 文件.txt")];
    let plan = plan_for(
        &ws,
        &inputs,
        &[RenameRule::Prefix {
            text: "已整理-".to_string(),
        }],
    );
    assert_eq!(plan.items[0].status, PlanItemStatus::Ready);
    assert!(plan.items[0].target_path.ends_with("已整理-中文 文件.txt"));
}
