//! weave-workflow — Workflow Composition（M10 上）。
//!
//! 职责（§2.2/§34/§35）：Workflow 定义模型（schemaVersion=1）、Tool
//! Registry（映射到真实已实现 M7 stages）、Validation（ERROR/WARNING）、
//! **Compiler：Workflow → weave_batch::JobPlan**（§33 preview/execute
//! 同源；执行/取消/重试/恢复全部复用 M7）。
//! 不负责：执行引擎、Safe Write、脚本执行（§2.2-§2.4 硬边界）。

pub mod compile;
pub mod model;
pub mod registry;
pub mod validation;

pub use compile::{CompileError, compile};
pub use model::{
    StepType, WORKFLOW_SCHEMA_VERSION, Workflow, WorkflowParamType, WorkflowParameter,
    WorkflowSizeLimits, WorkflowStep,
};
pub use registry::{FilterDescriptor, ToolDescriptor, ToolRegistry};
pub use validation::{ValidationIssue, WorkflowValidation, validate, validate_with_limits};
