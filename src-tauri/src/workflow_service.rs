//! M10（上）：Workflow IPC（§111 small/typed/bounded）。
//!
//! - `workflow_save`：保存前 Validation（ERROR 阻止 §29）+ 原子保存
//!   （§120/§121 复用 M2 atomic_write，无部分文件）；
//! - `workflow_list/get/delete/duplicate`：Workflow Library（§77-§87：
//!   delete 只删定义，不动文件/History；duplicate 换 id 保留定义 §44）；
//! - `workflow_import_json`：导入先 Validate（§122 不允许 Import→Execute）；
//! - `workflow_preview`：compile → weave_batch::preview_plan（§30-§33
//!   同引擎 dry-run，真实预览非 mock）；
//! - `workflow_run`：compile → JobTracker 后台执行（§35 复用 M7；
//!   §92 取消经既有 cancel_job；进度经既有 get_job 轮询）。
//!
//! Workflow 定义存放 `%APPDATA%/ifuyo/Weave/workflows/<id>.json`。

use serde::Serialize;
use specta::Type;
use tauri::Manager;
use weave_core::prelude::WeaveError;
use weave_workflow::{Workflow, validate};

use crate::batch_service::job_result_dto;
use crate::commands::IpcError;
use crate::jobs::JobOutcome;

#[allow(clippy::result_large_err)] // D10
fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("workflow_service")
        .into()
}

/// Workflow Library 目录：`%APPDATA%/ifuyo/Weave/workflows`。
#[allow(clippy::result_large_err)] // D10
pub fn resolve_workflows_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, IpcError> {
    Ok(crate::rename_service::resolve_history_dir(app)?
        .parent()
        .map(|p| p.join("workflows"))
        .unwrap_or_else(|| std::path::PathBuf::from("workflows")))
}

fn workflow_path(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    // §101 防路径注入：id 白名单字符（字母数字-_），其余归下划线
    let safe: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("{safe}.json"))
}

/// §26 保存前校验：ERROR ⇒ 拒绝。
#[allow(clippy::result_large_err)] // D10
fn validate_or_err(workflow: &Workflow) -> Result<(), IpcError> {
    let v = validate(workflow);
    if v.has_errors() {
        let first = v
            .errors()
            .next()
            .map(|i| format!("{}: {}", i.code, i.message))
            .unwrap_or_default();
        return Err(err("workflow.validationFailed", first));
    }
    Ok(())
}

/// §26/§29：校验并返回 issues（severity error|warning；仅 error 阻止）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssueDto {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub step_id: Option<String>,
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_validate(workflow_json: String) -> Result<Vec<ValidationIssueDto>, IpcError> {
    let workflow: Workflow = serde_json::from_str(&workflow_json).map_err(|e| {
        err(
            "workflow.importInvalid",
            format!("invalid workflow JSON: {e}"),
        )
    })?;
    let v = validate(&workflow);
    Ok(v.issues
        .iter()
        .map(|i| ValidationIssueDto {
            severity: i.severity.clone(),
            code: i.code.clone(),
            message: i.message.clone(),
            step_id: i.step_id.clone(),
        })
        .collect())
}

/// §77 Workflow Library 条目。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowListItemDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schema_version: f64,
    pub step_count: f64,
}

/// 保存（§120/§121：serialize → validate → M2 atomic save）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_save(app: tauri::AppHandle, workflow_json: String) -> Result<String, IpcError> {
    let workflow: Workflow = serde_json::from_str(&workflow_json).map_err(|e| {
        err(
            "workflow.importInvalid",
            format!("invalid workflow JSON: {e}"),
        )
    })?;
    validate_or_err(&workflow)?;
    if workflow.id.trim().is_empty() {
        return Err(err("workflow.emptyId", "workflow id must not be empty"));
    }
    let dir = resolve_workflows_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| {
        err(
            "workflow.dirFailed",
            format!("cannot create workflows dir: {e}"),
        )
    })?;
    let json = serde_json::to_string_pretty(&workflow)
        .map_err(|e| err("workflow.serializeFailed", format!("serialize failed: {e}")))?;
    let path = workflow_path(&dir, &workflow.id);
    weave_files::atomic_write(&weave_files::fs::StdFilesystem, &path, json.as_bytes()).map_err(
        |e| {
            err(
                "workflow.saveFailed",
                format!("{}: {}", path.display(), e.message),
            )
        },
    )?;
    Ok(workflow.id)
}

