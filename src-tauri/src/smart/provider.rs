//! M2.12 Provider 抽象（§13/§30/§31/§47）：AI = Advisor，Weave Core =
//! Authority。
//!
//! - **默认 Disabled**（§14）：本地规则引擎完整可用；Provider 仅在用户
//!   显式开启时参与（本阶段无内置云 Provider——trait + schema 验证 +
//!   mock 测试固化边界，§63 Out of Scope 不做具体云集成）。
//! - **Schema Validation（§30）**：Provider 输出（JSON）必须通过解析 →
//!   字段校验 → 路径 scope 校验 → 动作白名单 → 文件存在校验，才能进入
//!   Suggestion 模型；AI 不能决定权限边界。
//! - 输出必须结构化——原始模型文本不直接呈现给用户（§49）。

use std::path::Path;

use weave_core::prelude::WeaveError;

use super::model::{
    SmartConfidence, SmartEvidenceItem, SmartEvidenceKind, SmartEvidenceLevel, SmartProviderStatus,
    SmartRiskLevel, SmartSuggestion, SuggestionType,
};

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("smart::provider")
}

/// Provider trait（§13 ISmartProvider——只有真实边界才抽象；本阶段唯一
/// 实现是测试 mock 与 Disabled 直通，云实现属于后续 opt-in 阶段 §63）。
pub trait SmartProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn status(&self) -> SmartProviderStatus;
    /// 返回原始 JSON 输出（schema 验证前）——实现方负责网络/模型调用与
    /// Timeout/Retry/Cancellation（§48；mock 直接返回内嵌 JSON）。
    fn analyze_raw(&self, input: &ProviderInput) -> Result<Vec<serde_json::Value>, WeaveError>;
}

/// 发给 Provider 的输入（§15：**默认 Metadata Only**——绝不全文件上传；
/// text_snippet 有界且仅在用户显式允许时填充）。
pub struct ProviderInput {
    pub file_name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_ms: f64,
    /// Metadata Only 策略下恒为 None（§15 危险分级：Metadata < Text < Full）。
    pub text_snippet: Option<String>,
}

impl ProviderInput {
    /// Metadata Only 构造（§15 默认）。
    pub fn metadata_only(
        file_name: &str,
        extension: &str,
        size_bytes: u64,
        modified_ms: f64,
    ) -> Self {
        Self {
            file_name: file_name.to_string(),
            extension: extension.to_string(),
            size_bytes,
            modified_ms,
            text_snippet: None,
        }
    }
}

/// Provider 输出 schema（§30：action 白名单 + 必填字段 + 值域）。
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawProviderSuggestion {
    /// rename | organize | classify（delete 永远不在 Provider 白名单 §31）。
    pub action: String,
    pub path: String,
    pub suggested_value: String,
    /// 0.0-1.0（映射 Confidence 档位；§10 不做百分比宣称）。
    pub confidence: f64,
    #[serde(default)]
    pub reason: String,
}

/// Schema + Domain 验证（§30：Parse → Schema → Domain → Conflict → Model）。
///
/// `permitted_roots`：路径必须在用户输入的分析范围内（AI 不能越权指定
/// scope 外路径 §30）；`existing_paths`：分析时真实存在的文件集合。
pub fn validate_provider_output(
    raw: &[serde_json::Value],
    permitted_roots: &[String],
    existing_paths: &std::collections::HashSet<String>,
) -> Result<Vec<SmartSuggestion>, WeaveError> {
    let mut out = Vec::new();
    for (idx, value) in raw.iter().enumerate() {
        let raw: RawProviderSuggestion = serde_json::from_value(value.clone())
            .map_err(|e| err("smart.providerSchemaInvalid", format!("item {idx}: {e}")))?;
        // 动作白名单（§30/§31：delete/execute 等永不在白名单）
        let s_type = match raw.action.as_str() {
            "rename" => SuggestionType::Rename,
            "organize" => SuggestionType::Organize,
            "classify" => SuggestionType::Classify,
            other => {
                return Err(err(
                    "smart.providerActionNotAllowed",
                    format!("item {idx}: action '{other}' is not in the provider allow-list"),
                ));
            }
        };
        // 路径 scope（§30：path 在允许范围内）
        let path = Path::new(&raw.path);
        let in_scope = permitted_roots.iter().any(|root| path.starts_with(root));
        if !in_scope {
            return Err(err(
                "smart.providerPathOutsideScope",
                format!(
                    "item {idx}: path {} is outside the analyzed scope",
                    raw.path
                ),
            ));
        }
        // 文件存在（§30：不存在的文件仍生成 Action = 典型 AI 错误）
        if !existing_paths.contains(&raw.path) {
            return Err(err(
                "smart.providerPathMissing",
                format!(
                    "item {idx}: path {} does not exist in the analysis set",
                    raw.path
                ),
            ));
        }
        // 置信值域
        if !(0.0..=1.0).contains(&raw.confidence) {
            return Err(err(
                "smart.providerConfidenceOutOfRange",
                format!("item {idx}: confidence {} outside 0.0-1.0", raw.confidence),
            ));
        }
        let confidence = if raw.confidence >= 0.8 {
            SmartConfidence::High
        } else if raw.confidence >= 0.5 {
            SmartConfidence::Medium
        } else {
            SmartConfidence::Low
        };
        out.push(SmartSuggestion {
            id: String::new(),
            suggestion_type: s_type,
            target_path: raw.path.clone(),
            original_value: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            suggested_value: raw.suggested_value,
            reason: if raw.reason.is_empty() {
                "provider suggestion".into()
            } else {
                raw.reason
            },
            evidence: vec![SmartEvidenceItem {
                kind: SmartEvidenceKind::Provider,
                detail: "generated by external provider (schema-validated)".into(),
                level: SmartEvidenceLevel::ProviderGenerated,
            }],
            confidence,
            risk: match s_type {
                SuggestionType::Rename => SmartRiskLevel::Low,
                SuggestionType::Organize => SmartRiskLevel::Medium,
                _ => SmartRiskLevel::High,
            },
            conflict: None,
            alternatives: Vec::new(),
        });
    }
    Ok(out)
}

