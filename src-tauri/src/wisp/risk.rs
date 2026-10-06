//! M2.9 Wisp 风险引擎（§7-§10/§16）：内容敏感性 × 目标上下文 × 策略
//! ⇒ PasteRisk 与 PasteDecision。
//!
//! - 敏感性与粘贴风险严格分离（§7）：同一 API Key，目标 VS Code =
//!   Medium，浏览器 = High。
//! - 目标识别以**可执行文件名**为准（chrome.exe，§22），窗口标题只是
//!   辅助；前台窗口 ≠ 一定是粘贴目标 ⇒ 识别置信 Likely（§8 不冒充
//!   Confirmed）。
//! - Unknown ≠ Safe（§47）：目标未知时风险只升不降。
//! - 默认保守不过度阻断（§10）：Normal→Allow；Potentially→Warn；
//!   Highly+不确定目标→Warn/Confirm；Block 仅显式策略或
//!   block_critical 且 Critical。

use crate::wisp::model::{Confidence, SensitivityFinding, SensitivityLevel, SensitivityResult};

/// 粘贴目标应用类别（§51；exe 识别，标题仅辅助）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AppClass {
    Browser,
    CodeEditor,
    Terminal,
    Chat,
    Office,
    Unknown,
}

/// 目标识别置信（§8：Confirmed/Likely/Unknown——不冒充确定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TargetConfidence {
    /// 用户显式声明（UI 选择/用户规则）。
    Confirmed,
    /// 系统前台窗口推断（≠ 必然粘贴目标 §8）。
    Likely,
    Unknown,
}

/// 粘贴目标上下文（§8）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TargetContext {
    /// 可执行文件名（小写，如 "chrome.exe"）；未知 ⇒ None。
    pub process: Option<String>,
    /// 窗口标题（辅助证据；不作为身份 §22）。
    pub window_title: Option<String>,
    pub app_class: AppClass,
    pub identity_confidence: TargetConfidence,
}

impl TargetContext {
    pub fn unknown() -> Self {
        Self {
            process: None,
            window_title: None,
            app_class: AppClass::Unknown,
            identity_confidence: TargetConfidence::Unknown,
        }
    }

    /// 由前台窗口推断（Likely 置信；进程名小写化）。
    pub fn likely(process: String, window_title: Option<String>) -> Self {
        let lower = process.to_lowercase();
        Self {
            app_class: classify_app(&lower),
            process: Some(lower),
            window_title,
            identity_confidence: TargetConfidence::Likely,
        }
    }
}

/// 按可执行文件名分类（§22 不依赖标题；表驱动，P1 可扩展）。
/// 防御性内部小写化——调用方约定小写，函数自身不再依赖约定。
pub fn classify_app(process_lower: &str) -> AppClass {
    const BROWSERS: [&str; 8] = [
        "chrome", "msedge", "firefox", "opera", "brave", "vivaldi", "safari", "360se",
    ];
    const TERMINALS: [&str; 6] = [
        "cmd",
        "powershell",
        "pwsh",
        "wt",
        "windowsterminal",
        "conhost",
    ];
    const EDITORS: [&str; 7] = [
        "code",
        "devenv",
        "notepad",
        "notepad++",
        "sublime_text",
        "idea",
        "rider",
    ];
    const CHAT: [&str; 7] = [
        "teams", "slack", "discord", "wechat", "weixin", "qq", "telegram",
    ];
    const OFFICE: [&str; 5] = ["winword", "excel", "powerpnt", "outlook", "wps"];
    let lower = process_lower.to_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    let matches_group = |names: &[&str]| {
        names.iter().any(|g| {
            stem == *g || stem.starts_with(&format!("{g}-")) || stem.starts_with(&format!("{g} "))
        })
    };
    if matches_group(&BROWSERS) {
        AppClass::Browser
    } else if matches_group(&TERMINALS) {
        AppClass::Terminal
    } else if matches_group(&EDITORS) {
        AppClass::CodeEditor
    } else if matches_group(&CHAT) {
        AppClass::Chat
    } else if matches_group(&OFFICE) {
        AppClass::Office
    } else {
        AppClass::Unknown
    }
}

