//! M2.9 Wisp 服务（IPC 边界 + 运行时）。领域逻辑在 `wisp/` 模块。
//!
//! - **单一 Clipboard 数据源**（§27）：后台分析挂在 M2.3 监听线程同一
//!   通路（`note_clipboard_text` 钩子），无第二监听器；写入复用 arboard
//!   + M2.3 `suppress` 指纹抑制（写前登记，消 1s 轮询竞态）。
//! - **Paste Detection = Limited（§14 如实）**：workspace unsafe=forbid
//!   ⇒ 无全局键盘钩子；防线 = guarded recopy（Weave 内粘贴意图）+
//!   保护会话（mask → 用户粘贴 → TTL 内指纹守卫恢复）。
//! - 隐私（§26/§53）：事件/日志/通知只含元数据（规则 ID/类别/风险/
//!   决策/exe），绝无完整 secret；不新增网络。

#![expect(clippy::result_large_err)] // D10：IpcError DTO 体积豁免（与 archive_service 同）

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use tauri::Manager;

use crate::commands::IpcError;
use crate::config::WispConfig;
use crate::wisp::model::{SensitivityLevel, SensitivityResult};
use crate::wisp::risk::{
    AppClass, AppRule, GlobalPolicy, PasteRisk, PolicyMode, TargetConfidence, TargetContext,
};
use crate::wisp::rules::{ANALYZER_VERSION, DetectionToggles, RULESET_VERSION, analyze_text};
use crate::wisp::session::{
    self, ProtectionSession, SessionPayload, SessionState, apply_restore_outcome, precheck_restore,
};
use crate::wisp::{PasteAction, PasteDecision, mask_text};

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    IpcError::from(weave_core::prelude::WeaveError::io(code, message).with_location("wisp_service"))
}

pub const EVENTS_CAP: usize = 200;

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// 内容指纹（与 M2.3 监听同构：SHA-256 hex）。
fn fingerprint(text: &str) -> Option<String> {
    weave_files::hash::hash_bytes(weave_core::prelude::HashAlgorithm::Sha256, text.as_bytes()).ok()
}

/// 安全事件记录（§26/§45：仅元数据，绝无内容；有界 200 §34）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WispEventDto {
    pub at_ms: f64,
    /// analyzed | evaluated | masked | restored | restoreSkipped | cancelled | expired | warned | blocked | allowed
    pub kind: String,
    pub level: SensitivityLevel,
    pub risk: Option<PasteRisk>,
    pub action: Option<PasteAction>,
    /// 目标可执行名（未知 ⇒ None；无标题 §22）。
    pub target: Option<String>,
    pub rule_ids: Vec<String>,
}

/// 后台分析快照（同内容不重分析 §34）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WispAnalysisDto {
    pub at_ms: f64,
    pub result: SensitivityResult,
}

/// 状态（§31：命令状态反映 Enabled/Paused/Unsupported/Failed——
/// detectionCapability="limited" 如实声明无键盘钩子降级）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WispStatusDto {
    pub enabled: bool,
    pub paused: bool,
    pub detection_capability: String,
    pub ruleset_version: u32,
    pub analyzer_version: u32,
    pub session: Option<ProtectionSession>,
    pub latest: Option<WispAnalysisDto>,
    pub event_count: f64,
}

/// 评估结果（决策 + 证据）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WispEvaluateDto {
    pub decision: PasteDecision,
    pub result: SensitivityResult,
}

struct ActiveSession {
    meta: ProtectionSession,
    /// 原文/masked 文本（内存持有；retain=false 不落盘 §25）。
    payload: Option<SessionPayload>,
}

/// Wisp 运行时（AppState 持有；无自建线程——复用 Agent/M2.3 生命周期 §29）。
pub struct WispRuntime {
    pub enabled: AtomicBool,
    pub paused: AtomicBool,
    dir: std::path::PathBuf,
    config: Mutex<WispConfig>,
    session: Mutex<Option<ActiveSession>>,
    latest: Mutex<Option<WispAnalysisDto>>,
    last_analyzed_hash: Mutex<Option<String>>,
    events: Mutex<VecDeque<WispEventDto>>,
}

