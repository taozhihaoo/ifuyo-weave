//! M2.11 Website Monitor 服务（IPC 边界 + 运行时）。领域在 `monitor/`。
//!
//! 复用纪律（§1/§70/§80——零第二套）：
//! - HTTP/提取复用 M2.10（`web::fetch` + html 管线；唯一网络出口）。
//! - 调度复用 M2.6 Agent Scheduler（job_name = `monitor:<target_id>`，
//!   dispatch 分支 spawn 线程不阻塞 tick；无第二 Scheduler §23）。
//! - 通知复用 M2.6 NotificationCenter（§21/§22）。
//! - Diff 复用 text_service::compare_text（weave-text 有界 LCS §11）。
//! - 手动 Check Now 与调度检查走**同一执行函数**（§24）。
//! - 稳定性（§25/§28/§43/§46/§48）：per-target 执行锁 + 全局有界并发 +
//!   失败退避 + 失败不污染成功快照 + 历史容量控制 + 元数据/正文分离。

#![expect(clippy::result_large_err)] // D10：IpcError DTO 体积豁免

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use tauri::Manager;

use crate::commands::IpcError;
use crate::config::MonitorConfig;
use crate::files_dto::JobHandleDto;
use crate::monitor::model::{
    CheckResult, MIN_INTERVAL_SECS, MonitorChangeEvent, MonitorMode, MonitorSnapshot,
    MonitorTarget, TargetStatus,
};
use crate::monitor::store::{MonitorStore, valid_target_id};
use crate::monitor::{backoff_delay_secs, build_event, check_target};
use crate::web::fetch::FetchLimits;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    IpcError::from(
        weave_core::prelude::WeaveError::io(code, message).with_location("monitor_service"),
    )
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// Monitor 运行时（AppState 持有）。
pub struct MonitorRuntime {
    store: MonitorStore,
    config: Mutex<MonitorConfig>,
    /// per-target 执行锁（§48：同一 Target 同时只有一个 Check）。
    inflight: Mutex<HashSet<String>>,
    /// 全局并发上限（§25：100 targets ≠ 100 并发请求）。
    active: AtomicUsize,
}

impl MonitorRuntime {
    pub fn new(root: std::path::PathBuf, config: MonitorConfig) -> Self {
        Self {
            store: MonitorStore::open(root),
            config: Mutex::new(config),
            inflight: Mutex::new(HashSet::new()),
            active: AtomicUsize::new(0),
        }
    }

    pub fn store(&self) -> &MonitorStore {
        &self.store
    }

    pub fn config_snapshot(&self) -> MonitorConfig {
        self.config.lock().expect("monitor config").clone()
    }

    pub fn apply_config(&self, config: &MonitorConfig) {
        *self.config.lock().expect("monitor config") = config.clone();
    }

    fn limits(&self) -> FetchLimits {
        let c = self.config_snapshot();
        FetchLimits {
            timeout_secs: c.timeout_secs.max(1),
            max_bytes: c.max_response_mb.max(1) as usize * 1024 * 1024,
            ..FetchLimits::default()
        }
    }
}

fn validate_new(
    name: &str,
    url: &str,
    mode: MonitorMode,
    selector: Option<&str>,
    interval_secs: u32,
) -> Result<(), IpcError> {
    if name.trim().is_empty() {
        return Err(err("web.monitorNameRequired", "monitor name is required"));
    }
    crate::web::url::normalize(url).map_err(IpcError::from)?;
    if interval_secs < MIN_INTERVAL_SECS {
        return Err(err(
            "web.monitorIntervalTooLow",
            format!("interval must be at least {MIN_INTERVAL_SECS}s (rate-limit discipline §18)"),
        ));
    }
    if mode == MonitorMode::Element && selector.map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Err(err(
            "web.monitorSelectorRequired",
            "element mode requires a CSS selector",
        ));
    }
    Ok(())
}

// ── CRUD（§63/§64/§65：删除区分历史；暂停/禁用/删除分离）──

