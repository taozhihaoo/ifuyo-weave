//! Weave Agent Core（M2.6）：后台生命周期的唯一主人。
//!
//! 职责边界（§5/§7/§56）：
//! - **Runtime 只管生命周期/状态/健康**——业务执行走既有 JobTracker +
//!   各 service（无第二套执行引擎 §1）。
//! - **Start/Stop/Pause/Resume 幂等**（§8：重复 Start/Stop = No-op；
//!   并发安全 = 全部经 Mutex 状态机）。
//! - **组件隔离**（§19）：一个组件 Faulted 不影响其他组件；共享依赖致命
//!   故障 ⇒ Agent Degraded/Faulted 并可解释。
//! - **Scheduler**（§12/§13/§14）：Interval/Daily 基于 NextRunAt（无漂移），
//!   MissedRun 策略 Skip|RunOnce（默认 Skip，不补跑风暴），ConcurrencyPolicy
//!   SkipIfRunning（默认）。
//! - **有界队列**（§15）：溢出策略 DropOldest/Reject；Interactive 优先。

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

// ── 状态机（§4）──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentState {
    Stopped,
    Starting,
    Running,
    Pausing,
    Paused,
    Stopping,
    Faulted,
}

impl AgentState {
    /// 状态迁移校验（§4：显式、可观察）。
    pub fn can_transition(&self, to: AgentState) -> bool {
        matches!(
            (self, to),
            (AgentState::Stopped, AgentState::Starting)
                | (AgentState::Starting, AgentState::Running)
                | (AgentState::Starting, AgentState::Faulted)
                | (AgentState::Running, AgentState::Pausing)
                | (AgentState::Running, AgentState::Stopping)
                | (AgentState::Running, AgentState::Faulted)
                | (AgentState::Pausing, AgentState::Paused)
                | (AgentState::Paused, AgentState::Running)
                | (AgentState::Paused, AgentState::Stopping)
                | (AgentState::Stopping, AgentState::Stopped)
                | (AgentState::Faulted, AgentState::Starting) // Recovery §4
        )
    }
}

