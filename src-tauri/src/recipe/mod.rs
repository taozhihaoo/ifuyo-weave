//! M2.7 Recipe / Automation：声明式配方模型 + 运行器。
//!
//! - **Recipe 是编排者，不是第二个执行引擎**（§6）：rename/archive 均走
//!   weave-files 既有 Plan/Execute/archive 核心。
//! - **Preview 与 Apply 共用同一 Plan**（§15）：dry_run=true 只计数不落盘。
//! - **Broken 不运行**（§31）：validate 失败 ⇒ 结构化错误。
//! - 隐私（§48）：运行明细只含计数与状态，不含文件内容。

pub mod model;
pub mod runner;
pub mod store_history;

pub use model::{
    ActionType, RECIPE_SCHEMA_VERSION, Recipe, RecipeAction, RecipeParameter, RecipeStore,
    RecipeTrigger, RiskLevel, ValidationResult, import_recipe, validate,
};
pub use runner::{ActionRunResult, ActionRunStatus, RecipeRunReport, SafetyLimits, run_recipe};