#[tauri::command]
#[specta::specta]
pub fn monitor_list(app: tauri::AppHandle) -> Vec<MonitorTarget> {
    let state = app.state::<crate::state::AppState>();
    let mut targets = state.monitor.store().load_targets();
    targets.sort_by(|a, b| {
        a.created_at_ms
            .partial_cmp(&b.created_at_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    targets
}

#[tauri::command]
#[specta::specta]
pub fn monitor_create(
    app: tauri::AppHandle,
    name: String,
    url: String,
    mode: MonitorMode,
    selector: Option<String>,
    interval_secs: u32,
) -> Result<MonitorTarget, IpcError> {
    validate_new(&name, &url, mode, selector.as_deref(), interval_secs)?;
    let state = app.state::<crate::state::AppState>();
    let defaults = state.monitor.config_snapshot();
    let now = now_ms();
    let target = MonitorTarget {
        id: format!("m-{}", now as i64),
        name: name.trim().to_string(),
        url: url.trim().to_string(),
        enabled: true,
        paused: false,
        mode,
        selector: selector
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        interval_secs,
        notify_on_change: defaults.notify_on_change,
        notify_on_error: defaults.notify_on_error,
        notify_on_recovery: defaults.notify_on_recovery,
        consecutive_failures: 0,
        last_checked_at_ms: None,
        last_success_at_ms: None,
        last_fingerprint: None,
        last_snapshot_seq: None,
        status: TargetStatus::Waiting,
        last_error_code: None,
        created_at_ms: now,
        updated_at_ms: now,
    };
    let mut targets = state.monitor.store().load_targets();
    targets.push(target.clone());
    state
        .monitor
        .store()
        .save_targets(&targets)
        .map_err(IpcError::from)?;
    // 自动注册调度（§23：M2.6 Scheduler 真实消费者）
    register_schedule(&app, &target);
    tracing::info!(target: "monitor", id = %target.id, "monitor created");
    Ok(target)
}

#[tauri::command]
#[specta::specta]
pub fn monitor_update(
    app: tauri::AppHandle,
    target: MonitorTarget,
) -> Result<MonitorTarget, IpcError> {
    if !valid_target_id(&target.id) {
        return Err(err("web.monitorInvalidId", "invalid target id"));
    }
    validate_new(
        &target.name,
        &target.url,
        target.mode,
        target.selector.as_deref(),
        target.interval_secs,
    )?;
    let state = app.state::<crate::state::AppState>();
    let mut targets = state.monitor.store().load_targets();
    let Some(existing) = targets.iter_mut().find(|t| t.id == target.id) else {
        return Err(err(
            "web.monitorNotFound",
            format!("no monitor '{}'", target.id),
        ));
    };
    let mut next = target;
    next.updated_at_ms = now_ms();
    *existing = next.clone();
    state
        .monitor
        .store()
        .save_targets(&targets)
        .map_err(IpcError::from)?;
    // 调度随 interval/enabled 变化重建
    register_schedule(&app, &next);
    Ok(next)
}

/// 删除（§63：默认仅删 Target 保留历史；delete_history=true 连快照一起删，
/// UI 明示）。
#[tauri::command]
#[specta::specta]
pub fn monitor_delete(
    app: tauri::AppHandle,
    target_id: String,
    delete_history: bool,
) -> Result<(), IpcError> {
    if !valid_target_id(&target_id) {
        return Err(err("web.monitorInvalidId", "invalid target id"));
    }
    let state = app.state::<crate::state::AppState>();
    let mut targets = state.monitor.store().load_targets();
    let before = targets.len();
    targets.retain(|t| t.id != target_id);
    if targets.len() == before {
        return Err(err(
            "web.monitorNotFound",
            format!("no monitor '{target_id}'"),
        ));
    }
    state
        .monitor
        .store()
        .save_targets(&targets)
        .map_err(IpcError::from)?;
    if delete_history {
        state
            .monitor
            .store()
            .delete_target_data(&target_id)
            .map_err(IpcError::from)?;
    }
    remove_schedule(&app, &target_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn monitor_set_paused(
    app: tauri::AppHandle,
    target_id: String,
    paused: bool,
) -> Result<MonitorTarget, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut targets = state.monitor.store().load_targets();
    let Some(t) = targets.iter_mut().find(|t| t.id == target_id) else {
        return Err(err(
            "web.monitorNotFound",
            format!("no monitor '{target_id}'"),
        ));
    };
    t.paused = paused;
    t.status = if paused {
        TargetStatus::Paused
    } else {
        TargetStatus::Idle
    };
    t.updated_at_ms = now_ms();
    let next = t.clone();
    state
        .monitor
        .store()
        .save_targets(&targets)
        .map_err(IpcError::from)?;
    register_schedule(&app, &next); // enabled/paused 同步到调度
    Ok(next)
}

#[tauri::command]
#[specta::specta]
pub fn monitor_set_enabled(
    app: tauri::AppHandle,
    target_id: String,
    enabled: bool,
) -> Result<MonitorTarget, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut targets = state.monitor.store().load_targets();
    let Some(t) = targets.iter_mut().find(|t| t.id == target_id) else {
        return Err(err(
            "web.monitorNotFound",
            format!("no monitor '{target_id}'"),
        ));
    };
    t.enabled = enabled;
    t.status = if enabled {
        TargetStatus::Idle
    } else {
        TargetStatus::Disabled
    };
    t.updated_at_ms = now_ms();
    let next = t.clone();
    state
        .monitor
        .store()
        .save_targets(&targets)
        .map_err(IpcError::from)?;
    register_schedule(&app, &next);
    Ok(next)
}

// ── 历史与变化（§8/§9/§40）──

#[tauri::command]
#[specta::specta]
pub fn monitor_history(app: tauri::AppHandle, target_id: String) -> Vec<MonitorSnapshot> {
    let state = app.state::<crate::state::AppState>();
    let mut list = state.monitor.store().list_snapshots(&target_id);
    list.reverse(); // 最新在前（历史列表 §8）
    list
}

#[tauri::command]
#[specta::specta]
pub fn monitor_changes(
    app: tauri::AppHandle,
    target_id: Option<String>,
) -> Vec<MonitorChangeEvent> {
    let state = app.state::<crate::state::AppState>();
    let mut events = state.monitor.store().load_events();
    if let Some(tid) = target_id {
        events.retain(|e| e.target_id == tid);
    }
    events.reverse();
    events
}

/// Before/After 详细 diff（§11/§56：复用 M2.1/Text Compare，无第二 Diff）。
#[tauri::command]
#[specta::specta]
pub fn monitor_diff(
    app: tauri::AppHandle,
    target_id: String,
    seq_a: f64,
    seq_b: f64,
) -> Result<crate::text_dto::DiffReportDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let (_meta_a, payload_a) = state
        .monitor
        .store()
        .load_snapshot(&target_id, seq_a)
        .ok_or_else(|| {
            err(
                "web.monitorSnapshotMissing",
                format!("snapshot {seq_a} missing"),
            )
        })?;
    let (_meta_b, payload_b) = state
        .monitor
        .store()
        .load_snapshot(&target_id, seq_b)
        .ok_or_else(|| {
            err(
                "web.monitorSnapshotMissing",
                format!("snapshot {seq_b} missing"),
            )
        })?;
    let a = payload_a.ok_or_else(|| {
        err(
            "web.monitorNoPayload",
            format!("snapshot {seq_a} is metadata-only"),
        )
    })?;
    let b = payload_b.ok_or_else(|| {
        err(
            "web.monitorNoPayload",
            format!("snapshot {seq_b} is metadata-only"),
        )
    })?;
    crate::text_service::compare_text(&a, &b, "", false)
}

/// Selector 测试（§34 最低要求：输入框+测试+匹配数+提取结果——作用于
/// 最近一次 M2.10 抓取缓存，不重复抓取）。
#[tauri::command]
#[specta::specta]
pub fn monitor_test_selector(
    app: tauri::AppHandle,
    selector: String,
) -> Result<MonitorSelectorTestDto, IpcError> {
    use scraper::Selector;
    let state = app.state::<crate::state::AppState>();
    let raw = state.web.latest_raw_html().ok_or_else(|| {
        err(
            "web.monitorNoPage",
            "fetch a page first (selector test runs on the last fetch)",
        )
    })?;
    let parsed = Selector::parse(&selector).map_err(|e| {
        err(
            "web.monitorInvalidSelector",
            format!("invalid selector: {e}"),
        )
    })?;
    let dom = scraper::Html::parse_document(&raw);
    let matches: Vec<_> = dom.select(&parsed).collect();
    let preview = matches
        .first()
        .map(|el| {
            let text = crate::web::html::fragment_to_text(&el.inner_html());
            text.chars().take(2000).collect::<String>()
        })
        .unwrap_or_default();
    Ok(MonitorSelectorTestDto {
        matched: matches.len() as u32,
        preview,
    })
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSelectorTestDto {
    pub matched: u32,
    pub preview: String,
}

// ── 检查执行（§24 同一路径；手动=Job，调度=spawn 线程）──

/// 核心执行（手动与调度共用）。返回确定性结果串（调度 last_result 用）。
pub fn run_check(app: &tauri::AppHandle, target_id: &str, trigger: &str) -> String {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.monitor;
    // per-target 锁（§48：manual + scheduler 不并发同 target）
    if !rt
        .inflight
        .lock()
        .expect("monitor inflight")
        .insert(target_id.to_string())
    {
        return format!("status=skipped reason=alreadyChecking trigger={trigger}");
    }
    // 全局并发上限（§25）：满 ⇒ 跳过本轮（调度安静重试；手动报忙）
    let max = rt.config_snapshot().max_concurrent_checks.max(1) as usize;
    if rt.active.load(Ordering::SeqCst) >= max {
        rt.inflight
            .lock()
            .expect("monitor inflight")
            .remove(target_id);
        return format!("status=skipped reason=busy trigger={trigger}");
    }
    rt.active.fetch_add(1, Ordering::SeqCst);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_check_locked(app, target_id, trigger)
    }));
    rt.active.fetch_sub(1, Ordering::SeqCst);
    rt.inflight
        .lock()
        .expect("monitor inflight")
        .remove(target_id);
    match outcome {
        Ok(s) => s,
        Err(_) => "status=failed reason=panic".to_string(),
    }
}

