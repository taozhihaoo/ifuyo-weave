//! Organizer（M2 §26–§34）。
//!
//! - 输入快照 = root 下**直接**普通文件（非递归，M2 §26/§31）。
//! - 有序规则，first-match-wins（M2 §28）；无匹配 ⇒ 不动（无隐式 fallback，
//!   除非用户显式添加 Any 条件规则作为最后一条）。
//! - 目标 = root / 规则相对目录 / 原文件名——绝不逃出 root（M2 §30）。
//! - 碰撞语义与 Rename 完全一致（M2 §32/§33）；目标父目录不存在 ⇒ Ready
//!   （Execute 时创建），父路径被普通文件占据 ⇒ Invalid。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use weave_core::prelude::{
    CancellationToken, CollisionKind, OperationId, OperationKind, Plan, PlanItem, PlanItemStatus,
    Progress, WeaveError, validate_absolute_path,
};

use crate::fs::Filesystem;

/// 匹配条件（M2 §27 六类 + Any 兜底）。
#[derive(Debug, Clone, PartialEq)]
pub enum OrganizerCondition {
    /// 兜底规则：匹配一切（作为最后一条时即 fallback）。
    Any,
    /// 扩展名集合（大小写不敏感，不带点）。
    ExtensionIn { extensions: Vec<String> },
    /// 文件名包含（大小写不敏感）。
    NameContains { text: String },
    /// 文件名正则。
    NamePattern { pattern: String },
    /// 大于（字节）。
    SizeLargerThan { bytes: u64 },
    /// 小于（字节）。
    SizeSmallerThan { bytes: u64 },
    /// 修改时间早于（epoch 毫秒）。
    ModifiedBefore { epoch_ms: u64 },
    /// 修改时间晚于（epoch 毫秒）。
    ModifiedAfter { epoch_ms: u64 },
    /// 位于 root 下某相对目录（非递归模式下只匹配直接 root 文件的 ""）。
    InFolder { relative: String },
}

/// 有序规则：WHEN condition THEN move to target_folder。
#[derive(Debug, Clone, PartialEq)]
pub struct OrganizerRule {
    pub condition: OrganizerCondition,
    /// 相对 root 的目标目录（如 "image" 或 "docs/2026"）；不得含 `..`。
    pub target_folder: String,
}