/// 粘贴风险（§7：Low/Medium/High/Critical/Unknown）。
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
pub enum PasteRisk {
    Low,
    Medium,
    High,
    Critical,
    Unknown,
}

/// 决策动作（§16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PasteAction {
    Allow,
    Warn,
    Block,
    MaskAndPaste,
    Cancel,
    Unknown,
}

/// 用户策略模式（§9；Default=全局规则）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PolicyMode {
    Default,
    Allow,
    Warn,
    Mask,
    Block,
}

/// 每应用规则（§21：可执行身份 + 模式 + 启用；不存标题 §22）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppRule {
    pub process: String,
    pub mode: PolicyMode,
    pub enabled: bool,
}

/// 全局策略输入（来自 WispConfig 的决策相关字段快照）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GlobalPolicy {
    pub block_critical: bool,
    pub app_rules: Vec<AppRule>,
}

/// 统一决策对象（§16）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PasteDecision {
    pub action: PasteAction,
    pub risk: PasteRisk,
    /// 可解释理由（规则/目标/策略；不含内容 §26）。
    pub reasons: Vec<String>,
    pub requires_confirmation: bool,
    pub target: TargetContext,
    /// 参与决策的规则 ID（审计用 §26）。
    pub rule_ids: Vec<String>,
}

/// 高风险目标类别（§10：浏览器/聊天 = 内容外流边界；未知目标同理）。
fn is_high_exposure_target(target: &TargetContext) -> bool {
    matches!(
        target.app_class,
        AppClass::Browser | AppClass::Chat | AppClass::Unknown
    )
}

/// 风险评估（§7/§10 保守默认；可解释理由逐条给出）。
pub fn evaluate(
    result: &SensitivityResult,
    target: &TargetContext,
    policy: &GlobalPolicy,
) -> PasteDecision {
    let mut reasons = Vec::new();
    let mut rule_ids: Vec<String> = result
        .findings
        .iter()
        .map(|f: &SensitivityFinding| f.rule_id.clone())
        .collect();
    rule_ids.sort();
    rule_ids.dedup();

    // ── 每应用显式规则优先（§9）──
    if let Some(process) = &target.process
        && let Some(rule) = policy
            .app_rules
            .iter()
            .find(|r| r.enabled && r.process.eq_ignore_ascii_case(process))
    {
        let action = match rule.mode {
            PolicyMode::Allow => PasteAction::Allow,
            PolicyMode::Warn => PasteAction::Warn,
            PolicyMode::Mask => PasteAction::MaskAndPaste,
            PolicyMode::Block | PolicyMode::Default => PasteAction::Block,
        };
        reasons.push(format!("app rule: {} = {:?}", process, rule.mode));
        let risk = match rule.mode {
            PolicyMode::Allow => PasteRisk::Low,
            PolicyMode::Block => PasteRisk::Critical,
            _ => content_risk(result, target, &mut reasons),
        };
        return PasteDecision {
            action,
            risk,
            reasons,
            requires_confirmation: action == PasteAction::Warn,
            target: target.clone(),
            rule_ids,
        };
    }

    // ── 全局规则（§10）──
    let risk = content_risk(result, target, &mut reasons);
    // Unknown ≠ Safe（§47）：不可分析 ⇒ 至少提醒，绝不静默 Allow
    let (action, requires_confirmation) = match risk {
        PasteRisk::Low => (PasteAction::Allow, false),
        PasteRisk::Medium | PasteRisk::High | PasteRisk::Unknown => (PasteAction::Warn, true),
        PasteRisk::Critical if policy.block_critical => (PasteAction::Block, true),
        PasteRisk::Critical => (PasteAction::Warn, true),
    };
    PasteDecision {
        action,
        risk,
        reasons,
        requires_confirmation,
        target: target.clone(),
        rule_ids,
    }
}

