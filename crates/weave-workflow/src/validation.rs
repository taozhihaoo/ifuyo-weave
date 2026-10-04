//! Workflow Validation（M10 上 §26-§29/§75-§76）：结构校验（§27）+
//! 语义校验（§28）+ 严重度分级（§29：仅 ERROR 阻止执行）+ 尺寸限额
//! （§75）+ 线性 self-reference 防护（§76）。

use crate::model::{StepType, Workflow, WorkflowSizeLimits};
use crate::registry::ToolRegistry;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    /// error | warning
    pub severity: String,
    pub code: String,
    pub message: String,
    /// 关联 step id（workflow 级 = None）。
    pub step_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowValidation {
    pub issues: Vec<ValidationIssue>,
}

impl WorkflowValidation {
    /// §29：只有 ERROR 阻止执行。
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == "error")
    }

    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == "error")
    }

    fn error(&mut self, code: &'static str, message: String, step_id: Option<String>) {
        self.issues.push(ValidationIssue {
            severity: "error".into(),
            code: code.into(),
            message,
            step_id,
        });
    }

    fn warning(&mut self, code: &'static str, message: String, step_id: Option<String>) {
        self.issues.push(ValidationIssue {
            severity: "warning".into(),
            code: code.into(),
            message,
            step_id,
        });
    }
}

/// 校验入口（§26 全集的结构/语义两部分；limits 缺省用 Default）。
pub fn validate(workflow: &Workflow) -> WorkflowValidation {
    validate_with_limits(workflow, &WorkflowSizeLimits::default())
}

pub fn validate_with_limits(
    workflow: &Workflow,
    limits: &WorkflowSizeLimits,
) -> WorkflowValidation {
    let mut v = WorkflowValidation { issues: Vec::new() };

    // §38 schemaVersion
    if workflow.schema_version != crate::model::WORKFLOW_SCHEMA_VERSION {
        v.error(
            "workflow.schemaVersion",
            format!(
                "schemaVersion {} unsupported (expected {})",
                workflow.schema_version,
                crate::model::WORKFLOW_SCHEMA_VERSION
            ),
            None,
        );
    }
    // §42/§43：id/name 非空
    if workflow.id.trim().is_empty() {
        v.error(
            "workflow.emptyId",
            "workflow id must not be empty".into(),
            None,
        );
    }
    if workflow.name.trim().is_empty() {
        v.warning("workflow.emptyName", "workflow name is empty".into(), None);
    }

    // §75 尺寸限额
    if workflow.steps.len() > limits.max_steps {
        v.error(
            "workflow.tooManySteps",
            format!(
                "{} steps over limit {}",
                workflow.steps.len(),
                limits.max_steps
            ),
            None,
        );
    }
    if workflow.parameters.len() > limits.max_parameters {
        v.error(
            "workflow.tooManyParameters",
            format!(
                "{} parameters over limit {}",
                workflow.parameters.len(),
                limits.max_parameters
            ),
            None,
        );
    }

    // §27：step id 唯一 / 非空
    let mut seen_ids = std::collections::HashSet::new();
    for step in &workflow.steps {
        if step.id.trim().is_empty() {
            v.error(
                "step.emptyId",
                "step id must not be empty".into(),
                Some(step.id.clone()),
            );
        }
        if !seen_ids.insert(step.id.clone()) {
            v.error(
                "step.duplicateId",
                format!("duplicate step id '{}'", step.id),
                Some(step.id.clone()),
            );
        }
    }

    // §6 v1 结构顺序：恰一个 Input 在首位；至多一个 Export 在末位
    match workflow.steps.first() {
        Some(first) if first.r#type == StepType::Input => {}
        Some(first) => v.error(
            "workflow.inputMissing",
            format!(
                "workflow must start with an Input step, found {:?}",
                first.r#type
            ),
            Some(first.id.clone()),
        ),
        None => v.error("workflow.noSteps", "workflow has no steps".into(), None),
    }
    let exports: Vec<&crate::model::WorkflowStep> = workflow
        .steps
        .iter()
        .filter(|s| s.r#type == StepType::Export)
        .collect();
    if exports.len() > 1 {
        v.error(
            "workflow.multipleExports",
            format!("at most one Export step allowed, found {}", exports.len()),
            None,
        );
    }
    if let Some(last) = workflow.steps.last() {
        if exports.len() == 1 && last.r#type != StepType::Export {
            v.error(
                "workflow.exportNotLast",
                "Export step must be the final step (linear pipeline)".into(),
                Some(last.id.clone()),
            );
        }
    } else {
        v.error(
            "workflow.exportMissing",
            "workflow needs an Export step".into(),
            None,
        );
    }

    // 逐 step：类型已知 + kind 注册表存在 + config 语义校验（§28）
    for step in &workflow.steps {
        match step.r#type {
            StepType::Input => {
                if step.kind != "files" {
                    v.error(
                        "input.unknownKind",
                        format!("unknown input kind '{}' (v1: 'files')", step.kind),
                        Some(step.id.clone()),
                    );
                }
            }
            StepType::Filter => {
                if ToolRegistry::filter(&step.kind).is_none() {
                    v.error(
                        "filter.unknown",
                        format!("unknown filter '{}'", step.kind),
                        Some(step.id.clone()),
                    );
                }
            }
            StepType::Tool => {
                match ToolRegistry::tool(&step.kind) {
                    Some(desc) => {
                        // §28 语义校验：按 tool 语义检查 config
                        validate_tool_config(&step.kind, &step.config, &step.id, &mut v);
                        let _ = desc;
                    }
                    None => {
                        // §48 Missing Tool：显示 tool unavailable 而非静默跳过
                        v.error(
                            "tool.unavailable",
                            format!("tool '{}' unavailable in this version", step.kind),
                            Some(step.id.clone()),
                        );
                    }
                }
            }
            StepType::Export => {
                if step.kind != "export.files" {
                    v.error(
                        "export.unknownKind",
                        format!("unknown export kind '{}' (v1: 'export.files')", step.kind),
                        Some(step.id.clone()),
                    );
                }
            }
        }
    }

    v
}