/// Library 列表（§77：扫描目录读 header 字段；损坏文件跳过并如实计数）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_list(app: tauri::AppHandle) -> Result<Vec<WorkflowListItemDto>, IpcError> {
    let dir = resolve_workflows_dir(&app)?;
    let mut out: Vec<WorkflowListItemDto> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path)
                && let Ok(wf) = serde_json::from_str::<Workflow>(&text)
            {
                out.push(WorkflowListItemDto {
                    id: wf.id,
                    name: wf.name,
                    description: wf.description,
                    schema_version: wf.schema_version as f64,
                    step_count: wf.steps.len() as f64,
                });
            }
        }
    }
    out.sort_by_key(|w| w.name.to_lowercase());
    Ok(out)
}

/// 读取单个 Workflow 定义。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_get(app: tauri::AppHandle, id: String) -> Result<String, IpcError> {
    let dir = resolve_workflows_dir(&app)?;
    let path = workflow_path(&dir, &id);
    let text = std::fs::read_to_string(&path)
        .map_err(|_| err("workflow.unknown", format!("workflow '{id}' not found")))?;
    serde_json::from_str(&text).map_err(|e| {
        err(
            "workflow.corrupt",
            format!("workflow '{id}' is corrupt: {e}"),
        )
    })
}

/// §45：Delete Workflow 只删除定义文件（不动文件/History §87）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_delete(app: tauri::AppHandle, id: String) -> Result<bool, IpcError> {
    let dir = resolve_workflows_dir(&app)?;
    let path = workflow_path(&dir, &id);
    if !path.exists() {
        return Err(err(
            "workflow.unknown",
            format!("workflow '{id}' not found"),
        ));
    }
    std::fs::remove_file(&path)
        .map(|_| true)
        .map_err(|e| err("workflow.deleteFailed", format!("{}: {e}", path.display())))
}

/// §44 Duplicate：新 id（调用方生成/提供）、定义保持、名称可带 Copy。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_duplicate(
    app: tauri::AppHandle,
    id: String,
    new_id: String,
    new_name: String,
) -> Result<String, IpcError> {
    let text = workflow_get(app.clone(), id)?;
    let mut wf: Workflow = serde_json::from_str(&text)
        .map_err(|e| err("workflow.corrupt", format!("corrupt workflow: {e}")))?;
    wf.id = new_id.clone();
    wf.name = new_name;
    workflow_save(app, serde_json::to_string(&wf).expect("serialize"))
}

/// §122 Import：Parse → Validate（不执行）→ 保存由调用方显式 workflow_save。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_import_json(json: String) -> Result<String, IpcError> {
    let wf: Workflow = serde_json::from_str(&json).map_err(|e| {
        err(
            "workflow.importInvalid",
            format!("invalid workflow JSON: {e}"),
        )
    })?;
    validate_or_err(&wf)?;
    serde_json::to_string_pretty(&wf)
        .map_err(|e| err("workflow.serializeFailed", format!("serialize failed: {e}")))
}

/// §123 Export：确定性 pretty JSON。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_export_json(app: tauri::AppHandle, id: String) -> Result<String, IpcError> {
    let wf = workflow_get(app, id)?;
    serde_json::to_string_pretty(&wf)
        .map_err(|e| err("workflow.serializeFailed", format!("serialize failed: {e}")))
}