/// 内容 × 目标 ⇒ 风险（不混决策；§7 语义分离）。
fn content_risk(
    result: &SensitivityResult,
    target: &TargetContext,
    reasons: &mut Vec<String>,
) -> PasteRisk {
    match result.level {
        SensitivityLevel::Normal => {
            reasons.push("no findings".to_string());
            PasteRisk::Low
        }
        SensitivityLevel::Unknown => {
            reasons.push("content could not be analyzed (unknown ≠ safe)".to_string());
            PasteRisk::Unknown
        }
        SensitivityLevel::PotentiallySensitive => {
            let only_low_confidence = result
                .findings
                .iter()
                .all(|f| f.confidence == Confidence::Low);
            if only_low_confidence {
                // 单条邮箱/占位符等：低风险不打扰（§5D/§46）
                reasons
                    .push("only low-confidence findings (placeholder/personal info)".to_string());
                PasteRisk::Low
            } else {
                reasons.push("potentially sensitive content".to_string());
                PasteRisk::Medium
            }
        }
        SensitivityLevel::HighlySensitive => {
            let max_sev = result.findings.iter().map(|f| f.severity).max();
            let critical = max_sev.is_some_and(|s| s >= crate::wisp::model::Severity::Critical);
            if critical {
                reasons.push("highly sensitive: named credential pattern".to_string());
                PasteRisk::Critical
            } else {
                reasons.push("highly sensitive content".to_string());
                PasteRisk::High
            }
        }
    }
    .escalate_for_target(target, reasons)
}

trait RiskEscalation {
    fn escalate_for_target(self, target: &TargetContext, reasons: &mut Vec<String>) -> PasteRisk;
}

