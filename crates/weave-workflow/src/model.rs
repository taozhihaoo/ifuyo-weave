//! Workflow 域模型（M10 上 §4-§6/§37-§44）：线性 Pipeline 定义的
//! 稳定、可版本化、可序列化表示。
//!
//! §41：UI 状态（选中/展开/滚动）绝不进入本模型；
//! §42：id 稳定，重命名不变；§38：schemaVersion 显式数值。

use serde::{Deserialize, Serialize};
#[cfg(feature = "specta")]
use specta::Type;

/// §38：当前 schema 版本。未来结构变化必须走迁移（§39）。
pub const WORKFLOW_SCHEMA_VERSION: u32 = 1;

/// §5 Workflow 核心模型。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Workflow {
    pub schema_version: u32,
    /// §42：稳定 ID（重命名不变）。
    pub id: String,
    /// §43：名称可改，不参与执行语义。
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// §6 v1 四类 step，线性顺序即 steps 数组顺序。
    pub steps: Vec<WorkflowStep>,
    /// §23 工作流参数（有界类型集 §23）。
    #[serde(default)]
    pub parameters: Vec<WorkflowParameter>,
}

/// §5/§124 WorkflowStep。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct WorkflowStep {
    pub id: String,
    pub r#type: StepType,
    #[serde(default)]
    pub name: String,
    /// §6/§24：tool id 或 filter/export 子类型标识。
    #[serde(default)]
    pub kind: String,
    /// §24 配置（serde_json::Value；校验由 Validation 按 kind schema 执行）。
    #[serde(default = "empty_object")]
    pub config: serde_json::Value,
    /// §55 enabled = 显式开关；disabled = 执行 bypass（§55/§56 非静默跳过）。
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(Type))]
pub enum StepType {
    Input,
    Filter,
    Tool,
    Export,
}

/// §23 工作流参数（类型有界：string/number/boolean/path）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct WorkflowParameter {
    pub id: String,
    pub label: String,
    pub param_type: WorkflowParamType,
    #[serde(default)]
    pub default_value: serde_json::Value,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(Type))]
pub enum WorkflowParamType {
    String,
    Number,
    Boolean,
    Path,
}

/// §75 工作流尺寸限额（保守初值，依据真实测试调整）。
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct WorkflowSizeLimits {
    pub max_steps: usize,
    /// 序列化 JSON 字节数上限。
    pub max_serialized_bytes: usize,
    pub max_parameters: usize,
}

impl Default for WorkflowSizeLimits {
    fn default() -> Self {
        Self {
            max_steps: 32,
            max_serialized_bytes: 256 * 1024,
            max_parameters: 16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_json_roundtrip() {
        let wf = Workflow {
            schema_version: WORKFLOW_SCHEMA_VERSION,
            id: "wf-1".into(),
            name: "Test".into(),
            description: String::new(),
            steps: vec![WorkflowStep {
                id: "input-1".into(),
                r#type: StepType::Input,
                name: "Input".into(),
                kind: "files".into(),
                config: serde_json::json!({}),
                enabled: true,
            }],
            parameters: vec![],
        };
        let json = serde_json::to_string(&wf).expect("serialize");
        let back: Workflow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, wf);
        // §38：schemaVersion 字段名稳定
        assert!(json.contains("schemaVersion"));
    }
}
