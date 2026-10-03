//! Rename Plan 构建器（M2 §10/§11/§20/§21/§24/§25）。
//!
//! Plan = 文件系统状态的事实快照（§11）。Preview 与 Execute 共用同一 Plan；
//! Execute 前做 Revalidate（§38）而非重新推导。

use std::path::PathBuf;

use weave_core::prelude::{
    CancellationToken, CollisionKind, OperationId, OperationKind, Plan, PlanItem, PlanItemStatus,
    Progress, WeaveError, validate_absolute_path,
};

use super::rule::{RenameContext, RenameRule, compile_rules, split_name};
use crate::fs::Filesystem;

/// 单个输入的名字候选（规则应用结果），供碰撞检测。与 items 平行（同下标）。
struct NameCandidate {
    item_id: String,
    source_normalized: String,
    target: PathBuf,
    target_normalized: String,
}

/// 构建重命名计划。
///
/// - `inputs`：用户提供的绝对路径（顺序无关；内部按 case-insensitive 路径
///   升序规范化，序号分配基于该顺序，M2 §20）。
/// - `rules`：有序规则；`template`：可选最终模板（最后应用）。
pub fn build_rename_plan(
    fs: &dyn Filesystem,
    inputs: &[String],
    rules: &[RenameRule],
    template: Option<&str>,
    cancel: &CancellationToken,
    report: &mut dyn FnMut(Progress),
) -> Result<Plan, WeaveError> {
    // 规则先编译：正则/模板错误在此失败，不产生半成品 Plan（M2 §13/§18）。
    let mut compiled = compile_rules(rules)?;
    if let Some(t) = template {
        compiled.push(
            compile_rules(&[RenameRule::Template {
                template: t.to_string(),
            }])?
            .into_iter()
            .next()
            .expect("single template rule"),
        );
    }

    if inputs.is_empty() {
        return Err(
            WeaveError::validation("rename.emptySelection", "no input files selected")
                .with_location("weave-files::rename::plan"),
        );
    }

    // 确定性排序：完整路径 case-insensitive 升序（M2 §20）。
    let mut sorted: Vec<&String> = inputs.iter().collect();
    sorted.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });

    let operation_id = OperationId::generate();
    let mut items: Vec<PlanItem> = Vec::with_capacity(sorted.len());
    let mut candidates: Vec<NameCandidate> = Vec::with_capacity(sorted.len());

    for (index, raw) in sorted.into_iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(WeaveError::cancelled(
                "rename.planCancelled",
                "plan building was cancelled",
            )
            .with_location("weave-files::rename::plan"));
        }
        if index % 256 == 0 {
            report(Progress::running(
                operation_id.clone(),
                index as u64,
                Some(inputs.len() as u64),
            ));
        }

        let item_id = format!("item_{index:04}");
        let validation = match validate_absolute_path(raw) {
            Ok(v) => v,
            Err(e) => {
                items.push(invalid_item(&item_id, raw, e));
                candidates.push(rejected_candidate(&item_id, raw));
                continue;
            }
        };
        let source = PathBuf::from(&validation.normalized);

        let stat = match fs.stat(&source) {
            Ok(s) => s,
            Err(e) => {
                let code = match e.kind() {
                    std::io::ErrorKind::NotFound => "path.notFound",
                    std::io::ErrorKind::PermissionDenied => "path.permissionDenied",
                    _ => "rename.statFailed",
                };
                items.push(invalid_item(
                    &item_id,
                    raw,
                    WeaveError::io(
                        code,
                        format!("cannot stat '{}': {e}", validation.normalized),
                    ),
                ));
                candidates.push(rejected_candidate(&item_id, raw));
                continue;
            }
        };
        if stat.kind != weave_core::prelude::FileKind::RegularFile
            && stat.kind != weave_core::prelude::FileKind::Symlink
        {
            items.push(invalid_item(
                &item_id,
                raw,
                WeaveError::validation("rename.notAFile", "only files can be renamed in M2"),
            ));
            candidates.push(rejected_candidate(&item_id, raw));
            continue;
        }

        let full_name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| validation.normalized.clone());
        let (base, ext) = split_name(&full_name);
        let context = RenameContext {
            index: index as u64,
            date: Some(&date_for(&stat)),
        };
        let (new_base, new_ext) =
            super::rule::apply_rules(&compiled, &base, ext.as_deref(), &context);
        let candidate_name = match &new_ext {
            Some(e) if !e.is_empty() => format!("{new_base}.{e}"),
            _ => new_base.clone(),
        };

        // 目标名安全校验：组合路径统一走 M1 path safety（M2 §24 不复制校验）。
        let parent = source
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| source.clone());
        let target = parent.join(&candidate_name);
        let target_validation = match validate_absolute_path(&target.to_string_lossy()) {
            Ok(v) => v,
            Err(e) => {
                items.push(invalid_item(&item_id, raw, e));
                candidates.push(rejected_candidate(&item_id, raw));
                continue;
            }
        };
        let mut warnings = Vec::new();
        if target_validation.exceeds_legacy_limit {
            warnings.push(
                "target path exceeds 260 characters; may fail without long-path support"
                    .to_string(),
            );
        }

        items.push(PlanItem {
            item_id: item_id.clone(),
            source_path: validation.normalized.clone(),
            target_path: target_validation.normalized.clone(),
            status: PlanItemStatus::Ready, // 碰撞检测在下方统一处理
            collision: CollisionKind::None,
            source_size: Some(stat.size),
            source_modified: stat.modified,
            warnings,
            errors: Vec::new(),
        });
        candidates.push(NameCandidate {
            item_id,
            source_normalized: validation.normalized.to_lowercase(),
            target: PathBuf::from(&target_validation.normalized),
            target_normalized: target_validation.normalized.to_lowercase(),
        });
    }

    detect_collisions(fs, &mut items, &candidates);

    Ok(Plan {
        operation_id,
        kind: OperationKind::Rename,
        created_at: std::time::SystemTime::now(),
        items,
    })
}

