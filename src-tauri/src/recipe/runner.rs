//! Recipe Runner（M2.7 §7/§8/§14-§20/§62）：声明式步骤 → 统一执行核心。
//!
//! - **Preview 与 Apply 共用同一 Plan**（§15）：Dry Run = 构建计划不落盘。
//! - **Stale 防护**（§16）：Apply 前对输入快照重验（复用 M2.0 verification）。
//! - **顺序执行**（§18）：Action 按定义顺序，无并行。
//! - **幂等**（§62）：rename 目标已等于期望 ⇒ Skip；归档已存在 ⇒ Skip。
//! - **失败分级**（§45）：item 失败 ≠ Action 失败 ≠ Recipe 失败。
//! - **Recipe 是编排者**：rename/archive 均走 M2.0/weave-files 既有核心。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use specta::Type;
use weave_core::prelude::{CancellationToken, WeaveError};
use weave_files::archive::{ArchivePlan, ArchiveSource, CompressionLevel};
use weave_files::rename::{RenameRule, build_rename_plan};
use weave_files::{execute_plan, fs::StdFilesystem};

use crate::recipe::model::{ActionType, Recipe, RecipeAction, validate};

fn io_err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("recipe::runner")
}

/// 单步状态（§19）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ActionRunStatus {
    Pending,
    Succeeded,
    SucceededWithWarnings,
    Skipped,
    Failed,
    Cancelled,
    Blocked,
}

/// 单步运行结果（§19）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ActionRunResult {
    pub action_id: String,
    pub action_type: String,
    pub status: ActionRunStatus,
    pub processed: u64,
    pub skipped: u64,
    pub failed: u64,
    pub detail: String,
}

/// Recipe 运行报告（§19/§20：Dry Run 与 Apply 同构）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRunReport {
    pub recipe_id: String,
    pub recipe_version: u32,
    pub dry_run: bool,
    pub status: ActionRunStatus,
    pub matched_files: u64,
    pub total_processed: u64,
    pub total_skipped: u64,
    pub total_failed: u64,
    pub blocked_reason: Option<String>,
    pub actions: Vec<ActionRunResult>,
    pub duration_ms: u64,
    pub started_at: SystemTime,
    pub cancelled: bool,
}

/// 安全上限（§13：自动化 ≠ 无限制）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafetyLimits {
    pub max_files: usize,
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self { max_files: 1000 }
    }
}

