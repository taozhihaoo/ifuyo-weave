//! M2.12 Smart Processing 服务（IPC 边界）。领域在 `smart/`。
//!
//! 数据流（§1）：Input → Analyze(Job) → Suggestions → Review(UI) →
//! **smart_build_plan（Suggestion → M2.0 Plan）** → 既有 execute_plan /
//! execute_recycle_plan → 既有 undo_operation → History。
//!
//! - **执行零新代码（§22 P0）**：Rename/Move ⇒ 直接构造 `Plan` 进
//!   PlanCache（items 携带 source 快照 ⇒ 既有 Revalidate = STALE 防护
//!   §23；冲突/环在构建期标记 §24/§25）；Delete ⇒ DuplicateRecycle 计划
//!   ⇒ 既有 execute_recycle_plan（回收站可恢复）。
//! - **本地优先（§12/§14/§47）**：Provider 默认 Disabled；本服务不持有
//!   云 Provider——trait + schema 验证已就位（§30），云实现属后续 opt-in。
//! - **隐私（§16/§34）**：session 只存计数与元数据，绝不存建议正文/路径
//!   之外的敏感值；Provider 输入默认 Metadata Only（§15）。

#![expect(clippy::result_large_err)] // D10：IpcError DTO 体积豁免

use std::sync::Mutex;

use tauri::Manager;

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;
use crate::ops_dto::PlanDto;
use crate::smart::model::{
    SmartAnalysisResult, SmartError, SmartProviderStatus, SmartRiskLevel, SmartSuggestion,
    SuggestionType,
};
use crate::smart::rules::AnalyzeMode;
use crate::smart::{collect_facts, rules};
use weave_core::prelude::{
    CancellationToken, OperationId, OperationKind, Plan, PlanItem, PlanItemStatus,
};

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    IpcError::from(
        weave_core::prelude::WeaveError::io(code, message).with_location("smart_service"),
    )
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

pub const MAX_BATCH_INPUTS: usize = 200;

#[derive(Debug, Clone)]
struct CachedAnalysis {
    result: SmartAnalysisResult,
    /// 每条建议的 source 快照（Plan 构建与 STALE 防护 §23）。
    snapshots: std::collections::HashMap<String, (u64, Option<std::time::SystemTime>)>,
}

/// Smart 运行时（单槽结果 + 会话历史有界 20 §34）。
pub struct SmartRuntime {
    latest: Mutex<Option<CachedAnalysis>>,
    sessions: Mutex<Vec<crate::smart::SmartSessionSummary>>,
    history_path: std::path::PathBuf,
}

impl SmartRuntime {
    pub fn new(history_dir: &std::path::Path) -> Self {
        let history_path = history_dir.join("smart-sessions.json");
        let sessions = std::fs::read(&history_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            latest: Mutex::new(None),
            sessions: Mutex::new(sessions),
            history_path,
        }
    }

    fn record_session(&self, summary: crate::smart::SmartSessionSummary) {
        let mut sessions = self.sessions.lock().expect("smart sessions");
        sessions.push(summary);
        let len = sessions.len();
        if len > 20 {
            sessions.drain(0..len - 20);
        }
        if let Ok(json) = serde_json::to_string_pretty(&*sessions) {
            let tmp = self.history_path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &self.history_path);
            }
        }
    }
}

// ── Analyze（§18 Smart Job：JobTracker + 进度 + 取消 + 失败隔离）──