fn date_for(stat: &crate::metadata::FileStat) -> std::time::SystemTime {
    // §15：默认 Modified；不可用时退 Created。
    stat.modified
        .or(stat.created)
        .unwrap_or_else(std::time::SystemTime::now)
}

fn invalid_item(item_id: &str, requested: &str, error: WeaveError) -> PlanItem {
    PlanItem {
        item_id: item_id.to_string(),
        source_path: requested.to_string(),
        target_path: String::new(),
        status: PlanItemStatus::Invalid,
        collision: CollisionKind::None,
        source_size: None,
        source_modified: None,
        warnings: Vec::new(),
        errors: vec![error],
    }
}

fn rejected_candidate(item_id: &str, requested: &str) -> NameCandidate {
    NameCandidate {
        item_id: item_id.to_string(),
        source_normalized: requested.to_lowercase(),
        target: PathBuf::new(),
        target_normalized: String::new(),
    }
}

/// 碰撞检测（M2 §21 四类 + 环）。
///
/// 第一遍（只读）：逐条判 NoOp / CaseOnly / Cycle / ExistingTarget。
/// 第二遍：Ready 条目按归一化目标归并 ⇒ InternalTarget。
///
/// 环判定排除 **NoOp 条目的 source**：`b→b` 永不让位，对他人而言
/// `a→b` 是真实的 ExistingTarget 而非可让位的环。
fn detect_collisions(fs: &dyn Filesystem, items: &mut [PlanItem], candidates: &[NameCandidate]) {
    // Pass 0：NoOp 先行标记（供环判定的“会否让位”检查使用）。
    for (idx, item) in items.iter_mut().enumerate() {
        let cand = &candidates[idx];
        if item.status == PlanItemStatus::Ready
            && !cand.target_normalized.is_empty()
            && item.source_path == item.target_path
        {
            item.status = PlanItemStatus::NoOp;
        }
    }

    // Pass 1：NoOp / CaseOnly / Cycle / ExistingTarget。
    // 先不可变收集标记（环判定需要读其他条目状态），再统一应用。
    struct Mark {
        status: PlanItemStatus,
        collision: CollisionKind,
        error: Option<WeaveError>,
    }

    impl Clone for Mark {
        fn clone(&self) -> Self {
            Self {
                status: self.status,
                collision: self.collision,
                error: self.error.clone(),
            }
        }
    }
    let mut marks: Vec<Option<Mark>> = vec![None; items.len()];
    for (idx, item) in items.iter().enumerate() {
        let cand = &candidates[idx];
        if item.status != PlanItemStatus::Ready || cand.target_normalized.is_empty() {
            continue;
        }
        if cand.source_normalized == cand.target_normalized {
            marks[idx] = Some(Mark {
                status: PlanItemStatus::Ready,
                collision: CollisionKind::CaseOnly,
                error: None,
            });
            continue;
        }
        // 环：target 是另一条目的 source，且该条目会真正让位（非 NoOp）。
        let target_is_moving_batch_source = candidates.iter().enumerate().any(|(other_idx, c)| {
            c.item_id != cand.item_id
                && !c.source_normalized.is_empty()
                && c.source_normalized == cand.target_normalized
                && items[other_idx].status != PlanItemStatus::NoOp
        });
        if target_is_moving_batch_source {
            marks[idx] = Some(Mark {
                status: PlanItemStatus::Ready,
                collision: CollisionKind::Cycle,
                error: None,
            });
            continue;
        }
        if fs.exists(&cand.target) {
            marks[idx] = Some(Mark {
                status: PlanItemStatus::Conflict,
                collision: CollisionKind::ExistingTarget,
                error: Some(WeaveError::conflict(
                    "rename.targetExists",
                    format!("target already exists: {}", item.target_path),
                )),
            });
        }
    }
    for (idx, mark) in marks.into_iter().enumerate() {
        if let Some(mark) = mark {
            items[idx].status = mark.status;
            items[idx].collision = mark.collision;
            if let Some(e) = mark.error {
                items[idx].errors.push(e);
            }
        }
    }

    // InternalTarget：多个 Ready 条目归一化后指向同一目标。
    let mut by_target: std::collections::BTreeMap<&str, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (idx, cand) in candidates.iter().enumerate() {
        if items[idx].status == PlanItemStatus::Ready && !cand.target_normalized.is_empty() {
            by_target
                .entry(cand.target_normalized.as_str())
                .or_default()
                .push(idx);
        }
    }
    for (_, group) in by_target {
        if group.len() > 1 {
            for idx in group {
                let item = &mut items[idx];
                if item.status == PlanItemStatus::Ready {
                    item.status = PlanItemStatus::Conflict;
                    item.collision = CollisionKind::InternalTarget;
                    item.errors.push(WeaveError::conflict(
                        "rename.internalCollision",
                        "multiple items target the same path",
                    ));
                }
            }
        }
    }
}
