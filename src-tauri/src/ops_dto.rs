//! Rename / Organizer 的 IPC DTO 层（M2 §90–§91）。
//!
//! Plan 是 Preview 与 Execute 的同一契约；Execute 前服务端 Revalidate（M2 §38）。
//! 数值 f64（D4）、时间 epoch-ms，与 M1 DTO 约定一致。

// IPC 边界 Err DTO 体积豁免（同 D10）；ops_dto 的 into_domain 校验路径同样
// 不构成热路径。
#![expect(clippy::result_large_err)]

use serde::{Deserialize, Serialize};
use specta::Type;
use weave_core::prelude::WeaveError;
use weave_history::{
    HistoryEntry, OperationStatus, OperationTransaction, Reversibility, TransactionItemStatus,
};

use crate::commands::IpcError;

// ─── Rename 规则（M2 §13–§18 的九类，tagged enum）───

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RenameRuleDto {
    Prefix {
        text: String,
    },
    Suffix {
        text: String,
    },
    Replace {
        find: String,
        replace_with: String,
    },
    RegexReplace {
        pattern: String,
        replacement: String,
    },
    Counter {
        start: f64,
        step: f64,
        width: f64,
    },
    Date {
        field: String,
        format: String,
    },
    Case {
        form: String,
    },
    Extension {
        new_extension: String,
    },
    Template {
        template: String,
    },
}

impl RenameRuleDto {
    pub fn into_domain(self) -> Result<weave_files::RenameRule, IpcError> {
        fn num(v: f64, what: &str) -> Result<u64, IpcError> {
            if v < 0.0 || v != v.trunc() || v > u64::MAX as f64 {
                return Err(WeaveError::validation(
                    "rename.invalidOption",
                    format!("option '{what}' = {v} is out of range"),
                )
                .into());
            }
            Ok(v as u64)
        }
        Ok(match self {
            RenameRuleDto::Prefix { text } => weave_files::RenameRule::Prefix { text },
            RenameRuleDto::Suffix { text } => weave_files::RenameRule::Suffix { text },
            RenameRuleDto::Replace { find, replace_with } => {
                weave_files::RenameRule::Replace { find, replace_with }
            }
            RenameRuleDto::RegexReplace {
                pattern,
                replacement,
            } => weave_files::RenameRule::RegexReplace {
                pattern,
                replacement,
            },
            RenameRuleDto::Counter { start, step, width } => weave_files::RenameRule::Counter {
                start: num(start, "start")?,
                step: num(step, "step")?,
                width: num(width, "width")? as u32,
            },
            RenameRuleDto::Date { field, format } => weave_files::RenameRule::Date {
                field: match field.as_str() {
                    "created" => weave_files::DateField::Created,
                    "accessed" => weave_files::DateField::Accessed,
                    _ => weave_files::DateField::Modified,
                },
                format: match format.as_str() {
                    "YYYYMMDD" => weave_files::DateFormat::YmdCompact,
                    "YYYY-MM" => weave_files::DateFormat::YmDash,
                    _ => weave_files::DateFormat::YmdDash,
                },
            },
            RenameRuleDto::Case { form } => weave_files::RenameRule::Case {
                form: match form.as_str() {
                    "lower" => weave_files::CaseForm::Lower,
                    "upper" => weave_files::CaseForm::Upper,
                    _ => weave_files::CaseForm::Title,
                },
            },
            RenameRuleDto::Extension { new_extension } => {
                weave_files::RenameRule::Extension { new_extension }
            }
            RenameRuleDto::Template { template } => weave_files::RenameRule::Template { template },
        })
    }
}