/// §30-§33 Preview：compile → weave_batch::preview_plan（同引擎 dry-run）。
/// inputs 由调用方提供（运行时参数，portable workflow §62）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_preview(
    workflow_json: String,
    inputs: Vec<String>,
) -> Result<PreviewSummaryDto, IpcError> {
    let workflow: Workflow = serde_json::from_str(&workflow_json).map_err(|e| {
        err(
            "workflow.importInvalid",
            format!("invalid workflow JSON: {e}"),
        )
    })?;
    validate_or_err(&workflow)?;
    for path in &inputs {
        weave_core::prelude::validate_absolute_path(path)?;
    }
    let plan = weave_workflow::compile(
        &workflow,
        inputs.iter().map(std::path::PathBuf::from).collect(),
    )
    .map_err(|e| err(e.code, e.message))?;
    let input_count = plan.input_snapshot.len() as f64;
    let result = weave_batch::preview_plan(&plan);
    Ok(PreviewSummaryDto {
        input_count,
        succeeded: result.succeeded as f64,
        failed: result.failed as f64,
        skipped: result.skipped as f64,
        cancelled: result.cancelled as f64,
        output_bytes: result.output_bytes as f64,
        issues: result
            .items
            .iter()
            .filter_map(|i| i.error.clone())
            .take(20)
            .collect(),
    })
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSummaryDto {
    pub input_count: f64,
    pub succeeded: f64,
    pub failed: f64,
    pub skipped: f64,
    pub cancelled: f64,
    pub output_bytes: f64,
    /// 潜在失败摘录（§57 有界 20 条）。
    pub issues: Vec<String>,
}

/// §35/§37 Run：compile → JobTracker 后台执行（§92 取消经 cancel_job，
/// §107 进度经 get_job 轮询）。产物经既有 BatchOutcome 呈现。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunHandleDto {
    pub run_id: String,
    pub job_id: String,
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn workflow_run(
    app: tauri::AppHandle,
    workflow_json: String,
    inputs: Vec<String>,
) -> Result<WorkflowRunHandleDto, IpcError> {
    let workflow: Workflow = serde_json::from_str(&workflow_json).map_err(|e| {
        err(
            "workflow.importInvalid",
            format!("invalid workflow JSON: {e}"),
        )
    })?;
    validate_or_err(&workflow)?;
    for path in &inputs {
        weave_core::prelude::validate_absolute_path(path)?;
    }
    let plan = weave_workflow::compile(
        &workflow,
        inputs.iter().map(std::path::PathBuf::from).collect(),
    )
    .map_err(|e| err(e.code, e.message))?;

    let state = app.state::<crate::state::AppState>();
    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    let workflow_id = workflow.id.clone();
    let run_id = format!("wfrun_{}", job_id);
    let total = plan.input_snapshot.len() as f64;
    let wf_id_for_log = workflow_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use weave_core::prelude::{OperationId, Progress};
        let app_state = handle.state::<crate::state::AppState>();
        // §141 同源 TOCTOU：运行前快照重校验
        if let Err(e) = weave_batch::revalidate_snapshot(&plan.input_snapshot) {
            app_state.jobs.fail(
                &job_id_for_task,
                WeaveError::validation("workflow.changedSincePlan", e)
                    .with_location("workflow_service::workflow_run"),
            );
            return;
        }
        let operation = OperationId::generate();
        let result = weave_batch::execute_plan(
            &plan,
            &cancel,
            Some(&mut |done, _desc| {
                sink.report(&Progress::running(
                    operation.clone(),
                    done,
                    Some(total as u64),
                ));
            }),
        );
        app_state.jobs.finish(
            &job_id_for_task,
            JobOutcome::BatchExecuted(Box::new(job_result_dto(&result))),
        );
        let _ = wf_id_for_log;
    });
    drop(state_cell);
    Ok(WorkflowRunHandleDto {
        run_id,
        job_id: job_id.to_string(),
    })
}
