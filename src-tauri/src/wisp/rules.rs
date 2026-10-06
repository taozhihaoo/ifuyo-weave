//! M2.9 Wisp 规则引擎（§5/§37/§39）：确定性、数据驱动、可解释。
//!
//! - 内置规则集 v1：具名凭证前缀（Critical）→ 赋值形态（placeholder
//!   降置信 §5E）→ 本地路径/个人信息（仅 Potentially §5C/§5D）。
//! - **不写成覆盖所有服务的巨大硬编码表**（§5B）：具名前缀 + 结构形态
//!   + 共享原语（M2.3 §53 提取的 looks_like_* 直接复用防漂移）。
//! - 评分（§39）：Pattern + Known Prefix + Placeholder + 长度/熵提示，
//!   综合为 per-finding Confidence；不做 match → block（§39）。

use std::sync::OnceLock;

use crate::wisp::model::{
    overall_level, ANALYZE_MAX_CHARS, Confidence, SensitivityCategory, SensitivityFinding,
    SensitivityResult, Severity,
};
use crate::wisp::mask;

pub const RULESET_VERSION: u32 = 1;
pub const ANALYZER_VERSION: u32 = 1;

/// 检测类别开关（§32 Detection 设置）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectionToggles {
    pub credentials: bool,
    pub local_paths: bool,
    pub personal_info: bool,
    pub secret_config: bool,
}

impl Default for DetectionToggles {
    fn default() -> Self {
        Self { credentials: true, local_paths: true, personal_info: true, secret_config: true }
    }
}

/// 内置规则（§37 SensitivityRule；静态表即数据——v2 未来外置）。
struct SensitivityRule {
    id: &'static str,
    category: SensitivityCategory,
    pattern: &'static str,
    severity: Severity,
    confidence: Confidence,
    description: &'static str,
}

use SensitivityCategory as C;

