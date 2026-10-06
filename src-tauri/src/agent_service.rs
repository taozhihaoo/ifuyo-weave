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
                let (recipe_dir, history) = {
                    match crate::rename_service::resolve_history_dir(&app) {
                        Ok(dir) => (
                            dir.join("recipes"),
                            crate::recipe::store_history::RunHistoryStore::at(
                                dir.join("recipe-runs.json"),
                            ),
                        ),
                        Err(_) => continue, // 存储不可用 ⇒ 本 tick 跳过（Recoverable §58）
                    }
                };
                let dispatched = dispatch_due_schedules(
                    &mut state.agent.scheduler.lock().expect("sched"),
                    &recipe_dir,
                    &history,
                    &mut state.agent.run_tokens.lock().expect("tokens"),
                    &state.agent.agent_cancel,
                );
                if dispatched > 0 {
                    tracing::debug!(dispatched, "agent scheduler dispatched recipe runs");
                }
            }
        })
        .expect("spawn agent scheduler tick");
}

/// 到期调度分派（M2.7 §25/§43：tick 与测试共用的核心；test 可直调）。
/// - `job_name = "recipe:<id>"` ⇒ 加载 Recipe 经 run_recipe 执行（同一 Plan
///   走 M2.0 核心；Run History 记录计数/状态 §48）。
/// - 其他 job_name ⇒ 未知 handler，仅登记事实（M2.7 无内置业务 Job）。
/// - 每次运行独立 CancellationToken（登记进 run_tokens，供后续取消扩展）。
pub fn dispatch_due_schedules(
    scheduler: &mut crate::agent::SchedulerComponent,
    recipe_store_dir: &std::path::Path,
    history: &crate::recipe::store_history::RunHistoryStore,
    run_tokens: &mut std::collections::HashMap<String, weave_core::prelude::CancellationToken>,
    cancel: &weave_core::prelude::CancellationToken,
) -> usize {
    let now = std::time::SystemTime::now();
    let due = scheduler.collect_due(now);
    let mut dispatched = 0usize;
    for task in due {
        if cancel.is_cancelled() {
            break;
        }
        let Some(recipe_id) = task.job_name.strip_prefix("recipe:") else {
            // 未知 handler：登记事实后释放运行标记
            scheduler.set_running(&task.id, false);
            scheduler.last_result_for(&task.id, "no handler registered");
            continue;
        };
        let token = weave_core::prelude::CancellationToken::new();
        run_tokens.insert(task.id.clone(), token.clone());
        let outcome = dispatch_recipe_run(recipe_store_dir, history, &recipe_id, &token, &task.id);
        run_tokens.remove(&task.id);
        scheduler.set_running(&task.id, false);
        scheduler.last_result_for(&task.id, &outcome);
        scheduler.last_success = Some(std::time::SystemTime::now());
        dispatched += 1;
    }
    dispatched
}

