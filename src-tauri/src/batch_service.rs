//! M7 Batch Job IPC 命令（§87-§90 + 下 §42/§90/§126/§143/§183/§202-§204）。
//!
//! - `batch_preview`：同步——同引擎模拟（§20-§23），Export 不落盘；
//! - `batch_execute`：任务化——JobTracker 注册，任务内快照重校验（§75）
//!   后 execute；Journal 逐条落盘（下 §42）；目的地互斥锁（下 §202/§204）；
//! - `batch_pause` / `batch_resume`：协作暂停 + journal 驱动恢复
//!   （重校验 + 冲突上报，绝不盲目重放 §185）；
//! - `batch_retry_failed`：失败子集重跑（新快照，从 item 起点执行 §39）；
//! - `batch_jobs_list`：跨重启的 Job 清单（§183 Completed/Failed/…/Interrupted）。
//!
//! 本层只做 DTO 映射、持久化接线与并发纪律，引擎逻辑全部在 weave-batch
//! （§86）；History/Undo 见 execute 任务内的 BatchExecute 事务（下 C3）。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_batch::journal::{
    self, JOURNAL_SCHEMA_VERSION, JournalFinish, JournalHeader, JournalItemRecord,
    JournalItemStatus, JournalLine,
};
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

/// Journal 目录：`%APPDATA%/ifuyo/Weave/jobs`（与 history 同源的路径约定）。
#[allow(clippy::result_large_err)] // D10
pub fn resolve_jobs_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, IpcError> {
    let dir = crate::rename_service::resolve_history_dir(app)?
        .parent()
        .map(|p| p.join("jobs"))
        .unwrap_or_else(|| std::path::PathBuf::from("jobs"));
    Ok(dir)
}

/// Journal 文件路径（state/命令共用约定）。
pub fn journal_path(dir: &std::path::Path, job_id: &str) -> std::path::PathBuf {
    journal::journal_path(dir, job_id)
}

/// 运行中 Batch Job 的服务端登记（当前活动 run；journal 是跨重启事实源）。
#[derive(Clone)]
pub struct BatchJobRecord {
    /// Journal 所属 job（跨 resume 不变）。
    pub parent_job_id: String,
    pub plan: weave_batch::JobPlan,
    pub journal_path: std::path::PathBuf,
    pub pause: Arc<AtomicBool>,
    /// running | paused
    pub state: String,
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

/// 批处理选项（§19 JobPlan 的 IPC 投影；下 §211 默认语义由 UI 层给默认值）。
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchOptionsDto {
    pub continue_on_error: bool,
    pub overwrite_existing: bool,
    /// worker 数（下 §211 Concurrency = Bounded；≥1，上限 8）。
    pub workers: Option<f64>,
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
    /// 下 §142：Failed 时是否可 Retry。
    pub retryable: bool,
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
    /// 下 §143：Pause 安全点后未开始条目。
    pub pending: f64,
    pub input_bytes: f64,
    pub output_bytes: f64,
    pub preview: bool,
    /// 关联 History 操作（撤销入口；unavailable 时 None）。
    pub operation_id: Option<String>,
    /// Resume 时被排除的输入（§143 Review Conflicts / §185）。
    pub resume_conflicts: Vec<String>,
}

/// 任务启动句柄（与 M1 hash/scan、M3 recycle 一致：get_job 轮询）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobHandleDto {
    pub job_id: String,
}

/// Job 清单条目（下 §183/§143 Resume UI 数据源）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobListItemDto {
    pub job_id: String,
    /// running | paused | completed | failed | cancelled | interrupted
    pub state: String,
    pub total_items: f64,
    pub settled_items: f64,
    pub pending_items: f64,
    /// 恢复时将被排除的输入（ChangedSincePreview / FileMissing，§185）。
    pub conflicts: Vec<String>,
    pub created_ms: Option<f64>,
    pub destination_dir: Option<String>,
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
                retryable: r.retryable,
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
        pending: result.pending as f64,
        input_bytes: result.input_bytes as f64,
        output_bytes: result.output_bytes as f64,
        preview: result.preview,
        operation_id: None,
        resume_conflicts: Vec::new(),
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

