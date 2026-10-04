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

#[derive(Default)]
pub struct JobTracker {
    jobs: Mutex<HashMap<String, JobEntry>>,
}

impl JobTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个新任务；返回 (job_id, cancel token, 进度 sink, 状态共享句柄)。
    pub fn register(&self) -> (JobId, CancellationToken, ProgressSink, Arc<Mutex<JobState>>) {
        let id = JobId::generate();
        let cancel = CancellationToken::new();
        let state = Arc::new(Mutex::new(JobState::Running {
            progress_current: 0,
        }));
        self.jobs.lock().expect("job map").insert(
            id.to_string(),
            JobEntry {
                cancel: cancel.clone(),
                state: Arc::clone(&state),
            },
        );
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
