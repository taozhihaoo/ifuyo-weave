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

pub mod classify;
pub mod encoding;
pub mod fs;
pub mod hash;
pub mod inspect;
pub mod metadata;

pub use classify::{Classification, ClassificationEvidence, FileCategory, classify};
pub use encoding::detect_encoding;
pub use fs::Filesystem;
pub use hash::{HASH_CHUNK_SIZE, hash_file};
pub use metadata::FileStat;
