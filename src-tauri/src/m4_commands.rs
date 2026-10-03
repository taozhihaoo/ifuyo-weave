//! M4：Text 工具的 IPC 命令（preview 纯函数 + 安全写回任务）。
//!
//! 命令只做 orchestration（M2 §93 同源纪律）；预览无副作用（§86），
//! 写回经 Plan/缓存 → 任务内 TOCTOU Revalidate → 备份 → 原子写 →
//! 事务/历史（§89–§95），undo 复用既有 undo_operation（§94）。

// IPC 边界与 D10 同理：Err DTO 体积不构成热路径问题（见 DECISIONS.md D10）。
#![expect(clippy::result_large_err)]

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;
use crate::jobs::JobOutcome;
use crate::ops_dto::PlanDto;
use crate::text_dto::{
    DiffReportDto, ExtractMatchDto, FormatOutcomeDto, TextDocumentDto, TransformOpDto,
    TransformResultDto,
};
use tauri::Manager;
use weave_core::prelude::WeaveError;

fn too_big() -> IpcError {
    WeaveError::validation("text.tooLarge", "input exceeds the text size limit").into()
}

/// 加载文本文档（解码级联 + 二进制守卫 + 尺寸上限）。
#[tauri::command]
#[specta::specta]
pub fn load_text_document(
    path: String,
    encoding: Option<String>,
) -> Result<TextDocumentDto, IpcError> {
    crate::text_service::load_text_document(&path, encoding.as_deref())
}

/// Format/Validate/Minify/Sort/Normalize 预览（纯函数，§86）。
#[tauri::command]
#[specta::specta]
pub fn format_text(
    format: String,
    operation: String,
    content: String,
    indent_spaces: Option<f64>,
    final_newline: Option<bool>,
) -> Result<FormatOutcomeDto, IpcError> {
    crate::text_service::format_text(
        &format,
        &operation,
        &content,
        indent_spaces.unwrap_or(2.0),
        final_newline.unwrap_or(true),
    )
}

/// Transformer 预览（纯函数）。
#[tauri::command]
#[specta::specta]
pub fn transform_text(
    content: String,
    operation: TransformOpDto,
) -> Result<TransformResultDto, IpcError> {
    crate::text_service::transform_text(&content, operation)
}

/// Extractor（含用户 Regex）。
#[tauri::command]
#[specta::specta]
pub fn extract_text(
    content: String,
    kind: String,
    regex: Option<String>,
    unique_values: Option<bool>,
) -> Result<Vec<ExtractMatchDto>, IpcError> {
    crate::text_service::extract_text(&content, &kind, regex, unique_values.unwrap_or(false))
}

/// Compare（Side-by-side 模型 + Unified 输出）。
#[tauri::command]
#[specta::specta]
pub fn compare_text(
    a: String,
    b: String,
    whitespace: Option<String>,
    ignore_case: Option<bool>,
) -> Result<DiffReportDto, IpcError> {
    crate::text_service::compare_text(
        &a,
        &b,
        whitespace.as_deref().unwrap_or("none"),
        ignore_case.unwrap_or(false),
    )
}

/// 构建写回计划（内容进服务端缓存；快照随 PlanItem，§90）。
#[tauri::command]
#[specta::specta]
pub fn build_text_write_plan(
    app: tauri::AppHandle,
    path: String,
    content: String,
    encoding: String,
    bom: String,
    snapshot_size: f64,
    snapshot_modified_ms: Option<f64>,
) -> Result<PlanDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    crate::text_service::build_text_write_plan(
        &state.text_writes,
        &state.plans,
        &path,
        &content,
        &encoding,
        &bom,
        snapshot_size,
        snapshot_modified_ms,
    )
}

/// 执行写回（任务内 Revalidate → 备份 → 原子写 → 事务/历史）。
#[tauri::command]
#[specta::specta]
pub fn execute_text_plan(
    app: tauri::AppHandle,
    operation_id: String,
) -> Result<JobHandleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    let Some(plan) = state.plans.take(&operation_id) else {
        return Err(WeaveError::validation(
            "plan.unknownOrExpired",
            format!("plan '{operation_id}' is unknown or expired (rebuild the plan)"),
        )
        .with_location("m4_commands::execute_text_plan")
        .into());
    };
    let Some(entry) = state.text_writes.take(&operation_id) else {
        return Err(WeaveError::validation(
            "plan.unknownOrExpired",
            "text write payload is unknown or expired (rebuild the plan)",
        )
        .with_location("m4_commands::execute_text_plan")
        .into());
    };
    crate::rename_service::persist_in_progress(&history_dir, &plan)?;

    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let history_dir = crate::rename_service::resolve_history_dir(&handle)
            .unwrap_or_else(|_| std::path::PathBuf::new());
        match crate::text_service::run_text_write_job(
            &history_dir,
            &plan,
            entry,
            &cancel,
            &mut |p| sink.report(&p),
        ) {
            Ok((report, _tx)) => {
                let dto = crate::ops_dto::PlanReportDto {
                    operation_id: report.operation_id.to_string(),
                    items: Vec::new(),
                    executed: 1.0,
                    failed: u64::from(report.failed) as f64,
                    skipped: 0.0,
                    undoable: !report.failed,
                    duration_ms: report.duration_ms as f64,
                    transaction: None,
                };
                app_state
                    .jobs
                    .finish(&job_id_for_task, JobOutcome::TextExecuted(Box::new(dto)));
            }
            Err((report, e)) => {
                let _ = report;
                app_state.jobs.fail(&job_id_for_task, e);
            }
        }
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

// 输入尺寸统一守卫（供未来批量入口复用）
#[allow(dead_code)]
fn ensure_size(len: usize) -> Result<(), IpcError> {
    if len as u64 > crate::text_service::MAX_TEXT_BYTES {
        return Err(too_big());
    }
    Ok(())
}
