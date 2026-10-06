//! M2.6：Agent IPC + 生命周期接线（Runtime 拥有后台组件；App Exit → Agent Stop）。
//!
//! - Clipboard 组件归入 Agent（§27）：暂停/恢复经 agent；页面关闭不影响。
//! - Scheduler tick 线程（§12/§13）：NextRunAt 驱动，MissedRun 策略，
//!   ConcurrencyPolicy SkipIfRunning，作业经有界队列 + 并发闸门。
//! - 通知（§22/§23）：统一出口 + 去重聚合窗口（60s 同 code 合并）。

#![expect(clippy::result_large_err)] // D10

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;

use crate::agent::{ComponentState, MissedRunPolicy, ScheduleDef, ScheduleKind};
use crate::commands::IpcError;
use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("agent_service")
        .into()
}

// ── 通知（§22/§23：统一出口 + 去重聚合）──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentNotificationDto {
    pub code: String,
    /// info | warning | error
    pub level: String,
    pub message: String,
    pub count: f64,
    pub at_ms: Option<f64>,
}

#[derive(Default)]
pub struct NotificationCenter {
    inner: std::sync::Mutex<std::collections::HashMap<String, AgentNotificationDto>>,
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 通知（60s 窗口内同 code 聚合计数——§23 防风暴）。
    pub fn notify(&self, code: &str, level: &str, message: &str) {
        let mut map = self.inner.lock().expect("notifications");
        let now = std::time::SystemTime::now();
        let last_at = map.get(code).and_then(|n| {
            n.at_ms.map(|ms| {
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(ms as u64)
            })
        });
        let within_window = match last_at {
            Some(t) => {
                now.duration_since(t)
                    .map(|d| d.as_secs())
                    .unwrap_or(u64::MAX)
                    < 60
            }
            None => false,
        };
        match (within_window, map.get_mut(code)) {
            (true, Some(existing)) => {
                existing.count += 1.0;
                existing.message = message.to_string();
            }
            _ => {
                let at_ms = now
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_millis() as f64);
                map.insert(
                    code.to_string(),
                    AgentNotificationDto {
                        code: code.to_string(),
                        level: level.to_string(),
                        message: message.to_string(),
                        count: 1.0,
                        at_ms,
                    },
                );
            }
        }
    }

    pub fn list(&self) -> Vec<AgentNotificationDto> {
        let mut list: Vec<AgentNotificationDto> = self
            .inner
            .lock()
            .expect("notifications")
            .values()
            .cloned()
            .collect();
        list.sort_by(|a, b| {
            b.at_ms
                .partial_cmp(&a.at_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list
    }

    pub fn clear(&self) {
        self.inner.lock().expect("notifications").clear();
    }
}

// ── Scheduler 定义持久化（history_dir/agent-schedules.json；§35/§36）──

pub fn load_schedules(app: &tauri::AppHandle) -> Vec<ScheduleDef> {
    let Ok(dir) = crate::rename_service::resolve_history_dir(app) else {
        return Vec::new();
    };
    let path = dir.join("agent-schedules.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_schedules(app: &tauri::AppHandle, schedules: &[ScheduleDef]) {
    if let Ok(dir) = crate::rename_service::resolve_history_dir(app) {
        let path = dir.join("agent-schedules.json");
        if let Ok(json) = serde_json::to_string_pretty(schedules) {
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, json)
                .and_then(|()| std::fs::rename(&tmp, &path))
                .is_err()
            {
                tracing::warn!("agent schedules persist failed");
            }
        }
    }
}

// ── IPC DTO ──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusDto {
    /// stopped | running | paused | faulted …
    pub state: String,
    pub paused: bool,
    pub components: Vec<(String, String)>,
    pub queue_len: f64,
    pub queue_capacity: f64,
    pub queue_dropped: f64,
    pub gate_running: f64,
    pub gate_max: f64,
    pub scheduler_schedules: f64,
    pub scheduler_running: f64,
    pub last_error: Option<String>,
    pub notifications: Vec<AgentNotificationDto>,
}

#[tauri::command]
#[specta::specta]
pub fn agent_status(app: tauri::AppHandle) -> Result<AgentStatusDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let health = state.agent.health();
    Ok(AgentStatusDto {
        state: format!("{:?}", health.state).to_lowercase(),
        paused: health.paused,
        components: health
            .components
            .iter()
            .map(|c| (c.name.clone(), c.state.clone()))
            .collect(),
        queue_len: health.queue_len as f64,
        queue_capacity: health.queue_capacity as f64,
        queue_dropped: health.queue_dropped as f64,
        gate_running: health.gate_running as f64,
        gate_max: health.gate_max as f64,
        scheduler_schedules: health.scheduler_schedules as f64,
        scheduler_running: health.scheduler_running as f64,
        last_error: health.last_error,
        notifications: state.notifications.list(),
    })
}

