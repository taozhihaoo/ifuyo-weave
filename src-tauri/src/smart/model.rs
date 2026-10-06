//! M2.12 Smart Processing 数据模型（§8/§50/§51）。
//!
//! - specta 纪律：无 u64/SystemTime/usize。
//! - **AI = Advisor，Weave Core = Authority（§31）**：Suggestion 只是意图，
//!   执行永远经 M2.0 Execution Core（Plan → Preview → Apply → Undo）。
//! - Confidence 是建议系统内部的相对置信程度，不是准确率保证（§10）。

/// 置信等级（§10：不做百分比宣称）。
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    specta::Type,
)]
#[serde(rename_all = "camelCase")]
pub enum SmartConfidence {
    Low,
    Medium,
    High,
    /// 证据不足——与 Low 区分（§27 Unknown 一等公民）。
    Unknown,
}

/// 风险等级（§54：Rename=Low、Move=Medium、Delete=High——按操作种类契约）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SmartRiskLevel {
    Low,
    Medium,
    High,
}

/// 建议类型（§8 优先四种，其余诚实"未实现"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionType {
    Rename,
    Organize,
    Classify,
    DeleteCandidate,
}

/// 证据种类（§51 精选本项目真实存在的来源）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SmartEvidenceKind {
    FileName,
    Extension,
    Metadata,
    Path,
    Content,
    Duplicate,
    Rule,
    Provider,
}

/// 证据等级（§51：Confirmed/Derived/Heuristic/ProviderGenerated——不制造假依据 §9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SmartEvidenceLevel {
    Confirmed,
    Derived,
    Heuristic,
    ProviderGenerated,
}

/// 单条证据（§9：可解释；detail 不含敏感原文——脱敏由调用方保证 §16）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartEvidenceItem {
    pub kind: SmartEvidenceKind,
    pub detail: String,
    pub level: SmartEvidenceLevel,
}

/// 智能建议（§8 SmartSuggestion——Advisor 输出，非执行指令）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartSuggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    /// 目标文件绝对路径。
    pub target_path: String,
    /// Rename: 原文件名；Classify: 原类别；DeleteCandidate: 原路径语义占位。
    pub original_value: String,
    /// Rename: 建议文件名；Organize: 建议子目录；Classify: 建议类别。
    pub suggested_value: String,
    pub reason: String,
    pub evidence: Vec<SmartEvidenceItem>,
    pub confidence: SmartConfidence,
    pub risk: SmartRiskLevel,
    /// 冲突事实（目标已存在/批内环——Plan 侧会再次硬校验 §24）。
    pub conflict: Option<String>,
    /// 备选方案（§11：不强制唯一答案）。
    pub alternatives: Vec<String>,
}

/// 分析统计（§20 结果摘要）。
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartStats {
    pub files_analyzed: f64,
    pub high_confidence: f64,
    pub medium_confidence: f64,
    pub low_confidence: f64,
    pub unknown_confidence: f64,
    pub suggestions: f64,
    pub errors: f64,
}

/// 会话摘要（§34/§35：仅元数据与计数——绝不存正文/敏感值 §34）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartSessionSummary {
    pub session_id: String,
    pub created_at_ms: f64,
    pub modes: Vec<crate::smart::rules::AnalyzeMode>,
    pub roots: Vec<String>,
    pub stats: SmartStats,
    pub provider: String,
}

/// 一次分析的结果（单槽缓存——§46 内存纪律）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartAnalysisResult {
    pub session_id: String,
    pub stats: SmartStats,
    pub suggestions: Vec<SmartSuggestion>,
    /// 无法分析的条目（失败隔离 §41：PartialFailure 不丢整批）。
    pub errors: Vec<SmartError>,
    pub provider_status: SmartProviderStatus,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartError {
    pub path: String,
    pub code: String,
}

/// Provider 状态（§13；默认 Disabled——本地智能不依赖 AI §47）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SmartProviderStatus {
    Available,
    Disabled,
    Unavailable,
    Failed,
}

/// mtime 毫秒 → "YYYY-MM-DD"（Derived 证据用；epoch 前 ⇒ None）。
pub fn mtime_date(modified_ms: f64) -> String {
    let secs = (modified_ms / 1000.0).floor() as i64;
    // 无 chrono 依赖——civil-from-days 算法（Howard Hinnant）
    let days = secs.div_euclid(86400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// 置信归档（§20 计数）。
pub fn tally(stats: &mut SmartStats, confidence: SmartConfidence) {
    match confidence {
        SmartConfidence::High => stats.high_confidence += 1.0,
        SmartConfidence::Medium => stats.medium_confidence += 1.0,
        SmartConfidence::Low => stats.low_confidence += 1.0,
        SmartConfidence::Unknown => stats.unknown_confidence += 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_ordering_is_low_to_high_with_unknown_unranked() {
        assert!(SmartConfidence::Low < SmartConfidence::Medium);
        assert!(SmartConfidence::Medium < SmartConfidence::High);
        // Unknown 与 Low 同 Ord 位（都不可自动执行）——仅存在性验证
        let all = [
            SmartConfidence::Unknown,
            SmartConfidence::Low,
            SmartConfidence::Medium,
            SmartConfidence::High,
        ];
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn suggestion_serializes_without_sensitive_content_by_construction() {
        let s = SmartSuggestion {
            id: "s1".into(),
            suggestion_type: SuggestionType::Rename,
            target_path: "C:\\d\\a.jpg".into(),
            original_value: "a.jpg".into(),
            suggested_value: "2026-08-20_photo_001.jpg".into(),
            reason: "metadata date".into(),
            evidence: vec![SmartEvidenceItem {
                kind: SmartEvidenceKind::Metadata,
                detail: "EXIF datetime 2026-08-20".into(),
                level: SmartEvidenceLevel::Confirmed,
            }],
            confidence: SmartConfidence::High,
            risk: SmartRiskLevel::Low,
            conflict: None,
            alternatives: vec![],
        };
        let json = serde_json::to_string(&s).expect("json");
        assert!(json.contains("2026-08-20_photo_001.jpg"));
        assert!(json.contains("\"confidence\":\"high\""));
    }
}
