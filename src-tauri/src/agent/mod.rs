//! Agent Runtime（M2.6 §4/§5/§7/§8）：后台生命周期的唯一主人。
//!
//! - Start/Stop/Pause/Resume 幂等（§8）且经状态机校验（§4）。
//! - Clipboard Component 正式注册（§27：页面关闭不影响监听；Agent 停止时
//!   正确暂停/恢复 M2.3 的捕获）。
//! - Scheduler tick 驱动（§12）+ 健康快照（§20）。
//! - 组件隔离（§19）：Clipboard Faulted 不影响 Scheduler。

pub mod queue;
pub mod runtime;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use weave_core::prelude::WeaveError;

pub use queue::{BoundedJobQueue, ConcurrencyGate, EnqueueOutcome, JobPriority, OverflowPolicy};
pub use runtime::{
    AgentState, ConcurrencyPolicy, MissedRunPolicy, ScheduleDef, ScheduleDue, ScheduleKind,
    evaluate_missed, next_run_at,
};

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::validation(code, message).with_location("agent::runtime")
}

/// 组件健康（§20：可回答"哪些正常/异常/最后成功/最后失败"）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentHealth {
    pub name: String,
    /// running | paused | stopped | faulted
    pub state: String,
    pub last_error: Option<String>,
    pub last_success: Option<SystemTime>,
}

/// 组件状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentState {
    Running,
    Paused,
    Stopped,
    Faulted,
}

impl ComponentState {
    pub fn as_str(self) -> &'static str {
        match self {
            ComponentState::Running => "running",
            ComponentState::Paused => "paused",
            ComponentState::Stopped => "stopped",
            ComponentState::Faulted => "faulted",
        }
    }
}

/// Agent 组件 trait（§6：只有实际存在多实现时才抽象——Clipboard 与
/// Scheduler 均实现此 trait，抽象有真实边界）。
pub trait AgentComponent: Send {
    fn name(&self) -> &'static str;
    /// 启动组件（可重复调用 = No-op，由 Runtime 幂等保护）。
    fn start(&mut self);
    /// 暂停（保持进程内状态，但不再消费事件）。
    fn pause(&mut self);
    /// 恢复。
    fn resume(&mut self);
    /// 停止并释放资源（可重复调用 = No-op）。
    fn stop(&mut self);
    fn health(&self) -> ComponentHealth;
}

/// Scheduler 组件：持有调度定义 + 运行态（running 标记 = SkipIfRunning 依据）。
#[derive(Default)]
pub struct SchedulerComponent {
    pub schedules: Vec<ScheduleDef>,
    pub next_run: Vec<(String, SystemTime)>,
    pub running_jobs: Vec<String>,
    pub state: Option<ComponentState>,
    pub last_error: Option<String>,
    pub last_success: Option<SystemTime>,
}

impl SchedulerComponent {
    pub fn upsert(&mut self, def: ScheduleDef) {
        match self.schedules.iter_mut().find(|s| s.id == def.id) {
            Some(existing) => *existing = def,
            None => self.schedules.push(def),
        }
        self.recompute_next();
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.schedules.len();
        self.schedules.retain(|s| s.id != id);
        self.recompute_next();
        self.schedules.len() != before
    }

    pub fn recompute_next(&mut self) {
        let now = SystemTime::now();
        self.next_run = self
            .schedules
            .iter()
            .filter(|s| s.enabled)
            .map(|s| {
                (
                    s.id.clone(),
                    runtime::next_run_at(&s.kind, now, s.last_run_at)
                        .unwrap_or(now + std::time::Duration::from_secs(3600)),
                )
            })
            .collect();
    }

