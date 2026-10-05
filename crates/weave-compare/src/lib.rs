//! weave-compare — 统一 Compare/Difference 引擎（M2.1）。
//!
//! Read-only 分析能力：文件比较 + 目录比较 → 统一差异模型。
//! 绝不自带执行/恢复引擎——Actionable 差异经 Plan 进入 M2.0 共享执行器
//! （weave-files::execute 的 Copy/Rename/Move/Recycle 语义，M2.1 §14）。
//!
//! 分层策略（§8）：existence → size → mtime → SHA-256 流式 → 文本详情
//! 按需（复用 weave-text::compare 的有界 LCS）。昂贵档位显式选择。

pub mod dir_compare;
pub mod file_compare;
pub mod model;
pub mod options;

pub use dir_compare::compare_directories;
pub use file_compare::{TextDetail, compare_files, text_detail};
pub use model::{
    ComparisonEvidence, ComparisonSession, ComparisonSummary, ContentDifference, DifferenceItem,
    DifferenceKind, EntryFacts, NodeKind, SessionKind,
};
pub use options::{CompareOptions, ContentCheck, IgnoreRules};
