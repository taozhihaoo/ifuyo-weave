//! Preview 契约（M0 §21，charter #11）。
//!
//! M0 不实现任何具体工具的预览，但先把模型立住：
//! M2 的 Rename / Organizer 直接复用，不需要重新设计。

use crate::id::{OperationId, ToolId};
use crate::operation::{OperationKind, ReversibilityLevel, RiskLevel};
use serde::{Deserialize, Serialize};

/// 单个受影响条目的 before → after 描述。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    pub source: String,
    pub target: Option<String>,
    pub summary: String,
}

/// 一次操作的预览。Preview 不产生任何副作用（charter #11）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub operation: OperationId,
    pub tool: ToolId,
    pub affected_items: Vec<PreviewItem>,
    pub warnings: Vec<String>,
    pub conflicts: Vec<String>,
    pub estimated_output: Option<String>,
    /// 是否可撤销；不可撤销的操作必须明确标记（charter #12）。
    pub reversible: bool,
    /// 可逆性契约等级（M2.0 §20：Preview 必须显示可恢复级别）。
    pub reversibility: ReversibilityLevel,
    /// 预计风险（M2.0 §20：Preview 必须显示预计风险）。
    pub risk: RiskLevel,
}

impl Preview {
    /// 依据操作种类推导契约字段（reversible 与等级一致；L3 ⇒ reversible=false）。
    pub fn contract_for(
        kind: OperationKind,
        reversible_override: Option<bool>,
    ) -> (bool, ReversibilityLevel, RiskLevel) {
        let level = kind.reversibility_level();
        let reversible = reversible_override.unwrap_or(!level.requires_explicit_confirmation());
        (reversible, level, level.default_risk())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_serializes_with_all_contract_fields() {
        let preview = Preview {
            operation: OperationId::generate(),
            tool: ToolId::parse("files.rename").expect("valid"),
            affected_items: vec![PreviewItem {
                source: "a.png".to_string(),
                target: Some("b.png".to_string()),
                summary: "rename".to_string(),
            }],
            warnings: vec![],
            conflicts: vec![],
            estimated_output: Some("1.2 MB".to_string()),
            reversible: true,
            reversibility: ReversibilityLevel::OperationLog,
            risk: RiskLevel::Low,
        };
        let json = serde_json::to_value(&preview).expect("serialize");
        for key in [
            "operation",
            "tool",
            "affectedItems",
            "warnings",
            "conflicts",
            "estimatedOutput",
            "reversible",
            "reversibility",
            "risk",
        ] {
            assert!(json.get(key).is_some(), "missing field {key}");
        }
    }

    #[test]
    fn contract_for_matches_reversibility_contract() {
        // L1：操作日志可反向 ⇒ 默认可撤销、低风险
        let (rev, level, risk) = Preview::contract_for(OperationKind::Rename, None);
        assert_eq!(level, ReversibilityLevel::OperationLog);
        assert_eq!((rev, risk), (true, RiskLevel::Low));
        // L2：需要恢复件 ⇒ 默认可撤销、中风险
        let (rev, level, risk) = Preview::contract_for(OperationKind::DuplicateRecycle, None);
        assert_eq!(level, ReversibilityLevel::RecoveryRequired);
        assert_eq!((rev, risk), (true, RiskLevel::Medium));
        // L3：显式确认门 ⇒ 必须要求确认、高风险
        assert!(ReversibilityLevel::Irreversible.requires_explicit_confirmation());
        assert_eq!(
            ReversibilityLevel::Irreversible.default_risk(),
            RiskLevel::High
        );
    }
}