fn run_check_locked(app: &tauri::AppHandle, target_id: &str, trigger: &str) -> String {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.monitor;
    let cfg = rt.config_snapshot();
    let mut targets = rt.store().load_targets();
    let Some(target) = targets.iter_mut().find(|t| t.id == target_id) else {
        return format!("status=failed reason=noSuchTarget trigger={trigger}");
    };
    // backoff（§18）：连续失败后拉长实际检查间隔
    if target.consecutive_failures > 0 {
        let base = target.last_checked_at_ms.unwrap_or(0.0);
        let due_at = base
            + (backoff_delay_secs(target.interval_secs, target.consecutive_failures - 1) * 1000)
                as f64;
        if now_ms() < due_at {
            return format!("status=skipped reason=backoff trigger={trigger}");
        }
    }
    target.status = TargetStatus::Checking;
    target.updated_at_ms = now_ms();

    // 最近有正文快照作为 diff 基线（§46 metadata-only 快照向前回溯）
    let snapshots = rt.store().list_snapshots(target_id);
    let previous = snapshots
        .iter()
        .rev()
        .find(|s| s.payload_file.is_some())
        .map(|s| s.seq);
    let previous_payload = previous
        .and_then(|seq| rt.store().load_snapshot(target_id, seq))
        .and_then(|(_, p)| p);
    let next_seq = snapshots.last().map(|s| s.seq + 1.0).unwrap_or(1.0);
    let had_failure = target.consecutive_failures > 0;

    let outcome = check_target(
        target,
        &rt.limits(),
        &state.agent.agent_cancel,
        rt.store(),
        next_seq,
        previous_payload,
        had_failure,
    );
    let snapshot = outcome
        .snapshot
        .clone()
        .expect("check always yields snapshot meta");
    let payload_text = outcome.normalized_text.clone();
    rt.store()
        .save_snapshot(&snapshot, payload_text.as_deref())
        .map_err(|e| tracing::warn!(error = %e, "snapshot save failed"))
        .ok();

    // 更新 target（§15：失败绝不覆盖 last_fingerprint/成功快照）
    match outcome.result {
        CheckResult::FetchFailed
        | CheckResult::Blocked
        | CheckResult::Unsupported
        | CheckResult::Cancelled => {
            target.consecutive_failures += 1;
            target.status = match outcome.result {
                CheckResult::Blocked => TargetStatus::Blocked,
                CheckResult::Unsupported => TargetStatus::Unsupported,
                _ => TargetStatus::Failed,
            };
            target.last_error_code = outcome.error_code.clone();
        }
        _ => {
            target.consecutive_failures = 0;
            target.last_error_code = None;
            target.last_success_at_ms = Some(now_ms());
            target.last_fingerprint = Some(snapshot.fingerprint.clone());
            target.status = match outcome.result {
                CheckResult::ContentChanged => TargetStatus::Changed,
                _ => TargetStatus::Idle,
            };
        }
    }
    target.last_checked_at_ms = Some(now_ms());
    target.last_snapshot_seq = Some(snapshot.seq);
    target.updated_at_ms = now_ms();

    // 事件（§9）：变化/失败/恢复入事件；NoChange/Initial 仅历史
    let event_worthy = matches!(
        outcome.result,
        CheckResult::ContentChanged
            | CheckResult::Recovered
            | CheckResult::FetchFailed
            | CheckResult::Blocked
    );
    if event_worthy {
        let previous_seq = if outcome.result == CheckResult::ContentChanged {
            previous
        } else {
            None
        };
        let event = build_event(target, &outcome, snapshot.seq, previous_seq);
        rt.store()
            .append_event(&event)
            .map_err(|e| tracing::warn!(error = %e, "event append failed"))
            .ok();
    }

    // 通知（§21/§22：复用 M2.6 NotificationCenter；内容仅元数据 §62）
    let notifications = |code: &str, level: &str, message: String| {
        state.notifications.notify(code, level, &message);
    };
    match outcome.result {
        CheckResult::ContentChanged if target.notify_on_change => notifications(
            "monitor.changed",
            "warning",
            format!(
                "'{}' changed: +{} / -{} lines",
                target.name, outcome.added_lines, outcome.removed_lines
            ),
        ),
        CheckResult::Recovered if target.notify_on_recovery => notifications(
            "monitor.recovered",
            "info",
            format!("'{}' recovered", target.name),
        ),
        CheckResult::FetchFailed | CheckResult::Blocked if target.notify_on_error => notifications(
            "monitor.error",
            "warning",
            format!(
                "'{}' check failed ({})",
                target.name,
                outcome.error_code.clone().unwrap_or_default()
            ),
        ),
        _ => {}
    }

    if let Err(e) = rt.store().save_targets(&targets) {
        tracing::warn!(error = %e, "targets save failed");
    }
    // 历史容量（§28：数量+天数；最近成功基线受保护）
    let retention_ms = (cfg.retention_days.max(1) as f64) * 24.0 * 3600.0 * 1000.0;
    rt.store().retain_snapshots(
        target_id,
        u64::from(cfg.max_snapshots_per_target.max(1)),
        retention_ms,
        now_ms(),
    );
    tracing::info!(target: "monitor", id = %target_id, result = ?outcome.result, trigger = %trigger, "check done");
    format!(
        "status={:?} seq={} added={} removed={} trigger={trigger}",
        outcome.result, snapshot.seq, outcome.added_lines, outcome.removed_lines
    )
}

