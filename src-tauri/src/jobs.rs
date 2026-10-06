//! 轻量任务跟踪（M1 §31/§32：进度 + 协作取消的 IPC 管道）。
//!
//! 边界声明（防 M7 偷渡）：这是 IPC 层的任务管道，不是 Batch Engine——
//! 没有队列、没有重试、没有持久化、没有并发调度策略；每个任务 =
//! CancellationToken + 共享状态。M7 将统一为 Job 模型（charter #10.1）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use weave_core::prelude::{CancellationToken, JobId, Progress, WeaveError};

use crate::commands::IpcError;
use crate::files_dto::{HashResultDto, JobStatusDto, ScanReportDto};
use serde::Serialize;
use specta::Type;

/// 任务摘要（Job Center 列表行；仅元数据 §11）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JobSummaryDto {
    pub job_id: String,
    /// running | completed | failed
    pub state: String,
    pub progress_current: Option<f64>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone)]
pub enum JobOutcome {
    Hash(HashResultDto),
    /// Boxed：Scan 报告远大于 Hash 结果（clippy::large_enum_variant）。
    Scan(Box<ScanReportDto>),
    /// M2：Rename/Organizer 执行（含事务供 UI 展示 undo 能力）。
    PlanExecuted {
        report: Box<crate::ops_dto::ExecutionReportDto>,
        transaction: Option<Box<crate::ops_dto::TransactionDto>>,
    },
    /// M2：Undo 结果计数。
    Undo {
        restored: f64,
        conflicts: f64,
        leaked_temps: f64,
    },
    /// M3：重复扫描报告（Boxed：报告远大于计数）。
    DuplicateScan(Box<crate::files_dto::DuplicateScanReportDto>),
    /// M3：回收执行（复用 PlanReportDto：executed = recycled）。
    RecycleExecuted(Box<crate::ops_dto::PlanReportDto>),
    /// M4：文本写回（复用 PlanReportDto：executed = 1）。
    TextExecuted(Box<crate::ops_dto::PlanReportDto>),
    /// M7：Batch Job 结果（JobPlan 同引擎 Execute）。
    BatchExecuted(Box<crate::batch_service::BatchJobResultDto>),
    /// M2.1：目录比较完成（结果经 CompareResultCache 领取，终态仅登记）。
    CompareCompleted,
    /// M2.2：空间扫描完成（结果经 SpaceResultCache 领取，终态仅登记）。
    SpaceCompleted,
    /// M2.4：隐私扫描完成（结果经 PrivacyResultCache 领取，终态仅登记）。
    PrivacyCompleted,
    /// M2.5：归档/解压完成（结果经 ArchiveResultCache 领取，终态仅登记）。
    ArchiveCompleted,
    /// M2.10：Web 抓取/批量完成（结果经 WebRuntime 领取，终态仅登记）。
    WebCompleted,
}

#[derive(Debug)]
pub enum JobState {
    Running { progress_current: u64 },
    Done(Box<JobOutcome>),
    Failed(IpcError),
}

#[derive(Clone)]
pub struct JobEntry {
    pub cancel: CancellationToken,
    pub state: Arc<Mutex<JobState>>,
}

/// 进度回调：把当前计数写进共享状态（每 chunk 一次，开销可忽略）。
pub struct ProgressSink {
    state: Arc<Mutex<JobState>>,
}

impl ProgressSink {
    pub fn report(&self, progress: &Progress) {
        if let Ok(mut guard) = self.state.lock()
            && matches!(&*guard, JobState::Running { .. })
        {
            *guard = JobState::Running {
                progress_current: progress.current,
            };
        }
    }
}

/// 任务登记容量上限（M2.13 §61：Job Center 不无限累积——超过即淘汰
/// 最旧的终态条目；Running 条目永不被淘汰）。
pub const MAX_TRACKED_JOBS: usize = 200;

#[derive(Default)]
pub struct JobTracker {
    jobs: Mutex<HashMap<String, JobEntry>>,
    /// 登记顺序（FIFO 淘汰依据）。
    order: Mutex<std::collections::VecDeque<String>>,
}