// ─── Organizer 规则（M2 §27–§28）───

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OrganizerConditionDto {
    Any,
    ExtensionIn { extensions: Vec<String> },
    NameContains { text: String },
    NamePattern { pattern: String },
    SizeLargerThan { bytes: f64 },
    SizeSmallerThan { bytes: f64 },
    ModifiedBefore { epoch_ms: f64 },
    ModifiedAfter { epoch_ms: f64 },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OrganizerRuleDto {
    pub condition: OrganizerConditionDto,
    pub target_folder: String,
}

impl OrganizerRuleDto {
    pub fn into_domain(self) -> Result<weave_files::OrganizerRule, IpcError> {
        fn bounded_bytes(v: f64, what: &str) -> Result<u64, IpcError> {
            if v < 0.0 || v != v.trunc() {
                return Err(WeaveError::validation(
                    "rename.invalidOption",
                    format!("option '{what}' = {v} is out of range"),
                )
                .into());
            }
            Ok(v as u64)
        }
        let condition = match self.condition {
            OrganizerConditionDto::Any => weave_files::OrganizerCondition::Any,
            OrganizerConditionDto::ExtensionIn { extensions } => {
                weave_files::OrganizerCondition::ExtensionIn { extensions }
            }
            OrganizerConditionDto::NameContains { text } => {
                weave_files::OrganizerCondition::NameContains { text }
            }
            OrganizerConditionDto::NamePattern { pattern } => {
                weave_files::OrganizerCondition::NamePattern { pattern }
            }
            OrganizerConditionDto::SizeLargerThan { bytes } => {
                weave_files::OrganizerCondition::SizeLargerThan {
                    bytes: bounded_bytes(bytes, "bytes")?,
                }
            }
            OrganizerConditionDto::SizeSmallerThan { bytes } => {
                weave_files::OrganizerCondition::SizeSmallerThan {
                    bytes: bounded_bytes(bytes, "bytes")?,
                }
            }
            OrganizerConditionDto::ModifiedBefore { epoch_ms } => {
                weave_files::OrganizerCondition::ModifiedBefore {
                    epoch_ms: bounded_bytes(epoch_ms, "epochMs")?,
                }
            }
            OrganizerConditionDto::ModifiedAfter { epoch_ms } => {
                weave_files::OrganizerCondition::ModifiedAfter {
                    epoch_ms: bounded_bytes(epoch_ms, "epochMs")?,
                }
            }
        };
        Ok(weave_files::OrganizerRule {
            condition,
            target_folder: self.target_folder,
        })
    }
}

// ─── Plan / Preview（M2 §91：plan + items + validation summary）───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanItemDto {
    pub item_id: String,
    pub source_path: String,
    pub target_path: String,
    pub status: String,
    pub collision: String,
    pub warnings: Vec<String>,
    pub errors: Vec<IpcError>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    pub operation_id: String,
    pub kind: String,
    pub items: Vec<PlanItemDto>,
    pub ready_count: f64,
    pub conflict_count: f64,
    pub invalid_count: f64,
    pub noop_count: f64,
}

impl PlanDto {
    pub fn from_plan(plan: &weave_core::prelude::Plan) -> Self {
        let (ready, conflict, invalid, noop) = plan.status_counts();
        let status_str = |s: weave_core::prelude::PlanItemStatus| match s {
            weave_core::prelude::PlanItemStatus::Ready => "ready",
            weave_core::prelude::PlanItemStatus::Conflict => "conflict",
            weave_core::prelude::PlanItemStatus::Invalid => "invalid",
            weave_core::prelude::PlanItemStatus::NoOp => "noop",
        };
        let collision_str = |c: weave_core::prelude::CollisionKind| match c {
            weave_core::prelude::CollisionKind::None => "none",
            weave_core::prelude::CollisionKind::ExistingTarget => "existingTarget",
            weave_core::prelude::CollisionKind::InternalTarget => "internalTarget",
            weave_core::prelude::CollisionKind::CaseOnly => "caseOnly",
            weave_core::prelude::CollisionKind::Cycle => "cycle",
        };
        Self {
            operation_id: plan.operation_id.to_string(),
            kind: plan.kind.as_str().to_string(),
            items: plan
                .items
                .iter()
                .map(|i| PlanItemDto {
                    item_id: i.item_id.clone(),
                    source_path: i.source_path.clone(),
                    target_path: i.target_path.clone(),
                    status: status_str(i.status).to_string(),
                    collision: collision_str(i.collision).to_string(),
                    warnings: i.warnings.clone(),
                    errors: i.errors.iter().cloned().map(Into::into).collect(),
                })
                .collect(),
            ready_count: ready as f64,
            conflict_count: conflict as f64,
            invalid_count: invalid as f64,
            noop_count: noop as f64,
        }
    }
}

// ─── Execution / History / Transaction ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReportDto {
    pub job_id: String,
    pub operation_id: String,
    pub status: String,
    pub duration_ms: f64,
    pub executed: f64,
    pub failed: f64,
    pub skipped: f64,
    pub undoable: bool,
    pub items: Vec<PlanItemDto>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryDto {
    pub operation_id: String,
    pub kind: String,
    pub timestamp_ms: Option<f64>,
    pub summary: String,
    pub item_count: f64,
    pub success_count: f64,
    pub failed_count: f64,
    pub skipped_count: f64,
    pub undoable: bool,
    pub status: String,
}