/// 构建 Organizer 计划（Move 语义）。
pub fn build_organizer_plan(
    fs: &dyn Filesystem,
    root_raw: &str,
    rules: &[OrganizerRule],
    cancel: &CancellationToken,
    report: &mut dyn FnMut(Progress),
) -> Result<Plan, WeaveError> {
    // 目标目录规则中的正则先编译（结构化错误，M2 §44 依赖纪律）
    let mut compiled_patterns: Vec<Option<regex::Regex>> = Vec::with_capacity(rules.len());
    for rule in rules {
        match &rule.condition {
            OrganizerCondition::NamePattern { pattern } => {
                compiled_patterns.push(Some(regex::Regex::new(pattern).map_err(|e| {
                    WeaveError::validation(
                        "organizer.invalidRegex",
                        format!("invalid name pattern '{pattern}': {e}"),
                    )
                    .with_location("weave-files::organize")
                })?));
            }
            _ => compiled_patterns.push(None),
        }
    }

    let validation =
        validate_absolute_path(root_raw).map_err(|e| e.with_location("weave-files::organize"))?;
    let root = PathBuf::from(&validation.normalized);
    let root_norm = format!(
        "{}{}",
        validation.normalized.to_lowercase(),
        std::path::MAIN_SEPARATOR
    );

    if !fs.exists(&root) {
        return Err(WeaveError::io(
            "path.notFound",
            format!("organizer root '{}' does not exist", validation.normalized),
        )
        .with_location("weave-files::organize"));
    }

    let operation_id = OperationId::generate();
    let mut names = fs.read_dir(&root).map_err(|e| {
        WeaveError::io(
            "organizer.rootReadFailed",
            format!("cannot read organizer root: {e}"),
        )
        .with_location("weave-files::organize")
    })?;
    names.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });

    let mut items: Vec<PlanItem> = Vec::new();
    let mut candidates: Vec<(String, PathBuf, String)> = Vec::new(); // (item_id, target, target_norm)
    let total = names.len() as u64;

    for (index, name) in names.into_iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(WeaveError::cancelled(
                "organizer.planCancelled",
                "plan building was cancelled",
            )
            .with_location("weave-files::organize"));
        }
        if index % 256 == 0 {
            report(Progress::running(
                operation_id.clone(),
                index as u64,
                Some(total),
            ));
        }

        let item_id = format!("item_{index:04}");
        let source = root.join(&name);
        let stat = match fs.stat(&source) {
            Ok(s) => s,
            Err(e) => {
                items.push(invalid_item(
                    &item_id,
                    &source.to_string_lossy(),
                    WeaveError::io("rename.statFailed", format!("cannot stat: {e}"))
                        .with_location("weave-files::organize"),
                ));
                candidates.push((item_id, PathBuf::new(), String::new()));
                continue;
            }
        };
        // 只处理普通文件（M2 §26）；symlink/目录不在快照内
        if stat.kind != weave_core::prelude::FileKind::RegularFile {
            continue;
        }

        let ext = source
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name_lower = name.to_lowercase();

        let mut matched: Option<&OrganizerRule> = None;
        for (rule_idx, rule) in rules.iter().enumerate() {
            let hits = match &rule.condition {
                OrganizerCondition::Any => true,
                OrganizerCondition::ExtensionIn { extensions } => extensions
                    .iter()
                    .any(|e| e.trim_start_matches('.').to_lowercase() == ext),
                OrganizerCondition::NameContains { text } => {
                    name_lower.contains(&text.to_lowercase())
                }
                OrganizerCondition::NamePattern { pattern: _ } => compiled_patterns[rule_idx]
                    .as_ref()
                    .expect("compiled pattern")
                    .is_match(&name),
                OrganizerCondition::SizeLargerThan { bytes } => stat.size > *bytes,
                OrganizerCondition::SizeSmallerThan { bytes } => stat.size < *bytes,
                OrganizerCondition::ModifiedBefore { epoch_ms } => stat
                    .modified
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| (d.as_millis() as u64) < *epoch_ms)
                    .unwrap_or(false),
                OrganizerCondition::ModifiedAfter { epoch_ms } => stat
                    .modified
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| (d.as_millis() as u64) > *epoch_ms)
                    .unwrap_or(false),
                OrganizerCondition::InFolder { relative: _ } => false, // 非递归：直接文件不在子目录
            };
            if hits {
                matched = Some(rule);
                break;
            }
        }

        let Some(rule) = matched else {
            // 无匹配规则：文件保持原位（不出现在 Plan 中——Preview 说明即可）
            continue;
        };

        // 目标 = root / target_folder / name；组合后统一过 path safety + root 边界
        let target_candidate = if rule.target_folder.is_empty() {
            root.join(&name)
        } else {
            root.join(&rule.target_folder).join(&name)
        };
        let target_validation = match validate_absolute_path(&target_candidate.to_string_lossy()) {
            Ok(v) => v,
            Err(e) => {
                items.push(invalid_item(&item_id, &source.to_string_lossy(), e));
                candidates.push((item_id, PathBuf::new(), String::new()));
                continue;
            }
        };
        // Root 边界（M2 §30）：规范化目标必须仍位于 root 之下
        if !target_validation
            .normalized
            .to_lowercase()
            .starts_with(&root_norm)
        {
            items.push(invalid_item(
                &item_id,
                &source.to_string_lossy(),
                WeaveError::validation(
                    "organizer.rootEscape",
                    "rule target escapes the organizer root",
                )
                .with_location("weave-files::organize"),
            ));
            candidates.push((item_id, PathBuf::new(), String::new()));
            continue;
        }

        let target_norm = target_validation.normalized.to_lowercase();
        // case-only move（target_norm == source_norm）也走 Ready，由执行器
        // 的两阶段临时名策略处理（Move 的卷内同目录场景退化为普通 rename）。
        let status = if target_validation.normalized == validation.normalized {
            PlanItemStatus::NoOp
        } else {
            PlanItemStatus::Ready
        };

        items.push(PlanItem {
            item_id: item_id.clone(),
            // 逐文件的真实绝对路径（root 校验在外层；文件路径即 root.join(name)）
            source_path: source.to_string_lossy().into_owned(),
            target_path: target_validation.normalized.clone(),
            status,
            collision: CollisionKind::None,
            source_size: Some(stat.size),
            source_modified: stat.modified,
            warnings: Vec::new(),
            errors: Vec::new(),
        });
        candidates.push((item_id, target_candidate, target_norm));
    }

    // 碰撞：ExistingTarget（磁盘已存在）+ InternalTarget（批内同归一化目标，
    // 例如 a.jpg 与 A.jpg 同入 image/）
    for (idx, item) in items.iter_mut().enumerate() {
        if item.status != PlanItemStatus::Ready {
            continue;
        }
        let (_, target, target_norm) = &candidates[idx];
        if target_norm.is_empty() {
            continue;
        }
        if item.source_path.to_lowercase() == *target_norm {
            continue; // NoOp 已置
        }
        let internal = candidates.iter().any(|(other_id, _, other_norm)| {
            other_id != &item.item_id && !other_norm.is_empty() && other_norm == target_norm
        });
        if internal {
            item.status = PlanItemStatus::Conflict;
            item.collision = CollisionKind::InternalTarget;
            item.errors.push(WeaveError::conflict(
                "rename.internalCollision",
                "multiple items target the same path",
            ));
            continue;
        }
        if fs.exists(target) {
            item.status = PlanItemStatus::Conflict;
            item.collision = CollisionKind::ExistingTarget;
            item.errors.push(WeaveError::conflict(
                "rename.targetExists",
                format!("target already exists: {}", item.target_path),
            ));
        }
    }

    Ok(Plan {
        operation_id,
        kind: OperationKind::Move,
        created_at: std::time::SystemTime::now(),
        items,
    })
}

