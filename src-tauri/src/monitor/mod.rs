//! M2.11 Website Monitor 网站监控中心（§2 核心定位：指定网页持续知道
//! 是否变化）。
//!
//! 架构纪律（§1/§70/§80 审计结论——全部复用，零第二套）：
//! - **HTTP**：复用 M2.10 `web::fetch`（唯一网络出口 §17/§55）。
//! - **提取**：复用 M2.10 html 管线（WholePage=plain_text、MainContent=
//!   extract_main_content、Element=CSS Selector §34）。
//! - **调度**：复用 M2.6 Agent Scheduler（job_name = `monitor:<target>`，
//!   dispatch 分支 spawn 线程，不阻塞 tick）——无第二 Scheduler（§23）。
//! - **Diff**：检测走指纹 fast-path（§10）；详细 diff 复用
//!   text_service::compare_text（weave-text 有界 LCS）——无第二 Diff。
//! - **通知**：复用 M2.6 NotificationCenter（§21）。
//! - **持久化**：原子 JSON + payload 文件（recipe/clipboard 范式 §27）。
//! - 失败不污染成功快照（§15）；首次 = Initial（§14）；per-target 执行
//!   锁 + 全局有界并发（§25/§48）；失败退避（§18）。

pub mod check;
pub mod model;
pub mod normalize;
pub mod store;

pub use check::{build_event, check_target, extract_by_mode};
pub use model::{
    ChangeMagnitude, CheckResult, MIN_INTERVAL_SECS, MonitorChangeEvent, MonitorMode,
    MonitorSnapshot, MonitorTarget, TargetStatus, backoff_delay_secs, magnitude_for, summarize,
};
pub use normalize::{fingerprint, normalize_text};
pub use store::{MAX_EVENTS, MAX_SNAPSHOTS_PER_TARGET, MonitorStore, valid_target_id};
