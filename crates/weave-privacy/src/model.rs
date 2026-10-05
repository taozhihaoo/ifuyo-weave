//! 统一隐私发现模型（M2.4 §3/§4/§17/§18/§27/§28）。
//!
//! 证据三级（§3，P0-3）：Confirmed（文件中直接读到）/ Suspected（规则匹配）
//! / Unknown（无法可靠检查）。所有结论遵守：
//! 发现风险 ≠ 恶意内容；疑似 ≠ 敏感；未发现 ≠ 安全。
//!
//! Identity（§28）：fingerprint = hash(relPath+category+subCategory+location
//! +evidenceHash)——敏感值以 hash 参与，明文不入存储/日志（§14/§50）。

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use weave_core::prelude::HashAlgorithm;
use weave_core::prelude::WeaveError;
use weave_files::hash::hash_bytes;

/// 证据性质（§3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceLevel {
    Confirmed,
    Suspected,
    Unknown,
}

/// 严重度（§4：不轻易使用 Critical）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
}

/// 发现类别（§5 系列；集中管理，不散落 UI）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FindingCategory {
    /// 图片元数据（GPS/相机/时间/软件…）。
    ImageMetadata,
    /// Office 文档元数据（作者/公司/标题…）。
    DocumentMetadata,
    /// 隐藏内容（隐藏工作表/修订/批注/隐藏幻灯片）。
    HiddenContent,
    /// 疑似敏感串（token/key/password 形态）。
    SuspectedSecret,
    /// 本地路径/用户名暴露。
    LocalPath,
    /// 联系方式（邮箱/电话）。
    Contact,
    /// 格式不支持/无法检查（Unknown 面）。
    NotChecked,
}

/// 清理动作（§24；与"发现"分离——发现 ≠ 可安全删除，§9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupAction {
    /// 图片元数据剥离（像素重编码；方向已应用；§7 须明示）。
    StripImageMetadata,
    /// Office 核心元数据置空（docProps 字段清空；part 结构不变）。
    ClearDocumentCoreMetadata,
    /// 文本命中值遮蔽（[REDACTED]；仅 Safe Copy，高风险须确认 §24）。
    MaskDetectedValue,
    /// 无自动清理（scan-only；用户自行处理）。
    ScanOnly,
}

impl CleanupAction {
    /// 风险分级（§35）。
    pub fn risk(self) -> CleanupRisk {
        match self {
            CleanupAction::StripImageMetadata | CleanupAction::ClearDocumentCoreMetadata => {
                CleanupRisk::Low
            }
            CleanupAction::MaskDetectedValue => CleanupRisk::High,
            CleanupAction::ScanOnly => CleanupRisk::Low,
        }
    }

    /// 推荐自动勾选（§34：高置信+低破坏性才推荐；hidden/revision/mask 不推荐）。
    pub fn recommended_by_default(self, evidence: EvidenceLevel) -> bool {
        matches!(
            self,
            CleanupAction::StripImageMetadata | CleanupAction::ClearDocumentCoreMetadata
        ) && evidence == EvidenceLevel::Confirmed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupRisk {
    Low,
    Medium,
    High,
}

/// Finding 构造事实（new 的参数包——避免 10 参函数）。
pub struct FindingFacts {
    pub relative_path: String,
    pub category: FindingCategory,
    pub sub_category: String,
    pub severity: Severity,
    pub evidence: EvidenceLevel,
    pub location: String,
    pub description: String,
    pub evidence_summary: String,
    pub cleanup: CleanupAction,
    pub evidence_identity: String,
}

/// 单条发现（§4 统一模型）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyFinding {
    /// 稳定指纹（§28；非随机 id——Re-scan 对比依赖它）。
    pub fingerprint: String,
    /// 相对路径（会话根之下）。
    pub relative_path: String,
    pub category: FindingCategory,
    /// 子类（gps / camera_make / author / hidden_sheet / email / jwt …）。
    pub sub_category: String,
    pub severity: Severity,
    pub evidence: EvidenceLevel,
    /// 位置描述（"EXIF GPS" / "docProps/core.xml" / "第 12 行"）。
    pub location: String,
    /// 解释（§31：What was found / Why it matters）。
    pub description: String,
    /// 证据摘要（遮蔽后；原始值不落盘 §14）。
    pub evidence_summary: String,
    pub cleanup: CleanupAction,
    /// 原始值是否曾可用（对 UI：清理后原值不可恢复——mask 单向）。
    pub original_value_available: bool,
}

impl PrivacyFinding {
    /// 构造 + 计算稳定指纹（§28）。`evidence_identity` = 归一化证据标识
    /// （如 GPS tag 名、masked 值的 hash——明文不进指纹原文）。
    pub fn new(facts: FindingFacts) -> Self {
        let FindingFacts {
            relative_path,
            category,
            sub_category,
            severity,
            evidence,
            location,
            description,
            evidence_summary,
            cleanup,
            evidence_identity,
        } = facts;
        let identity =
            format!("{relative_path}|{category:?}|{sub_category}|{location}|{evidence_identity}");
        let fingerprint = hash_bytes(HashAlgorithm::Sha256, identity.as_bytes())
            .map(|h| format!("pf_{h}"))
            .unwrap_or_else(|_| format!("pf_fallback_{}", identity.len()));
        Self {
            fingerprint,
            relative_path,
            category,
            sub_category,
            severity,
            evidence,
            location,
            description,
            evidence_summary,
            cleanup,
            original_value_available: false,
        }
    }
}

/// 会话状态（§18）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    Queued,
    Scanning,
    Completed,
    Cancelled,
    PartiallyCompleted,
    Failed,
}

