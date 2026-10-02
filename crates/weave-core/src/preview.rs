//! Preview 契约（M0 §21，charter #11）。
//!
//! M0 不实现任何具体工具的预览，但先把模型立住：
//! M2 的 Rename / Organizer 直接复用，不需要重新设计。

use crate::id::{OperationId, ToolId};
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
        ] {
            assert!(json.get(key).is_some(), "missing field {key}");
        }
    }
}