/// 目的地互斥（下 §202/§204）：canonical dest dir 锁；策略 = 拒绝。
#[allow(clippy::result_large_err)] // D10
fn acquire_dest_lock(
    state: &crate::state::AppState,
    job_id: &str,
    plan: &weave_batch::JobPlan,
) -> Result<(), IpcError> {
    let key = std::fs::canonicalize(&plan.destination_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| plan.destination_dir.to_string_lossy().into_owned());
    let mut locks = state.batch_dest_locks.lock().expect("dest locks");
    if let Some(holder) = locks.get(&key) {
        return Err(err(
            "batch.jobConflictOnDestination",
            format!(
                "another batch job ({holder}) is writing to the same destination; \
                 wait for it to finish (§202/§204)"
            ),
        ));
    }
    locks.insert(key, job_id.to_string());
    Ok(())
}

fn release_dest_lock(state: &crate::state::AppState, job_id: &str) {
    state
        .batch_dest_locks
        .lock()
        .expect("dest locks")
        .retain(|_, v| v != job_id);
}

fn journal_item_record(item: &weave_batch::ItemResult) -> JournalLine {
    let status = match item.status {
        weave_batch::ItemStatus::Success => JournalItemStatus::Success,
        weave_batch::ItemStatus::Failed => JournalItemStatus::Failed,
        weave_batch::ItemStatus::Skipped => JournalItemStatus::Skipped,
        weave_batch::ItemStatus::Cancelled => JournalItemStatus::Cancelled,
    };
    let output_size = item
        .output
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .or_else(|| (item.output_bytes > 0).then_some(item.output_bytes));
    JournalLine::Item(JournalItemRecord {
        item_index: item
            .item_id
            .trim_start_matches("item_")
            .parse()
            .unwrap_or(usize::MAX),
        status,
        output_path: item
            .output
            .as_ref()
            .map(|o| o.to_string_lossy().into_owned()),
        output_size,
        error: item.error.clone(),
    })
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
/// ChangedSincePreview/FileMissing ⇒ Job Failed）；Journal 逐条落盘；
/// 目的地互斥（§202/§204）。
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
    let jobs_dir = resolve_jobs_dir(&app)?;
    let journal_file = journal_path(&jobs_dir, &job_id.to_string());
    let header = JournalHeader {
        schema_version: JOURNAL_SCHEMA_VERSION,
        job_id: job_id.to_string(),
        created_ms: now_ms(),
        plan: plan.clone(),
        total_items: plan.input_snapshot.len() as u64,
    };
    journal::create(&jobs_dir, &header).map_err(|e| err("batch.journalFailed", e))?;
    acquire_dest_lock(&state, &job_id.to_string(), &plan)?;

    let workers = options.workers.unwrap_or(1.0).clamp(1.0, 8.0) as usize;
    let pause_flag = Arc::new(AtomicBool::new(false));
    state.batch_jobs.lock().expect("batch jobs").insert(
        job_id.to_string(),
        BatchJobRecord {
            parent_job_id: job_id.to_string(),
            plan: plan.clone(),
            journal_path: journal_file.clone(),
            pause: Arc::clone(&pause_flag),
            state: "running".into(),
        },
    );

    spawn_batch_task(
        app,
        job_id.to_string(),
        plan,
        (0..header.total_items as usize).collect(),
        workers,
        journal_file,
        pause_flag,
        sink,
        cancel,
        Vec::new(),
    );
    drop(state_cell);
    Ok(BatchJobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 下 §90 Pause：安全点后停止调度（未开始条目 = pending，非终态）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn batch_pause(app: tauri::AppHandle, job_id: String) -> Result<String, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let jobs = state.batch_jobs.lock().expect("batch jobs");
    let record = jobs
        .get(&job_id)
        .or_else(|| jobs.values().find(|r| r.parent_job_id == job_id))
        .ok_or_else(|| {
            err(
                "batch.unknownJob",
                format!("no active batch job '{job_id}'"),
            )
        })?;
    record.pause.store(true, Ordering::SeqCst);
    Ok("pausing".into())
}

/// 下 §90/§126/§143 Resume：journal 驱动（对 paused 与 interrupted 统一）——
/// 已有终态事实的条目不重跑（§39 副作用不重复）；剩余条目逐一重校验
/// （§238 外部变更 ⇒ 冲突上报并排除，§185 No Magic Recovery）；损坏
/// journal 先隔离（§197）再基于完好前缀恢复。返回新 run 的 job_id。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn batch_resume(app: tauri::AppHandle, job_id: String) -> Result<BatchJobHandleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let jobs_dir = resolve_jobs_dir(&app)?;
    let jpath = journal_path(&jobs_dir, &job_id);
    let load = journal::load(&jpath).map_err(|e| err("batch.unknownJob", e))?;
    let header = load
        .header
        .clone()
        .ok_or_else(|| err("batch.journalCorrupt", "journal header missing"))?;
    if header.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err(err(
            "batch.journalVersion",
            format!(
                "journal schema {} unsupported (current {JOURNAL_SCHEMA_VERSION})",
                header.schema_version
            ),
        ));
    }
    let mut conflicts: Vec<String> = Vec::new();
    if load.corrupt_tail_lines > 0 {
        // §197：保留原件副本，恢复只基于完好前缀
        let copy =
            journal::quarantine_corrupt(&jpath).map_err(|e| err("batch.journalQuarantine", e))?;
        conflicts.push(format!(
            "journal corrupt tail quarantined: {} ({} lines)",
            copy.display(),
            load.corrupt_tail_lines
        ));
    }
    // paused 也是 finish 行——但它就是 Resume 的合法起点（下 §143）；
    // 真终态（completed/failed/cancelled）拒绝恢复。
    if let Some(f) = &load.finish
        && f.state != "paused"
    {
        return Err(err(
            "batch.jobAlreadyFinished",
            format!(
                "job '{job_id}' already reached a terminal state ({})",
                f.state
            ),
        ));
    }
    let settled = load.settled_indices();
    let total = header.plan.input_snapshot.len();
    let pending: Vec<usize> = (0..total).filter(|i| !settled.contains(i)).collect();

    // §185：逐条重校验——证据不足者排除并上报，绝不盲跑
    let runnable: Vec<usize> = pending
        .into_iter()
        .filter(|&i| {
            let entry = &header.plan.input_snapshot[i];
            match std::fs::metadata(&entry.path) {
                Ok(m) => {
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    if m.len() == entry.size && mtime == entry.modified_ms {
                        true
                    } else {
                        conflicts.push(format!(
                            "ChangedSincePreview: {} (size {}→{}, mtime {}→{})",
                            entry.path.display(),
                            entry.size,
                            m.len(),
                            entry.modified_ms,
                            mtime
                        ));
                        false
                    }
                }
                Err(e) => {
                    conflicts.push(format!("FileMissing: {} ({e})", entry.path.display()));
                    false
                }
            }
        })
        .collect();
    if runnable.is_empty() {
        return Err(err(
            "batch.nothingToResume",
            "all remaining inputs changed or missing; re-plan the job instead",
        ));
    }

    // 新 run（新 tracker job；journal 续写）。同 parent 旧记录移除。
    let (new_job, cancel, sink, state_cell) = state.jobs.register();
    {
        let mut jobs = state.batch_jobs.lock().expect("batch jobs");
        jobs.retain(|_, r| r.parent_job_id != job_id);
        jobs.insert(
            new_job.to_string(),
            BatchJobRecord {
                parent_job_id: job_id.clone(),
                plan: header.plan.clone(),
                journal_path: jpath.clone(),
                pause: Arc::new(AtomicBool::new(false)),
                state: "running".into(),
            },
        );
    }
    acquire_dest_lock(&state, &new_job.to_string(), &header.plan)?;
    spawn_batch_task(
        app,
        new_job.to_string(),
        header.plan.clone(),
        runnable,
        1,
        jpath,
        Arc::new(AtomicBool::new(false)),
        sink,
        cancel,
        conflicts,
    );
    drop(state_cell);
    Ok(BatchJobHandleDto {
        job_id: new_job.to_string(),
    })
}

