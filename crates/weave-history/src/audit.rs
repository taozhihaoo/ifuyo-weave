//! 审计事件派生（M2.0 §17）。
//!
//! Audit 描述**实际发生了什么**，不是意图。1.0 的事实源已经存在：
//! `OperationTransaction`（文件操作的实际变更事实）与 Batch Job Journal
//! （逐条完成事实）。本模块把它们**派生**为统一事件词汇，供报告、UI
//! 与后续扩展消费——不建立第二份持久化（§17/§18 镜像纪律，D63）。
//!
//! Before 事实来自事务的原始状态快照；After 路径事实 = target。
//! 未被记录的事实保持 None——派生层不猜测、不补造。

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::model::{OperationTransaction, TransactionItemStatus};
use weave_core::prelude::OperationKind;

/// 审计动作（M2.0 §17：Planned/Applied/Recovered/Failed/Skipped 词汇的
/// 事务落地面）。Planned 事件由计划方按需从 `core::Plan` 派生，不在此处。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditAction {
    /// 已真实执行（事务 Executed）。
    Applied,
    /// 未执行（失败/跳过/取消前未开始）——如实呈现，不与 Applied 混同。
    NotExecuted,
    /// source == target，无变更。
    NoOp,
    /// Undo 已把该条目还原。
    Recovered,
    /// Undo 无法还原（缺失/冲突）——恢复失败是事实，必须可审计。
    RecoveryNotPossible,
}

/// 单条审计事件（M2.0 §17 的统一形状）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub timestamp: Option<SystemTime>,
    pub operation_id: String,
    pub kind: OperationKind,
    pub item_id: String,
    pub action: AuditAction,
    pub source: String,
    /// After 路径事实（Executed 时为实际目标路径）。
    pub target: Option<String>,
    /// Before 快照：事务记录的原始大小。
    pub before_size: Option<u64>,
    /// Before 快照：事务记录的原始修改时间。
    pub before_modified: Option<SystemTime>,
}

/// 从事务派生审计事件（顺序 = 事务记录顺序 = 实际应用顺序）。
pub fn events_from_transaction(tx: &OperationTransaction) -> Vec<AuditEvent> {
    tx.items
        .iter()
        .map(|item| AuditEvent {
            timestamp: item.timestamp,
            operation_id: tx.operation_id.to_string(),
            kind: tx.kind,
            item_id: item.item_id.clone(),
            action: match item.status {
                TransactionItemStatus::Executed => AuditAction::Applied,
                TransactionItemStatus::NotExecuted => AuditAction::NotExecuted,
                TransactionItemStatus::NoOp => AuditAction::NoOp,
            },
            source: item.source_path.clone(),
            target: Some(item.target_path.clone()),
            before_size: item.original_size,
            before_modified: item.original_modified,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        OperationStatus, OperationTransaction, Reversibility, TransactionItem,
        TransactionItemStatus,
    };
    use std::time::Duration;
    use weave_core::prelude::OperationId;

    fn tx_with(statuses: &[TransactionItemStatus]) -> OperationTransaction {
        OperationTransaction {
            operation_id: OperationId::parse("op_aa00000001").expect("id"),
            kind: OperationKind::Rename,
            status: OperationStatus::Completed,
            timestamp: SystemTime::UNIX_EPOCH,
            reversible: Reversibility::Partial,
            items: statuses
                .iter()
                .enumerate()
                .map(|(i, status)| TransactionItem {
                    item_id: format!("item_{i:04}"),
                    source_path: format!("C:\\w\\f{i}.txt"),
                    target_path: format!("C:\\w\\g{i}.txt"),
                    status: *status,
                    timestamp: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(i as u64)),
                    original_size: Some(7),
                    original_modified: Some(SystemTime::UNIX_EPOCH),
                    original_created: None,
                })
                .collect(),
        }
    }

    #[test]
    fn events_map_statuses_to_audit_actions_with_before_facts() {
        let tx = tx_with(&[
            TransactionItemStatus::Executed,
            TransactionItemStatus::NotExecuted,
            TransactionItemStatus::NoOp,
        ]);
        let events = events_from_transaction(&tx);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].action, AuditAction::Applied);
        assert_eq!(events[1].action, AuditAction::NotExecuted);
        assert_eq!(events[2].action, AuditAction::NoOp);
        // Before 事实 + After 路径 + 顺序保持
        assert_eq!(events[0].before_size, Some(7));
        assert_eq!(events[0].target.as_deref(), Some("C:\\w\\g0.txt"));
        assert_eq!(events[0].operation_id, tx.operation_id.to_string());
        assert!(events[0].timestamp < events[2].timestamp);
        // 事件可序列化（消费方契约）
        let json = serde_json::to_string(&events[0]).expect("json");
        assert!(json.contains("\"action\":\"applied\""));
    }
}