impl JobTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 终态判定（淘汰只针对终态条目；Running 永留）。
    fn is_terminal(state: &JobState) -> bool {
        matches!(state, JobState::Done(_) | JobState::Failed(_))
    }

    /// 淘汰最旧的终态条目，直到总量 ≤ cap。
    fn evict_locked(
        map: &mut HashMap<String, JobEntry>,
        order: &mut std::collections::VecDeque<String>,
        cap: usize,
    ) {
        while map.len() > cap {
            // 找到最早的终态条目
            let Some(victim) = order
                .iter()
                .find(|id| {
                    map.get(*id)
                        .is_some_and(|e| Self::is_terminal(&e.state.lock().expect("job state")))
                })
                .cloned()
            else {
                break; // 全是 Running——不淘汰
            };
            if map.remove(&victim).is_some() {
                order.retain(|id| id != &victim);
            } else {
                order.retain(|id| id != &victim);
                break;
            }
        }
    }

    /// 注册一个新任务；返回 (job_id, cancel token, 进度 sink, 状态共享句柄)。
    pub fn register(&self) -> (JobId, CancellationToken, ProgressSink, Arc<Mutex<JobState>>) {
        let id = JobId::generate();
        let cancel = CancellationToken::new();
        let state = Arc::new(Mutex::new(JobState::Running {
            progress_current: 0,
        }));
        {
            let mut map = self.jobs.lock().expect("job map");
            map.insert(
                id.to_string(),
                JobEntry {
                    cancel: cancel.clone(),
                    state: Arc::clone(&state),
                },
            );
            let mut order = self.order.lock().expect("job order");
            order.push_back(id.to_string());
            Self::evict_locked(&mut map, &mut order, MAX_TRACKED_JOBS);
        }
        (
            id,
            cancel,
            ProgressSink {
                state: Arc::clone(&state),
            },
            state,
        )
    }

    pub fn finish(&self, job_id: &JobId, outcome: JobOutcome) {
        if let Some(entry) = self.jobs.lock().expect("job map").get(&job_id.to_string()) {
            *entry.state.lock().expect("job state") = JobState::Done(Box::new(outcome));
        }
    }

    pub fn fail(&self, job_id: &JobId, error: WeaveError) {
        if let Some(entry) = self.jobs.lock().expect("job map").get(&job_id.to_string()) {
            *entry.state.lock().expect("job state") = JobState::Failed(error.into());
        }
    }

    /// 失败但 DTO 层错误（如输入校验在任务内失败）。
    pub fn fail_ipc(&self, job_id: &JobId, error: IpcError) {
        if let Some(entry) = self.jobs.lock().expect("job map").get(&job_id.to_string()) {
            *entry.state.lock().expect("job state") = JobState::Failed(error);
        }
    }

    pub fn status(&self, job_id: &str) -> Option<JobStatusDto> {
        let entry = {
            let map = self.jobs.lock().expect("job map");
            map.get(job_id)?.clone()
        };
        let guard = entry.state.lock().expect("job state");
        let (state, progress_current, hash, scan, plan, undo, duplicate_scan, batch, error) =
            match &*guard {
                JobState::Running { progress_current } => (
                    "running",
                    Some(*progress_current as f64),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
                JobState::Done(outcome) => match &**outcome {
                    JobOutcome::Hash(hash) => (
                        "completed",
                        None,
                        Some(hash.clone()),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                    JobOutcome::Scan(scan) => (
                        "completed",
                        None,
                        None,
                        Some((**scan).clone()),
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                    JobOutcome::DuplicateScan(report) => (
                        "completed",
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some((**report).clone()),
                        None,
                        None,
                    ),
                    JobOutcome::CompareCompleted => {
                        ("completed", None, None, None, None, None, None, None, None)
                    }
                    JobOutcome::SpaceCompleted => {
                        ("completed", None, None, None, None, None, None, None, None)
                    }
                    JobOutcome::PrivacyCompleted => {
                        ("completed", None, None, None, None, None, None, None, None)
                    }
                    JobOutcome::ArchiveCompleted => {
                        ("completed", None, None, None, None, None, None, None, None)
                    }
                    JobOutcome::WebCompleted => {
                        ("completed", None, None, None, None, None, None, None, None)
                    }
                    JobOutcome::TextExecuted(report) => (
                        "completed",
                        None,
                        None,
                        None,
                        Some((**report).clone()),
                        None,
                        None,
                        None,
                        None,
                    ),
                    JobOutcome::RecycleExecuted(report) => (
                        "completed",
                        None,
                        None,
                        None,
                        Some((**report).clone()),
                        None,
                        None,
                        None,
                        None,
                    ),
                    JobOutcome::BatchExecuted(batch_result) => (
                        "completed",
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some((**batch_result).clone()),
                        None,
                    ),
                    JobOutcome::PlanExecuted {
                        report,
                        transaction,
                    } => (
                        "completed",
                        None,
                        None,
                        None,
                        Some(crate::ops_dto::PlanReportDto {
                            operation_id: report.operation_id.clone(),
                            items: report.items.clone(),
                            executed: report.executed,
                            failed: report.failed,
                            skipped: report.skipped,
                            undoable: report.undoable,
                            duration_ms: report.duration_ms,
                            transaction: transaction.as_deref().cloned(),
                        }),
                        None,
                        None,
                        None,
                        None,
                    ),
                    JobOutcome::Undo {
                        restored,
                        conflicts,
                        leaked_temps,
                    } => (
                        "completed",
                        None,
                        None,
                        None,
                        Some(crate::ops_dto::PlanReportDto {
                            operation_id: String::new(),
                            items: Vec::new(),
                            executed: *restored,
                            failed: *conflicts,
                            skipped: *leaked_temps,
                            undoable: false,
                            duration_ms: 0.0,
                            transaction: None,
                        }),
                        None,
                        None,
                        None,
                        None,
                    ),
                },
                JobState::Failed(err) => (
                    "failed",
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(err.clone()),
                ),
            };
        Some(JobStatusDto {
            job_id: job_id.to_string(),
            state: state.to_string(),
            progress_current,
            hash,
            scan,
            plan,
            undo,
            duplicate_scan,
            batch,
            error,
        })
    }

    /// 任务摘要列表（M2.13 Job Center §11）：Running 优先，其后按登记
    /// 顺序倒序（最新在前）的终态条目；只含元数据（id/state/进度/错误码）。
    pub fn list_jobs(&self) -> Vec<JobSummaryDto> {
        let map = self.jobs.lock().expect("job map");
        let order = self.order.lock().expect("job order");
        let mut running = Vec::new();
        let mut terminal = Vec::new();
        for id in order.iter().rev() {
            let Some(entry) = map.get(id) else { continue };
            let guard = entry.state.lock().expect("job state");
            let (state, progress) = match &*guard {
                JobState::Running { progress_current } => {
                    ("running", Some(*progress_current as f64))
                }
                JobState::Done(_) => ("completed", None),
                JobState::Failed(e) => ("failed", None),
            };
            let error_code = if let JobState::Failed(e) = &*guard {
                Some(e.code.clone())
            } else {
                None
            };
            let item = JobSummaryDto {
                job_id: id.clone(),
                state: state.to_string(),
                progress_current: progress,
                error_code,
            };
            if state == "running" {
                running.push(item);
            } else {
                terminal.push(item);
            }
        }
        running.extend(terminal);
        running
    }

    pub fn cancel(&self, job_id: &str) -> bool {
        self.jobs
            .lock()
            .expect("job map")
            .get(job_id)
            .map(|entry| {
                entry.cancel.cancel();
                true
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_evicts_oldest_terminal_but_never_running() {
        let tracker = JobTracker::new();
        // 注册 3 个并全部完成
        let mut done_ids = Vec::new();
        for _ in 0..3 {
            let (id, _cancel, _sink, _state) = tracker.register();
            tracker.finish(&id, JobOutcome::WebCompleted);
            done_ids.push(id.to_string());
        }
        // 再注册 1 个保持 Running
        let (run_id, _cancel, _sink, _state) = tracker.register();
        // 收紧容量到 2 ⇒ 最旧终态被淘汰，Running 保留
        {
            let mut map = tracker.jobs.lock().expect("map");
            let mut order = tracker.order.lock().expect("order");
            JobTracker::evict_locked(&mut map, &mut order, 2);
            assert_eq!(map.len(), 2);
        }
        let list = tracker.list_jobs();
        assert_eq!(list.len(), 2);
        assert!(
            list.iter().any(|j| j.job_id == run_id.to_string()),
            "Running 永不淘汰"
        );
        assert!(
            !list.iter().any(|j| j.job_id == done_ids[0]),
            "最旧终态被淘汰"
        );
    }

    #[test]
    fn list_orders_running_first_and_carries_error_code() {
        let tracker = JobTracker::new();
        let (failed_id, _c, _s, _st) = tracker.register();
        tracker.fail_ipc(
            &failed_id,
            crate::commands::IpcError {
                kind: "io".into(),
                code: "test.failed".into(),
                message: "boom".into(),
                location: None,
                recoverability: "retryable".into(),
                suggestion: None,
            },
        );
        let (run_id, _c2, _s2, _st2) = tracker.register();
        let list = tracker.list_jobs();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].job_id, run_id.to_string(), "Running 优先");
        let failed = list
            .iter()
            .find(|j| j.job_id == failed_id.to_string())
            .expect("failed");
        assert_eq!(failed.state, "failed");
        assert_eq!(failed.error_code.as_deref(), Some("test.failed"));
    }

    #[test]
    fn cancel_marks_running_then_terminal() {
        let tracker = JobTracker::new();
        let (id, cancel, _sink, state) = tracker.register();
        assert!(tracker.cancel(&id.to_string()));
        assert!(cancel.is_cancelled());
        // 状态仍是 Running 直到任务体自行退出（真实语义，不伪造）
        let guard = state.lock().expect("state");
        assert!(matches!(&*guard, JobState::Running { .. }));
    }
}