impl RiskEscalation for PasteRisk {
    fn escalate_for_target(self, target: &TargetContext, reasons: &mut Vec<String>) -> PasteRisk {
        if self >= PasteRisk::High {
            return self;
        }
        if self >= PasteRisk::Medium && is_high_exposure_target(target) {
            match target.app_class {
                AppClass::Browser => reasons.push("target is a browser".to_string()),
                AppClass::Chat => reasons.push("target is a chat app".to_string()),
                _ => reasons.push("target unknown (unknown ≠ safe)".to_string()),
            }
            return PasteRisk::High;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wisp::rules::{DetectionToggles, analyze_text};

    fn decision_for(text: &str, target: TargetContext) -> PasteDecision {
        let result = analyze_text(text, &DetectionToggles::default());
        evaluate(&result, &target, &GlobalPolicy::default())
    }

    #[test]
    fn app_classification_is_by_executable_not_title() {
        assert_eq!(classify_app("chrome.exe"), AppClass::Browser);
        assert_eq!(classify_app("CHROME.EXE"), AppClass::Browser);
        assert_eq!(classify_app("msedge.exe"), AppClass::Browser);
        assert_eq!(classify_app("code.exe"), AppClass::CodeEditor);
        assert_eq!(classify_app("powershell.exe"), AppClass::Terminal);
        assert_eq!(classify_app("wechat.exe"), AppClass::Chat);
        assert_eq!(classify_app("winword.exe"), AppClass::Office);
        assert_eq!(classify_app("unknownapp.exe"), AppClass::Unknown);
    }

    #[test]
    fn normal_content_is_allowed_quietly() {
        let d = decision_for(
            "Hello World",
            TargetContext::likely("chrome.exe".into(), None),
        );
        assert_eq!(d.action, PasteAction::Allow);
        assert_eq!(d.risk, PasteRisk::Low);
        assert!(!d.requires_confirmation);
    }

    #[test]
    fn placeholder_only_content_is_not_disturbed() {
        // §5D/§46：低置信（占位符/单邮箱）⇒ Low 风险不打扰
        let d = decision_for(
            "password = \"<your-password>\"",
            TargetContext::likely("code.exe".into(), None),
        );
        assert_eq!(d.risk, PasteRisk::Low);
        assert_eq!(d.action, PasteAction::Allow);
    }

    #[test]
    fn same_token_different_target_different_risk() {
        // Critical 级命名凭证：目标不升级（已到顶），决策仍 Warn
        let token = "token: ghp_AbcdefGhijKlmnopPqrstUv1234";
        let editor = decision_for(token, TargetContext::likely("code.exe".into(), None));
        let browser = decision_for(token, TargetContext::likely("chrome.exe".into(), None));
        assert_eq!(editor.risk, PasteRisk::Critical);
        assert_eq!(browser.risk, PasteRisk::Critical);
        assert_eq!(editor.action, PasteAction::Warn);
        // §7 示例主场景：Medium 级内容，编辑器 Medium → 浏览器 High
        let medium = "password = hunter2secret-value";
        let ed = decision_for(medium, TargetContext::likely("code.exe".into(), None));
        let br = decision_for(medium, TargetContext::likely("chrome.exe".into(), None));
        assert_eq!(ed.risk, PasteRisk::Medium);
        assert_eq!(br.risk, PasteRisk::High, "§7: 同内容浏览器目标升级");
    }

    #[test]
    fn unknown_target_escalates_not_silently_allows() {
        let medium = "password = hunter2secret-value";
        let d = decision_for(medium, TargetContext::unknown());
        assert_eq!(d.risk, PasteRisk::High, "Unknown ≠ Safe");
        assert_eq!(d.action, PasteAction::Warn);
        assert!(d.requires_confirmation);
    }

    #[test]
    fn default_is_warn_not_block() {
        // §10：HighlySensitive + 高风险目标 → Strong Warning，不默认 Block
        let token = "token: ghp_AbcdefGhijKlmnopPqrstUv1234";
        let d = decision_for(token, TargetContext::likely("chrome.exe".into(), None));
        assert_eq!(d.action, PasteAction::Warn);
    }

    #[test]
    fn block_only_with_explicit_policy() {
        let token = "token: ghp_AbcdefGhijKlmnopPqrstUv1234";
        let result = analyze_text(token, &DetectionToggles::default());
        let target = TargetContext::likely("chrome.exe".into(), None);
        let policy = GlobalPolicy {
            block_critical: true,
            app_rules: vec![AppRule {
                process: "chrome.exe".into(),
                mode: PolicyMode::Block,
                enabled: true,
            }],
        };
        let d = evaluate(&result, &target, &policy);
        assert_eq!(d.action, PasteAction::Block);
        assert!(d.reasons[0].starts_with("app rule"));
    }

    #[test]
    fn per_app_allow_overrides_everything() {
        let token = "token: ghp_AbcdefGhijKlmnopPqrstUv1234";
        let result = analyze_text(token, &DetectionToggles::default());
        let target = TargetContext::likely("code.exe".into(), None);
        let policy = GlobalPolicy {
            block_critical: false,
            app_rules: vec![AppRule {
                process: "code.exe".into(),
                mode: PolicyMode::Allow,
                enabled: true,
            }],
        };
        let d = evaluate(&result, &target, &policy);
        assert_eq!(d.action, PasteAction::Allow);
        assert_eq!(d.risk, PasteRisk::Low);
    }

    #[test]
    fn decision_carries_rule_ids_not_content() {
        let token = "token: ghp_AbcdefGhijKlmnopPqrstUv1234";
        let d = decision_for(token, TargetContext::likely("chrome.exe".into(), None));
        assert!(d.rule_ids.contains(&"wisp.github-token".to_string()));
        // 决策对象可序列化且不含原文
        let json = serde_json::to_string(&d).unwrap();
        assert!(!json.contains("AbcdefGhijKlmnopPqrstUv1234"));
    }
}