/// 手动 Check Now（§24：与调度同一 run_check；经 JobTracker 可观察）。
#[tauri::command]
#[specta::specta]
pub fn monitor_check_now(
    app: tauri::AppHandle,
    target_id: String,
) -> Result<JobHandleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    if !state
        .monitor
        .store()
        .load_targets()
        .iter()
        .any(|t| t.id == target_id)
    {
        return Err(err(
            "web.monitorNotFound",
            format!("no monitor '{target_id}'"),
        ));
    }
    ensure_network(&state)?;
    let (job_id, cancel, _sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let tid = target_id.clone();
    let job_id_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if cancel.is_cancelled() {
            return;
        }
        let summary = run_check(&handle, &tid, "manual");
        let app_state = handle.state::<crate::state::AppState>();
        app_state
            .jobs
            .finish(&job_id_task, crate::jobs::JobOutcome::WebCompleted);
        tracing::info!(target: "monitor", id = %tid, %summary, "manual check finished");
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

fn ensure_network(state: &tauri::State<'_, crate::state::AppState>) -> Result<(), IpcError> {
    if state
        .web
        .allow_network
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        Ok(())
    } else {
        Err(err(
            "web.networkDisabled",
            "network access is disabled in settings",
        ))
    }
}

// ── M2.6 Scheduler 集成（§23：无第二 Scheduler）──

