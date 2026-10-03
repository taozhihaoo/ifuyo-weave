//! weave-core — Weave 的稳定领域契约层。
//!
//! 这一 crate 是整个产品的地基：ID、错误模型、Tool trait、Tool Registry、
//! 路径安全契约。它不依赖 Tauri、React 或任何具体文件系统实现（`std::fs`
//! 只允许出现在测试里）。
//!
//! 依赖方向（charter #23）：
//!
//! ```text
//! UI → IPC → Application → weave-core → Adapters → OS
//! ```
//!
//! M0 只建立已被确认的边界；不为"看起来完整"添加未来类型。

pub mod cancellation;
pub mod error;
pub mod facts;
pub mod id;
pub mod path;
pub mod preview;
pub mod progress;
pub mod registry;
pub mod result;
pub mod tool;

/// 一次引入最常用的核心类型。
pub mod prelude {
    pub use crate::cancellation::CancellationToken;
    pub use crate::error::{ErrorKind, Recoverability, WeaveError};
    pub use crate::facts::{
        FileKind, HashAlgorithm, HashResult, HashStatus, ScanStatus, TextEncoding,
    };
    pub use crate::id::{InputKind, JobId, OperationId, ToolCategory, ToolId};
    pub use crate::path::{PathValidation, validate_absolute_path, validate_path};
    pub use crate::preview::{Preview, PreviewItem};
    pub use crate::progress::{OperationStatus, Progress};
    pub use crate::registry::ToolRegistry;
    pub use crate::result::OperationResult;
    pub use crate::tool::{Tool, ToolInput, ToolOptionSpec, UnsupportedTool};
}
