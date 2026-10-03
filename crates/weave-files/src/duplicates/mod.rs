//! 精确重复文件检测（M3 §1–§13 / §31–§43）。
//!
//! 三级管线（M2 §31 主算法骨架）：
//!
//! ```text
//! recursive scan（复用 M1 symlink/边界策略）
//!   → group by size          （最便宜的第一层）
//!   → partial hash 候选缩减  （性能优化层，非最终事实）
//!   → full SHA-256           （复用 M1 streaming hash）
//!   → exact duplicate groups
//! ```
//!
//! Exact Duplicate 工程定义（M3 §2）：size 相同 且 完整 SHA-256 相同。
//! 基于密码学哈希的工程判定，不宣称数学意义上的绝对证明。
//! 文件名 / mtime / 扩展名不参与判定；扩展名差异仅作事实展示。

pub mod model;
pub mod pipeline;
pub mod recycle_exec;

pub use recycle_exec::{RecycleExecution, execute_recycle_plan, execute_recycle_plan_with};

pub use model::{DuplicateFileEntry, DuplicateGroup, DuplicateScanReport, ScanStage};
pub use pipeline::{
    PARTIAL_HASH_BYTES, RecycleSelection, build_recycle_plan, partial_hash, scan_duplicates,
};

#[cfg(test)]
mod tests;