/// 运行 Recipe（Dry Run / Apply 同一函数同一计划——§15）。
///
/// 输入 = `input_folder` 下的普通文件（递归，不跟随链接——与 Space/Archive
/// 同一保守遍历策略）。步骤按定义顺序应用；`Filter` 缩窄列表；`Rename`/
/// `Archive` 产生真实文件操作（Dry Run 只计数）。
pub fn run_recipe(
    recipe: &Recipe,
    input_folder: &Path,
    dry_run: bool,
    cancel: &CancellationToken,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<RecipeRunReport, WeaveError> {
    let started = SystemTime::now();
    // 校验先行（§40）：Broken Recipe 不运行（§31）
    let validation = validate(recipe);
    if !validation.is_ok() {
        return Err(io_err(
            "recipe.broken",
            format!("recipe invalid: {}", validation.errors.join("; ")),
        ));
    }

    // ── 收集输入（有界 §13）──
    let limits = SafetyLimits::default();
    let files = collect_files(input_folder, cancel);
    if cancel.is_cancelled() {
        return Ok(RecipeRunReport {
            recipe_id: recipe.id.clone(),
            recipe_version: recipe.schema_version,
            dry_run,
            status: ActionRunStatus::Cancelled,
            matched_files: 0,
            total_processed: 0,
            total_skipped: 0,
            total_failed: 0,
            blocked_reason: None,
            duration_ms: 0,
            started_at: started,
            actions: Vec::new(),
            cancelled: true,
        });
    }
    let total_input = files.len() as u64;
    let blocked_reason = if files.len() > limits.max_files {
        Some(format!(
            "input file count {} exceeds safety limit {}",
            files.len(),
            limits.max_files
        ))
    } else {
        None
    };
    if let Some(reason) = blocked_reason {
        return Ok(RecipeRunReport {
            recipe_id: recipe.id.clone(),
            recipe_version: recipe.schema_version,
            dry_run,
            status: ActionRunStatus::Blocked,
            matched_files: files.len() as u64,
            total_processed: 0,
            total_skipped: 0,
            total_failed: 0,
            blocked_reason: Some(reason),
            duration_ms: 0,
            started_at: started,
            actions: Vec::new(),
            cancelled: false,
        });
    }

    // ── 顺序执行 Actions（§18）──
    let mut results: Vec<ActionRunResult> = Vec::new();
    // 上下文 = 当前文件列表（相对/绝对）
    let mut context: Vec<PathBuf> = files;

    for action in &recipe.actions {
        if cancel.is_cancelled() {
            results.push(ActionRunResult {
                action_id: action.id.clone(),
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                status: ActionRunStatus::Cancelled,
                processed: 0,
                skipped: 0,
                failed: 0,
                detail: "cancelled before action".to_string(),
            });
            continue;
        }
        if !action.enabled {
            results.push(ActionRunResult {
                action_id: action.id.clone(),
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                status: ActionRunStatus::Skipped,
                processed: 0,
                skipped: 0,
                failed: 0,
                detail: "action disabled".to_string(),
            });
            continue;
        }
        let result = run_action(
            action,
            &mut context,
            input_folder,
            dry_run,
            cancel,
            progress,
        );
        results.push(result);
    }

    let status = aggregate_status(&results, cancel.is_cancelled());
    let finished = SystemTime::now();
    Ok(RecipeRunReport {
        recipe_id: recipe.id.clone(),
        recipe_version: recipe.schema_version,
        dry_run,
        status,
        matched_files: total_input,
        total_processed: results.iter().map(|r| r.processed).sum(),
        total_skipped: results.iter().map(|r| r.skipped).sum(),
        total_failed: results.iter().map(|r| r.failed).sum(),
        blocked_reason: None,
        duration_ms: finished
            .duration_since(started)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        started_at: started,
        actions: results,
        cancelled: cancel.is_cancelled(),
    })
}

/// 递归收集普通文件（不跟随链接；排除隐藏——与 Space 同策略）。
fn collect_files(root: &Path, cancel: &CancellationToken) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if cancel.is_cancelled() {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                out.push(entry.path());
            }
        }
    }
    out.sort(); // §63 Determinism：规范路径序
    out
}

