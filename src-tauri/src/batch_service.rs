//! M7（上）：Batch Job IPC 命令（§87-§90）。
//!
//! - `batch_preview`：同步——同引擎模拟（§20-§23），Export 不落盘；
//! - `batch_execute`：任务化——JobTracker 注册，任务内快照重校验（§75）
//!   后 execute_plan；进度/取消经既有 get_job / cancel_job（§89/§90）。
//!
//! 本层只做 DTO 映射与路径校验，引擎逻辑全部在 weave-batch（§86）。

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_batch::{JobResult, StageSpec, TextOpSpec};
use weave_core::prelude::{OperationId, Progress, WeaveError};

use crate::commands::IpcError;
use crate::jobs::JobOutcome;

// IPC 边界与 D10 同理：Err DTO 体积不构成热路径问题。
#[allow(clippy::result_large_err)] // D10：Err DTO 体积豁免
fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("batch_service")
        .into()
}

/// 批量文本算子（镜像 weave_batch::TextOpSpec；§19 可序列化计划）。
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BatchTextOpDto {
    TrimLines,
    Lowercase,
    Uppercase,
    Replace {
        find: String,
        replace_with: String,
        case_sensitive: bool,
    },
}

/// Pipeline 阶段（镜像 weave_batch::StageSpec；Linear Only，§11-§13）。
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BatchStageDto {
    Source,
    Filter {
        extensions_in: Vec<String>,
        max_bytes: Option<f64>,
    },
    TextTransform {
        operations: Vec<BatchTextOpDto>,
    },
    ImageResize {
        width: f64,
        height: f64,
        mode: String,
        prevent_upscale: bool,
    },
    Encode {
        /// png | jpeg | webp | bmp | tiff | txt
        format: String,
        quality: Option<f64>,
    },
    Export {
        destination_dir: String,
        overwrite: bool,
    },
}

/// 批处理选项（§19 JobPlan 的 IPC 投影）。
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchOptionsDto {
    pub continue_on_error: bool,
    pub overwrite_existing: bool,
}

/// 阶段结果（§28）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchStageOutcomeDto {
    pub index: f64,
    pub stage: String,
    pub ok: bool,
    pub note: String,
}

/// 条目结果（§27）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemResultDto {
    pub item_id: String,
    pub source: String,
    pub output: Option<String>,
    /// success | failed | skipped | cancelled
    pub status: String,
    pub error: Option<String>,
    pub stages: Vec<BatchStageOutcomeDto>,
    pub input_bytes: f64,
    pub output_bytes: f64,
}

/// Job 结果（§26 Result Model；preview=true 表示模拟结果 §21）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobResultDto {
    pub items: Vec<BatchItemResultDto>,
    pub total: f64,
    pub succeeded: f64,
    pub failed: f64,
    pub skipped: f64,
    pub cancelled: f64,
    pub input_bytes: f64,
    pub output_bytes: f64,
    pub preview: bool,
    /// 关联 History 操作（撤销入口；unavailable 时 None）。
    pub operation_id: Option<String>,
}

/// 任务启动句柄（与 M1 hash/scan、M3 recycle 一致：get_job 轮询）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobHandleDto {
    pub job_id: String,
}

fn text_op_spec(op: &BatchTextOpDto) -> TextOpSpec {
    match op {
        BatchTextOpDto::TrimLines => TextOpSpec::TrimLines,
        BatchTextOpDto::Lowercase => TextOpSpec::Lowercase,
        BatchTextOpDto::Uppercase => TextOpSpec::Uppercase,
        BatchTextOpDto::Replace {
            find,
            replace_with,
            case_sensitive,
        } => TextOpSpec::Replace {
            find: find.clone(),
            replace_with: replace_with.clone(),
            case_sensitive: *case_sensitive,
        },
    }
}