/// 隐私扫描会话（§17/§18）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyScanSession {
    pub session_id: String,
    pub root: String,
    pub started_at: SystemTime,
    pub duration_ms: u64,
    pub status: SessionStatus,
    pub files_scanned: u64,
    pub files_supported: u64,
    pub files_unsupported: u64,
    pub files_failed: u64,
    pub findings: Vec<PrivacyFinding>,
    /// Unsupported/未完全检查清单（§32/§38：明示 reason，不静默跳过）。
    pub not_fully_checked: Vec<NotCheckedItem>,
    pub warnings: Vec<String>,
    pub errors: Vec<WeaveError>,
}

/// 未完全检查条目（§38：加密/锁定/损坏/不支持 + reason）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotCheckedItem {
    pub relative_path: String,
    pub reason: String,
}

impl PrivacyScanSession {
    pub fn new(root: &str, session_id: String) -> Self {
        Self {
            session_id,
            root: root.to_string(),
            started_at: SystemTime::now(),
            duration_ms: 0,
            status: SessionStatus::Queued,
            files_scanned: 0,
            files_supported: 0,
            files_unsupported: 0,
            files_failed: 0,
            findings: Vec::new(),
            not_fully_checked: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn high_count(&self) -> u64 {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count() as u64
    }

    pub fn medium_count(&self) -> u64 {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Medium)
            .count() as u64
    }

    pub fn low_count(&self) -> u64 {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Low || f.severity == Severity::Info)
            .count() as u64
    }

    pub fn confirmed_count(&self) -> u64 {
        self.findings
            .iter()
            .filter(|f| f.evidence == EvidenceLevel::Confirmed)
            .count() as u64
    }

    pub fn suspected_count(&self) -> u64 {
        self.findings
            .iter()
            .filter(|f| f.evidence == EvidenceLevel::Suspected)
            .count() as u64
    }
}

/// Re-scan 对比（§27：Resolved/Remaining/New——按稳定指纹）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyScanComparison {
    pub resolved: Vec<String>,
    pub remaining: Vec<String>,
    pub new: Vec<String>,
}

pub fn compare_sessions(
    before: &PrivacyScanSession,
    after: &PrivacyScanSession,
) -> PrivacyScanComparison {
    let before_set: std::collections::HashSet<&str> = before
        .findings
        .iter()
        .map(|f| f.fingerprint.as_str())
        .collect();
    let after_set: std::collections::HashSet<&str> = after
        .findings
        .iter()
        .map(|f| f.fingerprint.as_str())
        .collect();
    // 排序保证报告/测试确定性（集合遍历顺序不保证）
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    PrivacyScanComparison {
        resolved: sorted(before_set.difference(&after_set).map(|s| s.to_string()).collect()),
        remaining: sorted(before_set.intersection(&after_set).map(|s| s.to_string()).collect()),
        new: sorted(after_set.difference(&before_set).map(|s| s.to_string()).collect()),
    }
}