/// 为 target 注册/更新调度（job_name = `monitor:<id>`；enabled/paused/
/// disabled ⇒ 移除调度）。
pub fn register_schedule(app: &tauri::AppHandle, target: &MonitorTarget) {
    let state = app.state::<crate::state::AppState>();
    let should_schedule = target.enabled && !target.paused;
    let mut sched = state.agent.scheduler.lock().expect("sched");
    if !should_schedule {
        sched.remove(&format!("sched-monitor-{}", target.id));
        return;
    }
    sched.upsert(crate::agent::ScheduleDef {
        id: format!("sched-monitor-{}", target.id),
        name: format!("monitor: {}", target.name),
        enabled: true,
        kind: crate::agent::ScheduleKind::Interval {
            seconds: u64::from(target.interval_secs),
        },
        missed_run_policy: crate::agent::MissedRunPolicy::Skip,
        concurrency: crate::agent::ConcurrencyPolicy::SkipIfRunning,
        job_name: format!("monitor:{}", target.id),
        created_at: std::time::SystemTime::UNIX_EPOCH,
        last_run_at: None,
        last_result: None,
        next_run_at_cached: None,
    });
    sched.recompute_next();
}

fn remove_schedule(app: &tauri::AppHandle, target_id: &str) {
    let state = app.state::<crate::state::AppState>();
    let mut sched = state.agent.scheduler.lock().expect("sched");
    sched.remove(&format!("sched-monitor-{target_id}"));
}

