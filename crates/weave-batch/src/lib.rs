//! weave-batch — Unified batch engine（M7 上 §0-§58）。
//!
//! 职责：Job 状态机（§4）、输入快照（§10）、Linear Pipeline（§11-§13）、
//! 同引擎 Preview/Execute（§20-§23）、item 失败隔离（§25/§49）、协作
//! 取消（§32-§34）、诚实进度计数（§30）、确定性别名（§55）。
//! 不负责：React/Tauri、路径校验策略、历史存储（复用 weave-history）。

pub mod engine;
pub mod item;
pub mod plan;
pub mod state;

#[cfg(test)]
mod engine_tests;

pub use engine::{
    ItemResult, ItemStatus, JobResult, StageError, StageResult, execute_plan, preview_plan,
};
pub use item::{ItemContext, ItemPayload};
pub use plan::{
    InputSnapshotEntry, JobPlan, PayloadType, Pipeline, StageSpec, TextOpSpec, build_job_plan,
    revalidate_snapshot, snapshot_inputs,
};
pub use state::JobState;
