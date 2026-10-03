//! weave-files — Weave 文件核心（M1）。
//!
//! 职责（M1 §5.2）：filesystem domain、file inspection、directory scanning、
//! metadata collection、hashing、path safety integration。
//!
//! 架构位置：
//!
//! ```text
//! Application(src-tauri) → weave-files → Filesystem trait → std / OS
//! ```
//!
//! 一切文件系统访问经过 [`fs::Filesystem`] 抽象（可测试、可故障注入），
//! 一切路径先过 `weave_core::path` 校验。本 crate 只报告事实，不推测意图。

pub mod atomic_write;
pub mod classify;
pub mod duplicates;
pub mod encoding;
pub mod execute;
pub mod fs;
pub mod hash;
pub mod inspect;
pub mod metadata;
pub mod organize;
pub mod recycle;
pub mod rename;
pub mod scan;
pub mod undo;

pub use atomic_write::atomic_write;
pub use classify::{Classification, ClassificationEvidence, FileCategory, classify};
pub use duplicates::{
    DuplicateScanReport, RecycleExecution, RecycleSelection, build_recycle_plan,
    execute_recycle_plan, execute_recycle_plan_with, scan_duplicates,
};
pub use encoding::detect_encoding;
pub use execute::{ExecutionReport, execute_plan};
pub use fs::Filesystem;
pub use hash::{HASH_CHUNK_SIZE, hash_file};
pub use inspect::{FileInspection, InspectOptions, InspectionStatus, inspect_file};
pub use metadata::FileStat;
pub use organize::{OrganizerCondition, OrganizerRule, build_organizer_plan};
pub use recycle::{RecycleAdapter, RecycleOutcome, StdRecycleAdapter};
pub use rename::{CaseForm, DateField, DateFormat, RenameRule, TemplateError, build_rename_plan};
pub use scan::{
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_ENTRIES, DirectoryScanReport, FileLineItem, HARD_MAX_DEPTH,
    HARD_MAX_ENTRIES, ScanErrorEntry, ScanOptions, scan_directory,
};
pub use undo::{UndoItemStatus, UndoReport, undo_transaction};

#[cfg(test)]
mod execute_tests;