impl WispRuntime {
    /// 打开（启动时）：加载残留快照——**不自动恢复**（§20），仅把事实
    /// 交还 UI；恢复仍走指纹守卫（§19）。
    pub fn open(dir: std::path::PathBuf, config: WispConfig) -> Self {
        let runtime = Self {
            enabled: AtomicBool::new(config.enabled),
            paused: AtomicBool::new(false),
            dir,
            config: Mutex::new(config),
            session: Mutex::new(None),
            latest: Mutex::new(None),
            last_analyzed_hash: Mutex::new(None),
            events: Mutex::new(VecDeque::new()),
        };
        if let Some(payload) = session::load_snapshot(&runtime.dir) {
            let meta = ProtectionSession {
                session_id: payload.session_id.clone(),
                original_fingerprint: String::new(),
                masked_fingerprint: String::new(),
                created_at_ms: payload.created_at_ms,
                expires_at_ms: payload.expires_at_ms,
                state: SessionState::Masked,
                target: TargetContext::unknown(),
                original_level: SensitivityLevel::Unknown,
                last_restore_skip_reason: None,
            };
            *runtime.session.lock().expect("wisp session") = Some(ActiveSession {
                meta,
                payload: Some(payload),
            });
        }
        runtime
    }

    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }

    pub fn config_snapshot(&self) -> WispConfig {
        self.config.lock().expect("wisp config").clone()
    }

    pub fn apply_config(&self, config: &WispConfig) {
        self.enabled.store(config.enabled, Ordering::SeqCst);
        *self.config.lock().expect("wisp config") = config.clone();
    }

    fn toggles(&self) -> DetectionToggles {
        let c = self.config.lock().expect("wisp config");
        DetectionToggles {
            credentials: c.detect_credentials,
            local_paths: c.detect_local_paths,
            personal_info: c.detect_personal_info,
            secret_config: c.detect_secret_config,
        }
    }

    fn global_policy(&self) -> GlobalPolicy {
        let c = self.config.lock().expect("wisp config");
        GlobalPolicy {
            block_critical: c.block_critical,
            app_rules: c
                .app_rules
                .iter()
                .map(|r| AppRule {
                    process: r.process.clone(),
                    mode: match r.mode.as_str() {
                        "allow" => PolicyMode::Allow,
                        "warn" => PolicyMode::Warn,
                        "mask" => PolicyMode::Mask,
                        "block" => PolicyMode::Block,
                        _ => PolicyMode::Default,
                    },
                    enabled: r.enabled,
                })
                .collect(),
        }
    }

    fn push_event(&self, event: WispEventDto) {
        let mut events = self.events.lock().expect("wisp events");
        if events.len() >= EVENTS_CAP {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub fn recent_events(&self, limit: usize) -> Vec<WispEventDto> {
        let events = self.events.lock().expect("wisp events");
        events.iter().rev().take(limit).cloned().collect()
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().expect("wisp events").len()
    }

    pub fn clear_events(&self) {
        self.events.lock().expect("wisp events").clear();
    }

    /// 惰性 TTL 判定（无常驻定时器 §29）：过期 ⇒ 标记 + 事件 + 清载荷。
    fn expire_if_due(&self, now: f64) {
        let mut guard = self.session.lock().expect("wisp session");
        let Some(active) = guard.as_mut() else { return };
        if session::is_expired(&active.meta, now) {
            active.meta.state = SessionState::Expired;
            session::clear_snapshot(&self.dir);
            active.payload = None;
            let level = active.meta.original_level;
            self.push_event(WispEventDto {
                at_ms: now,
                kind: "expired".into(),
                level,
                risk: None,
                action: None,
                target: None,
                rule_ids: Vec::new(),
            });
        }
    }
}

/// 前台窗口目标（复用 M2.8 x-win 安全封装；**Likely 置信**——前台窗口
/// ≠ 必然粘贴目标 §8；进程名未知 ⇒ Unknown 如实）。
fn foreground_target() -> TargetContext {
    match crate::glint::foreground_window_rect() {
        Ok(win) => {
            let process = if win.process_name.is_empty() {
                None
            } else {
                Some(win.process_name.to_lowercase())
            };
            let app_class = process
                .as_deref()
                .map(crate::wisp::classify_app)
                .unwrap_or(AppClass::Unknown);
            TargetContext {
                app_class,
                process,
                window_title: if win.title.is_empty() {
                    None
                } else {
                    Some(win.title)
                },
                identity_confidence: TargetConfidence::Likely,
            }
        }
        Err(_) => TargetContext::unknown(),
    }
}

/// M2.3 监听钩子（同一线程同一通路——**不是第二监听器** §27）。
/// 同内容不重分析（§34 风暴防护）；事件/通知只含元数据（§26/§48）。
pub fn note_clipboard_text(app: &tauri::AppHandle, text: &str) {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.wisp;
    if !rt.enabled.load(Ordering::SeqCst) || rt.paused.load(Ordering::SeqCst) {
        return;
    }
    let Some(fp) = fingerprint(text) else { return };
    {
        let mut last = rt.last_analyzed_hash.lock().expect("wisp last");
        if last.as_deref() == Some(fp.as_str()) {
            return;
        }
        *last = Some(fp);
    }
    let result = analyze_text(text, &rt.toggles());
    let at = now_ms();
    if result.level == SensitivityLevel::Normal {
        *rt.latest.lock().expect("wisp latest") = Some(WispAnalysisDto { at_ms: at, result });
        return;
    }
    let mut rule_ids: Vec<String> = result.findings.iter().map(|f| f.rule_id.clone()).collect();
    rule_ids.sort();
    rule_ids.dedup();
    rt.push_event(WispEventDto {
        at_ms: at,
        kind: "analyzed".into(),
        level: result.level,
        risk: None,
        action: None,
        target: None,
        rule_ids: rule_ids.clone(),
    });
    *rt.latest.lock().expect("wisp latest") = Some(WispAnalysisDto {
        at_ms: at,
        result: result.clone(),
    });
    // 通知中心（M2.6 复用；60s 聚合防风暴 §23/§48）——只报数量不报内容
    state.notifications.notify(
        "wisp.sensitiveClipboard",
        "warning",
        &format!("clipboard findings: {} rule(s)", rule_ids.len()),
    );
    tracing::info!(target: "wisp", rules = rule_ids.len(), level = ?result.level, "sensitive clipboard analyzed");
}

// ── IPC ──

#[tauri::command]
#[specta::specta]
pub fn wisp_status(app: tauri::AppHandle) -> WispStatusDto {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.wisp;
    rt.expire_if_due(now_ms());
    WispStatusDto {
        enabled: rt.enabled.load(Ordering::SeqCst),
        paused: rt.paused.load(Ordering::SeqCst),
        detection_capability: "limited".to_string(),
        ruleset_version: RULESET_VERSION,
        analyzer_version: ANALYZER_VERSION,
        session: rt
            .session
            .lock()
            .expect("wisp session")
            .as_ref()
            .map(|a| a.meta.clone()),
        latest: rt.latest.lock().expect("wisp latest").clone(),
        event_count: rt.event_count() as f64,
    }
}

#[tauri::command]
#[specta::specta]
pub fn wisp_set_paused(app: tauri::AppHandle, paused: bool) {
    let state = app.state::<crate::state::AppState>();
    state.wisp.paused.store(paused, Ordering::SeqCst);
}

/// 设置立即生效（§32；持久化走既有 set_app_config）。
#[tauri::command]
#[specta::specta]
pub fn wisp_apply_config(app: tauri::AppHandle, config: WispConfig) {
    let state = app.state::<crate::state::AppState>();
    state.wisp.apply_config(&config);
}

/// 分析任意文本（面板"检测此文本"）。
#[tauri::command]
#[specta::specta]
pub fn wisp_analyze_text(app: tauri::AppHandle, text: String) -> SensitivityResult {
    let state = app.state::<crate::state::AppState>();
    analyze_text(&text, &state.wisp.toggles())
}

/// 评估当前剪贴板（assume_process 显式声明 ⇒ Confirmed 置信；否则前台
/// 窗口推断 ⇒ Likely）。
#[tauri::command]
#[specta::specta]
pub fn wisp_evaluate_current(
    app: tauri::AppHandle,
    assume_process: Option<String>,
) -> Result<WispEvaluateDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.wisp;
    rt.expire_if_due(now_ms());
    let mut clip = arboard::Clipboard::new().map_err(|e| {
        err(
            "wisp.clipboardUnavailable",
            format!("open clipboard failed: {e}"),
        )
    })?;
    let text = clip.get_text().map_err(|e| {
        err(
            "wisp.clipboardUnavailable",
            format!("read clipboard failed: {e}"),
        )
    })?;
    if text.is_empty() {
        return Err(err("wisp.clipboardUnavailable", "clipboard has no text"));
    }
    let result = analyze_text(&text, &rt.toggles());
    let target = match assume_process {
        Some(p) if !p.is_empty() => TargetContext {
            app_class: crate::wisp::classify_app(&p),
            process: Some(p.to_lowercase()),
            window_title: None,
            identity_confidence: TargetConfidence::Confirmed,
        },
        _ => foreground_target(),
    };
    let decision = crate::wisp::evaluate(&result, &target, &rt.global_policy());
    if result.level != SensitivityLevel::Normal {
        rt.push_event(WispEventDto {
            at_ms: now_ms(),
            kind: "evaluated".into(),
            level: result.level,
            risk: Some(decision.risk),
            action: Some(decision.action),
            target: target.process.clone(),
            rule_ids: decision.rule_ids.clone(),
        });
    }
    Ok(WispEvaluateDto { decision, result })
}