/// 敏感值遮蔽（§14/§50：UI/日志/审计/报告只见 masked 形态）。
/// 保留前 2 + 后 2 字符（短值全遮），中间 `…`。
pub fn masked_preview(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 6 {
        return "•".repeat(chars.len().max(3));
    }
    let head: String = chars[..2].iter().collect();
    let tail: String = chars[chars.len() - 2..].iter().collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_across_calls_and_evidence_hashed() {
        let mk = || {
            PrivacyFinding::new(FindingFacts {
                relative_path: "doc.docx".to_string(),
                category: FindingCategory::DocumentMetadata,
                sub_category: "author".to_string(),
                severity: Severity::Medium,
                evidence: EvidenceLevel::Confirmed,
                location: "docProps/core.xml".to_string(),
                description: "文档包含作者信息".to_string(),
                evidence_summary: "Author: J***".to_string(),
                cleanup: CleanupAction::ClearDocumentCoreMetadata,
                evidence_identity: "author-j".to_string(),
            })
        };
        let a = mk();
        let b = mk();
        assert_eq!(
            a.fingerprint, b.fingerprint,
            "同事实 ⇒ 同指纹（§28 非随机 id）"
        );
        assert!(a.fingerprint.starts_with("pf_"));
        // 证据标识不同 ⇒ 指纹不同
        let c = PrivacyFinding::new(FindingFacts {
            relative_path: "doc.docx".to_string(),
            category: FindingCategory::DocumentMetadata,
            sub_category: "author".to_string(),
            severity: Severity::Medium,
            evidence: EvidenceLevel::Confirmed,
            location: "docProps/core.xml".to_string(),
            description: "文档包含作者信息".to_string(),
            evidence_summary: "Author: K***".to_string(),
            cleanup: CleanupAction::ClearDocumentCoreMetadata,
            evidence_identity: "author-k".to_string(),
        });
        assert_ne!(a.fingerprint, c.fingerprint);
    }

    #[test]
    fn comparison_reports_resolved_remaining_new() {
        let mk_session = |fingerprints: &[&str]| {
            let mut session = PrivacyScanSession::new("R", "s1".to_string());
            for fp in fingerprints {
                session.findings.push(PrivacyFinding {
                    fingerprint: fp.to_string(),
                    relative_path: "a".to_string(),
                    category: FindingCategory::ImageMetadata,
                    sub_category: "gps".to_string(),
                    severity: Severity::High,
                    evidence: EvidenceLevel::Confirmed,
                    location: "EXIF".to_string(),
                    description: String::new(),
                    evidence_summary: String::new(),
                    cleanup: CleanupAction::StripImageMetadata,
                    original_value_available: false,
                });
            }
            session
        };
        let before = mk_session(&["f1", "f2", "f3"]);
        let after = mk_session(&["f2", "f4"]);
        let cmp = compare_sessions(&before, &after);
        assert_eq!(cmp.resolved, vec!["f1", "f3"]);
        assert_eq!(cmp.remaining, vec!["f2"]);
        assert_eq!(cmp.new, vec!["f4"]);
    }

    #[test]
    fn cleanup_risk_and_recommendation_follow_contract() {
        assert_eq!(CleanupAction::StripImageMetadata.risk(), CleanupRisk::Low);
        assert_eq!(CleanupAction::MaskDetectedValue.risk(), CleanupRisk::High);
        // §34：hidden/revision/mask 不默认勾选；GPS/基础元数据 Confirmed 才推荐
        assert!(CleanupAction::StripImageMetadata.recommended_by_default(EvidenceLevel::Confirmed));
        assert!(!CleanupAction::MaskDetectedValue.recommended_by_default(EvidenceLevel::Suspected));
        assert!(
            !CleanupAction::ClearDocumentCoreMetadata
                .recommended_by_default(EvidenceLevel::Suspected)
        );
    }

    #[test]
    fn masked_preview_never_shows_full_value() {
        assert_eq!(masked_preview("sk-1234567890abcdef"), "sk…ef");
        assert_eq!(masked_preview("abc").chars().all(|c| c == '•'), true);
        let long = "a".repeat(100);
        assert!(!masked_preview(&long).contains(&long));
    }
}
