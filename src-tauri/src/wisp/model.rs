//! M2.9 Wisp 敏感性模型（§4/§6）：可解释 findings，不返回裸 bool。
//!
//! - 与 M2.3 Sensitivity（Normal/Potentially/Unknown）同源但更细：M2.9
//!   增加 HighlySensitive 与 per-finding confidence/severity/evidence（§4
//!   "现有命名成熟可保持"——M2.3 枚举不动，Wisp 用自己的模型）。
//! - 与 M2.7/Paste Risk 严格分离：这里只回答"内容是什么"，风险在
//!   `risk.rs`（§7 敏感性 ≠ 粘贴风险）。
//! - 隐私（§26）：evidence 是描述性文本 + masked preview，绝不含完整 secret。

/// 内容敏感性等级（§4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SensitivityLevel {
    Normal,
    PotentiallySensitive,
    HighlySensitive,
    /// 输入不可分析（非文本/超限截断外的情况）——Unknown ≠ Safe（§47）。
    Unknown,
}

/// 检测置信度（§39：可解释规则优先，不造复杂数学模型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// 潜在风险严重度（检测侧；≠ 粘贴风险 §7）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// 敏感类别（§5 A-E）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SensitivityCategory {
    /// 认证与身份（password/token/private key/auth header）。
    Credential,
    /// 云/服务凭证（AWS/GitHub/Slack/DB 连接串等具名前缀）。
    CloudCredential,
    /// 本地路径/用户名（≠ 一定敏感 ⇒ 仅 Potentially §5C）。
    LocalPath,
    /// 个人信息（email/IP——保守，默认不过度阻断 §5D）。
    PersonalInfo,
    /// 配置/代码中的 secret 形态（password= ... 等）。
    SecretConfig,
}

/// 单条可解释发现（§6）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SensitivityFinding {
    /// 规则追踪 ID（如 "wisp.github-token"）。
    pub rule_id: String,
    pub category: SensitivityCategory,
    pub confidence: Confidence,
    pub severity: Severity,
    /// 证据描述（模式类别/上下文；不含原文 §26）。
    pub evidence: String,
    /// UI 展示用（如 `ghp_••••3456`）；告知风险，不重新复制风险（§18）。
    pub masked_preview: String,
    /// 匹配范围（UTF-8 字节偏移；specta 禁 usize ⇒ u32）。
    pub start_offset: u32,
    pub end_offset: u32,
}

/// 分析结果（§6）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SensitivityResult {
    pub level: SensitivityLevel,
    pub findings: Vec<SensitivityFinding>,
    /// 规则集/分析器版本（§36 缓存键的一部分；规则变更缓存失效）。
    pub ruleset_version: u32,
    pub analyzer_version: u32,
    /// 实际分析长度（超长截断如实标注 §35）。
    pub analyzed_chars: u32,
    pub truncated: bool,
}

/// 单次分析输入上限（§35 fast path：普通文本即时；超长截断后台语义如实）。
pub const ANALYZE_MAX_CHARS: usize = 65_536;

/// 整体等级推导：Critical 级或（High 置信 + 凭证类）⇒ HighlySensitive；
/// 其余非空 findings ⇒ PotentiallySensitive。
pub fn overall_level(findings: &[SensitivityFinding]) -> SensitivityLevel {
    let mut level = SensitivityLevel::Normal;
    for f in findings {
        let credentialish = matches!(
            f.category,
            SensitivityCategory::Credential | SensitivityCategory::CloudCredential
        );
        let high = f.severity >= Severity::High && f.confidence >= Confidence::High;
        if f.severity >= Severity::Critical || (high && credentialish) {
            return SensitivityLevel::HighlySensitive;
        }
        if f.confidence >= Confidence::Low {
            level = SensitivityLevel::PotentiallySensitive;
        }
    }
    level
}
