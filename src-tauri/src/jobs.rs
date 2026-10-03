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
    Scan(ScanReportDto),
}

#[derive(Debug)]
pub enum JobState {
    Running { progress_current: u64 },
    Done(JobOutcome),
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
        if let Ok(mut guard) = self.state.lock() {
            if matches!(&*guard, JobState::Running { .. }) {
                *guard = JobState::Running {
                    progress_current: progress.current,
                };
            }
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
            *entry.state.lock().expect("job state") = JobState::Done(outcome);
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
        let (state, progress_current, hash, scan, error) = match &*guard {
            JobState::Running { progress_current } => {
                ("running", Some(*progress_current as f64), None, None, None)
            }
            JobState::Done(JobOutcome::Hash(hash)) => {
                ("completed", None, Some(hash.clone()), None, None)
            }
            JobState::Done(JobOutcome::Scan(scan)) => {
                ("completed", None, None, Some(scan.clone()), None)
            }
            JobState::Failed(err) => ("failed", None, None, None, Some(err.clone())),
        };
        Some(JobStatusDto {
            job_id: job_id.to_string(),
            state: state.to_string(),
            progress_current,
            hash,
            scan,
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