/// 应用启动恢复调度（targets.json → Scheduler）。
pub fn register_all_schedules(app: &tauri::AppHandle) {
    let state = app.state::<crate::state::AppState>();
    for target in state.monitor.store().load_targets() {
        if target.enabled && !target.paused {
            register_schedule(app, &target);
        }
    }
}

/// 监控报告导出（§40/§66：Markdown；output-first + atomic_write +
/// creation-undo——与 M2.10 导出同一写入体系 §99）。
#[tauri::command]
#[specta::specta]
pub fn monitor_export(
    app: tauri::AppHandle,
    target_id: Option<String>,
    dest_dir: String,
) -> Result<crate::web_service::WebExportResult, IpcError> {
    use weave_files::atomic_write;
    let state = app.state::<crate::state::AppState>();
    let targets = state.monitor.store().load_targets();
    let selected: Vec<&MonitorTarget> = match &target_id {
        Some(id) => {
            let t = targets
                .iter()
                .find(|t| &t.id == id)
                .ok_or_else(|| err("web.monitorNotFound", format!("no monitor '{id}'")))?;
            vec![t]
        }
        None => targets.iter().collect(),
    };
    if selected.is_empty() {
        return Err(err("web.exportEmpty", "no monitors to report"));
    }
    let mut md = String::from(
        "# Website Monitor Report

",
    );
    for t in &selected {
        let snapshots = state.monitor.store().list_snapshots(&t.id);
        let checks = snapshots.len();
        let succeeded = snapshots.iter().filter(|s| s.status_code == 200).count();
        let failed = snapshots
            .iter()
            .filter(|s| matches!(s.result, CheckResult::FetchFailed | CheckResult::Blocked))
            .count();
        let changes = snapshots
            .iter()
            .filter(|s| {
                matches!(
                    s.result,
                    CheckResult::ContentChanged | CheckResult::Recovered
                )
            })
            .count();
        md.push_str(&format!(
            "## {}

- URL: {}
- Mode: {:?}
- Interval: {}s
- Status: {:?}
- Checks: {checks} (ok {succeeded} / failed {failed})
- Changes: {changes}
- Last checked: {:?}
- Last success: {:?}

",
            t.name,
            url_mask(&t.url),
            t.mode,
            t.interval_secs,
            t.status,
            t.last_checked_at_ms,
            t.last_success_at_ms,
        ));
        for e in state
            .monitor
            .store()
            .load_events()
            .iter()
            .filter(|e| e.target_id == t.id)
            .rev()
            .take(10)
        {
            md.push_str(&format!(
                "- change @ {:?}: {} ({} magnitude, +{}/-{})
",
                e.detected_at_ms,
                e.summary,
                format!("{:?}", e.magnitude).to_lowercase(),
                e.added_lines,
                e.removed_lines
            ));
        }
        md.push('\n');
    }
    let dest = std::path::PathBuf::from(dest_dir.trim());
    if !dest.is_dir() {
        return Err(err(
            "web.exportDestMissing",
            format!("destination dir not found: {}", dest.display()),
        ));
    }
    let output = dest.join(format!("website-monitor-report-{}.md", now_ms() as i64));
    if output.exists() {
        return Err(err(
            "document.destinationExists",
            format!("output already exists: {} (output-first)", output.display()),
        ));
    }
    atomic_write(&weave_files::fs::StdFilesystem, &output, md.as_bytes()).map_err(|e| {
        err(
            "web.exportWriteFailed",
            format!("{}: {}", output.display(), e.message),
        )
    })?;
    if let Ok(hdir) = crate::rename_service::resolve_history_dir(&app)
        && let Err(e) = crate::batch_service::record_creation_transaction(
            &hdir,
            &[],
            std::slice::from_ref(&output),
            &format!("Website Monitor report · {} target(s)", selected.len()),
        )
    {
        tracing::warn!(error = %e, "monitor report history failed");
    }
    Ok(crate::web_service::WebExportResult {
        output: output.to_string_lossy().into_owned(),
        bytes: md.len() as f64,
    })
}

/// URL 掩码（§62：报告不含敏感 query 值）。
fn url_mask(u: &str) -> String {
    crate::web::url::mask_sensitive(u)
}

// ── 设置（§68；持久化走既有 set_app_config round-trip）──

#[tauri::command]
#[specta::specta]
pub fn monitor_apply_config(app: tauri::AppHandle, config: MonitorConfig) {
    let state = app.state::<crate::state::AppState>();
    state.monitor.apply_config(&config);
}

#[tauri::command]
#[specta::specta]
pub fn monitor_get_config(app: tauri::AppHandle) -> MonitorConfig {
    let state = app.state::<crate::state::AppState>();
    state.monitor.config_snapshot()
}