/// 保护会话：读当前剪贴板 → 分析 → mask 写回（M2.3 抑制指纹写前登记）
/// → 建会话（§11/§12：原文保留、可恢复）。
#[tauri::command]
#[specta::specta]
pub fn wisp_mask_current(
    app: tauri::AppHandle,
    ttl_secs: Option<u32>,
) -> Result<ProtectionSession, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.wisp;
    rt.expire_if_due(now_ms());
    {
        let guard = rt.session.lock().expect("wisp session");
        if guard.is_some() {
            return Err(err(
                "wisp.sessionActive",
                "a protection session already exists",
            ));
        }
    }
    let mut clip = arboard::Clipboard::new().map_err(|e| {
        err(
            "wisp.clipboardUnavailable",
            format!("open clipboard failed: {e}"),
        )
    })?;
    let text = clip.get_text().map_err(|e| {
        err(
            "wisp.clipboardUnavailable",
            format!("read clipboard failed: {e}"),
        )
    })?;
    if text.is_empty() {
        return Err(err("wisp.clipboardUnavailable", "clipboard has no text"));
    }
    let result = analyze_text(&text, &rt.toggles());
    if result.level == SensitivityLevel::Normal {
        return Err(err(
            "wisp.nothingToMask",
            "no sensitive findings in clipboard text",
        ));
    }
    let Some(original_fp) = fingerprint(&text) else {
        return Err(err("wisp.fingerprintFailed", "hash unavailable"));
    };
    let masked = mask_text(&text, &result.findings);
    let Some(masked_fp) = fingerprint(&masked) else {
        return Err(err("wisp.fingerprintFailed", "hash unavailable"));
    };
    let target = foreground_target();
    let ttl_secs = ttl_secs
        .unwrap_or(rt.config_snapshot().session_ttl_secs)
        .max(5) as f64;
    let at = now_ms();
    let meta = ProtectionSession {
        session_id: format!("wisp-{}", at as i64),
        original_fingerprint: original_fp,
        masked_fingerprint: masked_fp.clone(),
        created_at_ms: at,
        expires_at_ms: at + ttl_secs * 1000.0,
        state: SessionState::Masked,
        target: target.clone(),
        original_level: result.level,
        last_restore_skip_reason: None,
    };
    let payload = SessionPayload {
        session_id: meta.session_id.clone(),
        original_text: text,
        masked_text: masked,
        state: SessionState::Masked,
        created_at_ms: at,
        expires_at_ms: meta.expires_at_ms,
    };
    // 写 masked 前登记 M2.3 抑制指纹（消自循环 §13/消 1s 轮询竞态）
    *state.clipboard.suppress.lock().expect("suppress") = Some(masked_fp.to_lowercase());
    clip.set_text(payload.masked_text.clone()).map_err(|e| {
        err(
            "wisp.maskWriteFailed",
            format!("set masked text failed: {e}"),
        )
    })?;
    // §25：原文落盘仅在显式开启（不伪装保险库；关 ⇒ 崩溃后不可恢复，如实）
    if rt.config_snapshot().retain_sensitive_payload {
        session::save_snapshot(rt.dir(), &payload).map_err(IpcError::from)?;
    }
    *rt.session.lock().expect("wisp session") = Some(ActiveSession {
        meta: meta.clone(),
        payload: Some(payload),
    });
    rt.push_event(WispEventDto {
        at_ms: at,
        kind: "masked".into(),
        level: meta.original_level,
        risk: None,
        action: None,
        target: target.process,
        rule_ids: Vec::new(),
    });
    tracing::info!(target: "wisp", session = %meta.session_id, "protection session armed");
    Ok(meta)
}

