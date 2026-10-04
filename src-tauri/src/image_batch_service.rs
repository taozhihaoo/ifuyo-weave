//! M6（下）：批量 IPC 命令（§145/§170-§175）。
//!
//! 批量执行为同步命令（Job 化列 M7）；进度经任务层（§172：per-file
//! 计数即可，不伪造百分比）。输入快照（§202）+ 失败隔离（§84）。

use serde::{Deserialize, Serialize};
use specta::Type;
use weave_core::prelude::{CancellationToken, WeaveError};
use weave_media::{
    BatchFileStatus, BatchResult, ImageBatchPlan, ImageFormat, ImageLimits, run_batch,
    snapshot_inputs,
};

use crate::commands::IpcError;

// IPC 边界与 D10 同理：Err DTO 体积不构成热路径问题。
#[allow(clippy::result_large_err)] // D10：Err DTO 体积豁免
fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("image_batch_service")
        .into()
}

fn limits() -> ImageLimits {
    ImageLimits::default()
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchFileResultDto {
    pub input: String,
    pub output: Option<String>,
    /// success | failed | skipped | cancelled
    pub status: String,
    pub error: Option<String>,
    pub input_bytes: f64,
    pub output_bytes: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BatchResultDto {
    pub results: Vec<BatchFileResultDto>,
    pub total: f64,
    pub succeeded: f64,
    pub failed: f64,
    pub cancelled: f64,
    pub input_bytes: f64,
    pub output_bytes: f64,
}

fn status_str(status: &BatchFileStatus) -> &'static str {
    match status {
        weave_media::BatchFileStatus::Success => "success",
        weave_media::BatchFileStatus::Failed => "failed",
        weave_media::BatchFileStatus::Skipped => "skipped",
        weave_media::BatchFileStatus::Cancelled => "cancelled",
    }
}

fn result_dto(result: &BatchResult) -> BatchResultDto {
    BatchResultDto {
        results: result
            .results
            .iter()
            .map(|r| BatchFileResultDto {
                input: r.input.to_string_lossy().into_owned(),
                output: r.output.as_ref().map(|o| o.to_string_lossy().into_owned()),
                status: status_str(&r.status).to_string(),
                error: r.error.clone(),
                input_bytes: r.input_bytes as f64,
                output_bytes: r.output_bytes as f64,
            })
            .collect(),
        total: result.total as f64,
        succeeded: result.succeeded as f64,
        failed: result.failed as f64,
        cancelled: result.cancelled as f64,
        input_bytes: result.input_bytes as f64,
        output_bytes: result.output_bytes as f64,
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageBatchOptionsDto {
    pub destination_dir: String,
    /// png | jpeg | webp | bmp | tiff
    pub target_format: String,
    pub quality: Option<f64>,
    pub overwrite_existing: bool,
}

/// 批量执行：输入快照 → 逐文件处理（失败隔离/取消安全点）→ 汇总。
#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn image_batch_execute(
    inputs: Vec<String>,
    options: ImageBatchOptionsDto,
) -> Result<BatchResultDto, IpcError> {
    if inputs.is_empty() {
        return Err(err("data.emptySelection", "no input files"));
    }
    weave_core::prelude::validate_absolute_path(&options.destination_dir)?;
    if !std::fs::metadata(&options.destination_dir)
        .map(|m| m.is_dir())
        .unwrap_or(false)
    {
        return Err(err(
            "data.destinationMissing",
            format!(
                "destination dir does not exist: {}",
                options.destination_dir
            ),
        ));
    }
    let target_format = ImageFormat::from_extension(&options.target_format).ok_or_else(|| {
        err(
            "image.unknownFormat",
            format!("unknown format '{}'", options.target_format),
        )
    })?;

    let input_paths: Vec<std::path::PathBuf> =
        inputs.iter().map(std::path::PathBuf::from).collect();
    let snapshot = snapshot_inputs(input_paths, target_format);
    let unsupported = inputs.len().saturating_sub(snapshot.len());

    let plan = ImageBatchPlan {
        inputs: snapshot,
        destination_dir: std::path::PathBuf::from(&options.destination_dir),
        target_format,
        quality: options.quality.map(|q| q.clamp(1.0, 100.0) as u8),
        overwrite_existing: options.overwrite_existing,
    };

    let result = run_batch(&plan, &limits(), &CancellationToken::new());
    let mut dto = result_dto(&result);
    if unsupported > 0 {
        dto.results.insert(
            0,
            BatchFileResultDto {
                input: String::new(),
                output: None,
                status: "skipped".to_string(),
                error: Some(format!("{unsupported} inputs skipped as non-image")),
                input_bytes: 0.0,
                output_bytes: 0.0,
            },
        );
    }
    Ok(dto)
}