impl HistoryEntryDto {
    pub fn from_entry(e: &HistoryEntry) -> Self {
        Self {
            operation_id: e.operation_id.to_string(),
            kind: e.kind.as_str().to_string(),
            timestamp_ms: e
                .timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_millis() as f64),
            summary: e.summary.clone(),
            item_count: e.item_count as f64,
            success_count: e.success_count as f64,
            failed_count: e.failed_count as f64,
            skipped_count: e.skipped_count as f64,
            undoable: e.undoable,
            status: history_status_str(e.status),
        }
    }
}

pub fn history_status_str(status: OperationStatus) -> String {
    match status {
        OperationStatus::InProgress => "inProgress".to_string(),
        OperationStatus::Completed => "completed".to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionItemDto {
    pub item_id: String,
    pub source_path: String,
    pub target_path: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    pub operation_id: String,
    pub kind: String,
    pub reversible: String,
    pub items: Vec<TransactionItemDto>,
}

impl TransactionDto {
    pub fn from_transaction(tx: &OperationTransaction) -> Self {
        let status_str = |s: TransactionItemStatus| match s {
            TransactionItemStatus::Executed => "executed",
            TransactionItemStatus::NotExecuted => "notExecuted",
            TransactionItemStatus::NoOp => "noop",
        };
        let reversible_str = |r: Reversibility| match r {
            Reversibility::Full => "full",
            Reversibility::Partial => "partial",
            Reversibility::None => "none",
        };
        Self {
            operation_id: tx.operation_id.to_string(),
            kind: tx.kind.as_str().to_string(),
            reversible: reversible_str(tx.reversible).to_string(),
            items: tx
                .items
                .iter()
                .map(|i| TransactionItemDto {
                    item_id: i.item_id.clone(),
                    source_path: i.source_path.clone(),
                    target_path: i.target_path.clone(),
                    status: status_str(i.status).to_string(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_rule_dto_maps_counter_bounds() {
        let dto = RenameRuleDto::Counter {
            start: 1.0,
            step: 1.0,
            width: 3.0,
        };
        let domain = dto.into_domain().expect("valid");
        match domain {
            weave_files::RenameRule::Counter { start, step, width } => {
                assert_eq!((start, step, width), (1, 1, 3));
            }
            other => panic!("wrong variant: {other:?}"),
        }

        let bad = RenameRuleDto::Counter {
            start: 1.5,
            step: 1.0,
            width: 3.0,
        };
        assert!(bad.into_domain().is_err());
    }
}

/// 任务终态载荷：Plan 执行报告（或 Undo 计数）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanReportDto {
    pub operation_id: String,
    pub items: Vec<PlanItemDto>,
    pub executed: f64,
    pub failed: f64,
    pub skipped: f64,
    pub undoable: bool,
    pub duration_ms: f64,
    pub transaction: Option<TransactionDto>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UndoReportDto {
    pub restored: f64,
    pub conflicts: f64,
    pub leaked_temps: f64,
}

impl ExecutionReportDto {
    /// 从执行报告构造（undoable = 事务已成功落盘）。
    pub fn from_report(report: &weave_files::ExecutionReport, undoable: bool) -> Self {
        Self {
            job_id: String::new(),
            operation_id: report.plan.operation_id.to_string(),
            status: "completed".to_string(),
            duration_ms: report.duration_ms as f64,
            executed: report.executed as f64,
            failed: report.failed as f64,
            skipped: report.skipped as f64,
            undoable,
            items: report
                .plan
                .items
                .iter()
                .map(|i| PlanItemDto {
                    item_id: i.item_id.clone(),
                    source_path: i.source_path.clone(),
                    target_path: i.target_path.clone(),
                    status: match i.status {
                        weave_core::prelude::PlanItemStatus::Ready => "ready",
                        weave_core::prelude::PlanItemStatus::Conflict => "conflict",
                        weave_core::prelude::PlanItemStatus::Invalid => "invalid",
                        weave_core::prelude::PlanItemStatus::NoOp => "noop",
                    }
                    .to_string(),
                    collision: "none".to_string(),
                    warnings: i.warnings.clone(),
                    errors: i.errors.iter().cloned().map(Into::into).collect(),
                })
                .collect(),
        }
    }
}

// ─── M3：Duplicates 选择与回收 ───

/// 组内回收选择（M3 §52–§56）。
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecycleSelectionDto {
    pub group_id: String,
    pub recycle_paths: Vec<String>,
}

impl RecycleSelectionDto {
    pub fn into_domain(self) -> weave_files::RecycleSelection {
        weave_files::RecycleSelection {
            group_id: self.group_id,
            recycle_paths: self.recycle_paths,
        }
    }
}