fn dispatch_recipe_run(
    recipe_store_dir: &std::path::Path,
    history: &crate::recipe::store_history::RunHistoryStore,
    recipe_id: &str,
    token: &weave_core::prelude::CancellationToken,
    schedule_id: &str,
) -> String {
    let store = match crate::recipe::model::RecipeStore::open(recipe_store_dir) {
        Ok(store) => store,
        Err(e) => return format!("store open failed: {}", e.code),
    };
    let outcome = store.load_all();
    let Some(recipe) = outcome.recipes.into_iter().find(|r| r.id == recipe_id) else {
        let msg = format!("recipe '{recipe_id}' not found");
        let _ = history.append(&missing_recipe_record(recipe_id, schedule_id, &msg));
        return msg;
    };
    let input_folder = recipe.input_folder.clone();
    // §48：仅计数/状态进入历史——run_recipe 报告本就无文件内容
    match crate::recipe::runner::run_recipe(
        &recipe,
        std::path::Path::new(&input_folder),
        false,
        token,
        &mut |_, _| {},
    ) {
        Ok(report) => {
            let summary = format!(
                "status={:?} matched={} processed={} failed={}",
                report.status, report.matched_files, report.total_processed, report.total_failed
            );
            let run_id = format!(
                "run_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
            let _ = history.record_from_report(
                &recipe,
                &report,
                &format!("schedule:{schedule_id}"),
                &run_id,
            );
            summary
        }
        Err(e) => format!("recipe run failed: {}", e.code),
    }
}

fn missing_recipe_record(
    recipe_id: &str,
    schedule_id: &str,
    msg: &str,
) -> crate::recipe::store_history::RecipeRunRecord {
    crate::recipe::store_history::RecipeRunRecord {
        run_id: format!("run_{schedule_id}_missing"),
        recipe_id: recipe_id.to_string(),
        recipe_name: String::new(),
        recipe_snapshot: crate::recipe::model::Recipe {
            id: recipe_id.to_string(),
            name: String::new(),
            description: msg.to_string(),
            schema_version: crate::recipe::model::RECIPE_SCHEMA_VERSION,
            enabled: false,
            created_at: std::time::SystemTime::UNIX_EPOCH,
            updated_at: std::time::SystemTime::UNIX_EPOCH,
            input_folder: String::new(),
            trigger: crate::recipe::model::RecipeTrigger::Manual,
            parameters: Vec::new(),
            actions: Vec::new(),
        },
        trigger: format!("schedule:{schedule_id}"),
        dry_run: false,
        status: "failed".to_string(),
        matched_files: 0.0,
        total_processed: 0.0,
        total_skipped: 0.0,
        total_failed: 1.0,
        duration_ms: 0.0,
        finished_at: std::time::SystemTime::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::model::{
        ActionType, RECIPE_SCHEMA_VERSION, Recipe, RecipeAction, RecipeStore, RecipeTrigger,
    };
    use std::time::Duration;
    use weave_core::prelude::CancellationToken;

    fn sample_recipe(id: &str, input_folder: &std::path::Path) -> Recipe {
        Recipe {
            id: id.to_string(),
            name: "scheduled manifest".to_string(),
            description: String::new(),
            schema_version: RECIPE_SCHEMA_VERSION,
            enabled: true,
            created_at: std::time::SystemTime::UNIX_EPOCH,
            updated_at: std::time::SystemTime::UNIX_EPOCH,
            input_folder: input_folder.to_string_lossy().into_owned(),
            trigger: RecipeTrigger::Interval { seconds: 1 },
            parameters: Vec::new(),
            actions: vec![RecipeAction {
                id: "a0".to_string(),
                action_type: ActionType::HashManifest,
                enabled: true,
                parameter: None,
            }],
        }
    }

    #[test]
    fn due_schedule_dispatches_recipe_and_records_run() {
        // §25/§43 集成：到期触发 → run_recipe 执行 → Run History 记录
        let ws = weave_testkit::TempWorkspace::new("agent-recipe-dispatch").expect("ws");
        let input_dir = ws.dir("input").expect("dir");
        std::fs::write(input_dir.join("a.txt"), b"hello").expect("seed");

        let recipe_dir = ws.path().join("recipes");
        let recipe_id = "r_sched_test";
        {
            let store = RecipeStore::open(&recipe_dir).expect("store");
            store
                .save(&sample_recipe(recipe_id, &input_dir))
                .expect("save recipe");
        }
        let history =
            crate::recipe::store_history::RunHistoryStore::at(ws.path().join("recipe-runs.json"));

        // Scheduler：Interval 1s，last_run 在过去 ⇒ 立即到期
        let mut scheduler = crate::agent::SchedulerComponent::default();
        scheduler.upsert(ScheduleDef {
            id: "sched1".to_string(),
            name: "test schedule".to_string(),
            enabled: true,
            kind: crate::agent::ScheduleKind::Interval { seconds: 1 },
            missed_run_policy: MissedRunPolicy::Skip,
            concurrency: crate::agent::ConcurrencyPolicy::SkipIfRunning,
            job_name: format!("recipe:{recipe_id}"),
            created_at: std::time::SystemTime::UNIX_EPOCH,
            last_run_at: Some(std::time::SystemTime::now() - std::time::Duration::from_secs(60)),
            last_result: None,
            next_run_at_cached: None,
        });

        let mut tokens = std::collections::HashMap::new();
        let dispatched = dispatch_due_schedules(
            &mut scheduler,
            &recipe_dir,
            &history,
            &mut tokens,
            &CancellationToken::new(),
        );
        assert_eq!(dispatched, 1, "到期调度应分派一次");

        // Run History 记录（§48：计数/状态，无内容）
        let runs = history.load();
        assert_eq!(runs.len(), 1, "run 被记录");
        assert_eq!(runs[0].recipe_id, recipe_id);
        assert_eq!(runs[0].status, "succeeded");
        assert!(runs[0].trigger.starts_with("schedule:sched1"));
        // Scheduler 状态回写
        assert!(scheduler.running_jobs.is_empty(), "运行标记已释放");
        assert!(scheduler.schedules.iter().any(|s| {
            s.last_result
                .as_deref()
                .unwrap_or("")
                .starts_with("status=")
        }));
    }

    #[test]
    fn unknown_handler_job_is_released_without_dispatch() {
        let ws = weave_testkit::TempWorkspace::new("agent-unknown-handler").expect("ws");
        let recipe_dir = ws.path().join("recipes");
        std::fs::create_dir_all(&recipe_dir).expect("dir");
        let history =
            crate::recipe::store_history::RunHistoryStore::at(ws.path().join("runs.json"));
        let mut scheduler = crate::agent::SchedulerComponent::default();
        scheduler.upsert(ScheduleDef {
            id: "sched2".to_string(),
            name: "unknown".to_string(),
            enabled: true,
            kind: crate::agent::ScheduleKind::Interval { seconds: 1 },
            missed_run_policy: MissedRunPolicy::Skip,
            concurrency: crate::agent::ConcurrencyPolicy::SkipIfRunning,
            job_name: "someFutureJob".to_string(),
            created_at: std::time::SystemTime::UNIX_EPOCH,
            last_run_at: Some(std::time::SystemTime::now() - std::time::Duration::from_secs(60)),
            last_result: None,
            next_run_at_cached: None,
        });
        let mut tokens = std::collections::HashMap::new();
        let dispatched = dispatch_due_schedules(
            &mut scheduler,
            &recipe_dir,
            &history,
            &mut tokens,
            &CancellationToken::new(),
        );
        assert_eq!(dispatched, 0);
        assert!(history.load().is_empty(), "未知 handler 不产生 run");
        // 运行标记已释放
        assert!(scheduler.running_jobs.is_empty());
    }
}