/// 恢复原文（§19 指纹守卫：剪贴板 ≠ masked ⇒ **跳过不恢复**，用户已
/// 复制新内容；§20 绝不用旧内容覆盖新剪贴板）。
#[tauri::command]
#[specta::specta]
pub fn wisp_restore_session(app: tauri::AppHandle) -> Result<ProtectionSession, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.wisp;
    let now = now_ms();
    rt.expire_if_due(now);
    let payload = {
        let mut guard = rt.session.lock().expect("wisp session");
        let Some(active) = guard.as_mut() else {
            return Err(err("wisp.noSession", "no protection session"));
        };
        if let Err((new_state, reason)) = precheck_restore(&active.meta, now) {
            active.meta.state = new_state;
            return Err(err("wisp.restoreRejected", reason));
        }
        active.payload.clone()
    };
    let Some(payload) = payload else {
        return Err(err(
            "wisp.restoreUnavailable",
            "original payload not retained (retain disabled and runtime restarted)",
        ));
    };
    let mut clip = arboard::Clipboard::new().map_err(|e| {
        err(
            "wisp.clipboardUnavailable",
            format!("open clipboard failed: {e}"),
        )
    })?;
    let current = clip.get_text().map_err(|e| {
        err(
            "wisp.clipboardUnavailable",
            format!("read clipboard failed: {e}"),
        )
    })?;
    let current_fp = fingerprint(&current);
    let mut guard = rt.session.lock().expect("wisp session");
    let Some(active) = guard.as_mut() else {
        return Err(err("wisp.noSession", "no protection session"));
    };
    let matches_masked = current_fp.is_some_and(|fp| fp == active.meta.masked_fingerprint);
    match apply_restore_outcome(&mut active.meta, matches_masked) {
        Ok(()) => {
            // 写原文前登记 M2.3 抑制（§13）
            if let Some(fp) = fingerprint(&payload.original_text) {
                *state.clipboard.suppress.lock().expect("suppress") = Some(fp.to_lowercase());
            }
            clip.set_text(payload.original_text)
                .map_err(|e| err("wisp.restoreWriteFailed", format!("{e}")))?;
            let meta = active.meta.clone();
            session::clear_snapshot(rt.dir());
            active.payload = None;
            rt.push_event(WispEventDto {
                at_ms: now,
                kind: "restored".into(),
                level: meta.original_level,
                risk: None,
                action: None,
                target: None,
                rule_ids: Vec::new(),
            });
            tracing::info!(target: "wisp", session = %meta.session_id, "original restored");
            Ok(meta)
        }
        Err(_reason) => {
            // 恢复被跳过是**正常事实**（用户复制了新内容）——返回会话状态
            // 而非 Err（§19 如实呈现，不升级；会话终结不占用名额）
            let meta = active.meta.clone();
            session::clear_snapshot(rt.dir());
            active.payload = None;
            rt.push_event(WispEventDto {
                at_ms: now,
                kind: "restoreSkipped".into(),
                level: meta.original_level,
                risk: None,
                action: None,
                target: None,
                rule_ids: Vec::new(),
            });
            Ok(meta)
        }
    }
}