    /// 到期扫描：更新 last_run/next_run 并返回到期任务。
    pub fn collect_due(&mut self, now: SystemTime) -> Vec<ScheduleDue> {
        let mut due = Vec::new();
        for def in &mut self.schedules {
            if !def.enabled {
                continue;
            }
            let next = runtime::next_run_at(&def.kind, now, def.last_run_at);
            if let Some(next_at) = next {
                if next_at <= now {
                    if def.concurrency == ConcurrencyPolicy::SkipIfRunning
                        && self.running_jobs.contains(&def.id)
                    {
                        continue; // §14 SkipIfRunning：上次未完成 ⇒ 跳过
                    }
                    def.last_run_at = Some(now);
                    due.push(ScheduleDue {
                        id: def.id.clone(),
                        job_name: def.job_name.clone(),
                        is_catch_up: false,
                    });
                }
            }
            def.next_run_at_cached = runtime::next_run_at(&def.kind, now, def.last_run_at);
        }
        // 刷新缓存
        self.next_run = self
            .schedules
            .iter()
            .map(|s| {
                (
                    s.id.clone(),
                    s.next_run_at_cached
                        .unwrap_or_else(|| now + std::time::Duration::from_secs(3600)),
                )
            })
            .collect();
        due
    }

    pub fn set_running(&mut self, id: &str, running: bool) {
        if running {
            if !self.running_jobs.iter().any(|j| j == id) {
                self.running_jobs.push(id.to_string());
            }
        } else {
            self.running_jobs.retain(|j| j != id);
        }
    }
}

/// Clipboard 组件：代理 M2.3 的运行时标志（capture paused）。
pub struct ClipboardComponent {
    pub paused_flag: std::sync::Arc<AtomicBool>,
    pub enabled_flag: std::sync::Arc<AtomicBool>,
    pub state: Option<ComponentState>,
}

impl AgentComponent for ClipboardComponent {
    fn name(&self) -> &'static str {
        "clipboard"
    }

    fn start(&mut self) {
        self.enabled_flag.store(true, Ordering::SeqCst);
        self.paused_flag.store(false, Ordering::SeqCst);
        self.state = Some(ComponentState::Running);
    }

    fn pause(&mut self) {
        self.paused_flag.store(true, Ordering::SeqCst);
        self.state = Some(ComponentState::Paused);
    }

    fn resume(&mut self) {
        self.paused_flag.store(false, Ordering::SeqCst);
        self.state = Some(ComponentState::Running);
    }

    fn stop(&mut self) {
        self.paused_flag.store(true, Ordering::SeqCst);
        self.state = Some(ComponentState::Stopped);
    }

    fn health(&self) -> ComponentHealth {
        ComponentHealth {
            name: self.name().to_string(),
            state: self
                .state
                .map(|s| s.as_str())
                .unwrap_or("stopped")
                .to_string(),
            last_error: None,
            last_success: None,
        }
    }
}

/// Agent 运行时（AppState 持有；全部经 Mutex——Start/Stop 幂等 §8）。
pub struct ClipboardComponentState {
    pub inner: Mutex<crate::clipboard_component::ClipboardComponent>,
}

pub struct AgentRuntime {
    state: Mutex<AgentState>,
    pub paused: AtomicBool,
    pub scheduler: Mutex<SchedulerComponent>,
    pub clipboard: ClipboardComponentState,
    pub queue: BoundedJobQueue,
    pub gate: ConcurrencyGate,
    pub started_at: Mutex<Option<SystemTime>>,
    pub last_error: Mutex<Option<String>>,
    tick_count: AtomicU64,
}

impl AgentRuntime {
    pub fn new(max_concurrent: usize, queue_capacity: usize) -> Self {
        Self {
            state: Mutex::new(AgentState::Stopped),
            paused: AtomicBool::new(false),
            scheduler: Mutex::new(SchedulerComponent::default()),
            clipboard: ClipboardComponentState {
                inner: Mutex::new(crate::clipboard_component::ClipboardComponent::new()),
            },
            queue: BoundedJobQueue::new(queue_capacity),
            gate: ConcurrencyGate::new(max_concurrent),
            started_at: Mutex::new(None),
            last_error: Mutex::new(None),
            tick_count: AtomicU64::new(0),
        }
    }

