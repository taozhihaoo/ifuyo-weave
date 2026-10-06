//! 有界后台队列 + 并发调度（M2.5 §15/§16/§17：无界队列/无限并发 = P0）。
//!
//! - 队列容量有界；溢出策略 DropOldest（后台类）或 Reject（用户显式任务）。
//! - 优先级：Interactive > Normal > Background（Interactive 不被后台挤占）。
//! - MaxConcurrentJobs 全局有界（默认 4）。
//! - 全部经 Mutex——与项目其余部分（JobTracker 等）同一并发原语。

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// 任务优先级（§16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Maintenance,
    Background,
    Normal,
    Interactive,
}

/// 溢出策略（§15）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    DropOldest,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnqueueOutcome {
    Queued,
    DroppedOldest,
    Rejected,
}

struct QueuedJob {
    priority: JobPriority,
    job_id: String,
}

/// 有界优先级队列（注意：PartialOrd 排序 High 优先）。
pub struct BoundedJobQueue {
    queue: Mutex<VecDeque<QueuedJob>>,
    capacity: usize,
    dropped_total: AtomicU64,
}

impl BoundedJobQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
            dropped_total: AtomicU64::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.queue.lock().expect("queue").len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().expect("queue").is_empty()
    }

    /// 入队（优先级降序；Interactive 挤到最前）。溢出按策略。
    pub fn enqueue(
        &self,
        job_id: impl Into<String>,
        priority: JobPriority,
        overflow: OverflowPolicy,
    ) -> EnqueueOutcome {
        let mut q = self.queue.lock().expect("queue");
        let mut dropped = false;
        if q.len() >= self.capacity {
            match overflow {
                OverflowPolicy::DropOldest => {
                    q.pop_front();
                    self.dropped_total.fetch_add(1, Ordering::SeqCst);
                    dropped = true;
                }
                OverflowPolicy::Reject => {
                    self.dropped_total.fetch_add(1, Ordering::SeqCst);
                    return EnqueueOutcome::Rejected;
                }
            }
        }
        let item = QueuedJob {
            priority,
            job_id: job_id.into(),
        };
        match priority {
            // Interactive 从前面插（下一次就跑）；其余按序入队尾
            JobPriority::Interactive => q.push_front(item),
            _ => q.push_back(item),
        }
        if dropped {
            EnqueueOutcome::DroppedOldest
        } else {
            EnqueueOutcome::Queued
        }
    }

    /// 出队最高优先级任务（dispatcher 调用）。
    pub fn dequeue(&self) -> Option<(JobPriority, String)> {
        let mut q = self.queue.lock().expect("queue");
        q.pop_front().map(|j| (j.priority, j.job_id))
    }

    pub fn dropped_total(&self) -> u64 {
        self.dropped_total.load(Ordering::SeqCst)
    }
}

/// 并发闸门（§17：MaxConcurrentJobs 有界；计数即真实占用）。
pub struct ConcurrencyGate {
    max: usize,
    running: Mutex<usize>,
}

impl ConcurrencyGate {
    pub fn new(max: usize) -> Self {
        Self {
            max: max.max(1),
            running: Mutex::new(0),
        }
    }

    /// 尝试占用一个槽位；满 ⇒ false（调用方排队或丢弃）。
    pub fn try_acquire(&self) -> bool {
        let mut r = self.running.lock().expect("gate");
        if *r >= self.max {
            return false;
        }
        *r += 1;
        true
    }

    pub fn release(&self) {
        let mut r = self.running.lock().expect("gate");
        *r = r.saturating_sub(1);
    }

    pub fn running(&self) -> usize {
        *self.running.lock().expect("gate")
    }

    pub fn max(&self) -> usize {
        self.max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_is_bounded_and_drop_oldest_works() {
        let q = BoundedJobQueue::new(3);
        for i in 0..3 {
            assert_eq!(
                q.enqueue(
                    format!("j{i}"),
                    JobPriority::Background,
                    OverflowPolicy::DropOldest
                ),
                EnqueueOutcome::Queued
            );
        }
        // 满 ⇒ DropOldest（j0 被挤出）
        assert_eq!(
            q.enqueue("j3", JobPriority::Background, OverflowPolicy::DropOldest),
            EnqueueOutcome::DroppedOldest
        );
        assert_eq!(q.len(), 3);
        let (p, id) = q.dequeue().expect("job");
        assert_eq!((p, id.as_str()), (JobPriority::Background, "j1"));
        assert_eq!(q.dropped_total(), 1);
    }

    #[test]
    fn queue_reject_policy_keeps_existing() {
        let q = BoundedJobQueue::new(2);
        q.enqueue("a", JobPriority::Normal, OverflowPolicy::Reject);
        q.enqueue("b", JobPriority::Normal, OverflowPolicy::Reject);
        assert_eq!(
            q.enqueue("c", JobPriority::Normal, OverflowPolicy::Reject),
            EnqueueOutcome::Rejected
        );
        assert_eq!(q.len(), 2);
        assert_eq!(q.dropped_total(), 1);
    }

    #[test]
    fn interactive_priority_jumps_ahead() {
        let q = BoundedJobQueue::new(10);
        q.enqueue("bg1", JobPriority::Background, OverflowPolicy::DropOldest);
        q.enqueue("bg2", JobPriority::Background, OverflowPolicy::DropOldest);
        q.enqueue("user", JobPriority::Interactive, OverflowPolicy::DropOldest);
        let (_, id) = q.dequeue().expect("job");
        assert_eq!(id, "user", "Interactive 优先（§16）");
    }

    #[test]
    fn concurrency_gate_bounds_and_releases() {
        let gate = ConcurrencyGate::new(2);
        assert!(gate.try_acquire());
        assert!(gate.try_acquire());
        assert!(!gate.try_acquire(), "满 ⇒ 拒绝");
        gate.release();
        assert!(gate.try_acquire());
        assert_eq!(gate.running(), 2);
        gate.release();
        gate.release();
        assert_eq!(gate.running(), 0);
    }
}
