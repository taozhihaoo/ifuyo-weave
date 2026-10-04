//! Job 状态机（M7 上 §4/§3）：显式、可校验、经测试。
//!
//! 生命周期（§3）：
//! ```text
//! Created → Planned → Ready → Running → Completed / CompletedWithFailures
//!                                    ↘ Cancelling → Cancelled
//!                                     ↘ Failed
//! ```
//! Running ↔ Paused 双向（Pause/Resume）。禁止模糊状态（done/success）。

/// Job 生命周期状态（§4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Created,
    Planned,
    Ready,
    Running,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    CompletedWithFailures,
    Failed,
}

impl JobState {
    /// 状态迁移校验（§4：explicit + validated）。非法迁移 ⇒ Err。
    pub fn transition(&self, to: JobState) -> Result<JobState, String> {
        let allowed = matches!(
            (self, to),
            (JobState::Created, JobState::Planned)
                | (JobState::Created, JobState::Failed)
                | (JobState::Planned, JobState::Ready)
                | (JobState::Planned, JobState::Failed)
                | (JobState::Ready, JobState::Running)
                | (JobState::Ready, JobState::Cancelled)
                | (JobState::Ready, JobState::Failed)
                | (JobState::Running, JobState::Paused)
                | (JobState::Running, JobState::Cancelling)
                | (JobState::Running, JobState::Completed)
                | (JobState::Running, JobState::CompletedWithFailures)
                | (JobState::Running, JobState::Failed)
                | (JobState::Paused, JobState::Running)
                | (JobState::Paused, JobState::Cancelling)
                | (JobState::Cancelling, JobState::Cancelled)
                | (JobState::Cancelling, JobState::CompletedWithFailures)
        );
        if allowed {
            Ok(to)
        } else {
            Err(format!("illegal job state transition {self:?} → {to:?}"))
        }
    }

    /// 终态（§3：不得再迁移；区分真实执行结果）。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobState::Completed
                | JobState::CompletedWithFailures
                | JobState::Cancelled
                | JobState::Failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_transitions() {
        let s = JobState::Created;
        let s = s.transition(JobState::Planned).expect("ok");
        let s = s.transition(JobState::Ready).expect("ok");
        let s = s.transition(JobState::Running).expect("ok");
        let s = s.transition(JobState::Completed).expect("ok");
        assert!(s.is_terminal());
    }

    #[test]
    fn pause_resume_and_cancel_paths() {
        let s = JobState::Running;
        let s = s.transition(JobState::Paused).expect("ok");
        let s = s.transition(JobState::Running).expect("ok");
        let s = s.transition(JobState::Cancelling).expect("ok");
        let s = s.transition(JobState::Cancelled).expect("ok");
        assert!(s.is_terminal());
        // 终态不可再迁移
        assert!(s.transition(JobState::Running).is_err());
    }

    #[test]
    fn illegal_transitions_rejected() {
        // §4：Created 不得直接 Running（必须先 Planned→Ready）
        assert!(JobState::Created.transition(JobState::Running).is_err());
        // Completed 不得回退
        assert!(JobState::Completed.transition(JobState::Running).is_err());
    }
}
