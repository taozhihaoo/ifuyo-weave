//! M2.9 Wisp / Paste Guard（§2）：敏感剪贴板识别与粘贴前防护。
//!
//! 架构纪律（§27/§44/§58）：
//! - **单一 Clipboard 数据源**：复用 M2.3 监听（同一线程同一轮询通路
//!   钩子），绝无第二监听器；存储复用 M2.3 store + suppress 指纹。
//! - **复用 Agent 生命周期**：无常驻自建循环；前台窗口识别复用 M2.8
//!   x-win 封装；通知复用 M2.6 NotificationCenter。
//! - **Paste Detection = Limited（§14 如实）**：workspace `unsafe_code
//!   =forbid` ⇒ 无全局键盘钩子；Wisp 在"经 Weave 的粘贴意图"
//!   （guarded recopy）+ 保护会话（mask→粘贴→TTL 恢复）处设防。
//! - 隐私（§26/§53/§55）：日志/事件只含元数据（规则 ID/类别/风险/
//!   决策/exe），绝无完整 secret；本地离线，无网络。

pub mod mask;
pub mod model;
pub mod risk;
pub mod rules;
pub mod session;

pub use mask::{mask_email, mask_preview, mask_text};
pub use model::{
    Confidence, SensitivityCategory, SensitivityFinding, SensitivityLevel, SensitivityResult,
    Severity,
};
pub use risk::{
    AppClass, AppRule, GlobalPolicy, PasteAction, PasteDecision, PasteRisk, PolicyMode,
    TargetConfidence, TargetContext, classify_app, evaluate,
};
pub use rules::{ANALYZER_VERSION, DetectionToggles, RULESET_VERSION, analyze_text};
pub use session::{ProtectionSession, SessionPayload, SessionState, is_expired, precheck_restore};
