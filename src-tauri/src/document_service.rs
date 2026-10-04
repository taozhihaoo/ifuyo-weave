//! M8（上）：Documents IPC 命令（§109/§113-§116）。
//!
//! - `document_inspect`：只读事实（§12/§64）；
//! - `pdf_merge_preview` / `pdf_merge_execute`：有序合并（§18-§20/§74，
//!   单一逻辑操作——非 item batch）；
//! - `pdf_extract_execute` / `pdf_rotate_execute` / `pdf_split_every_n_execute`：
//!   单文档 mutation（§65-§66 output-first：只写新文件，目标存在 ⇒ 拒绝）。
//!
//! 历史与撤销：产物经 OperationKind::BatchExecute creation-transaction
//! 落 History（撤销 = 删除已创建文件，stat 守卫）——复用 M7 收口的
//! record_creation_transaction，不建第二套事务（§205）。

use serde::Serialize;
use specta::Type;
use tauri::Manager;
use weave_core::prelude::WeaveError;
use weave_documents::{DocumentFacts, DocumentResourceLimits, PdfMergePlan, Rotation};

use crate::commands::IpcError;

// IPC 边界与 D10 同理。
#[allow(clippy::result_large_err)] // D10
fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("document_service")
        .into()
}

fn limits() -> DocumentResourceLimits {
    DocumentResourceLimits::default()
}

/// 只读检查（§12/§64：不改原文件；解析失败 = facts 内诊断，非 Err——
/// Err 仅保留给路径级失败）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn document_inspect(path: String) -> Result<DocumentFacts, IpcError> {
    weave_core::prelude::validate_absolute_path(&path)?;
    let p = std::path::PathBuf::from(&path);
    if !p.is_file() {
        return Err(err("document.pathMissing", format!("not a file: {path}")));
    }
    Ok(weave_documents::inspect_document(&p, &limits()))
}

/// 通用产物结果（§72：Created Outputs + Diagnostics）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PdfOperationResultDto {
    pub outputs: Vec<String>,
    pub page_counts: Vec<f64>,
    pub warnings: Vec<String>,
}

/// 目标输出命名 + 碰撞检查（§66 output-first；默认绝不覆盖）。
#[allow(clippy::result_large_err)] // D10
fn prepare_output(destination_dir: &str, file_name: &str) -> Result<std::path::PathBuf, IpcError> {
    weave_core::prelude::validate_absolute_path(destination_dir)?;
    if !std::fs::metadata(destination_dir)
        .map(|m| m.is_dir())
        .unwrap_or(false)
    {
        return Err(err(
            "document.destinationMissing",
            format!("destination dir does not exist: {destination_dir}"),
        ));
    }
    let out = std::path::PathBuf::from(destination_dir).join(file_name);
    if out.exists() {
        return Err(err(
            "document.destinationExists",
            format!(
                "output already exists: {} (output-first; overwrite is not enabled)",
                out.display()
            ),
        ));
    }
    Ok(out)
}

fn stem_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".into())
}

/// Merge plan 服务端缓存句柄（§19/§141：execute 用同一 plan 做 TOCTOU）。
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PdfMergePlanHandleDto {
    pub plan_id: String,
    pub plan: PdfMergePlan,
}

/// §19：Merge Preview（每输入页数 + 总页数 + 顺序）。Plan 进服务端缓存，
/// execute 只收 plan_id（M3/M4 同模式）——TOCTOU 重校验有据可依。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn pdf_merge_preview(
    app: tauri::AppHandle,
    inputs: Vec<String>,
) -> Result<PdfMergePlanHandleDto, IpcError> {
    if inputs.is_empty() {
        return Err(err("document.emptySelection", "no input files"));
    }
    for path in &inputs {
        weave_core::prelude::validate_absolute_path(path)?;
    }
    let paths: Vec<std::path::PathBuf> = inputs.iter().map(std::path::PathBuf::from).collect();
    let plan =
        weave_documents::plan_merge(&paths, &limits()).map_err(|e| err(e.code, e.message))?;
    let plan_id = weave_core::prelude::OperationId::generate().to_string();
    app.state::<crate::state::AppState>()
        .document_plans
        .lock()
        .expect("document plans")
        .insert(plan_id.clone(), plan.clone());
    Ok(PdfMergePlanHandleDto { plan_id, plan })
}

/// §18/§20/§74：有序合并（单一逻辑操作；输出 = 首输入 stem.merged.pdf）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn pdf_merge_execute(
    app: tauri::AppHandle,
    plan_id: String,
    destination_dir: String,
) -> Result<PdfOperationResultDto, IpcError> {
    // §141：只接受 Preview 缓存的 plan——重校验依据 = 预览时的输入快照
    let plan = {
        let taken = app
            .state::<crate::state::AppState>()
            .document_plans
            .lock()
            .expect("document plans")
            .remove(&plan_id);
        taken.ok_or_else(|| {
            err(
                "document.planUnknownOrExpired",
                format!("merge plan '{plan_id}' unknown or expired (re-preview)"),
            )
        })?
    };
    let execute_result = (|| -> Result<PdfOperationResultDto, IpcError> {
        let inputs: Vec<String> = plan
            .inputs
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let first_stem = stem_of(&inputs[0]);
        let output = prepare_output(&destination_dir, &format!("{first_stem}.merged.pdf"))?;
        let (pages, diagnostics) = weave_documents::execute_merge(&plan, &output, &limits())
            .map_err(|e| err(e.code, e.message))?;
        record_outputs(&app, &inputs, std::slice::from_ref(&output));
        Ok(PdfOperationResultDto {
            outputs: vec![output.to_string_lossy().into_owned()],
            page_counts: vec![pages as f64],
            warnings: diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect(),
        })
    })();
    if execute_result.is_err() {
        // 失败放回缓存允许重试（重试仍经 §141 重校验，§129 不盲放）
        app.state::<crate::state::AppState>()
            .document_plans
            .lock()
            .expect("document plans")
            .insert(plan_id, plan);
    }
    execute_result
}

