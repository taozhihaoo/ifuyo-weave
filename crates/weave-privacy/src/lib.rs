//! weave-privacy — Share Preflight 隐私预检（M2.4）。
//!
//! Scan ≠ Cleanup；Confirmed ≠ Suspected；Unknown ≠ Safe；默认 Safe Copy
//! 不动原文件（§77）。Detection 与 Response 分离（§53）——本 crate 只产出
//! Finding 事实与清理后的内容字节；执行走 M2.0 共享执行器（Export 语义）。
//!
//! 隐私铁律（§14/§47/§50）：原始敏感值不出检测边界——Finding 只携带
//! masked 证据与 hash 指纹。

pub mod cleanup;
pub mod model;
pub mod patterns;
pub mod scanners;

pub use cleanup::{apply_cleanup, verify_safe_copy};
pub use model::{
    CleanupAction, CleanupRisk, EvidenceLevel, FindingCategory, FindingFacts, NotCheckedItem,
    PrivacyFinding, PrivacyScanComparison, PrivacyScanSession, SessionStatus, Severity,
    compare_sessions, masked_preview,
};
pub use patterns::{PatternHit, PatternKind, scan_line};
pub use scanners::{ScanLimits, scan_bytes_auto};