#[tauri::command]
#[specta::specta]
pub fn smart_analyze(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    modes: Vec<AnalyzeMode>,
) -> Result<JobHandleDto, IpcError> {
    if inputs.is_empty() || inputs.len() > MAX_BATCH_INPUTS {
        return Err(err(
            "smart.batchSizeInvalid",
            format!("inputs must be 1..={MAX_BATCH_INPUTS} paths/dirs"),
        ));
    }
    if modes.is_empty() {
        return Err(err("smart.modesEmpty", "select at least one analysis mode"));
    }
    let state = app.state::<crate::state::AppState>();
    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_task = job_id.clone();
    let modes2 = modes.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let (result, snapshots) = run_analysis(&inputs, &modes2, &cancel, &sink);
        let summary = crate::smart::SmartSessionSummary {
            session_id: result.session_id.clone(),
            created_at_ms: now_ms(),
            modes: modes2.clone(),
            roots: inputs.clone(),
            stats: result.stats.clone(),
            provider: "local-rules".to_string(),
        };
        app_state.smart.record_session(summary);
        {
            let mut latest = app_state.smart.latest.lock().expect("smart latest");
            let _ = &result.session_id;
            *latest = Some(CachedAnalysis {
                result: result.clone(),
                snapshots,
            });
        }
        app_state
            .jobs
            .finish(&job_id_task, crate::jobs::JobOutcome::WebCompleted);
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

fn run_analysis(
    inputs: &[String],
    modes: &[AnalyzeMode],
    cancel: &CancellationToken,
    sink: &crate::jobs::ProgressSink,
) -> (
    SmartAnalysisResult,
    std::collections::HashMap<String, (u64, Option<std::time::SystemTime>)>,
) {
    let progress_op = weave_core::prelude::OperationId::generate();
    let want_hashes = modes.contains(&AnalyzeMode::Cleanup);
    let mut stats = crate::smart::SmartStats::default();
    let mut suggestions: Vec<SmartSuggestion> = Vec::new();
    let mut snapshots = std::collections::HashMap::new();
    let session_id = format!("smart-{}", now_ms() as i64);

    let (facts, errors) = match collect_facts(inputs, want_hashes, cancel, &mut |done| {
        sink.report(&weave_core::prelude::Progress::running(
            progress_op.clone(),
            done as u64,
            None,
        ));
    }) {
        Ok(v) => v,
        Err(e) => {
            return (
                SmartAnalysisResult {
                    session_id,
                    stats,
                    suggestions,
                    errors: vec![SmartError {
                        path: String::new(),
                        code: e.code,
                    }],
                    provider_status: SmartProviderStatus::Disabled,
                },
                snapshots,
            );
        }
    };
    stats.files_analyzed = facts.len() as f64;
    let errors: Vec<SmartError> = errors
        .into_iter()
        .map(|(path, code)| SmartError { path, code })
        .collect();
    for f in &facts {
        snapshots.insert(
            f.path.clone(),
            (
                f.size,
                std::fs::metadata(&f.path)
                    .ok()
                    .and_then(|m| m.modified().ok()),
            ),
        );
    }
    let idx = 0usize;
    for f in &facts {
        if cancel.is_cancelled() {
            break;
        }
        let mut mode_hits = 0usize;
        if modes.contains(&AnalyzeMode::Classify) {
            let c = rules::classify(f);
            crate::smart::model::tally(&mut stats, c.confidence);
            let suggestion = SmartSuggestion {
                id: format!("s-{idx}-{mode_hits}"),
                suggestion_type: SuggestionType::Classify,
                target_path: f.path.clone(),
                original_value: String::new(),
                suggested_value: c.category.clone(),
                reason: format!("type {} (extension evidence)", c.file_type),
                evidence: c.evidence.clone(),
                confidence: c.confidence,
                risk: SmartRiskLevel::Low,
                conflict: None,
                alternatives: c.alternatives.clone(),
            };
            if c.confidence != crate::smart::model::SmartConfidence::Unknown {
                suggestions.push(suggestion);
                stats.suggestions += 1.0;
            }
            mode_hits += 1;
        }
        if modes.contains(&AnalyzeMode::Rename)
            && let Some(mut s) = rules::rename_suggestion(f)
        {
            crate::smart::model::tally(&mut stats, s.confidence);
            s.id = format!("s-{idx}-{mode_hits}");
            // 冲突预检（§24：同目录已存在同名目标）
            let dir = std::path::Path::new(&f.path)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            let target = dir.join(&s.suggested_value);
            if target.exists() {
                s.conflict = Some(format!("target exists: {}", target.display()));
            }
            suggestions.push(s);
            stats.suggestions += 1.0;
            mode_hits += 1;
        }
        if modes.contains(&AnalyzeMode::Organize)
            && let Some(mut s) = rules::organize_suggestion(f)
        {
            crate::smart::model::tally(&mut stats, s.confidence);
            s.id = format!("s-{idx}-{mode_hits}");
            suggestions.push(s);
            stats.suggestions += 1.0;
            mode_hits += 1;
        }
        let _ = mode_hits;
    }
    if modes.contains(&AnalyzeMode::Cleanup) {
        for mut s in rules::cleanup_candidates(&facts) {
            crate::smart::model::tally(&mut stats, s.confidence);
            s.id = format!("s-cleanup-{}", suggestions.len());
            // 删除候选冲突：无（回收站可恢复）；但文件消失 ⇒ Invalid（构建期再验）
            suggestions.push(s);
            stats.suggestions += 1.0;
        }
    }
    let _ = idx;
    (
        SmartAnalysisResult {
            session_id,
            stats,
            suggestions,
            errors,
            provider_status: SmartProviderStatus::Disabled,
        },
        snapshots,
    )
}

#[tauri::command]
#[specta::specta]
pub fn smart_get_result(
    app: tauri::AppHandle,
    job_id: String,
) -> Result<SmartAnalysisResult, IpcError> {
    let _ = job_id; // 单槽语义：最近一次分析（§46 内存纪律）
    let state = app.state::<crate::state::AppState>();
    let guard = state.smart.latest.lock().expect("smart latest");
    guard
        .as_ref()
        .map(|c| c.result.clone())
        .ok_or_else(|| err("smart.noResult", "no analysis has run in this session"))
}

/// 已接受建议 → M2.0 Plan（§22：Suggestion → Action → Plan，执行走既有
/// execute_plan / execute_recycle_plan IPC；STALE 由 PlanItem source 快照
/// 的 Revalidate 既有机制防护 §23）。
#[tauri::command]
#[specta::specta]
pub fn smart_build_plan(
    app: tauri::AppHandle,
    accepted_ids: Vec<String>,
) -> Result<SmartPlanBundleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let guard = state.smart.latest.lock().expect("smart latest");
    let cached = guard
        .as_ref()
        .ok_or_else(|| err("smart.noResult", "no analysis has run in this session"))?;
    let accepted: Vec<&SmartSuggestion> = cached
        .result
        .suggestions
        .iter()
        .filter(|s| accepted_ids.contains(&s.id))
        .collect();
    if accepted.is_empty() {
        return Err(err("smart.emptySelection", "no suggestions selected"));
    }

    // ── Rename/Move 计划（per-file target 映射；冲突/环/NoOp 构建期判定）──
    let mut rename_items: Vec<PlanItem> = Vec::new();
    let mut move_items: Vec<PlanItem> = Vec::new();
    let mut recycle_sources: Vec<String> = Vec::new();
    let mut rename_count = 0f64;
    let mut move_count = 0f64;
    for s in &accepted {
        match s.suggestion_type {
            SuggestionType::Rename => {
                let source = std::path::PathBuf::from(&s.target_path);
                let dir = source.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                let target = dir.join(&s.suggested_value);
                let (size, modified) = cached
                    .snapshots
                    .get(&s.target_path)
                    .cloned()
                    .unwrap_or((0, None));
                let status = if s.target_path == target.to_string_lossy() {
                    PlanItemStatus::NoOp
                } else if target.exists() {
                    PlanItemStatus::Conflict
                } else {
                    PlanItemStatus::Ready
                };
                rename_items.push(PlanItem {
                    item_id: format!("item_{rename_count:04}"),
                    source_path: s.target_path.clone(),
                    target_path: target.to_string_lossy().into_owned(),
                    status,
                    collision: if target.exists() {
                        weave_core::prelude::CollisionKind::ExistingTarget
                    } else {
                        weave_core::prelude::CollisionKind::None
                    },
                    source_size: Some(size),
                    source_modified: modified,
                    source_digest: None,
                    warnings: vec![format!("smart: {}", s.reason)],
                    errors: Vec::new(),
                });
                rename_count += 1.0;
            }
            SuggestionType::Organize => {
                let source = std::path::PathBuf::from(&s.target_path);
                let file_name = source
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let root = &s.original_value; // organize 的 original_value = 原父目录
                let target_dir = if std::path::Path::new(root).join(&s.suggested_value).exists()
                    || std::path::Path::new(root).is_dir()
                {
                    std::path::Path::new(root).join(&s.suggested_value)
                } else {
                    // original_value 为空 ⇒ 用 target 所在目录
                    source
                        .parent()
                        .map(|p| p.join(&s.suggested_value))
                        .unwrap_or_else(|| std::path::PathBuf::from(&s.suggested_value))
                };
                let target = target_dir.join(&file_name);
                let (size, modified) = cached
                    .snapshots
                    .get(&s.target_path)
                    .cloned()
                    .unwrap_or((0, None));
                let status = if target.to_string_lossy() == s.target_path {
                    PlanItemStatus::NoOp
                } else if target.exists() {
                    PlanItemStatus::Conflict
                } else {
                    PlanItemStatus::Ready
                };
                move_items.push(PlanItem {
                    item_id: format!("item_{move_count:04}"),
                    source_path: s.target_path.clone(),
                    target_path: target.to_string_lossy().into_owned(),
                    status,
                    collision: if target.exists() {
                        weave_core::prelude::CollisionKind::ExistingTarget
                    } else {
                        weave_core::prelude::CollisionKind::None
                    },
                    source_size: Some(size),
                    source_modified: modified,
                    source_digest: None,
                    warnings: vec![format!("smart: {}", s.reason)],
                    errors: Vec::new(),
                });
                move_count += 1.0;
            }
            SuggestionType::DeleteCandidate => {
                recycle_sources.push(s.target_path.clone());
            }
            SuggestionType::Classify => {
                // Classify 不产生文件操作（纯标注）——执行层跳过
            }
        }
    }
    // 批内 rename 环检测（§25：A→B、B→C … 目标与批内其他 source 相同 ⇒
    // Conflict（Cycle）——不逐个执行造成碰撞；两阶段策略属 P1）
    for item in &mut rename_items {
        if item.status != PlanItemStatus::Ready {
            continue;
        }
        if accepted.iter().any(|other| {
            other.target_path == item.target_path && other.target_path != item.source_path
        }) {
            item.status = PlanItemStatus::Conflict;
            item.collision = weave_core::prelude::CollisionKind::Cycle;
            item.warnings
                .push("smart: batch rename cycle detected (two-phase strategy is P1)".into());
        }
    }
    drop(guard);

    let mut bundle = SmartPlanBundleDto {
        rename_plan: None,
        move_plan: None,
        delete_plan: None,
    };
    let plans = &state.plans;
    if !rename_items.is_empty() {
        let plan = Plan {
            operation_id: OperationId::generate(),
            kind: OperationKind::Rename,
            created_at: std::time::SystemTime::now(),
            items: rename_items,
        };
        plans.insert(&plan);
        bundle.rename_plan = Some(PlanDto::from_plan(&plan));
    }
    if !move_items.is_empty() {
        let plan = Plan {
            operation_id: OperationId::generate(),
            kind: OperationKind::Move,
            created_at: std::time::SystemTime::now(),
            items: move_items,
        };
        plans.insert(&plan);
        bundle.move_plan = Some(PlanDto::from_plan(&plan));
    }
    if !recycle_sources.is_empty() {
        // Delete ⇒ DuplicateRecycle 计划（target 留空 = 回收站语义；与 M3
        // build_recycle_plan 产物同构——经既有 execute_recycle_plan 执行，
        // 回收站可恢复 §29）
        let items: Vec<PlanItem> = recycle_sources
            .iter()
            .enumerate()
            .map(|(idx, path)| {
                let meta = std::fs::metadata(path).ok();
                PlanItem {
                    item_id: format!("item_{idx:04}"),
                    source_path: path.clone(),
                    target_path: String::new(),
                    status: PlanItemStatus::Ready,
                    collision: weave_core::prelude::CollisionKind::None,
                    source_size: meta.as_ref().map(|m| m.len()),
                    source_modified: meta.and_then(|m| m.modified().ok()),
                    source_digest: None,
                    warnings: vec!["smart: cleanup candidate (recycle bin, recoverable)".into()],
                    errors: Vec::new(),
                }
            })
            .collect();
        let plan = Plan {
            operation_id: OperationId::generate(),
            kind: OperationKind::DuplicateRecycle,
            created_at: std::time::SystemTime::now(),
            items,
        };
        plans.insert(&plan);
        bundle.delete_plan = Some(PlanDto::from_plan(&plan));
    }
    if bundle.rename_plan.is_none() && bundle.move_plan.is_none() && bundle.delete_plan.is_none() {
        return Err(err(
            "smart.noExecutableSuggestions",
            "accepted suggestions contain no file operations",
        ));
    }
    Ok(bundle)
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmartPlanBundleDto {
    /// Rename 建议计划（经既有 execute_plan 执行——Revalidate/Undo 免费）。
    pub rename_plan: Option<PlanDto>,
    /// Organize 建议计划（Move kind——目标目录自动创建，既有执行器语义）。
    pub move_plan: Option<PlanDto>,
    /// Cleanup 候选计划（DuplicateRecycle kind——经既有
    /// execute_recycle_plan 执行，回收站可恢复 §29）。
    pub delete_plan: Option<PlanDto>,
}

// ── Provider 状态 / 会话历史 ──

#[tauri::command]
#[specta::specta]
pub fn smart_provider_status() -> crate::smart::model::SmartProviderStatus {
    // §14 默认 Disabled：本地规则引擎完整可用；云 Provider 属后续 opt-in
    crate::smart::model::SmartProviderStatus::Disabled
}

#[tauri::command]
#[specta::specta]
pub fn smart_history(app: tauri::AppHandle) -> Vec<crate::smart::SmartSessionSummary> {
    let state = app.state::<crate::state::AppState>();
    let mut sessions = state.smart.sessions.lock().expect("smart sessions").clone();
    sessions.reverse();
    sessions
}
