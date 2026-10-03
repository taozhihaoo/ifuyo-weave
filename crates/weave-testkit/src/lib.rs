//! weave-testkit — 所有 Weave 工具共享的测试基础设施（M0 §23）。
//!
//! - [`temp_workspace::TempWorkspace`]：隔离的临时目录，测试结束自动清理。
//! - [`random::DetRandom`]：种子化确定性随机，测试可复现。
//! - [`fault::FaultInjector`]：统一故障注入机制（M0 只建框架，场景由后续里程碑接入）。
//!
//! 禁止任何测试依赖用户真实文件或机器特有路径。

pub mod fault;
pub mod fixtures;
pub mod random;
pub mod temp_workspace;

pub use fault::{Fault, FaultInjector};
pub use fixtures::{TreeStats, standard_tree};
pub use random::DetRandom;
pub use temp_workspace::TempWorkspace;
