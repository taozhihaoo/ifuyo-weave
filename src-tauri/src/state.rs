//! 应用状态（M0 §40 / M1）。
//!
//! M1 增加：轻量任务跟踪（进度/取消 IPC 管道，非 Batch Engine）与
//! Tool Registry（统一工具发现）。settings / history / operation manager
//! 由后续里程碑按需加入。

use crate::jobs::JobTracker;
use crate::logging::LogGuard;
use crate::rename_service::PlanCache;
use weave_core::prelude::ToolRegistry;

pub struct AppState {
    /// 保持 tracing 非阻塞写入器的 flush worker 存活到进程结束。
    _log_guard: LogGuard,
    pub jobs: JobTracker,
    pub tools: ToolRegistry,
    pub plans: PlanCache,
}

impl AppState {
    pub fn new(log_guard: LogGuard, tools: ToolRegistry) -> Self {
        Self {
            _log_guard: log_guard,
            jobs: JobTracker::new(),
            tools,
            plans: PlanCache::new(),
        }
    }
}