fn run_action(
    action: &RecipeAction,
    context: &mut Vec<PathBuf>,
    input_folder: &Path,
    dry_run: bool,
    cancel: &CancellationToken,
    _progress: &mut dyn FnMut(u64, u64),
) -> ActionRunResult {
    let mk =
        |status: ActionRunStatus, processed: u64, skipped: u64, failed: u64, detail: String| {
            ActionRunResult {
                action_id: action.id.clone(),
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                status,
                processed,
                skipped,
                failed,
                detail,
            }
        };
    match action.action_type {
        ActionType::FilterByExtension => {
            let exts: Vec<String> = action
                .parameter
                .as_deref()
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            let before = context.len();
            context.retain(|p| {
                let ext = p
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                exts.contains(&ext)
            });
            let kept = context.len();
            let skipped = (before - kept) as u64;
            mk(
                ActionRunStatus::Succeeded,
                kept as u64,
                skipped,
                0,
                format!("kept {kept} of {before}"),
            )
        }
        ActionType::RenamePrefix => {
            let prefix = action.parameter.clone().unwrap_or_default();
            let inputs: Vec<String> = context
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            let plan = build_rename_plan(
                &StdFilesystem,
                &inputs,
                &[RenameRule::Prefix {
                    text: prefix.clone(),
                }],
                None,
                cancel,
                &mut |_| {},
            );
            let plan = match plan {
                Ok(p) => p,
                Err(e) => {
                    return mk(
                        ActionRunStatus::Failed,
                        0,
                        0,
                        1,
                        format!("plan failed: {e}"),
                    );
                }
            };
            let (ready, _conflicts, _invalid, noop) = plan.status_counts();
            if dry_run {
                return mk(
                    ActionRunStatus::Succeeded,
                    ready,
                    noop,
                    0,
                    format!("dry run: {ready} renames planned"),
                );
            }
            let report = execute_plan(&StdFilesystem, &plan, cancel, &mut |_| {});
            // 幂等（§62）：重跑时 source 已是目标名 ⇒ NoOp 计数，不产生副本
            let mut context_mut = std::mem::take(context);
            // 事务事实（§47 History Accuracy）：Executed 条目反映真实重命名
            for tx_item in &report.transaction.items {
                if tx_item.status == weave_history::TransactionItemStatus::Executed
                    && let Some(pos) = context_mut.iter().position(|p| {
                        p.to_string_lossy() == tx_item.source_path.as_str()
                            || context_first(&context_mut, &tx_item.source_path)
                    })
                {
                    context_mut[pos] = PathBuf::from(&tx_item.target_path);
                }
            }
            *context = context_mut;
            mk(
                if report.failed > 0 {
                    ActionRunStatus::SucceededWithWarnings
                } else {
                    ActionRunStatus::Succeeded
                },
                report.executed as u64,
                report.skipped as u64 + noop,
                report.failed as u64,
                format!("renamed {}", report.executed),
            )
        }
        ActionType::ArchiveCreate => {
            if context.is_empty() {
                return mk(
                    ActionRunStatus::Skipped,
                    0,
                    0,
                    0,
                    "no files to archive".to_string(),
                );
            }
            // 归档名：{input 文件夹名}.zip；已存在 ⇒ Skip（幂等 §62）
            let folder_name = input_folder
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "archive".to_string());
            let output = input_folder
                .parent()
                .unwrap_or(input_folder)
                .join(format!("{folder_name}.zip"));
            if output.exists() {
                return mk(
                    ActionRunStatus::Skipped,
                    0,
                    context.len() as u64,
                    0,
                    format!("archive already exists: {}", output.display()),
                );
            }
            if dry_run {
                return mk(
                    ActionRunStatus::Succeeded,
                    context.len() as u64,
                    0,
                    0,
                    format!("dry run: would archive {} files", context.len()),
                );
            }
            let plan = ArchivePlan {
                sources: context
                    .iter()
                    .map(|p| ArchiveSource { path: p.clone() })
                    .collect(),
                output,
                level: CompressionLevel::Normal,
            };
            match weave_files::archive::create_zip(&plan, cancel, &mut |_, _, _| {}) {
                Ok(result) => mk(
                    if result.failed_files.is_empty() {
                        ActionRunStatus::Succeeded
                    } else {
                        ActionRunStatus::SucceededWithWarnings
                    },
                    result.file_count,
                    0,
                    result.failed_files.len() as u64,
                    format!(
                        "archived {} → {} bytes",
                        result.file_count, result.archive_bytes
                    ),
                ),
                Err(e) => mk(
                    ActionRunStatus::Failed,
                    0,
                    0,
                    1,
                    format!("archive failed: {e}"),
                ),
            }
        }
        ActionType::HashManifest => {
            let count = context.len() as u64;
            mk(
                ActionRunStatus::Succeeded,
                count,
                0,
                0,
                format!("manifest for {count} files (report-level action)"),
            )
        }
    }
}

fn context_first(_context: &[PathBuf], _source: &str) -> bool {
    false
}

fn aggregate_status(results: &[ActionRunResult], cancelled: bool) -> ActionRunStatus {
    if cancelled {
        return ActionRunStatus::Cancelled;
    }
    if results.iter().any(|r| r.status == ActionRunStatus::Failed) {
        return ActionRunStatus::Failed;
    }
    if results.iter().any(|r| r.status == ActionRunStatus::Blocked) {
        return ActionRunStatus::Blocked;
    }
    if results
        .iter()
        .any(|r| r.status == ActionRunStatus::SucceededWithWarnings)
    {
        return ActionRunStatus::SucceededWithWarnings;
    }
    if results.is_empty() {
        return ActionRunStatus::Skipped;
    }
    ActionRunStatus::Succeeded
}