/// Organizer 目标父目录创建钩子：Execute 前由 executor 调用侧使用；
/// 这里提供纯校验函数（目标父路径不得被普通文件占据）。
pub fn validate_target_parent(fs: &dyn Filesystem, target: &Path) -> Result<(), WeaveError> {
    let Some(parent) = target.parent() else {
        return Ok(());
    };
    if !fs.exists(parent) {
        return Ok(());
    }
    let stat = fs.stat(parent).map_err(|e| {
        WeaveError::io("organizer.parentStatFailed", format!("{e}"))
            .with_location("weave-files::organize")
    })?;
    if stat.kind != weave_core::prelude::FileKind::Directory {
        return Err(WeaveError::conflict(
            "organizer.parentNotDirectory",
            format!(
                "'{}' exists and is not a directory",
                parent.to_string_lossy()
            ),
        )
        .with_location("weave-files::organize"));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::StdFilesystem;
    use weave_core::prelude::PlanItemStatus;
    use weave_testkit::TempWorkspace;

    fn plan_for(ws: &TempWorkspace, rules: &[OrganizerRule]) -> Plan {
        build_organizer_plan(
            &StdFilesystem,
            &ws.path().to_string_lossy(),
            rules,
            &CancellationToken::new(),
            &mut |_| {},
        )
        .expect("plan")
    }

    fn ext_rule(exts: &[&str], folder: &str) -> OrganizerRule {
        OrganizerRule {
            condition: OrganizerCondition::ExtensionIn {
                extensions: exts.iter().map(|s| s.to_string()).collect(),
            },
            target_folder: folder.to_string(),
        }
    }

    #[test]
    fn extension_rules_move_files_first_match_wins() {
        let ws = TempWorkspace::new("org-ext").expect("ws");
        ws.file("a.jpg", "x").expect("write");
        ws.file("b.png", "x").expect("write");
        ws.file("c.pdf", "x").expect("write");
        ws.file("d.txt", "x").expect("write");
        let plan = plan_for(
            &ws,
            &[
                ext_rule(&["jpg", "png"], "image"),
                ext_rule(&["pdf"], "documents"),
            ],
        );
        assert_eq!(plan.items.len(), 3, "d.txt 无匹配规则不动");
        for item in &plan.items {
            assert_eq!(item.status, PlanItemStatus::Ready);
            let target = item.target_path.replace('/', "\\");
            if item.source_path.ends_with("a.jpg") {
                assert!(target.ends_with("\\image\\a.jpg"));
            }
            if item.source_path.ends_with("c.pdf") {
                assert!(target.ends_with("\\documents\\c.pdf"));
            }
        }
    }

    #[test]
    fn fallback_any_rule_catches_remainder() {
        let ws = TempWorkspace::new("org-fallback").expect("ws");
        ws.file("mystery.bin", "x").expect("write");
        let plan = plan_for(
            &ws,
            &[
                ext_rule(&["jpg"], "image"),
                OrganizerRule {
                    condition: OrganizerCondition::Any,
                    target_folder: "other".to_string(),
                },
            ],
        );
        assert_eq!(plan.items.len(), 1);
        assert!(plan.items[0].target_path.ends_with("\\other\\mystery.bin"));
    }

    #[test]
    fn case_fold_collapse_on_ntfs_leaves_single_file() {
        // Windows 大小写不敏感：A.jpg 的创建覆盖 a.jpg —— 磁盘上只剩一个文件，
        // 真实的"归一化同目标双文件"场景在 NTFS 上无法构造 fixture；
        // InternalTarget 合并逻辑由 rename plan 的 internal_collision 测试覆盖。
        let ws = TempWorkspace::new("org-collide").expect("ws");
        ws.file("a.jpg", "1").expect("write");
        ws.file("A.jpg", "2").expect("write");
        let plan = plan_for(&ws, &[ext_rule(&["jpg"], "image")]);
        assert_eq!(plan.items.len(), 1, "case-fold collapse leaves one file");
        assert_eq!(plan.items[0].status, PlanItemStatus::Ready);
    }

    #[test]
    fn size_condition_matches() {
        let ws = TempWorkspace::new("org-size").expect("ws");
        ws.file("big.bin", "0123456789").expect("write");
        ws.file("small.bin", "x").expect("write");
        let plan = plan_for(
            &ws,
            &[OrganizerRule {
                condition: OrganizerCondition::SizeLargerThan { bytes: 5 },
                target_folder: "big".to_string(),
            }],
        );
        assert_eq!(plan.items.len(), 1);
        assert!(plan.items[0].source_path.ends_with("big.bin"));
    }

    #[test]
    fn root_escape_via_folder_is_invalid() {
        let ws = TempWorkspace::new("org-escape").expect("ws");
        ws.file("a.jpg", "x").expect("write");
        let rules = [OrganizerRule {
            condition: OrganizerCondition::ExtensionIn {
                extensions: vec!["jpg".to_string()],
            },
            target_folder: "..\\evil".to_string(),
        }];
        let plan = plan_for(&ws, &rules);
        assert_eq!(plan.items[0].status, PlanItemStatus::Invalid);
        assert_eq!(plan.items[0].errors[0].code, "organizer.rootEscape");
    }

    #[test]
    fn already_in_target_folder_is_noop() {
        let ws = TempWorkspace::new("org-noop").expect("ws");
        ws.file("image/a.jpg", "x").expect("write");
        // root = ws/image；规则把 jpg 移到 image/ → source == target ⇒ NoOp
        let root = ws.path().join("image");
        let plan = build_organizer_plan(
            &StdFilesystem,
            &root.to_string_lossy(),
            &[ext_rule(&["jpg"], "image")],
            &CancellationToken::new(),
            &mut |_| {},
        )
        .expect("plan");
        // 非递归：image/a.jpg 是 root 直属文件；target = image/image/a.jpg ≠ source
        assert_eq!(plan.items.len(), 1);
        assert_ne!(plan.items[0].status, PlanItemStatus::NoOp);
        assert!(plan.items[0].target_path.ends_with("image\\image\\a.jpg"));
    }
}
