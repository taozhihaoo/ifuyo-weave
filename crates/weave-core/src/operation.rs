//! Operation / Plan 稳定语言（M2 §7–§11，charter #10）。
//!
//! Core 只定义跨工具的稳定词汇：kind、状态、Plan 结构。
//! Rename/Organizer 的业务实现属于 weave-files；持久化属于 weave-history。
//!
//! 铁律（M2 §10/§11）：Preview 与 Execute 使用**同一个 Plan**；Plan 是
//! 文件系统状态的事实快照，Execute 前重新验证（Revalidate）而非重新推导。

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// 操作种类（M2 §9）。M2 只实现 Rename / Move（Organizer = Move 的一种规划）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    Rename,
    Move,
    /// M3：重复文件回收（Move to Recycle Bin）。
    DuplicateRecycle,
    /// M4：文本工具安全写回（Formatter/Transformer Apply/Save/Overwrite）。
    TextTransform,
    /// M7：Batch Engine 统一执行（Linear Pipeline 批量产物）。
    BatchExecute,
}

impl OperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OperationKind::Rename => "rename",
            OperationKind::Move => "move",
            OperationKind::DuplicateRecycle => "duplicateRecycle",
            OperationKind::TextTransform => "textTransform",
            OperationKind::BatchExecute => "batchExecute",
        }
    }
}

/// Plan 阶段的条目状态（Preview 语义，M2 §32/§35/§62）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanItemStatus {
    /// 可以执行。
    Ready,
    /// 碰撞（目标已存在 / 批内目标重复），默认 No Overwrite 策略下不执行。
    Conflict,
    /// 规则/路径非法（名字非法、路径逃逸、正则错等）。
    Invalid,
    /// source 与 target 相同：无需变更，也不是错误。
    NoOp,
}

/// 碰撞种类（M2 §21，四类全覆盖 + None）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CollisionKind {
    None,
    /// 目标路径已存在于文件系统。
    ExistingTarget,
    /// 批内多个条目产生同一目标。
    InternalTarget,
    /// source 与 target 仅大小写不同（大小写不敏感文件系统需两阶段）。
    CaseOnly,
    /// target 与批内另一个条目的 source 规范化后相同 ⇒ rename 环，需要两阶段。
    Cycle,
}

/// 执行阶段的单条结果（M2 §42）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemOutcome {
    Success,
    Failed,
    Skipped,
    Cancelled,
    NoOp,
}

/// 计划中的一个条目（M2 §10）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    /// 批内稳定 ID（如 "item_0007"）；不使用 UI index（M2 §8）。
    pub item_id: String,
    pub source_path: String,
    pub target_path: String,
    pub status: PlanItemStatus,
    pub collision: CollisionKind,
    /// 计划时源快照——Execute 前的 Revalidate 依据（M2 §38；变化 ⇒ PreconditionFailed）。
    pub source_size: Option<u64>,
    pub source_modified: Option<SystemTime>,
    pub warnings: Vec<String>,
    pub errors: Vec<WeaveError>,
}

use crate::error::WeaveError;

/// 结构化执行计划（M2 §10）：Preview 与 Execute 共用的事实快照。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub operation_id: crate::id::OperationId,
    pub kind: OperationKind,
    pub created_at: std::time::SystemTime,
    pub items: Vec<PlanItem>,
}

impl Plan {
    /// 各状态条目计数（Preview 摘要用）。
    pub fn status_counts(&self) -> (u64, u64, u64, u64) {
        let (mut ready, mut conflict, mut invalid, mut noop) = (0, 0, 0, 0);
        for item in &self.items {
            match item.status {
                PlanItemStatus::Ready => ready += 1,
                PlanItemStatus::Conflict => conflict += 1,
                PlanItemStatus::Invalid => invalid += 1,
                PlanItemStatus::NoOp => noop += 1,
            }
        }
        (ready, conflict, invalid, noop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WeaveError;
    use crate::id::OperationId;

    #[test]
    fn enums_serialize_camel_case() {
        assert_eq!(
            serde_json::to_value(OperationKind::Rename).unwrap(),
            serde_json::json!("rename")
        );
        assert_eq!(
            serde_json::to_value(CollisionKind::ExistingTarget).unwrap(),
            serde_json::json!("existingTarget")
        );
        assert_eq!(
            serde_json::to_value(ItemOutcome::Cancelled).unwrap(),
            serde_json::json!("cancelled")
        );
    }

    #[test]
    fn plan_serializes_with_items_and_status_counts() {
        let plan = Plan {
            operation_id: OperationId::generate(),
            kind: OperationKind::Rename,
            created_at: std::time::SystemTime::UNIX_EPOCH,
            items: vec![
                PlanItem {
                    item_id: "item_0000".to_string(),
                    source_path: "C:\\a.jpg".to_string(),
                    target_path: "C:\\b.jpg".to_string(),
                    status: PlanItemStatus::Ready,
                    collision: CollisionKind::None,
                    source_size: Some(10),
                    source_modified: None,
                    warnings: vec![],
                    errors: vec![],
                },
                PlanItem {
                    item_id: "item_0001".to_string(),
                    source_path: "C:\\c.jpg".to_string(),
                    target_path: "C:\\b.jpg".to_string(),
                    status: PlanItemStatus::Conflict,
                    collision: CollisionKind::InternalTarget,
                    source_size: None,
                    source_modified: None,
                    warnings: vec![],
                    errors: vec![WeaveError::conflict("rename.internalCollision", "dup")],
                },
                PlanItem {
                    item_id: "item_0002".to_string(),
                    source_path: "C:\\d.jpg".to_string(),
                    target_path: "C:\\d.jpg".to_string(),
                    status: PlanItemStatus::NoOp,
                    collision: CollisionKind::None,
                    source_size: None,
                    source_modified: None,
                    warnings: vec![],
                    errors: vec![],
                },
            ],
        };
        assert_eq!(plan.status_counts(), (1, 1, 0, 1));
        let json = serde_json::to_value(&plan).unwrap();
        for key in ["operationId", "kind", "createdAt", "items"] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
        let back: Plan = serde_json::from_value(json).unwrap();
        assert_eq!(back, plan);
    }
}
