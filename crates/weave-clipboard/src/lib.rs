//! weave-clipboard — 剪贴板历史领域（M2.3）。
//!
//! Capture/Restore 的平台适配在应用层（src-tauri clipboard_service，
//! M3 §23 平台适配器隔离）；本 crate 只做：
//! 模型 / 指纹 / 去重 / 保留 / 敏感启发 / 搜索 / 版本化存储。
//!
//! 隐私边界（§52/§53）：本 crate 的错误与报告绝不携带原始内容——
//! 只有 id/kind/size 等安全诊断字段。

pub mod capture;
pub mod model;
pub mod store;

pub use capture::{CaptureInput, CapturePipeline, CapturedEntry};
pub use model::{
    ClipboardEntry, ContentKind, EntryStatus, RetentionPolicy, RetentionReport,
    STORAGE_SCHEMA_VERSION, Sensitivity, SourceInfo, SourceState, apply_retention, make_preview,
    search_entries, search_text_of, sensitivity_heuristic, storage_error,
};
pub use store::{ClipboardStore, LoadOutcome};