/// §21/§22：按页范围导出（新文件 {stem}.extract.pdf）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn pdf_extract_execute(
    app: tauri::AppHandle,
    input: String,
    ranges: String,
    destination_dir: String,
) -> Result<PdfOperationResultDto, IpcError> {
    weave_core::prelude::validate_absolute_path(&input)?;
    let output = prepare_output(
        &destination_dir,
        &format!("{}.extract.pdf", stem_of(&input)),
    )?;
    let pages = weave_documents::execute_extract_pages(
        std::path::Path::new(&input),
        &ranges,
        &output,
        &limits(),
    )
    .map_err(|e| err(e.code, e.message))?;
    record_outputs(&app, &[input], std::slice::from_ref(&output));
    Ok(PdfOperationResultDto {
        outputs: vec![output.to_string_lossy().into_owned()],
        page_counts: vec![pages as f64],
        warnings: Vec::new(),
    })
}

/// §25-§26：页旋转（/Rotate 元数据语义；新文件 {stem}.rotated.pdf）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn pdf_rotate_execute(
    app: tauri::AppHandle,
    input: String,
    degrees: f64,
    ranges: String,
    destination_dir: String,
) -> Result<PdfOperationResultDto, IpcError> {
    weave_core::prelude::validate_absolute_path(&input)?;
    let output = prepare_output(
        &destination_dir,
        &format!("{}.rotated.pdf", stem_of(&input)),
    )?;
    let (pages, rotated) = weave_documents::execute_rotate(
        std::path::Path::new(&input),
        match degrees as i64 {
            90 => Rotation::Deg90,
            180 => Rotation::Deg180,
            270 => Rotation::Deg270,
            other => {
                return Err(err(
                    "document.badRotation",
                    format!("rotation must be 90/180/270, got {other}"),
                ));
            }
        },
        &ranges,
        &output,
        &limits(),
    )
    .map_err(|e| err(e.code, e.message))?;
    record_outputs(&app, &[input], std::slice::from_ref(&output));
    Ok(PdfOperationResultDto {
        outputs: vec![output.to_string_lossy().into_owned()],
        page_counts: vec![pages as f64],
        warnings: vec![format!(
            "rotation metadata /Rotate set on {} page(s); content is not re-rendered (§26)",
            rotated.len()
        )],
    })
}

/// §23：Split Every N（确定性 part 命名 §28）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn pdf_split_every_n_execute(
    app: tauri::AppHandle,
    input: String,
    n: f64,
    destination_dir: String,
) -> Result<PdfOperationResultDto, IpcError> {
    weave_core::prelude::validate_absolute_path(&input)?;
    weave_core::prelude::validate_absolute_path(&destination_dir)?;
    let out_dir = std::path::PathBuf::from(&destination_dir);
    // §28 collision-safe：目标目录存在同名 part ⇒ 先拒绝（不覆盖）
    let stem = stem_of(&input);
    let probe = out_dir.join(weave_documents::split_output_name(&stem, 1));
    if probe.exists() {
        return Err(err(
            "document.destinationExists",
            format!("output already exists: {} (output-first)", probe.display()),
        ));
    }
    let parts = weave_documents::execute_split_every_n(
        std::path::Path::new(&input),
        n.max(1.0) as u64,
        &out_dir,
        &limits(),
    )
    .map_err(|e| err(e.code, e.message))?;
    let outputs: Vec<std::path::PathBuf> = parts.iter().map(|(p, _)| p.clone()).collect();
    record_outputs(&app, &[input], &outputs);
    Ok(PdfOperationResultDto {
        outputs: outputs
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        page_counts: parts.iter().map(|(_, c)| *c as f64).collect(),
        warnings: Vec::new(),
    })
}

/// History 记录（§77/§78：只记 what happened，不记内容）。
/// 失败 ≠ 操作失败（M2 §84）：仅日志告警。
fn record_outputs(app: &tauri::AppHandle, inputs: &[String], outputs: &[std::path::PathBuf]) {
    let Ok(history_dir) = crate::rename_service::resolve_history_dir(app) else {
        tracing::warn!("history dir unavailable; document outputs not recorded");
        return;
    };
    if let Err(e) = crate::batch_service::record_creation_transaction(
        &history_dir,
        inputs,
        outputs,
        &format!("Document operation · {} outputs", outputs.len()),
    ) {
        tracing::warn!(error = %e, "document history persistence failed");
    }
}