/// Disabled 直通（§47 优雅降级：Provider 不可用 ⇒ 纯本地分析照常）。
pub struct DisabledProvider;

impl SmartProvider for DisabledProvider {
    fn name(&self) -> &'static str {
        "disabled"
    }

    fn status(&self) -> SmartProviderStatus {
        SmartProviderStatus::Disabled
    }

    fn analyze_raw(&self, _input: &ProviderInput) -> Result<Vec<serde_json::Value>, WeaveError> {
        Err(err(
            "smart.providerDisabled",
            "AI enhanced processing is disabled (local analysis still available)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> Vec<String> {
        vec!["C:\\data".to_string()]
    }

    fn existing() -> std::collections::HashSet<String> {
        ["C:\\data\\a.pdf", "C:\\data\\b.jpg"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn raw(action: &str, path: &str, value: &str, confidence: f64) -> serde_json::Value {
        serde_json::json!({
            "action": action,
            "path": path,
            "suggestedValue": value,
            "confidence": confidence,
            "reason": "test"
        })
    }

    #[test]
    fn valid_output_becomes_structured_suggestion() {
        let out = validate_provider_output(
            &[raw("rename", "C:\\data\\a.pdf", "Q3-report.pdf", 0.9)],
            &roots(),
            &existing(),
        )
        .expect("valid");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].suggested_value, "Q3-report.pdf");
        assert_eq!(out[0].confidence, SmartConfidence::High);
        // 结构化建议（§49）：不是模型原始文本
        assert!(matches!(
            out[0].evidence[0].kind,
            SmartEvidenceKind::Provider
        ));
    }

    #[test]
    fn invalid_json_is_rejected() {
        let bad = serde_json::json!({ "action": "rename", "wrong": true });
        assert!(validate_provider_output(&[bad], &roots(), &existing()).is_err());
    }

    #[test]
    fn schema_mismatch_unknown_fields_rejected() {
        let bad = serde_json::json!({
            "action": "rename", "path": "C:\\data\\a.pdf",
            "suggestedValue": "x.pdf", "confidence": 0.9, "extra": 1
        });
        assert!(validate_provider_output(&[bad], &roots(), &existing()).is_err());
    }

    #[test]
    fn delete_action_never_passes_allow_list() {
        // §31：AI 不能决定删除
        let e = validate_provider_output(
            &[raw("delete", "C:\\data\\a.pdf", "-", 0.99)],
            &roots(),
            &existing(),
        )
        .expect_err("blocked");
        assert!(e.code.starts_with("smart.providerActionNotAllowed"), "{e}");
    }

    #[test]
    fn path_outside_scope_rejected() {
        let e = validate_provider_output(
            &[raw("rename", "C:\\windows\\system32\\x.dll", "y.dll", 0.9)],
            &roots(),
            &existing(),
        )
        .expect_err("blocked");
        assert!(e.code.starts_with("smart.providerPathOutsideScope"), "{e}");
    }

    #[test]
    fn missing_path_rejected() {
        let e = validate_provider_output(
            &[raw("rename", "C:\\data\\ghost.pdf", "y.pdf", 0.9)],
            &roots(),
            &existing(),
        )
        .expect_err("blocked");
        assert!(e.code.starts_with("smart.providerPathMissing"), "{e}");
    }

    #[test]
    fn confidence_out_of_range_rejected() {
        let e = validate_provider_output(
            &[raw("rename", "C:\\data\\a.pdf", "x.pdf", 1.5)],
            &roots(),
            &existing(),
        )
        .expect_err("blocked");
        assert!(
            e.code.starts_with("smart.providerConfidenceOutOfRange"),
            "{e}"
        );
    }

    #[test]
    fn disabled_provider_reports_honest_status() {
        // §47：AI 不可用 ⇒ 本地分析照常（Provider 直通只报状态）
        let p = DisabledProvider;
        assert_eq!(p.status(), SmartProviderStatus::Disabled);
        assert!(
            p.analyze_raw(&ProviderInput::metadata_only("a", "pdf", 1, 0.0))
                .is_err()
        );
    }
}