const RULES: &[SensitivityRule] = &[
    // ── A/B：凭证与云凭证（具名前缀 = Known Prefix，高置信 §39）──
    SensitivityRule { id: "wisp.aws-access-key", category: C::CloudCredential, pattern: r"\bAKIA[0-9A-Z]{16}\b", severity: Severity::Critical, confidence: Confidence::High, description: "AWS access key id prefix" },
    SensitivityRule { id: "wisp.github-token", category: C::CloudCredential, pattern: r"\bgh[pousr]_[A-Za-z0-9]{20,}\b", severity: Severity::Critical, confidence: Confidence::High, description: "GitHub token prefix" },
    SensitivityRule { id: "wisp.slack-token", category: C::CloudCredential, pattern: r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b", severity: Severity::Critical, confidence: Confidence::High, description: "Slack token prefix" },
    SensitivityRule { id: "wisp.secret-key-prefix", category: C::CloudCredential, pattern: r"\bsk-[A-Za-z0-9_-]{20,}\b", severity: Severity::Critical, confidence: Confidence::High, description: "secret key style prefix (sk-…)" },
    SensitivityRule { id: "wisp.private-key-block", category: C::Credential, pattern: r"-----BEGIN [A-Z ]*PRIVATE KEY-----", severity: Severity::Critical, confidence: Confidence::High, description: "PEM private key header" },
    SensitivityRule { id: "wisp.db-connection-string", category: C::CloudCredential, pattern: r"(?i)\b(?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|amqp|mssql)://[^\s:@/]+:[^\s@]+@", severity: Severity::Critical, confidence: Confidence::High, description: "database URL with embedded credentials" },
    SensitivityRule { id: "wisp.jwt", category: C::Credential, pattern: r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{5,}\b", severity: Severity::High, confidence: Confidence::High, description: "JWT three-segment shape" },
    SensitivityRule { id: "wisp.bearer-token", category: C::Credential, pattern: r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{16,}", severity: Severity::High, confidence: Confidence::High, description: "Bearer auth scheme with long token" },
    SensitivityRule { id: "wisp.long-hex-run", category: C::Credential, pattern: r"\b[a-fA-F0-9]{40,}\b", severity: Severity::Medium, confidence: Confidence::Medium, description: "long hex run (key/hash-like)" },
    // ── E：配置/代码 secret 形态（placeholder 降置信在匹配后处理 §5E）──
    SensitivityRule { id: "wisp.secret-assignment", category: C::SecretConfig, pattern: r#"(?im)^[ \t]*(?:password|passwd|secret|api[_-]?key|apikey|access[_-]?token|auth[_-]?token|token)[a-z0-9_-]*[ \t]*[:=][ \t]*["']?([^\s"']{6,})"#, severity: Severity::High, confidence: Confidence::Medium, description: "secret-like key/value assignment" },
    SensitivityRule { id: "wisp.authorization-header", category: C::Credential, pattern: r"(?im)^[ \t]*authorization:[ \t]*\S+", severity: Severity::High, confidence: Confidence::Medium, description: "Authorization header" },
    // ── C：本地路径（Potentially——本地路径 ≠ 一定敏感 §5C）──
    SensitivityRule { id: "wisp.win-user-path", category: C::LocalPath, pattern: r"(?i)\b[A-Z]:\\Users\\[^\\\s]+", severity: Severity::Low, confidence: Confidence::Medium, description: "Windows user profile path" },
    SensitivityRule { id: "wisp.unc-path", category: C::LocalPath, pattern: r"\\\\[A-Za-z0-9_.-]+\\[^\s]+", severity: Severity::Low, confidence: Confidence::Medium, description: "UNC network path" },
    SensitivityRule { id: "wisp.unix-home-path", category: C::LocalPath, pattern: r"(?:/Users|/home)/[A-Za-z0-9_.-]+/", severity: Severity::Low, confidence: Confidence::Medium, description: "unix home directory path" },
    // ── D：个人信息（保守；单条邮箱/ IP 默认不阻断 §5D）──
    SensitivityRule { id: "wisp.email", category: C::PersonalInfo, pattern: r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b", severity: Severity::Low, confidence: Confidence::Medium, description: "email address" },
    SensitivityRule { id: "wisp.ipv4", category: C::PersonalInfo, pattern: r"\b(?:\d{1,3}\.){3}\d{1,3}\b", severity: Severity::Low, confidence: Confidence::Low, description: "IPv4 address" },
];

static REGEXES: OnceLock<Vec<Option<regex::Regex>>> = OnceLock::new();

fn compiled() -> &'static Vec<Option<regex::Regex>> {
    REGEXES.get_or_init(|| RULES.iter().map(|r| regex::Regex::new(r.pattern).ok()).collect())
}

/// placeholder/示例值形态 ⇒ 降置信（§5E：password = "<your-password>"
/// 不应高置信）。检测输入值形状，不声称覆盖全部示例。
fn value_is_placeholder(value: &str) -> bool {
    let lower = value.to_lowercase();
    let trimmed = lower.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
    const HINTS: [&str; 12] = [
        "your", "xxx", "<", "${", "example", "changeme", "change_me", "placeholder", "todo",
        "dummy", "sample", "********",
    ];
    HINTS.iter().any(|h| trimmed.contains(h)) || trimmed == "test" || trimmed == "none" || trimmed == "null"
}

/// 值形状熵提示：长且多样 ⇒ 更像真实 secret（§39 Keyword Context/Length）。
fn value_looks_secret_like(value: &str) -> bool {
    let unique = value.chars().collect::<std::collections::HashSet<_>>().len();
    value.len() >= 16 && unique * 4 >= value.len() as usize
}

/// 分析文本（≤ ANALYZE_MAX_CHARS 截断，如实标注 §35）。
pub fn analyze_text(text: &str, toggles: &DetectionToggles) -> SensitivityResult {
    let truncated = text.chars().count() > ANALYZE_MAX_CHARS;
    let analyzed: String = text.chars().take(ANALYZE_MAX_CHARS).collect();
    let mut findings = Vec::new();
    let regexes = compiled();
    for (idx, rule) in RULES.iter().enumerate() {
        let enabled = match rule.category {
            C::Credential | C::CloudCredential => toggles.credentials,
            C::LocalPath => toggles.local_paths,
            C::PersonalInfo => toggles.personal_info,
            C::SecretConfig => toggles.secret_config,
        };
        if !enabled {
            continue;
        }
        let Some(re) = &regexes[idx] else { continue };
        let mut per_rule = 0usize;
        for m in re.captures_iter(&analyzed) {
            let whole = m.get(0).expect("match");
            // 单规则最多报 20 处（防风暴 §34；同一规则重复命中价值有限）
            if per_rule >= 20 {
                break;
            }
            per_rule += 1;
            let mut confidence = rule.confidence;
            let mut evidence = rule.description.to_string();
            // secret-assignment：值形态细化置信（§5E/§39）
            if let Some(value) = m.get(1) {
                if value_is_placeholder(value.as_str()) {
                    confidence = Confidence::Low;
                    evidence.push_str("; value looks like a placeholder/example");
                } else if value_looks_secret_like(value.as_str()) {
                    confidence = Confidence::High;
                    evidence.push_str("; value is long and high-entropy");
                }
            }
            findings.push(SensitivityFinding {
                rule_id: rule.id.to_string(),
                category: rule.category,
                confidence,
                severity: rule.severity,
                evidence,
                masked_preview: mask::mask_preview(whole.as_str()),
                start_offset: whole.start() as u32,
                end_offset: whole.end() as u32,
            });
        }
    }
    // 偏移排序（mask 替换需要顺序）
    findings.sort_by_key(|f| (f.start_offset, f.end_offset));
    SensitivityResult {
        level: overall_level(&findings),
        findings,
        ruleset_version: RULESET_VERSION,
        analyzer_version: ANALYZER_VERSION,
        analyzed_chars: analyzed.chars().count() as u32,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wisp::model::{Confidence, SensitivityLevel};

    fn levels(text: &str) -> SensitivityResult {
        analyze_text(text, &DetectionToggles::default())
    }

    // ── Normal（§38 误报集）──

    #[test]
    fn normal_texts_are_not_flagged() {
        for t in [
            "Hello World",
            "{\"name\": \"weave\", \"version\": 2}",
            "fn main() { println!(\"hi\"); }",
            "see https://example.com/docs for details",
            "C:\\Projects\\weave\\src\\lib.rs",
            "The quick brown fox jumps over the lazy dog",
        ] {
            let r = levels(t);
            assert_eq!(r.level, SensitivityLevel::Normal, "false positive on: {t}");
        }
    }

    #[test]
    fn single_email_is_only_low_risk_potential() {
        let r = levels("contact me at test@example.com please");
        assert_eq!(r.level, SensitivityLevel::PotentiallySensitive);
        assert!(r.findings.iter().all(|f| f.category == C::PersonalInfo));
    }

    // ── Sensitive（§38 漏报集）──

    #[test]
    fn named_tokens_are_highly_sensitive() {
        for t in [
            "token: ghp_AbcdefGhijKlmnopPqrstUv1234",
            "AKIAIOSFODNN7EXAMPLE",
            "sk-proJabCdeFgHiJkLmNoPqRsTuVwXyZ123456",
            "xoxb-123456789-AbCdEfGhIjKl",
            "-----BEGIN RSA PRIVATE KEY-----",
            "postgres://admin:s3cret@db.internal:5432/prod",
            "Authorization: Bearer abcdef1234567890abcdef",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJSMeKKF2QT4fwpM",
        ] {
            let r = levels(t);
            assert_eq!(r.level, SensitivityLevel::HighlySensitive, "missed: {t}");
        }
    }

    #[test]
    fn password_assignment_is_potential() {
        let r = levels("password = hunter2secret");
        assert_eq!(r.level, SensitivityLevel::PotentiallySensitive);
        assert_eq!(r.findings[0].rule_id, "wisp.secret-assignment");
    }

    // ── Ambiguous（§38：placeholder 降置信，不误判）──

    #[test]
    fn placeholder_values_are_low_confidence_not_high() {
        for t in [
            "password = \"<your-password>\"",
            "api_key = \"YOUR_KEY_HERE\"",
            "token = changeme",
            "SECRET = \"${VAULT_SECRET}\"",
        ] {
            let r = levels(t);
            assert_eq!(r.level, SensitivityLevel::PotentiallySensitive, "{t}");
            let f = r
                .findings
                .iter()
                .find(|f| f.rule_id == "wisp.secret-assignment")
                .expect("assignment finding");
            assert_eq!(f.confidence, Confidence::Low, "placeholder must be low: {t}");
        }
    }

    #[test]
    fn high_entropy_assignment_value_raises_confidence() {
        let r = levels("api_key = Ab3xY9QzW8eR5tH2kN6vJ4mP7qL");
        let f = r
            .findings
            .iter()
            .find(|f| f.rule_id == "wisp.secret-assignment")
            .expect("assignment finding");
        assert_eq!(f.confidence, Confidence::High);
    }

    #[test]
    fn local_path_is_potential_not_highly() {
        let r = levels("log at C:\\Users\\Alice\\project\\main.rs and \\\\server\\share\\f.txt");
        assert_eq!(r.level, SensitivityLevel::PotentiallySensitive);
        assert!(r.findings.iter().all(|f| f.category == C::LocalPath));
    }

    // ── 开关/截断/上限 ──

    #[test]
    fn toggles_disable_categories() {
        let t = DetectionToggles { personal_info: false, ..DetectionToggles::default() };
        let r = analyze_text("mail me at test@example.com", &t);
        assert_eq!(r.level, SensitivityLevel::Normal);
    }

    #[test]
    fn long_input_is_truncated_honestly() {
        let text = "a".repeat(ANALYZE_MAX_CHARS + 10);
        let r = levels(&text);
        assert!(r.truncated);
        assert_eq!(r.analyzed_chars, ANALYZE_MAX_CHARS as u32);
    }

    #[test]
    fn repeated_hits_are_capped_per_rule() {
        let token = "ghp_AbcdefGhijKlmnopPqrstUv1234 ";
        let text = token.repeat(100);
        let r = levels(&text);
        let count = r.findings.iter().filter(|f| f.rule_id == "wisp.github-token").count();
        assert_eq!(count, 20, "per-rule cap 防风暴 §34");
    }

    #[test]
    fn findings_carry_masked_preview_not_raw() {
        let r = levels("token: ghp_AbcdefGhijKlmnopPqrstUv1234");
        let f = &r.findings[0];
        assert!(!f.masked_preview.contains("AbcdefGhijKlmnopPqrstUv1234"));
        assert!(f.masked_preview.contains("••••"));
    }
}