/// DTO → 领域 StageSpec（§58：适配层不做隐式转换，只做投影）。
#[allow(clippy::result_large_err)] // D10
fn stage_spec(stage: &BatchStageDto) -> Result<StageSpec, IpcError> {
    Ok(match stage {
        BatchStageDto::Source => StageSpec::Source,
        BatchStageDto::Filter {
            extensions_in,
            max_bytes,
        } => StageSpec::Filter {
            extensions_in: extensions_in
                .iter()
                .map(|e| e.to_ascii_lowercase())
                .collect(),
            max_bytes: max_bytes.map(|b| b as u64),
        },
        BatchStageDto::TextTransform { operations } => StageSpec::TextTransform {
            operations: operations.iter().map(text_op_spec).collect(),
        },
        BatchStageDto::ImageResize {
            width,
            height,
            mode,
            prevent_upscale,
        } => StageSpec::ImageResize {
            width: *width as u32,
            height: *height as u32,
            mode: mode.clone(),
            prevent_upscale: *prevent_upscale,
        },
        BatchStageDto::Encode { format, quality } => StageSpec::Encode {
            format: format.clone(),
            quality: quality.map(|q| q.clamp(1.0, 100.0) as u8),
        },
        BatchStageDto::Export {
            destination_dir,
            overwrite,
        } => {
            weave_core::prelude::validate_absolute_path(destination_dir)?;
            if !std::fs::metadata(destination_dir)
                .map(|m| m.is_dir())
                .unwrap_or(false)
            {
                return Err(err(
                    "batch.destinationMissing",
                    format!("destination dir does not exist: {destination_dir}"),
                ));
            }
            StageSpec::Export {
                destination_dir: std::path::PathBuf::from(destination_dir),
                overwrite: *overwrite,
            }
        }
    })
}

fn status_str(status: &weave_batch::ItemStatus) -> &'static str {
    match status {
        weave_batch::ItemStatus::Success => "success",
        weave_batch::ItemStatus::Failed => "failed",
        weave_batch::ItemStatus::Skipped => "skipped",
        weave_batch::ItemStatus::Cancelled => "cancelled",
    }
}

fn job_result_dto(result: &JobResult) -> BatchJobResultDto {
    BatchJobResultDto {
        items: result
            .items
            .iter()
            .map(|r| BatchItemResultDto {
                item_id: r.item_id.clone(),
                source: r.source.to_string_lossy().into_owned(),
                output: r.output.as_ref().map(|o| o.to_string_lossy().into_owned()),
                status: status_str(&r.status).to_string(),
                error: r.error.clone(),
                stages: r
                    .stage_results
                    .iter()
                    .map(|s| BatchStageOutcomeDto {
                        index: s.stage_index as f64,
                        stage: s.stage.clone(),
                        ok: s.ok,
                        note: s.note.clone(),
                    })
                    .collect(),
                input_bytes: r.input_bytes as f64,
                output_bytes: r.output_bytes as f64,
            })
            .collect(),
        total: result.total as f64,
        succeeded: result.succeeded as f64,
        failed: result.failed as f64,
        skipped: result.skipped as f64,
        cancelled: result.cancelled as f64,
        input_bytes: result.input_bytes as f64,
        output_bytes: result.output_bytes as f64,
        preview: result.preview,
        operation_id: None,
    }
}

/// 快照 + 计划构建（§10/§19/§56）：同步 fail-fast。
#[allow(clippy::result_large_err)] // D10
fn build_plan(
    inputs: &[String],
    stages: &[BatchStageDto],
    destination_dir: &str,
    options: &BatchOptionsDto,
) -> Result<weave_batch::JobPlan, IpcError> {
    if inputs.is_empty() {
        return Err(err("batch.emptySelection", "no input files"));
    }
    let pipeline = weave_batch::Pipeline {
        stages: stages.iter().map(stage_spec).collect::<Result<_, _>>()?,
    };
    let input_paths: Vec<std::path::PathBuf> =
        inputs.iter().map(std::path::PathBuf::from).collect();
    // Export stage 已带目标目录；计划级 destination_dir 与之保持一致
    weave_batch::build_job_plan(
        input_paths,
        pipeline,
        std::path::PathBuf::from(destination_dir),
        options.continue_on_error,
        options.overwrite_existing,
    )
    .map_err(|e| err("batch.planRejected", e))
}

/// Preview（§20-§23）：同引擎模拟；Export 不落盘；碰撞 = 潜在失败。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn batch_preview(
    inputs: Vec<String>,
    stages: Vec<BatchStageDto>,
    destination_dir: String,
    options: BatchOptionsDto,
) -> Result<BatchJobResultDto, IpcError> {
    let plan = build_plan(&inputs, &stages, &destination_dir, &options)?;
    Ok(job_result_dto(&weave_batch::preview_plan(&plan)))
}