// ── Scheduler（§12/§13/§36）──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScheduleKind {
    /// 每 N 秒。
    Interval { seconds: u64 },
    /// 每天本地时刻（时:分；本地时区语义 §37）。
    Daily { hour: u8, minute: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MissedRunPolicy {
    /// 跳过错过的运行（默认；不补跑风暴 §13）。
    Skip,
    /// 恢复后最多补跑一次。
    RunOnce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConcurrencyPolicy {
    Allow,
    /// 默认：上次未完成 ⇒ 跳过本次（§14）。
    SkipIfRunning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDef {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub kind: ScheduleKind,
    pub missed_run_policy: MissedRunPolicy,
    pub concurrency: ConcurrencyPolicy,
    /// 作业名（经 Handler 注册表分派——Trigger 与业务解耦 §32）。
    pub job_name: String,
    pub created_at: SystemTime,
    pub last_run_at: Option<SystemTime>,
    pub last_result: Option<String>,
    /// 缓存的下次运行时刻（recompute 后填充；serde default 兼容旧配置）。
    #[serde(default)]
    pub next_run_at_cached: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleDue {
    pub id: String,
    pub job_name: String,
    /// RunOnce 补跑标记（MissedRun = RunOnce 且错过了上次窗口）。
    pub is_catch_up: bool,
}

/// 计算下一次运行时刻（§12：基于 NextRunAt，无延迟累积漂移）。
/// `now` 为注入时钟（§37 FakeClock 测试）。
pub fn next_run_at(
    kind: &ScheduleKind,
    now: SystemTime,
    last_run: Option<SystemTime>,
) -> Option<SystemTime> {
    match kind {
        ScheduleKind::Interval { seconds } => {
            if *seconds == 0 {
                return None; // §59 配置归一化：Interval=0 拒绝
            }
            let interval = Duration::from_secs(*seconds);
            match last_run {
                Some(last) => Some(last + interval), // 基于上次计划点，不基于 now
                None => Some(now + interval),
            }
        }
        ScheduleKind::Daily { hour, minute } => {
            if *hour > 23 || *minute > 59 {
                return None;
            }
            // 以本地日历日的 hh:mm 计算（SystemTime → 本地秒数近似；
            // DST/时钟跳变由"每日重算"自然收敛——每天只跑一次窗口）
            let secs_now = now
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let secs_per_day = 86_400u64;
            let day_start = secs_now - (secs_now % secs_per_day);
            let target_of_day = (*hour as u64) * 3600 + (*minute as u64) * 60;
            let candidate = day_start + target_of_day;
            let candidate_ts = SystemTime::UNIX_EPOCH + Duration::from_secs(candidate);
            // 今天已过（或已跑过）⇒ 明天
            let after_last = last_run.map(|l| candidate_ts > l).unwrap_or(true);
            if candidate_ts > now && after_last {
                Some(candidate_ts)
            } else {
                Some(SystemTime::UNIX_EPOCH + Duration::from_secs(candidate + secs_per_day))
            }
        }
    }
}

/// 判定一次**已错过**的运行（§13）。`missed` 由调用方（scheduler tick）
/// 依据"上次 tick 时间 < next_run_at"判定——本函数只应用策略：
/// Skip ⇒ None（调用方直接推 next）；RunOnce ⇒ Some(is_catch_up=true)。
/// 并发：SkipIfRunning + running ⇒ None（§14）。
pub fn evaluate_missed(def: &ScheduleDef, missed: bool, running: bool) -> Option<ScheduleDue> {
    if !def.enabled || !missed {
        return None;
    }
    if def.concurrency == ConcurrencyPolicy::SkipIfRunning && running {
        return None; // §14：上次未完成 ⇒ 跳过本次
    }
    if def.missed_run_policy != MissedRunPolicy::RunOnce {
        return None; // Skip：错过即跳过，不补跑（§13 防风暴）
    }
    Some(ScheduleDue {
        id: def.id.clone(),
        job_name: def.job_name.clone(),
        is_catch_up: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn state_machine_validates_transitions() {
        assert!(AgentState::Stopped.can_transition(AgentState::Starting));
        assert!(AgentState::Starting.can_transition(AgentState::Running));
        assert!(AgentState::Running.can_transition(AgentState::Pausing));
        assert!(AgentState::Paused.can_transition(AgentState::Running));
        assert!(AgentState::Running.can_transition(AgentState::Faulted));
        // 非法：Stopped 直接 Running；Stopped 直接 Paused
        assert!(!AgentState::Stopped.can_transition(AgentState::Running));
        assert!(!AgentState::Stopped.can_transition(AgentState::Paused));
        // Recovery：Faulted → Starting 允许（§4 有界重试由调用方控制）
        assert!(AgentState::Faulted.can_transition(AgentState::Starting));
    }

    #[test]
    fn interval_next_run_based_on_last_run_not_now() {
        // §12 无漂移：next = last + interval（与 now 无关）
        let kind = ScheduleKind::Interval { seconds: 60 };
        let now = ts(1000);
        let last = ts(1030);
        assert_eq!(next_run_at(&kind, now, Some(last)), Some(ts(1090)));
        // 无 last ⇒ now + interval
        assert_eq!(next_run_at(&kind, now, None), Some(ts(1060)));
        // Interval=0 ⇒ None（§59 配置归一化）
        assert_eq!(
            next_run_at(&ScheduleKind::Interval { seconds: 0 }, now, None),
            None
        );
    }

    #[test]
    fn daily_next_run_skips_past_window() {
        // 一天 86400s；目标 02:00 = 7200s
        let kind = ScheduleKind::Daily { hour: 2, minute: 0 };
        // now = 第一天 03:00（02:00 已过）⇒ 明天 02:00
        let now = ts(3 * 3600);
        let next = next_run_at(&kind, now, None).expect("next");
        let secs = next
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(secs % 86400, 2 * 3600, "明天 02:00");
        assert!(
            secs > now
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }

    #[test]
    fn daily_invalid_hour_rejected() {
        assert_eq!(
            next_run_at(
                &ScheduleKind::Daily {
                    hour: 24,
                    minute: 0
                },
                ts(0),
                None
            ),
            None
        );
    }

    #[test]
    fn missed_run_skipped_or_caught_up_once() {
        let mut def = ScheduleDef {
            id: "s1".to_string(),
            name: "test".to_string(),
            enabled: true,
            kind: ScheduleKind::Interval { seconds: 60 },
            missed_run_policy: MissedRunPolicy::Skip,
            concurrency: ConcurrencyPolicy::SkipIfRunning,
            job_name: "noop".to_string(),
            created_at: ts(0),
            last_run_at: None,
            last_result: None,
            next_run_at_cached: None,
        };
        // Skip 策略：错过 ⇒ 不补跑
        assert_eq!(evaluate_missed(&def, true, false), None);
        // RunOnce 策略：错过 ⇒ 补跑一次
        def.missed_run_policy = MissedRunPolicy::RunOnce;
        let due = evaluate_missed(&def, true, false).expect("catch up");
        assert!(due.is_catch_up);
        // 未错过（missed=false）不算 catch-up
        assert_eq!(evaluate_missed(&def, false, false), None);
        // SkipIfRunning + running ⇒ None（§14 防重复）
        assert_eq!(evaluate_missed(&def, true, true), None);
        // disabled ⇒ None
        def.enabled = false;
        assert_eq!(evaluate_missed(&def, true, false), None);
    }

    #[test]
    fn daily_missed_run_never_runs_multiple_catchups() {
        // §13：睡眠 8 小时 ⇒ 最多补跑一次（RunOnce），Skip 策略直接跳过
        let kind = ScheduleKind::Daily { hour: 1, minute: 0 };
        let now = ts(9 * 3600); // 09:00，01:00 的运行已错过
        let def = ScheduleDef {
            id: "daily".to_string(),
            name: "daily".to_string(),
            enabled: true,
            kind: kind.clone(),
            missed_run_policy: MissedRunPolicy::Skip,
            concurrency: ConcurrencyPolicy::SkipIfRunning,
            job_name: "noop".to_string(),
            created_at: ts(0),
            last_run_at: None,
            last_result: None,
            next_run_at_cached: None,
        };
        assert_eq!(
            evaluate_missed(&def, true, false),
            None,
            "Skip ⇒ 错过即跳过（不补跑风暴 §13）"
        );
    }
}