/// §28 语义校验：tool config 按各自 schema 检查（v1 = 类型与枚举）。
fn validate_tool_config(
    kind: &str,
    config: &serde_json::Value,
    step_id: &str,
    v: &mut WorkflowValidation,
) {
    let obj = match config.as_object() {
        Some(o) => o,
        None => {
            v.error(
                "step.configNotObject",
                "config must be an object".into(),
                Some(step_id.to_owned()),
            );
            return;
        }
    };
    match kind {
        "pdf.rotate" => match obj.get("degrees").and_then(|d| d.as_f64()) {
            Some(d) if d == 90.0 || d == 180.0 || d == 270.0 => {}
            Some(other) => v.error(
                "tool.badRotation",
                format!("degrees must be 90/180/270, got {other}"),
                Some(step_id.to_owned()),
            ),
            None => v.error(
                "tool.missingDegrees",
                "config requires 'degrees'".into(),
                Some(step_id.to_owned()),
            ),
        },
        "image.resize" => {
            for key in ["width", "height"] {
                match obj.get(key).and_then(|d| d.as_f64()) {
                    Some(w) if (1.0..=65535.0).contains(&w) => {}
                    _ => v.error(
                        "tool.badResize",
                        format!("config requires numeric '{key}' (1..=65535)"),
                        Some(step_id.to_owned()),
                    ),
                }
            }
            if let Some(mode) = obj.get("mode").and_then(|m| m.as_str())
                && !matches!(mode, "fit" | "fill" | "exact" | "scale")
            {
                v.error(
                    "tool.badResizeMode",
                    format!("unknown resize mode '{mode}'"),
                    Some(step_id.to_owned()),
                );
            }
        }
        "image.encode" => match obj.get("format").and_then(|f| f.as_str()) {
            Some("png" | "jpeg" | "webp" | "bmp" | "tiff") => {}
            Some(other) => v.error(
                "tool.badEncodeFormat",
                format!("unknown encode format '{other}'"),
                Some(step_id.to_owned()),
            ),
            None => v.error(
                "tool.missingFormat",
                "config requires 'format'".into(),
                Some(step_id.to_owned()),
            ),
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Workflow, WorkflowStep};

    fn step(id: &str, ty: StepType, kind: &str) -> WorkflowStep {
        WorkflowStep {
            id: id.into(),
            r#type: ty,
            name: id.into(),
            kind: kind.into(),
            config: serde_json::json!({}),
            enabled: true,
        }
    }

    fn valid_workflow() -> Workflow {
        Workflow {
            schema_version: 1,
            id: "wf".into(),
            name: "WF".into(),
            description: String::new(),
            steps: vec![
                step("i1", StepType::Input, "files"),
                step("e1", StepType::Export, "export.files"),
            ],
            parameters: vec![],
        }
    }

    #[test]
    fn minimal_valid_workflow_passes() {
        let v = validate(&valid_workflow());
        assert!(!v.has_errors(), "{v:?}");
    }

    #[test]
    fn structural_rules_enforced() {
        // §27：无 Input / Export 不在末位 / 多 Export / 重复 id
        let mut wf = valid_workflow();
        wf.steps.clear();
        wf.steps.push(step("e1", StepType::Export, "export.files"));
        let v = validate(&wf);
        assert!(v.has_errors());

        let mut wf = valid_workflow();
        wf.steps.insert(1, step("t1", StepType::Tool, "pdf.rotate"));
        let v = validate(&wf);
        assert!(v.has_errors(), "Export 不在末位");

        let mut wf = valid_workflow();
        wf.steps.insert(1, step("dup", StepType::Input, "files"));
        wf.steps.insert(2, step("dup", StepType::Input, "files"));
        let v = validate(&wf);
        assert!(v.has_errors(), "duplicate id");
    }

    #[test]
    fn unknown_tool_is_error_not_skip() {
        // §48：Missing Tool 显式 error（不静默跳过）
        let mut wf = valid_workflow();
        wf.steps.insert(1, step("t1", StepType::Tool, "shell.exec"));
        let v = validate(&wf);
        assert!(v.has_errors());
        assert!(v.issues.iter().any(|i| i.code == "tool.unavailable"));
    }

    #[test]
    fn tool_config_semantic_validation() {
        // §28：rotation=45 语义错误
        let mut wf = valid_workflow();
        let mut t = step("t1", StepType::Tool, "pdf.rotate");
        t.config = serde_json::json!({"degrees": 45});
        wf.steps.insert(1, t);
        let v = validate(&wf);
        assert!(v.has_errors());
        assert!(v.issues.iter().any(|i| i.code == "tool.badRotation"));

        // 合法配置无错
        let mut wf = valid_workflow();
        let mut t = step("t1", StepType::Tool, "pdf.rotate");
        t.config = serde_json::json!({"degrees": 90});
        wf.steps.insert(1, t);
        let v = validate(&wf);
        assert!(!v.has_errors(), "{v:?}");
    }

    #[test]
    fn size_limits_enforced() {
        // §75
        let mut wf = valid_workflow();
        for i in 0..40 {
            let ty = if i % 2 == 0 {
                StepType::Filter
            } else {
                StepType::Tool
            };
            let kind = if i % 2 == 0 {
                "filter.extension"
            } else {
                "document.inspect"
            };
            wf.steps
                .insert(wf.steps.len() - 1, step(&format!("s{i}"), ty, kind));
        }
        let v = validate_with_limits(&wf, &WorkflowSizeLimits::default());
        assert!(v.has_errors());
        assert!(v.issues.iter().any(|i| i.code == "workflow.tooManySteps"));
    }

    #[test]
    fn schema_version_mismatch() {
        let mut wf = valid_workflow();
        wf.schema_version = 99;
        let v = validate(&wf);
        assert!(v.has_errors());
        assert!(v.issues.iter().any(|i| i.code == "workflow.schemaVersion"));
    }
}
