//! 应用状态（M0 §40 / M1）。
//!
//! M1 增加：轻量任务跟踪（进度/取消 IPC 管道，非 Batch Engine）与
//! Tool Registry（统一工具发现）。settings / history / operation manager
//! 由后续里程碑按需加入。

use std::collections::HashMap;
use std::sync::Mutex;

use crate::batch_service::BatchJobRecord;
use crate::data_service::DataSessions;
use crate::duplicates_service::ScanCache;
use crate::jobs::JobTracker;
use crate::logging::LogGuard;
use crate::rename_service::PlanCache;
use crate::text_service::TextWriteCache;
use weave_core::prelude::ToolRegistry;

pub struct AppState {
    /// 保持 tracing 非阻塞写入器的 flush worker 存活到进程结束。
    _log_guard: LogGuard,
    pub jobs: JobTracker,
    pub tools: ToolRegistry,
    pub plans: PlanCache,
    /// M3：重复扫描结果缓存（scan_id 为键，build_recycle_plan 的衔接点）。
    pub scans: ScanCache,
    /// M4：文本写回内容缓存（operation_id 为键）。
    pub text_writes: TextWriteCache,
    /// M5：数据会话（ephemeral，§17）。
    pub data_sessions: DataSessions,
    /// M7（下）：Batch Job 登记（job_id → 计划/剩余子集/pause/冲突）。
    pub batch_jobs: Mutex<HashMap<String, BatchJobRecord>>,
    /// M7（下 §202/§204）：目的地互斥——canonical dest dir → job_id。
    /// 同一目标目录同时只允许一个 Batch Job（策略 = 拒绝，不排队）。
    pub batch_dest_locks: Mutex<HashMap<String, String>>,
    /// M8（下 §141）：merge plan 服务端缓存（preview 建、execute 用——
    /// TOCTOU 重校验依据；execute 取走即删，失败放回）。
    pub document_plans: Mutex<HashMap<String, weave_documents::PdfMergePlan>>,
}

impl AppState {
    pub fn new(log_guard: LogGuard, tools: ToolRegistry) -> Self {
        Self {
            _log_guard: log_guard,
            jobs: JobTracker::new(),
            tools,
            plans: PlanCache::new(),
            scans: ScanCache::new(),
            text_writes: TextWriteCache::new(),
            data_sessions: DataSessions::new(),
            batch_jobs: Mutex::new(HashMap::new()),
            batch_dest_locks: Mutex::new(HashMap::new()),
            document_plans: Mutex::new(HashMap::new()),
        }
    }
}