    fn transition(&self, to: AgentState) -> Result<(), WeaveError> {
        let mut state = self.state.lock().expect("agent state");
        if *state == to {
            return Ok(()); // 幂等（§8）
        }
        if !state.can_transition(to) {
            return Err(err(
                "agent.invalidTransition",
                format!("illegal transition {:?} → {to:?}", *state),
            ));
        }
        *state = to;
        Ok(())
    }

    /// 启动（幂等：已 Running ⇒ No-op §8）。启动时暂停被清除。
    pub fn start(&self) -> Result<(), WeaveError> {
        self.transition(AgentState::Starting)?;
        self.paused.store(false, Ordering::SeqCst);
        *self.started_at.lock().expect("started") = Some(SystemTime::now());
        *self.last_error.lock().expect("err") = None;
        self.transition(AgentState::Running)
    }

    /// 暂停（幂等：已 Paused ⇒ No-op）。暂停 ⇒ 新触发不再启动。
    pub fn pause(&self) -> Result<(), WeaveError> {
        self.transition(AgentState::Pausing)?;
        self.paused.store(true, Ordering::SeqCst);
        self.transition(AgentState::Paused)
    }

    /// 恢复（幂等）。
    pub fn resume(&self) -> Result<(), WeaveError> {
        self.transition(AgentState::Running)
    }

    /// 停止（幂等：已 Stopped ⇒ No-op）。
    pub fn stop(&self) -> Result<(), WeaveError> {
        self.transition(AgentState::Stopping)?;
        self.paused.store(true, Ordering::SeqCst);
        self.transition(AgentState::Stopped)
    }

    pub fn state(&self) -> AgentState {
        *self.state.lock().expect("agent state")
    }

    pub fn is_running(&self) -> bool {
        self.state() == AgentState::Running
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count.load(Ordering::SeqCst)
    }

    pub fn record_tick(&self) {
        self.tick_count.fetch_add(1, Ordering::SeqCst);
    }

    /// 降级记录（§19：可解释的 Degraded）。
    pub fn record_error(&self, message: impl Into<String>) {
        *self.last_error.lock().expect("err") = Some(message.into());
    }

    /// 健康快照（§20）。
    pub fn health(&self) -> AgentHealth {
        let scheduler = self.scheduler.lock().expect("sched");
        AgentHealth {
            state: self.state(),
            paused: self.paused.load(Ordering::SeqCst),
            queue_len: self.queue.len(),
            queue_capacity: self.queue.capacity(),
            queue_dropped: self.queue.dropped_total(),
            gate_running: self.gate.running(),
            gate_max: self.gate.max(),
            scheduler_schedules: scheduler.schedules.len() as u64,
            scheduler_running: scheduler.running_jobs.len() as u64,
            last_error: self.last_error.lock().expect("err").clone(),
            components: vec![
                scheduler
                    .state
                    .map(|s| ComponentHealth {
                        name: "scheduler".to_string(),
                        state: s.as_str().to_string(),
                        last_error: scheduler.last_error.clone(),
                        last_success: scheduler.last_success,
                    })
                    .unwrap_or(ComponentHealth {
                        name: "scheduler".to_string(),
                        state: "stopped".to_string(),
                        last_error: None,
                        last_success: None,
                    }),
                ComponentHealth {
                    name: "clipboard".to_string(),
                    state: self
                        .clipboard
                        .inner
                        .lock()
                        .expect("clip")
                        .health()
                        .state
                        .clone(),
                    last_error: None,
                    last_success: None,
                },
            ],
        }
    }
}

/// Agent 健康快照（§20）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHealth {
    pub state: AgentState,
    pub paused: bool,
    pub queue_len: usize,
    pub queue_capacity: usize,
    pub queue_dropped: u64,
    pub gate_running: usize,
    pub gate_max: usize,
    pub scheduler_schedules: u64,
    pub scheduler_running: u64,
    pub last_error: Option<String>,
    pub components: Vec<ComponentHealth>,
}
