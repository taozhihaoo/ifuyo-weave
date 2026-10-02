//! 应用状态（M0 §40）。
//!
//! M0 只放真正需要的字段（日志 flush guard）。settings / registries /
//! operation manager / history 由后续里程碑按需加入，禁止提前塞入未来系统。

use crate::logging::LogGuard;

pub struct AppState {
    /// 保持 tracing 非阻塞写入器的 flush worker 存活到进程结束。
    _log_guard: LogGuard,
}

impl AppState {
    pub fn new(log_guard: LogGuard) -> Self {
        Self {
            _log_guard: log_guard,
        }
    }
}
