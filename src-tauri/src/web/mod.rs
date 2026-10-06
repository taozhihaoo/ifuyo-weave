//! M2.10 Web 内容处理中心——模块组织（§4/§64）。
//!
//! 边界纪律（§1/§99 审计结论）：
//! - **单一网络出口**：仅 `fetch.rs` 发起 HTTP（ureq/rustls），其余模块
//!   零网络（§17）。
//! - Fetch 与 Parse 解耦（§65）：远程 URL 与本地 HTML 文件走同一
//!   Parse/Extract/Transform 管线（§63）。
//! - 不建第二套 Job/Clipboard/Export：批量走既有 JobTracker（M1 范式）；
//!   复制走 arboard + M2.3 suppress 指纹（§36/§51）；导出走
//!   atomic_write + creation-undo（M2.0，§69）。

pub mod fetch;
pub mod html;
pub mod markdown;
pub mod model;
pub mod target;
pub mod url;