/// 暂停/恢复 Agent（幂等 §8）。暂停 ⇒ Clipboard 捕获同步暂停（§27）。
#[tauri::command]
#[specta::specta]
pub fn agent_set_paused(app: tauri::AppHandle, paused: bool) -> Result<(), IpcError> {
    let state = app.state::<crate::state::AppState>();
    let result = if paused {
        state.agent.pause()
    } else {
        state.agent.resume()
    };
    result.map_err(IpcError::from)?;
    // Clipboard 组件联动（§27）：暂停 ⇒ 捕获暂停
    {
        let mut clip = state.agent.clipboard.inner.lock().expect("clip");
        use crate::agent::AgentComponent;
        if paused {
            clip.pause();
        } else {
            clip.resume();
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleUpsertDto {
    pub id: Option<String>,
    pub name: String,
    pub enabled: bool,
    /// interval | daily
    pub kind: String,
    pub interval_seconds: Option<f64>,
    pub hour: Option<f64>,
    pub minute: Option<f64>,
    /// skip | runOnce
    pub missed_run_policy: String,
    /// allow | skipIfRunning
    pub concurrency: String,
    pub job_name: String,
}

/// 新增/更新调度定义（持久化 agent-schedules.json §35/§36）。
#[tauri::command]
#[specta::specta]
pub fn agent_upsert_schedule(
    app: tauri::AppHandle,
    request: ScheduleUpsertDto,
) -> Result<String, IpcError> {
    let kind = match request.kind.as_str() {
        "interval" => ScheduleKind::Interval {
            seconds: request.interval_seconds.unwrap_or(0.0).max(1.0) as u64,
        },
        "daily" => ScheduleKind::Daily {
            hour: request.hour.unwrap_or(0.0).clamp(0.0, 23.0) as u8,
            minute: request.minute.unwrap_or(0.0).clamp(0.0, 59.0) as u8,
        },
        other => {
            return Err(err(
                "agent.unknownScheduleKind",
                format!("unknown kind '{other}'"),
            ));
        }
    };
    let id = request.id.unwrap_or_else(|| {
        format!(
            "sched_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        )
    });
    let def = ScheduleDef {
        id: id.clone(),
        name: request.name,
        enabled: request.enabled,
        kind,
        missed_run_policy: if request.missed_run_policy == "runOnce" {
            MissedRunPolicy::RunOnce
        } else {
            MissedRunPolicy::Skip
        },
        concurrency: if request.concurrency == "allow" {
            crate::agent::ConcurrencyPolicy::Allow
        } else {
            crate::agent::ConcurrencyPolicy::SkipIfRunning
        },
        job_name: request.job_name,
        created_at: std::time::SystemTime::now(),
        last_run_at: None,
        last_result: None,
        next_run_at_cached: None,
    };
    let state = app.state::<crate::state::AppState>();
    state
        .agent
        .scheduler
        .lock()
        .expect("sched")
        .upsert(def.clone());
    let schedules = state
        .agent
        .scheduler
        .lock()
        .expect("sched")
        .schedules
        .clone();
    save_schedules(&app, &schedules);
    Ok(id)
}

/// 删除调度定义（持久化）。
#[tauri::command]
#[specta::specta]
pub fn agent_remove_schedule(app: tauri::AppHandle, id: String) -> Result<bool, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let removed = state.agent.scheduler.lock().expect("sched").remove(&id);
    let schedules = state
        .agent
        .scheduler
        .lock()
        .expect("sched")
        .schedules
        .clone();
    save_schedules(&app, &schedules);
    Ok(removed)
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub kind: String,
    pub job_name: String,
    pub last_result: Option<String>,
}

/// 调度清单。
#[tauri::command]
#[specta::specta]
pub fn agent_list_schedules(app: tauri::AppHandle) -> Result<Vec<ScheduleDto>, IpcError> {
    let state = app.state::<crate::state::AppState>();
    Ok(state
        .agent
        .scheduler
        .lock()
        .expect("sched")
        .schedules
        .iter()
        .map(|s| ScheduleDto {
            id: s.id.clone(),
            name: s.name.clone(),
            enabled: s.enabled,
            kind: match s.kind {
                crate::agent::ScheduleKind::Interval { seconds } => {
                    format!("interval:{seconds}s")
                }
                crate::agent::ScheduleKind::Daily { hour, minute } => {
                    format!("daily:{hour:02}:{minute:02}")
                }
            },
            job_name: s.job_name.clone(),
            last_result: s.last_result.clone(),
        })
        .collect())
}

/// Agent 生命周期接线：状态启动 + Scheduler 加载 + tick 线程。
/// （Clipboard 监听线程已在 setup 启动——这里只注册其组件状态。）
pub fn start_agent(app: tauri::AppHandle) {
    let state = app.state::<crate::state::AppState>();
    let _ = state.agent.start();
    {
        let mut sched = state.agent.scheduler.lock().expect("sched");
        for def in load_schedules(&app) {
            sched.upsert(def);
        }
        sched.state = Some(ComponentState::Running);
    }
    {
        use crate::agent::AgentComponent;
        state.agent.clipboard.inner.lock().expect("clip").start();
    }
    // Scheduler tick 线程（Agent 拥有；App Exit ⇒ 进程退 ⇒ 线程终止 §10）
    std::thread::Builder::new()
        .name("agent-scheduler-tick".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                let state = app.state::<crate::state::AppState>();
                if !state.agent.is_running() {
                    continue;
                }
                state.agent.record_tick();
                let due = {
                    let mut sched = state.agent.scheduler.lock().expect("sched");
                    sched.collect_due(std::time::SystemTime::now())
                };
                for task in due {
                    // M2.6 无具体业务 Job——调度到已注册 handler（M2.7 Recipe 接入点 §52）。
                    // 当前 handler 注册表为空 ⇒ 记录 tick 事实即可（Audit §39 安全元数据）。
                    tracing::debug!(
                        schedule = %task.id,
                        job = %task.job_name,
                        catch_up = task.is_catch_up,
                        "scheduled tick"
                    );
                    let mut sched = state.agent.scheduler.lock().expect("sched");
                    sched.set_running(&task.id, false);
                    sched.last_success = Some(std::time::SystemTime::now());
                }
            }
        })
        .expect("spawn agent scheduler tick");
}
