//! M2.12 Smart Processing 智能处理中心（§1 定位：理解 → 建议 → 解释 →
//! 审核 → 安全执行）。
//!
//! 架构纪律（§2/§22/§31/§70 审计结论——零第二套）：
//! - **AI = Advisor，Weave Core = Authority**：本模块只产出 Suggestion；
//!   执行永远经 M2.0（Plan → 既有 execute_plan / execute_recycle_plan →
//!   事务/Undo/Audit）。
//! - **Local-first（§12/§14/§47）**：规则引擎完整可用，AI Provider 默认
//!   Disabled；Provider 输出必须过 schema/domain 验证（§30）。
//! - **复用**：Type 分类=weave-files classify；EXIF=weave-media（同
//!   kamadak 依赖）；文档 title=weave-documents inspect；重复分组=同一
//!   partial_hash 原语；Job=JobTracker；Apply=PlanCache + M2.0 IPC。

pub mod extract;
pub mod model;
pub mod provider;
pub mod rules;

pub use extract::collect_facts;
pub use model::{
    SmartAnalysisResult, SmartConfidence, SmartError, SmartEvidenceItem, SmartEvidenceKind,
    SmartEvidenceLevel, SmartProviderStatus, SmartRiskLevel, SmartSessionSummary, SmartStats,
    SmartSuggestion, SuggestionType,
};
pub use provider::{DisabledProvider, ProviderInput, SmartProvider, validate_provider_output};
pub use rules::{
    AnalyzeMode, FileFacts, classify, cleanup_candidates, organize_suggestion, rename_suggestion,
};
