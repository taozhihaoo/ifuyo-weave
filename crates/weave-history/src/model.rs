//! 历史模型（M2 §46–§47 / §52）。

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use weave_core::prelude::{OperationId, OperationKind};

/// 历史 schema 版本（M2 §54 versioned schema）。
pub const HISTORY_SCHEMA_VERSION: u32 = 1;

/// 事务整体状态（M2 §82/§83：执行前先落盘 InProgress，崩溃后可识别）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationStatus {
    /// 事务已写入、变更尚未开始；崩溃遗留时进入恢复路径。
    InProgress,
    /// 执行结束（无论部分成功/失败/取消——细节看 entry.counts）。
    Completed,
}

/// 可逆性（charter #12 / M2 §46）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Reversibility {
    /// 所有已执行项均可安全反向。
    Full,
    /// 部分项可逆（其余失败/跳过/NoOp）。
    Partial,
    /// 实际未发生任何变更（无可撤销内容，如实记录）。
    None,
}

/// 事务内单条目的执行事实（M2 §47）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionItem {
    pub item_id: String,
    pub source_path: String,
    pub target_path: String,
    /// 该条目是否真的被文件系统执行过。
    pub status: TransactionItemStatus,
    pub timestamp: Option<SystemTime>,
    pub original_size: Option<u64>,
    pub original_modified: Option<SystemTime>,
    pub original_created: Option<SystemTime>,
}

/// 条目执行事实——History 描述实际 mutation（M2 §79）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransactionItemStatus {
    /// 已真实执行（可尝试 Undo）。
    Executed,
    /// 未执行（失败/跳过/取消前未开始）——Undo 不得为其制造假恢复。
    NotExecuted,
    /// NoOp（source == target）：无变更，Undo 时跳过。
    NoOp,
}

/// 完整事务：一次操作的真实变更记录（M2 §46）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationTransaction {
    pub operation_id: OperationId,
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub timestamp: SystemTime,
    pub reversible: Reversibility,
    pub items: Vec<TransactionItem>,
}

/// 历史列表条目（M2 §52 的清单字段 + 摘要）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub operation_id: OperationId,
    pub kind: OperationKind,
    pub timestamp: SystemTime,
    pub summary: String,
    pub item_count: u64,
    pub success_count: u64,
    pub failed_count: u64,
    pub skipped_count: u64,
    /// 仅当存在已执行项且事务完整落盘时为 true。
    pub undoable: bool,
    pub status: OperationStatus,
    pub input_root: Option<String>,
    pub rule_summary: Option<String>,
}

/// entries.json 的文件形状（versioned schema，M2 §54）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFile {
    pub schema_version: u32,
    pub entries: Vec<HistoryEntry>,
}