/// Execute（§24-§26）：任务化执行；任务内先重校验快照（§75，
/// ChangedSincePreview/FileMissing ⇒ Job Failed），再 execute_plan。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn batch_execute(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    stages: Vec<BatchStageDto>,
    destination_dir: String,
    options: BatchOptionsDto,
) -> Result<BatchJobHandleDto, IpcError> {
    let plan = build_plan(&inputs, &stages, &destination_dir, &options)?;
    let state = app.state::<crate::state::AppState>();
    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        // §75 Execute Revalidation：Preview 与 Execute 之间输入可能已变化
        if let Err(e) = weave_batch::revalidate_snapshot(&plan.input_snapshot) {
            app_state.jobs.fail(
                &job_id_for_task,
                WeaveError::validation("batch.changedSincePreview", e)
                    .with_location("batch_service::batch_execute"),
            );
            return;
        }
        let total = plan.input_snapshot.len() as u64;
        let operation = OperationId::generate();
        let result = weave_batch::execute_plan(
            &plan,
            &cancel,
            Some(&mut |done, _desc| {
                sink.report(&Progress::running(operation.clone(), done, Some(total)));
            }),
        );
        // C3 §184/§246：批量产物入 History（kind=BatchExecute）。创建型产物
        // 撤销 = 删除已创建文件（stat 守卫）；覆盖写产物无备份 ⇒ 以
        // original_modified=None 记录，撤销时 creation-undo 守卫必报冲突
        // （拒绝删除，绝不静默二次破坏 §185）。历史写失败 ≠ 操作失败
        // （M2 §84）：结果照常返回，仅 undo 不可用。
        let history_dir = crate::rename_service::resolve_history_dir(&handle).ok();
        let mut history_ok = false;
        if let Some(hdir) = history_dir {
            let now = std::time::SystemTime::now();
            let mut created = 0usize;
            let mut replaced = 0usize;
            let items: Vec<weave_history::TransactionItem> = result
                .items
                .iter()
                .filter(|i| i.status == weave_batch::ItemStatus::Success)
                .map(|i| {
                    let post = i.output.as_ref().and_then(|p| std::fs::metadata(p).ok());
                    let post_modified = post.as_ref().and_then(|m| m.modified().ok());
                    if i.output_replaced {
                        replaced += 1;
                    } else {
                        created += 1;
                    }
                    weave_history::TransactionItem {
                        item_id: i.item_id.clone(),
                        source_path: i.source.to_string_lossy().into_owned(),
                        // 创建型记录：target 留空 ⇒ undo 走删除分支
                        target_path: String::new(),
                        status: weave_history::TransactionItemStatus::Executed,
                        timestamp: post_modified,
                        // replaced：None ⇒ undo 守卫必冲突（拒绝删除）
                        original_size: post
                            .as_ref()
                            .map(|m| m.len())
                            .filter(|_| !i.output_replaced),
                        original_modified: post_modified.filter(|_| !i.output_replaced),
                        original_created: None,
                    }
                })
                .collect();
            let reversible = match (created, replaced) {
                (0, _) => weave_history::Reversibility::None,
                (_, 0) => weave_history::Reversibility::Full,
                _ => weave_history::Reversibility::Partial,
            };
            let tx = weave_history::OperationTransaction {
                operation_id: operation.clone(),
                kind: weave_core::prelude::OperationKind::BatchExecute,
                status: weave_history::OperationStatus::Completed,
                timestamp: now,
                reversible,
                items,
            };
            let entry = weave_history::HistoryEntry {
                operation_id: operation.clone(),
                kind: weave_core::prelude::OperationKind::BatchExecute,
                timestamp: now,
                summary: format!("Batch pipeline · {} outputs", result.succeeded),
                item_count: result.total,
                success_count: result.succeeded,
                failed_count: result.failed,
                skipped_count: result.skipped,
                undoable: created > 0,
                status: weave_history::OperationStatus::Completed,
                input_root: None,
                rule_summary: None,
            };
            history_ok = weave_history::HistoryStore::open(&hdir)
                .and_then(|s| s.save_transaction(&tx).and_then(|_| s.upsert_entry(entry)))
                .is_ok();
            if !history_ok {
                tracing::warn!(job = %job_id_for_task, "batch history persistence failed");
            }
        }
        let mut dto = job_result_dto(&result);
        dto.operation_id = Some(operation.to_string());
        let _ = history_ok;
        app_state
            .jobs
            .finish(&job_id_for_task, JobOutcome::BatchExecuted(Box::new(dto)));
    });
    drop(state_cell);
    Ok(BatchJobHandleDto {
        job_id: job_id.to_string(),
    })
}