/// 放弃会话（不动剪贴板——masked 内容可能已被粘贴；如实保留现状）。
#[tauri::command]
#[specta::specta]
pub fn wisp_cancel_session(app: tauri::AppHandle) -> Result<(), IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.wisp;
    let mut guard = rt.session.lock().expect("wisp session");
    let Some(active) = guard.as_mut() else {
        return Err(err("wisp.noSession", "no protection session"));
    };
    active.meta.state = SessionState::Cancelled;
    session::clear_snapshot(rt.dir());
    active.payload = None;
    let level = active.meta.original_level;
    rt.push_event(WispEventDto {
        at_ms: now_ms(),
        kind: "cancelled".into(),
        level,
        risk: None,
        action: None,
        target: None,
        rule_ids: Vec::new(),
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn wisp_events(app: tauri::AppHandle, limit: Option<u32>) -> Vec<WispEventDto> {
    let state = app.state::<crate::state::AppState>();
    state
        .wisp
        .recent_events(limit.unwrap_or(50).min(EVENTS_CAP as u32) as usize)
}

#[tauri::command]
#[specta::specta]
pub fn wisp_clear_events(app: tauri::AppHandle) {
    let state = app.state::<crate::state::AppState>();
    state.wisp.clear_events();
}

/// 经 Weave 的粘贴意图防线（§14 Limited 能力下的可靠设防点）：
/// recopy 前评估策略——Block ⇒ 拒绝；Warn ⇒ 返回决策等确认；
/// Allow/确认后委托 M2.3 `clipboard_recopy`（复用，不重建 §27）。
/// 返回 Ok(Some(decision)) = 警告待确认；Ok(None) = 已放行完成。
#[tauri::command]
#[specta::specta]
pub fn wisp_recopy_guarded(
    app: tauri::AppHandle,
    id: String,
    confirmed: bool,
) -> Result<Option<PasteDecision>, IpcError> {
    use weave_clipboard::ContentKind;
    let state = app.state::<crate::state::AppState>();
    let rt = &state.wisp;
    let text = {
        let entries = state.clipboard.entries.lock().expect("entries");
        let Some(entry) = entries.iter().find(|e| e.id == id) else {
            return Err(err("wisp.unknownEntry", format!("no entry '{id}'")));
        };
        if !matches!(entry.kind, ContentKind::Text | ContentKind::Html) {
            None
        } else {
            weave_clipboard::CapturePipeline::load_payload(&state.clipboard.store, entry)
                .map_err(IpcError::from)?
                .and_then(|bytes| String::from_utf8(bytes).ok())
        }
    };
    if let Some(text) = text {
        let result = analyze_text(&text, &rt.toggles());
        if result.level != SensitivityLevel::Normal {
            let target = foreground_target();
            let decision = crate::wisp::evaluate(&result, &target, &rt.global_policy());
            let record = |kind: &str, action: PasteAction| WispEventDto {
                at_ms: now_ms(),
                kind: kind.to_string(),
                level: result.level,
                risk: Some(decision.risk),
                action: Some(action),
                target: target.process.clone(),
                rule_ids: decision.rule_ids.clone(),
            };
            match decision.action {
                PasteAction::Block => {
                    let event = record("blocked", decision.action);
                    rt.push_event(event);
                    return Err(err(
                        "wisp.blockedByPolicy",
                        format!("paste blocked: {}", decision.reasons.join("; ")),
                    ));
                }
                PasteAction::Warn if !confirmed => {
                    let event = record("warned", decision.action);
                    rt.push_event(event);
                    return Ok(Some(decision));
                }
                _ => {
                    let event = record("allowed", decision.action);
                    rt.push_event(event);
                }
            }
        }
    }
    crate::clipboard_service::clipboard_recopy(app, id).map(|()| None)
}