/// 下 §39/§142 Retry Failed：失败子集 + 全新快照（从 item 起点重新执行）；
/// 新 Job 新 Journal（Retry 是新的一次运行，不续写旧 journal）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn batch_retry_failed(
    app: tauri::AppHandle,
    job_id: String,
) -> Result<BatchJobHandleDto, IpcError> {
    let jobs_dir = resolve_jobs_dir(&app)?;
    let jpath = journal_path(&jobs_dir, &job_id);
    let load = journal::load(&jpath).map_err(|e| err("batch.unknownJob", e))?;
    let header = load
        .header
        .clone()
        .ok_or_else(|| err("batch.journalCorrupt", "journal header missing"))?;
    let failed: Vec<usize> = load
        .items
        .iter()
        .filter(|r| r.status == JournalItemStatus::Failed)
        .map(|r| r.item_index)
        .collect();
    if failed.is_empty() {
        return Err(err("batch.nothingToRetry", "no failed items to retry"));
    }
    let retry_inputs: Vec<String> = failed
        .iter()
        .map(|&i| {
            header.plan.input_snapshot[i]
                .path
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    // 重建计划：同管线同选项；新快照 = 当前文件事实（重校验天然内含）
    let stages_dto: Vec<BatchStageDto> = header
        .plan
        .pipeline
        .stages
        .iter()
        .map(stage_to_dto)
        .collect();
    let dest = header
        .plan
        .pipeline
        .stages
        .iter()
        .find_map(|s| match s {
            weave_batch::StageSpec::Export {
                destination_dir, ..
            } => Some(destination_dir.to_string_lossy().into_owned()),
            _ => None,
        })
        .unwrap_or_else(|| header.plan.destination_dir.to_string_lossy().into_owned());
    let options = BatchOptionsDto {
        continue_on_error: header.plan.continue_on_error,
        overwrite_existing: header.plan.overwrite_existing,
        workers: None,
    };
    // Retry 尾调 batch_execute 的完整纪律（锁/journal/记录）
    batch_execute(app, retry_inputs, stages_dto, dest, options)
}

/// Job 清单（下 §183）：journal 事实源 + 内存活动态合并。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn batch_jobs_list(app: tauri::AppHandle) -> Result<Vec<BatchJobListItemDto>, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let jobs_dir = resolve_jobs_dir(&app)?;
    let active: HashMap<String, String> = state
        .batch_jobs
        .lock()
        .expect("batch jobs")
        .values()
        .map(|v| (v.parent_job_id.clone(), v.state.clone()))
        .collect();
    let mut out: Vec<BatchJobListItemDto> = journal::list(&jobs_dir)
        .into_iter()
        .map(|s| {
            let live = active.get(&s.job_id);
            let state_str = match live.map(String::as_str) {
                Some("running") | Some("paused") => live.expect("matched").clone(),
                _ => finish_state(s.load.finish.as_ref()),
            };
            let total = s
                .header
                .as_ref()
                .map(|h| h.total_items)
                .unwrap_or(s.load.items.len() as u64);
            BatchJobListItemDto {
                job_id: s.job_id.clone(),
                state: state_str,
                total_items: total as f64,
                settled_items: s.load.settled_indices().len() as f64,
                pending_items: (total as usize - s.load.settled_indices().len()) as f64,
                conflicts: Vec::new(),
                created_ms: s.header.as_ref().map(|h| h.created_ms as f64),
                destination_dir: s
                    .header
                    .as_ref()
                    .map(|h| h.plan.destination_dir.to_string_lossy().into_owned()),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.created_ms
            .unwrap_or(0.0)
            .partial_cmp(&a.created_ms.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

fn finish_state(finish: Option<&JournalFinish>) -> String {
    match finish {
        Some(f) => f.state.clone(),
        None => "interrupted".into(),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// StageSpec → DTO（Retry 重建计划的投影；领域值已验证过，纯投影）。
fn stage_to_dto(stage: &StageSpec) -> BatchStageDto {
    match stage {
        StageSpec::Source => BatchStageDto::Source,
        StageSpec::Filter {
            extensions_in,
            max_bytes,
        } => BatchStageDto::Filter {
            extensions_in: extensions_in.clone(),
            max_bytes: max_bytes.map(|b| b as f64),
        },
        StageSpec::TextTransform { operations } => BatchStageDto::TextTransform {
            operations: operations
                .iter()
                .map(|op| match op {
                    TextOpSpec::TrimLines => BatchTextOpDto::TrimLines,
                    TextOpSpec::Lowercase => BatchTextOpDto::Lowercase,
                    TextOpSpec::Uppercase => BatchTextOpDto::Uppercase,
                    TextOpSpec::Replace {
                        find,
                        replace_with,
                        case_sensitive,
                    } => BatchTextOpDto::Replace {
                        find: find.clone(),
                        replace_with: replace_with.clone(),
                        case_sensitive: *case_sensitive,
                    },
                })
                .collect(),
        },
        StageSpec::ImageResize {
            width,
            height,
            mode,
            prevent_upscale,
        } => BatchStageDto::ImageResize {
            width: *width as f64,
            height: *height as f64,
            mode: mode.clone(),
            prevent_upscale: *prevent_upscale,
        },
        StageSpec::Encode { format, quality } => BatchStageDto::Encode {
            format: format.clone(),
            quality: quality.map(|q| q as f64),
        },
        StageSpec::Export {
            destination_dir,
            overwrite,
        } => BatchStageDto::Export {
            destination_dir: destination_dir.to_string_lossy().into_owned(),
            overwrite: *overwrite,
        },
    }
}

/// 共享任务体：重校验（首跑全量）→ execute（journal on_item）→ History
/// 事务（下 C3）→ Finish 行 → 记录/锁清理。
#[allow(clippy::too_many_arguments)]
fn spawn_batch_task(
    app: tauri::AppHandle,
    tracker_job_id: String,
    plan: weave_batch::JobPlan,
    indices: Vec<usize>,
    workers: usize,
    journal_file: std::path::PathBuf,
    pause_flag: Arc<AtomicBool>,
    sink: crate::jobs::ProgressSink,
    cancel: weave_core::prelude::CancellationToken,
    resume_conflicts: Vec<String>,
) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let tracker = weave_core::prelude::JobId::parse(&tracker_job_id)
            .unwrap_or_else(|_| weave_core::prelude::JobId::generate());
        // §75 Execute Revalidation：Preview 与 Execute 之间输入可能已变化
        if let Err(e) = weave_batch::revalidate_snapshot(&plan.input_snapshot) {
            app_state.jobs.fail(
                &tracker,
                WeaveError::validation("batch.changedSincePreview", e)
                    .with_location("batch_service::spawn_batch_task"),
            );
            release_dest_lock(&app_state, &tracker_job_id);
            return;
        }
        let total = plan.input_snapshot.len() as u64;
        let operation = OperationId::generate();
        // Journal 逐条事实（下 §42）：Mutex 串行化多 worker 追加
        let jwrite = std::sync::Mutex::new(());
        let jpath = journal_file.clone();
        let on_item = move |item: &weave_batch::ItemResult| {
            let _guard = jwrite.lock().expect("journal lock");
            let _ = journal::append(&jpath, &journal_item_record(item));
        };
        let result = weave_batch::execute_subset(
            &plan,
            &indices,
            weave_batch::JobExecOptions {
                workers,
                pause: Some(&pause_flag),
                dry_run: false,
                on_item: Some(&on_item),
            },
            &cancel,
            Some(&mut |done, _desc| {
                sink.report(&Progress::running(operation.clone(), done, Some(total)));
            }),
        );
        // C3 §184/§246：批量产物入 History（kind=BatchExecute）。创建型产物
        // 撤销 = 删除已创建文件（stat 守卫）；覆盖写产物无备份 ⇒ 以
        // original_modified=None 记录，撤销时守卫必报冲突（拒绝删除，§185）。
        // 历史写失败 ≠ 操作失败（M2 §84）。
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
                        // 创建型契约（同 text_service）：source_path = 已创建的
                        // 产物路径——undo 走删除分支删的是它（stat 守卫保护），
                        // 绝不是用户输入文件。
                        source_path: i
                            .output
                            .as_ref()
                            .map(|o| o.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        target_path: String::new(), // 创建型记录：undo 走删除分支
                        status: weave_history::TransactionItemStatus::Executed,
                        timestamp: post_modified,
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
                tracing::warn!(job = %tracker_job_id, "batch history persistence failed");
            }
        }
        let mut dto = job_result_dto(&result);
        dto.operation_id = Some(operation.to_string());
        dto.resume_conflicts = resume_conflicts;
        let _ = history_ok;
        app_state
            .jobs
            .finish(&tracker, JobOutcome::BatchExecuted(Box::new(dto)));

        // Journal Finish 行（下 §183：跨重启状态判定的事实）
        let finish_state = if result.pending > 0 {
            "paused"
        } else if result.cancelled > 0 {
            "cancelled"
        } else if result.failed > 0 {
            "failed"
        } else {
            "completed"
        };
        let _ = journal::append(
            &journal_file,
            &JournalLine::Finish(JournalFinish {
                state: finish_state.into(),
            }),
        );

        // 记录收尾 + 锁释放（paused 保留记录供 pause 查询；其余移除）
        {
            let mut jobs = app_state.batch_jobs.lock().expect("batch jobs");
            if result.pending > 0 {
                if let Some(rec) = jobs.get_mut(&tracker_job_id) {
                    rec.state = "paused".into();
                }
            } else {
                jobs.remove(&tracker_job_id);
            }
        }
        release_dest_lock(&app_state, &tracker_job_id);
    });
}
